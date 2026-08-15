# D18P - Chief of Staff Portable Durable Channel Profile

## Overview

D18 channels are one-way, append-only, encrypted pipes with one originator and
one or more independent receivers. This profile makes the deterministic part of
the production Rust channel implementation portable across Python, TypeScript,
Go, Ruby, Rust, and Elixir without creating a second persistence model.

The compatibility baseline is the existing Rust
`chief-of-staff-channel-store` and `chief-of-staff-channel-endpoints` behavior:

- the `D18C` version 1 durable channel definition
- the `D18S` version 1 next-sequence and pending-reservation record
- the `D18H` version 1 reserved authenticated header
- the `D18A` version 1 receiver cursor
- the `chief-channels` namespace and deterministic record keys
- atomic create, compare-and-swap, recovery, paging, acknowledgement, and
  irreversible lifecycle rules

`D18P` is the profile name. It does not replace any record magic. In particular,
`D18G` remains the existing sealed receiver-grant record, `D18H` remains the
reserved message-header record, and `D18M` remains the encrypted message record.

---

## Scope and ownership

This profile owns:

- immutable channel membership and lifecycle
- canonical channel-definition, reservation-state, and receiver-cursor bytes
- storage namespace, content types, metadata, and deterministic record keys
- the minimum atomic storage contract required by every implementation
- reserve-before-encrypt append semantics and crash recovery
- immutable message/grant persistence and ordered paging
- independent monotonic receiver acknowledgement
- structural originator and receiver authorization
- portable failure classes and conformance requirements

This profile does not own:

- `D18M` encrypted-message fields, creation, verification, or JSON; D18F owns
  those rules
- `D18G` sealed-grant cryptography, receiver key agreement, or key rotation;
  [D18Q](D18Q-chief-of-staff-channel-key-grant-profile.md) and issue #141 own
  those rules
- concrete filesystem, database, or cloud storage backends
- clocks, UUID generation, randomness, private-key custody, or zeroization
- process supervision, actor routing, host transport, or pipeline wiring
- retention, compaction, or deletion of channel history

A D18P implementation treats a `D18M` record as an immutable authenticated
value supplied by a conforming D18F implementation. It treats a `D18G` record as
an immutable receiver-bound value supplied by the channel-encryption protocol.
D18P decides where and when those records may be stored; it does not weaken or
duplicate their cryptographic verification.

---

## Normative vocabulary

The words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are normative.

All integers are unsigned. `u8`, `u32be`, and `u64be` mean fixed-width unsigned
integers, with the latter two encoded in big-endian byte order. Lengths count
octets, not characters. `bytes[n]` means exactly `n` octets.

An **atomic create** writes a record only if its `(namespace, key)` is absent at
the instant of commit. A **revision CAS** replaces a record only if its opaque
revision still equals the revision read by the caller. A failed condition MUST
not modify the record.

`next_sequence` is the first sequence that has not been reserved. A receiver
**cursor** is the first sequence that has not been acknowledged. Neither value
is the sequence of the last completed message.

---

## Immutable channel definition

```text
ChannelDefinition
|-- channel_id             bytes[16], UUID v7
|-- originator
|   |-- agent_id           bytes[1..4096]
|   `-- ed25519_public_key bytes[32]
|-- receivers              1..1024 ReceiverIdentity values
|   |-- agent_id           bytes[1..4096]
|   `-- x25519_public_key  bytes[32]
|-- created_at_ns          u64
|-- key_epoch              u64
`-- lifecycle              active | destroyed
```

The channel ID MUST contain an RFC 9562 UUID version 7 with the RFC variant
bits. The originator ID and every receiver ID are opaque bytes. They need not be
UTF-8, but they must satisfy the non-empty length bound.

Exactly one originator exists. The receiver set is non-empty, contains no
duplicate agent ID, and does not contain the originator ID. Public keys do not
establish identity by themselves; the definition binds each key to its agent
ID.

The canonical in-memory receiver order is ascending lexicographic order of raw
agent-ID bytes. Constructors and decoders MUST own or copy mutable inputs.
Public values MUST be immutable according to the language's normal conventions.

`created_at_ns` is injected by the caller. D18P neither reads a clock nor
assigns wall-clock meaning to that value. `key_epoch` identifies the only epoch
accepted for a new publish. Epoch advancement and new grant creation belong to
#141; D18P persists the selected value.

The lifecycle has one permitted transition:

```text
active  --destroy-->  destroyed
```

There is no reactivation transition. Destroying a channel denies all new
endpoint operations but does not delete definitions, reservations, messages,
grants, or cursors.

---

## `D18C` version 1 definition record

```text
offset  field                              encoding
------  ---------------------------------  -------------------------------
0       magic                              ascii("D18C"), 4 bytes
4       version                            0x01, 1 byte
5       channel_id                         bytes[16]
21      originator_id_length               u32be, maximum 4096
25      originator_id                      declared bytes, minimum 1
...     originator_ed25519_public_key       bytes[32]
...     receiver_count                     u32be, 1 through 1024
...     receivers, repeated receiver_count times:
        receiver_id_length                 u32be, maximum 4096
        receiver_id                        declared bytes, minimum 1
        receiver_x25519_public_key          bytes[32]
