## Unreleased - CV2-S5d (fixture PR): `v2-binding.json` is committed

No version bump and no change to the package's source or to any emitted corpus byte. The tables and the validator are
S5c's; this PR commits the file they derive.

### Added

- `code/specs/fixtures/warrant-corpus-conformance-v1/v2-binding.json` (176 cases: 30 accepted, 146 refused) and its
  row in `index.json`, which now holds 1,419 cases (461 in the four v1 categories, 958 in the seven v2 ones). The ten
  existing category files (four v1, six v2) are byte-identical.
- The v1 checker `code/scripts/warrant_corpus_conformance.py` requires every case id to be printable ASCII (U+0021 to
  U+007E; an escaped `é` in an id passed the old "bytes are ASCII" test) and checks a `v2-binding`
  `expect.reason` against the 39 of VER04r R85 (d), carried as a literal (the script imports nothing from the
  oracle). The closed set of the four v1 categories is scoped to them. Tests for each, and for a violating fixture
  directory being refused, are in its unittest file; one of them holds the literal equal to R85 (d) in the spec.

### Changed

- `make_conformance.py --check`: the "not committed yet" branch for `v2-binding.json` is removed (VER04r section
  16.19 R93). A missing file, a missing index row, or both together, is exit 1, and `--check` now covers all seven
  categories (958 cases). `test_conformance_binding.py` replaces the pending-branch test with one that requires exit 1
  for each of those directories.
- READMEs (this package, its `conformance-v2/` directory and the fixture family) describe the committed seventh
  category and the 1,419 total (the family README said 1243).
