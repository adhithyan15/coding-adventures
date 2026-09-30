### Added — the XAML host reports its environment (UI48 ENV4, §7.7)

- `MosaicRuntimeHost.ReportEnvironment(component, environment, requiredProps)`
  dispatches `environmentChanged` with the whole environment; nothing is sent
  without a runtime or when it equals the last report the runtime took, and a
  refusal (answered as a status line, never thrown) is not remembered.
- `EnvironmentReport(width, height, dark)` and `InitialEnvironment()` give the
  six UI48 §4 values from effective pixels (600/1024 size classes); the
  initial values join the start context.
- Every dispatch keeps the props showing when its update carries
  `props: null` at the revision already shown (`KeepShowingProps`), and a
  report that did not move the revision re-applies nothing to the component.
  One that did is applied strictly with the shell's required props.
- The XAML conformance harness checks the values, thresholds, the kept props,
  refusals, and (through the state file) that only a changed report is sent.

