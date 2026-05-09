# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Initial Chief of Staff Tool API core package.
- Canonical tool definition, invocation request, event, result, and metric types.
- JSON-schema-like input validation for model-facing tool arguments.
- First-phase built-in tool catalog definitions for context, artifact, memory,
  and job store/runtime tools.
- Deterministic in-memory tool registry with duplicate detection and call validation.
- Deterministic in-memory runtime that pairs definitions with handlers, validates
  invocations before execution, emits canonical events, and returns `ToolResult`
  records.
- Policy decision hooks and deterministic policy profiles for permission, tier,
  side-effect, and approval gates before handler execution.
