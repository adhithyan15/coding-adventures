- **The stdlib README now TEACHES `cites`.** Docs-only. Adds "When one sentence does not cover every
  row: add `cites`, do NOT drop the evidence" as the sibling of the existing `rule` section, placed
  immediately before it because evidence-completeness precedes derivation.

  *** THIS IS THE ROOT CAUSE OF ISSUE #13934, AND IT IS NARROWER THAN IT LOOKED. *** `cites` appears
  52 times in this README and was TAUGHT ZERO TIMES — every mention sits inside a per-library
  description, while `rule` has had its own explanatory section all along. An author learning the
  format from this document concluded, CORRECTLY FROM THE EVIDENCE AVAILABLE TO THEM, that a `table`
  carries one piece of evidence. So the 105 of 362 headers asserting "an ADJ table carries ONE
  provenance envelope" are ONE DOCUMENTATION GAP PROPAGATED, not 105 independent mistakes.

  What the gap cost, measured: **36 libraries** name evidence URLs that appear in no `locator` and
  have zero `cites` — **123 distinct source pages** documented and encoded nowhere. In **12 of those
  36** the header URL count exactly equals the ROW count, meaning every row was verified against its
  own page and one page was kept. `biology/hormone-glands.adj` fetched twelve SEER endocrine pages and
  ships with one.

  THE AUTHORS WERE NOT CUTTING CORNERS — THEY WERE DOING MORE WORK THAN THE FORMAT APPEARED TO REWARD.
  `biology/vitamins.adj`'s header states the verification rule it followed ("any pair whose disease
  could not be verified there was dropped") and quotes all seven NIH fact sheets. The rigour was real
  and it evaporated at the boundary between the comment and the table.

  The section states the rule — if a row's own supporting sentence is not the `source`, it must be a
  `cites`; evidence living only in the header is a comment, and nothing can check it — and records the
  detail nobody knew: **`cites` carries its OWN locator, which may differ from the `source`'s page.**
  It closes with "A citation must name its own subject", covering the anaphora defect with both real
  examples found in this stdlib ("They often are the first sign...", "One example is the joint formed
  by..."), since a citation is read DETACHED from its page.

  The worked example is a CHARACTER-IDENTICAL copy of the shipped `civics/government-branch-member.adj`,
  including all five rows — deliberately not abbreviated, since a section arguing that evidence must
  cover every row should not quietly drop two of them.

  533 test binaries / 1592 tests green, clippy -D warnings clean. Fixes the cause behind #13934; the 36
  libraries themselves follow.

