# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added D18D handlers for the D23A smart-home integration catalog tools:
  `smart_home.list_integrations`, `smart_home.describe_integration`,
  `smart_home.list_primitives`, and `smart_home.describe_primitive`.
- Added D18D smart-home tool definitions and in-memory handlers over
  `SmartHomeRuntime`.
- Added an end-to-end Hue-style fixture test that lists devices, commands a
  light, reads optimistic state, and records a D18D execution journal entry.
- Added D18D handlers for `smart_home.subscribe`,
  `smart_home.pair_bridge`, `smart_home.describe_capabilities`, and
  `smart_home.get_health` so Chief of Staff jobs can reach the existing D23
  subscription, pairing, capability, and health runtime paths.
- Added the `smart_home.discover` D18D handler over
  `RuntimeDiscoverToolRequest`, including discovery filters, bridge-candidate
  output, and end-to-end journal coverage.
- Added `smart_home.list_discovery_workers` and
  `smart_home.get_discovery_summary` handlers over the D23 runtime read facade
  so Chief of Staff jobs can inspect scheduled discovery work and candidate
  freshness without owning discovery scheduler logic.
- Added scheduled discovery worker observability to
  `smart_home.observe_supervision`, including worker status, due time, last run
  counts, and failure pressure from the D23 runtime.
- Added D23 discovery worker retry policy fields to
  `smart_home.observe_supervision`, including configured retry delays,
  multiplier, and the current retry delay during failure pressure.
- Added `smart_home.poll_events` and `smart_home.unsubscribe` handlers so Chief
  of Staff jobs can drain, peek, summarize, and retire runtime event
  subscriptions without bypassing D23 authorization.
- Added `smart_home.list_scenes` and `smart_home.describe_scene` handlers over
  the D23 runtime scene read facade, including Hue-style fixture coverage.
- Added `smart_home.list_subscriptions` and `smart_home.inspect_event_log`
  handlers over the D23 runtime read facade so Chief of Staff jobs can inspect
  event-stream backlog pressure and checkpointed event history without draining
  subscriptions.
- Added `smart_home.list_authorization_decisions` and
  `smart_home.get_authorization_summary` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect allow/deny audit history without
  owning smart-home authorization logic.
- Added `smart_home.list_capability_grants` and
  `smart_home.get_capability_grant_summary` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect grant governance without owning
  smart-home authorization policy.
- Added `smart_home.list_rooms` and `smart_home.get_topology_summary` handlers
  over the D23 runtime read facade so Chief of Staff jobs can inspect room,
  state, and scene coverage without owning registry topology logic.
- Added `smart_home.get_runtime_snapshot`, `smart_home.list_desired_states`,
  and `smart_home.list_pairing_sessions` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect automation backlog, reconciliation
  targets, and pairing ceremonies without bypassing runtime ownership.
- Added `smart_home.get_supervision_plan` over the D23 runtime read facade so
  Chief of Staff jobs can preview due pairing, refresh, reconciliation,
  restart, and discovery work without ticking supervision.
- Added `smart_home.list_workers` and
  `smart_home.get_worker_heartbeat_schedule` over the D23 runtime read facade
  so Chief of Staff jobs can inspect supervised bridge workers and heartbeat
  deadlines without mutating supervisor state.
- Added `smart_home.reconcile_desired_states` and
  `smart_home.run_supervision_tick` handlers over the D23 runtime supervision
  facade so Chief of Staff jobs can run authorized reconciliation and
  supervisor ticks without owning D23 mutation logic.
