---
category: Cross-platform & Windows BUILD_windows
---

# path.relative across Windows drives returns an absolute path, not a '..' path

The TypeScript warrant-corpus runner refused a corpus outside the repository
with `relative(root, corpus)` followed by `path.startsWith("..")`. On Windows CI
the repository was on `D:` and the test temp dir on `C:`. There is no way to
climb from one drive to another, so `path.relative` returns the absolute target
(`C:\tmp\x`), which does not start with `..`: the corpus "escaped" the root and
`prepare` resolved instead of rejecting. Linux never shows this, because every
path shares one root.

The fix is a shared `relativeInside(root, child, pathModule)` that also rejects
an absolute result, and tests `..` or `..<sep>` rather than a bare `..` prefix
(a directory named `..foo` is inside the root). Taking the path module as a
parameter lets `path.win32` with `D:\repo` and `C:\tmp\x` reproduce the bug on
Linux.

Next time: never test containment with `relative(...)` plus a `..` prefix alone.
Reject absolute results too, and unit-test with `path.win32` so the Windows
branch is exercised on every host. Python's `Path.relative_to` raises
`ValueError` instead; catch it and turn it into a refusal. Also remember that
editing runner `src/` changes `runner.src_sha256`, so regenerate the dependent
lane manifests.
