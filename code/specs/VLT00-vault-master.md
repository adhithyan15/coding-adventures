# VLT00 — Vault Master Specification

## Status

Draft 0.1 — architectural overview + learning textbook for the full vault stack.
Per-package specs (VLT01..VLT99) descend from this document.

## Purpose

This is the **master spec** for a transport-agnostic, storage-agnostic,
multi-user, end-to-end-encrypted secret vault built as a pipeline of small
composable packages.

It serves three audiences simultaneously:

1. **Architects** — who want to understand the layered package graph and
   the data flow of one request through it.
2. **Implementers** — who want a roadmap for which package to write next
   and how it composes with the others.
3. **Learners** — who want to understand modern secret-management crypto
   from first principles. Per the repo's literate-programming rule
   (CLAUDE.md), every concept is introduced from the ground up.

## How to read this document

The spec is laid out leaf-first, so a reader following it top-to-bottom
sees primitives before the structures that use them. If you only have
fifteen minutes, read:

- §1 Threat model
- §3 Architectural overview
- §4 Request pipeline
- §22 Roadmap

Each subsequent chapter (§5 through §21) is a self-contained primer for
one layer of the stack. Each ends with the **package list** for that
layer — the actual code units to be implemented.

The vault project's other specs are referenced by their existing IDs:

- **HF01..HF06** — hash functions (SHA-1, SHA-256, SHA-512, BLAKE2b, etc.)
- **D18A** — Chief of Staff stores (records, blobs, leases at the agent layer)
- **VLT01** — sealed store (envelope encryption on top of `storage-core`)
- **storage-sqlite** — one storage backend
- **hkdf**, **x25519**, **ed25519**, etc. — primitive specs

This document does not duplicate them; it places them in the larger map
and identifies the new specs that need to be written.

---

## §1 Threat Model

A vault is only meaningful with respect to the threats it defends against.
This section names them up front so every later design choice can be
checked against this list.

### Adversaries we defend against

1. **Untrusted storage backend.** Google Drive, S3, the user's local
   filesystem, an attacker who steals the laptop or breaches the cloud
   account. The backend may read every byte we write, may tamper with
   bytes (we'll detect), may delete bytes (we'll detect on access), and
   may roll back to old versions (we'll detect via signed manifests).
2. **Network observers.** Anyone on the wire between client and server.
   TLS is the floor; Signal-protocol session is the ceiling. Observers
   see only opaque, ratcheted ciphertext.
3. **Compromise of a single device.** A laptop is stolen, an extension
   is exploited, a process is dumped. The damage is bounded by the lease
   lifetime (seconds-to-minutes), and forward secrecy ensures that
   captured ciphertext from before the compromise stays secret.
4. **Coerced or careless emergency contact.** A designated emergency
   contact tries to access the vault without authorization. The owner
   has a wait period to deny; without denial they get scoped access only.
5. **Server operator (partial).** The server holds long-lived encrypted
   data and routes Signal sessions, but cannot decrypt vault contents
   (E2E) and cannot forge time (timer oracle is pluggable from trusted
   to fully decentralized via VDF).
6. **Lost passphrase.** Protected by recovery: Shamir social recovery,
   hardware-key recovery, written recovery sheet (out of scope of MVP).

### Threats explicitly out of scope

1. **Endpoint malware with kernel access** while the vault is unlocked.
   If the OS is owned, the vault key is in RAM and is exfiltratable.
   Mitigation: Secure-Enclave / TPM-bound keys for the master key
   reduce but do not eliminate this.
2. **Side-channel attacks** on the user's hardware (Spectre, Rowhammer,
   power analysis). Crypto code aims for constant-time, but we do not
   defend against advanced hardware side-channels.
3. **Traffic analysis** beyond size padding. Access patterns to storage
   are visible to the storage operator. Full PIR is too expensive.
4. **Pre-quantum guarantees only.** All asymmetric crypto in the MVP is
   ECC (X25519 / Ed25519 / P-256). Post-quantum migration is a future
   chapter.
5. **Legal compulsion.** The vault has no plausible-deniability mode.

### Properties we provide

- **Confidentiality.** Storage operator and network observer see only
  ciphertext.
- **Integrity.** Tampering is detected on read (AEAD tags + signed manifests).
- **Authenticity.** Every entry is signed by its writer. Audit log is
  signed and append-only.
- **Forward secrecy.** Signal session ratchet; lease keys derived from
  ratchet state.
- **Post-compromise security.** Ratchet recovers after the next clean
  message exchange.
- **Authenticated metadata listing.** Even the catalog of entries is
  encrypted; only blob IDs leak.

---

## §2 Glossary

Skim now, refer back later.

| Term | Definition |
|------|-----------|
| **AEAD** | Authenticated Encryption with Associated Data — encrypts and authenticates in one operation. ChaCha20-Poly1305 and AES-GCM are the two we use. |
| **AAD** | Associated Data — bytes covered by the AEAD authentication tag but not encrypted. |
| **CTAP2** | Client-to-Authenticator Protocol v2 — the wire protocol between a host and a FIDO2 hardware key. |
| **CRDT** | Conflict-free Replicated Data Type — data structure that auto-merges concurrent edits. |
| **DEK** | Data Encryption Key — per-record symmetric key. |
| **Envelope encryption** | Pattern where each record gets a fresh DEK, the body is encrypted under the DEK, and the DEK is wrapped under a master KEK. Rotation cost is O(records) wraps not O(records) re-encryptions. |
| **FIDO2** | Specification covering hardware authenticators (CTAP2) plus the WebAuthn browser API. |
| **HKDF** | HMAC-based Key Derivation Function (RFC 5869). |
| **KDF** | Key Derivation Function — turn a passphrase or random material into one or more keys. |
| **KEK** | Key Encryption Key — wraps DEKs. |
| **Lease** | Time-bounded capability. A leased secret is encrypted under a key the consumer holds for only the lease's lifetime. |
| **NMH** | Native Messaging Host — WebExtension protocol for browser↔native-process communication (stdio + 4-byte length prefix). |
| **PRK** | Pseudo-Random Key — output of HKDF-Extract. |
| **Recipient wrap** | Encrypting a key to multiple public keys so any recipient can decrypt. age-style. |
| **RP** | Relying Party (WebAuthn) — the service that consumes assertions, e.g. a web app authenticating a user. |
| **Secure Enclave** | Apple's hardware key store; keys cannot leave the chip. |
| **Shamir** | Shamir's Secret Sharing — split a secret into N shares, any K combine to recover. |
| **TOTP / HOTP** | Time-based / HMAC-based One-Time Password. RFC 6238 / RFC 4226. |
| **TPM** | Trusted Platform Module — hardware key store on PC/Linux/Windows. |
| **VDF** | Verifiable Delay Function — a computation that demonstrably takes time T regardless of parallelism. Foundation for trustless timer oracles. |
| **WebAuthn** | W3C browser API for FIDO2 authentication. |

---

## §3 Architectural Overview

The vault is **not a server**. It is a stack of layers where:

- The **bottom** is opaque-byte storage (disk, SQLite, Google Drive).
- The **middle** is encrypted-record storage, then typed entries, then
  the vault service that dispatches operations.
- The **top** is transport adapters (TCP, HTTP, stdio, browser native
  messaging) wrapped in a Signal-protocol secure channel.

Three orthogonal axes cut across all layers:

1. **Storage backend** — pluggable at the bottom.
2. **Transport** — pluggable at the top.
3. **Sync strategy** — last-writer-wins, vector-clock, or CRDT.

The vault becomes a "server" only when you instantiate the core stack
and bolt a transport in front of it. The same packages can be wired
together to produce:

- A standalone CLI vault (in-memory transport, file storage).
- A self-hosted server like HashiCorp Vault (TCP/HTTP transport, SQLite
  + cloud backup, secret engines, policies).
- A Bitwarden-style sync server (HTTP transport, S3 backup, multi-user).
- A 1Password-style desktop app with browser extension (NMH transport,
  iCloud Drive backup, passkey provider).

The same pipeline. Different choices at each socket.

### Layered stack (terse)

