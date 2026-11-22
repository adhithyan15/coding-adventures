## Unreleased - CV2-S5b: BUILD gates the committed v2 conformance kit

No version bump, no source change, no emitted byte changes.

### Changed

- `BUILD` and `BUILD_windows` run, after pytest, `tests/fixtures/conformance-v2/make_conformance.py --check --fixture
  <family directory>` (the committed `v2-*.json` bytes and the v2 rows of `index.json` equal the derivation), the v1
  checker `code/scripts/warrant_corpus_conformance.py` (indexes and structurally checks the `v2-*` files) and its
  tests `code/scripts/tests/test_warrant_corpus_conformance.py` (VER04r 11 and 16.13 R23: none of the three ran in CI
  before). The `--check` line re-derives the `2**24` suite, about a minute more than pytest alone; it is the line the
  spec prescribes, and the committed fixture is not otherwise compared with a derivation.
- README: the two-step regeneration and what `BUILD` gates.

### Notes

- The six `v2-*.json` files themselves (734 cases) and `index.json` (1195 cases in ten rows) are committed under
  `code/specs/fixtures/warrant-corpus-conformance-v1/` by the same PR; the four v1 category files are byte-identical.
- Number 00230 is skipped: S11a reserves it.

### Fixed (S5b review, N1-N3; script-only, no emitted byte changes)

- N1: the v1 checker's `_split_result` raised `TypeReject(type_text)`, an undefined name (a latent `NameError` for a
  comma-less `Result<...>`); it now raises `TypeReject(text)`.
- N2: a malformed v2 case (not an object, missing or non-string `id`, non-object `expect`) or a `v2-*.json` path that
  is a directory is now a problem line from `structural_problems` / `v2_files`, not an exception.
- N3: every v2 case must carry `id`, `summary`, `requires` and `expect` (VER04r 11 [D41]); each missing key is a
  problem line. The committed 734 cases all satisfy it.
