# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added a Home Assistant-compatible read-only local API `web-core::WebApp` for
  config, state, services, and events over smart-home runtime snapshots.
- Added a live runtime-backed API constructor with `POST /api/services/:domain/:service`
  dispatch through runtime command authorization and command results.
