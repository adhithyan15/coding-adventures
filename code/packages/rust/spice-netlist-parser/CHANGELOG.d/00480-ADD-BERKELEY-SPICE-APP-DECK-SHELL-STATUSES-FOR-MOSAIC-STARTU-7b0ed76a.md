- Add Berkeley SPICE app-deck shell statuses for Mosaic startup chrome and
  telemetry. `BerkeleyAppDeck::app_shell_status()`,
  `run_app_shell_status()`, and their JSON helpers derive a compact route,
  severity, message, entry action, and diagnostic counts from the shell handoff.
