### Added — the Compose host reports its environment (UI48 ENV4, §7.4)

- `MosaicRuntimeHost.reportEnvironment(environment)` dispatches
  `environmentChanged` with the six UI48 values. A report equal to the last
  accepted one is dropped (null); a runtime refusal answers `{"error": …}`
  instead of throwing and is not remembered, so the next report is sent.
- `handleEvent` keeps the props it is showing when an update carries none at
  the revision already showing — the runtime's answer to an environment the
  app ignored (UI48 §7.1) — and leaves any other props-less update alone.
  Before, that answer would have handed the strict shell empty props, which
  its required-prop accessors refuse.
- `MosaicRuntimeHost.initialEnvironment()` (pointer, hover, reduced motion:
  `fine`/`hover` on desktop, `coarse`/`none` on Android) goes into the start
  context, as on SwiftUI.
- `conformance/compose` checks, against the real runtime, that an ignored
  report keeps the props and revision, an unchanged one is not resent, an
  invalid one is refused and leaves the props, and a changed one is sent.
