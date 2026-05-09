# chief-of-staff-tool-api

`chief-of-staff-tool-api` owns the model-facing tool contract described by the
D18D Chief of Staff Tool API spec.

The crate intentionally stops at the contract layer:

- tool definitions and metadata
- invocation requests
- structured tool events and final results
- JSON-schema-like argument validation
- first-phase built-in store/job tool definitions from the D18D catalog,
  including SkillStore read and lifecycle tools
- a deterministic in-memory registry for runtimes and tests
- explicit policy decisions for permission, tier, and approval gates
- a deterministic in-memory runtime that validates, invokes handlers, emits
  ordered events, applies policy before handler execution, and returns canonical
  `ToolResult` records

Runtime adapters, sandboxing, approval gates, and built-in tool handlers can all
depend on this package without inventing their own wire vocabulary.

## Development

```bash
# Run tests
bash BUILD
```
