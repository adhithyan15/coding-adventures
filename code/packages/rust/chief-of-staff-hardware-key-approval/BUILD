# `cargo test` runs BOTH targets — the lib unit tests and the `command_provider`
# integration test — so nothing below reduces what is actually exercised.
cargo test -p chief-of-staff-hardware-key-approval -- --nocapture

# Coverage is measured over the lib only (`--lib`), which is a hard constraint,
# not a preference. `tests/command_provider.rs` is a `harness = false` test that
# drives the provider by re-executing its OWN binary (`env::current_exe()`) as
# the approval helper, switched by `CHIEF_APPROVAL_PROTOCOL=3`. Tarpaulin
# instruments by patching `int3` breakpoints into the executable and tracking
# them per-process under ptrace; the self-spawned child runs that same patched
# image with no bookkeeping of its own, hits a patched instruction it cannot
# interpret, and dies — `SIGILL raised in <pid>`, which tarpaulin reports as
# "Failed to run tests" and fails the build. That is a property of re-exec plus
# ptrace instrumentation, so no amount of test-side fixing makes this target
# measurable; only the lib can be.
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p chief-of-staff-hardware-key-approval --lib --out Stdout --skip-clean; fi
