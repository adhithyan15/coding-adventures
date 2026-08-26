## HL23 — the first four exam verbs reach A1, and the mocks are re-sat

`sitting-2026-08-26.md` produced the finding this entry acts on: Spanish is
green on all five HL09 §3.1 A1 criteria and still returns `NO APTO` on both
DELE A1 mocks, because **62 of 86 failed objective items involve a
high-frequency verb that is taught and then staged above A1**.

`VERB-DO-MAKE`, `VERB-BUY`, `VERB-OPEN` and `VERB-CLOSE` now sit on
`SPINE-NAME-EVERYDAY-ACTIONS` (A1) instead of `SPINE-SAY-WHAT-I-DO` (A2), at a
measured price of **11 lesson migrations across 5 tracks**. Spanish goes 617 →
621 headwords and 40 → 44 verbs at or below A1; pre-A1 stays at 304, untouched.

Three things this leaves for the next slices, recorded because they are the
reason the slice stops where it does.

**HL23 §8.2's price table is incomplete, and the omissions are the expensive
question.** It does not price `VERB-HAVE`, `VERB-DO-MAKE`, `VERB-BE`,
`VERB-SAY`, `VERB-HEAR` or `VERB-LEARN`. Measured here: `VERB-DO-MAKE` 2
foreign lessons (german, hindi), `VERB-SAY` 1 (persian), `VERB-LEARN` 1
(german), `VERB-HAVE` 3 (french, german, spanish) — and `VERB-HAVE` additionally
**empties `GE-PATH-018`** and carries two lessons with no explicit `spine_node`
(`FR-C14-avoir`, `GE-C14-haben`), the same `misplaced-shared-realization` shape
§8.2 flags for `VERB-LIVE`. `VERB-BE` is 8 lessons across 8 tracks and empties
`GE-PATH-021`.

**`poder` and `tener` need rungs that do not exist yet.** They are two of the
five most-missed verbs and both are cheap-ish to release, but neither is an
"everyday action", and `SPINE-NAME-EVERYDAY-ACTIONS`'s `canDo` cannot be
stretched over them without becoming the compound capability statement HL23 §5
rejected Option B for producing. Expressing ability is its own A1 function —
PCIC `A1-F2-16` and `A1-F2-17`, both currently **unmapped** in
`exam-inventory-es-a1.json` — so the rung is justified on the syllabus's terms,
not invented to hold a verb.

**Verbs alone will not pass this exam and it is worth saying so before the next
slice.** The sitting's own counterfactual granted *all 34* high-frequency verbs
and still failed both groups in both mocks; the ~100 everyday nouns, weighted to
commerce and price, transport, work, education and internet/email, are the other
half of the minimum. `euro`, `trabajo` (the noun) and `planta` are absent from
all 1 030 lessons at every level.

One mechanical lesson, cheap to relearn the hard way. **Path order is filename
order, and filename order carries the prerequisite contract.** Splitting a
segment to release a concept can therefore fail `curriculum-prerequisite-order`
depending on which side the new segment lands. Italian's `IT-PATH-024` needed a
**three-way** split rather than a two-way one, because `comprare` requires
`portare` and is required by `aspettare` and `incontrare`, all three in the
segment it was leaving.
