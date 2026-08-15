# D18Q - Chief of Staff Portable Channel-Key Grant Profile

## Overview

D18 channel messages use one shared 256-bit channel master key (CMK) per key
epoch. Every authorized receiver obtains that CMK through a receiver-bound,
originator-signed sealed grant. This profile makes the production Rust grant
and receiver-epoch behavior portable across Python, TypeScript, Go, Ruby, Rust,
and Elixir without creating a second cryptographic protocol.

The compatibility baseline is the existing Rust
`chief-of-staff-channel-crypto` implementation:

- the `D18G` version 1 sealed-grant record
- X25519 ephemeral-static key agreement
- HKDF-SHA256 receiver-specific wrapping-key derivation
- XChaCha20-Poly1305 CMK wrapping
- Ed25519 binding of every logical grant field
- byte-identical retry and monotonic receiver epoch state

`D18Q` is the profile name. It does not replace the `D18G` record magic. D18F
continues to own encrypted message bytes and D18P continues to own durable
channel storage, membership, grants, and acknowledgements.

---

## Scope and ownership

This profile owns:

- channel master key and receiver X25519 key value contracts
- deterministic and production entropy boundaries
- the exact key-wrap derivation, grant AAD, and signature input
- the `D18G` version 1 binary record
- portable seal, structural decode, open, and stable error behavior
- receiver-side epoch installation, retry, conflict, and history semantics
- the pure cryptographic plan for rotating one CMK to a new epoch
- secret-erasure capability reporting

This profile does not own:

- D18F message construction, verification, or nonce reservation
- D18P storage keys, atomic writes, durable membership, or acknowledgements
- authorization policy that decides which receivers remain after rotation
- atomic activation of a new durable epoch or crash-safe originator key custody
- process supervision, actor routing, or orchestrator transport
- physical memory guarantees that a managed runtime cannot provide

A D18Q value is not trusted merely because its `D18G` structure decoded.
Opening the grant must verify its expected identities, channel, signature, key
agreement, derivation, and AEAD authentication in the order defined below.

---

## Normative vocabulary

The words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are normative.

All integers are unsigned. `u8`, `u32be`, and `u64be` mean fixed-width unsigned
integers, with the latter two encoded in big-endian byte order. `bytes[n]`
means exactly `n` octets. Lengths count octets, not characters.

`frame(fields...)` means concatenation of each field preceded by its `u64be`
octet length. It has no field count, terminator, padding, or alignment bytes.

An **epoch grant** is one `D18G` record for one receiver and one channel epoch.
A **rotation plan** is a pure result containing a new CMK and one epoch grant
for every receiver selected by the caller. It does not itself make the epoch
durable or current.

---

## Immutable values and bounds

```text
ChannelMasterKey
`-- bytes                   bytes[32], secret

ReceiverKeyPair
|-- private_key             bytes[32], secret
`-- public_key              bytes[32]

OriginatorSigningKey
|-- secret_key              Ed25519 secret material, secret
`-- public_key              bytes[32]

SealedChannelKeyGrant
|-- originator_id           bytes[1..4096]
|-- receiver_id             bytes[1..4096]
|-- channel_id              bytes[16], UUID v7
|-- key_epoch               u64
|-- ephemeral_public_key    bytes[32]
|-- wrapping_nonce          bytes[24]
|-- wrapped_cmk             bytes[48]
`-- originator_signature    bytes[64]
```

`wrapped_cmk` is exactly 32 ciphertext octets followed by the 16-octet
Poly1305 tag. The tag is not stored in a separate field.

High-level constructors MUST require non-empty originator and receiver IDs of
at most 4,096 octets and a channel ID with RFC 9562 UUID version 7 and RFC
variant bits. The structural `D18G` decoder preserves the shipped version 1
behavior and MAY decode empty IDs; high-level validation rejects them before
cryptographic trust or installation.

Every public grant and public-key value MUST be immutable by the language's
normal conventions. Constructors and decoders MUST own or copy mutable inputs.
Accessors MUST return copies or read-only views. A public grant MUST NOT retain
a CMK, private X25519 key, shared secret, wrapping key, or signing secret.

---

## Entropy and explicit-material APIs

Production convenience APIs obtain these independent values from a
cryptographically secure random source:

- 32 octets for a new CMK
- 32 octets for a receiver private key
- 32 octets for each ephemeral X25519 private key
- 24 octets for each wrapping nonce
- 32 octets for an Ed25519 signing seed when a new signing identity is created

