- **#14070 installment 1: four shipped `source` values repaired — citations that did not appear on
  their own pages.** This is a different defect class from #13934: those are gaps between what a
  header documented and what the data carried; these are **citations the engine returns to callers**
  which are not verbatim spans.

  | library | defect |
  |---|---|
  | `geometry/quadrilateral-secondary-property:62` | dropped subject (**propagated**) |
  | `chemistry/subatomic-particles:79` | two-halves elision |
  | `astronomy/space-rock-alt-name:56` | elision **+** flattened curly quotes |
  | `astronomy/space-rock-stage:57` | flattened apostrophe (one character) |

  **Two of the four carry no ellipsis and were invisible to the screen that found the rest.** Both
  were found by accident. `quadrilateral-secondary-property` inherited `quadrilateral-types`' header
  quote, which dropped its subject — correcting that header in batch 5a did nothing for this derived
  copy. `space-rock-stage` surfaced while checking a suspected propagation that did not exist (the
  two space-rock libraries quote different sentences); its apostrophe is ASCII where the page renders
  U+2019, so the citation did not appear on its own page. That is the concrete basis for treating
  #14070's list as a **floor** rather than an inventory.

  The two space-rock libraries ship together deliberately — same page, same fetch, same class.
  Splitting them would repeat the `rectangle` mistake in miniature: fix one location, leave an
  equivalent one nearby, and "later" does not come.

  *** `chemistry/mixture-types` WAS IN THIS INSTALLMENT AND WAS TAKEN OUT, BECAUSE MY REPAIR WAS
  WRONG. *** Its damaged value joins five clauses with four ellipses, and **each elided clause
  grounds a different row** (`colloid → milk`, `suspension → salad_dressing`,
  `heterogeneous → vegetable_soup`…). I replaced it with a verbatim 230-character block that grounds
  **none** of them — fixing the ellipsis while destroying the evidence. The page states each fact in
  its own block, so the correct remedy is per-row `cites` (the `engineering-design-step` pattern),
  which makes it the same shape as `mitosis-phase-order` and not a mechanical repair at all.

  **A pre-existing anchored pin caught it**, and that pin deserves note: it was a correct,
  whole-value, JSON-key-anchored assertion — exactly the discipline applied throughout this effort —
  and what it was faithfully protecting was a **constructed span**. An anchored pin defends whatever
  it is pointed at, including a defect. It did its job by refusing my change; the change was the
  problem.

  A `coverage_delta` check now runs over every repair, comparing how many row values the old and new
  values each ground. A repair may improve coverage or leave it unchanged; it must never reduce it.
  All four shipped here hold steady (2→2, 3→3, 2→2, 2→2).

  Every repaired value gets its own pin — `muscle-body-aspect`'s three tests passed *identically*
  before and after its repair last batch, because its assertions are bare `contains(…)` scans. Four
  restore-the-defect mutations pass, including the `space-rock-stage` case where damaged and repaired
  differ by **one character**.

  The propagation screen also needed a fix: its bare-pronoun needles matched by substring, but those
  repairs *prepended* a subject-naming sentence, so the damaged form is a suffix of the repaired one
  and the screen flagged correct values forever. Needles now carry a match mode. All ten known header
  defects now read `contained`. The interior-ellipsis count over 563 shipped values falls **7 → 5**
  for this change (8 originally; `muscle-body-aspect` was fixed in batch 5e). I first wrote "8 → 4",
  which was measured while the withdrawn `mixture-types` repair was still applied — re-running the
  screen after reverting it corrected both numbers. `space-rock-stage`'s defect carries no ellipsis,
  so it never appeared in this screen at all.

  533 test binaries / 1616 tests green, clippy `-D warnings` clean; all five touched `.query.adj`
  companions parse, run and abstain correctly.

