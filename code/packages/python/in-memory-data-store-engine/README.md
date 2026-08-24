# coding-adventures-in-memory-data-store-engine

Pure-Python execution engine for the repository's in-memory data store stack.
It provides binary-safe strings, hashes, lists, sets, sorted sets,
HyperLogLog, TTLs, 16 logical databases, and the shared protocol response IR.

```python
from in_memory_data_store_engine import DataStoreEngine

engine = DataStoreEngine()
engine.execute_parts([b"SET", b"answer", b"41"])
response = engine.execute_parts([b"INCR", b"answer"])
assert response.value == 42
```

## Development

This package sits above `hash-functions`, `hyperloglog`, and
`in-memory-data-store-protocol`. Its BUILD fronts install that local closure in
leaf-to-root order, recreate a package-local Python 3.13 `.venv`, and run every
quality gate through that environment:

```text
python -m ruff check src tests
python -m ruff format --check src tests
python -m mypy --strict --follow-untyped-imports src tests
python -m pytest tests/ -v
```

The checked-in `BUILD` recipe supplies `.venv/bin/python`; `BUILD_windows`
supplies `.venv\Scripts\python.exe`. Running either complete recipe twice is
supported and clears the previous environment before rebuilding it.
