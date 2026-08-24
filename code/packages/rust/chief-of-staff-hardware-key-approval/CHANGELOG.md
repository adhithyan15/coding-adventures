# Changelog

## Unreleased

- Restore full-target coverage measurement, replacing the `--lib` scoping added
  when this SIGILL was first hit. That change concluded the integration target
  was "unmeasurable by construction"; that holds only for the ptrace engine, and
  switching engines measures it. Corrected here rather than left standing.
- Measure coverage with tarpaulin's LLVM engine (`--engine llvm`) instead of its
  default ptrace engine. `tests/command_provider.rs` re-executes its own binary
  as the approval helper, and ptrace-based instrumentation patches `int3`
  breakpoints the self-spawned child cannot survive (`SIGILL raised in <pid>`,
  reported as "Failed to run tests"). `-C instrument-coverage` carries no such
  per-process bookkeeping, so the spawn is a non-event and the integration
  target is measured rather than skipped.
- Tolerate `__LLVM_PROFILE_RT_INIT_ONCE` in the helper's environment assertion.
  compiler-rt's profile runtime sets it on the child itself after `exec`, so
  `env_clear()` on the parent cannot suppress it and its presence is not
  evidence of a leak. Every genuinely inherited variable still fails the check,
  confirmed by re-running the suite with `env_clear()` removed.
- Add a shell-free external command adapter for Tier 3 hardware-key approval.
- Bind one strict hardware-key decision to one fresh helper process and the
  complete bounded exact-resource prompt delivered to that process.
- Clear the inherited environment and accept hardware-key assurance only from
  the explicitly configured operator-reviewed helper.
- Preserve fail-closed Tier 3 timeout, denial, malformed output, launch, I/O,
  and process-control behavior.