A request that cannot be filled completely is `randomness_unavailable`; it
MUST NOT return a partial key, reuse prior bytes, fall back to a clock or
pseudorandom non-cryptographic source, or emit a grant.

Portable packages MUST also expose deterministic seams equivalent to:

```text
channel_master_key_from_bytes(cmk[32]) -> ChannelMasterKey
receiver_key_pair_from_private_key(private[32]) -> ReceiverKeyPair
originator_signing_key_from_seed(seed[32]) -> OriginatorSigningKey

seal_channel_key_with_material(
  originator_id,
  receiver_id,
  channel_id,
  key_epoch,
  channel_master_key,
  receiver_public_key[32],
  originator_signing_key,
  ephemeral_private_key[32],
  wrapping_nonce[24]
) -> SealedChannelKeyGrant
```

The explicit-material seal operation exists for deterministic fixtures,
recovery-safe prepared operations, and hosts with injected key custody. It is
not a weaker algorithm. It performs the same validation, derivation,
encryption, and signing steps as the production convenience operation.

Tests MUST use explicit material. Checked-in fixture keys and nonces MUST be
marked test-only and MUST NOT be accepted as production defaults.

---

## `D18G` version 1 record

```text
offset  field                              encoding
------  ---------------------------------  -------------------------------
0       magic                              ascii("D18G"), 4 bytes
4       version                            0x01, 1 byte
5       originator_id_length               u32be, maximum 4096
9       originator_id                      declared bytes
...     receiver_id_length                 u32be, maximum 4096
...     receiver_id                        declared bytes
...     channel_id                         bytes[16]
...     key_epoch                          u64be
...     ephemeral_public_key               bytes[32]
...     wrapping_nonce                     bytes[24]
...     wrapped_cmk                        bytes[48]
...     originator_signature               bytes[64]
EOF     no trailing bytes permitted
```

The structural decoder MUST reject the wrong magic, versions other than 1,
truncation, a declared identity length above 4,096 before allocating that
field, arithmetic overflow, and trailing bytes. It MUST consume exactly the
record above and MUST NOT infer trust from successful decoding.

The encoder MUST reject identity fields above 4,096 octets. A high-level
encoder additionally validates non-empty IDs and UUID-v7 channel identity.

---

## Canonical derivation and authentication inputs

The two ASCII contexts are:

```text
chief-channel-key-wrap-v1
chief-channel-key-grant-v1
```

Let `epoch_bytes = u64be(key_epoch)`.

```text
hkdf_salt = frame(
  channel_id,
  epoch_bytes
)

hkdf_info = frame(
  ascii("chief-channel-key-wrap-v1"),
  receiver_id
)

grant_aad = frame(
  ascii("chief-channel-key-grant-v1"),
  originator_id,
  channel_id,
  epoch_bytes,
  receiver_id,
  ephemeral_public_key
)

grant_signature_input = frame(
  ascii("chief-channel-key-grant-v1"),
  originator_id,
  channel_id,
  epoch_bytes,
  receiver_id,
  ephemeral_public_key,
  wrapping_nonce,
  wrapped_cmk
)
```

The order above is normative. It intentionally differs from the physical
`D18G` field order, where `receiver_id` precedes `channel_id`. Raw
concatenation, integer text, platform-native integers, UUID text, Unicode
normalization, JSON, and hashing a field instead of framing it are forbidden.

---

## Grant sealing algorithm

Inputs are the expected high-level fields, one 32-octet CMK, a receiver X25519
public key, an originator Ed25519 signing key, one ephemeral X25519 private key,
and one wrapping nonce.

1. Validate identity lengths and the channel UUID-v7 bits.
2. Derive `ephemeral_public_key = X25519.public(ephemeral_private_key)`.
3. Compute `shared_secret = X25519(ephemeral_private_key,
   receiver_public_key)` and reject a low-order input or all-zero result as
   `invalid_key_agreement`.
4. Derive exactly 32 wrapping-key octets with
   `HKDF-SHA256(ikm=shared_secret, salt=hkdf_salt, info=hkdf_info)`.
5. Seal the 32-octet CMK with XChaCha20-Poly1305 using `wrapping_key`, the
   supplied 24-octet `wrapping_nonce`, and `grant_aad`.
6. Store the 32-octet ciphertext followed by its 16-octet tag as
   `wrapped_cmk`.
7. Sign `grant_signature_input` with the originator Ed25519 signing key.
8. Return the immutable grant and erase temporary secret material to the
   extent reported by the implementation.

