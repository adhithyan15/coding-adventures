# artifact-store

Typed artifact store built on storage-core

`artifact-store` separates a durable artifact manifest from its opaque revision
bodies so plans, exports, screenshots, and reports can be referenced by ID.

## What it owns

- `Artifact` manifests
- `ArtifactRevision` bodies and metadata
- label and retention updates
- collection, label, retention, provenance, and bounded manifest listing
- catalog summaries for retention and revision coverage over selected artifacts
- provenance summaries for session/tool/job/agent attribution coverage
- manifest summaries for collection, content-type, and label coverage
- inventory summaries that compose catalog, provenance, and manifest status
- bounded revision-history listing without returning opaque bodies
- revision-history summaries for lineage, metadata, and body-size coverage

## Key layout

- `artifacts/manifests/<artifact_id>.json`
- `artifacts/revisions/<artifact_id>/<revision_id>.bin`

## Current API

- `create_artifact()`
- `fetch_artifact()`
- `append_revision()`
- `fetch_latest_revision()`
- `fetch_revision_by_id()`
- `list_revisions()`
- `revision_history_summary()`
- `list_artifacts()`
- `catalog_summary()`
- `provenance_summary()`
- `manifest_summary()`
- `inventory_summary()`
- `list_by_collection()`
- `attach_labels()`
- `mark_retention()`

`ArtifactListOptions` lets D18/D18D tool handlers compose a bounded read model
over artifact manifests without fetching revision bodies. Callers can filter by
collection, require one or more labels, select a retention state, require
session/tool/job/agent provenance, and cap the number of returned manifests
with `limit`.

`catalog_summary()` uses the same `ArtifactListOptions` read model to return
retention counts and revision coverage over the selected manifests. This lets
D18/D18D status tools answer lifecycle questions without fetching opaque
revision bodies.

`provenance_summary()` returns compact session, tool, job, and agent attribution
counts over the same selected manifests. This lets D18A/D18D status tools
detect tool/job outputs and unattributed artifacts before fetching individual
manifests or revision bodies.

`manifest_summary()` returns compact collection, content-type, and label
coverage over the same selected manifests. This lets status tools spot mixed
artifact inventories, unlabeled outputs, and broad content classes before
fetching revision bodies.

`inventory_summary()` composes catalog, provenance, and manifest summaries over
the same selected manifests. This gives D18A/D18D status tools one bounded
read-side response for lifecycle, attribution, and manifest-shape checks.

`ArtifactRevisionListOptions` gives read-side tools a bounded revision history
view with oldest-first or latest-first ordering, an optional revision cursor,
and an optional limit. It returns revision metadata, body length, and content
hashes without returning the opaque revision bodies.

`revision_history_summary()` uses the same bounded revision read model to count
root revisions, child revisions, metadata-bearing revisions, and total body
bytes over the selected window. This gives host/status tools a cheap aggregate
before deciding whether to fetch full revision bodies.

## Development

```bash
# Run tests
bash BUILD
```
