- **#14986: `language/comma-rule.adj` — the but and direct-address rules stop being warranted by the series sentence, and the but row carries the page's closing colon.**
  The envelope was the series row's own sentence, *"When you have a list that contains more than two
  elements, use commas to separate them."*, so it was the primary source of all three answers.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's comma article (HTTP 200; a nonsense path on the same host returns
  404), with inline tags removed without a space and only ASCII whitespace collapsed. The nonsense
  page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** Each of the three sentences names its rule and
    what it says to do, and each occurs **exactly once**, inside a `<p>`.
  - **The envelope** is now the article's *"There are lots of rules about comma usage, and often the
    factors that determine whether you should use one are quite subtle."* It occurs once in a `<p>`
    and names no rule.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### A sentence that ends in a colon

  The header's truth table, which calls its quotes verbatim, ends the but sentence with a **period**.
  The page ends it with a **colon** that introduces an example pair, and the period form occurs zero
  times. The row carries the page's colon form. It is still a whole instruction naming the rule and
  its action, so it takes a `source` rather than being split into `cites`. The header's quote is kept
  and a note says it isn't verbatim. The authoring note ("WebFetch-verified twice", each quote a
  "complete, standalone rule sentence") is marked as superseded.

  ### Pins

  The test keeps every behaviour it shipped with: the direct and reverse recalls and the abstention on
  oxford_comma. Its `contains("grammarly.com") && contains(trust)` check (#15209's shape) becomes one
  answer whose citations array holds exactly the series row's whole contiguous citation. Added:
  - every rule's answer is one answer whose citations array holds exactly its own sentence's citation,
    with no other rule's sentence and not the envelope;
  - the but row ends in the page's colon, and the period form is not in the table;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the but sentence given the header's period;
  - the but row warranted by the series sentence;
  - the direct-address `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row-level `cites` added under the series `source`;
  - a row restating a different trust tier;
  - a description atom rebound;
  - a rule named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
