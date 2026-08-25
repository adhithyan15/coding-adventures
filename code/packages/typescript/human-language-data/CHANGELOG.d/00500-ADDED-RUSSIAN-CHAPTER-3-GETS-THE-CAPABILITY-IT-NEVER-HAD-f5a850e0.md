### Added — Russian chapter 3 gets the capability it never had

- It was the **only generated chapter with no opening**, because `russian/chapters.json`
  had entries for chapters 2, 4 and 5 and skipped 3. The book could not describe a
  chapter the ledger did not know about.
- `canDo` names what the chapter title already promised — *"Six Verbs, and the One You
  Never Say"* — which is `быть`: Russian drops the present-tense copula, so the verb is
  learned in order **not** to say it.
- The payoff is `RU-C03-idti`, and every atom in its `assesses` list is taken from that
  lesson's own block directives. None is invented, and none is borrowed from a lesson
  that does not assess it.
- **It is an honest, declared regression on one metric.** The chapter has no terminal
  practice lesson, so the payoff is the last lesson by sequence and assesses 6 of 18
  atoms — below the 0.5 representativeness floor, taking the corpus 24 → **25**. That is
  recorded in the chapter's own non-printed `payoff.note` and pinned with its reason. A
  chapter with an opening and a thin payoff is better than one with neither; HL-C25
  exists to author real payoff lessons.
- Corpus: declared chapters 317 → **318**, chapters without a capability 99 → **98**.
  Every generated chapter now has an opening, so the chapter-intro test's by-name
  exception list is empty — and it is still a by-name check, not a count, so a future
  chapter without a capability is named rather than silently tolerated.

