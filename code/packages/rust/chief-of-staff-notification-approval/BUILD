cargo test -p chief-of-staff-notification-approval -- --nocapture
# Coverage runs on tarpaulin's LLVM engine, NOT its default ptrace engine.
#
# tests/command_provider.rs is a `harness = false` binary that re-executes
# ITSELF (env::current_exe) as the approval helper, and the provider spawns
# that child from worker threads. Tarpaulin's ptrace engine collects coverage
# by patching INT3 breakpoints into the running process and stepping each
# thread back over them; when a thread forks/clones to spawn the helper, the
# child resumes with its program counter parked mid-instruction and the run
# aborts with `SIGILL raised in <pid>` -- even though every test passes under
# plain `cargo test` on the line above.
#
# `--engine llvm` uses `-C instrument-coverage` instead of ptrace, so spawning
# a process is a non-event. The LLVM engine's one documented fork caveat -- a
# forked child overwriting its parent's .profraw -- does not apply here: the
# provider calls `.env_clear()` before spawning, so the helper child never
# inherits LLVM_PROFILE_FILE and cannot clobber the parent's profile. The
# coverage signal is unchanged: every line of src/ under measurement runs in
# the parent process, while the child only executes the test-side helper.
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p chief-of-staff-notification-approval --engine llvm --out Stdout --skip-clean; fi
