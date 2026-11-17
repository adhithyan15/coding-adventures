---
category: Cross-platform & Windows BUILD_windows
---

# A committed evidence manifest must not record exact heap peaks on a panicking path; panic-payload size differs by platform

**What went wrong.** The hash-functions Warrant evidence suite declared `heap <= 4096` on `polynomial_rolling_with_params`, and the generated harness recorded the observed peak (`max_observed_heap`) in `evidence/test-manifest.json`, which BUILD diffs with `git diff --exit-code`. The zero-modulus path panics on `assert!(modulus > 0)` and the harness catches it. The only allocation there is std's panic payload, whose size depends on platform and toolchain: 80 bytes on Linux, 16 on Windows. The committed 80 passed Linux and macOS and failed `build (windows-2025)` on the manifest diff, although every test passed.

**The fix.** Remove the `heap` row from that one operation (keep steps and depth), and record why in the warrant comment, DT17a section 7 and the CHANGELOG. The remaining heap rows are on operations that never panic, so their peak is exactly 0 on every platform.

**Do differently.** A manifest that is committed and diffed on several platforms may only hold platform-independent values. Before adding a `heap` (or any observed-metric) row, check that the operation's instances cannot reach a panic, unwinding or error-formatting path; if one can, leave the row off and say why. Passing tests on one platform do not show that a recorded measurement is portable.
