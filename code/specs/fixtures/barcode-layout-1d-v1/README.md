# Barcode layout 1D v1 fixtures

This directory is the executable, language-neutral corpus for the portable
integer contract in [`../../barcode-layout-1d-v1.md`](../../barcode-layout-1d-v1.md).
It covers binary and narrow/wide expansion, module-space layout, symbol spans,
rectangle projection, bounded metadata, stable errors, and zero-authority text
rejection.

`cases.json` is generated. Edit `generate_cases.py`, then regenerate and run the
strict corpus test from the repository root:

```text
python code/specs/fixtures/barcode-layout-1d-v1/generate_cases.py
python -m unittest discover -s code/scripts/tests -p test_barcode_layout_1d_fixtures.py
```

For a byte-for-byte drift check without writing:

```text
python code/specs/fixtures/barcode-layout-1d-v1/generate_cases.py --check
```

The test loader rejects oversized or deeply nested documents, duplicate object
keys, non-finite JSON numbers, invalid Unicode scalars, and external schema
references before validation. `schema.json` closes the corpus shape.

Large expected run arrays use the digest encoding defined by the normative
spec. Compact `repeat`, `repeatRuns`, and `repeatSymbols` inputs are fixture
transport encodings; their limits are validated before expansion.

`targets.json` records the eight current implementation lanes and their future
adapter/test hooks. Every target intentionally remains `pending-adoption` in
this contract-only slice. A lane may become conformant only after its real
package tests consume this corpus and its required-capability declaration is
explicitly empty. The registry also requires an adoption revision, corpus
digest, PR number, and zero-authority call-graph evidence before a target may be
marked `conformant`.

The generator is the independent reference oracle for corpus construction; it
is not production code and grants no runtime authority to package adapters.
