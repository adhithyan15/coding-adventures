# Changelog

## Unreleased

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

- Add a GET-only target inspection boundary for zero-side-effect deploy plans.
- Add explicit legacy ownership bootstrap bound to the repository, ref,
  destination, portable path set, content digests, and exact regular Git blob
  identities, committed through a non-forced compare-and-swap.
- Verify every unique legacy blob through the bounded read-only Git Data API,
  including Git object identity, SHA-256 content proof, and aggregate bytes;
  composite dry-run projects that proven ownership before validating the real
  deployment manifest against the target tree.

## 0.1.0 — 2026-09-20

Initial FM-B046 release.

- Validate and preflight complete Forme deploy manifests before target access.
- Preserve shared `gh-pages` siblings through per-owner manifests and
  base-tree inheritance while pruning only previously owned stale paths;
  reject unowned exact-path replacement and destructive file/directory
  transitions against unowned tree shape.
- Publish atomically through immutable blobs, trees, commits, and one
  non-forced ref update, after fully verifying the immutable candidate commit.
- Retry bounded transient requests and optimistic ref conflicts from a fresh
  base; fail closed on malformed or overlapping ownership.
- Provide a cancellation-aware, credential-explicit GitHub REST boundary with
  bounded response handling and sanitized diagnostics, with no shell,
  subprocess, filesystem, or ambient-environment access.
- Bound combined target paths and both individual and aggregate ownership
  state before publishing so a successful deploy cannot poison the next run.
