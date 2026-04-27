# VLT00 — Vault Roadmap

## Purpose

The Vault stack is a **generic, app-agnostic primitive for building
password-manager-class applications**. It is not itself a password
manager. Instead, it is a reusable layered library that lets us (or
anyone) implement a 1Password-, Bitwarden-, or KeePassXC-style product
on top of it, without re-deriving the cryptographic plumbing each time.

This document maps the full Vault architecture end-to-end. Each layer
gets its own detailed spec (`VLT01`, `VLT02`, …); this roadmap is the
table of contents, the dependency graph, and the design rationale for
the ordering.

### Scope — what a "Bitwarden-class" vault needs

To stand up something that genuinely competes with Bitwarden or
1Password, the underlying primitive has to support at minimum:

1. **At-rest encryption** of every secret, strong against an attacker
   who walks off with the disk.
2. **Typed records** — passwords, secure notes, credit cards, SSH
   keys, TOTP seeds, identity documents, arbitrary JSON — with a
   stable codec layer so apps don't reinvent it.
3. **Sharing** — a secret created by Alice can be read by Bob without
   either of them handing the plaintext to the server or to each other
   over an insecure channel.
4. **Multi-device** — the same account on a laptop, a phone, and a
   browser extension, each with its own device key, any of which can
   be revoked without re-encrypting the whole vault.
5. **Recovery** — if the user forgets their password, there is a
   well-defined (and optional) recovery path. If they don't opt in,
   the vault is unrecoverable — which is correct.
6. **Sync** — a server that holds only ciphertext, can synchronise
   deltas across devices, and cannot decrypt anything. Offline work
   merges cleanly when the device comes back online.
7. **Attachments** — binary blobs (scans, images, files) attached to
   items, encrypted with the same guarantees and streamable.
8. **Revision history** — prior versions of an item are preserved and
   readable, so a user who accidentally overwrites a password can roll
   back.
9. **Search** — find "github" across 5 000 items without shipping the
   plaintext to the server and without decrypting every record on
   every keystroke.
10. **Audit log** — a tamper-evident record of who read/wrote what,
    important for shared / team vaults.
11. **Import / export** — ingest from 1Password / Bitwarden / KeePass
    exports and emit our own portable format.

Anything beyond that (browser autofill, OS keychain integration,
mobile app, organisation billing, breach-password monitoring) belongs
in the **application**, not in the Vault primitive. The split is kept
deliberately narrow: the Vault is a library, the product is what you
build on top of it.

## Layer map

Each layer is one spec and (usually) one Rust crate. Higher layers
depend only on layers beneath them.

```text
  ┌──────────────────────────────────────────────────────────────┐
  │  Application (your Bitwarden clone, your 1Password clone, …) │
  └──────────────────────────────────────────────────────────────┘
                                 │
  ┌──────────────────────────────────────────────────────────────┐
  │  VLT09  import / export (1Password 1pux, Bitwarden json, …)  │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT08  audit log — tamper-evident who-did-what              │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT07  encrypted search index                               │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT06  revision history / version stream                    │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT05  attachments — streamable encrypted blobs             │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT04  secure sync channel — server holds ciphertext only   │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT03  multi-KEK wrapping — sharing, devices, recovery      │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT02  typed record codecs — logins, notes, cards, TOTP, …  │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT01  sealed store — per-record envelope AEAD   ◄── SHIPPED │
  ├──────────────────────────────────────────────────────────────┤
  │  storage-core  — opaque KV with CAS (sqlite, folder, memory) │
  └──────────────────────────────────────────────────────────────┘
                                 │
  ┌──────────────────────────────────────────────────────────────┐
  │  Primitives: csprng, chacha20-poly1305, argon2id, hkdf,      │
  │              ed25519, x25519, blake2b, hmac, sha256, …       │
  └──────────────────────────────────────────────────────────────┘
```

### Dependency rule

A layer may read the public API of any layer beneath it, but **must
not** reach around it. VLT05 may not bypass VLT01 and talk to
`storage-core` directly, because that would bypass envelope
encryption. The only intentional exception is VLT09 (import/export),
which operates on plaintext during a ceremony the user explicitly
initiates.

## Layers, in order

### VLT01 — Sealed store  ✅ shipped

Per-record envelope encryption. One KEK derived from a password via
Argon2id, one fresh DEK per record wrapped under the KEK, body AEAD
bound to `(namespace, key)` via AAD. KEK rotation without re-
encrypting bodies. Sits directly on `storage-core`.

See [VLT01-vault-sealed-store.md](./VLT01-vault-sealed-store.md) for
the full spec. This is the foundation the rest of the stack stands on.

### VLT02 — Typed record codecs

