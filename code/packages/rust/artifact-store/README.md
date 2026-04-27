# artifact-store

Typed artifact store built on storage-core

`artifact-store` separates a durable artifact manifest from its opaque revision
bodies so plans, exports, screenshots, and reports can be referenced by ID.

## What it owns

- `Artifact` manifests
- `ArtifactRevision` bodies and metadata
- label and retention updates
- collection-oriented listing

## Key layout

- `artifacts/manifests/<artifact_id>.json`
- `artifacts/revisions/<artifact_id>/<revision_id>.bin`

## Current API

- `create_artifact()`
- `fetch_artifact()`
- `append_revision()`
- `fetch_latest_revision()`
- `fetch_revision_by_id()`
- `list_by_collection()`
- `attach_labels()`
- `mark_retention()`

## Development

```bash
# Run tests
bash BUILD
```