The same epoch CMK is sealed independently to every authorized receiver. The
wrapping key is receiver-specific; the message content key is not. A shared
append-only log therefore stores one ciphertext per message rather than one
ciphertext per receiver.

An implementation MUST NOT reuse an ephemeral private key or wrapping nonce
across distinct production grants. Deterministic fixture reuse is confined to
the exact checked-in test cases and is never a production default.

---

## Grant opening and validation order

Opening is fail-closed and proceeds in this order:

1. Structurally decode one complete bounded `D18G` record. Reject wrong magic,
   unsupported version, truncation, oversized identity fields, or trailing
   bytes before cryptographic work.
2. Validate non-empty identity bounds and UUID-v7 channel identity.
3. Compare `originator_id` with the caller's expected originator. A mismatch is
   `unexpected_originator`.
4. Compare `receiver_id` with the local expected receiver. A mismatch is
   `unexpected_receiver`.
5. Compare `channel_id` with the expected channel. A mismatch is
   `unexpected_channel`.
6. Reconstruct `grant_signature_input` and verify the Ed25519 signature with
   the expected originator public key. Failure is `invalid_signature`.
7. Compute X25519 with the local receiver private key and the grant's
   `ephemeral_public_key`. Reject low-order/all-zero agreement as
   `invalid_key_agreement`.
8. Reconstruct the HKDF salt/info and derive the 32-octet wrapping key.
9. Reconstruct `grant_aad` and open the 48-octet `wrapped_cmk` with the stored
   wrapping nonce. AEAD rejection is `authentication_failed`.
10. Require exactly 32 recovered CMK octets, transfer them into a secret-key
    container, erase temporaries as supported, and return the CMK.

No unwrapped CMK may be returned before every step succeeds. An implementation
MUST NOT try another receiver identity, channel, public key, private key, epoch,
or derivation after a failure.

---

## Receiver epoch state

One receiver keeps state for exactly one `(originator_id, receiver_id,
channel_id, receiver_private_key, originator_public_key)` tuple.

