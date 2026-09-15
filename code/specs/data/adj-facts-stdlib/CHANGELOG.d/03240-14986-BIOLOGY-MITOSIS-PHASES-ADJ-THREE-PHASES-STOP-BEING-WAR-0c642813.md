- **#14986: `biology/mitosis-phases.adj` — three phases stop being warranted by prophase's line.**
  The envelope was prophase's own line, *"Chromatin is transformed into chromosomes composed of
  pairs of filaments called chromatids…"*, so it was the primary source of the metaphase, anaphase
  and telophase answers too. Each row now carries a row-level `cites` holding its own line. The
  envelope is now the page's statement of what cell division is, *"Cell division is the process by
  which cells reproduce (mitosis)."*, which names no phase. The table is deliberately **not**
  converted to per-row `source`, the same decision as `language/idiom-meaning.adj`.

  ### What the page's structure forced

  Measured 2026-09-15 on the NCI SEER "Cell Cycle" page (HTTP 200; a nonsense path on the same host
  returns 404). Each row's line is its own `<li>`, nested under a phase `<li>` ("Prophase",
  "Metaphase", …), and never names the phase. Which item a line sits under is #13934's held
  question, so each line corroborates its row rather than warranting it. The telophase line sits
  one level deeper still, under "Telophase I". Each of the four lines occurs **exactly once** in
  the raw HTML.

  ### A header paragraph built on a premise the language no longer has

  The header said *"An ADJ `table` carries ONE provenance envelope"*, and on that basis held one row's
  span as the envelope and listed the others only in `%` comments. That is the false premise #13934
  is about. The paragraph is replaced. Its twin, two paragraphs up, said each row "is fixed by its
  own verbatim span", which overclaims a warrant now that the rows only cite; it now says what a
  cited line does and does not establish.

  ### The envelope's parentheses

  On the page, "Cell division", "process" and "mitosis" are inline glossary links. My first
  extractor replaced tags with spaces and rendered the sentence as *"…reproduce ( mitosis )."*, which
  is not the page's text. Two raw needles then found it **zero** times, because link markup sits
  between the words. That zero came from the needle, not the page. A tag-tolerant match, controlled
  by a sentence known to be present (found once) and a fabricated one (found zero times), located
  it. With the tags removed and no space inserted it reads *"…reproduce (mitosis)."*, once.

  ### Pins

  The old citation check pinned the old envelope with `"corroborations":[]` and then re-checked
  `"trust":"authoritative"` as a loose second needle (#15209's shape). It is replaced by the whole
  contiguous run, and three tests are added:
  - every phase's answer is exactly one answer, with the envelope primary, its own line the only
    corroboration, and no other phase's line;
  - the table has exactly one `source` line, counted indentation-insensitively, and the envelope
    is primary on a real answer;
  - the envelope is not prophase's line and names no phase, and the shipped rows match the
    constants.

  The span constants were generated from the page-derived spans.

  **11 of 11 mutants killed, two controls.** The mutants:
  - metaphase and anaphase spans swapped;
  - the old envelope restored;
  - the envelope with the tag-boundary spaces `( mitosis )`;
  - a row `cites` promoted to `source`;
  - a row `source` added at twelve spaces;
  - one row's `cites` deleted;
  - a locator repointed;
  - the anaphase span cut to its first sentence;
  - the trust tier flipped;
  - an event atom rebound;
  - a phase appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **One arm was never observed to fire on its own.** As in `idiom-meaning`, the "envelope names no
  phase" loop checks the test's `ENVELOPE` constant. Every mutant that put a phase into the envelope
  was caught first by the envelope-equality assertion. It is kept as a guard for an edit that changes
  both, and claimed for nothing more.
