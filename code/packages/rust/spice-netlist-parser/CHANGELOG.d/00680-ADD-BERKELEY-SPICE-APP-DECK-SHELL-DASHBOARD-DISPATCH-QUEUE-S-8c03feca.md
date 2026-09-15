- Add Berkeley SPICE app-deck shell dashboard dispatch queue summaries for
  Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_summary()`,
  `run_app_shell_dashboard_dispatch_queue_summary()`, and their JSON helpers
  derive compact selected/default queue routing, first queued/blocked/attention
  queue item IDs, queue item ID lists, counts, and summary capability metadata
  from dashboard dispatch queues.
