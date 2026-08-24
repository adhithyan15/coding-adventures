# Changelog

## Unreleased

- Recreate the BUILD environment with Python 3.13 and run Ruff, formatting,
  strict MyPy with followed local imports, and pytest through the named
  interpreter on both canonical and Windows fronts.
- Replace the broad missing-import suppression with sound response-value and
  decoded-AOF narrowing while preserving runtime behavior.

## 0.1.0 - 2026-07-14

- Add RESP composition and append-only persistence for the Python data store.
