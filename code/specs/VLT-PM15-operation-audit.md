# VLT-PM15 - First-Class Operation Audit

Status: normative Phase 1A security track

Depends on: VLT09, VLT-PM01, VLT-PM03, VLT-PM04, VLT-PM05

## 1. Purpose

Every authenticated password-manager edit or access must be attributable and
traceable without leaking the data being protected. This specification defines
the product-specific audit event, its immutable repository integration, and the
ordering rules that make the audit trail part of the operation rather than an
optional side channel.

The existing `vault-audit` package is a reusable signed in-memory chain. The
existing `vault-pm audit verify` command is a repository-integrity traversal.
Neither currently records password-manager operations. This contract connects
those ideas through a new storage-neutral `vault-pm-audit` primitive and later
application/CLI slices.

The locked product rule is:

> An authenticated operation may not report a completed effect or disclose a
> result until its privacy-safe audit event is durably reachable from the
> active repository state.

## 2. What counts as an operation

The audit boundary is one user-visible application action, not each internal
encrypted-object read. V1 assigns closed actions for:

- audit epoch start and vault initialization;
- authenticated repository verification, diagnostics, and audit-history access;
- item create, list, read, update, delete, restore, history access, search, and
  conflict resolution; and
- portable import and export.

An `item show`, future secret reveal, clipboard copy, autofill, history show,
attachment export, TOTP display, browser fill, or API retrieval is an access.
Internal catalog traversal, object decryption, search-index rebuild, sync
verification, and rendering steps are facts inside one high-level operation and
do not each create another event.

Locked `status`, help, and grammar rejection reveal no vault content and do not
require an event. A wrong passphrase produces no vault access and cannot produce
a trustworthy device-signed vault event because the signing seed remains
locked. A later host-local attempt log may record such attempts but cannot be
presented as part of the authenticated vault chain.

## 3. Primitive boundary

`coding_adventures_vault_pm_audit` is a pure package. It owns:

- `AuditActionV1`, a closed product-operation registry;
- `AuditOutcomeV1::{Succeeded, Denied, Failed}`;
- `AuditEventV1`, the validated unsigned facts;
- `SignedAuditEventV1`, canonical encoding and the device signature; and
- `AuditError`, a closed payload-free failure taxonomy.

It imports no filesystem, cloud provider, clock, entropy, process, environment,
configuration, terminal, or repository implementation. Callers supply time,
trace randomness, observed heads, and the acting device seed. Persistence and
at-rest encryption belong to the application/repository composition.

This separation is required for web, desktop, browser, mobile, and bring-your-
own-cloud clients to produce identical event bytes and verification decisions.

## 4. Event fields

Each event binds:

| Field | Meaning |
|---|---|
| vault ID | exact vault receiving the event |
| device ID | certified device that performed the operation |
| device counter | counter reserved for the containing repository commit |
| trace ID | fresh 256-bit `OperationId` correlating the complete action |
| action | closed high-level operation |
| outcome | succeeded, denied, or failed after authentication |
| item ID | present only for item-scoped operations |
| selected revision | exact revision read or used as a mutation capability |
| result revision | new revision created by a successful item mutation |
| previous event | prior encrypted event object in this device's chain |
| basis heads | sorted repository heads observed before the operation |
| timestamp | advisory caller-supplied Unix milliseconds |
| signature | Ed25519 signature by the acting device |

The device counter and basis heads let a verifier bind the event to the exact
commit transition that carried it. The trace ID correlates application, CLI,
desktop, browser, or future IPC activity without reusing a title, URL, query,
username, provider request ID, or secret as a correlation key.

The V1 action registry is:

| Code | Action | Scope |
|---:|---|---|
| 1 | audit epoch start | vault |
| 2 | vault initialize | vault |
| 3 | vault verify | vault |
| 4 | vault diagnose | vault |
| 5 | audit history read | vault |
| 10 | item create | item |
| 11 | item list | vault |
| 12 | item read | item |
| 13 | item update | item |
| 14 | item delete | item |
| 15 | item restore | item |
| 16 | item history read | item |
| 17 | item search | vault |
| 18 | item conflict resolve | item |
| 20 | portable import | vault |
| 21 | portable export | vault |

Outcome codes are 1 succeeded, 2 denied, and 3 failed. Unassigned action and
outcome codes are unsupported rather than caller-defined extensions.

## 5. Privacy contract

An event must not contain:

- title, username, URL, search query, tag, or collection name;
- password, note body, TOTP seed, API key, private key, card data, database
  credential, opaque payload, or attachment bytes;
- plaintext attachment name or source/destination path;
- passphrase, derived key, nonce, wrapped key, signature key, or provider token;
- storage provider identity, account identity, bucket, path, or object URL; or
- arbitrary caller detail bytes.

Stable item and revision identities are sensitive metadata. They are allowed
inside the event only because the complete canonical signed event is sealed as
an encrypted application object before a backend receives it. Backends see an
opaque object ID, ciphertext size, publication timing, and access pattern.

`Debug`, display, error, status, and default list projections must not render
vault, device, trace, item, revision, head, or signature bytes. An explicit
authenticated audit-history surface may render canonical trace/item/revision
selectors after it has logged its own access.

## 6. Canonical encoding and signature

The signed event is one exact closed canonical CBOR map:

```text
1  version = 1
2  kind = operation-audit-v1
3  vault_id
4  device_id
5  device_counter
6  trace_id
7  action_code
8  outcome_code
9  optional item_id
10 optional selected_revision
11 optional result_revision
12 optional previous_event_object_id
13 sorted unique basis_head_object_ids
14 timestamp_ms
15 device_signature
```

