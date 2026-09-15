- **56 libraries: the `source` citation is now pinned by its own e2e test.** A batch bite out of
  issue #13918. Rows were checked everywhere; EVIDENCE was checked almost nowhere, which is exactly
  how the truncation in #13916 survived indefinitely.

  Two groups. THIRTY-THREE were entirely unpinned and carry a hedge, quantifier or conditional in the
  citation — "except", "usually", "almost", "some", "most", "about", "only", "unless", "if" — where a
  truncation would LOSE MEANING rather than merely shorten a sentence. TWENTY-THREE MORE already had
  a FRAGMENT pin and were upgraded to the anchored form, for the reason below.

  *** THE SELECTION PREDICATE WAS WRONG THE FIRST TIME, AND REVIEW CAUGHT IT. *** #13918's census
  counted a library as covered if ANY span of its citation appeared anywhere in the suite. That
  counts fragment needles — and a fragment has neither the `"source":"` key nor a closing quote, so
  the citation can be truncated AT THAT POINT and the test stays green. The worst instance was
  `transportation/red-signal-permitted-movement.adj`, shipped two changes earlier: truncating its
  citation to end at its own fragment needle DELETES THE ENTIRE OPERATIVE PERMISSION CLAUSE — "…
  permitted to enter the intersection to turn right, or to turn left from a one-way street into a
  one-way street, after stopping" — and all four of its tests passed. The #13916 defect shape,
  surviving in the sibling of the library #13916 fixed. Verified before and after: the truncation
  passed, and now fails.

  The right predicate is FULL ANCHORED PIN, not any-span. #13918's numbers were measured with the
  wrong one and are corrected there.

  Each assertion pins the WHOLE citation, anchored on the `"source":"` JSON key and closed by the
  terminating quote, per the rule established in #13920: pinning a fragment narrows the hole rather
  than closing it, because `contains` on a fragment cannot see what precedes or follows it. Verified
  by mutation on a sample — dropping "almost" from `volcano-type`, "about" from `temperature-scales`,
  or "usually" from `flower-parts` each fails its library's suite.

  THE BATCH CAUGHT A BUG IN ITS OWN TOOLING, AND THE SUITE CAUGHT IT IMMEDIATELY. The first run
  inserted a WRONG citation into `facts_statesofmatter_e2e.rs`: `chemistry/states-of-matter.adj` and
  `physics/states-of-matter.adj` share a basename, both mapped to that one test file, and the script
  silently used whichever sorted first.

  The first fix added two guards that refused to guess (skip basename collisions; skip files with
  more than one `source` envelope), which is why the initial batch is 33 rather than 34 — only the
  collision guard actually fired. But review showed name-based mapping is unsound in general: nothing
  checked that the mapped test LOADS the mapped library, and `mathematics/constants.adj` passes both
  guards while mapping by name to a test that loads `physics/physical-constants.adj` instead.

  The second pass replaced the heuristic entirely: PARSE EACH TEST FOR THE `.adj` FILES IT ACTUALLY
  COPIES and require the target library to be among them. That subsumes the collision guard, catches
  the `constants` mismapping (its pin correctly landed in `facts_mathconstants_e2e.rs`), refuses any
  library whose owning test is ambiguous or absent, and additionally requires — when a test loads
  several libraries — that the pinned text be UNIQUE among them, since about forty citations are
  byte-identical across sibling libraries and a pin could otherwise be satisfied by a neighbour's.

  A mechanical edit across dozens of files is exactly where a plausible-looking heuristic does the
  most damage, and the only reason this surfaced in seconds rather than in review is that the full
  suite was run rather than a spot check.

  No library content changed — this is test-side only. 533 test binaries / 1592 tests green, clippy
  -D warnings clean. Falsifiability verified by mutation on a sample spanning both groups: dropping
  "almost" from `volcano-type`, "about" from `temperature-scales`, or "usually" from `flower-parts`
  each fails its suite, as does truncating `red-signal-permitted-movement` at its old fragment point.
  (When repeating this, mutate the `source "` LINE specifically — most `.adj` files also quote their
  citation in the literate `%` header, so a naive first-occurrence replacement edits a comment and
  the test correctly still passes. That produced a false "the pin does not work" reading before the
  mutation was corrected.)

