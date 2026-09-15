- **#14986: `language/preposition-type.adj` — the time and direction types stop being warranted by the place sentence, and a header quote gets the page's apostrophe.**
  The envelope was the place row's own sentence, *"Prepositions of place show where something is or
  where something happened."*, so it was the primary source of all three answers.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's "Prepositions" article (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** Each of the three sentences names its type and
    what it shows, and each occurs **exactly once**, inside a `<p>`.
  - **The envelope** is now the article's *"However, the most common prepositions fit into four main
    categories, with a fifth category for additional types."* It occurs once in a `<p>` and names no
    type. It counts four categories; the fourth, manner/cause/purpose, is the one the header already
    explains is deliberately not tabled.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### A header quote the page never wrote

  The header's quote table, which calls its quotes verbatim, wrote the direction sentence with a
  **straight apostrophe** (U+0027) in "it's". The page writes **U+2019**, and the straight-apostrophe
  form occurs zero times. The table body never shipped that sentence, so no answer carried it, but
  the header's claim was wrong. The quote and the new row both carry U+2019. The "WebFetch-verified
  before writing" note is marked as the original authoring note, superseded by the raw-page
  measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the direct and reverse recalls and the abstention on
  manner/cause/purpose. Its `contains("grammarly.com") && contains(trust)` check (#15209's shape)
  becomes the time row's whole contiguous citation. Added:
  - every type's answer is one answer carrying its own sentence, with no other type's sentence and not
    the envelope;
  - the direction row carries U+2019, and the straight-apostrophe form is not in the table;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the direction sentence given a straight apostrophe;
  - the time row warranted by the place sentence;
  - the time sentence truncated before its parenthetical;
  - the direction `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a description atom rebound;
  - a type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
