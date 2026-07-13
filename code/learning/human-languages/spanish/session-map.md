# Session Map — Part I (Chapters 1-4)

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

## Chapters 2-4 (sessions 11-28)

From session 11 onward, more concepts are due for review than the
2-4-per-session core block can hold at once — exactly what `HL00`'s
**bonus queue** exists for (Session Composition section): the core block
takes the *most*-due items (closest to falling out of the schedule
entirely), everything else rolls into the bonus queue for a longer drive
or gets picked up the next session it's still due. This section stops
hand-tracking every single N+15 far-future review individually (as
sessions 1-10 did, while the schedule was still small enough to verify by
hand) and instead states the rule once: **one new unit per session, in
this order**, with the two-or-so nearest-due reviews opening the core
block.

| Session | New unit | Nearest-due reviews (illustrative) |
|---|---|---|
| 11 | `U09` Numbers 11-30 | `U01` (N+7), `U05` (N+3) |
| 12 | `U10` Numbers 40-100 | `U02` (N+7), `U07` (N+3), `U09` (N+1) |
| 13 | `U11` Telling time | `U03` (N+7), `U10` (N+1) |
| 14 | `U12` Months & seasons | `U04` (N+7), `U09` (N+3), `U11` (N+1) |
| 15 | `U13` Question words | `U10` (N+3), `U12` (N+1) |
| 16 | `U14` Survival phrases | `U11` (N+3), `U13` (N+1) |
| 17 | *(practice-mix)* `U15` Chapter 2 capstone | `U09` (N+7), `U12` (N+3), `U14` (N+1) |
| 18 | `U16` Articles | `U13` (N+3), `U15`-reviewed items |
| 19 | `U17` Gender pattern & exceptions | `U10` (N+7), `U16` (N+1) |
| 20 | `U18` Adjective agreement | `U14` (N+3), `U17` (N+1) |
| 21 | `U19` Colors | `U11` (N+7), `U16` (N+3), `U18` (N+1) |
| 22 | `U20` Family vocabulary | `U17` (N+3), `U19` (N+1) |
| 23 | *(practice-mix)* `U21` Chapter 3 capstone | `U12` (N+7), `U18` (N+3), `U20` (N+1) |
| 24 | `U22` *Hay* | `U19` (N+3), `U21`-reviewed items |
| 25 | `U23` *Tener* | `U13` (N+7), `U16` (N+7), `U20` (N+3), `U22` (N+1) |
| 26 | `U24` Possessive adjectives | `U09` (N+15), `U17` (N+7), `U23` (N+1) |
| 27 | `U25` Negation | `U14` (N+7), `U18` (N+7), `U24` (N+1) |
| 28 | *(practice-mix)* `U26` Part I cumulative review | `U19` (N+7), `U25` (N+1), plus a deliberate free pick of whichever Part I concept the learner flags as weakest (`U26`'s own Wrap-up Recall asks directly) |

Part I is complete at session 28. Part II (Chapter 5, regular *-ar* verbs)
picks up at session 29, inheriting Chapter 4's still-outstanding reviews
the same mechanical way.
