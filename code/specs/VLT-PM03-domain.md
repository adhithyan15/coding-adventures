# VLT-PM03 — Password-Manager Domain Model V1

## 1. Purpose

This specification defines the pure product-domain layer shared by every
password-manager host. It turns VLT02 typed records into password-manager item
documents, fixes the identity and metadata merge vocabulary, represents
no-loss conflicts, and produces views that never carry plaintext secret
fields.

The implementation lives in
`code/packages/rust/vault-pm-domain/`. It has no filesystem, network, clock,
entropy, process, environment, key-custody, encryption, or repository access.
Callers provide identifiers, operation IDs, timestamps, and causal relations.

## 2. Layer boundary

```text
CLI / web / desktop / extension / mobile
                 |
          application workflows
                 |
        +--------v---------+
        | vault-pm-domain  |  VLT-PM03
        | items and rules  |
        +--------+---------+
                 |
          vault-records    VLT02
```

VLT-PM03 owns product-only identifiers, validated item documents,
observed-remove metadata sets, last-writer-wins registers, revision candidates,
tombstones, conflicts, pure merge decisions, and default redacted host views.

VLT-PM03 does not serialize repository objects. VLT-PM01 owns canonical
repository formats, a future repository package owns commit ancestry and
publication, and VLT-PM02 owns opaque storage effects.

## 3. Opaque product identifiers

The following identifiers are closed fixed-width types:

| Type | Width | Role |
|---|---:|---|
| `ItemId` | 16 bytes | stable item identity |
| `CollectionId` | 16 bytes | stable collection identity |
| `AttachmentId` | 16 bytes | immutable attachment identity |
| `ConflictId` | 16 bytes | stable conflict identity |
| `RevisionId` | 32 bytes | content/repository revision identity |
| `OperationId` | 32 bytes | globally unique observed-set add identity |

This package never generates IDs. The application layer draws random IDs or
derives content identities in the appropriate cryptographic package.

User-boundary rendering is canonical uppercase, unpadded Crockford Base32.
Parsing is strict: exact encoded length, canonical alphabet, and zero unused
high bits are required. `Display` and `Debug` are redacted so an incidental log
cannot create a plaintext-to-item-ID lookup table. Callers must explicitly use
`to_user_string()` when an ID is intentionally shown to a user.

## 4. Content types

`ContentType` is a non-empty, bounded ASCII identifier with a maximum of 128
bytes. V1 permits lowercase letters, digits, `-`, `.`, `/`, `+`, `_`, and `:`.
Whitespace, controls, uppercase aliases, and unbounded future type strings are
rejected.

An `ItemDocument` constructor derives the expected content type from its VLT02
`AnyRecord` and rejects a caller-supplied schema that does not match. Unknown
VLT02 records remain opaque and retain their validated content type.

## 5. Mergeable metadata primitives

### 5.1 Observed-remove set

`ObservedSet<T>` records each add as `(value, OperationId)` and each removal as
the set of add-operation IDs observed at removal time.

- `add(value, operation_id)` is idempotent.
- `remove(value)` tombstones every currently observed add for that value.
- A later add with a new operation ID makes the value present again.
- `merge(a, b)` unions adds and removals.
- Merge is associative, commutative, and idempotent.
- Tombstones are retained. Only a future repository GC operation with proof
  that every retained head observed the removal may discard them.

The in-memory V1 document bounds present values. Before a repository decoder
accepts persistent observed sets, the Phase 0 security review must fix total
wire bounds for retained values, tombstones, and operation IDs plus the proof
required for safe compaction.

`OperationId` uniqueness is a caller invariant. A future repository can derive
it from a signed device/counter operation or a domain-separated commit value.

### 5.2 Last-writer-wins register

`LwwRegister<T>` carries a value, caller timestamp, and `OperationId` tie-break.
Merge selects the lexicographically greater `(timestamp_ms, operation_id)`.
This is deterministic under clock ties. V1 uses it for `favorite`; losing
values remain available through revision history rather than inside the
register.

## 6. Item document

```rust
pub struct ItemDocument {
    id: ItemId,
    schema: ContentType,
    created_at_ms: u64,
    updated_at_ms: u64,
    favorite: LwwRegister<bool>,
    collection_ids: ObservedSet<CollectionId>,
    tags: ObservedSet<String>,
    payload: AnyRecord,
    attachments: ObservedSet<AttachmentId>,
}
```

