#!/usr/bin/env python3
"""Stop hook: cargo check (+ clippy) after agent turns that touch Rust.

Returns followup_message on failure so the agent auto-fixes (bounded by loop_limit).
Set LEDGERKIT_HOOK_SKIP=1 to disable. Set LEDGERKIT_HOOK_FULL=1 to also run tests.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path


def emit(payload: dict) -> None:
    sys.stdout.write(json.dumps(payload))
    sys.stdout.flush()


def main() -> int:
    raw = sys.stdin.read()
    try:
        data = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        emit({})
        return 0

    if os.environ.get("LEDGERKIT_HOOK_SKIP") == "1":
        emit({})
        return 0

    status = data.get("status")
    # Skip aborted / errored turns — nothing useful to verify.
    if status in {"aborted", "error"}:
        emit({})
        return 0

    loop_count = int(data.get("loop_count") or 0)
    if loop_count >= 3:
        emit({})
        return 0

    root = Path.cwd()
    cargo_home = Path.home() / ".cargo" / "bin"
    env = os.environ.copy()
    env["PATH"] = str(cargo_home) + os.pathsep + env.get("PATH", "")

    cargo_bin = shutil_which("cargo", env["PATH"])
    if not cargo_bin:
        # Fail open if toolchain missing in hook env.
        emit({})
        return 0

    commands: list[list[str]] = [
        [cargo_bin, "check", "--workspace"],
        [cargo_bin, "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ]
    if os.environ.get("LEDGERKIT_HOOK_FULL") == "1":
        commands.append([cargo_bin, "test", "--workspace"])

    failures: list[str] = []
    for cmd in commands:
        try:
            proc = subprocess.run(
                cmd,
                cwd=root,
                env=env,
                capture_output=True,
                text=True,
                timeout=240,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            failures.append(f"{' '.join(cmd)}\n{exc}")
            break
        if proc.returncode != 0:
            tail = (proc.stdout or "")[-2500:] + (proc.stderr or "")[-2500:]
            failures.append(f"$ {' '.join(cmd)}\nexit={proc.returncode}\n{tail}")
            break

    if not failures:
        emit({})
        return 0

    msg = (
        "LedgerKit quality gate failed after your turn. "
        "Fix the compile/clippy (or test) failures below, then stop. "
        "Do not expand scope.\n\n" + failures[0][:4000]
    )
    emit({"followup_message": msg})
    return 0


def shutil_which(cmd: str, path: str) -> str | None:
    for folder in path.split(os.pathsep):
        candidate = Path(folder) / (cmd + (".exe" if os.name == "nt" else ""))
        if candidate.is_file():
            return str(candidate)
    return None


if __name__ == "__main__":
    raise SystemExit(main())
