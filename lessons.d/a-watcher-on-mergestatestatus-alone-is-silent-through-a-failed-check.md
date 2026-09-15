---
category: CI & GitHub Actions
---

# A watcher on mergeStateStatus alone is silent through a failed check

2026-09-13.

A PR watcher polled `mergeStateStatus` and printed only on a change. It sat
reporting `BLOCKED` for a long stretch while two checks were **red**, and
nothing said so.

`BLOCKED` conflates two states that call for opposite actions:

- checks still running — wait, do nothing;
- checks failed — act, because auto-merge will never fire on its own.

A watcher that cannot tell them apart reports the same word for both, and the
quiet reading is the default one. The failure was noticed only when the
watcher's own window EXPIRED and printed "check by hand" — that is, by
accident.

This is the complement of the rule that a babysitter must poll
`mergeStateStatus` at all, because a DIRTY PR keeps `state == OPEN` forever.
Both are true, and none of the three signals is sufficient alone:

| signal | blind to |
|---|---|
| `state` | DIRTY and BLOCKED — stays OPEN through both |
| `mergeStateStatus` | failed vs pending — both read `BLOCKED` |
| `gh pr checks` | merge conflicts — a DIRTY PR can be all-green |

**How to apply.** Watch the thing that decides the outcome, not the thing that
summarises it: `state` for merged/closed, `mergeStateStatus` for DIRTY, and a
failing-check tally from `gh pr checks`. And when a watcher reports a steady
value, ask what that value would look like if the thing being watched had
already failed. If the answer is "the same", the watcher is decorative.

Related: an unasserted measurement is not a gate, and silence is not success.
