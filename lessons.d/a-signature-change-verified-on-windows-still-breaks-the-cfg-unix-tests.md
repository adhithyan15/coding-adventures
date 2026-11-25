---
category: Cross-platform & Windows BUILD_windows
---

# A signature change verified on Windows still breaks the cfg(unix) tests that call it

`vos-capacity-probe` PR #19864 (VOS-028c3c-2a) keyed `runner::cpu_limit_s` on the
experiment instead of on the `wall_s` argument. Every local check on Windows was green
(tests, `clippy -Dwarnings`, fmt, `check`), and CI then failed on both Ubuntu and macOS.
The failing test was in `tests/hardening.rs`'s `#[cfg(unix)] mod unix_children`. It passed
a 600 s wall for an experiment named after a shell-injection string and still expected the
old CPU limit, 780. Keyed on the (unknown) experiment, the limit is now 300. On Windows
that module is neither compiled nor run, so nothing local could see it. Clippy was blind
to it too.

The fix was the test's expectation: 300, with a comment that N follows the experiment.

What to do differently: when you change the meaning of a function that Unix-only code
calls, grep for its callers inside `cfg(unix)` modules and update them before pushing:
`grep -rn "fn_name(" tests src`. That includes cases where the signature still compiles
but the semantics move. If an agent reports "all green on Windows" for such a crate, read
that as covering only the non-Unix half.
