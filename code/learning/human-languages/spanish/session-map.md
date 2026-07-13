# Session Map — Part 0 & Chapter 1

How Part 0 and Chapter 1's units compose into car-ride sessions, and the
worked spaced-repetition schedule behind it. Mechanics are defined in
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) — this
file is the concrete worked example.

Reminder: reviews are scheduled in **session-counts**, not calendar days. A
`new`-type concept introduced in session *N* is due for review at sessions
*N+1*, *N+3*, *N+7*, *N+15*. Part 0 (pronunciation) and `morphology` units
get **no** dedicated review cycle — they're reinforced implicitly by every
subsequent unit's Spanish text, so a separate schedule would be redundant
(see `HL00`'s Part 0 section). This session map uses one session per day
(sessions 1-10); from Chapter 2 onward a day may hold two sessions
(there-and-back commute) — the counting rule doesn't change, only how fast
session numbers tick up.

## Ramp-up exception (sessions 1-8)

The full three-part core block (due reviews → one new unit → a dedicated
practice-mix) only makes sense once there's enough vocabulary to recombine
meaningfully. Through session 8, each new unit's own **Guided Practice**
section already provides that local recombination. Dedicated `practice-mix`
unit files start at session 9, once enough vocabulary exists to draw on. See
`HL00`'s Session Composition section.

## Session-by-session

| Session | Reviews due | New unit | Morphology | Practice-mix |
|---|---|---|---|---|
| 1 | — (bootstrap) | `ES-P0-U00A` Vowel sounds | — | — |
| 2 | — | `ES-P0-U00B` Consonants | — | — |
| 3 | — | `ES-P0-U00C` Stress & accents | — | — |
| 4 | — | `ES-P0-U01` Greetings | — | — |
| 5 | `ES-P0-U01` (N+1) | `ES-P0-U02` Subject pronouns | `ES-P0-M01` (*clam-*) | — |
| 6 | `ES-P0-U02` (N+1) | `ES-P0-U03` *Ser* | — | — |
| 7 | `ES-P0-U01` (N+3), `ES-P0-U03` (N+1) | `ES-P0-U04` Numbers 0-10 | — | — |
| 8 | `ES-P0-U02` (N+3), `ES-P0-U04` (N+1) | `ES-P0-U05` *Estar* | — | — |
| 9 | `ES-P0-U03` (N+3), `ES-P0-U05` (N+1) | `ES-P0-U07` Days of the week | — | `ES-P0-U06` *Ser* vs *estar* |
| 10 | `ES-P0-U04` (N+3), `ES-P0-U07` (N+1) | — | — | `ES-P0-U08` Introduce yourself (capstone) |

`ES-P0-M01` sits at session 5 because it depends on `ES-P0-U01` (it extends
the *llamar* ← *clamare* note from Greetings) — the earliest session where
that dependency is satisfied and there's room in the core block alongside
that day's new unit.

## On the review units themselves

`ES-P0-R01` and `ES-P0-R02` (reviewing `U01` and `U02` respectively) are
fully authored as concrete examples of "fresh combination, not verbatim
repeat." The remaining due reviews in the table above follow the same
pattern and are generated the same way when a session reaches them, rather
than each being hand-authored ahead of time — same rationale as the roadmap
itself being a skeleton beyond what's currently authored.

## Carrying into Chapter 2

By the end of session 10, every Chapter 1 concept still has reviews
outstanding (sorted by when they next come due):

- `ES-P0-U01` — next due at session 11 (N+7)
- `ES-P0-U05` — next due at session 11 (N+3)
- `ES-P0-U02` — next due at session 12 (N+7)
- `ES-P0-U07` — next due at session 12 (N+3)
- `ES-P0-U03` — next due at session 13 (N+7)
- `ES-P0-U04` — next due at session 14 (N+7)

Chapter 2's session map (not yet written) picks these up alongside its own
new units (numbers 11-100, telling time, months/seasons, question words,
survival phrases — see `roadmap.md`).
