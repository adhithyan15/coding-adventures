# VLT-PM01 — Password Manager Repository Format V1

**Status:** Draft 0.1 — implemented with the first `vault-pm-format` crate

**Parent:** VLT-PM00 §§8–10 and §23 Phase 0

## 1. Purpose

This specification fixes the bytes that all password-manager clients exchange.
It covers bootstrap records, repository object frames, device certificates,
commit objects, and discovery announcements. Storage adapters treat every value
as opaque bytes.

The format crate serializes, validates, hashes, and constructs signing
preimages. It does not derive keys, encrypt, decrypt, sign, verify signatures,
authorize devices, choose merge results, or perform storage I/O.

## 2. V1 invariants

- Structured values use the repository's RFC 8949 length-first canonical CBOR
  profile from `canonical-cbor`.
- Integer map keys are schema field numbers. Unknown, missing, duplicate, or
  incorrectly typed fields are rejected in V1 security objects.
- Fixed-size identifiers and cryptographic fields are CBOR byte strings of
  their exact declared length.
- Counts and variable byte strings are bounded before construction.
- Signing preimages prepend a domain label to the canonical unsigned value.
- IDs prepend a separate domain label before SHA-256.
- Object frames use an exact binary layout and reject trailing bytes.
- Error display text never includes attacker-controlled persisted bytes.

## 3. Common vocabulary

| Type | Size | Meaning |
|---|---:|---|
| `VaultId` | 16 bytes | random vault identifier |
| `DeviceId` | 16 bytes | random certified device identifier |
| `ObjectId` | 32 bytes | SHA-256 identifier of a complete object frame |
| `BootstrapId` | 32 bytes | SHA-256 identifier of a signed bootstrap |
| Ed25519 public key | 32 bytes | authority or device signing key |
| X25519 public key | 32 bytes | device wrapping key |
| Ed25519 signature | 64 bytes | signature supplied/verified by another package |
| XChaCha nonce | 24 bytes | independently generated AEAD nonce |
| Poly1305 tag | 16 bytes | detached AEAD tag |

V1 bounds:

| Value | Bound |
|---|---:|
| object ciphertext | 64 MiB |
| generic AEAD ciphertext | 64 KiB |
| recovery wraps | 16 |
| commit parents | 32 |
| commit `added_objects` | 4096 |
| device capability codes | 64 |

## 4. Canonical CBOR schemas

Notation: `uint` is a non-negative CBOR integer, `bstr.N` is an exact N-byte
string, `array<T>` is a definite-length array, and `null|T` is a nullable field.
Every table is encoded as a CBOR map keyed by the integer in the first column.

### 4.1 `AeadEnvelopeV1`

| Key | Field | Type |
|---:|---|---|
| 1 | suite | `uint`, currently 1 |
| 2 | nonce | `bstr.24` |
| 3 | ciphertext | bounded `bstr` |
| 4 | tag | `bstr.16` |

Suite 1 means XChaCha20-Poly1305 with a detached tag. The format does not imply
that every envelope has the same key purpose or associated data.

### 4.2 `Argon2idParametersV1`

| Key | Field | Type |
|---:|---|---|
| 1 | memory KiB | `uint`, 8 MiB–1 GiB |
| 2 | iterations | `uint`, 1–32 |
| 3 | lanes | `uint`, 1–16 |
| 4 | salt | `bstr.16` |

These are parser safety bounds, not recommended defaults. Policy chooses a
calibrated value within them.

### 4.3 `RecoveryWrapV1`

| Key | Field | Type |
|---:|---|---|
| 1 | kind | `uint`, non-zero registry value |
| 2 | recipient ID | `bstr.16` |
| 3 | wrapped root key | `AeadEnvelopeV1` |

The envelope ciphertext is exactly 32 bytes because it wraps the 256-bit VRK.

### 4.4 `BootstrapV1`

| Key | Field | Type |
|---:|---|---|
| 1 | format version | `uint`, exactly 1 |
| 2 | vault ID | `bstr.16` |
| 3 | generation | `uint` |
| 4 | previous bootstrap | `null|bstr.32` |
| 5 | crypto suite | `uint`, exactly 1 |
| 6 | Argon2id parameters | map from §4.2 |
| 7 | passphrase root wrap | envelope from §4.1 |
| 8 | authority public key | `bstr.32` |
| 9 | recovery wraps | bounded array from §4.3 |
| 10 | authority signature | `bstr.64` |

Generation zero requires `previous_bootstrap = null`. Later generations require
a previous ID. The signature field is omitted from the unsigned form.

### 4.5 `DeviceCertificateV1`

