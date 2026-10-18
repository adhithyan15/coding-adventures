## Eight more verbs — Chapters 38 and 39

- Adds a **second verb tranche**, one canonical concept per lesson, split
  across two chapters of four. Chapter 38, *Thinking, Reading, Writing*:
  `LA-C38-cogito` (VERB-THINK), `LA-C38-lego` (VERB-READ), `LA-C38-intellego`
  (VERB-UNDERSTAND), `LA-C38-scribo` (VERB-WRITE). Chapter 39, *Taking,
  Asking, Helping, Loving*: `LA-C39-capio` (VERB-TAKE), `LA-C39-rogo`
  (VERB-ASK), `LA-C39-adiuvo` (VERB-HELP), `LA-C39-amo` (VERB-LIKE-LOVE).
- **Four per chapter, not eight, and the ramp is the reason.** Eight verbs is
  16 new knowledge atoms — two per lesson, one lexical and one etymological —
  against a per-chapter budget of 12. Chapter 37 carries exactly that shape and
  is a standing ramp violation. The fix here is the chapter, not the budget:
  raising `maxNewAtomsPerChapter` would make the number disappear without
  making anything gentler, whereas two sittings of four genuinely is a gentler
  ramp than one sitting of eight, and page count costs a reader nothing. Both
  chapters land at **8 atoms against the budget of 12**, and neither appears in
  the ramp report's violation list.
- The seam is pedagogical, not arithmetic. *Cōgitō → legō → intellegō →
  scrībō* is one arc — turning something over, gathering it off a page,
  picking it apart, scratching it down — and *intellegō* is literally *inter* +
  *legō*, so it has to sit beside the verb it is built from. *Capiō → rogō →
  adiuvō → amō* is the other: what you do with and for other people.
- **These eight concepts were taught by no track anywhere in the corpus.** Of
  the taxonomy's forty core verbs, twenty-three were unrealized in every one of
  the twenty-two tracks before these chapters; this closes eight of them and
  leaves fifteen still universally missing. Latin now realizes 16 of 40 — the
  deepest verb coverage in the corpus.
- **This is where Latin's English dividend is largest.** *Legō* first meant "to
  gather," which is why it is behind *legible*, *lecture*, *lesson*, *collect*,
  *select*, *elect*, *neglect*, and *elegant*, and why its Greek twin supplies
  every *-logy*. *Intellegō* is literally *inter* + *legō*, "to pick out from
  among" — and the classical spelling with *e* beside the later spelling with
  *i* is exactly why English carries both *intellect* and *intelligent*.
  *Scrībō* rests on a root meaning "to scratch," because a Roman writing a note
  scratched wax. *Capiō* gives *capture*, *accept*, *receive*, *deceive*,
  *conceive*, *concept*, *chase* and *catch*.
- **The false friends are named, not avoided.** English *read*, *write*,
  *think*, *ask* and *help* are all Germanic and unrelated to the Latin verbs
  that translate them. *Capital* and *captain* come from *caput* ("head"), not
  from *capiō*. *Juvenile* comes from *iuvenis* ("young"), not from *iuvō* ("to
  help"). *Amen* is Hebrew. Where the trail is uncertain the lesson says so:
  the link from *lēx* ("law") to *legere* is marked **debated**, *rogāre*'s
  descent from the root behind *rēx* is marked **the standard account rather
  than a certainty**, and *amāre* is given **no** reconstructed root at all,
  because it has none — it is taken as a nursery word, and inventing an
  ancestor would have been worse than admitting the gap.
- **Chapter 37's conventions carried forward unchanged.** One verb per lesson;
  six present-tense forms as a spoken bullet list, never a paradigm grid; no
  table anywhere. All eight lessons derive as `voice`, so both chapters are
  fully drivable end to end, and effective durations are 270–285 s against the
  300 s ceiling.
- Registers the eight in [`curriculum.json`](./curriculum.json) as two further
  `SPINE-SAY-WHAT-I-DO` segments, `LA-PATH-026` and `LA-PATH-027`, each with a
  matching `language-specific` extension node, and drops the eight concepts
  from that node's `omits` ledger. Both capability entries land in
  [`chapters.json`](./chapters.json). Chapter 38's payoff is `LA-C38-scribo`
  and chapter 39's is `LA-C39-amo`; each closes its own chapter by producing
  that chapter's four first-person and four third-person forms, so neither
  payoff reaches across the seam for material the reader met a chapter ago.
- The Latin book compiles with XeLaTeX at 142 pages with **zero** `Missing
  character` warnings and no new overfull or underfull boxes; PIE roots that
  would need a glyph Latin Modern lacks are cited in plain ASCII
  (`*ag-`, `*leg-`, `*reg-`) rather than with a palatal-g diacritic.

