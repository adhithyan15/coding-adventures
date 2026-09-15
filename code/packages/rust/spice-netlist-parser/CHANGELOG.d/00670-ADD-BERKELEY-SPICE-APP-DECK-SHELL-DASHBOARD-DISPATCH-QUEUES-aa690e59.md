- Add Berkeley SPICE app-deck shell dashboard dispatch queues for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue()`,
  `run_app_shell_dashboard_dispatch_queue()`, and their JSON helpers derive
  stable queue item IDs, selected/default queue routing, queued/blocked state,
  dispatch queue messages, event/action joins, and queue capability metadata
  from dashboard dispatch events.
