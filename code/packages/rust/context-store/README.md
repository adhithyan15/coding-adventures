# context-store

Typed session context store built on storage-core

`context-store` translates portable storage records into the transcript model a
Chief of Staff runtime needs.

## What it owns

- `ContextSession` headers
- ordered `ContextEntry` transcripts
- `ContextSnapshot` checkpoints for compaction/resume
- compare-and-swap session updates on top of `storage-core`

## Key layout

- `context/sessions/<session_id>.json`
- `context/entries/<session_id>/<timestamp>-<entry_id>.json`
- `context/snapshots/<session_id>/<snapshot_id>.json`

## Current API

- `create_session()`
- `open_session()`
- `append_entry()`
- `fetch_ordered_entries()`
- `create_snapshot()`
- `fetch_latest_snapshot()`
- `compact_before_entry()`
- `archive_session()`

## Development

```bash
# Run tests
bash BUILD
```
