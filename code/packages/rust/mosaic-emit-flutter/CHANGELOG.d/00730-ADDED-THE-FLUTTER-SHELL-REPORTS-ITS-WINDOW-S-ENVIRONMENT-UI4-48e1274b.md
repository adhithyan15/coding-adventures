### Added — the Flutter shell reports its window's environment (UI48 ENV4, §7.8)

The native-complete `main.dart` passes `builder: _observeEnvironment` to its
`MaterialApp`. The builder reads `MediaQuery.sizeOf`, `platformBrightnessOf`
and `disableAnimationsOf`, reduces them through
`MosaicHost.environmentReport`, and queues one post-frame callback (one at a
time) that hands the latest report to `MosaicHost.reportEnvironment` -- never
during build, and only once the runtime is up and showing. An answer with
props is shown like an event's; a failure is only logged with `debugPrint` and
never reaches the startup-failure screen. The emit-only placeholder host
gains the two members so the strict shell type-checks before the builder
installs the standard binding. Sample-props shells are unchanged.

