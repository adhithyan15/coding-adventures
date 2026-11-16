---
category: Cross-platform & Windows BUILD_windows
---

# filepath.Rel across Windows drives returns an error, so a containment check that ignores the error accepts every path on another drive

The TypeScript lane's containment bug was that `path.relative` returns an *absolute* path across Windows
drives, which a `..` prefix test waves through. Go differs in the other direction: `filepath.Rel("D:\\repo",
"C:\\tmp\\x")` returns an **error** and an empty string. A check written as `rel, _ := filepath.Rel(...)`
followed by "is it `..`-prefixed?" sees `""`, which is neither absolute nor a climb, and accepts the path.

The Go lane's `relativeInside` treats the error as "outside", also refuses an absolute result (a UNC share, or
any path module that behaves like Node's), and tests `..` or `..` plus a separator, because a directory named
`..foo` is inside the root. It takes the Rel, IsAbs and separator as parameters, so the tests drive a
Windows-shaped implementation (volumes, case-insensitive names, an error across drives) from a Linux host.

Do differently: never use `strings.HasPrefix(rel, "..")` alone, never discard `filepath.Rel`'s error, and
unit-test containment with injected Windows-shaped path functions rather than waiting for the Windows job.
