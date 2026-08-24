# Changelog

## Unreleased

- Made both BUILD fronts repeatable with a cleared, package-local Python 3.13
  environment and explicit interpreter ownership for install, Ruff, strict
  MyPy, and pytest.
- Broadened `EngineResponse.bulk_string`'s annotation to match its existing
  bytes-like normalization behavior.

## 0.1.0

- Initial Python parity package for in-memory data store protocol frames.
