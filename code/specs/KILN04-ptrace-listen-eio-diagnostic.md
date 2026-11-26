# KILN04 — Linux ptrace LISTEN EIO diagnostic

## Status

Proposed, diagnostic-only work on `codex/kiln-ptrace-diagnostic-20261126`.
This branch must not be merged. Its workflow is manually dispatched only to
this exact branch and is not a product or required-CI change.

## Purpose

An observed Ubuntu 24.04 Kiln run failed inside `strace` with
`ptrace(PTRACE_LISTEN, pid, sig:0): Input/output error` during
`TestSandboxStraceDyingThreads`; the test command returned a real failure.
Other repetitions in that same invocation passed. Existing evidence does not
prove the cause. This diagnostic preserves that command's result while
capturing the inner tracer's `ptrace`, `wait4`, and `waitid` calls, so a future
controlled run can show the syscall chronology around the failure.

## Isolation and execution

- Add a boolean `workflow_dispatch` input that defaults to false and a string
  `expected_sha` input that defaults to empty. The diagnostic invocation
  supplies the reviewed full commit SHA in `expected_sha`.
- The diagnostic job is eligible only when the boolean is true and the ref is
  exactly `refs/heads/codex/kiln-ptrace-diagnostic-20261126`. The script
  rejects an empty, malformed, or mismatched expected SHA before the test.
- On the dedicated branch, all ordinary jobs are skipped; on other refs, a
  true diagnostic boolean also skips the ordinary detector and aggregate
  gate. Neither case may accidentally start the broad CI workflow.
- For that invocation, skip the ordinary detector and aggregate CI gate, whose
  dependencies would otherwise cause unrelated workflow jobs to run. All
  other jobs remain dependent on the detector and therefore skip.
- Run one Ubuntu 24.04 job with `contents: read`, no secrets, a bounded timeout,
  and the existing Go version. Record the actual kernel and strace versions.
- Run the unchanged `TestSandboxStraceDyingThreads` five times with the same
  sandbox-required and trace-differential settings as the existing Kiln CI.
  Preserve the Go test exit status; a failure remains a failure.
- Run the test from `code/programs/go/kiln`, its Go module directory.

## Capture method

The test resolves `strace` with `exec.LookPath`, then executes that path with a
child environment whose `PATH` is `/usr/bin:/bin`. A test-local shim placed
ahead of the Go test's PATH is therefore resolved by `LookPath`; it invokes an
outer system `strace` on the absolute inner system strace path. The outer
tracer filters only `ptrace`, `wait4`, and `waitid`, and does not use `-f`.
Therefore it traces the inner strace process, not the inner tracer's target
process. A shared FIFO collector remains open across the repeated invocations
and preserves their syscall output in one chronological stream. The shim
changes instrumentation and may perturb the race; a non-reproduction is not
evidence that the original failure was fixed or impossible.

The collector starts and confirms that its bounded output file is open before
the Go test starts. Its reader remains alive across writer EOFs between
invocations and drains each available FIFO read without waiting for a full
buffer; the parent closes its keeper descriptor only after `go test` returns,
then joins the collector before hashing or uploading evidence. The
collector has no separate timeout that could close the reader during a test:
the existing Go test deadline and diagnostic job timeout bound its lifetime.
The per-attempt evidence directory is created exclusively so readiness cannot
be mistaken for a stale output from an earlier local invocation.
The keeper descriptor is closed for the Go command and both capture helpers so
neither they nor their descendants inherit it. Upload only regular evidence
files, never the FIFO itself.

Upload the bounded test log and outer syscall trace even on failure. Do not
change the Kiln parser, accepted refusal shapes, timeout, test count, or
production behavior as part of this diagnostic.

The test log capture stores at most 8 MiB, drains any remaining output without
blocking the test, and records whether output was truncated. A separate
collector drains the outer tracer's FIFO and stores at most 32 MiB of syscall
trace, while continuing to drain after the storage cap. The cap applies only
to the outer trace artifact; no file-size or process-resource limit is applied
to inner strace or the fixture process tree. Missing or empty trace output, a
reached capture limit, or a capture error makes an otherwise successful test
command an overall diagnostic failure. A successful command also requires
exactly five top-level RUN markers for `TestSandboxStraceDyingThreads`, no skip
marker for that test, exactly five outer-tracer invocation markers, and at
least one captured `ptrace` call in the outer trace. A nonempty file containing
only setup/exit text is not sufficient.
Metadata always records the actual `go test` exit status separately from the
diagnostic status.

## Acceptance

Review the workflow conditions to establish that diagnostic dispatch runs only
the single diagnostic job, and that ordinary dispatch, push, pull-request,
and scheduled behavior remains unchanged when the diagnostic boolean is false.
Check the shell script syntax and compile
the Kiln test package for Linux locally when the host is not Linux. A Windows
compile is not evidence of executing the diagnostic; a hosted run, if later
authorized, must report its actual result and instrumentation limits.
