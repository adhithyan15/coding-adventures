- Add Berkeley SPICE app-deck shell dashboard action dispatch for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_action_dispatch()`,
  `run_app_shell_dashboard_action_dispatch()`, and their JSON helpers derive
  stable action dispatch IDs, selected/default dispatch routing, dispatchable
  state, disabled reasons, and action-dispatch capability metadata from
  dashboard panel-card actions.
