- **#14986: `language/idiom-meaning.adj` — 23 idioms stop being warranted by one idiom's meaning.**
  The envelope was *"Something very easy to do."*, which is `piece_of_cake`'s own meaning, so it was
  the primary source of all 23 answers, 22 of which it says nothing about. Each row now carries a
  row-level `cites` holding the page's own "Meaning:" line for that idiom, and the envelope is the
  page's definition of an idiomatic expression, which names no idiom. The table is deliberately
  **not** converted to per-row `source`.

  ### Why `cites`, not `source`

  Measured 2026-09-15 on the cited page (HTTP 200; a nonsense path on the same host returns 404, so
  the 200 means something): each idiom is a numbered `<h3>` heading, and its meaning is a separate
  `<p>` beneath it, after a `<strong>Meaning:</strong>` label, that **never names the idiom**. Which
  heading a line sits under is #13934's held question. `physics/energy-form-family.adj` (#15208) is
  the shipped precedent for that shape, and this follows it: the line corroborates, and no line is
  claimed to warrant a row. The `02950` entry below had already recorded this table as the heading
  shape.

  Unlike that precedent, the envelope here **had** to change: `energy-form-family`'s names no row,
  and this one was a row's own value. Its replacement, *"Idiomatic expressions are phrases with
  meanings that are different from the literal meanings of the words."*, occurs once in the page's
  visible text and once in the raw HTML; a fabricated variant occurs zero times.

  ### The header's "verbatim" quotes were not, for six rows

  Each of the 23 lines occurs **exactly once** in the page's visible text, directly after its own
  heading. But six contain an apostrophe, and **the page writes U+2019 in all six**. The header
  quoted them with a straight apostrophe, which occurs zero times, under a paragraph saying the lines
  were "WebFetch-verified", including a second pass "confirming the extraction is accurate and not
  paraphrased". The shipped spans and the header quotes now carry the page's curly apostrophe, and
  that verification claim is withdrawn rather than restated.

  **My own first check missed this too.** It folded curly quotes to straight before counting, so all
  six "matched". A second pass counting with no folding found it. The same first check also reported
  two rows (`pull_someones_leg`, `steal_someones_thunder`) as not following their own heading; the
  page writes *someone’s*, which the check's name match could not see. Both were the instrument.

  `cost_an_arm_and_a_leg`'s line has no terminal period, and neither does the page's.

  ### Pins

  The loose `contains("oxfordinternationalenglish.com") && contains("\"trust\":\"consensus\"")`
  citation check is replaced by the whole contiguous citation run (#15209's shape). Four tests are
  added:
  - every row's answer is exactly one answer, carries the envelope as primary and its own line as the
    only corroboration, and no other row's line;
  - the table has exactly one `source` line, counted indentation-insensitively (the envelope's), and
    the envelope is primary on a real answer;
  - the envelope is not the old one and names no idiom. It's checked against whole idiom phrases read
    from the table, because a word-level check passes vacuously on `let_the_cat_out_of_the_bag`,
    whose every word is three letters or fewer, and mine did exactly that;
  - the six apostrophe lines carry U+2019, and no shipped `cites` line carries a straight one.

  The test's span constants were generated from the spans the converter took from the page text,
  not retyped.

  **12 of 12 mutants killed, two controls.** The mutants:
  - two rows' spans swapped;
  - a straight apostrophe restored;
  - the old envelope restored;
  - the envelope reworded by one word;
  - a row `cites` promoted to `source`;
  - a row `source` added at twelve spaces;
  - one row's `cites` deleted;
  - a locator repointed;
  - a span truncated by one character;
  - the trust tier flipped;
  - a meaning atom rebound;
  - an idiom appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **One arm was never observed to fire.** The "envelope names no idiom" loop checks the test's own
  `ENVELOPE` constant. Every mutant that put an idiom into the envelope was caught first by the
  envelope-equality assertion, so an edit to the `.adj` alone can't reach that loop. It only guards
  a future edit that changes the constant and the file together. It is kept for that and claimed
  for nothing more.
