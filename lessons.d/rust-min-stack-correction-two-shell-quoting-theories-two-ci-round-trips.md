# `RUST_MIN_STACK` correction: two shell-quoting theories, two CI round trips, both wrong — the fix that actually worked doesn't depend on the shell at all

Continuing the `chief-of-staff-smart-home-tools` saga (two lessons up): the
"drop the unnecessary quotes" fix (`set "RUST_MIN_STACK=..." &&` → `set
RUST_MIN_STACK=... &&`) was itself wrong. The NEXT CI round crashed at the
exact same test with the exact same `0xC00000FD` — proving the quoting
theory, however plausible it sounded, was not the actual mechanism (or was
only part of a bigger propagation problem never fully diagnosed).

**What actually fixed it:** stop trying to get an environment variable
through the Go executor → `cmd.exe` `/C` → `cargo test` → spawned
test-binary chain at all. Wrap the test body in
`std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(...).join()`
instead — the stack size is a Rust-level property of the thread `cargo
test`'s harness spawns to run the test, not something inherited from a
shell environment several processes up the chain. Verified locally that this
is the actual fix (not just "different enough to maybe work"): running the
compiled test binary directly with `RUST_MIN_STACK=1048576` (the exact value
that reproduced the original overflow) now passes, because the explicit
`stack_size()` call overrides whatever `RUST_MIN_STACK` would have set —
proof the fix no longer depends on that env var reaching the process at all.

**Lesson, sharper than the one two entries up:** when an env-var-based
workaround for a platform bug fails to change the failure on the very next
CI round, don't spend a SECOND blind ~80-minute round trip on a second theory
about why the env var isn't propagating (quoting, escaping, `cmd.exe`
quirks, Go's argument handling — there are a lot of plausible-sounding
culprits in that chain, and this got two wrong in a row). If the *tool*
whose behavior you're trying to influence exposes a way to set the same
property directly in code — here, `Thread::Builder::stack_size()` for
exactly what `RUST_MIN_STACK` configures — prefer that path outright rather
than debugging an indirect shell/env mechanism you can't test end-to-end
locally (only the CI round trip proves an env var actually reached the
target process; a direct API call you can verify with a single local run).
