- `facts_skeletonbones_e2e.rs` and `facts_speleothemsubstrate_e2e.rs` — each gains a test that
  reads the shipped `.adj` and asserts **which rows share a span**, against a declared set
  (#15193). Counted 2026-09-14: of the 17 tables converted under #14986, only 2 asserted their
  own sharing structure. These are the two where getting it wrong costs most —
  `skeleton-bones` is the table whose original defect was five rows pinned by nothing behind
  one shared span (#15171), and `speleothem-substrate` puts 11 rows behind just 2 sentences.

  The shape matters: one side of the comparison is the FILE. That is how these differ from the
  per-row pairing checks corrected in #15191 and #15192, which compared two test literals to
  each other and were documented as discriminators they could not be.

  Proved to fire in **both** directions per table, not assumed — a row made to JOIN a group it
  does not belong to, and a row made to LEAVE a declared group — with the specific test named
  in each case so a kill by some other assertion is not counted as success. 4 of 4 reddened the
  intended test; baselines green.