```text
ReceiverEpochKeys
|-- expected identities and public keys
|-- latest_grant            D18G grant or absent
`-- epoch_keys              map<u64, ChannelMasterKey>
```

Installing a grant follows these rules before any state mutation:

1. If no grant is installed, open the candidate and install it.
2. If `candidate.key_epoch` is below the latest installed epoch, return
   `decreasing_epoch` without opening it.
3. If the epoch equals the latest epoch and every grant field is
   byte-identical, return `idempotent` without opening or replacing state.
4. If the epoch equals the latest epoch but any grant field differs, return
   `conflicting_grant` without opening or replacing state.
5. If the epoch is greater, open the candidate. Only after successful opening
   store its CMK, retain prior epoch CMKs, make it the latest grant, and return
   `installed`.

A receiver may legitimately skip epochs for which it received no grant, so a
higher candidate need not equal `latest + 1`. The channel's originator-side
rotation planner, however, advances the global channel epoch exactly once per
rotation.

Failed opening MUST leave the latest grant and every retained key unchanged.
Historic keys remain available for historic D18F messages. No state operation
may silently replace or discard a prior epoch key.

---

## Rotation and revocation plan

Rotation is a pure cryptographic operation. Inputs are the current global
channel epoch, a newly generated CMK, the caller-authorized non-empty receiver
set, the originator signing key, and independent seal material for each
receiver.

1. Reject `current_epoch == u64::MAX` as `epoch_exhausted`.
2. Set `new_epoch = current_epoch + 1` exactly.
3. Sort receivers by raw receiver-ID bytes for deterministic output. Reject an
   empty set, duplicate IDs, IDs outside 1 through 4,096 octets, and duplicate
   public-key bindings for the same ID.
4. Seal the same `new_cmk` independently to every selected receiver using
   `new_epoch` and unique ephemeral private key/nonce material.
5. Return `new_epoch`, the secret `new_cmk`, and the ordered grants only after
   every seal succeeds. A partial grant list is never a successful plan.

A revoked receiver is absent from the selected receiver set and receives no
grant for `new_epoch`. It can still open historic messages for epochs whose CMK
it retained. It cannot open new messages merely because it retains an old CMK:
D18F resolves the exact CMK named by each message's `key_epoch`.

This is prospective revocation, not retroactive erasure. Once a receiver has
obtained plaintext or an old CMK, no protocol can make it forget those bytes.

The rotation planner does not authorize receiver additions or removals. The
caller supplies an already-authorized target set. Governance, durable
membership history, and operator approval remain outside D18Q.

---

## Durable activation boundary

D18Q produces key material and grants but does not make a new epoch current.
An integration with D18P MUST preserve all of these conditions:

- the originator's new CMK is durably placed in approved secret custody before
  any message can publish at the new epoch
- every selected receiver's immutable `D18G` grant is durable before the new
  epoch becomes visible to publishers
- the current epoch advances atomically from `E` to `E + 1`; concurrent
  rotations cannot both win
- a crash either leaves epoch `E` current or permits deterministic completion
  of the already-selected `E + 1` plan
- a publisher never falls back to an old CMK, invents a CMK, or emits a message
  for an epoch whose activation is incomplete
- revoked receivers receive no new grant while old message/grant history
  remains immutable

D18P version 1 deliberately makes its channel definition immutable except for
destruction. It therefore has no portable epoch-activation transaction today.
That missing durable transition is tracked by #11734 and MUST NOT be claimed as
implemented merely because an in-memory D18Q rotation plan succeeds.

---

## Secret erasure and capability reporting

Every implementation MUST prevent secret values from appearing in errors,
logs, audit records, debug formatting, fixture summaries, or public grant
objects. It MUST minimize copies and release temporary references promptly.

Each language package MUST report one of these implementation capabilities:

| Capability | Meaning |
| --- | --- |
| `guaranteed` | Secret containers and temporary buffers are overwritten on every controlled destruction path; the runtime and compiler contract make that claim enforceable |
| `best_effort` | Mutable owned buffers are overwritten where possible, but garbage collection, copies, or compiler/runtime behavior prevents a physical-memory guarantee |
| `not_enforceable` | The language/runtime cannot reliably overwrite the relevant immutable secret representation; logical lifetime and non-disclosure controls still apply |

A package MUST NOT report `guaranteed` merely because it drops a reference,
clears a collection, relies on garbage collection, or wraps an immutable value
whose copies cannot be tracked. Cross-language conformance records the declared
capability but does not pretend all runtimes provide identical physical erasure.

Channel destruction drops every locally retained D18Q CMK and private key and
invokes the strongest supported erasure path. Durable ciphertext and public
grants remain append-only and are not deleted by logical destruction.

---

## Portable API contract

Names may follow language conventions, but each package exposes operations
equivalent to:

```text
channel_master_key_generate(random_source) -> ChannelMasterKey
receiver_key_pair_generate(random_source) -> ReceiverKeyPair

seal_channel_key(fields, cmk, receiver_public_key, signing_key, random_source)
  -> immutable SealedChannelKeyGrant

seal_channel_key_with_material(
  fields, cmk, receiver_public_key, signing_key,
  ephemeral_private_key, wrapping_nonce
) -> immutable SealedChannelKeyGrant

open_channel_key_grant(
  grant, expected_originator_id, expected_receiver_id, expected_channel_id,
  receiver_private_key, originator_public_key
) -> ChannelMasterKey

grant_serialize(grant) -> D18G v1 bytes
grant_deserialize(bytes) -> structurally valid immutable grant

receiver_epoch_keys.install(grant) -> installed | idempotent
receiver_epoch_keys.key(epoch) -> ChannelMasterKey | missing

plan_rotation(current_epoch, authorized_receivers, new_cmk,
              signing_key, per_receiver_material)
  -> new_epoch, new_cmk, ordered grants

