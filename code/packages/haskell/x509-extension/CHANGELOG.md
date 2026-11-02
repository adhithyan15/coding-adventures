# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Add bounded generic X.509 Extension decoding.
- Consume the 48-case language-neutral portable conformance fixture.
- Preserve shared DER budgets, extension-local offsets, opaque values, and
  payload-redacted diagnostics.
