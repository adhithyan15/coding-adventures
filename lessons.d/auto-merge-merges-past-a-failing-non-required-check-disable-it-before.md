---
category: CI & GitHub Actions
---

# Auto-merge merges past a failing non-required check; disable it before pausing a PR

main's branch protection requires only "CI gate" and "Human Languages Books
gate". The "test262 parse level (grow-only list)" workflow is NOT required.
With squash auto-merge on, #17875 merged while that check was red, so the
grow-only list then failed on main.

I had decided to "pause" #17875 while its fix was under review, but I never
turned auto-merge off. Worse, the merge watch then only polled the other PRs.

Do this:
- To pause a PR, run `gh pr merge <n> --disable-auto`, and say so in the PR.
- A red non-required check doesn't block the merge, so treat any red check
  on an auto-merge PR as urgent.
- Keep every open PR in the merge watch until it is merged or closed, paused
  ones included.
