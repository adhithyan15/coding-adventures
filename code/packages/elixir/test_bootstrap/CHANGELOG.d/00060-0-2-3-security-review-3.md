## 0.2.3

- Round-3 fixes: the post-execution drain exits the engine if it swallows the caller's
  `:DOWN`; `bound_ms:` is validated (0..2^32-1) before an engine is spawned.
- Lifecycle tests now prove the engine, launcher and target are all dead after a caller
  kill and after the bound expires.
