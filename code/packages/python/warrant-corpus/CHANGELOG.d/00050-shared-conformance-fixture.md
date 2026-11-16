## Unreleased

- Runs the shared `warrant-corpus-conformance-v1` fixture (VER04i 9.1, WD-100):
  359 cases across `json`, `tagged`, `containment` and `label-digest`, in
  `tests/test_conformance_fixture.py`.
- **Fix:** `parse_json` accepted a lone-surrogate escape (`"\\ud800"`); it now
  rejects it in keys and values, so every lane reads the same set of documents.
- **Fix (profile alignment):** `parse_json` rejects fractions, exponents and
  `-0` (it silently produced a `float`), integers outside `+-(2**53 - 1)`, and
  nesting beyond 256 containers (it had no limit of its own).
- New `containment.relative_inside(root, child, flavor)`: a lexical check that
  takes the path flavour, so the Windows rules run on any host. `corpus_relative`
  and the binding's source-root check use it.
- New `labels.manifest_label_digest`, the one place the label digest is defined
  (toolchain name bound, version ignored). `runner.src_sha256` changed, so the
  `ct-compare` manifest is regenerated separately.