...     created_at_ns                       u64be
...     key_epoch                           u64be
...     lifecycle                           u8: 0 active, 1 destroyed
EOF     no trailing bytes permitted
```

Encoders MUST emit receivers in canonical agent-ID order. For compatibility
with the shipped version 1 decoder, decoders MAY accept a non-canonical receiver
order, but they MUST sort the resulting immutable value before returning it.
Duplicate IDs remain invalid regardless of encoded order. Re-encoding a decoded
record therefore produces canonical order.

Decoders MUST reject unknown magic, versions other than 1, truncation, length
overflow, identity lengths outside the high-level bounds, receiver counts
outside 1 through 1024, invalid UUID-v7 bits, duplicate receivers,
originator/receiver overlap, unknown lifecycle values, and trailing bytes.

Structural decoding never authorizes an endpoint. Authorization requires the
record to be loaded from its exact storage key and content type and to match the
caller-supplied identity and public key.

---

## Storage contract

### Required operations

Every implementation MUST depend on an injected storage interface with these
semantics:

```text
initialize() -> void
get(namespace, key) -> Record | null
put(namespace, key, content_type, metadata, body,
    if_absent | if_revision) -> Record
list(namespace, prefix, recursive=true, page_size, cursor) -> Page
```

`Record.revision` is an opaque, non-empty token. A successful write returns the
new revision. `list` returns records in ascending lexicographic key order and an
opaque continuation cursor when another page exists.

Exactly one of `if_absent` or `if_revision` is used for every D18P mutation.
Unconditional read-modify-write is forbidden. A backend reports a failed write
condition as a conflict; the D18P layer then either proves an idempotent retry or
repeats the state transition from a fresh read.

Concrete durability belongs to the injected backend. A conforming backend MUST
make each individual record write atomic. D18P does not append bytes directly to
a raw file, so a process crash cannot expose a partially committed record as a
successful write. Truncated or otherwise corrupt bytes returned by a backend are
rejected fail-closed.

### Namespace and envelope

Every record uses this namespace:

```text
chief-channels
```

D18P writes the empty JSON object `{}` as record metadata. Backend revisions,
creation times, update times, and content hashes are backend envelope data and
are not embedded in the portable record body.

```text
kind        content type
----------  ------------------------------------------------------------------
definition  application/vnd.coding-adventures.chief-channel-definition-v1
state       application/vnd.coding-adventures.chief-channel-state-v1
message     application/vnd.coding-adventures.chief-channel-message-v1
grant       application/vnd.coding-adventures.chief-channel-key-grant-v1
ack         application/vnd.coding-adventures.chief-channel-ack-v1
```

A read MUST verify that the record content type matches the key's logical kind.
An unexpected content type is a corrupt record even when the body could be
decoded as another kind.

### Deterministic keys

Let `channel_hex` be the 32 lowercase hexadecimal digits of the 16 channel-ID
bytes. Let `receiver_hash` be the 64 lowercase hexadecimal digits of
`SHA-256(receiver_id)`. Decimal sequences and epochs are zero-padded to exactly
20 ASCII digits, which preserves unsigned 64-bit numeric order under
lexicographic sorting.

```text
definition  {channel_hex}/definition
state       {channel_hex}/state/next-sequence
message     {channel_hex}/messages/{sequence:020}
grant       {channel_hex}/grants/{key_epoch:020}/{receiver_hash}
ack         {channel_hex}/receivers/{receiver_hash}/ack
```

Raw agent IDs never appear in storage keys. Hashing receiver IDs prevents path
injection and keeps arbitrary identity bytes out of backend logs. Message keys
provide direct random access and ordered range reads without scanning unrelated
records.

---

## `D18H` version 1 reserved header

A pending append persists the exact header that will later become the D18F
message's authenticated data. Its binary envelope is:

```text
offset  field                    encoding
------  -----------------------  -------------------------------
0       magic                    ascii("D18H"), 4 bytes
4       version                  0x01, 1 byte
5       message_id               bytes[16]
21      timestamp_ns             u64be
29      originator_id_length     u32be, maximum 4096
33      originator_id            declared bytes
...     channel_id               bytes[16]
...     sequence                 u64be
...     key_epoch                u64be
...     content_type_length      u32be, maximum 1024
...     content_type             declared UTF-8 bytes
...     plaintext_hash           bytes[32]
EOF     no trailing bytes permitted
```

The header record is structurally compatible with the shipped Rust channel wire
codec. High-level publish validation additionally applies D18F's UUID-v7,
non-empty originator, MIME, and plaintext-hash rules before delivery or trust.

---

## `D18S` version 1 reservation state

```text
offset  field                    encoding
------  -----------------------  -------------------------------
0       magic                    ascii("D18S"), 4 bytes
4       version                  0x01, 1 byte
5       next_sequence            u64be
13      pending_flag             u8: 0 none, 1 header follows
14      if pending_flag = 1:
        header_length            u32be, maximum 16384
