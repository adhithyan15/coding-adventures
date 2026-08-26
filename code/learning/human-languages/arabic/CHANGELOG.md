# Changelog

## Split the four-mark harakat spike (#12256)

- Keep `AR-W06-harakat-and-hamza` focused on the three short-vowel marks and
  their *anta/anti* payoff.
- Add a separate four-minute lesson for *sukūn*, *shadda*, and *fatḥatan*, then
  make the hamza lesson depend on it. The valid writing chain now introduces at
  most three new Arabic-script marks per lesson.

## Chapter 1 speaks first, then writes three shapes at a time (#12315)

- Rewrite *salām*, *marḥaban*, and *as-salāmu ʿalaykum* as voice-first
  lessons, so a learner owns the sound, meaning, and social use before being
  asked to decode or copy the complete Arabic form.
- Interleave seven four-to-five-minute observe, trace, guided-copy, and delayed
  recall lessons. Six introduce only two or three new Arabic glyphs; the final
  lesson retrieves the complete greeting without introducing another shape.
- Mirror the new ramp in the hand-authored book and generated narration,
  reducing the chapter-1 glyph spikes from three to zero without metadata-only
  reordering.

## Arabic chapters 1-2 regain their reading order (#12255)

- Add one global, spaced sequence to the 26 legacy content and writing lessons
  before the already-ordered Chapter 3 tranche.
- Recover content order from the hand-authored book, then place each four-minute
  writing set after the words it reuses and in its documented prerequisite chain.
- Remove 26 missing-sequence findings and the 27 false forward references caused
  by alphabetical fallback, without hiding Arabic's remaining curriculum debt.

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

## Chapters 31–32 — the eight-verb tranche, and the root system paying out (2026-08-07)

- Added eight lessons, one verb each, realizing the eight canonical `VERB-*`
  concepts that Spanish, Latin and Portuguese already share:
  `AR-C31-fahima` (VERB-UNDERSTAND), `AR-C31-qaraa` (VERB-READ),
  `AR-C31-saala` (VERB-ASK), `AR-C31-kataba` (VERB-WRITE), `AR-C32-akhadha`
  (VERB-TAKE), `AR-C32-fakkara` (VERB-THINK), `AR-C32-saada` (VERB-HELP),
  `AR-C32-ahabba` (VERB-LIKE-LOVE). Arabic goes **6 of 40 → 14 of 40** (15% →
  35%), and each of the eight becomes a **four-way** cross-language concept
  rather than the three-way join it was.
- **Two chapters, not one.** Eight one-verb lessons at Arabic's usual two atoms
  apiece would have been 15 atoms against `maxNewAtomsPerChapter: 12`. Split,
  Chapter 31 introduces **7** and Chapter 32 **8** — both inside budget, and
  `ramp` chapter violations hold at 25 rather than rising to 26. Page count is
  never a cost, so the split cost nothing.
