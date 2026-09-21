---
category: Testing & coverage
---

# A test gate that trusts a value from a data file must also test the rule that the value is armed, or the guard is one edit from being off with every test green

Four consecutive security-review rounds on the same file found the same defect,
one level up each time. Each round's fix added a field to a hand-edited data
file, and that field became the next round's attack surface.

The file was `tests/ladder/divergences.json`, the ledger of known gaps for
`closurec`'s differential test ladder, read by the gate in `tests/ladder.rs`.

| Round | What was added | How it was disarmed |
|---|---|---|
| 1 | — parity was `actual == expected && code == 0` | a rung the oracle **refused** has an empty expected output, so an empty actual scored as agreement and the harness said "NOW MATCHES UPSTREAM — delete its entry" |
| 2 | `closurec_stderr_starts_with`, pinning which stage a rung fails at | optional and unvalidated: delete the key, set it to `null`, or set it to `""` — and `starts_with("")` is vacuously true |
| 3 | `upstream_exit`, added by round 1's fix | corroborated by nothing. One token, non-zero, and that rung's staleness detection is off forever |
| 4 | a test pinning the exempt cohort, added by round 3's fix | pinned a **count** while its own doc comment claimed an **identity**, so the cohort could be swapped |

Every round's report reduced to the same sentence: *the guard exists, but
nothing pins that it is armed.*

**The structural cause.** The data file is **input** to the gate, not part of it.
Its fields split two ways, and only one way is safe:

- *Corroborated* — cross-checked against other committed evidence on every run.
  One field was: the ledger's copy of the oracle's output, compared against the
  committed fixture bytes. Lying about it fails immediately.
- *Self-declared* — nothing outside the file produces them. Every rule guarding
  these is a rule about internal consistency **between self-declared values**,
  which is strictly weaker than agreement with evidence.

Round 3 was the sharp case: `upstream_exit` looked exactly as trustworthy as the
field beside it, and was not, because the oracle's *exit status* was recorded in
no fixture while its *output* was.

**What to do.**

1. Before adding a field to a data file a gate trusts, ask what corroborates it.
   If the answer is "nothing", either capture the evidence too, or accept that
   you now owe a rule pinning its legal values — and a test of that rule.
2. Extract the rules into a pure predicate returning `Option<complaint>` rather
   than inline `assert!`s. Inline assertions run only against data that satisfies
   them, so they are exercised in the passing direction and nowhere else.
3. Negative-test each rule in its rejecting direction, **and** assert the
   baselines are still accepted — otherwise the rejections are satisfied by a
   predicate that rejects everything.
4. Pin cohorts by **name**, never by size. A count is satisfied by any swap.
5. Run each attack against the real data file, then restore it byte-identical.
   A guard verified only against a synthetic fixture has not been shown to bind
   the thing it is guarding.

**And the durable fix is upstream of all five.** Validation layers over
self-declaration leak at the next seam; four rounds is the empirical argument.
Derive the file from captured runs so the self-declared class shrinks to the
fields that carry no gate weight — commentary like a reason and a tracking link.
That is filed as its own work item rather than patched a fifth time.
