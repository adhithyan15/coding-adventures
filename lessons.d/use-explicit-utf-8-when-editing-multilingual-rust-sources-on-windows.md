---
category: Rust
---

# Use explicit UTF-8 when editing multilingual Rust sources on Windows

A partial Railroad test-wiring script used Python's platform-default cp1252 when reading a multilingual Rust file. It failed before wiring the paint tests, while an earlier file edit had already succeeded. Always pass encoding="utf-8" for repository text reads and writes, inspect every command exit, and verify each new test module is wired before treating a test run as evidence.
