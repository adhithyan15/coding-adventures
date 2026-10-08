---
category: Testing & coverage
---

# Turn off auto-merge before pushing a reconciliation commit to a PR that is about to merge

PR #16968 added a raw-cue gate to `check:books`. While it was in review,
Punjabi Chapter 163 (#16961) landed on main with old-renderer output, and a
review comment asked for it to be regenerated. The reconciliation commit was
pushed while auto-merge was still on and every check on the old head was
already green: GitHub merged the old head three minutes before the push
arrived, so main carried the new gate *and* a chapter that fails it. A
follow-up PR (#16982) had to regenerate the straggler.

Next time: before pushing a fix-up to a PR whose checks are green (or nearly
so) with auto-merge enabled, disable auto-merge first (or re-fetch the PR and
confirm it is still open), push, and re-enable it once the new head is green.
A generator change that adds a gate should also regenerate against the very
latest main immediately before its last push.
