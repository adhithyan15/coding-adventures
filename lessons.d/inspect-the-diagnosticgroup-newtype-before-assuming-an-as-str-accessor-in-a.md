---
category: Rust
---

# Inspect the DiagnosticGroup newtype before assuming an as_str accessor in a scheduler fixture

The scheduler cap fixture called an invented `DiagnosticGroup::as_str` method.
This existing newtype exposes its string as `.0`. Inspect the declaration or
existing assertions before writing a fixture accessor; a type error must not
stand in for the intended runtime red regression.
