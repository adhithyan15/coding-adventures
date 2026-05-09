# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Initial Chief of Staff Tool API core package.
- Canonical tool definition, invocation request, event, result, and metric types.
- Tool catalog query helpers for filtering by family, side effects, tier,
  capability, tag, stability, and limit.
- JSON-schema-like input validation for model-facing tool arguments.
- First-phase built-in tool catalog definitions for context, artifact, memory,
  and job store/runtime tools.
- SkillStore read and lifecycle built-ins for listing, manifest/asset reads,
  install, activation, deactivation, and uninstall.
- Deterministic in-memory tool registry with duplicate detection and call validation.
- Deterministic in-memory runtime that pairs definitions with handlers, validates
  invocations before execution, emits canonical events, and returns `ToolResult`
  records.
- Policy decision hooks and deterministic policy profiles for permission, tier,
  side-effect, and approval gates before handler execution.
- Expanded D18D first-party catalog parity for ContextStore snapshots,
  ArtifactStore revision/list/retention tools, MemoryStore lifecycle tools, and
  Job runtime list/status/run/uninstall tools.
- Explicit `ToolApprovalGrant` support so approval-required calls can be replayed
  through the same validated runtime path while stale or mismatched grants are
  rejected before handler execution.
- Handler output validation against advertised tool output schemas before the
  runtime emits completed results.
