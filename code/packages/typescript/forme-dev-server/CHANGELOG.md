# Changelog

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

## 0.1.0 — 2026-08-31

- Added a loopback HTTP preview server backed by in-memory Forme artifacts.
- Added SSE live reload with automatic browser reconnection.
- Preserved the last successful snapshot across failed rebuilds.
- Added a build-status endpoint, safe error page, MIME handling, and HEAD support.
