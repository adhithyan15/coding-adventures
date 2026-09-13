---
category: BUILD files & dependency management
---

# Do not use `mise exec --` in BUILD files

CI runners install language tools directly into PATH via `actions/setup-*`; they do not have mise. BUILDs that prefix `mise exec --` (or hardcode `/Users/adhithya/.local/bin/mise`) fail with `mise: not found`. Call `cargo`, `npm`, `python`, `go`, `bundle` directly — mise's local shims handle dispatch transparently. Re-learned during rebases; conflict resolution that picks the branch's `mise exec`-prefixed BUILD over main's bare-command BUILD reintroduces this break. After rebase: `git diff origin/main...HEAD -- '**/BUILD'` to verify only intentional BUILD diffs remain.
