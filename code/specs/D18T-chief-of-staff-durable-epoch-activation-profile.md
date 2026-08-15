# D18T - Chief of Staff Durable Channel Epoch Activation Profile

Status: normative portability profile tracked by #11781 for issue #11734

D18P makes channel messages, grants, cursors, and sequence reservations durable.
D18Q produces a complete next-epoch CMK and receiver-grant plan. Neither profile
alone can make that epoch current without racing publication or losing key
material across a crash. D18T defines that missing transaction without assuming
multi-record storage transactions.

D18T has three authorities:

- the D18P channel store owns public records and the publish-reservation CAS;
- an injected originator key-custody implementation atomically owns prepared
  CMKs and their recovery bundles;
- D18Q owns grant creation, parsing, signature verification, and key material.

The active epoch is deliberately stored in the same versioned state record as
the pending publish reservation. A separate mutable epoch head is not
conforming: two independent CAS operations cannot exclude a new old-epoch
reservation while activation is committing.

---

## Scope

D18T fixes:

- the `D18S` version 2 reservation and active-epoch state record;
- the immutable `D18T` version 1 activation-plan record;
- deterministic keys, content types, encodings, commitments, and ordering;
- the atomic injected originator-key custody contract;
- bootstrap custody for the already-active CMK;
- migration from D18P `D18S` version 1;
- prepare, public-record replay, activation, publication, retry, crash recovery,
  concurrency, destruction, and bounded-CAS behavior;
- stable errors, secret-safe diagnostics, and the shared fixture obligations.

D18T does not define:

- receiver authorization policy or operator approval;
- D18G cryptography, which remains D18Q-owned;
- D18M encryption, which remains D18F-owned;
- a concrete Vault, hardware key, HSM, or operating-system keychain;
- retrospective revocation of plaintext or old CMKs already received;
- compaction or deletion of public channel history.

---

## Normative vocabulary

The words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are normative.

All integers are unsigned. `u8`, `u32be`, and `u64be` are fixed-width unsigned
integers, with the latter two encoded in big-endian byte order. Lengths count
octets. `bytes[n]` means exactly `n` octets.

An **activation preparation** is one atomically durable custody entry containing
the secret new CMK and the exact public recovery bundle for one candidate
epoch. A **plan commitment** is `SHA-256(D18T_plan_bytes)`. A **grant
commitment** is `SHA-256(D18G_bytes)`.

An epoch is **active** only when its number appears as `active_epoch` in the
canonical `D18S` version 2 state at the channel state key. A prepared or
publicly persisted plan is not active.

---

## Required composition

An implementation MUST compose these existing contracts rather than creating
parallel wire formats:

- D18P `ChannelDefinition`, `D18H`, message/grant keys, immutable writes,
  revision CAS, destruction, and maximum 16-attempt rule;
- D18Q `RotationPlan`, exact D18G bytes, canonical receiver ordering, stable
  grant validation, and secret-erasure capability reporting;
- `storage-core`-equivalent point reads, atomic create-if-absent, and revision
  CAS through an injected backend.

The immutable channel definition's `key_epoch` becomes the initial epoch. Once
the channel state has been migrated to `D18S` version 2, `active_epoch` in that
state is the only authority for a new publish. The immutable definition remains
the identity, original membership, key, creation-time, and lifecycle authority.

---

## `D18S` version 2 reservation and epoch state

```text
offset  field                    encoding
------  -----------------------  -------------------------------
0       magic                    ascii("D18S"), 4 bytes
4       version                  0x02, 1 byte
5       active_epoch             u64be
13      next_sequence            u64be
21      pending_flag             u8: 0 none, 1 header follows
22      if pending_flag = 1:
        header_length            u32be, maximum 16384
26      reserved_header          exact D18H v1 bytes
EOF     no trailing bytes permitted
```

When `pending_flag` is zero, the record length is exactly 22 octets. When it is
one, the total length is exactly `26 + header_length`.

All D18P state invariants remain. A pending header MUST name the state record's
channel, MUST satisfy `header.sequence + 1 == next_sequence` without overflow,
and MUST have `header.key_epoch == active_epoch`. Unknown flags, other versions,
oversized or malformed headers, inconsistent fields, and trailing bytes are
`corrupt_record`.

