# haskell FFI — GHC 9.4.8 int-conversion + runtime dylib path (WEB18)

**Date:** 2026-06-14

The Haskell Conduit port (WEB18, the first Haskell package using GHC C FFI) was
green locally on GHC 9.14.1 but red on CI, which pins GHC 9.4.8. Two distinct,
environment-specific failures that a newer local GHC masked:

1. **macOS compile error — `-Wint-conversion`.** GHC < 9.6 generates libffi
   adjustor C stubs for every `foreign import ccall "wrapper"` that assign a
   `void*` (`HsPtr`) to an `ffi_arg` (`unsigned long`): `*(ffi_arg*)resp = cret;`.
   Clang 15+ (macOS CI) promotes `-Wint-conversion` from a warning to a hard
   error, so `gcc' failed in phase C Compiler`. Fix: add
   `-optc-Wno-error=int-conversion` to the **library** `ghc-options` (the stub is
   emitted when compiling the module that declares the `"wrapper"` imports). GHC
   9.6+ fixed the codegen, which is why a modern local GHC never sees it.

2. **Linux runtime error — `libconduit_capi.so: cannot open shared object file`.**
   `cabal test --extra-lib-dirs=DIR` only affects **link** time. At **run** time
   the test binary loads the cdylib through the OS dynamic loader, which does not
   consult `--extra-lib-dirs`. Fix: `export LD_LIBRARY_PATH` (Linux) and
   `DYLD_LIBRARY_PATH` (macOS) to the release dir (and `…/deps`) in run-tests.sh
   before invoking `cabal test`. Locally this was masked because I had been
   exporting `DYLD_LIBRARY_PATH` by hand in every manual run.

Durable lessons:
- **Match the CI toolchain version locally before declaring green.** CI's
  `haskell-actions/setup` pins `ghc-version: 9.4`; a newer local GHC hid a
  codegen bug. For any FFI-heavy package, test on the CI-pinned compiler.
- **Link path ≠ run path for cdylibs.** Any port that dynamically links
  conduit-capi (vs. static) must set the loader path at run time, not just the
  linker path at build time. Sibling dynamic-load ports (C#) sidestep this with a
  custom `CONDUIT_CAPI_PATH` resolver; GHC FFI has no such hook, so the OS loader
  env vars are mandatory.
