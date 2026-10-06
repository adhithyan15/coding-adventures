# `@coding-adventures/forme-authoring-core`

The capability-free data foundation for Forme's authoring shell. It validates
one bounded project format, applies semantic immutable edits, and persists each
acknowledged edit through compare-and-swap autosave. Undo and redo are part of
the persisted state, so restarting the shell restores the same history cursor.

This is the first FM09 / FM-B062 authoring slice. It deliberately contains no
DOM, filesystem, network, environment, pipeline, deploy, or Tauri code. Those
boundaries are injected by later editor and shell packages.

## Why the storage contract uses compare-and-swap

Two windows can open the same project. A last-writer-wins `save(bytes)` API
would quietly discard one person's work. The adapter instead returns an opaque
revision from `load` and accepts it as `expectedRevision` on the next write.
An adapter must atomically reject a stale token. The core publishes the new
in-memory snapshot only after that write succeeds.

```text
edit command
    │
    ▼
validate complete next project
    │
    ▼
append bounded history + canonicalize
    │
    ▼
compareAndSwap(old revision, bytes)
    │ success                 │ conflict / failure
    ▼                         ▼
publish snapshot          retain old snapshot
```

## Project shape

Projects contain a title, theme selection, optional HTTP(S) base URL, closed
publication workflow metadata, and ordered documents. Document bodies are the repository's backend-neutral
`DocumentNode` Content IR. The codec accepts the normal authorable block and
inline vocabulary but rejects raw backend nodes, unsafe URL schemes, cycles,
shared objects, prototypes, accessors, sparse arrays, unknown fields, duplicate
document identities/slugs, and dangling active-document references.

```ts
import {
  createAuthoringProject,
  openAuthoringSession,
  type AuthoringStorage,
} from "@coding-adventures/forme-authoring-core";

const project = createAuthoringProject({
  projectId: "01952c0d-7e63-7000-8000-000000000001",
  title: "My site",
});

declare const storage: AuthoringStorage;
const session = await openAuthoringSession({ storage, initialProject: project });

await session.dispatch({
  type: "configure-site",
  title: "My published site",
  baseUrl: "https://example.com",
  themeId: "forme-classless",
});

await session.undo();
await session.redo();
```

The initial save uses `expectedRevision: null`. Reopening a missing store
without `initialProject` fails; malformed stored bytes also fail without being
replaced. Recovery or reset must therefore be an explicit shell workflow.

## Commands

- `create-document`
- `remove-document`
- `update-document-metadata`
- `replace-document-body`
- `configure-site`
- `set-active-document`
- `record-publication` (only through `dispatchAtRevision`)

Commands run sequentially even when callers dispatch concurrently. A new edit
after undo discards the redo branch. Retained history defaults to 100 snapshots
and can be lowered or raised to the hard maximum of 200.

The publication command records the exact authoring revision, canonical
manifest SHA-256, and reviewed target identity while marking every document in
that unchanged snapshot published. `dispatchAtRevision` rejects stale queued
edits before persistence so an older external deployment cannot label newer
content as published.

## Hard limits

| Resource | Maximum |
|---|---:|
| Canonical stored JSON | 8 MiB |
| Documents | 1,000 |
| Document depth | 64 |
| Nodes per document | 100,000 |
| Inline/literal string | 1 MiB Unicode scalars |
| Title | 512 Unicode scalars |
| Slug or URL | 2,048 Unicode scalars |
| History | 200 snapshots |

Callers may select lower limits for a constrained host. Values above these
ceilings are rejected.

## Storage adapters

An adapter implements only:

```ts
interface AuthoringStorage {
  load(signal?: AbortSignal): Promise<StoredAuthoringState | null>;
  compareAndSwap(
    expectedRevision: string | null,
    bytes: Uint8Array,
    signal?: AbortSignal,
  ): Promise<{ revision: string }>;
}
```

The adapter owns atomic publication, restrictive permissions, and any host
capabilities. Revisions are opaque to the core. An acknowledged adapter write
must already be durable; debounced persistence belongs above semantic input
coalescing, not below it.

Adapter rejection strictly guarantees that no publication occurred.
Cancellation is observed only before the atomic commit point; after
publication an adapter must resolve successfully with the committed revision.
If a successful commit returns a malformed result, the core reports
`STORAGE_INDETERMINATE` and the shell must reload before retrying because
durable state may have advanced.
Rejected adapter operations surface as bounded `STORAGE_ERROR` diagnostics;
stale-revision conflicts and standardized `AbortError` cancellation remain
distinct without exposing host paths or exception messages.

## Development

```bash
bash BUILD
```

The gate compiles the public declarations and runs 95%+ statement/line and
90%+ branch coverage. See [FM09](../../../specs/FM09-forme-authoring-shell.md)
for the complete authoring path.
