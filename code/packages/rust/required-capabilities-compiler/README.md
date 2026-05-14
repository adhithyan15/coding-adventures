# Required Capabilities Compiler

Compiles a package `required_capabilities.json` into Rust operation boundary
source. The first generated boundary is an operation-side HTTP client wrapper
whose allowlist comes directly from declared `net:dns` and `net:connect`
capabilities.

Generated operation code calls `operation_primitives::OperationHttpClient`
before any transport runs, so operation implementations cannot accidentally
fetch a URL outside the manifest-declared domains.
