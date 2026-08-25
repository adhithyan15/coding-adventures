### Changed - source-bounded exam inventory completeness (#12230)

- Require every inventory to declare sourced boundaries for communicative
  functions, grammar, phonology/orthography, and lexicon. A file is complete
  only when all four dimensions are complete.
- Keep valid partial inventories measurable: their enumerated exam points still
  generate coverage work, while their presence no longer suppresses the
  `exam-inventory` backlog item.
- Migrate the Spanish, French, and German A1 inventories without inflating their
  claims. The measured baseline is now 0 complete and 3 partial of 138 targets;
  Spanish still covers 85/85 currently enumerated points, but that 100% is not
  misreported as a complete A1 construct.
- Reject missing dimensions, extra dimensions, empty provenance, empty boundary
  notes, identity mismatches, malformed metadata, and malformed point labels at
  the strict loader boundary.

