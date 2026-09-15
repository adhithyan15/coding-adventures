- Add Berkeley SPICE app-deck shell event summaries for Mosaic startup
  dashboards and gates. `BerkeleyAppDeck::app_shell_event_summary()`,
  `run_app_shell_event_summary()`, and their JSON helpers derive compact
  event-kind, severity, diagnostic, repaired-state, and capability counts from
  shell event logs.
