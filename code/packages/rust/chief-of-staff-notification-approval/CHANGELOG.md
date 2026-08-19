# Changelog

## Unreleased

- Add a shell-free external command adapter for Tier 1 notification approval.
- Send bounded exact-resource prompts over a versioned, environment-cleared
  standard-input protocol and accept only canonical approval or denial lines.
- Distinguish a live canonical timeout from early exit, malformed output, and
  process or pipe failures so timeout remains the sole auto-approval path.
- Require an explicit post-presentation `ready` acknowledgement before a live
  decision-window timeout can be treated as Tier 1 auto-approval.
- Measure coverage with tarpaulin's LLVM engine instead of its default ptrace
  engine. The integration test re-executes its own binary as the approval
  helper, and ptrace-based instrumentation aborted the run with `SIGILL` when a
  breakpointed thread forked to spawn that child. No test or assertion changed;
  only the coverage collection method did.
