# chief-of-staff-tool-api

`chief-of-staff-tool-api` owns the model-facing tool contract described by the
D18D Chief of Staff Tool API spec.

The crate intentionally stops at the contract layer:

- tool definitions and metadata
- provider-neutral JSON-schema-shaped exports for model gateway adapters
- query helpers for selecting catalog entries by family, side effects, tier,
  capability, tag, stability, and limit
- read-side query helpers for invocation requests, call records, event streams,
  and terminal results by scope, time window, status, approval state, outcome,
  references, metrics, sort order, and limit
- invocation requests
- structured tool events and final results
- JSON-schema-like argument validation
- first-phase built-in store/job tool definitions from the D18D catalog,
  including ContextStore, ArtifactStore, SkillStore, MemoryStore, and Job
  runtime parity definitions
- a deterministic in-memory registry for runtimes and tests
- explicit policy decisions for permission, tier, and approval gates
- explicit approval grants that let a previously gated invocation proceed
  without weakening permission or tier denials
- a deterministic in-memory runtime that validates, invokes handlers, emits
  ordered events, applies policy before handler execution, and returns canonical
  `ToolResult` records
- handler output validation against advertised tool output schemas before a
  completed result is emitted

Runtime adapters, sandboxing, approval gates, and built-in tool handlers can all
depend on this package without inventing their own wire vocabulary.

## Development

```bash
# Run tests
bash BUILD
```
