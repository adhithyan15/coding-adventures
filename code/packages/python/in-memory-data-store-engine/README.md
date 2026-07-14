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