V1 bounds are 64 present collections, 64 present tags, 128 UTF-8 bytes per tag,
and 64 present attachments. `updated_at_ms` must not precede `created_at_ms`.
Empty tags and tags containing control characters are invalid.

The document has a custom redacted `Debug`. Its drop path explicitly zeroizes
the VLT02 payload, including unknown opaque payload bytes, and wipes tag text.
IDs and non-secret counters are not claimed to be secret in memory, but are not
printed implicitly.

Attachment metadata and bytes are deliberately absent. The document contains
only immutable attachment IDs; VLT14 and the repository package own encrypted
attachment manifests and chunks.

## 7. Revisions, tombstones, and conflicts

An `ItemCandidate` contains a `RevisionId`, a bounded ordered set of direct
causal parent revision IDs, and either one live `ItemDocument` or a tombstone
containing the stable `ItemId` and deletion time.

The repository determines the relation between two candidates and calls the
pure domain merge with `Same`, `LeftDescends`, `RightDescends`, or `Concurrent`.
The domain returns `MergeDecision::Selected`, `AutoMerged`, or `Conflict`.

For concurrent live candidates, identity and schema mismatch is corruption.
If payloads are equal, metadata merges automatically: observed sets union,
`favorite` uses its LWW register, creation time takes the minimum, and update
time takes the maximum. If payloads differ, the whole records become conflict
candidates. No password, seed, card, note, or opaque payload is merged field by
field.

Two concurrent tombstones select deterministically by `(deleted_at_ms,
revision_id)`. A concurrent tombstone and live edit is always a conflict.

`ItemConflict` retains both complete candidates, discovery time, a stable
caller-supplied `ConflictId`, and either `Unresolved` or
`Resolved { resolution_revision, resolved_at_ms }`. Resolution changes only
the state. It never removes either candidate. The repository later writes the
user resolution as a new revision whose parents include both candidates.

## 8. Redacted views

`RedactedItemView::from_document` is the default host projection. It contains
item identity, schema/kind, display metadata needed for an interactive client,
favorite and membership counts, plus a typed redacted payload view.

The view may carry titles, usernames, URLs, labels, and connection metadata
because those are needed for normal list/show behavior after unlock. It never
copies passwords, note bodies, card numbers, CVVs, TOTP seeds, API tokens,
database passwords, lease IDs, or opaque payload bytes. Their positions are
represented by `RedactedSecret`, presence booleans, lengths, or safe derived
hints such as a card's last four digits.

`Debug` for the complete view and every nested redacted payload omits even
display metadata. Thus the view is suitable for host rendering after unlock,
but diagnostic logging still sees only type and count information.

An explicit reveal workflow belongs in `vault-pm-application`; this package
does not expose a "redaction off" flag.

## 9. Errors

Errors are closed and low-resolution: `InvalidIdentifier`,
`InvalidContentType`, `InvalidTimestamp`, `InvalidTag`, `BoundExceeded`,
`SchemaMismatch`, `IdentityMismatch`, and `InvalidConflict`. `Display` uses
package literals only and never includes input content, IDs, tags, record
values, or attacker-controlled text.

## 10. Required verification

V1 tests must cover:

1. every identifier's canonical round trip and rejection of aliases;
2. redacted `Debug`/`Display` behavior;
3. content-type bounds and record/schema agreement;
4. observed-set add/remove/re-add behavior and merge laws;
5. deterministic LWW ties;
6. item bounds and timestamp validation;
7. concurrent metadata auto-merge;
8. whole-record and delete/edit conflict preservation;
9. deterministic tombstone merge;
10. conflict resolution retaining both candidates; and
11. redacted views containing no fixture secret or opaque payload bytes.

## 11. Security properties and non-goals

- No default formatter emits IDs or secret-bearing record fields.
- Merge never silently discards a concurrent secret edit.
- Unknown records are preserved but never projected as plaintext.
- No package API generates entropy, reads a clock, persists data, or invokes
  cryptography.
- This is not a wire format, repository, search index, import adapter, policy
  engine, or UI framework.

## 12. References

- `VLT-PM00-local-first-password-manager.md` — product architecture and phases.
- `VLT02-vault-records.md` — typed plaintext record schemas.
- `VLT-PM01-format.md` — canonical repository formats.
- `VLT-PM02-storage.md` — provider-neutral opaque storage contract.
- `VLT10-vault-sync-engine.md` — earlier generic merge primitives whose
  no-loss lessons inform this product-specific model.
