# Changelog

## Unreleased

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

- Add schema-v2 per-channel payload-schema version declarations while retaining
  strict parsing and deterministic rendering for installed schema-v1 packages.
- Add a fail-closed originator/receiver channel compatibility check.

## 0.1.0 - 2026-08-03

- Define the shared typed schema-v1 agent manifest contract.
- Add strict parsing with explicit version compatibility and duplicate-key rejection.
- Preserve deterministic JSON generation for Level 1 and packaged agents.
