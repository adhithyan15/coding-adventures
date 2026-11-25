## [0.1.0] - UNRELEASED

### Added (the census says WHY strace wrote to stderr, without the text)

- `internal/record/strace_stderr.go`: `ClassifyStraceStderr` reduces strace's own stderr (first 4 KiB,
  on the runner) to a closed code: `sandbox-exec-notify`, `sandbox-helper` (the helper shares the stream; checked first),
  `ptrace-listen-eio`, `attach-failed` (needs the attach wording), `oom`, `tracee-gone`, `unknown-pid`,
  `umove-failed`, `ptrace-other`, `wait-failed`, `exit-killed`, `unknown`. strace wordings unverified against 6.19. The text is never copied anywhere that leaves the runner.
- Step sidecar: additive omitempty `strace_stderr_code` (schema stays `kiln-record-step/3`);
  `Validate` refuses a code outside the list or beside no stderr. strace's exit code and signal were
  already in the sidecar (`strace_exit`, `strace_signal`).
- Census: `explain.sidecar.strace_stderr_codes` and `strace_signals` and `strace_exits` (`exit-N` for steps with stderr text; `strace_signals` by name, e.g. `SIGKILL(9)`), and
  two lines in the Markdown. Informational: nothing fails on them, `census_not_clean.py` unchanged.
- `kiln:recorder` stays 21: the reason is still `strace-stderr-not-empty` and nothing keyed changed.
- Motivation: census run 45466205573 had one `go/blake2b` step unclean for `strace-stderr-not-empty`
  with no way to tell what strace said.
