# Changelog

## Unreleased

- Add a shell-free external command adapter for Tier 3 hardware-key approval.
- Bind one strict hardware-key decision to one fresh helper process and the
  complete bounded exact-resource prompt delivered to that process.
- Clear the inherited environment and accept hardware-key assurance only from
  the explicitly configured operator-reviewed helper.
- Preserve fail-closed Tier 3 timeout, denial, malformed output, launch, I/O,
  and process-control behavior.
- Measure coverage with tarpaulin's LLVM engine instead of its default ptrace
  engine. The integration test re-executes its own binary as the approval
  helper, and ptrace-based instrumentation aborted the run with `SIGILL` when a
  breakpointed thread forked to spawn that child. No test or assertion changed;
  only the coverage collection method did.
