- Add Berkeley SPICE app-deck shell dashboard dispatch queue digests for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_digest()`,
  `run_app_shell_dashboard_dispatch_queue_digest()`, and their JSON helpers
  derive a compact headline queue item with queue state, message, target,
  dispatch/action joins, first queue item routing, counts, and digest
  capability metadata from dashboard dispatch queues.
