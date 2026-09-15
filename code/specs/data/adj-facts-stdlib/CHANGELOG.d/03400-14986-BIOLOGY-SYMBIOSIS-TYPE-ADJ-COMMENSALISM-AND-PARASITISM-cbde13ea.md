- **#14986: `biology/symbiosis-type.adj` — commensalism and parasitism stop being warranted by the mutualism sentence.**
  The envelope was the mutualism row's own sentence, *"Finally, where both parties benefit, the
  relationship is described as mutualistic."*, so it was the primary source of all three answers. The
  header said the table's `source` "carries the first row's (mutualism) quote" and documented the other
  two in the header.

  ### What each row carries now

  Measured 2026-09-15 on Wikipedia's "Symbiosis" article (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes the sentence its header already quoted** as a `source`. Each names its type and
    what defines it.
  - **The parasitism sentence** is followed on the page by a "[49]" citation marker. The marker is not
    part of the quote.
  - **The envelope** is now the page's *"Symbiosis is diverse and can be classified in multiple ways."*
    It names no type and no description.
  - Each of the three sentences and the envelope occurs **exactly once**, inside a `<p>`, both with
    script and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4). The trust tier stays `consensus`.

  ### Header corrections

  - **"Standalone defining sentence".** The provenance note said each type has one. The mutualism
    sentence opens "Finally," as the last of a list of relationships, and the note now says so.
  - **The first-row and WebFetch wording.** The "carries the first row's quote" and "WebFetch-verified
    across two passes" wording is replaced by the measurement.

  ### Pins

  The test keeps every behaviour it shipped with, including the whole-sentence mutualism pin. Its
  `contains("en.wikipedia.org") && contains(trust)` check (#15209's shape) becomes one answer whose
  citations array holds exactly the mutualism sentence. Added:
  - every type's answer is one answer whose citations array holds exactly its own sentence, with no
    other type's sentence and not the envelope;
  - a table-shape test, including that the "[49]" marker is not part of the parasitism quote.

  **9 of 9 mutants killed, two controls:**
  - the parasitism sentence given the page's "[49]" marker;
  - the commensalism row warranted by the mutualism sentence;
  - the parasitism `source` demoted to `cites`;
  - the mutualism row reverted to a bare row that inherits the envelope;
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
