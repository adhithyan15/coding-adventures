# Changelog

## Unreleased

- Assert that the spawned helper observes only the protocol variable, which this
  crate's suite never checked. `src/lib.rs` has always called `env_clear()` so a
  Tier 1 helper cannot read the daemon's secrets, tokens, or paths, but nothing
  tested it -- the sibling biometric and hardware-key adapters both assert it.
  Verified the check can fail: with `env_clear()` removed the helper panics and
  the provider surfaces `InvalidResponse`.
- Measure coverage with tarpaulin's LLVM engine (`--engine llvm`) instead of its
  default ptrace engine. `tests/command_provider.rs` re-executes its own binary
  as the approval helper, and ptrace-based instrumentation patches `int3`
  breakpoints the self-spawned child cannot survive (`SIGILL raised in <pid>`,
  reported as "Failed to run tests"). `-C instrument-coverage` carries no such
  per-process bookkeeping, so the spawn is a non-event and the integration
  target is measured rather than skipped.
- Tolerate `__LLVM_PROFILE_RT_INIT_ONCE` in the helper's environment assertion,
  matched on name AND value. compiler-rt's profile runtime sets it on the child
  itself after `exec`, so `env_clear()` on the parent cannot suppress it and its
  presence is not evidence of a leak. The value is pinned to its fixed sentinel
  because compiler-rt uses `setenv(..., overwrite=0)`: a variable of that name
  that was genuinely inherited would reach the child with the parent's value
  intact, so a name-only allowlist would be a one-variable smuggling channel.
  Every genuinely inherited variable still fails the check, confirmed by
  re-running the suite with `env_clear()` removed.
- Add a shell-free external command adapter for Tier 1 notification approval.
- Send bounded exact-resource prompts over a versioned, environment-cleared
  standard-input protocol and accept only canonical approval or denial lines.
- Distinguish a live canonical timeout from early exit, malformed output, and
  process or pipe failures so timeout remains the sole auto-approval path.
- Require an explicit post-presentation `ready` acknowledgement before a live
  decision-window timeout can be treated as Tier 1 auto-approval.