The deterministic key remains:

```text
{channel_hex}/state/next-sequence
```

The version 2 content type is:

```text
application/vnd.coding-adventures.chief-channel-state-v2
```

A D18T implementation MUST reject a version/content-type mismatch even when
the body could otherwise be decoded.

### Migration from D18P version 1

Migration receives either an opaque custody handle for the definition's
already-active CMK or the exact caller-owned CMK that the version 1 channel was
already using. Before changing public state, it MUST use the custody operation
defined below to prove that `(channel_id, definition.key_epoch)` is durably
resolvable. It MUST NOT generate or substitute a CMK.

Migration then reads the immutable definition and the existing state record
under one bounded retry loop.

1. If no state exists, atomically create version 2 with the definition's
   `key_epoch`, sequence zero, and no pending header.
2. If version 1 exists, decode it under D18P. If it has a pending header, that
   header's key epoch MUST equal the definition's initial epoch.
3. Revision-CAS the exact sequence and pending header into version 2, adding
   `active_epoch = definition.key_epoch`.
4. On CAS conflict, restart from a fresh state read. At most 16 attempts are
   permitted.
5. An already valid version 2 state is an idempotent success only after custody
   proves that its active epoch is resolvable. Its active epoch MUST NOT be
   reset from the immutable definition.

Migration never clears a pending publish, resets a sequence, changes a
definition, or generates key material. Failure to prove current-key custody is
`active_key_missing` and leaves the public state unchanged.

Publishing D18S version 2 is the rolling-upgrade boundary. A version 1 process
will reject that record rather than write it, so operators MUST deploy
D18T-aware readers and writers before migrating a channel. No implementation
may decode version 2 as version 1 or downgrade a version 2 record.

---

## `D18T` version 1 immutable activation plan

```text
offset  field                    encoding
------  -----------------------  -------------------------------
0       magic                    ascii("D18T"), 4 bytes
4       version                  0x01, 1 byte
5       channel_id               bytes[16], UUID v7
21      base_epoch               u64be
29      new_epoch                u64be
37      receiver_count           u32be, 1 through 1024
41      receivers, repeated receiver_count times:
        receiver_id_hash         bytes[32]
        grant_hash               bytes[32]
EOF     no trailing bytes permitted
```

`new_epoch` MUST equal `base_epoch + 1` without overflow. Receiver entries MUST
be strictly sorted by `receiver_id_hash`, with no duplicate hash or grant
commitment. `receiver_id_hash` is `SHA-256(receiver_id)` and `grant_hash` is
`SHA-256(exact_D18G_bytes)`.

The plan does not contain a CMK, private key, shared secret, wrapping key,
nonce, raw receiver ID, plaintext, or custody locator. The exact D18G bytes
commit to the raw originator and receiver IDs, channel, epoch, wrapped CMK, and
originator signature.

The plan commitment is the 32-octet SHA-256 of the complete canonical D18T
record. It is rendered as 64 lowercase hexadecimal digits only when a text
representation is required.

### Storage location

```text
key          {channel_hex}/epochs/{new_epoch:020}/activation
content type application/vnd.coding-adventures.chief-channel-epoch-activation-v1
metadata     {}
body         exact D18T v1 bytes
```

The plan is written with atomic create-if-absent. Byte-identical content is an
idempotent retry. Different content, content type, or metadata at the same key
is `conflicting_plan` and MUST NOT be replaced.

Accepted plan and grant records are append-only. Rotation and logical channel
destruction never overwrite or delete them.

---

## Plan validation

Before a preparation can be offered to custody, the implementation MUST:

1. load the exact active definition and require an authorized originator;
2. load version 2 state and require no pending publish;
3. require `new_epoch == active_epoch + 1` without overflow;
4. require a non-empty trusted D18Q `RotationPlan` with at most 1024 grants;
5. serialize every grant as canonical D18G and require the same originator,
   channel, and new epoch in every grant;
6. verify every D18G originator signature using the definition's Ed25519 public
   key without requiring a receiver private key;
7. require every receiver to be authorized by the caller-supplied target
   roster, require exact agreement between that roster and the grant receiver
   IDs, and reject duplicate receivers;
