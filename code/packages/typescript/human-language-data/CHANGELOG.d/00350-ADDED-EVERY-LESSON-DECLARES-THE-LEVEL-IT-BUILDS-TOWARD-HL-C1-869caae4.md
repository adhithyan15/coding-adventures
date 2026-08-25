### Added — every lesson declares the level it builds toward (HL-C10)

- Add `src/levels.ts`: `CEFR_LEVELS` (`pre-A1` … `C2`), `deriveLessonLevel`,
  `summarizeLevels`, and `lessonsUpToLevel` — the filter a "gentle ramp to A1" edition
  applies. Published through the gap report's new `levels` section, and
  `core/exam-levels.json` records how the language-specific exams line up.
- **Derived, never authored.** A lesson sits in a realization-path segment, the segment
  names a spine node, the node declares a CEFR stage. HL08 refused to write `modality:`
  into 1,134 frontmatter files because that is 1,134 places for a computed fact to go
  stale; a level is the same kind of fact. Deriving it also means a track cannot claim A1
  by editing frontmatter — it has to actually realize the A1 spine nodes.
- **The measured answer to "how far is each track from Advanced":**

      pre-A1 657 | A1 307 | A2 0 | B1 0 | B2 0 | C1 0 | C2 0
      964 of 1,134 lessons placed (85%); 170 unmapped, all schema-v1

  **No track has reached A2**, and five (`chinese`, `japanese`, `persian`, `russian`,
  `urdu`) have not reached A1. A ramp-to-A1 edition would today contain **964 lessons** —
  as a filter over the one corpus, not a second corpus.
- Unmapped lessons report `null` and are **excluded** from a ramp edition rather than
  included by default. A wrong level is worse than a missing one: it would put material a
  reader is not ready for inside a book that promises a gentle ramp, so the honest failure
  is a shorter book.
- `core/spine.json` `stages` extends to `B2`, `C1`, `C2` so later tranches can declare
  their own stage. The project owner's direction is that the content reaches the most
  advanced level, gently, with page count explicitly not a cost.
- `core/exam-levels.json` maps CEFR onto the exams a learner would actually sit, and
  **every one of the 22 tracks is mapped — no gaps.** An unmapped track silently drops out
  of every level report, and a learner asking "what is A1 in Tamil?" deserves an answer.
- **What is recorded instead of a gap is the KIND of answer.** `basis: published` means the
  awarding body states the alignment (DELE, DELF/DALF, Goethe, CILS, CAPLE, TORFL, HSK);
  `research` means a widely-cited third-party correspondence (JLPT, Arabic ILR/ACTFL);
  `editorial` means this project's judgement — a working default to be corrected, never a
  claim about what a certificate is worth. A test enforces that every registered track has
  a mapping and a valid basis, so registering a track now requires answering the question.
- Judgement calls worth knowing: **Hindi** is anchored to the Dakshina Bharat Hindi Prachar
  Sabha ladder (Prathmic → Praveen), which is real and widely sat but built to spread Hindi
  within India rather than against CEFR descriptors. **Tamil** is mapped straight to CEFR
  because its diglossia makes any mapping unclean — this curriculum teaches the spoken
  register first, so A1 means the CEFR descriptor, not a claim about a Tamil examination.
  **Latin** takes CEFR too, with the honest note that CEFR is communicative and Latin is
  read; a reading-only ladder would fit it better. A second test requires a caveat on any
  mapping that names a specific foreign ladder without the awarding body's backing — it
  caught a bare Persian/AMFA correspondence during this change.


