# chief-of-staff-smart-home-tools

`chief-of-staff-smart-home-tools` connects the D18D Chief of Staff tool runtime
to the D23 smart-home runtime.

The crate is intentionally a thin adapter:

- it publishes `smart_home.*` D18D tool definitions
- it registers in-process handlers on `InMemoryToolRuntime`
- handlers translate JSON arguments into `SmartHomeRuntime` read, discover,
  event-subscription, pair, command, event-ingest, and supervision execution
  requests, desired-state target mutations, plus read-only D23A catalog queries
- `SmartHomeRuntime` still owns smart-home authorization, command validation,
  event subscriptions, pairing sessions, optimistic state, discovery scheduler
  policy and observability, discovery pairing plans, supervision, and audit
  decisions
- `smart-home-integration-catalog` still owns D23A integration and primitive
  catalog semantics
- D18D still owns tool validation, policy decisions, event streams, terminal
  results, and execution journals

The first slice proves an end-to-end local path with an in-memory Hue-style
fixture:

```text
Chief of Staff job/session/agent
  -> D18D smart_home.discover / smart_home.command tool calls
  -> smart-home runtime authorization
  -> discovery records and unpaired bridge candidates
  -> discovery worker inventory and discovery freshness summaries
  -> discovery pairing-plan previews for required host actions
  -> human-approved pairing completion with VaultRef handles
  -> room topology and aggregate topology summary reads
  -> scene inventory and scene detail reads
  -> discovery worker health and retry state in smart_home.observe_supervision
  -> event-log and subscription-backlog reads
  -> typed command-result audit reads and summaries
  -> authorization decision audit reads and summaries
  -> capability grant ledger reads and summaries
  -> runtime snapshot, desired-state, and pairing-session inventory reads
  -> desired-state target set/clear through runtime authorization
  -> non-mutating supervision plan previews
  -> authorized desired-state reconciliation and supervision ticks
  -> supervised worker inventory and heartbeat schedule reads
  -> device command acceptance
  -> adapter-observed device and bridge-health event ingest
  -> confirmed state update
  -> D18D trace and audit record
```

## Included Tools

- `smart_home.list_integrations`
- `smart_home.describe_integration`
- `smart_home.list_primitives`
- `smart_home.describe_primitive`
- `smart_home.get_integration_catalog_summary`
- `smart_home.get_tool_catalog_summary`
- `smart_home.list_integration_readiness`
- `smart_home.get_integration_readiness_summary`
- `smart_home.list_integration_readiness_gaps`
- `smart_home.get_integration_readiness_gap_summary`
- `smart_home.discover`
- `smart_home.list_discovery_workers`
- `smart_home.get_discovery_summary`
- `smart_home.get_pairing_plan`
- `smart_home.list_bridges`
- `smart_home.list_devices`
- `smart_home.list_rooms`
- `smart_home.list_scenes`
- `smart_home.describe_scene`
- `smart_home.get_state`
- `smart_home.describe_capabilities`
- `smart_home.get_health`
- `smart_home.command`
- `smart_home.report_event`
- `smart_home.subscribe`
- `smart_home.poll_events`
- `smart_home.unsubscribe`
- `smart_home.list_subscriptions`
- `smart_home.inspect_event_log`
- `smart_home.list_command_results`
- `smart_home.get_command_result_summary`
- `smart_home.list_authorization_decisions`
- `smart_home.get_authorization_summary`
- `smart_home.list_capability_grants`
- `smart_home.get_capability_grant_summary`
- `smart_home.get_runtime_snapshot`
- `smart_home.get_topology_summary`
- `smart_home.list_desired_states`
- `smart_home.set_desired_state`
- `smart_home.clear_desired_state`
- `smart_home.list_pairing_sessions`
- `smart_home.list_workers`
- `smart_home.get_worker_heartbeat_schedule`
- `smart_home.get_supervision_plan`
- `smart_home.reconcile_desired_states`
- `smart_home.run_supervision_tick`
- `smart_home.pair_bridge`
- `smart_home.complete_pairing`
- `smart_home.observe_supervision`

## Development

```bash
bash BUILD
```
