# Changelog

## Unreleased

- Scope the `BUILD` file's coverage run to `--lib`. `tests/command_provider.rs`
  is a `harness = false` test that re-executes its own binary as the approval
  helper; tarpaulin instruments via ptrace `int3` patching, which the self-spawned
  child cannot survive (`SIGILL raised in <pid>`, reported as "Failed to run
  tests"). `cargo test` still runs both targets, so nothing is left unexercised —
  only the coverage measurement is narrowed, and only because that target is
  unmeasurable by construction.
- Add a shell-free external command adapter for Tier 3 hardware-key approval.
- Bind one strict hardware-key decision to one fresh helper process and the
  complete bounded exact-resource prompt delivered to that process.
- Clear the inherited environment and accept hardware-key assurance only from
  the explicitly configured operator-reviewed helper.
- Preserve fail-closed Tier 3 timeout, denial, malformed output, launch, I/O,
  and process-control behavior.
