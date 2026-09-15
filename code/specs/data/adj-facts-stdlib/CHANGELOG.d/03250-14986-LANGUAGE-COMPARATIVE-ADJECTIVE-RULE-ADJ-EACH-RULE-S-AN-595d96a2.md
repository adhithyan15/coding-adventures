- **#14986: `language/comparative-adjective-rule.adj` — each rule's answer carries the sentences it can support, and a header that called itself byte-for-byte is corrected.**
  The envelope was rule 1's own sentence, *"For most adjectives with one syllable, simply add the
  suffix –er …"*, so it was the primary source of all five answers, including the rules for -e, -y,
  consonant-vowel-consonant and two-syllable adjectives.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's "Comparative Adjectives" article (HTTP 200; a nonsense path on
  the same host returns 404), with inline tags removed without a space and only ASCII whitespace
  collapsed. Every sentence below occurs exactly once on the page.
  - **Rules 1, 2 and 4 take a per-row `source`.** Each has one sentence naming both the spelling
    pattern and the action. Rule 2's is its first sentence only, *"If a one-syllable adjective already
    ends in -e, just add an -r at the end."*
  - **Rules 3 and 5 `cites` two sentences each.** Rule 3's action sentence names its pattern only as
    "these". Rule 5's -er/-ow half and its -le half are separate paragraphs, and neither states the
    combined row. The envelope stays their primary source.
  - **The envelope** is now the article's *"In theory, any adjective can become a comparative
    adjective, as long as you follow the rules."*, which names no rule. Every row's page is the
    envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md` §4).

  ### The header's quotes did not match the page

  The header said the page was "read … byte-for-byte" and that all five quotes "appear verbatim".
  Measured, four quoted sentences did not:
  - rule 2 wrote an EN DASH in "add an –r", where the page has HYPHEN-MINUS;
  - rule 2 wrote a straight apostrophe in "don't", where the page has U+2019;
  - rule 5 wrote HYPHEN-MINUS in "-ow" and in "-le", where the page has EN DASH.

  The claim is withdrawn. The header's quote table now carries each sentence in the page's own
  characters, and names which row shape each rule takes.

  ### Pins

  The test keeps every behaviour it shipped with: the direct and reverse recalls, the silent-e,
  CVC-doubling and two-syllable rules, and the honest abstention on an untabled rule. Each loose
  citation needle becomes the whole contiguous citation run for that row's shape. Three checks are
  added:
  - every rule's answer is one answer carrying its measured citation shape, with no other rule's
    sentence, and the envelope is absent from the three `source` rows;
  - the rows carry the page's dashes, and the header's old dash forms appear nowhere in the table;
  - the table's shape: three row `source`s, four row `cites` at the article, a framing envelope that
    names no rule, no table-level `cites`, no row locator line or trust.

  **14 of 14 mutants killed, two controls.** The mutants:
  - rule 2 given the header's EN DASH before "r";
  - rule 5 given the header's hyphen before "ow", and separately before "le";
  - rule 3's `cites` promoted to a `source` of its action sentence alone;
  - rule 5 losing its -le sentence;
  - rule 3's sentences in the wrong order;
  - rules 1 and 4 swapping sentences;
  - rule 2 demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a `cites` locator repointed;
  - the trust tier flipped;
  - a description atom rebound;
  - a rule appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced from
  the repo.

  Unlike the cloud tables, the envelope's wording does reach answers: it is the primary source of the
  rule 3 and rule 5 answers, and the contiguous citation pins cover it there. The old-envelope and
  envelope-names-a-rule mutants were each killed by the rule 3 and rule 5 answer tests, not only by
  the file-shape test. The query example asks
  only about rules 4 and 1 and an untabled rule, so its output carries no rule 3 or rule 5 citation
  and does not show the envelope.
