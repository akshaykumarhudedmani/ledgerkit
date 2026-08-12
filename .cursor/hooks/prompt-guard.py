#!/usr/bin/env python3
"""beforeSubmitPrompt: block prompts that look like they contain secrets."""
from __future__ import annotations

import json
import re
import sys


SECRETISH = [
    r"AKIA[0-9A-Z]{16}",
    r"ghp_[A-Za-z0-9]{20,}",
    r"gho_[A-Za-z0-9]{20,}",
    r"github_pat_[A-Za-z0-9_]{20,}",
    r"cursor_[A-Za-z0-9]{20,}",
    r"-----BEGIN (RSA |OPENSSH )?PRIVATE KEY-----",
]


def main() -> int:
    raw = sys.stdin.read()
    try:
        data = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        sys.stdout.write(json.dumps({"continue": True}))
        return 0

    prompt = data.get("prompt") or ""
    for pattern in SECRETISH:
        if re.search(pattern, prompt):
            sys.stdout.write(
                json.dumps(
                    {
                        "continue": False,
                        "user_message": (
                            "Blocked: prompt looks like it contains a secret/token. "
                            "Use env vars or `gh auth` instead of pasting keys into chat."
                        ),
                    }
                )
            )
            return 0

    sys.stdout.write(json.dumps({"continue": True}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