Optional byte strings are canonical arrays of zero or one value. The signature
preimage is `"VPM-AUDIT-EVENT-v1" || canonical_fields_1_through_14`.

Decode rejects missing, duplicate, unknown, wrong-typed, trailing, oversized,
noncanonical, unsupported, and structurally inconsistent values. Verification
uses the signing public key from the exact authority-certified device named by
the event; an embedded untrusted replacement key is not accepted.

## 7. Structural rules

1. Device counters are non-zero.
2. Basis heads are strictly sorted, unique, and bounded by
   `MAX_COMMIT_PARENTS`.
3. New-vault initialization is counter one, has empty basis heads, has no prior
   event, and succeeds.
4. A pre-audit vault begins with one successful `AuditEpochStart`, non-empty
   basis heads, and no prior event. This makes the unlogged historical prefix
   explicit instead of pretending it was audited.
5. Every later event has a prior per-device event and non-empty basis heads.
6. Item-scoped actions carry exactly one item ID; vault-scoped actions do not.
7. Successful read/update/delete/restore/conflict actions name the exact
   selected revision.
8. Successful item mutations name the exact resulting revision.
9. Failed or denied operations do not claim a result revision.
10. Non-mutations do not claim a result revision.

Each device has its own signed prior-event chain. Concurrent devices therefore
form a set of independently ordered chains embedded in the signed repository
DAG; they do not race on a fictitious global sequence counter. Repository
commit ancestry supplies cross-device causal context.

## 8. Encrypted repository integration

The application integration slice adds `ObjectKind::AuditEvent` and seals the
canonical signed event with the existing V1 object envelope. The acting commit:

- uses the event's basis heads as its parents;
- uses the same device ID and counter as the event;
- lists the encrypted event object in `added_objects`; and
- advances an owner-private per-device audit head only through the existing
  crash-resumable publication journal.

Successful item mutations publish revision, catalog, audit event, and commit in
one journaled repository publication. There is no state in which the item
effect is active but its event is absent, or the event claims success while the
item effect is absent.

Successful read/access operations publish an audit-only commit: the catalog
root is unchanged, the encrypted event is newly reachable, and the device
counter advances. The application completes and durably activates that commit
before returning a secret-bearing or redacted result to the host.

Authenticated failures or denials publish an audit-only event before returning
the closed error when the unlocked session and repository remain healthy. If
the event cannot be published, the operation returns storage/integrity failure
and emits no result that could be confused with a completed access or effect.

## 9. Verification

Full audit verification must prove, for every audited commit:

1. the event object is hash-verified and decrypts under `ObjectKind::AuditEvent`;
2. canonical decode and the certified device signature pass;
3. event vault/device/counter match the containing commit;
4. basis heads exactly match the commit parents;
5. the prior event belongs to the same certified device and has a lower counter;
6. action/resource/revision/result shapes are valid;
7. mutation events correspond to the exact catalog transition and produced
   revision; and
8. every post-epoch authenticated-operation commit contains exactly one event.

Missing, duplicated, replayed, cross-vault, cross-device, detached, reordered,
restarted-epoch, or falsely successful events fail closed. A provider can still
withhold the entire newest commit; local signed pins and later multi-device
witnesses detect rollback according to VLT-PM04. External transparency
witnesses remain a later defense against deletion of all local evidence.

## 10. CLI ordering

For a successful authenticated access command, the CLI/application order is:

```text
parse -> lock writer -> unlock -> resolve exact operation facts
      -> prepare signed encrypted event -> publish audit-only commit
      -> activate owner state -> lock/wipe -> render result
```

For a successful mutation, operation facts and the event are prepared together
and published atomically. Output follows durable active-state installation.

An audit-history command logs its own access first and then reads from the new
active state, so the returned view may include the access event that authorized
that view. This is one event, not recursive self-auditing.

## 11. Delivery slices

The security track is deliberately split into reviewable PRs:

1. **Primitive:** closed event model, canonical codec, device signing,
   verification, redacted diagnostics, tests, and this contract.
2. **Repository integration:** encrypted audit object kind, per-device active
   head, audit-only catalog-reusing journal, migration epoch, atomic mutation
   publication, and complete verifier.
3. **Access enforcement:** audit-only commits before every authenticated
   list/show/history/search/verify/diagnose/export disclosure.
4. **Audit surface:** redacted `audit list`/`audit show TRACE`, trace-aware
   output, export policy, and tamper/fault/PTY acceptance.
5. **Cross-device witnesses:** sync verification, signer registry, retention,
   transparency/witness options, and rollback reporting.

Search does not outrank slices 1 through 4 because adding new access paths
before the common auditing boundary would expand unaudited surface area.

## 12. Acceptance for the primitive slice

Package tests prove:

1. deterministic signed encode/decode round-trip;
2. verification by the acting device public key;
3. tampered signatures and wrong device keys fail;
4. new-vault and migration-epoch genesis rules are explicit;
5. action/item/selected/result combinations fail closed;
6. basis heads are sorted, unique, and bounded;
7. unknown fields and oversized input fail closed; and
8. debug output contains no stable trace, item, or revision identity.

The primitive slice does not claim that existing CLI operations are already
audited. Product completion requires repository integration and access
enforcement to merge and a migration test to prove that every subsequent
authenticated operation has exactly one durable event.
