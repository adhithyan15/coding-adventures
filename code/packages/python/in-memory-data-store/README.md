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
