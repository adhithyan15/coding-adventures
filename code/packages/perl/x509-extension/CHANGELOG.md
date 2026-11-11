# Changelog

## [Unreleased]

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0

- Add bounded generic X.509 Extension decoding.
- Consume the 48-case language-neutral portable conformance fixture.
- Preserve shared DER budgets, opaque values, private construction, local
  offsets, and payload-redacted diagnostics.
