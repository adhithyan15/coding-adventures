---
category: Rust
---

# Validate emitted JSON with the existing driver before adding test dependencies

A profiling regression test used serde_json without checking this crate dependencies, causing E0433. Keep JSON schema and phase assertions in the existing compiled C# driver, which already has System.Text.Json, and invoke that case from Rust. Inspect the owning Cargo.toml before importing another crate.
