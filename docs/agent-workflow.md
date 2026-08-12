# Agent & slash-command workflow

What is **automatic** vs what **you type with `/`**.

## Already set up (automatic)

| Mechanism | When it runs | What it does |
|-----------|--------------|--------------|
| **Project rules** (`.cursor/rules/*.mdc`) | Every Agent chat in this repo | Enforces pillars, phase scope, Rust/adapter/testing norms |
| **`stop` → quality-gate** | After each Agent turn completes | `cargo check` + `clippy -D warnings`; auto-asks agent to fix (max 3 loops) |
| **`beforeShellExecution` → shell-guard** | Before shell commands | Blocks force-push to main/master, `reset --hard`, `clean -fdx`, etc. |
| **`beforeSubmitPrompt` → prompt-guard** | When you hit send | Blocks prompts that look like pasted API/git tokens |
| **GitHub Actions CI** | On push/PR | fmt + clippy + test + demo smoke |

### Hook toggles

```powershell
# Skip quality gate for a session (docs-only chats)
$env:LEDGERKIT_HOOK_SKIP = "1"

# Also run full `cargo test --workspace` on stop (slower, use near phase end)
$env:LEDGERKIT_HOOK_FULL = "1"
```

If hooks don’t fire: open **Cursor Settings → Hooks**, ensure project hooks are enabled, and that the workspace is **trusted**. Restart Cursor once after first pull.

---

## You run these with `/` (manual — highest quality)

These are **not** always-on. Invoke them at the moments below.

### `/review-bugbot` (or Bugbot review)

**When:** End of **every phase** (2–7), **on the phase PR/branch** (not after merging to `master`).

**Why:** Defect-first review of the change set before you merge. A branch vs `master` diff is required; reviews on `master` itself see an empty diff.

```text
/review-bugbot
```

Say: review branch changes for LedgerKit Phase N.

---

### `/review-security`

**When:**
- After **Phase 3** (import paths + artifact storage)
- After **Phase 5** (report file writes)
- Always before calling Phase 7 “done”

**Why:** Path traversal, safe file handling, no telemetry, checksum hygiene.

---

### `/split-to-prs`

**When:** A phase branch has mixed concerns (e.g. schema + CLI + docs + fixtures in one blob) and you want a **clean portfolio history**.

**Why:** Reviewable PRs beat one giant commit for interviews/GitHub.

---

### `/autopilot`

**When:** You have an **open PR** and want CI + review comments kept merge-ready.

**Why:** Triages comments / fixes clear CI failures in a loop. Skip until you open PRs.

---

### `/loop`

**When:** Hardening / flaky fixes near **Phase 7** (e.g. “run tests, fix failures, repeat every N minutes”).

**Why:** Unattended grind. Don’t use during early design/scaffold — it burns turns.

---

### `/create-rule` / `/create-hook`

**When:** Standards change (new invariant, new adapter policy). Prefer editing files under `.cursor/rules/` and `.cursor/hooks/` directly; use these if you want guided creation.

**Already done for v1** — only re-run if you need new policies.

---

### Canvas

**When:** **Phase 7** evaluation (dedup precision/recall, reconcile success rates, benchmarks).

**Why:** Metrics belong in a visual board, not a markdown dump. Optional until then.

---

### Skip for LedgerKit

| Slash / MCP | Why skip |
|-------------|----------|
| `/sdk` | Automates Cursor agents elsewhere — not how we build this crate |
| Cloudflare / Workers skills | Wrong stack |
| Browser MCP | No v1 UI to drive |

---

## Per-phase checklist

| Phase | Auto (hooks/rules/CI) | You run manually (agent will remind) |
|-------|------------------------|--------------------------------------|
| 2 Ledger core | rules + quality gate | `/review-bugbot` at end — **wait before Phase 3** |
| 3 Import | same | `/review-bugbot` + `/review-security` — **wait before Phase 4** |
| 4 Dedupe/rules | same | `/review-bugbot` — **wait before Phase 5** |
| 5 Reconcile/why | same | `/review-bugbot` + `/review-security` — **wait before Phase 6** |
| 6 Export/demo | same | `/review-bugbot`; `/split-to-prs` if messy |
| 7 Hardening | set `LEDGERKIT_HOOK_FULL=1` | Bugbot + Security + optional Canvas + `/loop` if grinding |

**Commits:** agent commits logical chunks during the phase, not only at the end.

**PRs (Phase 4+):** work on `phase-N-<name>`, push, open a PR, *then* run `/` reviews. Merge to `master` only after review.

---

## Suggested chat prompts (copy/paste)

**Start Phase 2:**
> Implement Phase 2 only: persist postings + event append/replay + real verify. Do not start Phase 3.

**End of phase:**
> Phase N is feature-complete. Run `/review-bugbot` on branch changes and fix all actionable findings.

**Before thesis freeze:**
> Run `/review-security` on uncommitted + branch changes. Fix path/artifact issues only.
