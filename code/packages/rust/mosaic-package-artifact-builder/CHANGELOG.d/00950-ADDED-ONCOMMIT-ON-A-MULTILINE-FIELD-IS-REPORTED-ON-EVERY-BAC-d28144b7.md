### Added — `onCommit` on a multiline field is reported on every backend (#15930)

- Enter inserts a newline in a multiline `HostInput` or `Input` on every
  backend, so an authored `onCommit` there has no trigger. The degradation
  report now says so as `interaction.multiline-commit-unbound` on all nine
  backends, instead of shipping a handler nothing can fire. UI25's promised
  Ctrl/Cmd+Enter commit was never implemented anywhere; commit a text area
  from a button. No package in the repo authors this combination today.