```
┌──────────────────────────────────────────────────────────────┐
│ Clients: CLI, Tauri desktop, browser extension, agent SDKs   │
├──────────────────────────────────────────────────────────────┤
│ Importers / exporters: bitwarden, 1password, kdbx, csv       │
├──────────────────────────────────────────────────────────────┤
│ Transports (any) ──── wrapped in signal-session              │
│   stdio │ uds │ tcp │ http │ nmh │ in-memory                 │
├──────────────────────────────────────────────────────────────┤
│ Vault service: protocol, dispatch, ACL, audit, session       │
├──────────────────────────────────────────────────────────────┤
│ Secret engines     │   Auth methods                          │
│   kv │ db │ pki    │     token │ passphrase │ webauthn       │
│   transit │ totp   │     approle │ oidc                      │
├──────────────────────────────────────────────────────────────┤
│ Sharing & multi-user: invite, share, revoke, groups          │
├──────────────────────────────────────────────────────────────┤
│ Emergency & recovery: timer-oracle, emergency-mpc, shamir    │
├──────────────────────────────────────────────────────────────┤
│ Typed entries: password, note, file, totp, passkey, ssh-key  │
├──────────────────────────────────────────────────────────────┤
│ Lease layer: lease-store, lease-wrap, lease-client           │
├──────────────────────────────────────────────────────────────┤
│ Sync & CRDT: lww │ vclock │ crdt (lww-register, or-set, rga) │
├──────────────────────────────────────────────────────────────┤
│ Encrypted-blob layer: vault-index, chunk-store, padding      │
├──────────────────────────────────────────────────────────────┤
│ Storage backend trait + adapters                             │
│   fs │ sqlite │ mem │ gdrive │ icloud │ webdav │ s3 │ git    │
├──────────────────────────────────────────────────────────────┤
│ Identity & secure channel                                    │
│   user-identity │ device-identity │ x3dh │ double-ratchet    │
├──────────────────────────────────────────────────────────────┤
│ Envelope & wrapping: envelope, aead-wrap, kdf-params,        │
│   master-key, key-hierarchy, recipient-wrap, shamir          │
├──────────────────────────────────────────────────────────────┤
│ Encoding: cbor, cose-{key,sign,encrypt}, base64, der-asn1    │
├──────────────────────────────────────────────────────────────┤
│ Crypto leaves: sha2/3, hmac, hkdf, argon2id, aes-gcm,        │
│   chacha20-poly1305, x25519, ed25519, p256-ecdsa             │
└──────────────────────────────────────────────────────────────┘
```

---

## §4 Request Pipeline

To make the layering concrete, here is a single "read entry" request
traced through every package it touches. This is the thing to keep in
mind whenever you implement a layer: where in this pipeline does my
code sit, and what does it owe to the layer above and below it?

```
CLIENT                              NETWORK                  SERVER

caller: vault.get("github-token")
   │
   ▼
vault-client
   build GetEntry{name="github-token"}              (vault-protocol)
   │
   ▼
device-identity        sign with device Ed25519
   │
   ▼
signal-session         encrypt (advance ratchet, derive lease-key root)
   │
   ▼
cbor                   canonical encode
   │
   ▼
vault-transport-tcp    length-prefix + TLS
   │
   │═══════════════════════════════════════════════>
                                                vault-transport-tcp recv
                                                    │
                                                    ▼
                                                cbor decode
                                                    │
                                                    ▼
                                                signal-session decrypt
                                                    │
                                                    ▼
                                                vault-protocol parse
                                                    │
                                                    ▼
                                                vault-service dispatch
                                                    │
                                                    ├─► auth-method check token
                                                    ├─► vault-acl check policy
                                                    ├─► vault-audit append
                                                    ├─► vault-session look up KEK
                                                    │
                                                    ▼
                                                secret-engine-kv resolve path
                                                    │
                                                    ▼
                                                vault-index name → blob-id
                                                    │
                                                    ▼
                                                vault-sync resolve conflicts
                                                    │
                                                    ▼
                                                blob-store-encrypted get
                                                  └─► blob-padding
                                                      └─► blob-store-gdrive
                                                    │
                                                    ▼
                                                envelope parse header
                                                    │
                                                    ▼
                                                recipient-wrap unwrap DEK
                                                    │
                                                    ▼
                                                aead-wrap decrypt body
                                                    │
                                                    ▼
                                                entry-schema → entry-password
                                                    │
                                                    ▼
                                                lease-store issue lease
                                                    │
                                                    ▼
                                                lease-wrap encrypt under
                                                            lease-key
                                                    │
                                                    ▼
                                                vault-protocol response
                                                    │
                                                    ▼
                                                signal-session encrypt
                                                    │
                                                    ▼
                                                cbor encode → transport send
   <═══════════════════════════════════════════════
   │
   ▼
vault-transport-tcp recv → cbor decode → signal-session decrypt
   │
   ▼
vault-protocol parse GetEntryResponse
   │
   ▼
lease-client register, start expiry timer
   │
   ▼
lease-wrap decrypt under lease-key
   │
   ▼
entry-schema → entry-password
   │
   ▼
caller receives entry; lease-client zeros key on expiry
```

Every named layer in that diagram is a package. Reading bottom up,
they are introduced one chapter at a time below.

---

## §5 Crypto Leaves (already implemented)

These are the foundations. Most of them already exist in the repo as
separately-specified packages (HF01..HF06, hkdf, argon2{d,i,id},
aes-modes, chacha20-poly1305, x25519, ed25519). The vault depends on
them, does not redefine them, and adds **p256-ecdsa** as a new leaf for
WebAuthn compatibility.

### What each does, in one line

- **SHA-2 family** (sha256, sha512): Merkle-Damgård hash. Not directly
  used as a vault primitive but transitively needed by HMAC, HKDF, the
  signature schemes, and CBOR canonicalization.
- **SHA-3 / SHAKE**: Keccak-based. Optional alternative.
- **HMAC**: keyed pseudo-random function over a hash. Foundation of
  HKDF, TOTP, and many integrity tags.
- **HKDF**: extract-then-expand KDF on top of HMAC. The primary
  symmetric key-derivation primitive.
- **Argon2id**: memory-hard password KDF. Preferred over PBKDF2 for new
  vaults; PBKDF2 retained for Bitwarden-import compat.
- **AES-GCM, ChaCha20-Poly1305**: AEAD ciphers. ChaCha20-Poly1305 is the
  default; AES-GCM is provided for hardware-accelerated platforms.
- **X25519**: Diffie-Hellman over Curve25519. Foundation of ECDH key
  agreement, recipient wrapping, and Signal X3DH.
- **Ed25519**: Edwards-curve digital signature. Foundation of identity,
  audit-log signing, and CTAP2 attestation.
- **P-256 (ECDSA + ECDH)**: NIST curve required by WebAuthn (most
  authenticators only support COSE alg `-7`).

### New leaves to add for the vault

| Package | Purpose | Spec |
|---------|---------|------|
| `p256-ecdsa` | NIST P-256 ECDSA (WebAuthn compat) | new |
| `p256-ecdh` | NIST P-256 ECDH | new |
| `secp256k1` | Optional, only for Bitcoin-style use cases | deferred |

---

## §6 Encoding & Format Primitives

Crypto primitives produce raw bytes. Everything above them needs a
canonical, self-describing serialization. CBOR is the workhorse.

### Why CBOR (and not JSON or Protobuf)?

- **Binary and compact** — important when payloads carry crypto material.
- **Self-describing** — types are tagged; readers don't need a schema to
  parse, only to interpret. Good for evolution.
- **Canonical encoding** — RFC 8949 §4.2 specifies a deterministic
  encoding so that signatures over CBOR are reproducible.
- **The standard for COSE** (RFC 8152), CTAP2, and WebAuthn — using
  CBOR everywhere lets us reuse one decoder for vault-protocol messages,
  CTAP2 wire frames, WebAuthn attestation objects, and COSE signed
  payloads.
- **No schema compiler step** — easier than Protobuf / FlatBuffers.

JSON-RPC is provided at the transport layer for HTTP debuggability, but
internally everything is CBOR.

### Primer on CBOR

A CBOR item is one byte of "major type" + length info, followed by
content. Major types:

