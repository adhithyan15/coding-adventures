- **#14986: `anatomy/lung-lobe-names.adj` — the two left lobes stop being warranted by the right-lung sentence.**
  The envelope was *"The right lung is comprised of the right upper (RUL), middle (RML), and lower
  (RLL) lobes."*, so it was the primary source of the `left_upper_lobe` and `left_lower_lobe`
  answers, although it names no left lobe. Each row now carries **its own lung's sentence as a
  per-row `source`**. The three right lobes cite the right-lung sentence. The two left lobes cite
  *"The left lung consists of the left upper (LUL) and lower (LLL) lobes."* The envelope is now the
  paragraph's framing sentence, *"The right and left lungs' structural organization is similar,
  though asymmetrical."*, which names no lobe, and every row overrides it.

  ### `source` this time, not `cites`

  The four other tables converted this cycle carry `cites`: `idiom-meaning`, `mitosis-phases` and
  the two landform tables. Their lines never name their key, so which heading a line sits under is
  #13934's held question. Here each sentence names **both** the lung and its lobes, so it warrants
  its rows.

  **One reading is involved, and the header now says so.** *"middle (RML)"* and *"lower (RLL)"*
  attach to *"the right … lobes"* only by distributing the coordination. The abbreviations RML and
  RLL fix that reading. That is a reading, not a quotation. The precedent is the `02890` entry's
  `bacteria → gram_positive_bacteria`, kept on the same kind of reading with the header required to
  say it is one.

  Measured 2026-09-15 on the NCBI Bookshelf StatPearls page (HTTP 200; a nonsense NBK id returns 404;
  not a challenge page). Both lung sentences occur **exactly once**, raw and rendered, inside one
  `<p>` with no inline markup. A fabricated left-middle-lobe sentence occurs zero times. The framing
  sentence occurs once, with a plain straight apostrophe. Row blocks carry `source` only, and the
  locator and trust are the envelope's (`ADJ-TABLES.md` §4).

  ### The header

  - **False premise:** *"An ADJ `table` carries ONE provenance envelope"* is replaced.
  - **Quoted fragments:** the truth table's cells such as *"… middle (RML) … lobes"* had ellipses
    inside quotation marks. Inside quotation marks an ellipsis claims what the page says, and the
    page never writes those fragments. The cells now name which whole sentence each row carries.
  - **WebFetch claim:** the header said WebFetch re-verified the sentence in "THREE separate passes,
    all byte-identical". That claim is withdrawn rather than restated, since the measurement is a raw
    fetch.
  - **Stale citation line:** *"carrying the table's citation"* now reads *"carrying its own row's
    citation"*.

  ### Pins

  The shipped test's `contains("ncbi.nlm.nih.gov") && contains(trust)` check (#15209's shape) is
  replaced by whole contiguous runs: each lung's sentence as a primary source with an empty
  corroboration list. Its recalls and the trachea abstention are kept. Three tests are added:
  - every lobe's answer is one answer, whose primary source is its own lung's sentence, and which
    carries neither the other lung's sentence nor the framing envelope;
  - the two left lobes, asserted on real answers, are warranted by the left-lung sentence and never
    carry the right one;
  - the table's shape: five row `source`s, one framing envelope that names no lobe, no row
    locator or trust, no `cites`.

  **13 of 13 mutants killed, two controls.** The mutants:
  - `left_upper_lobe` warranted by the right-lung sentence;
  - `right_middle_lobe` warranted by the left-lung sentence;
  - a row's `source` deleted, so it falls back to the envelope;
  - a row `source` demoted to `cites`;
  - the old envelope restored;
  - the left sentence given an invented middle lobe;
  - a row `locator` added pointing at another NCBI id;
  - a row `trust` downgraded;
  - the envelope locator repointed;
  - the trust tier flipped;
  - a lung value rebound;
  - a lobe appended to the envelope;
  - the right sentence truncated before its final lobe.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs **zero** times in the query example's output. The two envelope
  mutants (the old envelope restored, a lobe appended) were killed only by the file-shape test. That
  test reads the shipped `.adj`, so a drift is still a failure, but no answer-level test could see
  it. Within that test, the "envelope names no lobe" loop was caught first by the envelope-equality
  assertion and was never observed to fire on its own.
