# chief-of-staff-tool-api

`chief-of-staff-tool-api` owns the model-facing tool contract described by the
D18D Chief of Staff Tool API spec.

The crate intentionally stops at the contract layer:

- tool definitions and metadata
- invocation requests
- structured tool events and final results
- JSON-schema-like argument validation
- a deterministic in-memory registry for runtimes and tests

Runtime adapters, sandboxing, approval gates, and built-in tool handlers can all
depend on this package without inventing their own wire vocabulary.

## Development

```bash
# Run tests
bash BUILD
```
