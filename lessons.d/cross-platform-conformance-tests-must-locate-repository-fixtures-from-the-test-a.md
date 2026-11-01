---
category: Testing & coverage
---

# Cross-platform conformance tests must locate repository fixtures from the test assembly ancestry instead of fixed relative working-directory paths.

The SHA-512 C# and F# conformance tests initially opened the neutral corpus through a fixed relative path. That happened to resolve in the local and Linux test layouts, but the Windows .NET runner used a different output-directory shape and resolved the path under `code/packages/specs`, failing before the implementations were exercised. Locate repository-owned fixtures by walking upward from `AppContext.BaseDirectory` and checking the canonical `code/specs/fixtures/...` path, as the established cross-platform conformance suites do. Validate at least one Windows-shaped output layout whenever a test depends on repository-relative data.
