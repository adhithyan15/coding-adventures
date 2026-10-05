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

`targets.json` records all twelve current implementation lanes and their
adapter/test hooks. All twelve established implementations are now conformant
with the portable v1 contract. A pending target may name only its planned
conformance test and known divergences; promotion fields are rejected. A conformant target must
name an executed package test, carry the exact raw `cases.json` SHA-256, bind a
tested pre-publication revision, match the checked-out canonical package tree,
and name the durable adoption PR. Revision and PR evidence are checked against
the durable backlog owner. A conformant target must use the canonical
language/package root and have no remaining divergence. Package evidence paths
must remain inside that root and be tracked in the adoption commit.

The repository gate also loads each promoted target's capability manifest,
validates it against the shared schema, and requires an empty capability list.
Structured zero-authority evidence must point at the executed conformance test
and name both text-request precedence assertions. Free-form claims and empty
manifests alone are not promotion evidence.

The generator is the independent reference oracle for corpus construction; it
is not production code and grants no runtime authority to package adapters.
