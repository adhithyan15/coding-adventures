### Added — Punjabi A2 core corpus case and Gurmukhi evidence for ਔ and ਢ

- `tests/corpus/punjabi/adds-punjabi-a2-core-6e2b4c91.case.ts` pins the
  Punjabi A2 core (chapters 211-224): every chapter opens on a review a driver
  can start by ear and ends on a review; `VERB-NEGATE`, `QUESTION-POLAR`,
  `VERB-PAST` and `VERB-FUTURE` each sit on exactly one grammar lesson of their
  own spine node, whose omits are now empty; the negated present keeps its
  helper, no request uses the untaught ਓ, and a third-person past doer is a
  name or ਉਸ; the five word owners cite their source; the copy-only letters ਔ
  and ਢ follow their anchor words; and the two formerly thin script atoms are
  practised twice more.
- `tests/script-inventories/gurmukhi.evidence.ts` expects 38 Gurmukhi letters
  (36 + ਔ + ਢ), neither of which carries a cited stroke order.
