- **#14986: `language/superlative-adjective-rule.adj` — the consonant-vowel-consonant and -y rules stop being warranted by the one-syllable sentence.**
  The envelope was rule 1's own sentence, *"If an adjective has only one syllable, most of the time you
  can simply add the suffix –est …"*, so it was the primary source of all three answers.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's "What Are Superlative Adjectives?" article (HTTP 200; a nonsense
  path on the same host returns 404), with inline tags removed without a space and only ASCII
  whitespace collapsed. Every sentence below occurs **exactly once**, each inside a `<p>`, and the
  nonsense page holds none of them.
  - **Rules 1 and 2 take a per-row `source`.** Each has one sentence naming both the spelling pattern
    and the action.
  - **Rule 3 (-y) `cites` two sentences.** The action sentence, *"To make the superlative, first change
    the y into an i and then add –est."*, names its pattern only as "the y". The pattern is the
    sentence before it in the same paragraph, *"Adjectives with either one or two syllables have
    special spelling rules if they end in -y."* The envelope stays rule 3's primary source.
  - **The envelope** is now the article's *"You can make any adjective into a superlative."*, which
    names no rule. Every row's page is the envelope's, so no row restates a locator line or trust
    (`ADJ-TABLES.md` §4).

  The pattern sentence writes "-y" with **HYPHEN-MINUS**, although the section heading above it
  writes "–y" with an EN DASH. The row carries the sentence's own character.

  ### The header

  Unlike its sibling `comparative-adjective-rule` (#15287), this header's three quotes all match the
  page exactly, so none is changed. A paragraph is added recording the per-row shapes, the pattern
  sentence, the new envelope and the measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the direct and reverse recalls and the abstention
  on an untabled rule. Its `contains("grammarly.com") && contains(trust)` check (#15209's shape)
  becomes the whole contiguous citation run, and the reverse recall gains one. Added:
  - every rule's answer is one answer carrying its measured citation shape, with no other rule's
    sentence, and the envelope is absent from the two `source` rows;
  - the rule 3 pattern sentence carries HYPHEN-MINUS, and the heading's EN DASH form is not in the
    table;
  - a table-shape test.

  **12 of 12 mutants killed, two controls:**
  - the rule 3 pattern sentence given the heading's EN DASH;
  - rule 3's sentences in the wrong order;
  - rule 3 losing its pattern sentence;
  - rule 3 promoted to a `source` of its action sentence;
  - rule 2 demoted to `cites`;
  - rules 1 and 2 swapping sentences;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a `cites` locator repointed;
  - the trust tier flipped;
  - a description atom rebound;
  - a rule named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  The envelope's wording reaches answers: it is rule 3's primary source, and the contiguous citation
  pins cover it there.
