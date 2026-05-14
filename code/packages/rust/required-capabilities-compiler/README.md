# Required Capabilities Compiler

Compiles a package `required_capabilities.json` into Rust operation boundary
source. The first generated boundary is an operation-side HTTP client wrapper
whose allowlist comes directly from declared `net:dns` and `net:connect`
capabilities.

Generated operation code calls `operation_primitives::OperationHttpClient`
before any transport runs, so operation implementations cannot accidentally
fetch a URL outside the manifest-declared domains.

Generate a statically linked operation module with:

```bash
cargo run -p required-capabilities-compiler -- \
  --input weather-agent-e2e/required_capabilities.json \
  --output weather-agent-e2e/src/generated_operations.rs
```

After generation, application code links the emitted Rust source. Runtime code
does not parse or trust a mutable JSON file for HTTP allowlist enforcement.
