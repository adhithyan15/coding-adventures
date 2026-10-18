## Chapters 33–36 — the pre-A1 noun tranche, and what the level gate actually counts (2026-08-07)

- Added fifteen everyday-noun lessons across four chapters, all filed under
  **pre-A1** spine nodes: `AR-C33-shay`, `AR-C33-halib`, `AR-C33-sukkar`,
  `AR-C33-qahwa` (drinks); `AR-C34-milh`, `AR-C34-lahm`, `AR-C34-jubn`,
  `AR-C34-taam` (food); `AR-C35-jar`, `AR-C35-ibn-bint`, `AR-C35-sadiq`
  (people); `AR-C36-zayt`, `AR-C36-kub`, `AR-C36-tabaq`, `AR-C36-milaqa`
  (vessels and plurals). Chapters 33, 34 and 36 realize
  `SPINE-POLITE-REQUEST-REPAIR` — these are the things you ask for across a
  counter — and Chapter 35 realizes `SPINE-EXCHANGE-NAMES`, the people you
  introduce.

- **This tranche was a measurement probe, and it answers its question.** For
  Arabic, `levelGate.tracks[arabic]` moves as follows:

  | figure | before | after |
  |---|---|---|
  | vocabulary blocker (headwords at or below pre-A1) | 33 of 300 | 48 of 300 |
  | vocabulary shortfall | 267 | 252 |
  | reinforcement blocker (pre-A1 atoms revisited < 2) | 60 | 49 |
  | `vocabulary` field (whole track, every level) | 65 | 80 |

  So yes: track-local pre-A1 vocabulary does move the gate. But it moves it
  **one headword per lesson**, because `vocabularyOf` in `src/level-gate.ts`
  counts *distinct `headword` frontmatter strings*, not words taught. A lesson
  whose body hands the learner *ṭaʿām*, *ṭaʿima*, *ṭaʿm* and *maṭʿam* still
  contributes exactly one. Closing pre-A1 at the present target therefore costs
  roughly **252 more lessons** for this track alone.

- **What actually levels a lesson is `path[].lessons`, not the extension.**
  `lessonSpineNodes` walks only `curriculum.path[].lessons`; it never reads
  `extensions[]`, nor `before`/`inline`/`after`. An `AR-EXT-*` entry is still
  required — `validateCurriculum` errors with
  `unclassified-curriculum-extension-lesson` for any content lesson whose
  namespaced `concept_tag` no spine node owns — but it contributes nothing to
  the level. Both wirings are present here: segments `AR-PATH-030`–`033` and
  extensions `AR-EXT-030`–`033-LANGUAGE-SPECIFIC`, all staged `pre-A1`.

- **Words the brief proposed that the track already taught, and which were
  therefore not duplicated:** *māʾ* and *khubz* (`AR-C13-maa-khubz`), *ism*
  (`AR-C02-ism`), *ab* and *umm* (`AR-C10-ab-umm`), *akh* and *ukht*
  (`AR-C10-akh-ukht`). *Kitāb* is not a headword anywhere, but
  `AR-C31-kataba` already derives it, *kātib*, *maktūb*, *maktab* and
  *maktaba* in full, so a *kitāb* lesson would have re-taught that lesson's
  payoff; it is reached back to instead.

- **Words that do not honestly serve any pre-A1 node, and were left out.**
  The seven pre-A1 nodes are all social transaction — greet, thank, yes/no,
  exchange names, check wellbeing, request/repair, take leave — and their 34
  concepts contain no concrete object at all. *Bayt* (house), *bāb* (door),
  *kursī* (chair), *ṭāwila* (table), *miftāḥ* (key), *madrasa* (school) and
  *sūq* (market) belong to "describe where I am", whose nearest spine node,
  `SPINE-ASK-LOCATION`, is **A1**. Putting them at pre-A1 would have inflated
  the gate rather than satisfied it.

- **Arabic's signature, used rather than described.** `AR-C34-taam` pours the
  new root **ط-ع-م** through the *ma-* place shape the learner already owns
  from *maktab* (Ch. 31) and *madhhab* (Ch. 28) and hands back **مَطْعَم**
  (*maṭʿam*, "a restaurant") without teaching it. `AR-C36-milaqa` sets the
  *mi-* **tool** shape (*mirʾāh* from Ch. 29, *milʿaqa*) against that *ma-*
  **place** shape. The **فَعيل** (*faʿīl*) pattern is named once and then
  collected across three chapters — *ḥalīb*, *malīḥ*, *ṣadīq*. **لحم** is
  taught as **ملح** reordered, which is the root system stated in one line.
  Sound and broken plurals are named in Chapter 36: *akwāb*, *aṭbāq* and
  *malāʿiq* re-poured against *qahawāt* and *ṣadīqāt* merely suffixed.

- **Gender is taught as a rule you can apply on sight**, not a list: the
  *tāʾ marbūṭa* **ة** marks *qahwa*, *ṣadīqa*, *ṭabaqa* and *milʿaqa*
  feminine, and it is the tied form of the plain **ت** that already separated
  *akh* from *ukht* and now separates *ibn* from *bint*.

- **Honesty kept, three times.** `AR-C34-jubn` refuses to make one root of
  *jubn* "cheese" and *jubn* "cowardice"; `AR-C36-kub` refuses to claim
  descent from Latin *cuppa*; `AR-C33-halib` refuses the milk story told about
  Aleppo, whose name predates Arabic. `AR-C35-ibn-bint` gives the grammarians'
  **ب-ن-ي** derivation and then says why comparative Semitists hold it loosely.

- **Reach-back at two cadences.** Every lesson practises the preceding one to
  three lessons' atoms, so each new atom is revisited at least twice inside
  R1 (n+1…n+3); each chapter payoff additionally rescues atoms several chapters
  back. Ten previously never-revisited pre-A1 atoms leave the blocker:
  `AR-CONCEPT-C04-AL-SALAMA-01`, `C06-MIN-FADLIK-01`, `C10-AB-UMM-01`,
  `C10-AKH-UKHT-01`, `C13-MAA-KHUBZ-01`, `C13-MAA-KHUBZ-02`, `C23-AFWAN-01`,
  `W07-HOOK-FAMILY-HA-KHA-01`, `W08-KAF-AND-RA-01/-02`,
  `W11-HA-AND-TA-MARBUTA-02`. The tranche adds none of its own.

- **The script-section inconsistency in this track is real and is recorded.**
  Chapters 28–30 head their inline-letters section `## You'll want to know —
  The letters in this word`, which `classifyBlock` types `input` — not
  detachable, so the letter shapes sit in the lesson's **core** and the lesson
  is not drivable. Chapters 31–32 use the canonical `## The letters in this
  word`, typed `script` and detachable, so the core stays `voice`. All fifteen
  new lessons use the canonical heading; all fifteen have a `voice` core. The
  six Chapter 28–30 lessons are **not** fixed here and remain measurable debt.

- Wiring: `chapters.json` gains four HL05 ledgers with capabilities and
  payoffs; `core/book-generation.json` gains four `arabic-main` targets;
  `book/book.tex` inputs the four new chapters. The whole book compiles under
  XeLaTeX at **164 pages with zero `Missing character` warnings**. Narration
  regenerated for `ch33`–`ch36`; no lesson carries a table, so nothing is
  narrated as a grid.

- All fifteen lessons are under the five-minute computed ceiling (265–298
  effective seconds) and inside `maxNewAtomsPerLesson: 3` and
  `maxNewAtomsPerChapter: 12` — each chapter introduces 8, except Chapter 35,
  which introduces 6.

