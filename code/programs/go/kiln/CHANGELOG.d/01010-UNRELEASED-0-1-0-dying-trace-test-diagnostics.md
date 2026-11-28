## [0.1.0] - UNRELEASED

### Tests

- Dying-thread strace failures now report a bounded trace digest and whitelisted
  same-PID event summaries, including unfinished/resume and terminal linkage.
  Arguments and trace payloads are not emitted; numeric PID linkage remains
  explicitly ambiguous. The refusal oracle and accepted trace shapes are
  unchanged.
- Focused Go diagnostic tests pass. The real sandbox test could not run in this
  local environment because sandbox setup cannot bind the read-only `/etc`
  root; CI remains responsible for the real-strace execution.
