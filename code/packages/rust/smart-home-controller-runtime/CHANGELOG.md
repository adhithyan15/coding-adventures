# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Central restart-safe ownership of `SmartHomeRuntime`,
  `SmartHomeAutomationRuntime`, and `SmartHomeRuntimeStore`.
- Serialized clone-persist-publish transactions with exact rollback on callback,
  encoding, storage, and compare-and-swap failures.
- Shared runtime handles, HTTP persistence adapters, snapshot saves, automation
  evaluation and schedule ticks, and durable revision metadata.
- Local-folder restart, failure atomicity, and concurrent no-lost-update tests.
- Expected-revision controller transactions that reject stale work before
  invoking its mutation callback or changing in-memory or durable state.
