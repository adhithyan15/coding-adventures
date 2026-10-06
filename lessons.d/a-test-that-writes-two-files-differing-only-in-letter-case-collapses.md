---
category: Cross-platform & Windows BUILD_windows
---

# A test that writes two files differing only in letter case collapses them into one on macOS

PR #16246 added a builder test that wrote `Grid.touch.mll` and
`Grid.Touch.mll` to prove two variants folding to one generated view are
refused. It passed on Linux and failed on the macOS runner: APFS is
case-insensitive by default, so the second write replaced the first, only one
variant existed, and `discover_variants` rightly returned `Ok`.

Fix: every pair in the test differs by more than case (`touch-first` /
`TouchFirst` still exercises the case fold), and the test asserts all its
files exist side by side before checking the result, so a filesystem that
merges names fails loudly at the setup, not at the assertion.

Do differently: a test that relies on two file names coexisting must not let
them differ only in letter case (macOS and Windows both fold case), and should
assert the directory holds what it wrote.