| Type | Meaning |
|------|---------|
| 0 | Unsigned int |
| 1 | Negative int |
| 2 | Byte string |
| 3 | Text string (UTF-8) |
| 4 | Array |
| 5 | Map |
| 6 | Tag (semantic annotation) |
| 7 | Float / simple value |

`{"a": 1}` becomes `A1 61 61 01` — five bytes. JSON would be 8.

Canonical encoding rules: shortest length encoding, map keys sorted by
encoded byte order, no indefinite-length items.

### Primer on COSE

COSE is "JOSE for CBOR" — it specifies how to express signed and
encrypted payloads in CBOR.

- **COSE_Key**: a CBOR map representing a public/private key, with
  algorithm and curve identified by integers (e.g. `kty=2, crv=1, alg=-7`
  for P-256 ECDSA).
- **COSE_Sign1**: a CBOR array `[protected_header, unprotected_header,
  payload, signature]`. Protected headers are covered by the signature.
- **COSE_Encrypt0**: similar shape for AEAD.

WebAuthn attestation objects are COSE structures. CTAP2 messages are
CBOR maps where keys are integers.

### Packages

| Package | Purpose |
|---------|---------|
| `cbor` | RFC 8949 encoder + canonical-encoding decoder. |
| `cose-key` | COSE_Key parse / serialize. |
| `cose-sign` | COSE_Sign / COSE_Sign1. |
| `cose-encrypt` | COSE_Encrypt / COSE_Encrypt0. |
| `base32` | RFC 4648; used by `otpauth://` URIs. |
| `base64` | RFC 4648; URL-safe variant. |
| `base58` | Bitcoin-style; used by some recovery encodings. |
| `der-asn1` | X.509 / PKCS#8 / PKCS#10 — used by PKI engine and WebAuthn attestation chains. |

---

## §7 Envelope & Key Wrapping

This is the lowest cryptographic layer specific to the vault. Everything
else builds on it.

### The envelope-encryption pattern

A naive design encrypts every record under one master key. This is
brittle: rotating the master key means re-encrypting every record, and
nonce reuse becomes a global risk.

The envelope pattern fixes both:

1. For each record, generate a fresh **Data Encryption Key (DEK)** —
   32 random bytes.
2. Encrypt the body under the DEK using AEAD (one-key, fresh DEK → no
   nonce-collision risk).
3. **Wrap** the DEK by encrypting it under a longer-lived **Key
   Encryption Key (KEK)**, also via AEAD.
4. Store `(wrapped_dek, body_ciphertext, body_nonce, body_tag)` together.

Rotating the KEK becomes O(records × 32 bytes) rewrap, not
O(records × body_size) re-encryption. Compromise of one DEK leaks
exactly one record. The pattern is well-explored — VLT01 already
implements it.

### The envelope binary format

A versioned, self-describing container. Sketch:

```
struct Envelope {
    magic:      [u8; 4]   = b"VLT\x01"
    version:    u8        = 1
    algo_kdf:   u8        // see registry
    algo_aead:  u8        // see registry
    flags:      u16       // padded? recipient-wrapped? lease-wrapped?
    aad_len:    u32
    aad:        [u8; aad_len]
    payload_len: u64
    payload:    [u8; payload_len]
    crc:        u32       // detect bit-rot, not security
}
```

The format is **crypto-agile** — algorithm IDs let multiple algorithms
coexist for interop with legacy formats (Bitwarden, KDBX, age) while
defaulting to a single modern choice (Argon2id + ChaCha20-Poly1305) for
fresh data.

### Recipient wrapping (multi-recipient encryption)

To share a record with multiple users, generate one DEK per record and
wrap it once per recipient. The wrap list is a CBOR array of
`{recipient_pubkey_id, wrapped_dek}` tuples. Any one recipient can
unwrap the DEK with their X25519 private key.

This is the same pattern used by **age** and **PGP**. It composes
cleanly with sharing (add a recipient = append a wrap) and revocation
(remove a recipient = generate a fresh DEK and re-wrap for survivors).

### Shamir's Secret Sharing

A polynomial over GF(2^8) lets us split a secret S into N shares such
that any K of them recover S exactly, and any K-1 reveal nothing.

Used for:
- **Vault unseal** (HashiCorp parity): split the unseal key across
  M operators, require K to bring the vault online.
- **Social recovery**: split a recovery key across N trusted contacts.
- **N-of-M emergency**: combined with the timer oracle.

### Packages

| Package | Purpose |
|---------|---------|
| `envelope` | versioned binary container + algo-id registry. |
| `aead-wrap` | dispatch to ChaCha20-Poly1305 or AES-GCM by algo ID. |
| `kdf-params` | serializable Argon2id (or PBKDF2) parameters + salt. |
| `master-key` | passphrase → KEK via `kdf-params` + Argon2id. |
| `key-hierarchy` | KEK → wrapping subkeys via HKDF labels. |
| `recipient-wrap` | age-style multi-recipient X25519 wrapping. |
| `shamir` | K-of-N secret sharing over GF(2^8). |

---

## §8 Identity, Devices, Secure Channel

Now we get to long-lived asymmetric identities and the Signal-protocol
secure channel that protects vault traffic above any transport.

### User and device identity

A user has a **long-term identity keypair**:

- Ed25519 for signatures (audit trail, designation of emergency
  contacts, signed manifests).
- X25519 for encryption (recipient wrapping, X3DH prekeys).

A user can have multiple devices. Each device has its own subordinate
keypair signed by the user's long-term key. This gives:

- **Per-device forward secrecy** — losing a device only loses that
  device's session keys; the user's long-term keys are unaffected.
- **Selective revocation** — a stolen laptop can be removed from the
  vault by the user signing a "revoke device X" statement.
- **Multi-device sharing** — every device is a recipient of any record
  the user can read.

### Device enrollment

A new device joins the user via QR-code pairing (Signal/WhatsApp model):

1. New device generates its sub-keypair.
2. Existing trusted device scans QR code containing the new device's
   public key.
3. Existing device signs a "device added" statement with its long-term
   key.
4. Existing device sends the statement to the vault server, which
   propagates to all of the user's devices.
5. Re-wrap recent records to include the new device as a recipient
   (lazy: only on next access).

### Primer on the Signal Protocol

The Signal Protocol gives **forward-secret, post-compromise-secure**
end-to-end encryption between two parties over an untrusted network.
It has two phases:

**Phase 1 — X3DH (extended triple Diffie-Hellman):**

Each party publishes a **prekey bundle**: their identity public key
plus a one-time prekey signed by the identity key. Initiator A computes
a shared secret using **three** ECDH operations (identity-A↔prekey-B,
ephemeral-A↔identity-B, ephemeral-A↔prekey-B; optionally a fourth with
a one-time prekey). The combined shared secret seeds the ratchet.

Three DHs is what gives X3DH its forward secrecy AND mutual
authentication AND deniability. Each DH covers a different combination
of long-term and ephemeral keys.

**Phase 2 — Double Ratchet:**

After the X3DH handshake, both parties hold a **root key**. Every
message advances two ratchets:

- **Diffie-Hellman ratchet**: each party periodically generates a new
  ephemeral keypair and includes its public key with messages. The
  receiver does ECDH and updates the root key. Compromise here heals
  after one round trip.
- **Symmetric ratchet**: between DH steps, each message derives a fresh
  message key from the chain key via HKDF. Past message keys can be
  zeroed after use → forward secrecy.

The result is that **every message has its own unique key**, captured
ciphertext from before a compromise stays secret, and after one clean
round trip the channel is secure again even if state was leaked.

### Lease-key derivation from the ratchet

We tie the lease layer (§12) to the Signal ratchet:

```
lease_key = HKDF-Expand(ratchet_chain_key, "vault.lease." || lease_id, 32)
```

When the ratchet advances (new message), the chain key changes, and any
new lease keys derive from the new chain. Old lease keys are still
valid until their TTL — but they are scoped to the version of the
session in which they were issued. Tearing down the session
(rotating the ratchet root) invalidates them.

### Packages

