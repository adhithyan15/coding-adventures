# The first four exam verbs come down from A2 to A1, and the mocks are re-sat

`sitting-2026-08-26.md` found that Spanish passes every HL09 §3.1 criterion for
A1 and then returns **`NO APTO`** on both DELE A1 mocks, with **62 of 86 failed
objective items involving a missing high-frequency verb**. Its diagnosis was
that the corpus is not too small — 689 headwords against a ~600 target — but
**composed and ordered wrong**: the verbs the exam asks for are taught and then
parked above A1 by the shared spine.

This is the first slice of the fix, and it re-sits the mocks to say what the
slice bought.

## What moved

Four concepts leave `SPINE-SAY-WHAT-I-DO` (A2, GRAMMAR) for
`SPINE-NAME-EVERYDAY-ACTIONS` (A1, LEXICON):

| concept | Spanish | mock items involving it |
|---|---|---|
| `VERB-DO-MAKE` | `hacer` | 9 |
| `VERB-BUY` | `comprar` | 5 |
| `VERB-OPEN` | `abrir` | 4 |
| `VERB-CLOSE` | `cerrar` | 3 |

**No `canDo` was touched.** The destination node already says *"I can name
common everyday actions"*, and doing, buying, opening and closing are common
everyday actions. That constraint is why `poder` and `tener` are **not** in this
slice: being able to do something is not an action, and having something is not
an action either. Widening one capability statement to fit a verb in is the
defect `HL23` exists to prevent, and a later slice will mint the rungs that
honestly cover them.

## What that cost

HL23 §8.1's rule sets the price: a concept cannot leave a node unless every
lesson realizing it, in every track, moves with it or is already inside an
extension. Measured, then paid:

| concept | lessons re-homed | tracks |
|---|---|---|
| `VERB-DO-MAKE` | 3 | german, hindi, spanish |
| `VERB-BUY` | 2 | italian, spanish |
| `VERB-OPEN` | 3 | french, german, spanish |
| `VERB-CLOSE` | 3 | french, german, spanish |

**11 lesson migrations across 5 tracks**, plus every track's realization ledger
for both nodes — 23 tracks × 2 nodes, recomputed rather than hand-edited.

Two findings worth keeping. `VERB-DO-MAKE` is **absent from HL23 §8.2's price
table**, and it is the joint-highest-value verb on the exam; it costs two
foreign lessons, which puts it among the cheapest rows in the table it is
missing from. And Italian's `IT-PATH-024` could not be split in two: `comprare`
requires `portare` and is required by `aspettare` and `incontrare`, all in the
same segment, so it needed a **three-way** split to keep
`curriculum-prerequisite-order` satisfied. Path order is filename order, and
filename order carries the prerequisite contract.

## The numbers

| | before | after |
|---|---|---|
| headwords at or below **pre-A1** | 304 | **304** |
| headwords at or below **A1** | 617 | **621** |
| verbs at or below **A1** | 40 | **44** |

pre-A1 does not move, and that is deliberate: 304 against a floor of 300 is four
headwords of slack for every future slice, and this work moves A2 → A1 only.
Verified from a from-scratch `dist/`.

## The re-sat scores

Both mocks re-sat with the same harness, on the same rule. **Both still return
`NO APTO`** — which the sitting predicted: its bundle C, *all 34 high-frequency
verbs*, still failed both groups in both mocks. Verbs alone were never going to
be enough; the ~100 everyday nouns are the other half.

| | Grupo 1 (needs 30,00) | Grupo 2 (needs 30,00) | global |
|---|---|---|---|
| **mock 1** before | 4,00 / 50 | 11,58 / 50 | NO APTO |
| **mock 1** after | **5,00 / 50** | **11,58 / 50** | NO APTO |
| **mock 2** before | 0,00 / 50 | 5,00 / 50 | NO APTO |
| **mock 2** after | **5,17 / 50** | **11,67 / 50** | NO APTO |

Per paper:

| paper | mock 1 before → after | mock 2 before → after |
|---|---|---|
| Comprensión de lectura | 4 → **5** / 25 | 0 → **1** / 25 |
| Comprensión auditiva | 7 → **7** / 25 | 5 → **5** / 25 |
| Expresión e interacción escritas | 0,00 → **0,00** / 25 | 0,00 → **4,17** / 25 |
| Expresión e interacción orales | 4,58 → **4,58** / 25 | 0,00 → **6,67** / 25 |

Objective items failed: **84 → 82** of 100.

Mock 2 moves and mock 1 barely does. That is informative rather than
disappointing: mock 1's items tend to need a missing verb *and* a missing noun,
so releasing the verb alone does not flip them. Mock 2's production tareas were
one lexeme short of a band in two places, and `hacer` supplied it.

## On the harness

The scoring scripts of `sitting-2026-08-26.md` §8 were scratchpad artifacts and
were not committed, so they were rebuilt from that document's description and
**calibrated against its published result before being used**. The
reconstruction is validated on three pinned figures it reproduces exactly — 689
headwords and 756 lessons at or below A1, and 810 headwords / 78 verbs at or
below A2 — and on the project's own **40-verb** count.

It reproduces six of the eight published paper scores exactly, including every
production paper and both of mock 2's group totals. It differs on two, by
**exactly one objective item each**, both in the **generous** direction: mock 1
Comprensión de lectura 4 where the original recorded 3, and mock 2 Comprensión
auditiva 5 where the original recorded 4. So the baseline in the table above is
the reconstruction's own baseline, not the published one, and before/after are
therefore measured on the same instrument. The residual is stated rather than
tuned away, and it moves results *towards* passing, never away — which is the
safe direction for a change that is trying to demonstrate an improvement.