VLT01 stores opaque `Vec<u8>` plaintexts. That is intentional — it is
a primitive. But no application wants to hand-roll serialisation of a
`Login { title, username, password, urls, notes }` record, and every
app that does so reinvents the same bugs.

VLT02 defines:

- A **record schema** trait (`VaultRecord`): typed struct ↔ canonical
  bytes (CBOR or MessagePack — TBD in the VLT02 spec). Canonical
  encoding is required so that wrapped ciphertext is deterministic
  across platforms, which matters for sync conflict detection.
- A **schema registry**: every record has a `content_type` tag
  (`"vault/login/v1"`, `"vault/note/v1"`, `"vault/card/v1"`,
  `"vault/totp/v1"`, `"vault/ssh-key/v1"`, `"vault/identity/v1"`,
  `"vault/custom/v1"` for app-defined types).
- **Versioning**: `v1 → v2` migration is a codec concern, not a
  storage concern. The codec layer reads whatever version is on disk
  and up-converts on read.
- **Extensibility**: app code registers new content types. The vault
  treats unknown types as opaque bytes and will not crash on them.

Depends on: VLT01. Adds no new cryptography.

### VLT03 — Multi-KEK wrapping

VLT01's DEK is wrapped under exactly one KEK. That is fine for a
single-user, single-device vault but breaks as soon as you want:

- **Sharing.** Alice wants Bob to read one item. Server cannot
  decrypt. So Alice must re-wrap the item's DEK under a key that Bob
  owns — specifically, Bob's **public key**.
- **Multiple devices.** Laptop, phone, extension — each has its own
  device keypair. Each can unwrap the vault KEK because the vault KEK
  is wrapped under every device's public key.
- **Recovery.** Optional "recovery key" printed at account creation:
  another public key that can unwrap the vault KEK. If the user
  forgets the password, they type the recovery key and re-establish
  access.
- **Revocation.** Lost phone? Remove that device's wrap. All future
  KEK rotations won't include it; the old wrap is unchanged but has
  no new data.

