# chief-of-staff-smart-home-tools

`chief-of-staff-smart-home-tools` connects the D18D Chief of Staff tool runtime
to the D23 smart-home runtime.

The crate is intentionally a thin adapter:

- it publishes `smart_home.*` D18D tool definitions
- it registers in-process handlers on `InMemoryToolRuntime`
- handlers translate JSON arguments into `SmartHomeRuntime` read, subscribe,
  discover, pair, and command requests
- `SmartHomeRuntime` still owns smart-home authorization, command validation,
  event subscriptions, pairing sessions, optimistic state, supervision, and
  audit decisions
- D18D still owns tool validation, policy decisions, event streams, terminal
  results, and execution journals

The first slice proves an end-to-end local path with an in-memory Hue-style
fixture:

```text
Chief of Staff job/session/agent
  -> D18D smart_home.discover / smart_home.command tool calls
  -> smart-home runtime authorization
  -> discovery records and unpaired bridge candidates
  -> device command acceptance
  -> optimistic state update
  -> D18D trace and audit record
```

## Included Tools

- `smart_home.discover`
- `smart_home.list_bridges`
- `smart_home.list_devices`
- `smart_home.get_state`
- `smart_home.describe_capabilities`
- `smart_home.get_health`
- `smart_home.command`
- `smart_home.subscribe`
- `smart_home.pair_bridge`
- `smart_home.observe_supervision`

## Development

```bash
bash BUILD
```
