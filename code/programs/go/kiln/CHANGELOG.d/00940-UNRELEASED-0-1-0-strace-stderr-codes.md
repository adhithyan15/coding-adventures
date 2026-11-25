## [0.1.0] - UNRELEASED

### Added (the census says WHY strace wrote to stderr, without the text)

- `internal/record/strace_stderr.go`: `ClassifyStraceStderr` reduces strace's own stderr (first 4 KiB,
  on the runner) to a closed code: `ptrace-listen-eio`, `attach-failed`, `oom`, `ptrace-other`,
  `wait-failed`, `exit-killed`, `unknown`. The text is never copied anywhere that leaves the runner.
- Step sidecar: additive omitempty `strace_stderr_code` (schema stays `kiln-record-step/3`);
  `Validate` refuses a code outside the list or beside no stderr. strace's exit code and signal were
  already in the sidecar (`strace_exit`, `strace_signal`).
- Census: `explain.sidecar.strace_stderr_codes` and `strace_signals` (by name, e.g. `SIGKILL(9)`), and
  two lines in the Markdown. Informational: nothing fails on them, `census_not_clean.py` unchanged.
- `kiln:recorder` stays 21: the reason is still `strace-stderr-not-empty` and nothing keyed changed.
- Motivation: census run 45466205573 had one `go/blake2b` step unclean for `strace-stderr-not-empty`
  with no way to tell what strace said.
