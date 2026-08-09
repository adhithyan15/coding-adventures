# VLT-PM02 — Password-manager object storage contract

**Status:** Draft V1
**Parent:** VLT-PM00 §11 and §23 Phase 0
**Depends on:** VLT-PM01 object identifiers

## 1. Purpose

This specification defines the smallest storage contract shared by the local
repository, cloud replicas, browser hosts, and test doubles. A conforming store
persists and enumerates opaque immutable bytes. It does not understand vault
records, keys, commits, signatures, or plaintext.

The contract deliberately targets the weakest useful provider semantics. Strong
conditional writes, checksums, range reads, change feeds, notifications, and
physical deletion are reported capabilities and optimization hints. They are
never prerequisites for correctness.

## 2. Security boundary

Storage is hostile. A provider or fault-injection backend may omit, replay,
duplicate, reorder, truncate, or corrupt data and may lie about optional
metadata. Repository code above this contract verifies content-derived object
IDs, signatures, ancestry, and pinned heads before accepting data.

A backend MUST:

- treat bucket IDs, object IDs, cursors, revisions, and bodies as opaque;
- never log or place object bodies in errors or debug output;
- never transform, compress, index, import, or interpret object bodies;
- enforce configured size and page bounds before allocating or issuing I/O;
- distinguish absent objects from authorization, withholding, corruption, and
  provider failures;
- return only committed complete bodies.

The contract does not hide access timing, object sizes, bucket membership, or
the fact that opaque objects belong to one configured provider account.

## 3. Primitive values and bounds

| Value | V1 representation | Bound or rule |
|---|---|---|
| `VaultLocator` | 32 opaque bytes | binds one store instance to one vault locator |
| `BucketId` | 32 opaque bytes | compared bytewise; no caller-readable names |
| `ObjectId` | VLT-PM01 32-byte ID | compared bytewise; storage does not recompute it |
| `ObjectBytes` | opaque byte string | at most 64 MiB |
| `ListCursor` | backend-owned byte string | at most 256 bytes; bound to one bucket |
| `ChangeCursor` | backend-owned byte string | at most 256 bytes; bound to one store |
| provider revision | UTF-8 single-line token | 1–256 bytes |
| list limit | positive integer | at most 10,000 entries |
| change page | backend selected | at most 1,000 events |

Fixed-width identifiers have value semantics. Their `Debug` representation MAY
show a short type label but MUST NOT reveal their bytes. Bodies and cursors MUST
use redacted `Debug` output.

## 4. Store lifecycle

`initialize(locator)` is idempotent. The first successful call binds a store
instance and its configured container to that locator. Repeating the same call
succeeds without replacing data. Initializing the same instance with a different
locator returns `Conflict`.

All data operations before successful initialization return `NotInitialized`.
Initialization may create provider-owned container metadata, but MUST NOT write
plaintext vault names or user data.

## 5. Normative operations

```rust
pub trait VaultObjectStore: Send + Sync {
    fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError>;
    fn capabilities(&self) -> BackendCapabilities;
    fn get(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<Option<ObjectBytes>, StoreError>;
    fn stat(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<Option<ObjectStat>, StoreError>;
    fn put_immutable(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
        bytes: &ObjectBytes,
    ) -> Result<PutImmutableOutcome, StoreError>;
    fn list(
        &self,
        bucket: &BucketId,
        cursor: Option<&ListCursor>,
        limit: usize,
    ) -> Result<ObjectPage, StoreError>;
    fn delete_unreferenced(
        &self,
        bucket: &BucketId,
        object: &ObjectId,
    ) -> Result<DeleteOutcome, StoreError>;
    fn changes(
        &self,
        cursor: Option<&ChangeCursor>,
    ) -> Result<Option<ChangePage>, StoreError>;
}
```

### 5.1 Exact reads

`get` returns the exact committed bytes or `None`. `stat` returns the exact body
length plus an optional opaque provider revision and optional provider checksum.
Metadata is advisory; object-ID verification remains repository-owned.

A body shorter or longer than the committed stat, a checksum disagreement, or
different bytes observed for the same `(bucket, object)` is `Corruption`, not
`NotFound` or `Conflict`.

### 5.2 Immutable put

`put_immutable` is the only V1 write:

| Prior logical state | Requested bytes | Outcome |
|---|---|---|
| absent | any bounded bytes | `Created` |
| present | byte-for-byte identical | `AlreadyPresent` |
| present | different | `Corruption` |

The outcome is idempotent across retries. A provider without atomic
create-if-absent may upload to a temporary physical name, race publication,
re-read all contenders, and collapse identical duplicates. It MUST never report
success until a subsequent exact read can return the requested complete bytes.

### 5.3 Listing and cursors

Each page contains unique logical object IDs in ascending bytewise order. A
cursor is an exclusive continuation position and is scoped to its originating
bucket. Reusing it with another bucket or passing a malformed/oversized cursor
returns `InvalidInput`.

For an unchanged store, following `next_cursor` to exhaustion returns every
object in the bucket exactly once. Concurrent additions MAY appear in the
current traversal when their ID is after the cursor. Additions at or before the
cursor may wait until the next complete traversal. Consequently, correctness
uses repeated complete scans; change feeds and strong list-after-write only
reduce latency.

`next_cursor` is present exactly when another page may exist. A conforming store
MUST NOT return an empty page with a continuation cursor for an unchanged
bucket.

### 5.4 Deletion

Physical deletion is optional. Unsupported stores return `Unsupported` without
side effects. A supported store returns `Deleted` or `Missing` idempotently.
The method name is an assertion by repository GC that the object is
unreferenced; the backend does not decide reachability.

### 5.5 Change hints

