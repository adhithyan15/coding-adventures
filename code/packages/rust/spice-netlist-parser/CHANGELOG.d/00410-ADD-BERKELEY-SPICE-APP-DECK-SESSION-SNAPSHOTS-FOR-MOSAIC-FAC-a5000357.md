- Add Berkeley SPICE app-deck session snapshots for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::session_state()` and `run_session_state()` now
  expose deterministic source fingerprints, selected-analysis state,
  run/blocked status, diagnostics, table columns, output probes, and selected
  waveform availability without requiring UI hosts to own simulator internals.
