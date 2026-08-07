# Changelog

## Eight more verbs — Chapter 38

- Adds a **second verb tranche**, one canonical concept per lesson:
  `LA-C38-cogito` (VERB-THINK), `LA-C38-lego` (VERB-READ), `LA-C38-intellego`
  (VERB-UNDERSTAND), `LA-C38-scribo` (VERB-WRITE), `LA-C38-capio` (VERB-TAKE),
  `LA-C38-rogo` (VERB-ASK), `LA-C38-adiuvo` (VERB-HELP), and `LA-C38-amo`
  (VERB-LIKE-LOVE).
- **These eight concepts were taught by no track anywhere in the corpus.** Of
  the taxonomy's forty core verbs, twenty-three were unrealized in every one of
  the twenty-two tracks before this chapter; this closes eight of them and
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
  table anywhere. All eight lessons derive as `voice`, so chapter 38 is fully
  drivable end to end, and effective durations are 266–282 s against the 300 s
  ceiling.
- Registers the eight in [`curriculum.json`](./curriculum.json) as a second
  `SPINE-SAY-WHAT-I-DO` segment, `LA-PATH-026`, with the matching
  `LA-EXT-026-LANGUAGE-SPECIFIC` extension node, and drops the eight concepts
  from that node's `omits` ledger. The chapter's capability entry lands in
  [`chapters.json`](./chapters.json); its payoff is `LA-C38-amo`, which closes
  the set by producing all eight first-person and all eight third-person forms.
- The Latin book compiles with XeLaTeX at 138 pages with **zero** `Missing
  character` warnings and no new overfull or underfull boxes; PIE roots that
  would need a glyph Latin Modern lacks are cited in plain ASCII
  (`*ag-`, `*leg-`, `*reg-`) rather than with a palatal-g diacritic.

## Eight core verbs — Chapter 37

- Adds the track's first eight **verb** lessons, one canonical concept each:
  `LA-C37-sum` (VERB-BE), `LA-C37-habeo` (VERB-HAVE), `LA-C37-eo` (VERB-GO),
  `LA-C37-venio` (VERB-COME), `LA-C37-dico` (VERB-SAY), `LA-C37-video`
  (VERB-SEE), `LA-C37-scio` (VERB-KNOW), and `LA-C37-do` (VERB-GIVE). Before
  this the Latin track taught **zero** verbs as headwords across 36 chapters,
  and realized none of the shared taxonomy's forty core verb concepts.
- **These are the first lessons anywhere in the corpus to realize a canonical
  `VERB-*` concept.** Every other track's verb tags are namespaced
  (`ES-VERB-HABLAR` and the like) and therefore join no cross-language index.
  Latin now covers 8 of the 40, and the other twenty-one tracks still cover
  none.
- **Atom-first, one verb per lesson.** Each lesson gives one verb its six
  present-tense forms as a spoken list — never a paradigm grid — plus the
  etymology that makes it stick. Effective durations are 236–280 s against the
  300 s ceiling, and all eight lessons derive as `voice`: chapter 37 is fully
  drivable end to end, the first new chapter in a while that costs the commuter
  nothing.
- **Every false friend is flagged rather than quietly avoided.** English *have*
  is not descended from *habēre* (it belongs with *capere*, "to seize"); English
  *know* is not from *sciō* but from the root behind *nōscō*; *Venus* and
  *venom* are not relatives of *veniō*; *addere* belongs to a different root
  from *dare*; and *scīre*'s link to "cut, separate" is marked as the standard
  account rather than a certainty.
- **Chapter 37 is the corpus's first A2 material.** The eight attach to
  `SPINE-SAY-WHAT-I-DO`, which the shared spine declares at stage A2, so the
  level is derived from the spine rather than claimed in frontmatter. Latin is
  now the only track with any lesson above A1.
