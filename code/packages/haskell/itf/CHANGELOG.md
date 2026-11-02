# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.
- Add shared fixture conformance, bounded scalar preflight, and stable error IDs.

## 0.1.0

- Validate non-empty, even-length ASCII digit payloads.
- Encode standard Interleaved 2 of 5 digit pairs and expose their patterns.
- Expand start, data, and stop symbols through the shared 1D barcode layer.
- Emit backend-neutral paint scenes with explicit symbol and pair metadata.