| Package | Status | Purpose |
|---------|--------|---------|
| `user-identity` | new | Ed25519 + X25519 long-term keypair. |
| `device-identity` | new | Per-device sub-keypair + signature chain. |
| `device-enroll` | new | QR pairing protocol. |
| `prekey-bundle` | new | Server-side prekey storage + signed prekey rotation. |
| `x3dh` | **exists** (Rust) | X3DH handshake. See `msg-signal` spec. |
| `double-ratchet` | **exists** (Rust) | Forward-secret message ratchet. |
| `sealed-sender` | **exists** (Rust) | Hides sender identity from server. Reusable for vault metadata-hiding. |
| `signal-session` | new | Thin wrapper combining X3DH + ratchet for byte-in/byte-out vault use. |

---

## §9 Storage Backends

Storage is **untrusted by default**. The encrypted-blob layer (§10)
ensures that backends only ever see opaque ciphertext + sync metadata.
This frees us to write many backends with no security knowledge.

### The backend trait

```
trait BlobStore {
    fn put(key: BlobId, bytes: &[u8]) -> Result<Revision>;
    fn get(key: BlobId) -> Result<Option<(Revision, Vec<u8>)>>;
    fn delete(key: BlobId, if_revision: Option<Revision>) -> Result<()>;
    fn list(prefix: BlobIdPrefix, opts: ListOpts) -> Result<Vec<Stat>>;
    fn stat(key: BlobId) -> Result<Option<Stat>>;
}
```

Five operations. No knowledge of encryption, no knowledge of users, no
knowledge of entries. A `BlobId` is an opaque 256-bit identifier (32
hex chars or base32). Revisions are monotonic per blob, used for
optimistic concurrency.

### Backend portfolio

| Package | Backing store |
|---------|---------------|
| `blob-store-mem` | HashMap. For tests, ephemeral use, and as the in-process backend for the inmem transport. |
| `blob-store-fs` | One file per blob in a directory tree (sharded by prefix). |
| `blob-store-sqlite` | Single SQLite file with a `(blob_id, revision, bytes)` table. Built on the existing `storage-sqlite` package. |
| `blob-store-gdrive` | Google Drive folder. Uses Drive's REST API; OAuth handled at config. |
| `blob-store-icloud` | iCloud Drive (macOS / iOS only). File-system bridge plus iCloud sync. |
| `blob-store-webdav` | Nextcloud, ownCloud, generic WebDAV. |
| `blob-store-s3` | S3-compatible (AWS, R2, MinIO, Backblaze). |
| `blob-store-git` | Bare git repo. Each blob is one commit's tree. Useful for auditable history. |

### Decorators

| Package | Purpose |
|---------|---------|
| `blob-store-encrypted` | Wraps any backend; transparently AEAD-seals bytes via the envelope format on write, opens on read. |
| `blob-padding` | Wraps any backend; pads to size tier (1KB / 4KB / 16KB / 64KB / 256KB / next power of two) to limit length leakage. |
| `chunk-store` | Content-addressable chunking layer. Splits large entries (files) into ~1 MiB chunks, hashes each, stores under `chunk/<hash>`. Entries reference chunk hashes. |

### `vault-index` — the encrypted catalog

A pure list-by-prefix on the storage backend would leak entry names.
Instead, we store one (or a few sharded) **encrypted index blob(s)**
that maps human-readable entry name → blob ID. The index is itself a
CRDT (LWW-Map) so concurrent updates from multiple devices merge. On
unlock, the client downloads and decrypts the index; further operations
go directly to blob IDs.

Tradeoff: a backend listing call shows blob IDs but reveals nothing
about names, folders, or types.

---

## §10 Sync & CRDT

Multiple devices, sometimes offline, may concurrently write the same
entry. Three conflict-resolution strategies are configurable.

### Last-Writer-Wins (LWW)

The simplest approach. Each write carries a Lamport timestamp; the
highest wins. Loses concurrent edits. Bitwarden does this. Useful when
the user accepts data loss in exchange for simplicity.

### Vector clocks + manual resolution

Each device increments its own counter on every write. Two writes are
**concurrent** if neither's vector dominates the other. Concurrent
writes surface as a conflict the user must resolve (merge UI, like
Mercurial / Git).

### CRDTs (auto-merge, no data loss)

We provide three CRDT primitives composed per field:

- **`crdt-lww-register`** — Last-writer-wins register for scalar fields
  where the user accepts losing one of two simultaneous edits (e.g. URL).
- **`crdt-or-set`** — Observed-Remove Set for tags / labels / shared
  collections.
- **`crdt-rga`** — Replicated Growable Array for long text (notes,
  password change history). Yjs / Automerge use similar structures.

A typed entry declares per-field CRDT semantics:

```
entry-password {
    title:    crdt-lww-register<String>
    username: crdt-lww-register<String>
    password: crdt-lww-register<String>   // editing both at once → LWW
    notes:    crdt-rga<String>            // collaborative text merge
    tags:     crdt-or-set<String>         // concurrent add/remove merge
    history:  crdt-or-set<{ts, value}>    // append-only
}
```

### Packages

| Package | Purpose |
|---------|---------|
| `crdt-lww-register` | LWW scalar. |
| `crdt-or-set` | Observed-remove set. |
| `crdt-rga` | Replicated growable array (text). |
| `vault-sync` | Trait for sync strategies. |
| `vault-sync-lww` | Lamport-clock LWW. |
| `vault-sync-vclock` | Vector-clock with explicit conflicts. |
| `vault-sync-crdt` | Compose CRDT primitives per entry field. |

---

## §11 Typed Entries

The vault stores **typed entries**, each a discriminated union variant
on a versioned schema. Entries are serialized to CBOR, encrypted via
the envelope, stored as opaque blobs.

| Package | Variant | Notable fields |
|---------|---------|----------------|
| `entry-schema` | enum + version registry | discriminator + version |
| `entry-password` | password | username, password, url, notes (CRDT), tags, history |
| `entry-note` | note | title, body (CRDT text), attachments |
| `entry-file` | file | filename, mime, size, chunk-refs (chunk-store) |
| `entry-card` | card | issuer, number (PCI care!), expiry, cvv |
| `entry-identity` | identity | name, address, phone, email, dob |
| `entry-ssh-key` | ssh keypair | private key (encrypted), public key, comment, KDF metadata |
| `entry-totp` | OTP seed | issuer, label, secret (base32), digits, period, algo |
| `entry-passkey` | discoverable credential | rpId, credentialId, privKey, userHandle, signCount |
| `entry-api-key` | generic API key | service, key, secret, scopes, expiry |
| `entry-custom` | user-defined | freeform CBOR map |

Schema evolution: each entry header carries a (discriminator, version)
tuple. Readers refuse unknown discriminators with a "newer-version"
error rather than silently dropping fields.

---

## §12 Leases

A **lease** is a time-bounded capability over a secret. When you read a
secret, the server doesn't hand you the plaintext directly; it hands
you ciphertext under a fresh **lease key** with a TTL. The consumer
holds the lease key only for the lease's lifetime; expiry zeros the
key, rendering the ciphertext unreadable.

### Why leases?

- **Bounded blast radius.** If the consumer is compromised, only the
  outstanding (un-expired) leases are at risk.
- **Decoupling read from use.** A secret can be re-issued under a fresh
  lease without rotating the underlying credential.
- **Revocation.** The server can broadcast "revoke lease L"; consumers
  drop the key immediately.
- **Audit.** Every lease grant is logged; lease use is implicit but
  bounded.

### Lease lifecycle

```
1. Consumer: GetEntry(name)
2. Server:   issue lease L = {id, ttl, revocation_token}
             lease_key = HKDF(signal_chain_key, "vault.lease." || id)
             body = AEAD-encrypt(plaintext, lease_key)
             return {lease=L, body}
3. Consumer: stash lease_key (derived same way from local ratchet)
             decrypt body
             use plaintext
4. Consumer: at TTL → zero lease_key. body is now unreadable.
5. Server (anytime): RevokeLease(L) → broadcast → consumer drops key.
```

### Lease key derivation

Tied to the Signal ratchet (§8). Lease key = HKDF over the current
chain key + lease ID. This means:

- A new ratchet step makes the chain key fresh; future leases use the
  new chain.
- Old leases remain valid until TTL but are anchored in the old chain.
- Tearing down the session (root rotation) invalidates the chain →
  any captured ciphertext-with-lease-key from before tear-down is
  protected by forward secrecy.

