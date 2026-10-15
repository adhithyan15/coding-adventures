## Unreleased — verify the scoped macOS Haskell toolchain before package builds

Add CI bootstrap integration for a private Cabal compiler/linker/SDK overlay,
with actual phase-refusal and C/GHC/hsc2hs/Cabal compile/link/run gates before
publication. Preserve original config, package tests and other tool environments.
Pin the version-checked GHC through the raw top-level compiler selector; refuse
competing compiler paths, flavors and package tools. Retain actual failed
ghc-location-only/quoted-path probes and the successful raw-selector local control.
Portable and local Windows receipts are scoped evidence; genuine macOS hosted
acceptance and full required CI remain pending. Miri DEFERRED/PENDING.