18      reserved_header          exact D18H v1 bytes
EOF     no trailing bytes permitted
```

When `pending_flag` is zero, the record length is exactly 14 bytes. When it is
one, the total length is exactly `18 + header_length`.

A pending header MUST name the state record's channel and MUST have
`header.sequence + 1 == next_sequence`. Addition overflow is invalid. Unknown
flags, unsupported versions, malformed embedded headers, inconsistent channel
or sequence values, and trailing bytes are corrupt state.

The initial state is `next_sequence = 0` with no pending header.

---

## `D18A` version 1 receiver cursor

```text
offset  field                    encoding
------  -----------------------  -------------------------------
0       magic                    ascii("D18A"), 4 bytes
4       version                  0x01, 1 byte
5       first_unread_sequence    u64be
EOF                              total length exactly 13 bytes
```

An absent receiver cursor means `first_unread_sequence = 0`. A stored cursor is
the first sequence not covered by a successful acknowledgement, not the last
acknowledged sequence.

---

## Definition creation and destruction

### Create

Creation proceeds as follows:

1. Validate and canonicalize an active immutable definition.
2. Initialize the injected backend.
3. Atomically create the definition record with `if_absent`.
4. If the key already exists, load it. Byte-identical canonical content is an
   idempotent retry; different content is `conflicting_definition`.
5. Atomically create the initial `D18S` state with `if_absent`. If it already
   exists, decode and preserve it.
6. Reload the definition and require the exact canonical value to remain active.

Definition creation and state initialization intentionally use two records. A
crash after step 3 is recoverable: repeating creation proves the existing
definition is identical and completes state initialization. A state record is
never reset merely because initialization is retried.

### Destroy

Destruction reads the definition, changes only `lifecycle` from active to
destroyed, and writes the new canonical bytes with `if_revision`. A conflict
restarts from a fresh read. A byte-identical destroyed definition makes a retry
idempotent.

Implementations make at most 16 CAS attempts for one public operation. Exhausting
that bound returns `concurrent_update`; it must not fall back to an unconditional
write.

No D18P operation may delete or overwrite a message or grant during destruction.
Retention and compaction are outside version 1. A future policy cannot claim
D18P v1 conformance if it silently removes accepted history.

---

## Append protocol

### Phase 1: reserve before encryption

For each publish attempt:

1. Load and validate the active definition and the caller's originator role.
2. Obtain an injected UUID-v7 message ID and timestamp, or accept explicit
   deterministic metadata for retry/recovery.
3. Read and decode the current `D18S` record.
4. If a pending header exists, return `pending_append`; do not reserve another
   sequence.
5. Let `sequence = next_sequence`. Reject if incrementing it would overflow.
6. Compute the D18F plaintext hash and construct the exact `D18H` header.
7. Revision-CAS the state to `next_sequence = sequence + 1` with that pending
   header.
8. On conflict, restart from step 3, for at most 16 total attempts.

No message encryption may occur before step 7 succeeds. D18P v1 permits exactly
one pending append per channel, which serializes sequence reservation without a
multi-record transaction.

### Phase 2: complete idempotently

To complete a reservation:

1. Require the supplied header's channel ID to equal the store's channel ID.
2. Load `D18S` and require its pending header to equal every supplied header
   byte and logical field.
3. Ask the D18F implementation to encrypt/sign the plaintext using that exact
   header. A plaintext whose SHA-256 digest differs from the reserved hash is
   rejected before a record can be accepted.
4. Atomically create the `D18M` body at its deterministic message key.
5. If that key exists, require both the message content type and every body byte
   to be identical. Identical content is an idempotent retry; any difference is
   `conflicting_record`.
6. Revision-CAS `D18S` to the same `next_sequence` with no pending header.

A crash after the message write but before state cleanup is repaired by
repeating completion. A retry after cleanup may also succeed only when the
stored record has the same header and the D18F implementation reproduces the
same complete encrypted bytes. A missing record with no matching pending state
is `no_pending_append`.

### Recovery and abandonment

After restart, initialization returns the exact pending header. Recovery code
may either:

- complete it with plaintext whose hash matches the reservation, or
- explicitly abandon it by revision-CAS clearing the pending header.

Abandonment never decrements `next_sequence`. The abandoned sequence is a
permanent gap. This is required because the sequence is part of the XChaCha20
nonce; reusing it could reuse a nonce under the same channel master key.

---

## Immutable messages and grants

Completed `D18M` messages and `D18G` receiver grants are create-if-absent
records. Saving a byte-identical value at the same deterministic key is
idempotent. A different content type or body at that key is a conflict and must
never replace the existing value.

On load, a message MUST decode to the channel and sequence named by its key. A
grant MUST decode to the channel, epoch, and receiver ID named by its lookup.
Decoded records are not trusted until D18F or #141 cryptographic verification
succeeds.

The grant key contains `SHA-256(receiver_id)`, but the receiver ID remains
inside the authenticated grant. Implementations compare that inner value to the
requested receiver after decoding; the path hash alone is not authorization.

---

## Ordered reads

`read_messages(start, page_size)` requires `page_size > 0` and lists the
channel's message prefix recursively in lexicographic key order.

- For `start = 0`, no cursor is supplied.
- For `start > 0`, the cursor is the deterministic message key for
  `start - 1`, so the first returned key is at least `start`.
- Every result must have the message content type, decode as D18M, name the
  requested channel, have sequence at least `start`, and exactly match its
  deterministic key.
- Returned message sequences must be strictly increasing.
- Missing sequence keys are valid abandoned gaps and are skipped.

If the backend supplies another-page cursor, D18P returns `next_start` equal to
the last returned message sequence plus one. A continuation with an empty page,
or an overflowing continuation sequence, is corrupt storage.

`read_for_receiver(receiver_id, page_size)` begins at that receiver's stored
first-unread cursor. Receiver cursors do not affect one another.

---

## Acknowledgement protocol

At the store layer, acknowledging sequence `N` requests cursor `N + 1`.

1. Validate the receiver ID and load `D18S`.
2. Require `N < next_sequence`; otherwise return `acknowledgement_ahead`.
3. If a pending sequence exists, require `N < pending.sequence`; otherwise
   return `acknowledgement_pending`.
4. Reject `N + 1` overflow.
5. If the cursor record is absent, atomically create `N + 1`.
6. If it exists, reject `N + 1 < current` as
   `acknowledgement_regression`; treat equality as an idempotent retry; otherwise
   revision-CAS the larger cursor.
7. Retry conflicts from a fresh read for at most 16 attempts.

The low-level store may advance across an abandoned sequence because it knows
only durable sequence state. The authorized receiver API is stricter: it keeps
a session-local map from delivered D18F message ID to sequence and accepts an
acknowledgement only for a message delivered by that receiver instance. An
unknown message ID is rejected. After restart, the receiver reads again from
its durable cursor before acknowledging.

Acknowledging message `N` covers all earlier sequences. The API does not model
sparse per-message acknowledgements.

---

## Structural endpoint roles

Portable packages expose idiomatic equivalents of two separate interfaces:

```text
Originator
  id() -> AgentId
  channel_id() -> ChannelId
  public_key() -> Ed25519PublicKey
  publish(payload, content_type) -> PublishedMessage

