## 0.1.0

- Slice 1 (in-memory log): `CVLog` with mint (`create` /
  `tryCreate`), mutate (`contribute`, `derive` / `tryDerive`,
  `merge` / `tryMerge`, `delete` / `tryDelete`, `passthrough` /
  `tryPassthrough`), query (`get`, `ancestors`, `descendants`,
  `history`, `lineage`), `entries` / `passOrder` views,
  enable/disable, compact-v1 IDs, and checked logs with
  `GraphLimits` budgets (`newCheckedCompact`), over the
  `Origin` / `Contribution` / `DeletionRecord` / `CVEntry` types.
- Slice 2 (queued): serialization (`toJsonString` /
  `fromJsonString`, canonical encoding, bounded parsing),
  `validateGraph`, fallible `tryAncestors` / `tryDescendants` /
  `tryLineage`, checked export, and the journal.
- Tests mirror 37 of the reference lib suite's 41 tests (the 4
  serialization tests await slice 2) plus 13 extras, 10 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package (op scripts across all 5 log configurations); 100%
  line coverage.