- Registers the eight in [`curriculum.json`](./curriculum.json) as `LA-PATH-025`
  with one new `language-specific` extension node — the eight canonical concepts
  live in the shared taxonomy but belong to no spine node, so they attach as a
  track extension rather than as shared spine content — and adds the chapter's
  capability entry to [`chapters.json`](./chapters.json). The chapter's payoff is
  `LA-C37-do`, which closes the set by producing all eight first-person and all
  eight third-person forms in sequence.

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

## Warning-free 36-chapter book

- Supplies Latin Modern's matching small-caps face explicitly, keeps chapter
  openings on right-hand pages with truly empty versos, and uses compact
  section running heads.
- Removes every overfull and underfull box, missing-glyph, duplicate-destination,
  Hyperref, generic LaTeX, and font warning from a forced 115-page XeLaTeX
  build.
- Recasts the dense Chapters 16, 17, and 20 recall blocks as canonical bullet
  lists so the app and generated book share the same more-scannable review.

## Chapters 2–36 — canonical app/book publication

- Migrates all 53 Latin lessons to schema v2 with shared-spine placements,
  topological sequences, explicit knowledge boundaries, stable typed blocks,
  and sub-five-minute effective durations.
- Generates Chapters 2–36 from those canonical lessons, with deterministic
  lesson ids and source hashes independently checked by the book pipeline and
  Language Ladder.
- Expands the book from its hand-authored Chapter 1 opening to all 36 authored
  chapters without creating a second, book-only copy of the curriculum.
- Records the expanded PDF's layout and small-caps warning baseline separately
  for the focused cleanup tranche instead of altering canonical lesson content
  during publication.

## Sub-five-minute lesson remediation — 43 violations to zero

- Corrects thirty-seven declared budgets whose lesson bodies already compute
  below five minutes.
- Splits six genuinely long lessons into prerequisite-ordered pairs, adding
  focused companions rather than deleting grammar, etymology, usage, or
  attestation depth.
- Separates weather-word history from weather verbs, wellbeing questions from
  the `valeō/valē` family, dative possession from authorial case variation,
  Plautine wording from its usage limits, `vesper` from its daughter-language
  afterlives, and the absent afternoon formula from time-independent `salvē`.
- Leaves every affected step below 300 effective seconds and keeps all
  prerequisite references resolvable for the shared app/book corpus.

## Chapter 1 — Greetings (taproot track)

- New Latin track on the HL00 framework — Italic/Indo-European, Latin script
  (Latin Modern font, no vendored font needed). One word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. Classical pronunciation with macrons.
- Chapter 1 (`lessons/LA-C01-*`):
  - **salvē / salvēte** ("hello," lit. "be well") — imperative of *salvēre* ←
    *salvus* "safe"; hub for English *save/safe/salvage/salvation/salute* and
    Romance *salud/salute/salut*. Introduces macrons, *v*=w, and the singular vs.
    plural *-ē / -ēte* ending.
  - **avē / avēte** ("hail," formal) ← *avēre*; survives in *Ave Maria* and *avē
    atque valē*.
  - **valē / valēte** ("goodbye," lit. "be strong") ← *valēre*; hub for *value/
    valid/valiant/prevail/convalesce* and Romance *valer/valere/valoir*.
  - **grātiās (tibi) agō** ("thank you," lit. "I give thanks") ← *grātia* "grace";
    hub for *grace/gratitude/congratulate* and the direct parent of Romance
    *grazie/gracias/graças*; teaches the *-ō* = "I" verb ending.
  - **ita / nōn** ("yes / no") — Latin had no plain "yes" (*ita/sīc/certē* or
    repeat the verb); *sīc* → Romance *sí/sì*; *nōn* ← *ne-* → English
    *no/not/none*, cousin of Sanskrit *na*.
  - **practice**.
- The recurring thread: Latin as **taproot** — each greeting an etymological hub
  whose children populate both English and the Romance tracks (Spanish, French,
  Italian, Portuguese) already in the curriculum. Grammar lens foregrounds
  meaning-in-endings and pronoun-dropping. Classical pronunciation documented in
  the appendix. Book compiles clean with XeLaTeX.
