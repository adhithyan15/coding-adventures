- **#14986: `astronomy/lunar-eclipse-type.adj` — the partial and penumbral types stop being warranted by the total sentence.**
  The envelope was the total row's own sentence, *"The Moon moves into the inner part of Earth’s shadow,
  or the umbra."*, so it was the primary source of all three answers.

  ### What each row carries now

  Measured 2026-09-15 on NASA's "Eclipses and the Moon" page (HTTP 200; a nonsense path on the same
  host returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** The page's mixed apostrophes are kept, as the
    header's #14070 note already requires: U+2019 in the total and penumbral sentences, U+0027 in the
    partial one. With the apostrophes swapped, each form occurs zero times.
  - **The envelope** is now the page's *"Lunar eclipses occur at the full Moon phase."* It names no
    type.
  - Each of the three sentences and the envelope occurs **exactly once**, inside a `<p>`, both with
    script and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header claims the page contradicts

  - **"Its own single opening sentence."** The provenance note said the page defines each type "in its
    own single opening sentence". The total and partial sentences do open their paragraphs. The
    penumbral one does not: its paragraph opens with "If you don’t know this one is happening, you
    might miss it." The note now says "its own single sentence" and records the exception.
  - **The blood-moon heading** was quoted with single quotes. The page writes it with double quotes, in
    its `<h2>` and in its table-of-contents link, and the single-quote form occurs zero times. The
    quote now uses double quotes.
  - **The "WebFetch-verified twice" note** is marked as superseded by the raw-page measurement.

  ### Pins

  The test keeps every behaviour it shipped with, including the curly-apostrophe citation pin. Its
  `contains("science.nasa.gov") && contains(trust)` check (#15209's shape) becomes one answer whose
  citations array holds exactly the total sentence's whole citation. Added:
  - every type's answer is one answer whose citations array holds exactly its own sentence, with no
    other type's sentence and not the envelope;
  - the rows keep the page's mixed apostrophes, and no curled partial or straightened total or
    penumbral form is in the table;
  - a table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the partial sentence's straight apostrophe curled;
  - the penumbral sentence's curly apostrophe straightened;
  - the partial row warranted by the total sentence;
  - the penumbral `source` demoted to `cites`;
  - the total row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - a description atom rebound;
  - a type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
