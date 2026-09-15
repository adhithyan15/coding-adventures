- **#14986: `geography/landform-secondary-feature.adj` — each answer stops carrying all four landforms' definitions.**
  The envelope was valley's own definition, and the other three were attached as table-level
  `cites`, so every answer, valley's included, carried all four definitions and was primarily
  sourced to valley's. Each row now carries a row-level `cites` holding its own landform's
  definition, and both canyon rows cite the one canyon definition, which states both features. The
  envelope is now the thesaurus's own description, *"Types of named geographic features."*, which
  names no landform. The table is deliberately **not** converted to per-row `source`.

  ### What the page's structure forced

  Measured 2026-09-15 on the USGS-hosted Alexandria Digital Library Feature Type Thesaurus page
  (HTTP 200; a nonsense path on the same host returns 404). Each definition is a
  `<span class="scope">` inside a `<div>` following a separate term link (`valleys`, `plateaus`,
  `canyons`, `plains`), and never names its landform. That is #13934's held question, decided as in
  `physics/energy-form-family.adj`. Each definition and the new envelope occur **exactly once** in
  the page's visible text. The envelope is the italic line directly under the thesaurus's title.

  **An apostrophe that looked wrong and was not.** The valley definition's raw-HTML count was
  **zero**. The page writes `Earth&#039;s`, an entity that renders as the straight apostrophe the
  table ships, so the zero came from the entity. Rendered, it occurs once. `language/idiom-meaning.adj`
  had the opposite result for the same question.

  The header said the spans were reproduced "byte-for-byte" with "no new WebFetch". That paragraph
  is replaced by the raw-fetch measurement above.

  ### Pins

  - The shipped test's two repair pins, the plateau definition as the page's whole 399-character
    sentence and the canyon definition with **no** full stop, are carried forward. Before, they were
    pinned inside the table-level corroboration list every answer carried. Now each is pinned on
    its own row's answer as that row's only corroboration.
  - The two loose `contains("apps.usgs.gov") && contains(trust)` checks (#15209's shape) are
    replaced by the whole contiguous citation run.
  - Three tests are added:
    - every row's answer is one answer, with the envelope primary, its own landform's definition the
      only corroboration, and no other landform's definition;
    - no row carries a `source` and no `cites` sits at table level, where it would reach every
      answer;
    - the envelope is not valley's definition and names no landform.

  **A check that would have passed over empty output, caught by its own guard.** The per-row loop first
  asked fully ground queries, `landform_secondary_feature(valley, contains_stream_with_outlet)`. The
  CLI ranks a fully ground query as a **hypothesis** and emits no citations at all, so the "no other
  definition reaches this answer" arm would have passed on empty output. The loop's one-answer count
  failed instead (0 answers, not 1). The loop now binds the landform from the feature; every feature
  atom is distinct, so each query has exactly one answer, even for canyon's pair.

  **13 of 13 mutants killed, two controls.** The mutants:
  - the valley row citing plateau's definition;
  - the `steep_sides` row citing plain's;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row `cites` promoted to `source`;
  - a row `source` added at twelve spaces;
  - one canyon row's `cites` deleted;
  - the canyon definition given a full stop;
  - the plateau definition truncated mid-sentence;
  - a locator repointed;
  - the trust tier flipped;
  - a feature atom rebound;
  - a landform appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **One arm was never observed to fire on its own.** As in `idiom-meaning` and `mitosis-phases`, the
  "envelope names no landform" loop checks the test's `ENVELOPE` constant. The mutant that put a
  landform into the envelope was caught first by the envelope-equality assertion. It is kept as a
  guard for an edit that changes both, and claimed for nothing more.
