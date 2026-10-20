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

