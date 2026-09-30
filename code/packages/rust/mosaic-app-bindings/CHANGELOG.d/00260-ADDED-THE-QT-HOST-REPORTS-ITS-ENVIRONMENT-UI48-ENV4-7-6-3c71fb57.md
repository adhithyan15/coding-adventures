### Added — the Qt host reports its environment (UI48 ENV4, §7.6)

- `MosaicHost::reportEnvironment(environment)` dispatches `environmentChanged`
  with the whole environment; nothing is sent without a runtime or when it
  equals the last report the runtime took, and a refused report is not
  remembered.
- `MosaicHost::environmentReport(width, height, dark)` and
  `initialEnvironment()` give the six UI48 §4 values (600/1024 size classes);
  the initial values join the start context.
- `handleEvent` keeps the props showing when an update carries `props: null`
  at the revision already shown (`keepShowingProps`), as the Swift and Kotlin
  hosts do. `isSettling()` lets a shell hold a report until a settle is over.
- In a native-complete shell (`configureRequiredProps`) the answer is checked
  and mapped to QML names (`requireAndMapUpdate`), as `handleRequiredEvent`'s
  is. The identical refused report is not resent; a host deleted during the
  report's settle is not touched afterwards; kept props are the runtime's own,
  without a persistence warning that has since cleared (`showUpdate`).
- The Qt effect driver checks all of it against the conformance runtime.

