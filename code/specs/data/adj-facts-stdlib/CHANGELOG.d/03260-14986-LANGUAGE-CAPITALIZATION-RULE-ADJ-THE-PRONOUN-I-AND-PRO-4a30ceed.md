- **#14986: `language/capitalization-rule.adj` — the pronoun-I and proper-noun rules stop being warranted by the first-word sentence.**
  The envelope was the first-word rule's own sentence, *"Here’s an easy rule to follow—whenever you
  start a sentence, capitalize the first letter of the first word."*, so it was the primary source of
  all three answers.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's "Capitalization Rules and Examples" article (HTTP 200; a nonsense
  path on the same host returns 404), with inline tags removed without a space and only ASCII
  whitespace collapsed. The nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** Each of the three sentences the header already
    quotes names its trigger and what it requires, and each occurs **exactly once**, inside a `<p>`.
  - **The envelope** is now the article's *"Knowing which types of words to capitalize is an important
    part of learning English capitalization rules."* It occurs once in a `<p>` and names no rule.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  The page states the proper-noun rule **twice**. The `<p>` sentence the header quotes is the one the
  row carries. A list item near the top adds a parenthetical, *"Proper nouns (specific names for a
  particular person, place, or thing) are always capitalized …"*. That is a different string, and it
  also occurs once.

  ### The header

  All three quotes match the page, so none is changed. A paragraph records the per-row shape, the new
  envelope and the measurement. The "WebFetch-verified before writing (twice, across two cycles of
  this loop)." note is now marked as the original authoring note, superseded by the raw-page
  measurement.

  ### Pins

  The test keeps every behaviour it shipped with, including the existing whole-sentence pin on the
  first-word citation. Its `contains("grammarly.com") && contains(trust)` check (#15209's shape)
  becomes the pronoun-I row's whole contiguous citation. Added:
  - every rule's answer is one answer carrying its own sentence, with no other rule's sentence and not
    the envelope;
  - the proper-noun row carries the `<p>` sentence, not the list item's parenthetical form;
  - a table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the pronoun-I row warranted by the first-word sentence;
  - the proper-noun row given the list item's parenthetical form;
  - the proper-noun `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a row `locator` added pointing elsewhere;
  - a description atom rebound;
  - the first-word sentence's EM DASH replaced by a hyphen;
  - a rule named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
