- Six more e2e suites gain a test that reads the shipped `.adj` and asserts **which rows share a
  span**, against a declared set (#15193): `brain-parts` (15 rows, 10 sharing the brainstem
  sentence), `element-groups` (27 rows in 5 family groups, no row alone), `joint-types`,
  `hormone-glands`, `speleothem-alt-name` (5 rows sharing the flowstone sentence) and
  `reference-lines` (2 rows, 1 sentence).

  With the two in the sibling entry, **8 of the 17 tables converted under #14986 now assert
  their own sharing structure, up from 2 counted on 2026-09-14.** The remaining 7 share no span
  at all — every row already has a distinct sentence — which needs a different assertion
  (all-distinct) and is deliberately not lumped in here, so one generator's bug cannot pass as
  coverage for two different properties.

  **11 fire-proofs, 11 behaved as intended.** Each table gets a row made to JOIN a group it does
  not belong to and a row made to LEAVE a declared group; `reference-lines` has only the LEAVE
  arm, since with two rows and one sentence there is nowhere to join from. The harness names
  the test that must redden, so a kill by some other assertion is reported as a failed
  experiment rather than counted as success. Baselines green.
