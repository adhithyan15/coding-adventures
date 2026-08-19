# Changelog

## Unreleased

- Add a shell-free external command adapter for Tier 2 biometric approval.
- Bind one strict biometric decision to one fresh helper process and the complete
  bounded exact-resource prompt delivered to that process.
- Clear the inherited environment and accept biometric assurance only from the
  explicitly configured operator-reviewed helper.
- Preserve fail-closed Tier 2 timeout, denial, malformed output, launch, I/O, and
  process-control behavior.
- Measure coverage with tarpaulin's LLVM engine instead of its default ptrace
  engine. The integration test re-executes its own binary as the approval
  helper, and ptrace-based instrumentation aborted the run with `SIGILL` when a
  breakpointed thread forked to spawn that child. No test or assertion changed;
  only the coverage collection method did.
