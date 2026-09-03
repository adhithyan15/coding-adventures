## HL-C316 — `practises` is checked against the prerequisite closure, not against reading order

Adding a cross-chapter retrieval to Telugu needed no other change. The identical
edit to Sanskrit was rejected by `validateCurriculum` with 28 errors of two kinds:

    SA-C21-dance: practised atom 'SA-LEX-C19-NUM-01' is not yet available
    SA-C21-dance: block 'Guided Practice' assesses 'SA-LEX-C19-NUM-01' before it is available

The atom is taught in chapter 19 and practised in chapter 21. By reading order
it is available and then some. The check does not read reading order. It walks
`prerequisites` transitively (`prerequisiteKnowledge` in `curriculum.ts`) and
asks whether the atom is introduced anywhere in *that* closure.

Telugu's lessons form a single chain — every lesson names the previous one — so
its closure happens to contain everything earlier and the retrieval validated by
accident. Sanskrit's does not: chapter 21 hangs off chapter 20 in places and
skips 19 entirely, so a perfectly ordered retrieval was invisible to the
validator.

**The lesson generalises past this fix.** Any edit that makes a lesson practise
something from further back than it already did has to add the source lesson to
`prerequisites`, and whether it needs to depends on a graph nobody looks at
while authoring. It is also the correct claim on its own terms — a lesson that
retrieves a word does depend on that word having been taught — and the corpus
already spells it that way where a reach-back was authored by hand:
`TE-C75-today` lists `TE-C40-bhojanam` among its prerequisites for exactly this
reason.

Two consequences worth carrying forward.

**Do not read a green Telugu run as evidence for another track.** The tracks
differ in a property — prerequisite-graph shape — that nothing in the report
surfaces and that no author chose deliberately. Run the whole suite per track;
`tests/integration.test.ts` is the one that catches this, and it is a single
assertion comparing an error array to `[]`, so it costs one failing test file
and is easy to miss if only the `Tests` line is read.

**A backward prerequisite is free; a forward one is a defect.** `continuity.ts`
counts `forwardPrerequisites` separately, so adding an earlier lesson cannot
make that number worse. Sanskrit's stayed at 0 across 156 edited lessons.
