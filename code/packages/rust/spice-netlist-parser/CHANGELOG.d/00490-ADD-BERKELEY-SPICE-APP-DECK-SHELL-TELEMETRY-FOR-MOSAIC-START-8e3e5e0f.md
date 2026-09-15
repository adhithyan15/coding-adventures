- Add Berkeley SPICE app-deck shell telemetry for Mosaic startup metrics.
  `BerkeleyAppDeck::app_shell_telemetry()`, `run_app_shell_telemetry()`, and
  their JSON helpers derive compact route, entry-action, availability,
  diagnostic, repaired-state, and capability counts from the shell handoff.