| Key | Field | Type |
|---:|---|---|
| 1 | format version | `uint`, exactly 1 |
| 2 | vault ID | `bstr.16` |
| 3 | device ID | `bstr.16` |
| 4 | Ed25519 public key | `bstr.32` |
| 5 | X25519 public key | `bstr.32` |
| 6 | creation time ms | `uint`, advisory |
| 7 | capabilities | sorted unique `array<uint>` |
| 8 | authority signature | `bstr.64` |

Capability codes are registry values. Sorting and uniqueness prevent multiple
encodings of the same set.

### 4.6 `CommitV1`

| Key | Field | Type |
|---:|---|---|
| 1 | format version | `uint`, exactly 1 |
| 2 | vault ID | `bstr.16` |
| 3 | device ID | `bstr.16` |
| 4 | device counter | `uint`, non-zero |
| 5 | parents | bounded unique `array<bstr.32>` |
| 6 | catalog root | `bstr.32` |
| 7 | added objects | bounded unique `array<bstr.32>` |
| 8 | tombstone root | `null|bstr.32` |
| 9 | advisory wall time ms | `uint` |
| 10 | device certificate object | `bstr.32` |
| 11 | device signature | `bstr.64` |

Parent order and added-object order are preserved and signed. Producers must
sort them bytewise; decoders reject unsorted or duplicate IDs. This gives one
representation for a logical set while preserving deterministic traversal.

### 4.7 `AnnouncementV1`

| Key | Field | Type |
|---:|---|---|
| 1 | format version | `uint`, exactly 1 |
| 2 | vault ID | `bstr.16` |
| 3 | device ID | `bstr.16` |
| 4 | device counter | `uint`, non-zero |
| 5 | commit object ID | `bstr.32` |
| 6 | device certificate object ID | `bstr.32` |
| 7 | device signature | `bstr.64` |

Announcements are signed to reject provider-injected discovery spam before a
client walks arbitrary object graphs. The client fetches and authority-verifies
the one referenced certificate, then verifies the announcement before fetching
the commit graph. Its counter, device identity, and certificate must match the
referenced verified commit.

## 5. Signing domains

The signing preimage is `ASCII_DOMAIN || canonical_unsigned_cbor`.

| Object | ASCII domain |
|---|---|
| bootstrap | `VPM-BOOTSTRAP-SIGN-v1` |
| device certificate | `VPM-DEVICE-CERT-SIGN-v1` |
| commit | `VPM-COMMIT-SIGN-v1` |
| announcement | `VPM-ANNOUNCEMENT-SIGN-v1` |

Signatures are raw 64-byte Ed25519 signatures. The crate exposes unsigned bytes
and a `with_signature` constructor; it never accepts a secret key.

## 6. IDs

```text
BootstrapId = SHA256("VPM-BOOTSTRAP-ID-v1" || signed_bootstrap_cbor)
ObjectId    = SHA256("VPM-OBJECT-ID-v1"    || complete_object_frame)
```

Commit parents and announcements carry the `ObjectId` of the encrypted commit
frame, not a hash of commit plaintext. This preserves ciphertext-only storage
and allows randomized repository encryption.

## 7. Object frame V1

All integers are unsigned big-endian:

```text
offset  size       field
0       4          magic = "VPO1"
4       2          suite = 1
6       24         object-DEK wrap nonce
30      32         wrapped 256-bit object DEK ciphertext
62      16         object-DEK wrap tag
78      24         payload nonce
102     8          payload ciphertext length N
110     N          payload ciphertext
110+N   16         payload tag
```

Total length is `126 + N`. Non-V1 magic/suite, `N > 64 MiB`, truncation,
overflow, and trailing bytes are rejected before allocating payload storage.

Associated data is owned by the encryption layer and must bind the V1 magic,
suite, vault ID, object kind, and key-purpose label. Those inputs are not stored
as plaintext provider metadata.

## 8. Compatibility and errors

V1 decoders are closed schemas. A later format adds a new version/suite and a
new decoder; it does not make V1 accept unknown fields. Callers can distinguish
CBOR failure, schema failure, unsupported version/suite, bound violation,
ordering failure, invalid generation, invalid counter, and frame truncation.

Error display strings contain a static category only. Detailed variants may
carry a static field name, never persisted field bytes.

## 9. Verification

- round trips for every structured type;
- exact golden bytes and IDs for representative values;
- signature-field mutation changes signed bytes/IDs but not unsigned preimages;
- every field deletion/type substitution/unknown field is rejected;
- boundary tests at and beyond every count/length/KDF limit;
- frame truncation at every fixed boundary and length-overflow rejection;
- noncanonical CBOR, unsorted sets, duplicates, zero counters, and invalid
  bootstrap generations are rejected;
- no filesystem, network, environment, clock, process, or random-number access.

The shared fixture file is `code/specs/fixtures/vault-pm-format-v1.hex`.

---

*End of VLT-PM01.*
