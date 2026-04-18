# D18A - Chief of Staff Stores

## Overview

D18 defines the secure runtime and message-passing world for Chief of Staff agents.
This document defines the persistent data layer that sits above raw storage and below
agent-facing APIs.

The key rule is simple:

- Context, Artifact, Skills, and Memory stores MUST be written against a repository-
  owned storage abstraction
- those stores MUST NOT know whether bytes live in a local folder, SQLite database,
  NAS mount, Google Drive folder, or some future backend

This separation is important for both portability and security. A model-facing or
agent-facing store should think in terms of records, indexes, snapshots, manifests,
and blobs. It should not think in terms of `open()`, SQL tables, Drive file IDs, or
SMB shares.

The existing content-addressable-storage packages already prove the monorepo pattern
we want: a repository-owned backend interface with swappable persistence engines. The
store abstraction in this spec should build on that idea, but it must be richer than
CAS because Chief of Staff stores need named records, metadata, conditional writes,
prefix listing, and leases.

---

## Where It Fits

```text
Agents / Session Kernel / Tool Runtime
    |
    +--> ContextStore
    +--> ArtifactStore
    +--> SkillStore
    +--> MemoryStore
              |
              v
        Storage Abstraction
              |
              +--> LocalFolderBackend
              +--> SqliteBackend
              +--> NasBackend
              +--> GoogleDriveBackend
              +--> Future backends
```

**Depends on:** D18 Chief of Staff, D20 JSON, D21 Capability Cage, future Vault and
job specs.

**Used by:** Session kernel, tool runtime, job framework, model gateway caching,
CLI/desktop/mobile clients, future sync services.

---

## Design Principles

1. **Bytes live behind one abstraction.** Backends differ; stores do not.
2. **The guaranteed contract is small.** Random backend-specific query features do not
   leak upward.
3. **Stores own their own indexes.** If a backend cannot query by tag or timestamp
   natively, the store still works because it maintains index records inside the same
   abstraction.
4. **Records are immutable by default.** Revisions create a new version; "latest" is a
   pointer.
5. **Blobs and metadata are separate concepts.** Metadata is JSON; blob bodies are
   opaque bytes.
6. **Conditional writes are required.** Stores need compare-and-swap semantics to avoid
   trampling concurrent writers.

---

## Storage Abstraction

### StorageRecord

```text
StorageRecord
|-- namespace      logical partition, e.g. "context", "artifacts", "skills", "memory"
|-- key            stable logical key within namespace
|-- revision       opaque backend revision / etag / monotonic version
|-- content_type   MIME type for the body
|-- metadata       JSON object owned by the store
|-- body           opaque bytes
|-- created_at     UTC timestamp
|-- updated_at     UTC timestamp
|-- content_hash   SHA-256 of body
```

### Required interface

```typescript
type Revision = string;

type StoragePutInput = {
  namespace: string;
  key: string;
  contentType: string;
  metadata: JsonValue;
  body: Uint8Array;
  ifRevision?: Revision | null;
};

type StorageListOptions = {
  prefix?: string;
  recursive?: boolean;
  pageSize?: number;
  cursor?: string | null;
};

type StorageBackend = {
  initialize(): Promise<void>;
  get(namespace: string, key: string): Promise<StorageRecord | null>;
  put(input: StoragePutInput): Promise<StorageRecord>;
  delete(namespace: string, key: string, ifRevision?: Revision | null): Promise<void>;
  list(namespace: string, options?: StorageListOptions): Promise<StoragePage>;
  stat(namespace: string, key: string): Promise<StorageStat | null>;
  acquireLease(name: string, ttlMs: number): Promise<StorageLease | null>;
};
```

### What is guaranteed

- point reads by `(namespace, key)`
- writes with optional compare-and-swap via `ifRevision`
- prefix listing
- basic metadata and timestamps
- advisory leases for compaction, index rebuilds, and background jobs

### What is NOT guaranteed

- SQL queries
- joins
- backend-native full-text search
- backend-native vector search
- filesystem path semantics
- strong multi-record transactions across the entire store

If a store needs richer query behavior, it must build that behavior explicitly using
its own index records.

---

## Backend packages

### Required pure implementations

- `storage-core`
  Defines portable types, errors, lease semantics, and test fixtures
- `storage-local-folder`
  Stores records as files plus sidecar metadata in a normal directory tree
- `storage-sqlite`
  Stores records in SQLite tables but still exposes the generic contract

### Optional backend implementations

- `storage-google-drive`
- `storage-nas`
- `storage-s3`
- `storage-postgres`

### Native vs pure policy

The default package in each language SHOULD be pure-language where possible. Native
accelerators MAY exist for hashing, streaming IO, or file watching, but the core
behavior must remain available without native extensions.

---

## ContextStore

### Purpose

ContextStore holds session context for agents and users:

- conversation transcripts
- compacted summaries
- working context snapshots
- model input checkpoints
- session-scoped instructions
- imported references from channels, artifacts, skills, and memory

### Key types

