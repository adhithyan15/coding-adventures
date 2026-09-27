### Changed — the packaged Qt shell observes each host it starts (UI48 ENV4, §7.6)

- `qt_main_with_startup_states` carries the emitter's
  `mosaicObserveEnvironment(view, mosaicHost)` into the retrying startup: each
  host that starts is observed, and a host a retry replaces takes its
  connections and timer with it.