8. sort by raw receiver ID for D18Q behavior, derive receiver/grant commitments,
   sort the public plan entries by receiver hash, and encode canonical D18T;
9. preserve the D18Q plan's single secret CMK in a redacted secret container.

Hash collisions are not treated as equal authorization. If distinct receiver
IDs produce the same receiver hash, validation returns `invalid_plan`.

Receiver public keys are inputs to D18Q sealing, not D18G fields. A D18T API
therefore MUST either construct the D18Q plan internally from the exact target
roster or accept only the opaque trusted `RotationPlan` returned by D18Q beside
that roster. It MUST NOT accept caller-assembled grant bytes as proof that a
receiver public key was used.

---

## Atomic originator-key custody contract

Every implementation depends on an injected custody interface. A production
implementation MUST be durable across process and machine restart and MUST
provide these operations:

```text
import_active_if_absent(channel_id, epoch, cmk) -> selected | idempotent | conflict
resolve_handle(channel_id, epoch) -> redacted handle | absent
prepare_if_absent(channel_id, new_epoch, complete bundle)
  -> selected | idempotent | conflict
load_preparation(channel_id, new_epoch) -> complete recovery bundle | absent
```

`import_active_if_absent` is used only when creating a D18T-aware channel or
upgrading a version 1 channel whose current CMK already exists. It atomically
owns a copy of exactly 32 CMK octets under `(channel_id, epoch)`. A conflict is
`conflicting_active_key`, is fail-closed, and MUST NOT reveal whether the stored
secret differs. The caller MUST erase or release its CMK according to the
implementation's D18Q erasure capability after custody reports `selected` or
`idempotent`.

`resolve_handle` never returns key bytes. A production channel may expose D18S
version 2 only while the handle for its `active_epoch` is resolvable. New
channel creation therefore imports the initial CMK before atomically creating
its initial version 2 state. Version 1 migration imports or resolves the
definition epoch before its state CAS. A crash after custody import but before
state creation/migration is an idempotent retry and leaves the old public epoch
authoritative.

`prepare_if_absent` is keyed by `(channel_id, new_epoch)` and atomically owns
the following indivisible bundle:

