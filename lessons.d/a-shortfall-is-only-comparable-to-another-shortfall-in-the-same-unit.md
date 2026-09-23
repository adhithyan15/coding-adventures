---
category: Repo policy / workflow reminders
---

# A shortfall is only comparable to another shortfall in the same unit, so never sort or rank across criteria

The HL09 §3.1 level gate reports a `shortfall` per blocker, and the number means
something different for each criterion:

| criterion | unit | observed range |
|---|---|---|
| vocabulary | headwords | 79 – 257 |
| reinforcement | atoms | 13 – 78 |
| verb-vocabulary | verbs | 1 – 29 |
| atom-budget | lessons | 1 – 5 |
| spine-nodes | nodes | 1 |

`report.ts` chose which blocker to display with
`sort((a, b) => b.shortfall - a.shortfall)[0]` and called the winner the worst.
Because headword targets are in the hundreds and verb targets are ~5,
**`vocabulary` won on all 23 tracks forever**, and `verb-vocabulary` and
`atom-budget` could never be printed at all. Two criteria blocking 16 and 8
tracks stayed out of every plan the project made for months.

**Symptom to recognise:** a ranked list where one category always wins. That is
rarely a fact about the data; it is usually a fact about the units.

Three rules that follow:

- **Rank only within a criterion**, or rank on something unit-free. The fix here
  orders tracks by *how many criteria are left*, which compares cleanly.
- **When a row must show several, sort them by NAME.** Any ordering over the
  numbers reintroduces the bug in a new place.
- **A gate that is a conjunction must be reported as one.** §3.1 passes only
  when all five criteria pass, so showing the first failing one presents a
  necessary condition as if it were sufficient, and every estimate downstream
  becomes a lower bound on a number nobody has seen.

The rule was already written in the test's own comment — *"would have ranked a
track needing one spine node above one needing 79 headwords"* — and had been
applied to the row ordering but not to the selection, one line away in the same
expression. **When you write a comment stating an invariant, check every branch
of the surrounding expression obeys it, not the one you were looking at.**

And when fixing it: assert **set equality against the source data**, not shape
properties of the output. The old test checked that criterion groups were
contiguous and shortfalls ascended within a group — both true of the broken
renderer, and both satisfiable while 41 of 64 blockers went unmentioned.
