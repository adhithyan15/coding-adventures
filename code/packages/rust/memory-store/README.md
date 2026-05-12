# memory-store

Typed memory store built on storage-core

`memory-store` holds durable knowledge that should survive any one session.
This first phase keeps memory intentionally portable and simple: every memory is
one JSON record, and lexical search is implemented as a store-level scan rather
than a backend-specific index.

## What it owns

- `MemoryRecord`
- `MemoryRecordSummary`
- `MemoryClass`
- `MemoryLifecycleStatus` plus shared record/summary lifecycle helpers
- confidence/review updates
- supersede, expiry, and tombstone transitions
- lexical search across subject/body/tags, including active-at filtering and
  bounded result sets for tool calls
- bounded read selectors for class, tag, source, active-at, confidence,
  tombstone inclusion, sorting, and limits
- metadata-only memory summaries for read tools that should not return memory
  body text
- deterministic review candidates for low-confidence, stale, expiring, and
  expired memories

## Key layout

- `memory/records/<memory_id>.json`

## Current API

- `remember()`
- `fetch_memory()`
- `update_confidence()`
- `supersede_old_memory()`
- `list_memories_with_options()`
- `list_memory_summaries()`
- `list_by_class()`
- `list_by_tag()`
- `search_lexical()`
- `search_lexical_with_options()`
- `search_active_lexical_at()`
- `review_candidates()`
- `mark_expired()`
- `forget_tombstone()`

## Development

```bash
# Run tests
bash BUILD
```