```text
PreparedEpoch
|-- channel_id               bytes[16]
|-- base_epoch               u64
|-- new_epoch                u64
|-- plan_bytes               exact D18T v1 bytes
|-- grants                   exact D18G bytes in D18Q receiver order
`-- channel_master_key       secret bytes[32]
```

The custody operation has exactly three classifications:

- `selected`: the key was absent and the complete bundle became durable;
- `idempotent`: every public byte and the secret CMK equal the durable bundle;
- `conflict`: another bundle already owns the `(channel_id, new_epoch)` slot.

`conflict` maps to `conflicting_preparation` and is not a successful result.
The operation MUST NOT expose which individual field differed. It MUST NOT
partially store a bundle, replace a winner, or compare only the plan
commitment. Every custody comparison of CMK bytes, including active-key import,
MUST use the platform's constant-time primitive where one exists.

After restart, custody MUST return the exact public plan/grant recovery bundle
and resolve an opaque redacted key handle for every retained epoch. Only the
originator encryption boundary may resolve that handle internally to a CMK.
Listing, debug formatting, errors, audit records, and public storage MUST never
contain the CMK or a reversible custody locator.

The in-memory custody used by deterministic tests MUST report itself as
non-durable and MUST NOT be accepted by a production constructor.

---

## Prepare and replay protocol

Preparing a rotation follows these phases in order:

1. validate the D18Q plan and construct the canonical D18T plan;
2. atomically call custody `prepare_if_absent` with the entire bundle;
3. reload the selected bundle from custody; never continue from caller-owned
   mutable inputs;
4. atomically create or byte-verify the immutable D18T plan record;
5. atomically create or byte-verify every D18G grant at its D18P deterministic
   key;
6. reload and validate the plan and all grants from public storage;
7. return `prepared`; do not change the active epoch.

Custody is first because it is the only operation that both selects a
candidate and makes all information needed for replay durable atomically. A
crash before phase 2 has no selected candidate. A crash after phase 2 is
recoverable using only custody's durable recovery bundle and the public store.

Recovery calls the same replay phases 3 through 6. It does not generate a CMK,
reseal a grant, accept replacement bytes, or choose another candidate.

If the definition became destroyed, recovery MUST NOT activate. Public records
already accepted remain append-only. Custody applies its configured logical
destruction/erasure policy to the secret handle.

---

## Activation protocol

Activation is a bounded operation:

1. load and validate the active definition;
2. load version 2 state and the custody-selected preparation for the caller's
   requested `new_epoch`;
3. if `active_epoch == new_epoch`, replay and byte-verify that preparation and
   return `idempotent`; if it is greater, return `decreasing_epoch`;
4. require `new_epoch == active_epoch + 1` without overflow and require the
   preparation's base/new epochs to equal those state epochs;
5. replay and byte-verify the immutable plan and every selected grant;
6. require custody to confirm that the opaque CMK handle is available;
7. require the state to have no pending header;
8. revision-CAS the state to identical `next_sequence` and no pending header,
   changing only `active_epoch` from `E` to `E + 1`;
9. on CAS conflict, restart from step 1 for at most 16 attempts.

A missing requested preparation is `preparation_missing`. A selected
preparation whose base/new epoch does not match the state is
`unexpected_epoch`.

No activation may skip a global epoch. Receivers may still skip epochs for
which they were not authorized, as D18Q specifies.

### Serialization with publication

A D18T-aware publisher MUST obtain the key epoch from `D18S` version 2 while
constructing its D18H reservation and MUST resolve that exact epoch's redacted
custody handle before reserving. A missing handle is `active_key_missing`; it
MUST NOT mutate state. A caller-supplied epoch is either absent or must exactly
equal `active_epoch`; otherwise the request fails with `unactivated_epoch`
before encryption.

The publish reservation and activation both revision-CAS the same D18S record:

- if publication wins, activation observes the pending header and returns
  `pending_append` until it is completed or explicitly abandoned;
- if activation wins, the publication CAS conflicts, reloads state, and builds
  a new reservation using E+1 and its custody-resolved CMK.

Encryption never falls back to E, invents a missing E+1 CMK, or accepts a
prepared-but-unactivated epoch.

---

## Crash and concurrency outcomes

| Last durable phase | Required recovery result |
| --- | --- |
| Before custody selection | E remains current; a later candidate may compete |
| Custody selected only | E remains current; replay the exact selected bundle |
| Plan stored, grants partial | E remains current; byte-verify and finish grants |
| Plan and grants stored | E remains current; activation may resume |
| D18S activation CAS committed | E+1 is current; retry is idempotent |
| Publish reservation CAS won | E remains current until completion/abandonment |
| Channel destroyed at any point | no activation; retain public history |

Concurrent candidates call the same atomic custody slot. Exactly one receives
`selected`; its byte-identical retries receive `idempotent`; every other
candidate receives `conflicting_preparation`. A loser MUST NOT write its plan
or grants to public storage.

A storage conflict after custody selection cannot choose another plan. It is
resolved only by byte-identical replay or a stable conflict/corruption error.

---

## Revocation and history

For rotation from authorized receivers A+B at E to B only at E+1:

- the initial definition or E activation plan, plus the A/B grants, remain
  immutable;
- A receives no E+1 grant and keeps any E key already installed;
- B's exact E+1 grant is durable before activation and B may retain E and E+1;
- messages retain their original key epochs and are never re-encrypted;
- current authorization is the initial definition's roster until the first
  activation, then the receiver roster committed by the active plan and its
  grants; every older definition/plan/grant remains append-only authorization
  history.

This is prospective revocation. It cannot erase plaintext or an old CMK from a
receiver that already obtained it.

---

## Stable error codes

Implementations MUST expose these exact portable codes:

| Code | Meaning |
| --- | --- |
| `not_initialized` | no valid D18S state exists |
| `channel_destroyed` | the immutable definition is destroyed |
| `invalid_plan` | the candidate violates D18T/D18Q or authorization invariants |
| `corrupt_record` | a stored state, plan, grant, key, content type, or body is invalid |
| `pending_append` | a D18H reservation prevents activation |
| `unactivated_epoch` | publication named an epoch other than active_epoch |
| `active_key_missing` | custody cannot resolve the CMK for the public active epoch |
| `conflicting_active_key` | custody already owns different bytes for an imported active epoch |
| `preparation_missing` | custody has no selected bundle for the required epoch |
| `conflicting_preparation` | another custody bundle owns the epoch slot |
| `conflicting_plan` | the immutable activation-plan slot contains different bytes |
| `conflicting_grant` | an immutable receiver-grant slot contains different bytes |
| `unexpected_epoch` | a selected plan is not exactly active_epoch + 1 |
| `decreasing_epoch` | activation or recovery targeted an older epoch |
| `epoch_exhausted` | active_epoch is u64::MAX |
| `concurrent_update` | 16 state CAS attempts did not converge |
| `storage_error` | the injected public backend failed |
| `custody_error` | the injected secret-custody backend failed |
| `crypto_error` | D18Q validation or D18F encryption failed |

Errors may include public channel/epoch/plan commitments where useful, but MUST
NOT include CMKs, private keys, shared secrets, wrapping keys, nonces,
plaintext, raw custody locators, or complete D18G bodies.

---

## Required public API semantics

Names may be idiomatic, but every implementation MUST provide equivalents of:

```text
migrate_epoch_state(definition, current_cmk_or_handle, custody, store)
  -> D18S-v2 state
