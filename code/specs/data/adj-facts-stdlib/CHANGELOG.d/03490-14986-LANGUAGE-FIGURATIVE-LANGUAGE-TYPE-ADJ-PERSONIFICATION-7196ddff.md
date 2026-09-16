- **#14986: `language/figurative-language-type.adj` — personification and hyperbole stop being warranted by the metaphor sentence.**
  The envelope was the metaphor definition, so every answer's primary source was a sentence about
  metaphors, and the other two definitions were header prose that reached no answer.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's "Figurative Language Examples" article (HTTP 200; a nonsense path
  on the same host returns a real 404 holding none of these spans), with inline tags removed without a
  space and only ASCII whitespace collapsed.
  - **Each row takes the definition that names its own figure of speech.** Each occurs **exactly
    once**, inside one innermost `<p>`, scripts set aside and over the whole file, zero on the nonsense
    page, and with no `<head>` copy. No row needs a widened span or a corroboration — the first table
    in this batch where every span names its row key unaided.
  - **The envelope** is the article's framing sentence — *"Figurative language is a type of
    communication that does not use a word's strict or literal meaning."* It names no figure of speech
    and no description word.
  - Every row's page is the envelope's, so no row restates a locator line or trust.

  ### The curly apostrophe is load-bearing, and now it is pinned per row

  The metaphor definition contains **U+2019** in "isn't". Measured: the shipped form occurs once and an
  ASCII-apostrophe variant occurs **zero** times. This file already carried a dedicated
  glyph-for-glyph test, because the ASCII form once shipped and so could not be found on its own page.

  **That test had to be re-pointed, not merely kept.** It pinned the *hyperbole* answer's binding
  against the *metaphor* citation — which was only correct because the metaphor sentence was the
  envelope and rode on every answer. That is the defect being removed, so the pin now binds the
  metaphor answer to the metaphor sentence, and the table-shape test additionally asserts that an
  ASCII-apostrophe span never ships.

  This is the mirror image of `soil-texture-class` (#15324), where plain spaces were typed and the page
  writes U+00A0, so all three shipped spans occur nowhere. Same failure mode, opposite direction —
  which is why spans are extracted from the fetched page rather than retyped.

  ### Pins

  - **Kept:** the direct bind, the reverse bind, and the `allusion` abstention.
  - **Inverted:** `contains("grammarly.com") && contains("\"trust\":\"consensus\"")` — satisfied by any
    Grammarly citation and by a truncated span. Each answer is now pinned to its own whole citations
    array, closing on both the corroborations `]` and the citations `]` (#14735).
  - **Re-pointed:** the glyph-for-glyph pin, as above.
  - **Added:** a per-type check with negative arms and an assertion that each span *names* its own type;
    a table-shape test pinning the `consensus` tier, the envelope, and the absence of an
    ASCII-apostrophe span.

  **10 of 10 mutants killed, two controls:**
  - the metaphor apostrophe "repaired" to ASCII, which occurs nowhere on the page;
  - the metaphor row reverted to a bare row that inherits the envelope;
  - the hyperbole row reverted to the metaphor span (the exact defect being fixed);
  - the personification row reverted to the metaphor span;
  - the old envelope restored;
  - the trust tier swapped to `authoritative`;
  - a table-level `cites` re-added;
  - a description atom rebound;
  - the envelope naming a figure of speech;
  - the hyperbole span truncated before its emphasis clause.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer.** The shipped query example returns 2 answers and 1
  abstention; the framing sentence occurs zero times in the output, and so does the metaphor sentence,
  which used to ride on every answer.

  The query example needed no change — bare queries, no prose about the citation shape. The README row
  now records the conversion, as its #14986 siblings do.
