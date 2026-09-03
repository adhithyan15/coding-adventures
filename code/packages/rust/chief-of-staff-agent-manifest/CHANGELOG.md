# Changelog

## Unreleased

- Add schema-v4 `tool_capabilities`: the D18D tool-capability scopes granted to
  an agent, matched against a `ToolDefinition`'s `required_capabilities`. This
  is the last `HostProfile` field that had no signed source.
- Deliberately NOT derived from `allowed_tools`. Deriving would make the
  capability check always pass for an allowed tool, and would destroy the
  property it exists for: a new version of a tool that starts requiring a new
  capability is denied until the manifest is re-signed, so a tool cannot
  silently widen what it does to already-approved agents.
- Validate scopes as colon-delimited `[A-Za-z0-9_-]` (`smart_home:read`),
  mirroring `chief-of-staff-host-runtime`'s `validate_capability` exactly. Note
  the separator differs from a tool identifier's dot; `smart_home:read` is a
  capability and `smart_home.discover` is a tool.
- Add `MANIFEST_V3_VERSION`; v3 and v4 share the `allowed_tools` shape.

- Add schema-v3 `allowed_tools`: the D18D tool identifiers an agent may call.
  Before v3 the signed manifest named no tools at all, so `HostProfile
  .allowed_tools` had no signed source and a profile-backed supervisor could not
  be derived from the manifest.
- Require `allowed_tools` at v3, so "calls no tools" is declared as `[]` rather
  than defaulted into, and reject the field on v1 and v2 manifests so a consumer
  trusting `version` cannot be told something false about the signed bytes.
- Validate tool identifiers as namespaced (`artifact.write`), sorted, and
  deduplicated, bounded by `MAX_ALLOWED_TOOLS`. A bare namespace is rejected
  because it names no tool and invites prefix matching.
- Add `MANIFEST_V2_VERSION` and move `MANIFEST_VERSION` to 3; v2 and v3 share
  the per-channel payload-schema-version shape.
- Make `to_json` validate before rendering and return `ManifestError`. It
  emitted `allowed_tools` only at v3 and otherwise dropped the field silently,
  so a manifest whose `version` disagreed with its tool list rendered to a
  signed artifact authorizing something other than what its author declared,
  with no error raised anywhere.
- Evolve `code/specs/schemas/agent_manifest.schema.json` for v3 and add v3 cases
  to the capability-taxonomy gate. The schema is the contract a reviewer or a
  non-Rust consumer reads; leaving it at `enum: [1, 2]` with
  `additionalProperties: false` made a valid v3 manifest fail its own published
  schema, and made its tool surface invisible there.

- Add schema-v2 per-channel payload-schema version declarations while retaining
  strict parsing and deterministic rendering for installed schema-v1 packages.
- Add a fail-closed originator/receiver channel compatibility check.

## 0.1.0 - 2026-08-03

- Define the shared typed schema-v1 agent manifest contract.
- Add strict parsing with explicit version compatibility and duplicate-key rejection.
- Preserve deterministic JSON generation for Level 1 and packaged agents.
