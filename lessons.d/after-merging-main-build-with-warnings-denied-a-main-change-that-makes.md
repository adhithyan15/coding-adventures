---
category: Rust
---

# After merging main, build with warnings denied: a main change that makes a function return Result turns a merged call site into a -D warnings compile error

**What went wrong.** PR7b (#18377) added `handle_f64_query` to
`vm-core/src/dispatch.rs`. It wrote its result with
`ctx.frames.last_mut().unwrap().assign(dest, result.clone());`, which was
correct when written, because `Frame::assign` returned `()`. Main's #18402 then
made `assign` return `Result` (the register-memory budget). Merging main into
the PR branch was textually clean, and `cargo test -p vm-core` passed locally,
because a plain `cargo test` only warns about `unused_must_use`. CI builds
with `-D warnings`, so `build (windows-2025)` failed on vm-core and on its five
dependents (cobol, flow-matic, nib, oct and twig compilers). The ignored `Err`
would also have dropped a register-budget error instead of reporting it.

**The fix.** Propagate the result with `?`, as every other handler does.

**What to do differently.** After merging main into a branch, build or clippy
the crates whose code changed with `RUSTFLAGS="-D warnings"` (or
`cargo clippy -- -D warnings`) before pushing. A clean textual merge says
nothing about signature changes on the other side.
