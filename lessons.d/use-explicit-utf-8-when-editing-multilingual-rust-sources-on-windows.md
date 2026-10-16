---
category: Rust
---

# Use explicit UTF-8 when editing multilingual Rust sources on Windows

A partial Railroad test-wiring script used Python's platform-default cp1252 when reading a multilingual Rust file. It failed before wiring the paint tests, while an earlier file edit had already succeeded. Always pass encoding="utf-8" for repository text reads and writes, inspect every command exit, and verify each new test module is wired before treating a test run as evidence.

This also applies to saved Python helpers containing non-ASCII documentation.
The D2 review-guard repair decoded UTF8 correctly but wrote with Windows' default
cp1252. Its em dash became an invalid UTF8 source byte and the next invocation
failed before any repository mutation. Preserve the broken helper/startup
receipt, decode its known actual encoding, then write explicit UTF8. Use explicit
encoding on both reads and writes of generated helper programs.
