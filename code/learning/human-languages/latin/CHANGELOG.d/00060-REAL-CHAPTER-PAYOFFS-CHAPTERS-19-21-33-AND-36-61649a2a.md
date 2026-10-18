## Real chapter payoffs — Chapters 19, 21, 33, and 36

- Adds four terminal consolidation lessons — `LA-C19-practice`,
  `LA-C21-practice`, `LA-C33-practice`, and `LA-C36-practice` — so the track now
  has five chapters that end on a lesson *written* to be a payoff rather than on
  whichever teaching lesson happened to come last. Latin had exactly one
  (`LA-C01-practice`) across 36 chapters before this.
- Targets the four weakest endings measured by the chapter ledger: chapters 21,
  33, and 36 previously ended on a `culture` or `etymology` lesson, so the reader
  finished by tracing a root rather than doing anything; chapters 19 and 36 were
  also the thinnest, with two atoms each.
- **Every Latin word in every new lesson is already taught.** Each payoff
  declares the atoms it needs in `requires.knowledge` and each is closed by a
  transitive prerequisite; where the material lived in a sibling branch, the
  payoff names that lesson as an explicit prerequisite (chapter 36 pulls in
  `LA-C34-bonum-vesperum` and `LA-C19-practice`; chapter 21 pulls in
  `LA-C19-practice`).
- **Chapter 33's payoff is honestly a `task`, not a `dialogue`.** The chapter
  teaches *vesper* and its afterlives and contains no greeting or exchange, so
  its payoff is the usable skill it really hands over: sort any European evening
  word into the *vesper* family or the *sērus* family, then produce *vespere*.
  Latin is a taproot track, and a fabricated conversation there would have
  misrepresented it.
- Chapters 19, 21, and 36 do support genuine exchanges and get them —
  *salvē / quid agis? / valeō, grātiās tibi agō / valē*; a first meeting that
  trades names with *quid tibi nōmen est?* alongside the chance re-encounter
  Plautus actually wrote *volup est convēnisse* for; and a whole Roman day
  greeted morning to night, with *salvē* and *valē* marked as the only attested
  pair among the four rows.
- Each new lesson introduces exactly **one** knowledge atom, well inside the
  gentle-ramp budget of three, and carries one compiled objective activity.
  Effective durations are 220–239 s against the 300 s ceiling.
- Registers the four lessons in [`curriculum.json`](./curriculum.json) in
  prerequisite-safe positions with three new `consolidation` extension nodes, and
  repoints chapters 19, 21, 33, and 36 in [`chapters.json`](./chapters.json).
  Representativeness stays at 100% for all 36 chapters — it was 100% before, too,
  which is itself the finding: the metric cannot distinguish a payoff from a
  chapter's last teaching lesson.

