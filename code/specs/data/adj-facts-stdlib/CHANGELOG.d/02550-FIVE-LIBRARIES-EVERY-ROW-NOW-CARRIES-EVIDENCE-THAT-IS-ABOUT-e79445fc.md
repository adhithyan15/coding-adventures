- **Five libraries: every row now carries evidence that is ABOUT that row.** Fixes the first batch of
  issue #13931. 12 `cites` clauses added across `anatomy/brain-parts`, `anatomy/heart-chamber-vessel`,
  `anatomy/joint-formed-by`, `anatomy/skin-layer-function` and `meteorology/cloud-signal`.

  Each had ONE table-level citation naming only ONE of its table's categories, so rows in the other
  categories were returned evidenced by a sentence about a DIFFERENT SUBJECT — the same shape as
  #13928. `heart-chamber-vessel` returned `left_ventricle -> aortic_valve` cited by a sentence about
  the RIGHT ATRIUM. `brain-parts` was worst: one sentence about the CEREBRUM was the sole evidence for
  fifteen rows, twelve of them brainstem functions.

  *** brain-parts DRAWS ON THREE SEPARATE NIH/NCI PAGES, which is the concrete demonstration that
  `cites ... locator ...` CARRIES ITS OWN LOCATOR *** — the capability the 105 headers in #13934 denied
  exists, and which the README now teaches. Verified in running output, not argued.

  EVERY SENTENCE WAS RE-EXTRACTED FROM ITS LOCATOR AND VERIFIED VERBATIM AGAINST RAW HTML, never copied
  from the library's own header. That is not ceremony: `brain-parts`' header quote for the hippocampus
  was DAMAGED TWO WAYS — it silently changed the page's double quotes around "flash drive" to single
  quotes, AND truncated mid-sentence, dropping the clause that WITHDRAWS the metaphor ("but it is far
  more complex in structure and function than a flash drive"). As the header had it, NIH appears to
  ENDORSE a metaphor the page is explicitly qualifying. Copying it would have shipped an overclaim, and
  an anchored pin would then have locked it in permanently.

  TWO QUOTES ARE DELIBERATELY WIDENED PAST AN ANAPHOR. `cloud-signal`'s cirrus evidence began "They
  often are the first sign..." and `joint-formed-by`'s saddle evidence began "One example is..." —
  both perfectly clear on the page, where a heading or the previous sentence supplies the subject, and
  both meaningless in a citation envelope, which is read DETACHED from its page. Each is widened to
  include its antecedent rather than replaced, so the quote stays on the same page and paragraph.
  A CITATION MUST NAME ITS OWN SUBJECT.

  What this buys: every row's supporting sentence is now PRESENT. It is still not ATTRIBUTED per-answer
  — provenance remains table-level (#13893 unchanged). The gain is the difference between evidence that
  is INCOMPLETE and evidence that is WRONG.

  All five `.query.adj` companions still parse and run. 533 test binaries / 1592 tests green, clippy
  -D warnings clean.

