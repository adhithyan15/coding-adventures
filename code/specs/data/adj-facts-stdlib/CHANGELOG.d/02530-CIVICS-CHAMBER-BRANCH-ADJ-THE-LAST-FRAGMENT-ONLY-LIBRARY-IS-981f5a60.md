- **`civics/chamber-branch.adj`: the last fragment-only library is pinned, and the obvious pin was
  WRONG.** Test-side only. This is the library every previous batch refused, because its `source` is
  BYTE-IDENTICAL to co-loaded `congress-chamber.adj` — `chamber_branch` is a RULE, so its provenance
  *is* the composed table's citation — and an assertion a sibling can satisfy is not a pin.

  It turned out to have BOTH defects at once. Its test carried FRAGMENT needles (bare spans with
  neither the `"source":"` key nor a closing quote, so a citation could be truncated at the fragment)
  AND the INDEPENDENT-SCAN shape that let #13928 ship (the locator and `"trust":"authoritative"`
  asserted as their own `contains` calls over the whole blob, which cannot tell which ANSWER a citation
  belongs to).

  *** MY FIRST PIN WAS WRONG, AND ONLY A DIRECTIONAL MUTATION SHOWED IT. *** I pinned the binding
  joined to the answer's first citation object, reasoning it was unique because `congress_chamber`
  never binds `B`. That reasoning was right about uniqueness and wrong about ownership:
  `citations[0]` is populated by the PREMISE's envelope, not the rule's. Truncating THIS library's
  `source` left the test PASSING, while truncating `congress-chamber.adj` broke it. The pin bound the
  SIBLING's citation under this library's name — the exact defect it was written to prevent.

  "Does a mutation fail?" would have reported green three times over. The question that mattered was
  WHICH FILE'S mutation fails. A falsification harness that does not vary the target cannot tell a pin
  from a pin on something else.

  The correct anchor is the RULE STEP, which joins `"goal":"chamber_branch(...)"` to the rule's own
  envelope — no sibling ever emits a step with that goal. All four mutations now fail: truncate the
  rule's own source, append to it, truncate the sibling's identical source, truncate the second
  premise's source.

  The original needle is KEPT as a secondary assertion, with a comment saying plainly that it pins the
  PREMISE's envelope rather than this library's provenance. An accurate weaker check is worth keeping;
  a mislabelled one is not.

  A MAXIMALLY STRICT PIN IS NOT AUTOMATICALLY THE RIGHT PIN. The full 674-character answer span was
  available and rejected: it also pins `government-branch-member.adj`'s corroborations, which would
  couple this library's test to a SIBLING's citation list and break it on unrelated edits. The two
  needles used pin the same guarantees while staying independent of what is not this library's
  responsibility.

  Census after this change, re-run to confirm rather than asserted: **67 anchored / 1 ambiguous / 0
  fragment-only / 290 no assertion / 4 multi-envelope**, of 362. FRAGMENT-ONLY IS NOW ZERO — the
  truncatable-but-green category is closed. `chamber-branch` is the single "ambiguous" entry and stays
  there honestly: its PLAIN citation really is ambiguous, and the rule-step needle that actually pins
  it is a shape the census does not model. The census was also corrected while measuring this — it
  demoted `congress-chamber` the moment a SIBLING's test quoted its sentence, because it required
  EVERY pinning test to be twin-free rather than at least one.

  533 test binaries / 1592 tests green, clippy -D warnings clean. See issues #13916, #13918, #13928.

