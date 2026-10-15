## Unreleased — verify the scoped macOS Haskell toolchain before package builds

Add CI bootstrap integration for a private Cabal compiler/linker/SDK overlay,
with actual phase-refusal and C/GHC/hsc2hs/Cabal compile/link/run gates before
publication. Preserve original config, package tests and other tool environments.
Portable and local Windows receipts are scoped evidence; genuine macOS hosted
acceptance and full required CI remain pending. Miri DEFERRED/PENDING.
