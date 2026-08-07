# Changelog

## Unreleased

- Add schema-v2 per-channel payload-schema version declarations while retaining
  strict parsing and deterministic rendering for installed schema-v1 packages.
- Add a fail-closed originator/receiver channel compatibility check.

## 0.1.0 - 2026-08-03

- Define the shared typed schema-v1 agent manifest contract.
- Add strict parsing with explicit version compatibility and duplicate-key rejection.
- Preserve deterministic JSON generation for Level 1 and packaged agents.