### Packages

| Package | Purpose |
|---------|---------|
| `lease` | Lease ID, TTL, revocation token. |
| `lease-store` | Server-side registry, expiry, revocation broadcast. |
| `lease-wrap` | AEAD wrap response under derived lease key. |
| `lease-client` | Consumer-side: register lease, schedule expiry, zero key. |

---

## §13 OTP

The simplest chapter. RFC 4226 (HOTP) and RFC 6238 (TOTP) are tiny
constructions on top of HMAC.

### HOTP

```
HOTP(K, C) = truncate(HMAC-SHA1(K, C)) mod 10^digits
```

Where:

- `K` is a shared secret (typically 20 bytes).
- `C` is a 64-bit counter.
- `truncate` does **dynamic offset truncation**: take the low 4 bits of
  the last byte of the HMAC as an offset; read 4 bytes starting at that
  offset; mask the high bit (to avoid signed-int issues); that gives a
  31-bit integer.
- `digits` is typically 6.

### TOTP

```
TOTP(K, t) = HOTP(K, floor((t - T0) / period))
```

Where `t` is Unix time, `T0` is usually 0, and `period` is usually 30
seconds.

### otpauth:// URI

```
otpauth://totp/Issuer:label?secret=BASE32SECRET&issuer=Issuer&period=30&digits=6&algorithm=SHA1
```

This is what QR codes encode. Parsing and emitting these is its own
small package because every authenticator app needs it.

### Packages

| Package | Purpose |
|---------|---------|
| `hotp` | RFC 4226. ~50 LOC. |
| `totp` | RFC 6238 wrapping HOTP. ~30 LOC. |
| `otpauth-uri` | `otpauth://` parse + emit. |

---

## §14 Hardware Keys & Biometrics

Biometrics are **not crypto**. They are **gates** on hardware keys
that already exist in the Secure Enclave (Apple), TPM (Windows /
Linux), or Windows Hello key store.

### The model

```
            ┌──────────────────────┐
  caller ──▶│ biometric-gate       │── prompts user for TouchID/etc
            │  if (gate passes):   │
            │    use hardware-key  │
            └──────────┬───────────┘
                       ▼
            ┌──────────────────────┐
            │ hardware-key (trait) │
            │   sign(challenge)    │
            │   hmac(salt)         │
            └──────────┬───────────┘
                       ▼
       ┌────────────────────────────────────┐
       │ se / tpm / wh implementations      │
       │  key material never leaves chip    │
       └────────────────────────────────────┘
```

### Apple Secure Enclave

- Use `LocalAuthentication` framework: `LAContext.evaluatePolicy(
  .deviceOwnerAuthenticationWithBiometrics, ...)`.
- Create a `SecKey` with `kSecAttrTokenIDSecureEnclave` plus a
  `SecAccessControl` requiring biometric.
- The key is P-256 only on Secure Enclave. ECDH and ECDSA only.
- The private key never leaves the chip; you pass challenges in, get
  signatures out.

### Windows Hello / TPM

- TPM 2.0 generates and stores keys; Windows Hello provides the
  biometric gate. Use Windows `Cryptography.Core.AsymmetricKeyAlgorithm`
  with `KeyProtectionLevel.ConsentWithFingerprint` etc.
- Linux: `tpm2-tss` directly; no biometric gate, but a passphrase or
  PIN gate via `tpm2_unseal`.

### Use in the vault

A hardware-key-backed master key:

1. On first setup, vault generates a 32-byte KEK.
2. Vault wraps the KEK using the hardware key's HMAC (CTAP `hmac-secret`
   or TPM `Unseal`).
3. To unlock: prompt biometric → hardware key derives wrapping key →
   unwrap KEK.

Optionally combined with passphrase: `KEK = HKDF(hw_secret, passphrase)`.
This requires both the device and the passphrase to unlock — the second
factor is genuinely two-factor.

### Packages

| Package | Purpose |
|---------|---------|
| `hardware-key` | Trait: `sign(challenge)`, `hmac(salt)`. |
| `hardware-key-se` | Apple Secure Enclave (Swift native). |
| `hardware-key-tpm` | TPM 2.0 (Linux/Windows). |
| `hardware-key-wh` | Windows Hello. |
| `biometric-gate` | Policy: "require biometric to use this key." |

---

## §15 FIDO2 / CTAP2

Hardware security keys (YubiKey, Solo, Google Titan) speak **CTAP2**
over USB-HID, NFC, or BLE. CTAP2 is what WebAuthn calls underneath when
you tap a hardware key in a browser.

### Primer on CTAP2

CTAP2 is a request/response protocol. The host sends a CBOR-encoded
command; the authenticator returns a CBOR-encoded response.

Key commands:

- `authenticatorGetInfo` — capabilities, supported algorithms.
- `authenticatorMakeCredential` — create a new credential. Inputs:
  challenge, RP info, user info, pubkey-algorithms list. Output: COSE
  public key + attestation statement.
- `authenticatorGetAssertion` — sign a challenge with an existing
  credential. Output: signed `authenticatorData || clientDataHash`.
- `authenticatorClientPIN` — PIN management.

### The hmac-secret extension

The KEK-derivation superpower. With this extension:

1. During credential creation, the authenticator generates a per-credential
   secret S, internal to the device.
2. During an assertion, the host can ask: "compute HMAC(S, salt)" and
   get a deterministic output. The same salt always produces the same
   output, but only when this device is plugged in.

Use this to derive a vault KEK from a hardware key:

```
hw_kek = HMAC(per_credential_secret, fixed_salt)
vault_kek = HKDF(hw_kek || passphrase_kek, "vault.kek")
```

Now unlocking the vault requires both the passphrase and the physical
key. ssh-keygen, `age`, and Bitwarden's hardware unlock all use this.

### The wire — CTAP2 over USB HID

CTAP2 frames live inside CTAP-HID frames:

```
CTAP-HID frame:
  channel_id: u32     // assigned by INIT
  cmd:        u8      // 0x10 = MSG (carries a CTAP2 command)
  bcnt:       u16     // body byte count
  payload:    [u8]    // body, possibly chunked across frames
```

Long messages span multiple HID reports, with continuation frames
identified by sequence numbers.

### Packages

| Package | Purpose |
|---------|---------|
| `ctap2-protocol` | Message types (Make/GetAssertion/GetInfo, ClientPIN). |
| `ctap2-transport` | Trait. |
| `ctap2-transport-hid` | USB HID (most common). |
| `ctap2-transport-nfc` | NFC. |
| `ctap2-transport-ble` | Bluetooth LE. |
| `ctap2-client` | High-level operations. |
| `hmac-secret` | The CTAP extension for KEK derivation. |

---

## §16 WebAuthn & Passkeys

WebAuthn is the W3C browser API on top of CTAP2. Same crypto, exposed
to JavaScript. Passkeys are WebAuthn discoverable credentials that the
platform syncs across devices.

### The two ceremonies

**Registration (attestation):**

```
RP server         Browser/JS              Authenticator (CTAP2)
   │                  │                          │
   │── challenge ────▶│                          │
   │                  │── makeCredential ───────▶│
   │                  │                          │ (user gesture: touch/biometric)
   │                  │◀── attestation object ───│
   │◀── publicKey,    │                          │
   │    attestation   │                          │
   │                  │                          │
verify attestation
store publicKey
associated to user
```

**Authentication (assertion):**

```
RP server         Browser/JS              Authenticator
   │                  │                          │
   │── challenge ────▶│                          │
   │                  │── getAssertion ─────────▶│
   │                  │                          │ (user gesture)
   │                  │◀── authenticatorData,    │
   │                  │    clientDataJSON,       │
   │                  │    signature             │
   │◀── signed bundle │                          │
verify signature
against stored pubkey
```

### Key data structures

- **`AuthenticatorData`** — binary: `rpIdHash || flags || signCount || (attestedCredentialData?) || (extensions?)`.
- **`ClientDataJSON`** — JSON with `type`, `challenge`, `origin`, `crossOrigin`. The browser builds this; the authenticator never sees the origin directly.
- **`AttestationObject`** — CBOR map with `fmt`, `attStmt`, `authData`. Different `fmt` values (packed, fido-u2f, tpm, none) give different attestation chains.