secret_erasure_capability() -> guaranteed | best_effort | not_enforceable
```

Production overloads MAY hide the random source behind an operating-system
CSPRNG. Tests and fixture generators use the explicit source/material APIs.
Deserialization is structural and MUST NOT be named or documented as
verification.

---

## Stable error taxonomy

Language-specific exception or result types are idiomatic. Conformance
fixtures compare these exact machine-readable codes:

| Code | Meaning |
| --- | --- |
| `invalid_magic` | Record is not `D18G` |
| `unsupported_version` | Binary version is not 1 |
| `truncated_record` | A declared or fixed field is incomplete |
| `trailing_bytes` | Bytes remain after a complete record |
| `length_limit_exceeded` | An identity field exceeds 4,096 octets |
| `invalid_field` | Empty identity, UUID, key length, field type, or field shape is invalid |
| `randomness_unavailable` | A required CSPRNG request did not return complete secure material |
| `invalid_key_agreement` | X25519 rejected a private/public input or produced an all-zero secret |
| `key_derivation_failed` | HKDF-SHA256 could not derive exactly 32 octets |
| `invalid_signature` | Ed25519 verification of the canonical grant signature input failed |
| `unexpected_originator` | Grant originator differs from the expected identity |
| `unexpected_receiver` | Grant receiver differs from the local expected identity |
| `unexpected_channel` | Grant channel differs from the expected channel |
| `authentication_failed` | XChaCha20-Poly1305 rejected wrapped CMK, tag, nonce, key, or AAD |
| `invalid_wrapped_key` | Successfully opened plaintext is not exactly 32 octets |
| `conflicting_grant` | A different grant already occupies the latest epoch |
| `decreasing_epoch` | Receiver installation attempted an older epoch |
| `epoch_exhausted` | Rotation would increment `u64::MAX` |
| `missing_epoch_key` | Receiver has no installed CMK for a requested message epoch |

Error text may differ. Errors and structured fields MUST NOT contain CMKs,
private keys, shared secrets, wrapping keys, recovered plaintext, or full
secret-bearing state.

---

## Required shared fixtures

The fixture package lives under:

```text
code/fixtures/chief-of-staff-channel-key-grant/v1/
```

Its Rust-generated manifest identifies this spec, fixture version, generator
Git blob hash, a prominent test-only secret warning, deterministic inputs,
intermediate public bytes, exact `D18G` bytes, expected opened CMKs, rotation
traces, stable errors, and the closed secret-erasure capability vocabulary.
Each consumer declares its own capability beside its native fixture test; the
central gate reports those declarations without rewriting the Rust-generated
cryptographic corpus. The content-addressed generator identity remains stable
across rebase and squash merges.

The Rust `chief-of-staff-channel-crypto::grant_profile` adapter is the
reference consumer. It routes production and explicit-material sealing through
the same D18G implementation, exposes immutable high-level values and stable
errors, consumes the complete manifest, and declares `guaranteed` controlled
destruction through the repository-owned volatile-zeroization container.

The corpus must cover at least:

- epoch 0 with binary originator/receiver IDs and an RFC UUID-v7 channel
- two receivers opening the same CMK from distinct ephemeral keys, wrapping
  keys, nonces, ciphertexts, and signatures
- exact X25519 public/shared outputs, HKDF salt/info/output, grant AAD,
  signature input, wrapped CMK, signature, and `D18G` bytes
- the `u64::MAX` epoch encoding without attempting another rotation
- structural wrong-magic, unsupported-version, every truncated prefix,
  oversized identity, and trailing-byte failures
- empty identity and invalid UUID-v7 high-level failures
- unexpected originator, receiver, and channel in their required order
- invalid Ed25519 signature before key agreement or AEAD work
- low-order receiver and ephemeral X25519 public keys
- wrong receiver private key, wrong wrapping nonce, mutated wrapped CMK/tag,
  and derivation/AAD binding failures
- first install, byte-identical retry, same-epoch conflict, decreasing epoch,
  failed higher-epoch install with unchanged state, and a legitimate skipped
  receiver epoch
- rotation from receivers A+B to B only: B opens old and new CMKs, A retains
  the old CMK and has no new grant
- compact oversize recipes rather than large checked-in blobs

Every language must reproduce the declared deterministic bytes, outcomes, and
stable errors. No lane may shell out to another language or substitute a host
crypto API for the repository-owned X25519, HKDF-SHA256,
XChaCha20-Poly1305, Ed25519, or SHA-256 primitives.

---

## Six-language rollout and completion gate

Issue #141 is complete only after this sequence:

1. Land this normative D18Q profile without changing the production Rust
   `D18G` wire format.
2. Add the Rust-generated shared fixture manifest and a Rust compatibility
   adapter that consumes it byte-for-byte.
3. Implement the D18Q grant, receiver epoch, rotation, and honest erasure
   capability contract in TypeScript, Python, Go, Ruby, and Elixir.
4. Add one central repository check that requires exactly those six consumers,
   runs every package-native build, verifies generator provenance, and
   regenerates the manifest byte-for-byte.
5. Close #141 only after that exact-head gate and required Ubuntu, macOS, and
   Windows builds pass.

The durable epoch-activation follow-up #11734 remains required for an end-to-end
crash-safe rotating channel, but it does not change the D18Q cryptographic bytes
or receiver-local epoch behavior.

Part of #141 and #128. The normative profile is tracked by #11727; the shared
fixture and Rust compatibility lock are tracked by #11735.
