# memory-store

Typed memory store built on storage-core

`memory-store` holds durable knowledge that should survive any one session.
This first phase keeps memory intentionally portable and simple: every memory is
one JSON record, and lexical search is implemented as a store-level scan rather
than a backend-specific index.

## What it owns

- `MemoryRecord`
- `MemoryClass`
- confidence/review updates
- supersede, expiry, and tombstone transitions
- lexical search across subject/body/tags

## Key layout

- `memory/records/<memory_id>.json`

## Current API

- `remember()`
- `fetch_memory()`
- `update_confidence()`
- `supersede_old_memory()`
- `list_by_class()`
- `list_by_tag()`
- `search_lexical()`
- `mark_expired()`
- `forget_tombstone()`

## Development

```bash
# Run tests
bash BUILD
```