```text
ContextSession
|-- session_id
|-- owner_id
|-- title
|-- status          active | paused | archived
|-- latest_revision
|-- head_pointer    points to latest snapshot/checkpoint

ContextEntry
|-- entry_id
|-- session_id
|-- kind            user | assistant | tool_call | tool_result | summary | note | attachment_ref
|-- timestamp
|-- metadata
|-- body

ContextSnapshot
|-- snapshot_id
|-- session_id
|-- basis_entry_id
|-- token_estimate
|-- included_entry_ids
|-- summary_refs
|-- memory_refs
|-- artifact_refs
```

### Required operations

- create/open session
- append entry
- fetch ordered entries
- create snapshot
- fetch latest snapshot
- compact before entry X into summary Y
- archive session

### Storage layout

ContextStore owns these logical keyspaces:

- `context/sessions/<session_id>.json`
- `context/entries/<session_id>/<entry_id>.json`
- `context/snapshots/<session_id>/<snapshot_id>.json`
- `context/indexes/<session_id>/...`

The exact backend layout is private to the store implementation.

---

## ArtifactStore

### Purpose

ArtifactStore holds durable outputs and inputs that should be referenced by ID instead
of inlined into context:

- plans
- drafts
- patches
- reports
- screenshots
- exported files
- notebook outputs
- generated images

### Key types

```text
Artifact
|-- artifact_id
|-- collection       e.g. plans, drafts, exports, screenshots
|-- name
|-- content_type
|-- labels[]
|-- provenance       session_id, tool_id, job_id, agent_id
|-- latest_revision

ArtifactRevision
|-- revision_id
|-- artifact_id
|-- parent_revision_id?
|-- metadata
|-- body
|-- created_at
```

### Required operations

- create artifact
- append revision
- fetch latest revision
- fetch revision by id
- list by collection
- attach labels
- mark retained / temporary / exported

ArtifactStore MUST treat bodies as opaque bytes. Text, images, JSON, and binary files
all travel through the same abstraction.

---

## SkillStore

### Purpose

SkillStore holds reusable agent behaviors and their assets:

- skill manifests
- prompt templates
- examples
- bundled artifacts
- version metadata
- install source metadata

### Key types

```text
SkillManifest
|-- skill_id
|-- version
|-- name
|-- description
|-- entrypoints[]
|-- required_tools[]
|-- required_capabilities[]
|-- assets[]
|-- source

SkillAsset
|-- skill_id
|-- asset_path
|-- content_type
|-- checksum
```

### Required operations

- install skill
- load manifest
- list installed skills
- read asset by logical path
- activate/deactivate version
- uninstall skill

Skills are not executable because they live in storage. They become executable only
when loaded by the skill runtime.

---

## MemoryStore

### Purpose

MemoryStore holds durable knowledge extracted from work over time. It is intentionally
separate from ContextStore.

Use ContextStore for "what happened in this session."
Use MemoryStore for "what should be remembered across sessions."

### Memory classes

- `profile`
  Stable preferences, identity, teams, writing style, tone
- `fact`
  Concrete durable facts
- `episodic`
  Summaries of past sessions or events
- `procedure`
  Reusable workflows and habits
- `warning`
  Known failures, constraints, and risks

### Key types

```text
MemoryRecord
|-- memory_id
|-- class
|-- subject
|-- body
|-- confidence
|-- source_refs[]
|-- tags[]
|-- supersedes[]
|-- created_at
|-- reviewed_at?
|-- expires_at?
```

### Required operations

- remember
- update confidence
- supersede old memory
- list by class/tag
- search lexical index
- mark expired
- forget tombstone

### Embeddings

Vector embeddings are OPTIONAL and must not be part of the minimum storage contract.
If enabled later, MemoryStore owns the embedding index the same way it owns lexical
indexes today.

---

## Indexing

Every store MAY maintain secondary indexes under its own namespace. Example:

- `memory/indexes/by-tag/<tag>.json`
- `artifacts/indexes/by-collection/<collection>.json`
- `skills/indexes/by-name/<name>.json`
- `context/indexes/by-session/<session_id>.json`

This keeps the storage contract small while still allowing rich store behavior.

Phase 1 note: store implementations MAY derive list/search behavior by scanning
their own primary records first. Secondary indexes remain a store-owned
optimization, not part of the minimum `StorageBackend` contract.

---

## Error model

Stores MUST use repository-owned errors rather than leaking backend errors directly.

```text
StorageNotFound
StorageConflict
StorageUnavailable
StorageLeaseDenied
StorageValidationError
StorageBackendError
```

---

## Test Strategy

### Shared backend conformance tests

Every backend must pass the same suite:

1. initialize twice is safe
2. put then get round-trips metadata and bytes
3. compare-and-swap rejects stale revision
4. delete is idempotent
5. prefix list ordering is stable
6. advisory lease expires correctly

### Store tests

1. Context compaction preserves reconstructable history
2. Artifact revisions form a valid chain
3. Skill activation switches versions without corrupting assets
4. Memory supersede leaves an audit trail
5. All stores behave the same across local folder and SQLite backends

---

## Initial package plan

- `code/packages/rust/storage-core`
- `code/packages/rust/storage-local-folder`
- `code/packages/rust/storage-sqlite`
- `code/packages/typescript/storage-core`
- `code/packages/typescript/context-store`
- `code/packages/typescript/artifact-store`
- `code/packages/typescript/skill-store`
- `code/packages/typescript/memory-store`

Rust should define the reference contract and backend test harness. Other languages
should mirror the same types and semantics.
