# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.
- Add strict v1 binary, width, layout, and scene adapters while preserving the
  established public API.
- Execute all 56 language-neutral conformance cases, including stable errors,
  checked limits, Unicode scalar handling, metadata ownership, and text-first
  zero-authority rejection.
- Declare the production package's empty capability profile.
- Return a closed, payload-blind `Barcode1DV1Error` from every portable-v1
  entry point while preserving the legacy diagnostic error API.

## 0.1.0

- Add typed bar and space runs with semantic source roles.
- Expand binary and configurable narrow/wide patterns.
- Compute inferred or explicit symbol spans with validated quiet zones.
- Emit metadata-rich bar rectangles through the shared paint IR.
- Reject invalid geometry, non-alternating runs, and unsupported text output.
