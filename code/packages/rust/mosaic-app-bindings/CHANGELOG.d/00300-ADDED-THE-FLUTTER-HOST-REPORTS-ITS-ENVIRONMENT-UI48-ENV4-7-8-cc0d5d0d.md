### Added — the Flutter host reports its environment (UI48 ENV4, §7.8)

- `MosaicHost.reportEnvironment(environment)` dispatches `environmentChanged`
  with the whole environment. Nothing is sent without a runtime, inside a
  settle, or when the report equals the last one the runtime took or the last
  one it refused; a report taken is remembered at once, a refused one is
  remembered as refused. It returns null when there is nothing new to show
  (the answer did not move the revision), the answer when it did or carries a
  settle guard's `error`, and `{'error': 'Mosaic environment report failed:
  ...'}` for a refusal or a closed runtime -- it never throws.
- `MosaicHost.environmentReport(width, height, dark, {reduceMotion})` and
  `MosaicHost.initialEnvironment()` give the six UI48 §4 values from logical
  pixels (600/1024 size classes; coarse/none on Android and iOS); the initial
  values join the start context. The binding still imports no Flutter, so it
  keeps running under the plain Dart VM.
- Every dispatch and effect completion keeps the props showing when its update
  carries `props: null` at the revision already shown (`_keepShowingProps`),
  keeping the runtime's own props (`_runtimeUpdate`), not the
  persistence-decorated ones.
- The Flutter conformance harness checks the values, thresholds, kept props
  and revision, refusals, and (through the state file) which reports are sent.

