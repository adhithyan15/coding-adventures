### Haskell ITF stack Windows build fronts

- Replace the unconditional Windows skips in the Haskell `barcode-layout-1d`
  and `itf` packages with fail-closed Cabal discovery and execution of their
  existing test-enabled package graphs.
- Record the separate metadata-only follow-up for truthful empty ITF
  capability manifests without changing encoder behavior or public APIs.