### Passkeys

Passkeys are discoverable credentials (the authenticator can list them
without the RP providing a credential ID) that the platform syncs. The
crypto is identical to FIDO2; the difference is the private key is
syncable rather than hardware-bound.

Sync mechanisms:

- **Apple iCloud Keychain** — end-to-end encrypted via iCloud Keychain
  syncing protocol.
- **Google Password Manager** — synced via Google account.
- **1Password / Bitwarden / Dashlane** — vault-as-passkey-provider.

**Our vault is a passkey provider.** A `passkey-provider` integration
with the OS lets the system AutoFill flows present our passkeys.

### Packages

| Package | Purpose |
|---------|---------|
| `webauthn-types` | AuthenticatorData, ClientDataJSON, AttestationObject. |
| `webauthn-rp` | Relying-party server-side: generate challenges, verify assertions, store pubkeys. |
| `webauthn-authenticator` | Implement an authenticator. The vault becomes one. |
| `passkey-store` | Sync-friendly storage of discoverable credentials (uses entry-passkey + CRDT sync). |
| `passkey-provider` | OS integration: macOS AutoFill provider, Android CredentialProvider, Windows. |

---

## §17 Emergency & Recovery

The hardest chapter, because the timer is the hard part of any
emergency-access scheme.

### Designation flow (recap from braindump)

1. **Designation:** A picks B as emergency contact, wait period T.
   - A generates EAK (Emergency Access Key).
   - A wraps relevant vault keys with EAK.
   - A wraps EAK with B's X25519 public key.
   - A signs the whole designation.
2. **Invocation:** B sends a signed request. A timer starts.
3. **Race:** A can sign a denial before T expires.
4. **Grant:** if no denial, B's wrapped EAK becomes available.

The hard part is the timer.

### Timer oracle — three implementations

**`timer-oracle-trusted`** — one server vouches for time. Simplest,
but the server can lie. Acceptable when the server is the user's own
infra (Chief of Staff scenario).

**`timer-oracle-witness-committee`** — K-of-N witness servers co-sign
"T has elapsed since invocation." Each witness sees only encrypted
designation metadata + signed timestamps. Compromise of fewer than K
witnesses cannot fake elapsed time. Decentralized trust.

**`timer-oracle-drand`** — Verifiable Delay Function via the drand
network. A encrypts the EAK to a future drand round R ≈ now + T. The
drand network publicly reveals decryption material at round R; nobody
can cheat time. Clean cryptographic guarantee, but ties the system to
an external public-good network.

### Verifiable Delay Functions

A VDF is a function `f(x)` that:

- Takes time T to compute, regardless of parallelism.
- Produces a proof that `y = f(x)` was computed correctly, verifiable
  in `O(log T)`.

Combined with timed encryption (drand-style threshold BLS), you get
ciphertext that becomes decryptable at a specific future time, without
trusting any single party.

### Social recovery (separate from emergency)

For a lost passphrase, split the recovery key with Shamir:

```
recovery_key = HKDF(master_key, "vault.recovery")
shares       = shamir-split(recovery_key, k=3, n=5)
for each contact i:
    wrap shares[i] with contact[i]'s X25519 public key
    publish wrapped share to vault
```

To recover: 3 of 5 contacts authorize → reconstruct → unwrap → unlock.

### N-of-M emergency = Shamir + timer

Combine: split EAK with Shamir. Each contact gets a wrapped share.
Recovery requires K wrapped shares **AND** the timer oracle confirming
T elapsed. "Any 3 of my 5 friends, after 7 days, can recover."

### Packages

| Package | Purpose |
|---------|---------|
| `timer-oracle` | Trait. |
| `timer-oracle-trusted` | One trusted timer service. |
| `timer-oracle-witness-committee` | K-of-N co-signed timestamps. |
| `vdf` | Verifiable delay function primitive. |
| `timer-oracle-drand` | VDF / drand-backed timer. |
| `emergency-designation` | Single-contact, timer-gated. |
| `emergency-mpc` | N-of-M (shamir + timer). |
| `social-recovery` | Shamir-based passphrase recovery. |
| `vault-seal-shamir` | HashiCorp-style N-of-M unseal. |

---

## §18 Sharing, Multi-User, Audit

### Sharing

To share entry E with user B:

1. Look up E's existing wrap list.
2. Add `{recipient: B.pubkey, wrapped: AEAD(E.dek, B.pubkey-derived)}`.
3. Re-upload the envelope (only the wrap list changed; body
   ciphertext is unchanged).

