# VLT-PM19 — Audited Portable Restore Verification

## Status

Normative Phase 1A application contract for an opaque source expectation and
an independently reopened, publish-before-release target comparison. This
slice completes the storage-neutral semantic verifier. It does not yet create
or switch CLI target configurations.

## 1. Purpose

A successful portable import proves that one atomic target publication was
accepted. It does not by itself prove that a fresh target session can reopen
the durable result and independently match every source value. The application
therefore owns a separate verification capability and a distinct audit action.

The verifier must prove all of the following without exposing source or target
records to host orchestration:

- every source item remains one target item group;
- every source current candidate remains one target current candidate;
- live schema, timestamps, CRDT state, collection/tag/attachment observations,
  record variant, and complete field values are identical;
- tombstone deletion times are identical;
- conflict grouping is identical even though item and revision ordering change;
- target vault, item, and revision identities are disjoint from the source; and
- imported target revisions retain no source causal-parent identity.

## 2. Opaque expectation

After an encrypted artifact has authenticated and parsed through
`open_portable_with_passphrase`, but before import consumes the opened
snapshot, the application may derive `PortableRestoreExpectationV1`.

The type:

- has no clone, display, serialization, digest, identity, or field accessor;
- has redacted debug output;
- retains source vault/item/revision identities only for disjointness checks;
- wipes its semantic root on drop; and
- exposes no provider, path, passphrase, bootstrap, title, URL, username,
  schema, timestamp, deletion time, record body, or secret field.

Hosts may move the token across the import/reopen sequence but cannot inspect
or reconstruct its semantic contents.

## 3. Canonical semantic root

For each source item group, the application processes every validated current
candidate as follows:

1. replace only the item identity with the all-zero comparison identity;
2. retain complete live-document or tombstone state;
3. deliberately encode an empty causal-parent set because portable import is a
   current-state re-identification operation;
4. canonical-encode the normalized revision through the existing application
   revision codec;
5. hash a domain-separated, length-delimited candidate preimage; and
6. sort the fixed-size candidate hashes before hashing a domain-separated
   group preimage.

The application sorts all group hashes and hashes a final domain-separated
preimage containing exact item, candidate, and conflicted-item counts. Sorting
removes source/target identity ordering without erasing duplicate candidates
or candidate grouping. The existing canonical revision codec binds:

- content type and complete first-party or opaque record payload;
- creation and update timestamps;
- favorite LWW value, timestamp, and operation;
- retained observed-set values, add operations, and removal tombstones for
  collections, tags, and attachments; and
- live versus tombstone state and deletion time.

The target semantic root is compared with the expectation root using a
constant-time fixed-size comparison.

## 4. Independent target requirements

Verification consumes a newly opened `UnlockedVaultV1`, not the in-memory
session that prepared or published import. That target session must:

- authenticate its own durable owner state and bootstrap;
- verify and materialize the complete current repository;
- have an active operation-audit epoch; and
- represent the target after import recovery has reached stable `Active` state.

The verifier rejects the source vault as target. It also rejects any target
item or revision identity present in the source expectation, any retained
target causal parent, an empty candidate group, or a candidate whose state
identity does not match its target group.

Audit-only commits between import and verification do not change semantic
eligibility because comparison is over the verified current catalog.

## 5. Audit ordering

Independent verification is `PortableRestoreVerify`, not `PortableImport` and
not generic `VaultVerify`.

The application consumes the target session, the expectation, one advisory
wall time, one `AuditedAccessRandomnessV1`, and the local state store. It first
computes one closed operation result:

- exact match: `Succeeded`;
- any semantic, identity, grouping, or parent mismatch: `Failed` with the
  application integrity class.

It then publishes the itemless, revisionless audit event and advances owner
state through the ordinary audit-only crash journal. Only after publication
does the `AuditedAccessResultV1` release either the aggregate proof or the
closed error. Audit publication ambiguity withholds both and retains the exact
recovery journal.

The event contains no semantic root, count, source identity, target identity,
record metadata, field value, path, provider, or arbitrary detail.

## 6. Result surface

`PortableRestoreVerificationV1` contains only:

- matched item count;
- matched candidate count; and
- matched conflicted-item count.

Its debug representation contains only those aggregate values. It does not
authorize field disclosure, mutation, configuration switching, or a second
verification.

## 7. Error policy

- A pre-audit target returns invalid before comparison or result release.
- Semantic, grouping, identity, or parent mismatch returns integrity only after
  a failed verification event is durable.
- Repository, audit-event, commit, or owner-state publication failure returns
  the existing closed provider/concurrency/integrity class without releasing
  the comparison result.
- The application never reports which item, candidate, schema, timestamp,
  tombstone, CRDT member, or record field differed.

## 8. Acceptance gates

The slice is complete only when tests prove:

1. source and independently re-identified target semantics match;
2. source vault, item, or revision reuse is rejected;
3. retained target causal parents are rejected;
4. a same-count field-level change such as deletion time is rejected;
5. the expectation and result debug surfaces reveal no source values or IDs;
6. mismatch publishes a failed itemless `PortableRestoreVerify` event before
   its integrity error;
7. match publishes a succeeded itemless event before aggregate release;
8. the resulting audit chain verifies across restart; and
9. formatting, Clippy, rustdoc, and the application/audit package tests pass.

## 9. Remaining CLI recovery work

The next slice must compose this verifier with explicit target creation and
configuration switching. It must preserve or reacquire enough authenticated
source expectation state to retry verification after a post-import host or
provider interruption, and it must not print “fully verified restore” until a
durably reopened target has produced the succeeded audited aggregate proof.
