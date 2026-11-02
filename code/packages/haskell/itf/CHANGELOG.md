# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.
- Add shared fixture conformance, bounded scalar preflight, and stable error IDs.

## 0.1.0

- Validate non-empty, even-length ASCII digit payloads.
- Encode standard Interleaved 2 of 5 digit pairs and expose their patterns.
- Expand start, data, and stop symbols through the shared 1D barcode layer.
- Emit backend-neutral paint scenes with explicit symbol and pair metadata.
