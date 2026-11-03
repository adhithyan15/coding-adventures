# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add bounded generic X.509 Extension decoding.
- Consume the 48-case language-neutral portable conformance fixture.
- Preserve shared DER budgets, extension-local offsets, opaque values, and
  payload-redacted diagnostics.