VLT03 introduces a **key hierarchy** (terminology matches Bitwarden's
and 1Password's published models, which converge on the same design):

```text
   User password ──Argon2id──▶ MUK  (Master Unlock Key)
                                │
                                ▼
                       unwraps USK (User Symmetric Key)
                                │
                                ▼
                    USK wraps vault records' DEKs
                    USK is ALSO wrapped under every
                       device public key and every
                       share-recipient public key
```

Where the sealed store (VLT01) previously had a single `wrapped_dek`
per record, VLT03 generalises to a **set of wraps**: one per grantee.
Adding a grantee = one public-key wrap operation, not a re-encryption.

Primitives: we already have `x25519` (ECDH), `ed25519` (signing), and
`hkdf`. VLT03 adds an **asymmetric wrap** format — X25519-ECDH +
HKDF-Expand + ChaCha20-Poly1305, which is a well-understood
construction (NaCl's `crypto_box` is the classic reference).

Depends on: VLT01, VLT02 (so we can store key-bundle records).

### VLT04 — Secure sync channel

Once the vault is useful on one device, the next question is "how do
multiple devices share it?" VLT04 specifies a **server-neutral sync
protocol** with two invariants:

1. **Zero-knowledge server.** The server stores opaque ciphertext
   blobs plus revision metadata. It cannot read any user data.
   Everything it sees is already AEAD'd under keys it does not hold.
2. **Deterministic merge.** Two offline devices making disjoint edits
   must converge cleanly on reconnect. The vault is modelled as a set
   of records identified by `(namespace, key)`; each record has a
   monotonically increasing revision; merge is last-writer-wins *per
   record* with CAS conflicts surfaced to the app for manual
   resolution.

Shape:

- **Transport.** HTTPS + an auth channel (likely **SRP** or **OPAQUE**
  so the server never sees the user's password; OPAQUE is the modern
  choice and we'll spec it in VLT04).
- **Wire format.** `(namespace, key, revision, sealed_record_bytes)`.
  The server literally cannot tell what kind of record it is.
- **Delta protocol.** `GET /sync?since=<revision>` returns records
  changed since the caller's last revision. Device uploads new
  revisions with `If-Revision-Match`.
- **Conflict-free metadata.** Namespace registry (already a VLT01
  concept) becomes the sync anchor.

Depends on: VLT01 (for sealed bytes), VLT03 (so the sync endpoint
works with shared / multi-device vaults).

### VLT05 — Attachments

Big binary blobs — receipt scans, ID photos, key files, GPG private
keys — do not fit the "one record at a time" model because:

- They can be >10 MiB, which we don't want to hold in memory.
- They are often immutable once stored; re-encrypting them on every
  edit of the parent item is wasteful.

VLT05 introduces:

- **Blob objects**: streamable, chunked, each chunk AEAD'd with its
  own nonce under a per-blob DEK.
- **Blob IDs** stored inside parent records (VLT02). The parent is
  small; the blob lives in its own `__vault_blobs__` namespace.
- **Stream API**: `open_attachment(parent, attachment_id) -> impl
  Read`. Decryption happens chunk by chunk.
- **Server-side upload**: the sync channel (VLT04) is extended so
  attachment chunks can be POSTed without the server ever seeing
  plaintext.

Depends on: VLT01, VLT04.

### VLT06 — Revision history

Every write to a record via VLT01 overwrites the prior revision. For
a password manager, that is a bug — users want to recover the
password they accidentally just overwrote.

VLT06 layers a **version stream** on top:

- On every `put(namespace, key, new_body)`, the old ciphertext is
  appended to a sibling `(namespace, key, __history__)` list rather
  than discarded.
- History entries are themselves sealed records, so they inherit
  VLT01's at-rest encryption.
- Retention policy (keep-last-N, keep-last-T-days, unbounded) is a
  per-namespace config stored in the namespace registry.
- The vault exposes `history(namespace, key) -> iter<Revision>` and
  `restore(namespace, key, revision)`.

Depends on: VLT01 only. Orthogonal to sharing / sync in the sense
that "restored revision N" is just a new write with revision N+k.

### VLT07 — Encrypted search index

Decrypting 5 000 records on every keystroke is too slow; shipping
the plaintext to the server is forbidden; so we need an **encrypted
searchable index**.

Design space (full spec in VLT07):

- **Substring prefix search** over titles, URLs, usernames — the
  minimum useful for a password manager.
- **Client-side index**: each device maintains a local trigram or
  BM25 index, stored encrypted under the vault's USK. The server
  never sees the index.
- **Sync-friendly**: rebuilding the index from scratch on every
  sync would work but is slow for large vaults. So the index is
  itself a set of vault records (one per shard), synced like any
  other record.
- **No server-side search** in v1. If we ever want server-side
  search we will add a second layer on top (searchable-symmetric-
  encryption is a whole research field; we defer).

Depends on: VLT01, VLT02 (to know which fields of a record are
indexable — the schema declares them).

### VLT08 — Audit log

Shared / team vaults need an accountability trail: "Bob accessed
the prod DB password on 2026-05-04 at 14:22". VLT08 specifies:

- **Append-only** log records in `__vault_audit__`.
- **Tamper-evident** via a hash chain: each entry contains
  `prev_hash = blake2b(prev_entry || entry_body)`. Once
  shipped to the server, the chain is pinned.
- **Signed** — each entry is signed by the actor's device key
  (ed25519), so the server cannot forge entries.
- **Privacy-respecting**: log entries are encrypted under the
  vault's USK like any other record. The server sees hashes and
  signatures but not the event body, so it can verify chain
  integrity without reading the content.

Depends on: VLT01, VLT03 (for the signing key identity), VLT04 (for
the server side of the chain).

### VLT09 — Import / export

Practical adoption depends on getting existing data in and out:

- **Import** from 1Password `.1pux`, Bitwarden JSON, KeePassXC
  `.kdbx`, LastPass CSV, Chrome / Firefox CSV.
- **Export** a portable, versioned JSON bundle that another instance
  of our vault (or a competitor) can re-import.
- **Migration ceremony** is explicitly UI-driven: plaintext is
  touched only in the import/export path, under a user-initiated
  action with clear warnings.

Depends on: VLT02 (schemas are the target of import and the source of
export). Touches VLT01 only via the public API — never reaches
around.

## Primitives inventory

What we already have in-repo:

| Primitive              | Crate                        | Used by        |
|------------------------|------------------------------|----------------|
| ChaCha20-Poly1305 AEAD | `chacha20-poly1305`          | VLT01, all     |
| XChaCha20-Poly1305     | (same crate, X variant)      | VLT01          |
| Argon2id KDF           | `argon2id`                   | VLT01, VLT03   |
| HKDF-SHA-256           | `hkdf`                       | VLT03, VLT04   |
| HMAC-SHA-256           | `hmac`                       | VLT04, VLT08   |
| BLAKE2b                | `blake2b`                    | VLT08          |
| SHA-256 / SHA-512      | `sha256` / `sha512`          | various        |
| Ed25519                | `ed25519`                    | VLT03, VLT08   |
| X25519                 | `x25519`                     | VLT03          |
| CSPRNG                 | `csprng`                     | all            |
| Constant-time compare  | `ct-compare`                 | VLT01          |
| Zeroizing wrapper      | `zeroize`                    | VLT01          |
| KV storage (CAS)       | `storage-core`               | VLT01          |

What we do not yet have and will need to introduce on the road to
VLT09:

- **OPAQUE** (or SRP-6a as a fallback) — the password-authenticated
  key exchange used by VLT04. New crate — likely `opaque-pake`.
- **Canonical CBOR** (or MessagePack) — deterministic serialisation.
  New crate — likely `canonical-cbor`.
- **Chunked AEAD stream** — for VLT05 attachments. May live inside
  `chacha20-poly1305` as a `Stream` mode or as a new tiny crate
  (`aead-stream`).
- **Trigram / BM25 index** — for VLT07. New crate.
- **Import parsers** — one tiny crate per competitor format (likely
  under `code/packages/rust/vault-import-*`).

## Ordering rationale

The layering order is not arbitrary. The constraints:

1. **Sharing changes the shape of every record.** The decision
   between "one wrap" (VLT01) and "many wraps" (VLT03) affects the
   on-disk format. Doing VLT02 *before* we generalise wrapping is
   fine, because schemas are about *plaintext* — the wrap layer is
   oblivious to them. Doing VLT05 (attachments) *before* VLT03 would
   be a mistake: we'd spec an attachment wire format then have to
   redo it when multi-KEK lands.
2. **Sync assumes sharing and multi-device.** VLT04 is written
   against a VLT03 world, not a VLT01 world. If you stand up sync
   against single-KEK vaults, you will have to re-spec it once
   sharing exists.
3. **History, search, and audit are orthogonal to each other.** Any
   ordering among VLT06/07/08 is fine; we pick history first because
   it is the smallest and the most immediately useful to a
   single-user build.
4. **Import/export is last** because it has to target a stable
   schema layer (VLT02) and a stable revision/history model (VLT06)
   to round-trip faithfully.

If future research says (e.g.) "encrypted search needs to be aware
of sharing", we will revisit — but today, VLT07 can treat the USK as
"the one key it encrypts the index under" and stay decoupled.

## Cross-cutting concerns

### Threat model

Assumed across the whole stack:

- Attacker can read the entire at-rest database.
- Attacker can observe all sync traffic.
- Attacker controls the sync server (it is honest-but-curious at
  best, fully malicious at worst).
- Attacker does **not** control the user's device at the moment of
  unseal. (If they do, the game is over — no software vault solves
  "attacker has RAM access with the vault open".)
- Attacker does not have the user's password and cannot exhaust
  Argon2id offline (we pick parameters that take ≥250 ms on modern
  hardware; VLT01 already enforces ceilings).

### Error surfaces

Every layer returns typed `*Error` enums (pattern already set by
VLT01). Error messages are **never** derived from persisted bytes;
they come only from literals in the crate. This is a hard rule,
because a malicious sync server could otherwise inject an error
message that appears to come from our code.

### Logging & key material

**No layer logs key material, wrapped keys, ciphertext, or user
passwords — ever.** Tests assert this by pattern-matching the debug
impls of key-holding types. VLT01 already does this; subsequent
layers inherit the rule.

### Testing

Every layer ships with:

- Round-trip tests (write → read → assert equality).
- Tamper detection tests (flip one byte in storage, assert decrypt
  fails).
- Boundary / parameter tests (min/max sizes, empty inputs, huge
  inputs).
- Rotation / migration tests where applicable.
- Cross-instance tests (write on one `SealedStore` instance, read
  from another over the same backend).

For VLT04 specifically, we also ship a **reference server** (tiny,
in-memory) so client-side sync tests have something to talk to
without booting real infrastructure.

## What is out of scope for the Vault primitive

Belongs in the **application** (whatever password manager is being
built), not here:

- UI of any kind (CLI, TUI, GUI, browser extension).
- Autofill, form detection, credit-card autofill.
- OS keychain / Touch ID / Face ID / Windows Hello integration.
- Breach-password monitoring (HaveIBeenPwned etc.).
- Organisation-level billing, SSO, SCIM user provisioning.
- Phishing-resistant URL matching heuristics.
- Password generators (might live in a tiny sibling crate, but it
  is not part of the Vault primitive and can be swapped out).

These are meaningful features of a real product, but they are not
primitives; each is opinionated in ways that would contaminate the
Vault library.

## Milestone plan

Rough target ordering, each layer its own PR / set of PRs:

- [x] VLT01 — sealed store
- [ ] VLT02 — typed record codecs
- [ ] VLT03 — multi-KEK wrapping
- [ ] VLT04 — secure sync channel (includes OPAQUE ceremony)
- [ ] VLT05 — attachments
- [ ] VLT06 — revision history
- [ ] VLT07 — encrypted search index
- [ ] VLT08 — audit log
- [ ] VLT09 — import / export

After VLT04 the stack is minimally useful as a library: a multi-
device, sharable, syncable, encrypted KV store. After VLT09 it is
close to feature-parity with an early Bitwarden / 1Password.

Everything above VLT09 is the *application*, and that is where the
interesting product work starts.
