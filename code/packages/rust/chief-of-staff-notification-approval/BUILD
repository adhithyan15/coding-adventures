# `cargo test` runs BOTH targets -- the lib unit tests and the `command_provider`
# integration test -- so nothing below reduces what is actually exercised.
cargo test -p chief-of-staff-notification-approval -- --nocapture

# Coverage runs on tarpaulin's LLVM engine, NOT its default ptrace engine.
#
# `tests/command_provider.rs` is a `harness = false` test that drives the
# provider by re-executing its OWN binary (`env::current_exe()`) as the approval
# helper, switched by `CHIEF_APPROVAL_PROTOCOL=1`. Tarpaulin's default ptrace
# engine instruments by patching `int3` breakpoints into the executable and
# tracking them per process; the self-spawned child runs that same patched image
# with no bookkeeping of its own, hits a patched instruction it cannot
# interpret, and dies -- `SIGILL raised in <pid>`, which tarpaulin reports as
# "Failed to run tests" and fails the build, even though every test passes under
# the plain `cargo test` above.
#
# `--engine llvm` instruments with `-C instrument-coverage` rather than ptrace,
# so spawning a process is a non-event and the integration target stays
# measurable. The LLVM engine's one documented fork caveat -- a forked child
# overwriting its parent's `.profraw` -- does not apply here: the provider calls
# `.env_clear()` before spawning, so the helper child never inherits
# `LLVM_PROFILE_FILE` and cannot clobber the parent's profile.
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p chief-of-staff-notification-approval --engine llvm --out Stdout --skip-clean; fi
