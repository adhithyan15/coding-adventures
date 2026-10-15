---
category: CI & GitHub Actions
---

# Checking PATH GHC does not establish Cabal compiler identity

The macOS bootstrap checked PATH GHC9.4.8, while the copied Cabal configuration
could select another compatible compiler through with-compiler or ghc-location.
Healthy phase refusals and runtime42 would not establish the claimed version.
Independent review caught this before publication. Commit exact selector refusals
and a pinned-path control before repair; preserve the actual seven-failure red.
Also account for compiler flavor and package-tool selectors. Mocked controls do
not establish hosted macOS acceptance; retain that as PENDING and Miri as
DEFERRED/PENDING until genuine final-source validation.

The companion-selector specification commit encountered a Git index lock while a
previous command was still completing. Its staged specification was committed
with the next test-only commit, still before behavior. Consume every pending Git
session before issuing further index mutations; do not assume a yielded commit
has finished and do not remove locks without checking the owning operation.
