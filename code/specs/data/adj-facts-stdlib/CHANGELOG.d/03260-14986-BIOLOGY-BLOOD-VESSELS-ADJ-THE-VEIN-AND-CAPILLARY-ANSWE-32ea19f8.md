- **#14986: `biology/blood-vessels.adj` — the vein and capillary answers stop being warranted by the artery sentence.**
  The envelope was the artery row's own sentence, *"Arteries carry blood away from the heart."*, so it
  was the primary source of all three answers.

  ### What each row carries now

  Measured 2026-09-15 on SEER's "Classification & Structure of Blood Vessels" page (HTTP 200; a
  nonsense path on the same host returns 404), with inline tags removed without a space and only ASCII
  whitespace collapsed. The nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** Each of the three sentences the header already
    quotes names its vessel and its function, and each occurs **exactly once**, inside a `<p>`.
  - **The envelope** is now the page's *"Blood vessels are the channels or conduits through which blood
    is distributed to body tissues."* It occurs once in a `<p>` and names no vessel and no function.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### The header

  The paragraph that said an ADJ `table` carries ONE provenance envelope, holding "the single cleanest
  span", is replaced by the per-row account and the measurement. The header's two non-row quotes are
  reported as measured rather than assumed. The capillary quote's "connection" fragment is part of
  a longer sentence that occurs once. The "closed system" framing quote does **not** occur on the
  cited page. The header attributes it to the parent module, which was not fetched, so it is marked
  unverified. Neither is a row's source.

  ### Pins

  The test keeps every behaviour it shipped with: the forward, reverse and abstention recalls. Its
  artery `"source":"..."` pin becomes the whole contiguous citation, and its
  `contains("training.seer.cancer.gov") && contains(trust)` check (#15209's shape) is subsumed by it.
  Added:
  - every vessel's answer is one answer carrying its own sentence, with no other vessel's sentence and
    not the envelope;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the vein row warranted by the artery sentence;
  - the capillary sentence truncated;
  - the capillary `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a row `locator` added pointing elsewhere;
  - a function atom rebound;
  - a vessel named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
