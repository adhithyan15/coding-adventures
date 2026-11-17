---
category: Cross-platform & Windows BUILD_windows
---

# On Windows, canonicalize() returns verbatim paths: check .. textually before resolving and build adversarial test paths as text

**What went wrong.** The path-containment checks added for `vos-link` and
`vos-manifest` (PR #19529) passed on macOS and Linux and failed on the Windows
CI job, twice, after a reviewer had judged the first case safe.

**Why.** On Windows `std::fs::canonicalize` returns a *verbatim* path
(`\\?\C:\...`). Two behaviours follow, and neither shows up on Unix:

1. `std` does not parse `..` inside a verbatim path as a `ParentDir`
   component. It is an ordinary name, so a check that walks `Path::components()`
   looking for `ParentDir` never sees it.
2. `PathBuf::join` onto a verbatim base silently resolves `..` while joining.
   A test that builds an adversarial path as `root.join("a/../b")` from a
   canonical root therefore tests nothing on Windows: by the time the code under
   test sees the path, the `..` is already gone.

**Fix and rule.** Check for `..` **textually** (on the string, for both
separators) before any resolution, and build adversarial test paths **as text**
(format a string, then wrap it), never by joining onto a canonical base. A path
check that looks correct from reading the code and passes on Unix is not
evidence for Windows; the Windows CI job is the only check, so write the test
so that it can fail there.