create_epoch_channel(definition, initial_cmk, custody, store)
  -> D18S-v2 state
prepare_rotation(definition, d18q_rotation_plan, custody, store)
  -> prepared | idempotent
recover_preparation(definition, new_epoch, custody, store)
  -> prepared | idempotent
activate_prepared_epoch(definition, new_epoch, custody, store)
  -> activated | idempotent
reserve_publish_using_active_epoch(request_without_epoch, custody, store)
  -> exact D18H reservation plus redacted CMK handle
activation_plan(new_epoch) -> immutable public D18T plan or absent
```

Constructors MUST own mutable inputs. Public plan/state values and custody
handles MUST be immutable according to the language's conventions.

---

## Shared fixture and conformance obligations

The Rust reference adapter owns the deterministic version 1 manifest under:

```text
code/fixtures/chief-of-staff-channel-epoch-activation/v1/
```

The manifest MUST record the generator Git blob SHA-1 and contain:

- exact D18S v1-to-v2 migration bytes, including a pending D18H case;
- initial-key custody import, missing-active-key, and import-before-state crash
  traces;
- exact D18T plan bytes, plan commitment, deterministic key/content type, and
  receiver/grant commitments;
- a clearly labeled test-only D18Q A+B to B-only preparation;
- replay traces after custody, plan, first grant, all grants, and activation;
- both outcomes of the activation-versus-publication CAS race;
- byte-identical retry and conflicting-candidate traces;
- pending, corrupt, missing-custody, destroyed, exhaustion, and CAS-limit
  failures with stable codes;
- proof that A retains only E while B retains E and E+1;
- secret-erasure capability reporting for every language.

Deterministic CMKs and private material are permitted only as clearly labeled
test inputs. Fixture summaries, expected errors, public records, and audit
events MUST remain secret-free.

Rust, TypeScript, Python, Go, Ruby, and Elixir MUST consume the same manifest
directly, reproduce every byte and transition, and run through their native
package `BUILD` entry points. One aggregate validator MUST require exactly
those six consumers, verify generator provenance, regenerate the manifest
byte-for-byte, and feed both push and pull-request CI gates.

---

## Security invariants

1. **One selected candidate.** Atomic custody selection prevents two E+1 plans
   from becoming durable winners.
2. **Custody before visibility.** The CMK and complete recovery bundle are
   durable before any public write or active-epoch CAS.
3. **All grants before visibility.** Every selected exact D18G record is durable
   and reloaded before activation.
4. **One CAS boundary.** Publication reservation and activation contend on the
   same D18S revision.
5. **No fallback.** A missing active key is an error, never permission to use an
   old or generated key.
6. **Append-only history.** Definitions, plans, grants, and messages remain
   immutable through rotation and destruction.
7. **Secret-safe surfaces.** Public storage, metadata, logs, errors, debug
   output, audit records, and fixture summaries reveal no production secret.
8. **Bounded progress.** Public operations stop after 16 CAS attempts and
   report `concurrent_update` rather than writing unconditionally.