`changes` is optional and returns `Unsupported` when no change-feed capability
is present. A supported feed returns `Ok(None)` when no events exist after the
requested cursor. Otherwise events carry a monotonically increasing
backend-local sequence, bucket, object ID, and `Put` or `Delete` kind. Events
are hints: they may be duplicated or delayed, and consumers still perform
periodic full list scans. A cursor resumes strictly after its last sequence.

## 6. Capability report

```rust
pub struct BackendCapabilities {
    pub strong_read_after_write: bool,
    pub strong_list_after_write: bool,
    pub conditional_create: bool,
    pub conditional_replace: bool,
    pub change_feed: bool,
    pub push_notifications: bool,
    pub resumable_upload: bool,
    pub range_read: bool,
    pub server_checksum: bool,
    pub physical_delete: bool,
    pub shareable_container: bool,
    pub max_object_size: Option<u64>,
    pub preferred_pack_size: u64,
}
```

`max_object_size`, when present, MUST be no greater than the provider's tested
limit. V1 callers also enforce the contract-wide 64 MiB limit. A preferred pack
size of zero means no provider preference. Capability reports are stable for
one configured backend session; a changed report triggers a new storage health
check, not weaker verification.

## 7. Error taxonomy

| Error | Meaning | Default retry behavior |
|---|---|---|
| `InvalidInput` | invalid bound, token, cursor, or argument | never retry unchanged |
| `NotInitialized` | lifecycle violation | initialize first |
| `Authorization` | credentials absent, expired, or rejected | reauthorize |
| `Quota` | account/container quota exhausted | user action |
| `RateLimited` | provider request budget exhausted | retry after bounded delay |
| `Network` | transport unavailable/interrupted | bounded retry |
| `Corruption` | immutable identity or bytes contradicted | stop and surface |
| `Conflict` | locator/container identity conflict | stop and surface |
| `Unsupported` | optional operation unavailable | select baseline path |
| `Provider` | typed provider failure not covered above | bounded retry or surface |

Errors MUST NOT contain body bytes, identifiers, provider tokens, paths, or raw
provider response bodies in `Display` or `Debug`. Provider adapters may retain a
redacted stable reason code and retry delay in private telemetry.

## 8. Reference in-memory backend

Phase 0 ships a deterministic, thread-safe `InMemoryObjectStore` with:

- atomic immutable puts;
- exact point reads and stats;
- ascending cursor pagination;
- optional deletion and change-feed support reflected in capabilities;
- monotonically increasing revisions and change sequences;
- no clock, filesystem, network, environment, process, entropy, or secret-key
  authority.

The reference backend is the executable model for adapter conformance. Its
default configuration implements every optional capability that can be modeled
in memory except notifications, range reads, resumable uploads, and sharing.

## 9. Deterministic fault injection

`FaultInjectingObjectStore<S>` wraps any store. Tests enqueue one-shot actions
for a specific operation. Actions are consumed in FIFO order and can:

- return a typed authorization, quota, rate-limit, network, corruption,
  conflict, unsupported, or provider error;
- corrupt one successful `get` body without changing the inner store;
- omit the last entry from one successful list page to model a stale listing;
- duplicate the first entry in one successful list page to model a broken or
  adversarial provider adapter;
- commit one put in the inner store and then return `Network` to model an
  ambiguous response.

Faults contain no attacker-controlled message. With an empty fault queue the
wrapper is transparent and conforming. Some injected successful responses are
intentionally non-conforming so repository verification and retry logic can be
tested against hostile storage.

## 10. Conformance suite and fixture

The language-neutral fixture is
`code/specs/fixtures/vault-pm-storage-v1.json`. It fixes identifiers, byte
payloads, bounds, operations, and expected outcomes. Implementations MUST run
the equivalent sequence through their public interface.

The reusable conformance runner verifies at minimum:

1. initialization is idempotent and locator-bound;
2. pre-initialization operations fail closed;
3. absent `get` and `stat` return `None`;
4. immutable put distinguishes create, replay, and conflicting bytes;
5. get returns exact bytes and stat returns exact length;
6. pagination is ordered, unique, cursor-bound, and exhaustive;
7. invalid limits and cursors fail without backend mutation;
8. optional deletion behavior matches its capability;
9. optional change-feed behavior matches its capability;
10. bodies and identifier bytes are absent from debug/errors;
11. ambiguous-after-commit retry converges to `AlreadyPresent`;
12. stale and corrupted reads remain detectable by the repository-facing test.

An adapter passes only when the baseline suite succeeds with its optional
capabilities both enabled where supported and disabled/fallback where not.

## 11. Package boundary

`vault-pm-storage` owns only values, the trait, the reusable conformance runner,
the in-memory model, and deterministic fault injection. It MUST NOT import a
filesystem, HTTP client, OAuth SDK, provider SDK, record codec, custody provider,
clock, or randomness source.

Later packages own:

- `vault-pm-storage-storage-core`: mapping bucket/object IDs to opaque
  `storage-core` namespace/key values;
- provider adapters such as Google Drive, WebDAV, and S3;
- caching, retry, rate limiting, metrics, and replica-set decorators;
- repository verification, publication ordering, discovery, merge, and GC.

## 12. Acceptance gates

- the spec, fixture, Rust model, README, changelog, and capability manifest agree;
- the reference backend passes the reusable suite with pagination limits of 1,
  2, and the maximum allowed value;
- property-style generated object sets preserve sorted exhaustive listing and
  immutable replay behavior;
- fault actions are one-shot, deterministic, and cannot leak bodies;
- line and branch coverage exceed 90%;
- clippy passes with warnings denied;
- the monorepo build graph recognizes the new package and affected closure;
- no filesystem, network, process, environment, clock, CSPRNG, or secret-key
  capability is declared or used.
