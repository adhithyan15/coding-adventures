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
