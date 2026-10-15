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

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
