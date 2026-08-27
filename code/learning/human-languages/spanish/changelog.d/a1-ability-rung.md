# The ability rung — `tener`, `poder` and `saber` reach A1

HL23 §9.3 left `tener` and `poder` at A2 deliberately, and said what would have
to exist before they could move: **a rung of their own**. This builds it.

`SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO`, stage A1, strand FUNCTION:

> **I can say what I have and what I am able to do.**

carrying `VERB-HAVE` (`tener`) and `VERB-CAN` (`poder`) — the DELE A1 sitting's
most-missed verb and its fourth.

The node is FUNCTION, not LEXICON, because it exists to close two enumerated
PCIC A1 function points that stood **unmapped** in `exam-inventory-es-a1.json`:
`A1-F2-16` (ask about ability) and `A1-F2-17` (express ability).

## Closing those two points meant authoring a verb

Both points carried a note saying the PCIC exponent is **`saber`** plus an
infinitive, and that substituting `poder` "would be a different structure".
Pointing them at `poder` anyway would have moved a coverage number by
overriding a judgement somebody had already made and written down.

So **chapter 389 authors `saber`** — one lesson, introducing `ES-LEX-SABER` and
`ES-GRAMMAR-SABER-INFINITIVO`, teaching *sé leer* against *puedo leer* (the
learned skill against the present opportunity), and built on the etymology that
makes the word memorable: Latin *sapere* meant **to taste**, which is why *esta
sopa sabe a ajo* is the same verb, why something with no *sapor* is *insipid*,
and why *Homo sapiens* read strictly names us the tasting animal rather than the
wise one.

`saber` appears on **no** mock item's `requires:` line. It buys nothing on the
exam. It was authored because the rung's stated justification was those two
points, and an undischarged justification is decoration.

## What it cost

HL23 §9.1 priced `VERB-HAVE` at 3 lesson migrations plus two
`misplaced-shared-realization` repairs, and said it would **empty
`GE-PATH-018`**. It does not, and §10.2 records why: a concept can move without
its lesson moving, if the lesson's whole **segment** is retargeted instead.
Level derives from the segment's `spine_node`, and a segment's path position is
independent of the stage its node declares.

| track | what happened |
|---|---|
| spanish | `ES-PATH-030-TENER` split — `ES-PATH-A1-TENER` (A1) and `ES-PATH-030-TENER-PLURAL` (A2, keeps `tenemos`/`tienen`) |
| spanish | `ES-PATH-030-ABILITY-CH11` retargeted |
| french | `FR-PATH-016` split — `FR-PATH-A1-AVOIR` (A1); `FR-C14-age` stays at A2 |
| german | `GE-PATH-018` retargeted; **not emptied** |

Plus 23 tracks × 2 nodes of realization ledger, recomputed from the same
expressions `curriculum.ts` validates against, never hand-edited.

The `tener` plural paradigm stays at A2 on purpose: it is morphology, and this
node's `canDo` claims saying what you have, not conjugating it.

## The counts

| | before | after |
|---|---|---|
| headwords ≤ **pre-A1** | 304 | **304** |
| headwords ≤ **A1** | 621 | **624** |
| verbs ≤ **A1** | 44 | **47** |
| PCIC A1 points covered | 223 / 273 | **225 / 273** |
| points with no atom | 50 | **48** |

**pre-A1 does not move.** Verified from a from-scratch `dist/` before and after.

`SPINE-SAY-WHAT-I-DO` goes **35 → 33** concepts. That pin may only ever fall.

No ceiling was raised. Coverage `percent` stays at 82 — two points in 273 is
0.7pp and rounds away, which is why `covered` and `unmapped` are pinned beside
it rather than the percentage alone.

## Re-sat, and still `NO APTO`

| | Grupo 1 (needs 30,00) | Grupo 2 (needs 30,00) | |
|---|---|---|---|
| mock 1, before → after | 4,00 → **4,00** / 50 | 11,58 → **12,58** / 50 | NO APTO |
| mock 2, before → after | 5,17 → **5,17** / 50 | 11,67 → **13,33** / 50 | NO APTO |

Objective items failed: 83 → **82** of 100.

**Grupo 1 does not move by one point on either mock.** Releasing the two
most-missed verbs in the corpus bought a single objective item, in *auditiva*.
Every reading item that wanted `tener` or `poder` wanted a noun too — #2 also
wanted `terraza` and `habitación`, #13 also wanted `aeropuerto` and `necesitar`,
#23 also wanted `ordenador` and `internet`.

The verb famine and the noun famine are **multiplicative on the same items**.
HL23 §9.4 said verbs were necessary and not sufficient; §10.5 measures how
insufficient, and the consequence is that the remaining verb backlog should be
sequenced *with* the noun tranches rather than before them.
