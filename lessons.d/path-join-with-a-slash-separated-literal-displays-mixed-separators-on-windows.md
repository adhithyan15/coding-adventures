---
category: Rust
---

# Path::join with a slash-separated literal displays mixed separators on Windows, so a test comparing it to a real path's display() fails only there

**What went wrong.** `mosaic-package-artifact-builder`'s test
`a_package_build_refuses_an_undeclared_package_reference` (added in #18426)
built the manifest path it expected the error to name with
`ws.path().join("mosaic-pkg-shell/mosaic-package.toml")`. It then asserted
the error message `contains` that path's `display()`.

On macOS and Linux both sides read `…/mosaic-pkg-shell/mosaic-package.toml`,
and the test passed locally and on Linux CI. On Windows, `join` keeps the
literal's `/`, so the expected text was
`C:\…\.tmpX\mosaic-pkg-shell/mosaic-package.toml`. The build had found the
manifest by walking directories, so the error named
`C:\…\.tmpX\mosaic-pkg-shell\mosaic-package.toml`. The strings never matched,
and `build (windows-2025)` failed on every later PR whose diff made the build
tool run this crate's tests (first seen on #18467).

**The fix.** Join one component at a time:
`ws.path().join("mosaic-pkg-shell").join("mosaic-package.toml")`. The code
under test was right; only the expectation was wrong.

**Do differently.** In a test that compares a path's text against a path the
code computed, never pass a multi-component literal with `/` to `join`.
Either join one component per call, or compare `Path`s rather than strings.
Before pushing such a test, grep the diff for `join("…/…")`. A test that
passes on macOS proves nothing about Windows path display.
