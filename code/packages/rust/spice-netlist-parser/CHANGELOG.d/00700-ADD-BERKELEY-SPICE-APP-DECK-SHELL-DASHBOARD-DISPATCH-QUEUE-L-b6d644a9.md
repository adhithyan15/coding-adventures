- Add Berkeley SPICE app-deck shell dashboard dispatch queue lanes for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lanes()`,
  `run_app_shell_dashboard_dispatch_queue_lanes()`, and their JSON helpers
  bucket dashboard dispatch queues into stable queued, blocked, and attention
  lanes with active-lane routing, lane item IDs, headline queue metadata, and
  lanes capability metadata for product-shell dispatch telemetry.
