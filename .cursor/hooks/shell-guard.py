#!/usr/bin/env python3
"""beforeShellExecution: deny destructive git / filesystem foot-guns."""
from __future__ import annotations

import json
import re
import sys


DENY_PATTERNS = [
    (r"git\s+push\s+[^\n]*--force[^\n]*\b(main|master)\b", "Force-push to main/master is blocked."),
    (r"git\s+push\s+[^\n]*-f[^\n]*\b(main|master)\b", "Force-push to main/master is blocked."),
    (r"git\s+reset\s+--hard", "git reset --hard is blocked; use safer recovery."),
    (r"git\s+clean\s+-fdx", "git clean -fdx is blocked (would delete fixtures/work)."),
    (r"\brm\s+-rf\s+/(?:\s|$)", "Recursive delete of filesystem root is blocked."),
    (r"Remove-Item\s+-Recurse\s+-Force\s+[A-Za-z]:\\?\s*$", "Recursive wipe of drive root is blocked."),
]

ASK_PATTERNS = [
    (r"git\s+push\s+[^\n]*--force", "Force-push requested — confirm intentionally."),
    (r"gh\s+repo\s+delete", "Repo delete requested — confirm intentionally."),
]


def main() -> int:
    raw = sys.stdin.read()
    try:
        data = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        sys.stdout.write(json.dumps({"permission": "allow"}))
        return 0

    command = data.get("command") or ""
    for pattern, message in DENY_PATTERNS:
        if re.search(pattern, command, flags=re.IGNORECASE):
            sys.stdout.write(
                json.dumps(
                    {
                        "permission": "deny",
                        "user_message": message,
                        "agent_message": message
                        + " Choose a non-destructive alternative.",
                    }
                )
            )
            return 0

    for pattern, message in ASK_PATTERNS:
        if re.search(pattern, command, flags=re.IGNORECASE):
            sys.stdout.write(
                json.dumps(
                    {
                        "permission": "ask",
                        "user_message": message,
                        "agent_message": message,
                    }
                )
            )
            return 0

    sys.stdout.write(json.dumps({"permission": "allow"}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
