- Add Berkeley SPICE app-deck shell event logs for Mosaic product-shell startup
  streams. `BerkeleyAppDeck::app_shell_event_log()`,
  `run_app_shell_event_log()`, and their JSON helpers derive stable status,
  route, primary-action, diagnostic, repaired-state, and capability events from
  shell handoffs.
