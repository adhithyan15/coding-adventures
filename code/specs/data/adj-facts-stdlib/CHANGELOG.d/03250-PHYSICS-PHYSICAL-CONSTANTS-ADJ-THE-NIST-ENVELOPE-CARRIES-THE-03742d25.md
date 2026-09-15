- **`physics/physical-constants.adj` — the NIST envelope carries the two non-breaking spaces the page writes.**
  The table's `source` is NIST's definition of the metre, *"The meter is defined by taking the fixed
  numerical value of the speed of light in vacuum c to be 299,792,458 …"*. As shipped, it occurred
  **zero** times on the page it cites.

  Measured 2026-09-15 on "Definitions of the SI base units" (HTTP 200; a nonsense path on the same
  host returns 404), with inline tags removed without a space and only ASCII whitespace collapsed,
  so U+00A0 is kept. Walking every divergence between the shipped string and the page found exactly
  two, both an ordinary space where the page writes **U+00A0**:
  - after "value of", before "the speed of light";
  - after "where the", before "second".

  With the page's two characters the sentence occurs exactly once, inside one `<p>`, and zero times
  on the nonsense page. Nothing else in the span changed; its U+2212 in "s−1" and its "∆νCs" already
  matched. A `git grep` of `origin/main` finds the sentence only in this table, and no test pinned it.

  This is the fourth U+00A0 span found this cycle, after `long-bone-parts`, `muscle-groups` and the
  cloud pair's stratus sentence. I have not searched the stdlib for U+00A0 defects in general.

  ### The header

  A paragraph names the two U+00A0, which a comment can't show. It also scopes the header's
  "Every value is copied verbatim" claim: that holds of the `source` span only with the two
  characters, and the row values are ADJ number literals (`6.62607015e-34`), not the page's spelling
  ("6.62607015 ×10−34").

  ### Pin

  The test's `contains("nist.gov/si-redefinition") && contains(trust)` check (#15209's shape) becomes
  the whole contiguous citation: the sentence with both U+00A0, the locator, the tier and no
  corroborations. It also checks that the envelope constant carries exactly two U+00A0 and that the
  ordinary-space form reaches no answer.

  **7 of 7 mutants killed, two controls:**
  - the first U+00A0 restored to a space;
  - the second U+00A0 restored;
  - the U+2212 in "s−1" replaced by a hyphen-minus;
  - the locator repointed;
  - the trust tier flipped;
  - a table-level `cites` added;
  - the span truncated after its value.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  ### Not changed: per-row provenance

  This table still has one envelope, the speed-of-light sentence, which is the primary source of all
  five answers (#14986). Four constants each have one `<p>` sentence on the same page naming the
  constant and its value. Avogadro's value and name are in two separate sentences. But no sentence
  on the page frames the constants without naming one, so a per-row conversion would need a framing
  decision this change does not make.
