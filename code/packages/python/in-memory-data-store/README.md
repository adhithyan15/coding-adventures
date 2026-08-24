# coding-adventures-in-memory-data-store

Pure-Python composition layer for the in-memory data store stack. It decodes
RESP2 commands, executes them through `in-memory-data-store-engine`, encodes
responses without losing their RESP types, and optionally persists successful
mutations to an append-only file with replay on startup.

```python
from in_memory_data_store import InMemoryDataStore

store = InMemoryDataStore()
assert store.execute_resp_bytes(b"*1\r\n$4\r\nPING\r\n") == b"+PONG\r\n"
```

## Development

This composition package installs the local stack in leaf-to-root order:
`hash-functions`; `hyperloglog`, `in-memory-data-store-protocol`, and
`resp-protocol`; `in-memory-data-store-engine`; then this package. Its BUILD
fronts recreate a package-local Python 3.13 `.venv` and run:

```text
python -m ruff check src tests
python -m ruff format --check src tests
python -m mypy --strict --follow-untyped-imports src tests
python -m pytest tests/ -v
```

The checked-in `BUILD` recipe supplies `.venv/bin/python`; `BUILD_windows`
supplies `.venv\Scripts\python.exe`. Both complete recipes are repeatable and
clear the prior environment before rebuilding it. AOF persistence remains an
optional runtime feature; its filesystem-authority contract is reviewed
separately from these build-only changes.
