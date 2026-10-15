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
DEFERRED/PENDING until genuine final-source validation. An actual local Cabal3.16
build ignored an intentionally absent ghc-location and built with PATH GHC;
preserve that false semantic receipt. Pin top-level with-compiler, then prove
its discovery behavior with a real absent-compiler refusal. Exact overlay-path
presence alone is only a portable control, not execution evidence.
An initial quoted top-level path was treated literally by Cabal; preserve that
failed control too. The raw validated path subsequently proved actual missing
compiler refusal plus selected installed-compiler build0/run0 exact42 on Windows.
Those local3.16 receipts still do not establish hosted macOS3.10 acceptance.

The companion-selector specification commit encountered a Git index lock while a
previous command was still completing. Its staged specification was committed
with the next test-only commit, still before behavior. Consume every pending Git
session before issuing further index mutations; do not assume a yielded commit
has finished and do not remove locks without checking the owning operation.
