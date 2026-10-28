### Haskell ITF stack Windows build fronts

- Replace the unconditional Windows skips in the Haskell `barcode-layout-1d`
  and `itf` packages with toolchain-optional Cabal routing that skips
  successfully when Cabal is unavailable and propagates `cabal test` failures
  when Cabal is present.
- Record the separate metadata-only follow-up for truthful empty ITF
  capability manifests without changing encoder behavior or public APIs.
- Re-verify the unchanged Haskell barcode-layout-1d lane and repoint its
  package-tree evidence after the Windows front changes outside lockfiles.
