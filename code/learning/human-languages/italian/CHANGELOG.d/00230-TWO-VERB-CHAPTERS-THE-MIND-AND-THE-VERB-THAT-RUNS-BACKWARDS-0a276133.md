## Two verb chapters: the mind, and the verb that runs backwards

Italian joins Spanish, Latin and Portuguese on eight core verbs, taking the
track from **6 of 40** core verbs to **14 of 40**. Eight one-verb lessons in
**two** chapters, not one: eight lessons introduce more atoms than
`maxNewAtomsPerChapter: 12` allows in a single chapter, and splitting is the
resolution rather than raising the budget.

- **Chapter 18 — Four Verbs of the Mind** (`IT-C18-pensare`, `IT-C18-capire`,
  `IT-C18-leggere`, `IT-C18-scrivere`; concepts `VERB-THINK`,
  `VERB-UNDERSTAND`, `VERB-READ`, `VERB-WRITE`). One verb from each of Italian's
  three conjugation families, and the tranche where the **-isc- class** earns
  its keep: *capisco / capisci / capisce* but *capiamo / capite*, a family
  Italian shares with French (*je finis, nous finissons*) and Spanish does not
  have. Etymological thread: all four Latin roots name a **physical act** before
  a mental one — *pēnsāre* to weigh, *capere* to seize, *legere* to gather,
  *scrībere* to scratch. 12 new atoms, at the chapter budget.
- **Chapter 19 — Taking, Asking, Helping — and Mi Piace**
  (`IT-C19-prendere`, `IT-C19-chiedere`, `IT-C19-aiutare`, `IT-C19-mi-piace`;
  concepts `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE`). The payoff
  is *mi piace*, which is backwards exactly as Spanish *gustar* is: *mi piace il
  vino* says the wine pleases me, so the thing liked is the subject and the verb
  goes plural for it. Chapter 3 taught *piacere* as a one-word courtesy; this
  chapter reveals it as the same verb's dictionary form. 11 new atoms.
- **The two chapters are linked by their roots, not by a cross-reference.**
  *Capire* is built on *capere*, "to seize"; *prendere* on *prehendere*, a
  **different** Latin verb with the same meaning. Italian built understanding on
  one grasping-verb and taking on the other, and Chapter 19 says so by looking
  **back** at Chapter 18 rather than Chapter 18 teasing forward.
- **Reinforcement (HL09 §7), measured rather than asserted.** Each payoff
  re-practises atoms from *earlier* chapters, not only its own: Chapter 18's
  reaches back to `IT-GRAMMAR-PARLARE-04` (Ch. 5), `IT-GRAMMAR-PASSATO-PROSSIMO-02`
  (Ch. 15), `IT-ETYMON-NUMERI-6-10-03` (the *-ct-* → *-tt-* law, Ch. 6) and
  `IT-ETYMON-MANO-04` (Ch. 17, via *manoscritto*); Chapter 19's reaches back to
  `IT-SOUND-PIACERE-02`, `IT-ETYMON-PIACERE-03`, `IT-GRAMMAR-PIACERE-04` and
  `IT-GRAMMAR-MI-CHIAMO-04` (Ch. 3), `IT-ETYMON-STAGIONI-02` (Ch. 9),
  `IT-ETYMON-ACQUA-VINO-03` (Ch. 11) and Chapter 18's *leggere* and *capire*.
  On the committed corpus, **none of the 23 atoms these chapters introduce
  misses a reinforcement window**, and only **3 of the 23** are never revisited
  — the three introduced by the track's final lesson, where no later lesson
  exists to revisit them. That is a 13% orphan rate against the corpus's 50%.
  The tranche also rescues a pre-existing orphan: `IT-ETYMON-MANO-04`, taught in
  Chapter 17 and never practised again, is now retrieved twice (via
  *manoscritto* and via *mandāre*). Italian's orphan count therefore moves 26 →
  28 while its taught-atom count moves 112 → 135.
- **No forward references.** The continuity walk reports **zero** forward
  references from any of the eight lessons, and no lesson closes with a "next
  time we'll meet X" tease. Every example uses only vocabulary the track has
  already taught — *pane*, *acqua*, *vino*, *ora*, *stagioni*, *Roma*,
  *italiano* — which is also why *non capisco* is **not** taught here: Italian's
  `SPINE-NEGATE-AND-ASK` is still unrealised, so *non* does not exist yet in
  this track. The lesson teaches *Capisco* and *Capisci?* instead.
- **Wiring.** `SPINE-SAY-WHAT-I-DO` gains two real path segments, `IT-PATH-022`
  and `IT-PATH-023`, and drops the eight concepts from its `omits`; the segment
  ledger is refreshed against the authored path. `chapters.json` gains capability
  entries 18 and 19, `core/book-generation.json` two Italian targets, and
  `book/book.tex` two `\input` lines. Book chapters, the modality manifest and
  the narration export are regenerated; all eight lessons classify **voice**, so
  both chapters are fully drivable.
- Every lesson is under five minutes as computed, not merely as declared: the
  effective durations run 257–298 s against the 300 s gate.