Receiver
  id() -> AgentId
  channel_id() -> ChannelId
  public_key() -> X25519PublicKey
  receive(limit) -> [ReceivedMessage]
  acknowledge(message_id) -> first_unread_sequence
```

There is no receiver write method and no originator read method. An entity may
implement both roles only on different channel definitions.

Opening an originator requires the exact definition originator ID and a signing
key whose public key equals the durable Ed25519 key. Opening a receiver requires
membership and a private key whose public key equals the durable X25519 key.

Every privileged endpoint operation reloads the definition, requires it to be
active, and requires it to equal the definition captured when the endpoint was
opened. This prevents a cached endpoint from surviving membership or epoch
changes.

Before receiver delivery, implementations require each message to name the
durable channel and originator and not exceed the definition epoch. They obtain
the exact receiver grant for the message epoch, verify/open it under #141, and
fully verify/decrypt the D18F message. No plaintext is returned before every
membership and cryptographic check succeeds.

Clocks, UUID generation, entropy, storage, and crypto are injected boundaries.
Portable tests use deterministic sources and an in-memory atomic backend. Native
production callers may supply OS-backed storage and secret providers without
changing D18P behavior.

---

## Stable portable failure codes

Language-specific exception or result types are idiomatic. Conformance fixtures
compare these exact machine-readable codes, never localized messages:

| Code | Meaning |
| --- | --- |
| `invalid_definition` | Static channel membership, UUID, bound, or lifecycle rule failed |
| `invalid_message_id` | Message ID is not UUID v7 with the RFC variant |
| `definition_not_found` | No definition exists at the channel key |
| `conflicting_definition` | Create retry found different definition bytes |
| `corrupt_definition` | Persisted definition envelope/body/invariant is invalid |
| `definition_changed` | Cached endpoint no longer matches durable definition |
| `channel_destroyed` | Operation requires an active definition |
| `unauthorized_originator` | Caller is not the single originator |
| `unauthorized_receiver` | Caller is not an authorized receiver |
| `public_key_mismatch` | Supplied private/signing key does not match durable public key |
| `missing_key_grant` | Receiver lacks the required epoch grant |
| `unknown_message_id` | Receiver tried to acknowledge an undelivered message ID |
| `unauthorized_message` | Message fields violate durable membership or epoch |
| `not_initialized` | Durable sequence state is absent |
| `corrupt_record` | Stored state/message/grant/ack violates its envelope or key |
| `pending_append` | Another reservation must be completed or abandoned first |
| `no_pending_append` | Completion has neither matching pending state nor stored message |
| `pending_header_mismatch` | Completion differs from the durable reservation |
| `conflicting_record` | Immutable message or grant key contains different bytes |
| `concurrent_update` | Sixteen CAS attempts could not complete |
| `invalid_receiver_id` | Receiver ID is empty or exceeds 4096 bytes |
| `invalid_page_size` | Ordered read limit is zero |
| `acknowledgement_regression` | Requested first-unread cursor is below current |
| `acknowledgement_ahead` | Acknowledgement names an unreserved sequence |
| `acknowledgement_pending` | Acknowledgement would cover an unfinished reservation |
| `sequence_exhausted` | A required sequence increment exceeds `u64` |
| `storage_error` | Injected backend failed outside an expected conflict |
| `wire_error` | D18H, D18M, or D18G structural codec rejected bytes |
| `crypto_error` | D18F or #141 cryptographic processing failed |
| `metadata_error` | Injected message metadata source failed |

Implementations MAY attach structured fields such as current/attempted sequence,
epoch, or field name. They MUST NOT include plaintext, CMKs, private keys,
unwrapped grants, or other secrets in errors, logs, audit records, or fixtures.

Validation order should be deterministic enough for malformed fixture inputs to
produce one code. Structural size/truncation failures precede membership and
cryptographic checks; authorization precedes plaintext delivery; storage
conflicts are resolved or classified before any unconditional retry.

---

## Security and availability properties

A conforming implementation preserves all of these properties:

1. **One-way authority.** Only the definition originator can publish; receivers
   can only read and acknowledge.
2. **Nonce safety.** A sequence is durably consumed before D18F encryption and
   is never reused after failure, abandonment, rotation, or restart.
3. **Append-only history.** Accepted message and grant bytes are never modified
   or deleted by D18P v1.
4. **Crash convergence.** Repeating create, completion, destruction, and
   acknowledgement after a crash is either byte-identical/idempotent or fails
   closed.
5. **Independent progress.** Each receiver has a separate monotonic cursor.
6. **Opaque routing.** Storage and orchestration do not require plaintext or a
   channel master key.
7. **Bounded work.** Identity counts/sizes, header size, and CAS retries are
   bounded before allocation or repetition. The portable API rejects a zero
   page size; deployments SHOULD also impose a finite caller-level maximum that
   fits their memory budget.
8. **Corruption visibility.** Key/body mismatches, partial bodies, invalid
   versions, unexpected content types, and ordering violations are errors; they
   are never silently skipped as if acknowledged.

---

## Required conformance corpus

The shared D18P fixture manifest is generated from the Rust compatibility
baseline at `code/fixtures/chief-of-staff-channel/v1/manifest.json` and is
consumed unchanged by every language lane. The completed six-language corpus
must cover at least:

- canonical active and destroyed D18C records, unsorted receiver input, binary
  agent IDs, maximum bounds, and membership failures
- initial D18S, a pending D18H reservation, overflow, mismatched channel and
  sequence, invalid flags, truncation, and trailing bytes
- absent/zero and advanced D18A cursors plus invalid length/version
- every deterministic key and content type, including binary receiver IDs
- atomic create races and conflicting definitions
- reserve, recover, complete, completion retry, write-before-cleanup recovery,
  abandon, and never-reused sequence gaps
- ordered multi-page reads, random access, key/body mismatch, wrong content
  type, corrupt record, and empty continuation
- two independent receiver cursors, idempotent ack, regression, ahead, pending,
  conflict retry, and session-delivery enforcement
- active-to-destroyed CAS, idempotent destruction, and rejection of all endpoint
  operations after destruction
- exact stable failure codes with no secret material

The corpus contains only deterministic public keys, test-only symmetric/private
keys where #141 fixtures require them, opaque ciphertext, and non-secret sample
plaintext. Test-only keys must be labeled and must never be accepted as
production defaults.

---

## Six-language rollout and completion gate

Issue #131 is complete only after this sequence:

1. **Complete:** merge this normative D18P profile (#11685).
2. **Complete:** the shared fixture manifest and Rust adapter prove the profile
   is byte- and behavior-compatible with the shipped production implementation
   (#11691).
3. **Complete:** the deterministic definition/store/role kernel is implemented
   in TypeScript (#11700), Python (#11705), Go (#11709), Ruby (#11712), and
   Elixir (#11716), using idiomatic immutable values and injected atomic
   storage/crypto sources.
4. **Complete:** the central repository check requires exactly those six
   consumers, runs every package-native build, verifies fixture-generator
   provenance, and regenerates the manifest byte-for-byte (#11721).
5. **Complete:** the exact-head gate and required Ubuntu, macOS, and Windows
   builds passed on #11721, satisfying the close condition for #131.

[D18Q](D18Q-chief-of-staff-channel-key-grant-profile.md) reuses D18P's
immutable grant slots and endpoint boundary for portable sealed-key generation,
opening, receiver epoch state, and cryptographic rotation. D18P version 1 does
not activate a new epoch durably. [D18T](D18T-chief-of-staff-durable-epoch-activation-profile.md)
defines the version 2 D18S upgrade, originator-key custody, immutable activation
plan, crash recovery, and active-epoch CAS tracked by #11734; saving grants alone
does not imply activation. Issue #133 may reuse the structural role interfaces
while adding the wider SKILL, function, program, stdin/stdout, and bridge
developer surfaces.

---

## Repository mapping

The Rust compatibility sources are:

```text
code/packages/rust/chief-of-staff-channel-crypto/src/wire.rs
code/packages/rust/chief-of-staff-channel-store/src/lib.rs
code/packages/rust/chief-of-staff-channel-store/src/profile.rs
code/packages/rust/chief-of-staff-channel-endpoints/src/lib.rs
code/packages/rust/chief-of-staff-channel-endpoints/src/profile.rs
code/packages/rust/chief-of-staff-channel-endpoints/tests/d18p_fixtures.rs
code/packages/rust/storage-core/src/lib.rs
code/fixtures/chief-of-staff-channel/v1/manifest.json
```

D18F remains normative for the encrypted message:

```text
code/specs/D18F-chief-of-staff-message-profile.md
```

The high-level architecture and encrypted-channel cryptography remain in:

```text
code/specs/D18-chief-of-staff.md
```
