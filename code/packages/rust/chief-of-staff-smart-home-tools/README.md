# chief-of-staff-smart-home-tools

`chief-of-staff-smart-home-tools` connects the D18D Chief of Staff tool runtime
to the D23 smart-home runtime.

The crate is intentionally a thin adapter:

- it publishes `smart_home.*` D18D tool definitions
- it registers in-process handlers on `InMemoryToolRuntime`
- handlers translate JSON arguments into `SmartHomeRuntime` read, discover,
  event-subscription, pair, and command requests, plus read-only D23A catalog
  queries
- `SmartHomeRuntime` still owns smart-home authorization, command validation,
  event subscriptions, pairing sessions, optimistic state, discovery scheduler
  policy and observability, supervision, and audit decisions
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
  -> scene inventory and scene detail reads
  -> discovery worker health and retry state in smart_home.observe_supervision
  -> event-log and subscription-backlog reads
  -> device command acceptance
  -> optimistic state update
  -> D18D trace and audit record
```

## Included Tools

- `smart_home.list_integrations`
- `smart_home.describe_integration`
- `smart_home.list_primitives`
- `smart_home.describe_primitive`
- `smart_home.discover`
- `smart_home.list_bridges`
- `smart_home.list_devices`
- `smart_home.list_scenes`
- `smart_home.describe_scene`
- `smart_home.get_state`
- `smart_home.describe_capabilities`
- `smart_home.get_health`
- `smart_home.command`
- `smart_home.subscribe`
- `smart_home.poll_events`
- `smart_home.unsubscribe`
- `smart_home.list_subscriptions`
- `smart_home.inspect_event_log`
- `smart_home.pair_bridge`
- `smart_home.observe_supervision`

## Development

```bash
bash BUILD
```
