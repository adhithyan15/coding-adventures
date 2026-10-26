---
category: Haskell
---

# Haskell package bounds must admit the containers version bundled with the active GHC

The first `starlark-compiler` cabal file constrained `containers` to `<0.8`,
copied from an older toolchain assumption. GHC 9.14.1 ships `containers-0.8`,
and the installed `binary` and `text` packages require that exact instance, so
the solver could not even reach the intended missing-source red test. Use the
installed compiler's package set as evidence before choosing upper bounds; for
this ordinary `Data.Map` API, `containers >=0.6 && <0.9` admits both current CI
and the active compiler without an unbounded dependency.