That's it. No special "share protocol." Sharing == growing the wrap
list. The recipient discovers shared entries via the encrypted
`vault-index` (which now lists this entry's blob ID for them).

### Revocation

To revoke B's access to E:

1. Generate a fresh DEK for E.
2. Re-encrypt body under fresh DEK.
3. Build wrap list for surviving recipients only.

This is more expensive than sharing (body re-encryption), but rare.
Revoked B retains any cached plaintext; this is unavoidable.

### Invitation (joining the vault)

For B's first introduction to the vault, we need to bootstrap
their X25519 public key into A's view of the world without trusting the
server to map "B" → pubkey honestly.

Signal-style **safety numbers** solve this: A and B compute a hash
fingerprint of each other's identity public keys and compare them
out-of-band (in person, via a side channel). Match = trust the binding.

### Groups

A "group" is a named recipient set: `{group_id: "design-team",
members: [user_pubkey, ...]}`. Sharing to a group expands to wrapping
for each member. Adding a new member triggers re-wrap of the group's
entries on next access.

### Audit log

Append-only, signed. Every operation that changes vault state appends:

```
audit_entry = sign(ed25519_signer_key,
                   {ts, actor, op, target, before_revision, after_revision})
prev_link   = hash(previous_audit_entry)
```

A hash chain (`prev_link`) gives tamper-evidence — any backwards
modification breaks the chain. The audit log is itself stored as
encrypted blobs, sharded by date. Auditors get read-only access via a
specific role.

### Packages

| Package | Purpose |
|---------|---------|
| `vault-share` | Add recipient to entry's wrap list. |
| `vault-revoke` | Rotate DEK + rebuild wrap list. |
| `vault-invite` | OOB safety-number verification. |
| `user-store` | Principal registry. |
| `group-store` | Named recipient sets. |
| `vault-audit` | Append-only signed log with hash chain. |

---

## §19 Vault Core, Engines, Auth Methods

The middle layer that ties storage, sharing, leases, and entries
together into a service.

### vault-protocol

A discriminated-union request/response type. Pure data, no I/O. CBOR-
encoded on the wire.

```
Request =
  | Unlock { passphrase | webauthn-assertion | hw-key-response }
  | Lock
  | Get { name }
  | List { prefix, limit }
  | Put { name, entry }
  | Delete { name }
  | Search { query }
  | Share { entry-id, recipient }
  | Revoke { entry-id, recipient }
  | LeaseRevoke { lease-id }
  | EmergencyInvoke { designation-id }
  | EmergencyDeny { designation-id }
  | ...
```

### vault-service

Takes one request, runs it through the pipeline, returns one response.
Holds:

- The currently-unlocked KEK (or none if locked).
- The lease store.
- The audit log writer.
- Configured secret engines and auth methods.

### vault-session

Per-client session state on the server: identity, capabilities,
unlocked KEK reference, signal-session state, idle timer, lease set.

### Secret engines

A secret engine is a pluggable backend that **produces secrets** in
response to requests. HashiCorp's design.

- **`secret-engine-kv`** — static key/value. The default, used by most
  password-manager features.
- **`secret-engine-db`** — generates **dynamic** database credentials.
  Vault holds DB admin credentials; on request, generates a fresh
  user, returns it under a lease, deletes the user when the lease
  expires.
- **`secret-engine-pki`** — issues short-lived TLS certificates from a
  CA.
- **`secret-engine-transit`** — encryption-as-a-service. Vault holds
  encryption keys; clients send plaintext, get ciphertext (or vice
  versa). No secret storage; the secrets stay client-side.
- **`secret-engine-totp`** — server-side TOTP code generation
  (Vault's `totp` engine: stores the seed, returns the current code).

### Auth methods

How a client proves identity to obtain a session token.

- **`auth-method-token`** — bearer token (root tokens, child tokens).
- **`auth-method-passphrase`** — passphrase + KDF.
- **`auth-method-approle`** — role_id + secret_id (for service
  accounts).
- **`auth-method-oidc`** — OAuth2 / OpenID Connect.
- **`auth-method-webauthn`** — WebAuthn assertion against a registered
  credential.

### Policy

Path-based ACL, HashiCorp-style:

```
path "secret/data/team-design/*" {
    capabilities = ["read", "list"]
}

path "pki/issue/web" {
    capabilities = ["update"]
    allowed_parameters = { common_name = ["*.example.com"] }
}
```

The ACL evaluator walks the path tree, finds the most-specific match,
checks capabilities, validates parameter constraints.

### Packages

| Package | Purpose |
|---------|---------|
| `vault-protocol` | Request/response sum types. |
| `vault-service` | Dispatcher. |
| `vault-session` | Per-client session state. |
| `vault-lock` | Lifecycle, secure memory zeroing. |
| `vault-acl` | Path-based ACL. |
| `vault-audit` | Audit log (covered in §18). |
| `secret-engine` (+ kv/db/pki/transit/totp) | Engines. |
| `auth-method` (+ token/passphrase/approle/oidc/webauthn) | Auth methods. |

---

## §20 Transports

Each transport is a thin adapter: bytes-in → `vault-service` → bytes-out.

### Wire framing per transport

| Transport | Framing |
|-----------|---------|
| `vault-transport-inmem` | Function call. No framing. |
| `vault-transport-stdio` | 4-byte LE length prefix + CBOR body. |
| `vault-transport-uds` | Same as stdio over Unix socket. |
| `vault-transport-tcp` | Length-prefix + TLS. |
| `vault-transport-http` | JSON-RPC 2.0 over HTTPS, body is base64-CBOR. |
| `vault-transport-grpc` | gRPC service generated from CBOR schema. |
| `vault-transport-nmh` | WebExtension Native Messaging: 4-byte LE length + JSON or base64-CBOR. |

### Signal-session decorator

`vault-transport-secured` wraps any transport. It performs X3DH on
session establishment, then encrypts every subsequent vault-protocol
frame with the double ratchet. The underlying transport sees only
opaque ratcheted bytes.

This is **defense in depth** — even if TLS is MITM'd or the
transport is plaintext, the vault payload remains E2E-encrypted between
the actual client and the actual vault service.

### Browser extension specifically

Native Messaging Hosts run as child processes of the browser. The
manifest declares which extension may launch them. Communication is
stdio, framed by 4-byte LE length, with JSON bodies (per Chrome /
Firefox spec).

We carry CBOR base64'd inside the JSON envelope so the transport stays
spec-compliant while we keep our canonical wire format.

### Packages

| Package | Purpose |
|---------|---------|
| `vault-transport` | Trait. |
| `vault-transport-inmem` | In-process. |
| `vault-transport-stdio` | stdio + length-prefixed CBOR. |
| `vault-transport-uds` | Unix socket. |
| `vault-transport-tcp` | TCP + TLS. |
| `vault-transport-http` | HTTP/JSON-RPC. |
| `vault-transport-grpc` | (optional) gRPC. |
| `vault-transport-nmh` | Browser Native Messaging. |
| `vault-transport-secured` | signal-session decorator. |

---

## §21 Importers / Exporters

Reading legacy formats lets users migrate. Writing them lets users
escape.

| Package | Format |
|---------|--------|
| `import-bitwarden` | Bitwarden JSON export (encrypted or unencrypted). |
| `import-1password` | 1PUX (1Password export bundle). |
| `import-kdbx` | KeePass KDBX 4. |
| `import-lastpass` | LastPass CSV / encrypted XML. |
| `import-csv` | Generic CSV with column mapping. |
| `import-chrome` | Chrome's CSV password export. |
| `import-safari` | Safari's CSV export. |
| `export-csv` | Generic CSV (warns about plaintext). |
| `export-bitwarden` | Bitwarden-compatible JSON. |
| `export-kdbx` | KDBX. |

Each importer reads, decrypts (using the source format's KDF + AEAD),
and emits typed entries that flow through the standard pipeline.
Bitwarden uses PBKDF2 + AES-CBC + HMAC; KDBX uses Argon2 + ChaCha20 or
AES; 1PUX uses PBKDF2 + AES-GCM. Implementing them exercises the
crypto-agile envelope nicely.

---

## §22 Roadmap (PR Order)

The package count is large. Implementation must proceed in waves where
each wave is independently valuable and testable.

### Wave 0 — Foundations (mostly exist)

Already present: SHA-2/3, HMAC, HKDF, Argon2id, AES-GCM,
ChaCha20-Poly1305, X25519, Ed25519. Add: **P-256 ECDSA + ECDH**,
**CBOR**, **COSE-Key**.

### Wave 1 — Envelope & wrapping

`envelope`, `aead-wrap`, `kdf-params`, `master-key`, `key-hierarchy`,
`recipient-wrap`, `shamir`. Combined with VLT01 (existing sealed store)
this gives single-user envelope-encrypted records.

### Wave 2 — Storage portfolio

`blob-store` trait + `blob-store-{mem,fs,sqlite}` first.
`blob-store-encrypted`, `blob-padding`, `chunk-store`, `vault-index`.
Cloud backends (gdrive/icloud/s3/webdav/git) follow as separate PRs.

### Wave 3 — Typed entries + lock/session

`entry-schema` + `entry-{password,note,totp}` (the MVP triad).
`vault-lock`, `vault-session`. Combined with Wave 1 + 2 gives an
in-process single-user vault.

### Wave 4 — OTP

`hotp`, `totp`, `otpauth-uri`. Tiny, parallelizable.

### Wave 5 — Protocol + simplest transport + CLI

`vault-protocol`, `vault-service`, `vault-transport-inmem`,
`vault-transport-stdio`, `vault-client-cli`. First user-facing usable
artifact: a CLI vault.

### Wave 6 — Sync foundation

`crdt-lww-register`, `crdt-or-set`, `crdt-rga`, `vault-sync` trait,
`vault-sync-lww`. Basic sync between two devices over stdio.

### Wave 7 — Identity, signal, leases

`user-identity`, `device-identity`, `device-enroll`, `prekey-bundle`,
`x3dh`, `double-ratchet`, `signal-session`. Then `lease`, `lease-store`,
`lease-wrap`, `lease-client`. Multi-device with E2E + leased reads.

### Wave 8 — Network transports

`vault-transport-tcp`, `vault-transport-http`, `vault-transport-secured`.
Network-accessible vault.

### Wave 9 — Cloud storage

`blob-store-{gdrive,icloud,webdav,s3,git}`. Sync to cloud.

### Wave 10 — Sharing & multi-user

`user-store`, `group-store`, `vault-share`, `vault-revoke`,
`vault-invite`, `vault-acl`, `vault-audit`.

### Wave 11 — Emergency & recovery

`timer-oracle` + `timer-oracle-trusted` first; `social-recovery`,
`vault-seal-shamir`, `emergency-designation`, `emergency-mpc`.
`timer-oracle-witness-committee` and `vdf` + `timer-oracle-drand` later.

### Wave 12 — HashiCorp parity

`secret-engine` trait + `secret-engine-{kv,db,pki,transit,totp}`,
`auth-method` trait + `auth-method-{token,passphrase,approle,oidc,webauthn}`.

### Wave 13 — Advanced sync

`vault-sync-vclock`, `vault-sync-crdt`. Per-field CRDT semantics for
typed entries.

### Wave 14 — Hardware keys

`hardware-key` trait + `hardware-key-{se,tpm,wh}`, `biometric-gate`.
`ctap2-{protocol,transport-hid,transport-nfc,transport-ble,client}`,
`hmac-secret`.

### Wave 15 — WebAuthn & passkeys

`webauthn-{types,rp,authenticator}`, `passkey-store`,
`passkey-provider`. Vault becomes a passkey provider.

### Wave 16 — Browser extension

`vault-transport-nmh`, `vault-client-webext`. Browser autofill.

### Wave 17 — Importers / exporters

`import-{bitwarden,1password,kdbx,lastpass,csv}`, exporters.

### Wave 18 — Desktop app

`vault-client-tauri` (Tauri shell with the webext UI).

---

## §23 Interop Targets

A specific list of "we should be able to do this" goals to validate
against:

- [ ] Read and write a Bitwarden export bundle.
- [ ] Read and write a KDBX 4 file.
- [ ] Run as an SSH-AGENT-compatible socket holding ssh keys.
- [ ] Implement a HashiCorp-compatible REST API (subset) with the kv
      and transit engines, usable via the `vault` CLI.
- [ ] Pass the FIDO2 conformance test as an authenticator.
- [ ] Pass WebAuthn L3 conformance as a relying party.
- [ ] Provide passkeys to macOS Safari AutoFill.
- [ ] Generate dynamic Postgres credentials with leases.
- [ ] Sync a vault between two devices via Google Drive only — the
      Drive owner cannot read content.
- [ ] Recover a lost passphrase via 3-of-5 social recovery contacts.
- [ ] Hand emergency access to a designated contact after a 7-day
      wait, defeated by the owner signing a denial.

---

## §24 Non-Goals (for now)

- Post-quantum cryptography. Migration plan deferred.
- Plausible-deniability mode (multiple decoy passphrases unlocking
  different vaults).
- On-disk full-vault encrypted search (we use in-memory search after
  unlock; encrypted search is a research topic).
- Federated user directory (each vault is its own world; users
  introduce each other via OOB invitation).
- Hardware Security Module backends beyond TPM/SE/Hello (no Nitrokey,
  no SmartCard yet; deferred until requested).
- Mobile native apps (handled by Tauri / passkey-provider integrations
  for now; first-class iOS/Android apps later).

---

## §25 References

### Specifications

- RFC 8949 — Concise Binary Object Representation (CBOR)
- RFC 8152 — CBOR Object Signing and Encryption (COSE)
- RFC 5869 — HMAC-based Extract-and-Expand Key Derivation Function
- RFC 4226 — HOTP
- RFC 6238 — TOTP
- RFC 7748 — Curve25519 / X25519
- RFC 8032 — EdDSA / Ed25519
- RFC 9106 — Argon2
- RFC 8439 — ChaCha20 and Poly1305
- W3C WebAuthn Level 3
- FIDO Alliance CTAP 2.2
- NIST FIPS 186-5 — ECDSA (P-256)

### Inspiration / Prior Art

- HashiCorp Vault — secret engines, dynamic credentials, leases, ACL
  policies, Shamir unseal.
- 1Password — emergency kit, secret key, account recovery.
- Bitwarden — sharing model, organizations, emergency access.
- KeePass / KDBX — file format, KDF parameters, attachments.
- age — recipient-wrapping pattern, modern envelope format.
- Signal Protocol — X3DH + Double Ratchet, safety numbers.
- ssh-agent — socket-based key holding, agent forwarding.
- drand — distributed randomness beacon, threshold BLS, time-lock.

### Internal references

- VLT01 — vault sealed store (existing implementation of envelope
  encryption layer; this spec is authoritative for that layer).
- msg-signal — Signal Protocol spec (X3DH, Double Ratchet, Sealed Sender);
  the existing `x3dh`, `double-ratchet`, `sealed-sender` Rust packages
  are this spec's implementation.
- D18A — Chief of Staff stores (the agent-facing record/blob/lease
  abstraction this vault eventually lives behind).
- HF01..HF06 — hash function specs.
- hkdf, argon2id, x25519, ed25519, aes-modes, chacha20-poly1305 specs.
- storage-sqlite — existing SQLite backend, the `blob-store-sqlite`
  prerequisite.

---

## Appendix A — Full Package Index

Numbered by chapter for easy lookup. ~95 packages at full vision; ~30
for MVP single-user CLI.

```
§5   Crypto leaves (mostly exist)
       p256-ecdsa, p256-ecdh

§6   Encoding
       cbor, cose-key, cose-sign, cose-encrypt
       base32, base64, base58, der-asn1

§7   Envelope & wrapping
       envelope, aead-wrap, kdf-params, master-key, key-hierarchy,
       recipient-wrap, shamir

§8   Identity & secure channel
       user-identity, device-identity, device-enroll, prekey-bundle,
       x3dh, double-ratchet, signal-session

§9   Storage
       blob-store (trait)
       blob-store-mem, blob-store-fs, blob-store-sqlite,
       blob-store-gdrive, blob-store-icloud, blob-store-webdav,
       blob-store-s3, blob-store-git
       blob-store-encrypted, blob-padding, chunk-store, vault-index

§10  Sync & CRDT
       crdt-lww-register, crdt-or-set, crdt-rga
       vault-sync, vault-sync-lww, vault-sync-vclock, vault-sync-crdt

§11  Typed entries
       entry-schema
       entry-password, entry-note, entry-file, entry-card,
       entry-identity, entry-ssh-key, entry-totp, entry-passkey,
       entry-api-key, entry-custom

§12  Leases
       lease, lease-store, lease-wrap, lease-client

§13  OTP
       hotp, totp, otpauth-uri

§14  Hardware keys
       hardware-key, hardware-key-se, hardware-key-tpm, hardware-key-wh
       biometric-gate

§15  FIDO2 / CTAP2
       ctap2-protocol, ctap2-transport,
       ctap2-transport-hid, ctap2-transport-nfc, ctap2-transport-ble,
       ctap2-client, hmac-secret

§16  WebAuthn / Passkeys
       webauthn-types, webauthn-rp, webauthn-authenticator,
       passkey-store, passkey-provider

§17  Emergency & recovery
       timer-oracle,
       timer-oracle-trusted, timer-oracle-witness-committee,
       vdf, timer-oracle-drand,
       emergency-designation, emergency-mpc, social-recovery,
       vault-seal-shamir

§18  Sharing & audit
       vault-share, vault-revoke, vault-invite,
       user-store, group-store, vault-audit

§19  Vault core / engines / auth
       vault-protocol, vault-service, vault-session, vault-lock,
       vault-acl,
       secret-engine,
         secret-engine-kv, secret-engine-db, secret-engine-pki,
         secret-engine-transit, secret-engine-totp,
       auth-method,
         auth-method-token, auth-method-passphrase, auth-method-approle,
         auth-method-oidc, auth-method-webauthn

§20  Transports
       vault-transport,
       vault-transport-inmem, vault-transport-stdio,
       vault-transport-uds, vault-transport-tcp, vault-transport-http,
       vault-transport-grpc, vault-transport-nmh,
       vault-transport-secured

§21  Import / export
       import-bitwarden, import-1password, import-kdbx,
       import-lastpass, import-csv, import-chrome, import-safari
       export-csv, export-bitwarden, export-kdbx

§22  Clients
       vault-client (reference SDK), vault-client-cli,
       vault-client-tauri, vault-client-webext
```

---

## Appendix B — Spec Numbering Plan

| ID | Spec |
|----|------|
| VLT00 | Master (this document) |
| VLT01 | Sealed store (exists) |
| VLT02 | Envelope format + algo registry |
| VLT03 | Recipient wrapping + sharing |
| VLT04 | Vault index (encrypted catalog) |
| VLT05 | Lease layer |
| VLT06 | Signal session integration |
| VLT07 | Vault protocol + service + session |
| VLT08 | Sync strategies (LWW / vclock / CRDT) |
| VLT09 | Typed entry catalog |
| VLT10 | Audit log format |
| VLT11 | Sharing, revocation, invitation |
| VLT12 | Emergency access (designation + timer oracles) |
| VLT13 | Social recovery + Shamir unseal |
| VLT14 | Hardware-key abstraction |
| VLT15 | CTAP2 / FIDO2 |
| VLT16 | WebAuthn / passkeys |
| VLT17 | Secret engines |
| VLT18 | Auth methods |
| VLT19 | Transports |
| VLT20 | Browser extension protocol |
| VLT21 | Import/export formats |
| VLT22 | CRDT primitives detail |
| VLT23 | VDF + drand integration |

Per-package specs (`name.md` with no VLT prefix) for individual leaves
follow the existing per-primitive convention (`hkdf.md`, `ed25519.md`).

---

*End of VLT00.*
