# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added a Home Assistant-compatible read-only local API `web-core::WebApp` for
  config, state, services, and events over smart-home runtime snapshots.
- Added a live runtime-backed API constructor with `POST /api/services/:domain/:service`
  dispatch through runtime command authorization and command results.
- Added dashboard-ready runtime read routes for snapshot pending work,
  event-log replay, command-result audit, authorization-decision audit, and
  desired-state supervision targets.
- Added a state-history read route over registry-backed device events with
  entity-alias, event-type, and timestamp filtering.
- Added Home Assistant-style `/api/history/period` routes backed by the same
  runtime state-history projection.
- Added a Hue fixture controller example that serves the API through the repo
  HTTP server for manual local smoke tests.
