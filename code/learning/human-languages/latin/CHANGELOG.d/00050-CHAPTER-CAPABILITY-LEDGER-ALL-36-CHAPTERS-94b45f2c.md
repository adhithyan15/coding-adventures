## Chapter capability ledger — all 36 chapters

- Adds [`chapters.json`](./chapters.json), the track's
  [HL05](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
  capability ledger: one entry per chapter carrying a first-person `canDo`, the
  shared spine nodes the chapter realises, and a `payoff` naming the lesson that
  delivers the chapter's usable result.
- Covers **all 36 chapters with no skips**. Every Latin chapter's terminal lesson
  is already schema v2 with a non-empty `practises.knowledge`, so nothing here is
  a placeholder and nothing was omitted as schema-v1 debt.
- Copies each `payoff.assesses` verbatim from its payoff lesson's own
  `practises.knowledge`, so the ledger cannot claim an atom the lesson does not
  actually practise. Every chapter's payoff exercises **100%** of the atoms its
  own chapter introduces, clearing the 0.5 representativeness threshold in
  `core/chapter-policy.json` with room to spare.
- Records two honest facts about the track rather than papering over them.
  Chapter 1 is the only chapter with a terminal `practice` lesson
  (`LA-C01-practice`); every other chapter's payoff is its last lesson by
  `sequence`, which is where that chapter's recombination and wrap-up recall live.
  Chapter 1 is also the only Latin chapter with no `core/book-generation.json`
  target, so its `title` and `label` come from its hand-written
  `book/chapters/ch01-greetings.tex`.
- Leaves lessons, curriculum, spine, and book output untouched: this is a new
  capability layer above the corpus, not a revision of it.

