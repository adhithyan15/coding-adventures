# A non-recursive function can still overflow a 1 MiB Windows test-thread stack — debug builds don't reuse stack slots across `match` arms

`chief-of-staff-smart-home-tools` aborted on `windows-latest` with
`0xC00000FD`/`STATUS_STACK_OVERFLOW` in a two-call, non-recursive integration test.
The existing stack-overflow lessons (line ~1980, ~2557) are all about *recursive*
walkers whose frame gets multiplied by depth. This one had no recursion at all.

**Cause:** the tool dispatcher is one `match tool_id.as_str() { ... }` with 364 arms,
each declaring its own small `let query = ...;` local before calling a handler
function. In a debug build, rustc does not reuse stack slots across mutually
exclusive `match` arms (same mechanism as the recursive-eval lessons, just applied
to breadth instead of depth) — so the function's single stack frame sums roughly
364 arms' worth of locals. That is comfortably under macOS/Linux's ~8 MiB default
test-thread stack but over Windows' ~1 MiB floor. Reproduced locally without a
Windows box: build the test binary, then run it directly (not via `cargo test`,
which also applies `RUST_MIN_STACK` to `rustc` itself and breaks compilation) with
`RUST_MIN_STACK=1048576 ./target/debug/deps/<binary> tests::<name> --exact` —
overflowed at 1 MiB, passed cleanly at 2 MiB.

**Fix, and why raising the stack here is correct (not a band-aid):** the earlier
recursive-`eval` lesson fixed depth×frame-size by shrinking the frame, because a
depth cap that keeps growing would eventually re-overflow any fixed stack. Here
there is no depth to bound — the frame size is fixed regardless of input, so
widening the stack for this one test binary is a proportionate, permanent fix, not
a deferral. Set it in `BUILD_windows` (not committed to `Cargo.toml` or a global
CI env var, since only this package's frame is oversized):
`set "RUST_MIN_STACK=4194304" && cargo test -p <pkg> && cargo clippy -p <pkg> --all-targets -- -D warnings`.
Chose 4 MiB (2x the proven-sufficient 2 MiB) for margin. `set "VAR=value" &&
command` (not bare `VAR=value command`) is required — see the existing Windows
env-var lesson (line 35); each `BUILD_windows` line runs as its own `cmd /C`
process, so the `set` and the command it configures must be chained with `&&` on
one line.
