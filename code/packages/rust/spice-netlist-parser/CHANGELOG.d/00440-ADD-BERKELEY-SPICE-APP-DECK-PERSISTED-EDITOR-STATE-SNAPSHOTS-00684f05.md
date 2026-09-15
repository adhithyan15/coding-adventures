- Add Berkeley SPICE app-deck persisted editor-state snapshots for Mosaic host
  restoration. `BerkeleyAppDeck::editor_state_snapshot()` and
  `run_editor_state_snapshot()` now resolve saved selected-card and
  active-command IDs against the current deck, including stale-state repair
  flags.