- **Chapter 31 is where the root engine stops being demonstrated and starts
  being used.** *fahima* names **و** (read unnamed since *qawl*) because every
  done-to word ahead needs it; *qaraʾa* sorts out hamza's three seats (أ ؤ ئ)
  and the seatless ء; *saʾala* watches that seat move — *saʾala*, *suʾāl*,
  *sāʾil* — one consonant, three spellings; and *kataba* closes the chapter by
  pouring **ك-ت-ب** through every pattern the track already owns: *kitāb*,
  *kātib* (doer, as *qāʾil*/*qāriʾ*), *maktūb* (done-to, as *maqūl*/*mafhūm*),
  *maktab* (the *ma-* place shape of *madhhab*/*masjid*) and *maktaba*. Five
  words, none memorised separately.
- **Chapter 32 teaches the derived forms as a system, not as exceptions.**
  *ʾakhadha* is the plain **فَعَلَ** control; *fakkara* is **Form II**
  (*faʿʿala*, middle consonant doubled by a shadda that finally does
  load-bearing work); *sāʿada* is **Form III** (*fāʿala*, vowel stretched, verb
  aimed at another person — from a root that means *good fortune*, not
  helping); *ʾaḥabba* is **Form IV** (*ʾafʿala*, prefixed, a root made to bring
  about its own idea), and it closes by naming all four shapes together. The
  new **مُ** doer of *mufakkir* and *musāʿid* is set against the long *ā* of
  *kātib* and *ākhidh* rather than left as noise.
- **Cousins claimed only where they are real, and refused loudly where they are
  not.** **ق-ر-أ** genuinely gave English **Quran**/**Koran** (*qurʾān*, "the
  recitation") and **س-ع-د** the **Saudi** in Saudi Arabia (**آل سُعود**, a
  family name, not a borrowed verb). Everything else says so plainly: *fahima*
  has no English relative and *fathom* is Germanic; *fakkara* has neither an
  English cousin nor a Hebrew twin; **ك-ت-ب** gave English nothing, though
  English **ketubah** comes from the same Semitic root through Hebrew *katav*;
  and Hebrew *ahav*, "he loved", is **not** a relative of *ʾaḥabba* — *ḥaviv*
  is. Semitic twins are given with their sound correspondences: *shaʾal*/*saʾala*
  on the *s*↔*sh* swap already met in *salām*/*shalom*, and *ʾaḥaz*/*ʾakhadha*
  on Arabic **خ**↔Hebrew **ח** and **ذ**↔**ז**.
- **No new letters, and no forward references.** Every glyph the eight lessons
  put on the page was taught earlier; only hamza's **ؤ** and **ئ** seats are
  new shapes, and they are introduced in the lesson (*qaraʾa*) whose own family
  needs them. Corpus forward references hold at **504**, and the script-ramp
  violation count holds at **61**.
- **Reinforcement at two cadences, measured rather than asserted.** Every
  lesson declares `practises.knowledge` naming atoms from the immediately
  preceding one to three lessons — across the chapter seam — so **every atom
  introduced in Chapters 31–32 has its R1 window closed**. The two payoffs
  reach further: `AR-C31-kataba` practises `AR-CONCEPT-C28-DHAHABA-02` (the
  *ma-* place engine, nine lessons back) and `AR-CONCEPT-C29-QALA-02` (the
  doer/done-to pair, seven back), and `AR-C32-ahabba` practises
  `AR-CONCEPT-C28-DHAHABA-02` again at thirteen. Four Chapter 28–30 atoms that
  had **never been revisited at any distance** now are —
  `AR-CONCEPT-C28-DHAHABA-01`, `AR-CONCEPT-C30-ARAFA-01`,
  `AR-CONCEPT-C30-AKALA-01` and `AR-CONCEPT-C30-AKALA-02` — and the last three
  of those, along with `AR-CONCEPT-C28-DHAHABA-02` and
  `AR-CONCEPT-C29-QALA-02`, now carry no open window at all. Corpus
  never-revisited moves 767 → **765** while the atom count rises 1,519 →
  **1,534**, and missed-R1 moves 766 → **765**. `reviews_of` was updated
  alongside, but nothing here relies on it: it names lesson ids, not atoms, and
  has never closed a window.
- **Modality is labelled honestly, and it costs something.** All eight use the
  canonical `## The letters in this word` heading, which types as a `script`
  block: `modality: sight`, `coreModality: voice`, detachable. Track modality
  moves 44/17/16 → **44 `voice` / 25 `sight` / 16 `pen`**, 73% → **75%
  drivable** (56 of 85 lessons with a voice core), 48 → **56** lessons
  reachable in chapter-prefix order, and 12 → **20** rescued by a detachable
  block. Two chapters join the "unstartable at full modality" count (90 → 92),
  which is the true statement rather than the flattering one.
- Effective durations run **273–288s** against the 300s ceiling; corpus
  duration violations stay at **0**.
- Wired through the whole pipeline: `curriculum.json` gains `AR-PATH-028` and
  `AR-PATH-029` on `SPINE-SAY-WHAT-I-DO` with `AR-EXT-028-LANGUAGE-SPECIFIC`
  and `AR-EXT-029-LANGUAGE-SPECIFIC` inline, and the eight concepts drop out of
  that node's `omits`; `chapters.json` gains two HL05 entries whose payoffs
  assess 5 of 7 and 5 of 8 chapter atoms, both above the 0.5 floor;
  `core/book-generation.json` gains two targets and `book.tex` two `\input`
  lines. Verified with a clean three-pass XeLaTeX run: **130 pages, 0 missing
  characters, 0 overfull, 0 underfull**. (Chapter 31's title was shortened from
  "Understanding, Reading, Asking, Writing" to "Understanding, Reading,
  Writing" because the longer running head overran the text block by 0.199pt.)

## Chapters 28–30 — the track's first verbs (2026-08-06)

- Added six lessons, one verb each, realizing six **canonical** `VERB-*`
  concepts: `AR-C28-dhahaba` (VERB-GO), `AR-C28-jaa` (VERB-COME),
  `AR-C29-qala` (VERB-SAY), `AR-C29-raa` (VERB-SEE), `AR-C30-arafa`
  (VERB-KNOW), `AR-C30-akala` (VERB-EAT). Arabic taught **zero** verbs before
  this; it is now the first track in the corpus to realize any core verb
  concept at all — 6 of 40 — and the first to place lessons on an **A2** spine
  node (`SPINE-SAY-WHAT-I-DO`).
- **The root-and-pattern system is the subject, not a remark.** `dhahaba` opens
  on ذ-ه-ب giving "he went," *dhahab* ("gold") and *madhhab* ("a school of
  law"), then pays the *ma-* place shape off against **mosque** (*masjid*) and
  the **Maghreb** (*maghrib*) — words English already owns, so the pattern
  arrives already-known and only the root is new. A Grammar Lens states the
  load-bearing fact: Arabic has **no infinitive**, so the dictionary form is
  *dhahaba*, "he went," the فَعَلَ (*faʿala*) shape every verb here wears.
- **Weak roots taught as regularity, not exception**: middle-weak in *jāʾa*
  (ج-ي-أ, the *yāʾ* walking back out in *majīʾ*) and *qāla* (ق-و-ل, the *wāw*
  back in *qawl*); final-weak in *raʾā* (ر-أ-ي, surfacing as the dotless ى).
  Pattern families then carry the vocabulary: *qāʾil*/*maqūl*/*maqāla* reuse
  the doer, done-to and place shapes met in *shukran*; *mirʾāh* ("mirror")
  introduces the instrument shape; *raʾy* ("opinion") is a *seeing*, the same
  metaphor as English "in my view."
- **Cousins claimed only where they are real.** *ʿarafa* carries a genuine
  Al-Andalus loan — *al-ʿarīf* → Spanish *alarife*, and the **Generalife** in
  Granada, flagged as the leading reading rather than a settled one — and its
  root-sibling ع-ل-م gives *ʿālim*, whose plural English borrowed as **ulema**.
  *akala* says the opposite plainly: **no English cousin exists**, because
  Arabic and English are unrelated; what it has is the Hebrew twin *akhal*,
  beside Arabic *akl* and Hebrew *okhel*.
- **Letters taught inline, per HL00.** New: **ذ** (*dhāl*, *dāl* plus one dot),
  **ق** (*qāf*), **ى** (*alif maqṣūra*, *yāʾ* stripped of its dots) and **ف**
  (*fāʾ*) — the last two a fourth dots-family, one dot against two on one body.
  ق and ى were both flagged in Chapters 4 and 24 as letters the reader could
  read but had not been taught; this arc names them. `AR-C30-akala` needs
  **no new letters at all**, which is the inline rule paying itself off.
- **All six measure `voice`** and all three chapters are drivable end to end:
  the letter work is speakable prose, with no table, no `script` block and no
  sight cue. Track modality moves 50/5/16 → **56 `voice` / 5 `sight` / 16
  `pen`**, 70% → **73% drivable**, 42 → **48** lessons reachable in
  chapter-prefix order. Corpus totals move 1,134 → 1,140 lessons, 957 → 963
  `voice`, `sight` and `pen` unchanged, 377 → 380 chapters.
- Wired through the whole pipeline: `curriculum.json` gains `AR-PATH-027` on
  `SPINE-SAY-WHAT-I-DO` with the `AR-EXT-027-LANGUAGE-SPECIFIC` extension;
  `chapters.json` gains three HL05 ledger entries whose payoffs each assess
  three atoms, above the representativeness floor; the book gains three
  generated chapters and three `\input` lines. Verified with a real XeLaTeX
  run: 114 pages, **0 overfull, 0 underfull, 0 missing characters**.

## Drivable-prefix reordering audit — no lesson moved (2026-08-06)

- Audited HL-C30's proposal to raise Arabic's drivable prefix by moving the
  `AR-W*` writing lessons that open Chapters 3 and 4 later in their chapters.
  **No lesson was moved, and none should be.** Every Arabic number is unchanged:
  37 `voice` / 18 `sight` / 16 `pen`, 52% drivable, 31 lessons reachable in
  chapter-prefix order across 27 chapters. Per chapter the prefix is 4/14, 6/12,
  0/9, 0/8, 1/2, 1, 1, 0/2, 2, 2, 2, 0/1, 1, 0/1, 1, 1, 1, 1, 0/1, 0/1, then 1
  for each of Chapters 21–27 — before and after.
- **Chapters 3 and 4 are prefix-0 under every legal ordering**, not just the
  authored one. A prefix can only start with a lesson that has no in-chapter
  prerequisite. Chapter 3's only such roots are `AR-W07-hook-family-ha-kha`
  (`pen`) and `AR-C03-kayfa` (`sight` — it carries a two-column table);
  Chapter 4's only root is `AR-W10-ayn`, because `AR-C04-maa-with` declares it
  as a prerequisite — مع cannot be read without ʿayn. Removing all six writing
  lessons would leave both chapters at 0.
- **The obstacle is the table, not the script.** All 18 of the track's `sight`
  lessons are `sight` because of a Markdown table — 18 of 18, none because of a
  script block. Only `AR-C03-bi-khayr` is `voice` in Chapter 3 and it sits
  behind kayfa → hal → kayfa-ḥāluka; all five non-writing lessons in Chapter 4
  are `sight`. Recovering these chapters is HL-C17 (table linearisation) work.
- The move would also have broken the teaching. `AR-C03-kayfa` tells the reader
  ك has already been written "in the writing set" — it assumes
  `AR-W08-kaf-and-ra`, which requires `AR-W07` — and `AR-W09-khayr-bikhayr`
  assembles خير so the *bi-khayr* reply the chapter ends on can be written by
  hand. HL00's inline-letters rule puts them where they are.
- Chapters 8, 12, 14, 19 and 20 are also prefix 0 and also have nothing to
  reorder: 12, 14, 19 and 20 hold a single table-bearing lesson each, and
  Chapter 8's second lesson declares its table-bearing first lesson as a
  prerequisite.
- Recorded but not acted on: Chapters 1 and 2 are **undercounted**. All 26 of
  their lessons are legacy and carry no `sequence`, so the report sorts them
  alphabetically and reports prefixes of 4 and 6 where the authored
  `curriculum.json` path gives 7 and 7. Recovering those four lessons means
  giving legacy lessons sequences — a schema migration no track has made (0 of
  565 legacy lessons corpus-wide carry one) and one the validator cannot check
  for collisions outside schema v2. Slots 10–260 are reserved below Chapter 3's
  270 for whoever does it. Full analysis in
  the [`../BACKLOG.d/`](../BACKLOG.d/) history.

## Chapter capability ledger for Chapters 3–27 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger: one `canDo` promise and one validated payoff for each of Chapters
  3–27. Titles and labels are copied from `core/book-generation.json` so the
  two agree until HL-C04 inverts that dependency; `spineNodes` are derived from
  `curriculum.json`'s path segments; every `payoff.assesses` atom is taken from
  the payoff lesson's own `practises.knowledge`, never invented.
- **Chapters 1–2 are deliberately absent.** Their terminal practice lessons
  (`AR-C01-practice`, `AR-C02-practice`) are still schema v1 and declare no
  `practises.knowledge`, so no payoff can name an atom without fabricating one.
  Those two chapters also have no `book-generation.json` target to copy a title
  from. The gap is recorded in the file's own `note` and stays visible to the
  HL05 gap report rather than being filled with a placeholder.
- Only Chapters 3 and 4 end in a `practice` lesson; every later chapter's payoff
  is its last lesson by `sequence`, which is what the corpus actually offers.
  All sixteen inline `writing` steps fall inside Chapters 1–4, so no chapter's
  payoff is a script-formation exercise — Chapter 3's payoff does include
  writing **بخير** by hand, and its summary says so.

## Warning-free complete book (2026-08-03)

- Added explicit static bold and italic faces for Arabic and Hebrew, plus
  bookmark-safe Unicode commands, eliminating all font-shape and Hyperref
  warnings without dropping multilingual examples.
- Made the two handwritten recap labels unique and added a small emergency
  line-break reserve, removing every horizontal overflow while preserving the
  canonical teaching sequence.
- Added natural page bottoms for deliberately short micro-lessons and made
  open-right chapter versos truly empty, without a running header or page
  number.
- The forced 104-page build now has zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font
  warnings. All 104 pages were rendered and visually inspected.
- The 29 top-level and 90 total outline entries, title and author metadata,
  generated source hashes, and zero schema or generator leaks remain intact.

## Canonical Chapters 3–27 in the book (2026-08-03)

- Migrated all forty-five Arabic lessons in Chapters 3–27, including six
  dependency-ordered writing companions, to the strict schema-v2 curriculum
  contract: canonical spine nodes, unique prerequisite-safe sequence, explicit
  sub-five-minute budgets, typed block boundaries, and closed knowledge
  introductions and assessments.
- Generated twenty-five LaTeX chapters from those canonical lessons instead of
  copying app content into a separate book source. The committed source-hash
  manifest is independently checked against Language Ladder for Chapters 3–27.
- Added reusable Arabic and Hebrew script mappings backed by vendored static
  fonts. The 104-page PDF has zero missing glyphs and preserves the full
  29-entry top-level chapter outline.
- Rendered and inspected all 104 pages, including the writing companions and
  dense calendar, clock, age, number, and Semitic-comparison sections. No
  teaching content is clipped, colliding, accidentally omitted, or replaced by
  generator metadata.
- The expanded artifact's cleanup baseline is five overfull boxes, ten
  underfull vertical boxes, one duplicate practice label, 77 Hyperref warnings,
  two LaTeX warnings, and six font warnings. `HL-B29` tracks those warnings and
  the running headers on intentionally empty versos.
- The single all-books publication gate still compiles and catalogs all twenty
  downloadable volumes successfully.

## Sub-five-minute writing sequence

- All 39 Arabic duration violations are resolved without removing vocabulary,
  grammar, script, or etymology content. Thirty-five lessons already computed
  below the limit and now declare honest four-minute budgets.
- Four longer writing lessons became eight prerequisite-ordered micro-lessons:
  `AR-W01-direction-and-alif` → `AR-W01-abjad-short-vowels`,
  `AR-W02-joining-sin-lam` → `AR-W02-lam-alif-joins`,
  `AR-W03-dots-mim-ba-salam` → `AR-W03-write-salam`, and
  `AR-W06-harakat-and-hamza` → `AR-W06-hamza`.
- The seams follow the script itself: direction and the first *alif* stroke
  precede the abjad's hidden short vowels; positional shapes precede joined
  **سل** and obligatory **لا**; the dot family precedes assembling **سلام**;
  and short-vowel marks precede hamza as its own consonant sign.
- Downstream writing lessons now depend on and review the immediately preceding
  skill. All eight changed or added writing steps compute between 135 and 279
  seconds, and the full corpus still has zero unknown prerequisite ids.

## Chapter 4 — Farewells (and the root that closes the circle)

- **Chapter 4 authored** (`AR-C04-maa-with`, `-al-salama`, `-maa-salama`,
  `-ila-liqaa`, `-practice`): the **word** lessons for **مع السلامة**, which
  `AR-W12` had already taught the learner to hand-write — the same gap-filling
  move as Ch. 3. Completes the greet → introduce → ask → **part** arc.
- **مع** (*maʿa*, "with") — the first preposition met that does **not** attach,
  in explicit contrast to *al-*, *bi-* and *li-* from Chapters 1 and 3; and
  distinguished from *bi-* by sense (*bi-khayr* = "**in** goodness" vs *maʿa* =
  "**accompanied by**"). ← Semitic *\*ʿimma*, cousin of Hebrew **ʿim** — the
  *salām/shalom* and *ism/shem* pairing for a third time, presented as systematic
  rather than coincidental.
- **السلامة** (*as-salāma*) — **not a new word**. It is Ch. 1's *salām* run
  through the ***faʿāla*** pattern, which converts a thing into **the state of
  being** that thing (peace → safety/soundness), plus *al-* in front and the
  feminine **ة** behind. Reactivates two already-taught rules: **س** is a **sun
  letter** so *al-* assimilates (*as-salāma*), and *tāʾ marbūṭa* marks the
  feminine, as Arabic abstract nouns usually are.
- **مع السلامة** — the assembly, and the chapter's point: the **first greeting**
  (*as-salāmu ʿalaykum*) and the **last farewell** are built on the **same root
  s–l–m**, so Arabic opens and closes a conversation with one idea. Set beside
  English **goodbye** ← "***God be with ye***" — both languages hinge their
  farewell on "**with**" — plus Spanish *adiós* / French *adieu* ("to God"): four
  languages, one instinct.
- **إلى اللقاء** (*ilā l-liqāʾ*, "until the meeting," root **l–q–y**) — whose
  "until + a future point" shape is exactly Spanish *hasta luego / hasta mañana*.
  The payoff: ***hasta*** is **itself an Arabic loanword** (← *ḥattā*), one of the
  rare borrowed **function** words among al-Andalus's ~4000 — so Spanish didn't
  just copy the pattern, it borrowed the word that builds it. **Honest letter
  note**: **ق** (*qāf*) and **ى** (*alif maqṣūra*) are outside
  `data/scripts/arabic.json`'s letter set, so the lesson says to read them now and
  draw them later — *alif maqṣūra* is framed via the already-taught dots principle
  (yāʾ's body, no dots, long *ā*).
- **practice** — the full four-chapter dialogue with a literal-translation column
  showing that **no line contains a verb "to be,"** a **root ledger** (s–l–m,
  s–m–w, ḥ–w–l, kh–y–r, ḥ–m–d, l–q–y — six roots carry the whole conversation),
  and the attaching-pieces table (*al-*, *bi-*, *li-*, *-ī*, *-ka/-ki*) with *maʿa*
  as the one that doesn't.
- Taxonomy: canonical `FAREWELL`, `FAREWELL-LATER`, `REVIEW`; namespaced
  `AR-PREP-WITH`, `AR-WORD-SALAMA`. Roadmap: Ch. 4 authored and removed from the
  planned table.

## Chapter 3 — Responding (how are you, and two ways to answer)

- **Chapter 3 authored** (`AR-C03-kayfa`, `-hal`, `-kayfa-haluka`, `-bi-khayr`,
  `-al-hamdu-lillah`, `-practice`): the **word** chapter the writing set had
  already prepared — `AR-W07`–`AR-W09` hand-assembled **خير → بخير**, and this
  chapter finally says what it means. Closes the greet → introduce → **ask after
  someone** arc.
- **كيف** (*kayfa*, "how") — the partner to Chapter 2's **ما** (*mā*, "what"). A
  question word rather than a root-and-pattern noun, but *k-y-f* still generates
  **kayfiyya**, "quality" — literally "**how-ness**" — and *takyīf*, which in
  modern Arabic means **air conditioning**. Honest note in the lesson: **ف**
  (*fāʾ*) is the one letter here the writing track hasn't reached, so it is read,
  not drawn. (Every other letter in the chapter is in `data/scripts/arabic.json`
  and has been written by hand.)
- **حال** (*ḥāl*, "state") — the chapter's best root. **ḥ–w–l** means "**to turn,
  change**," so a *state* is literally **how things have turned**: *ḥawla*
  ("around"), *taḥwīl* ("transformation"), *ḥawl* ("**a year**" — one full turn).
  Set against Spanish *estar* ← *stāre* "to **stand**": two languages, two
  metaphors — one turns, one stands.
- **كيف حالك؟** (*kayfa ḥāluka / ḥāluki*) — assembled from parts, not memorised.
  The **-ka/-ki** "your" suffix attaches exactly as Chapter 2's **-ī** "my" did
  (*ism* → *ismī*), and splits by **gender** exactly as *anta/anti* did — the same
  *fatḥa*/*kasra* pair taught in `AR-W06`. Plus the **zero copula** once more:
  "how — your-state," with no word for "is."
- **بخير** (*bi-khayr*, "in goodness") — the payoff lesson. *Khayr* is **not a new
  word**: it was inside Chapter 1's **صباح الخير** all along, and *bi-* is a
  one-letter preposition that glues on like *al-* (also opening *bismillāh*, which
  contains Chapter 2's *ism*).
- **الحمد لله** — *al-* + *ḥamd* + *li-* + *allāh*, where *allāh* is itself *al-* +
  *ilāh*, "**the** god" (Semitic cousin of Hebrew *El/Elohim*, the same pairing as
  *salām*/*shalom*). The root **ḥ–m–d** "to praise" also gives **Aḥmad** and
  **Muḥammad** ("the much-praised") — so the language's commonest phrase and the
  world's commonest given name come out of the same three consonants.
- **practice** — the full greet-ask-answer dialogue, with the right-hand
  literal-translation column showing that **not one line contains a verb "to be,"**
  and a table of the three attaching particles (*al-*, *bi-*, *li-*) and two
  suffixes (*-ī*, *-ka/-ki*) that carry most of the chapter.
- Taxonomy: canonical `QUESTION-HOW`, `STATE-HOW-ARE-YOU`, `WORD-WELL`, `REVIEW`;
  namespaced `AR-WORD-HAL`, `AR-PRAISE-GOD`.

## Chapter 4 — Writing set (ʿayn, the feminine ending, and the farewell)

- **Writing lessons added** (`AR-W10-ayn`, `AR-W11-ha-and-ta-marbuta`,
  `AR-W12-maa-salama`): closes the writing track (AR-W01–12) on the **farewell**.
  No `concept_tag` (writing lessons are exempt from the concept join).
- **W10 — ع (ʿayn)**: the sound English doesn't have and the letter people picture
  when they picture Arabic — an open-mouth curve over a bowl. Opens a **third
  dots-family** (skeleton shared with **غ** *ghayn*, one dot above), rhyming with
  the Ch.2 bowl-family and the Ch.3 hook-family. **The surprise**: Phoenician
  *ʿayin* meant "**eye**"; the Greeks had no throat-sound to use it for, so they
  **repurposed** the sign as the vowel **omicron** — our **O**. The learner has
  been saying this letter since Ch.1 in *as-salāmu **ʿ**alaykum*.
- **W11 — ه and ة**: **ه** (*hāʾ*) as the most **shape-shifting** letter yet (its
  loop wears four genuinely different coats — the Lesson-2 four-coats rule at its
  extreme; *hē* → Greek epsilon → **E**). Then **ة** (*tāʾ marbūṭa*, "**tied
  tāʾ**") shown as exactly what it is — *hāʾ*'s loop **plus tāʾ's two dots** from
  Ch.2 — appearing only word-finally and marking the **feminine**.
- **W12 — assemble مع السلامة**: the farewell, "**with safety**." The payoff is
  that **السلامة** is not a new word — it is **سلام** *from Lesson 3* with **ال**
  added in front and **ة** behind. Ties the greeting *as-salāmu ʿalaykum* and the
  farewell *maʿa s-salāma* to the **same root s–l–m** (peace/safety): Arabic opens
  and closes a conversation with one idea, and the learner can now write both.

## Chapter 3 — Writing set (a second dots-family, and writing your reply)

- **Writing lessons added** (`AR-W07-hook-family-ha-kha`, `AR-W08-kaf-and-ra`,
  `AR-W09-khayr-bikhayr`): a `writing`-type companion continuing the AR-W01–06
  track, building toward the "how are you?" **reply**. No `concept_tag` (writing
  lessons are exempt from the concept join). Anchor: **خير** (*khayr*, "good") —
  the word already inside the Ch. 1 greeting **صباح الخير** (*ṣabāḥ al-khayr*).
- **W07 — the hook-and-tail dots-family**: the bowl-family trick reborn on a
  **new skeleton** — one curvy hook-and-tail body gives **ح** (*ḥāʾ*, no dot),
  **خ** (*khāʾ*, one dot above), **ج** (*jīm*, one dot below). Phoenician tie-back:
  *ḥēth*→**H**, *gīml*→**C/G**. Note that *خير* opens with *خ*.
- **W08 — kāf & rā**: **ك** (*kāf*, an angular body + inner stroke; *kaph* "palm"
  →**K**) and **ر** (*rā*; *rēsh* "head" →**R**) — the latter another **non-joiner**
  like *alif*, so the pen lifts after it.
- **W09 — assemble خير → بخير**: writes **خير** (*khāʾ·yāʾ·rā*, "good"; root
  **kh-y-r**), then prefixes **بـ** (*bi-*, "in/with") for **بخير** (*bi-khayr*,
  "well," literally "in goodness") — the everyday answer to *kayfa ḥāluk?*. A
  greeting and its reply are now both hand-writable.

## Chapter 2 — Writing set (the dots family, your name, and the hidden vowels)

- **Writing lessons added** (`AR-W04-dots-family-nun-ta`, `AR-W05-ya-and-my-name`,
  `AR-W06-harakat-and-hamza`): a `writing`-type companion to the Ch. 2
  self-introduction words, extending the Ch. 1 set (AR-W01–03). No `concept_tag`
  (writing lessons are exempt from the concept join). Chapter-2 anchors: **أنت**
  (*anta/anti*) and **اسمي** (*ismī*, "my name").
- **W04 — the bowl-skeleton dots-family, finished**: turns Lesson 3's hint into a
  truth-table — the *same* boat-bowl gives **ب** (1 dot below), **ن** (1 above),
  **ت** (2 above), **ث** (3 above); the dots do all the work. Anchored on **أنت**,
  which is *alif · nūn · tāʾ*. Phoenician tie-back: *nun*/*taw* → **N**/**T**.
- **W05 — yāʾ + "my name"**: teaches **ي** (two dots below — closing the
  below/above dial) as a **triple-threat** (consonant *y*, long vowel *ī*, and the
  possessive **-ī** "my"; Phoenician *yod* "hand" → *iota* → **I/J**), then
  **assembles اسم → اسمي** ("name" → "my name") entirely from letters taught in
  W01–05.
- **W06 — the ḥarakāt & the hamza**: pays off W01's *abjad* claim by showing the
  optional short-vowel marks (*fatḥa* above = a, *kasra* below = i, *ḍamma* above =
  u; + *sukūn*/*shadda*). The showpiece: **أنتَ** ("you," m.) vs **أنتِ** ("you,"
  f.) differ by a **single mark** (*fatḥa* vs *kasra*) — the Ch. 2 gender split is
  one dash. Also introduces the **hamza** (**ء**, glottal stop) riding an *alif*
  seat (**أ**).

## Chapter 1 — Writing set (break the script apart and draw it)

- **Writing lessons added** (`AR-W01-direction-and-alif`,
  `AR-W02-joining-sin-lam`, `AR-W03-dots-mim-ba-salam`): a `writing`-type
  companion to the Ch. 1 greetings, matching the Russian/Cyrillic writing set —
  the hand-writing track the language-ladder app renders. Anchored on
  the real Ch. 1 word **سلام** (*salām*, the "peace" inside *as-salāmu ʿalaykum*).
  No `concept_tag` (writing lessons are exempt from the concept join).
- **W01 — direction + abjad + alif**: leads with the two big surprises before
  any stroke — Arabic runs **right-to-left** (the older habit, shared with
  Hebrew/Phoenician) and is an **abjad** (short vowels normally unwritten; the
  word *abjad* = *a-b-j-d* recited, exactly like *alphabet* = *alpha-beta*).
  Then **ا** (*alif*), one clean downstroke, one of the six non-joining letters.
  Cousin payoff: *alif* ← Phoenician **ʾālep** ("ox") → Greek *alpha* → Latin **A**.
- **W02 — cursive joining**: the biggest difference from Latin/Cyrillic print —
  a letter wears up to **four coats** (isolated/initial/medial/final). Taught on
  **ل** (*lām* ← *lāmed*, cousin of **L**) and **س** (*sīn* ← *shin* "tooth" —
  the shape *is* teeth), plus the obligatory **لا** *lām-alif* ligature and the
  joined pair **سل**.
- **W03 — dots as a piece + first whole word**: the truth-table of the shared
  bowl skeleton (ح/ب/ن/ت/ث differ **only by dots**), so the dot under **ب** is
  what draws the letter, not decoration. Teaches **ب** (*bāʾ* ← *bēt* "house" →
  **B**) and **م** (*mīm* ← *mem* "water" → **M**), then **assembles سلام**
  right-to-left — including the pen-lift after *alif*. Payoff: *ʾālep*/*bēt*/*mem*
  = ox/house/water = **A**/**B**/**M** — Arabic, Greek, and Latin are one family
  tree.

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*ismī … / mā ismuka?*),
  atom-first, Arabic inline, RTL (`lessons/AR-C02-*`,
  `book/chapters/ch02-introductions.tex`). Built from Ch. 1's letters/roots:
  - **اسم** ism ("name") ← Semitic root *s–m–w*; cousin of Hebrew *shem*, **not**
    the Indo-European *name/nōmen*.
  - **ي** *-ī* ("my") — a glued possessive suffix (like *al-*).
  - **اسمي …** — **"my name is…"**; the **zero copula** (no "is") — shared with
    the Dravidian languages, a meeting of two unrelated families.
  - **أنت** anta/anti — "you," split by **gender** (not register, unlike the
    European/Dravidian tracks); introduces ت (tāʾ) and أ (alif-hamza).
  - **ما** mā ("what").
  - **ما اسمك؟** — **"what's your name?"**; the *-ka/-ki* "your" suffix, gendered.
  - **تشرفنا** tasharrafnā — "pleased to meet you" ("we are honoured," root
    *sh–r–f* → *sharīf*, English *sheriff*).
  - **practice** — the whole dialogue.
- Example names are invented (Mira / Arun). Book compiles clean with XeLaTeX.

## Reworked to inline-letters: one Chapter 1, script taught within the words

Replaced the standalone reading-course structure (Ch1 reading course + Ch2
greetings) with the inline-letters model the rest of the curriculum now uses —
per `HL00`'s updated rule and direct user feedback ("introduce the letters as
you introduce words that use them… I do not want people to sit through a reading
course before they start").

- **Merged into a single `Chapter 1 — Greetings`** (`lessons/AR-C01-*`,
  `book/chapters/ch01-greetings.tex`): salām → marḥaban → al- → as-salāmu
  ʿalaykum → ṣabāḥ al-khayr → masāʾ al-khayr → shukran → practice. Each word
  lesson now carries a *"The letters in this word"* section teaching only the
  new letters that word needs (RTL, connecting letters, dots-on-a-skeleton, the
  emphatic ṣ, ʿayn, hamza), so reading + meaning + root arrive together. The
  root engine (k-t-b) and the three script facts moved into the chapter intro.
- **Removed** the old `book/chapters/ch01-reading.tex` + `ch02-greetings.tex`
  and the `AR-C01-read-*` / `AR-C02-*` lesson files (their content folded into
  the new inline lessons).
- **Beginner-audience fixes** (HL00 Audience rule): the preface and appendix no
  longer say "you already read Arabic (rusty)" or assume the reader knows
  Spanish; the Spanish-loanword thread is now self-contained enrichment.
- Retitled the book's green callout box from "Sounds & script you'll need" to
  "The letters in this word." Removed HL00's note that Arabic was still to be
  reworked (now done). Book compiles clean with XeLaTeX.

## Reading course: Chapter 1 = learn to read, greetings → Chapter 2 (superseded)

Reworked after feedback that the first draft "dropped a bunch of words but
never taught how to actually read any of it" — a vocabulary list, not
reading, and a break from the atom-first playbook.

- **Chapter 1 is now an incremental reading course** (`lessons/AR-C01-read-*`):
  a few letters per lesson, each cashing out in a real, decodable word —
  ل+ا → **لا** ("no"), then م/س → **سلام**, ب/ر/ح → **مرحبا**, ص/خ/ي → **صباح
  الخير**, ش/ك → **شكرا**, ع/ء → **السلام عليكم**/**مساء الخير**, then a reading
  recap. ~15 letters, half the alphabet, each welded to a word. RTL, letter
  connection, dots-on-shared-skeletons, one-way letters — all taught inline as
  the words need them.
- **The greeting lessons moved to Chapter 2** (`lessons/AR-C02-*`) — same
  content (root system, salām family, al-/sun-moon letters, the greetings and
  replies), now that the learner can read the words. The book's chapters were
  restructured to match (ch01-reading + ch02-greetings; LaTeX auto-renumbers).
- `HL00` updated to codify: **for any non-Latin script, Chapter 1 is a reading
  course** (letters → words, incremental), not a gated alphabet chart and not
  word-first.

## Chapter 1 — Greetings (initial draft, superseded by the reading course)

- New Arabic track on the HL00 framework: one word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. First **right-to-left** track and
  first to use a **vendored font** (Noto Naskh Arabic, static instance, loaded
  by relative `Path=` so local and CI builds match).
- Chapter 1 (`lessons/AR-C01-*`), built around Arabic's own structure:
  - **the root system** — the three-consonant root engine (k-t-b → kitāb/
    kātib/maktab), Arabic's version of the curriculum's root obsession, made
    literal; plus the RTL/connecting-script refresher inline
  - **مرحبا** marḥaban (root r-ḥ-b, "there's room for you") · **سلام** salām
    (root s-l-m — salām/islām/muslim; Hebrew *shalom* the same Semitic root)
  - **ال** al- ("the"; the Al-Andalus loanword web — algebra/alcohol/azúcar;
    sun/moon-letter assimilation) · **السلام عليكم** as-salāmu ʿalaykum
  - **صباح** ṣabāḥ · **خير** khayr · **صباح الخير** ṣabāḥ al-khayr (reply
    ṣabāḥ an-nūr) · **مساء** masāʾ · **مساء الخير** masāʾ al-khayr
  - **شكرا** shukran (root sh-k-r; shākir/mashkūr reuse the kātib/maktūb
    patterns) · **practice**
- Grounds each word against English and Spanish; foregrounds Arabic's shadow
  over Spanish (al-, azúcar). Book compiles clean with XeLaTeX (13 pages).
- Added a shared `_fonts/` dir (vendored static Noto fonts + OFL license) for
  this and the later Indic-script tracks.
