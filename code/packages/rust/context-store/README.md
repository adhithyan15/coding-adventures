# context-store

Typed session context store built on storage-core

`context-store` translates portable storage records into the transcript model a
Chief of Staff runtime needs.

## What it owns

- `ContextSession` headers
- `ContextSessionCatalogSummary` projections for read-side lifecycle coverage
- ordered `ContextEntry` transcripts
- body-free `ContextEntrySummary` projections for read-side transcript indexes
- `ContextTranscriptSummary` aggregates for entry kind and time-span coverage
- `ContextSnapshot` checkpoints for compaction/resume
- `ContextSnapshotSummary` aggregates for compaction checkpoint coverage
- compare-and-swap session updates on top of `storage-core`
- bounded transcript reads by cursor, entry kind, timestamp range, and limit
- bounded session header listing by owner, status, sort, and limit
- bounded snapshot listing by basis entry, refs, sort, and limit

## Key layout

- `context/sessions/<session_id>.json`
- `context/entries/<session_id>/<timestamp>-<entry_id>.json`
- `context/snapshots/<session_id>/<snapshot_id>.json`

## Current API

- `create_session()`
- `open_session()`
- `list_sessions()`
- `session_catalog_summary()`
- `append_entry()`
- `fetch_entries()`
- `fetch_entry_summaries()`
- `transcript_summary()`
- `fetch_ordered_entries()`
- `create_snapshot()`
- `list_snapshots()`
- `snapshot_summary()`
- `fetch_latest_snapshot()`
- `compact_before_entry()`
- `archive_session()`

`transcript_summary()` uses the same bounded transcript window options as
`fetch_entry_summaries()` and returns counts by entry kind, metadata coverage,
and first/latest timestamps without reading opaque entry bodies.

`snapshot_summary()` uses the same bounded snapshot filters as
`list_snapshots()` and returns compaction checkpoint counts, reference counts,
and token estimate ranges without reading transcript bodies.

## Development

```bash
# Run tests
bash BUILD
```
