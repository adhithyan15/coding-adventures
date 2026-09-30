### Added — the XAML host reports its environment (UI48 ENV4, §7.7)

- `MosaicRuntimeHost.ReportEnvironment(component, environment, requiredProps)`
  dispatches `environmentChanged` with the whole environment. Nothing is sent
  without a runtime, inside a settle, or when the report equals the last one
  the runtime took or the last one it refused; a report taken is remembered
  at once. Failures come back as a status line, never thrown, as does a
  tripped settle guard.
- `EnvironmentReport(width, height, dark)` and `InitialEnvironment()` give the
  six UI48 §4 values from effective pixels (600/1024 size classes); the
  initial values join the start context.
- Every dispatch and effect completion keeps the props showing when its
  update carries `props: null` at the revision already shown
  (`KeepShowingProps`). A report re-applies props only when the revision
  showing is newer than the last one applied (`appliedRevision`), strictly
  with the shell's required props, so an ignored report rebuilds nothing and
  a failed apply is retried.
- The XAML conformance harness checks the values, thresholds, kept props,
  refusals, and (through the state file) which reports are sent.

