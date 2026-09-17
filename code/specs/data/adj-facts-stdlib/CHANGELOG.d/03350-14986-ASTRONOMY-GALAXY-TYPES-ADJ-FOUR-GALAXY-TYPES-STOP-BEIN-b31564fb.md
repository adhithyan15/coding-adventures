- **#14986: `astronomy/galaxy-types.adj` — four galaxy types stop being warranted by the spiral sentence, and a header paraphrase stops wearing quotation marks.**
  The envelope was the spiral row's own sentence, *"Our Milky Way is one example of a broad class of
  galaxies defined by the presence of spiral arms."*, so it was the primary source of all five
  answers. The header said so on purpose, "An ADJ `table` carries ONE provenance envelope", and only
  listed the other rows' spans. Row blocks (RS-5e) made that no longer true.

  ### What each row carries now

  Measured 2026-09-15 on NASA Science's "Galaxy Types" page (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the spans below.
  - **Spiral, barred spiral, elliptical and irregular** each take the sentence the header already quoted
    for them as a `source`.
  - **Lenticular** takes two contiguous sentences: *"Lenticular galaxies are a kind of cross between
    spirals and ellipticals. They have the central bulge and disk common to spiral galaxies but no
    arms."* The header had quoted only the second, which says "They" and names no type.
  - **The envelope** is now the page's introduction to its types, *"Scientists sometimes categorize
    galaxies based on their shapes and physical features."* It names no type and no shape value.
  - **Counts:**
    - Each of the five row spans occurs **exactly once** in the whole file, inside a `<p>`.
    - The envelope occurs once in the page body, inside a `<p>`. The page's `<head>` metadata repeats
      it as the page description, and the header says so.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header corrections

  - **A paraphrase in quotation marks.** The opening summary ("A spiral galaxy is the one with spiral
    arms; an elliptical galaxy is a smooth ball ...") sat in quotation marks. The page never writes it,
    and it occurs zero times. The marks are removed and a note says it's a paraphrase.
  - **The table's citation.** "Carrying the table's citation" becomes "carrying its row's citation".
  - **The one-envelope paragraph** is replaced by the measurement above.
  - **The barred-spiral claim.** The header said NASA "says a bar that cuts 'across their centers'".
    The page says barred spirals have "ribbons of stars, gas, and dust that cut across their centers".
    The word "bar" is only in the next sentence, which is not the row's span. The header now quotes that
    phrase, and says `bar_across_center` reads "Barred" and that phrase together.

  ### Pins

  The test keeps every behaviour it shipped with: the forward recalls of spiral, elliptical and
  irregular, the reverse recall to elliptical, and the abstention on a planet. Its
  `contains("science.nasa.gov") && contains(trust)` check (#15209's shape) becomes four answers, each
  whose citations array holds exactly its own row's span. Added:
  - every shape's answer is one answer whose citations array holds exactly its own span, with no other
    type's span and not the envelope;
  - the lenticular row carries both sentences, and no row carries "They have ..." alone;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the lenticular row cut to the "They have" sentence alone;
  - the barred-spiral row warranted by the spiral sentence;
  - the irregular `source` demoted to `cites`;
  - the elliptical row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - a shape atom rebound;
  - a type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
