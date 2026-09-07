# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-07

### Added

- Initial implementation of the KDBX4 (KeePass 2.x / KeePassXC) format
  adapter closing VLT-PM49 Amendment 2's original KDBX deferral
  (`code/specs/VLT-PM49-cli-external-import.md` §8).
- `decode(container: &[u8], password: &Zeroizing<Vec<u8>>) -> Result<Vec<PortableRecord>, ImportError>`
  — a free function rather than an `Importer` impl, because decoding needs
  the database's own master password, which the shared `Importer::import`
  trait has no slot for (§8.7).
- Full from-scratch KDBX4 pipeline composed from existing sibling crates:
  outer-header TLV + `VariantDictionary` parsing, header SHA-256/HMAC-SHA256
  integrity checks, Argon2d/Argon2id key derivation, AES-256-CBC/ChaCha20
  outer decryption, HMAC-SHA256 block-stream reassembly, gzip-container
  stripping + bounded `deflate` inflation, inner-header parsing, and a
  document-order ChaCha20 inner-stream decoder for `Protected="True"` XML
  values.
- Maps every `<Entry>` at any depth to a `Login` or `SecureNote`
  `PortableRecord` (plus a second `Totp` record for an `otp`/`TOTP Seed`
  field), with unrecognized `String/Key` fields kept as `custom_fields`
  rather than dropped.
- Bounds: `MAX_SOURCE_BYTES` (16 MiB), `MAX_KDF_MEMORY_KIB` (1 GiB),
  `MAX_KDF_ITERATIONS` (64), `MAX_KDF_PARALLELISM` (16),
  `MAX_DECOMPRESSED_BYTES` (256 MiB), `MAX_CUSTOM_FIELDS_PER_ENTRY` (64),
  `MAX_ENTRIES` (50,000), `MAX_FIELD_LEN` (64 KiB) — the three KDF cost
  bounds are checked against the header's raw declared values *before* any
  division or Argon2 call, closing the "crafted `M` near `u64::MAX`"
  allocation lever §8.2 identifies as this amendment's one genuinely new
  attack surface over the rest of VLT-PM49.
- Named, distinct rejections (not a generic decode failure) for every
  explicitly-out-of-scope primitive: KDBX3 (`ImportError::UnsupportedVersion`),
  legacy AES-KDF, AES128-CBC, Twofish-CBC, and the Salsa20 inner stream
  (each `ImportError::Adapter` with a specific message).
- A wrong password and a byte-corrupted-but-structurally-valid file are
  indistinguishable by construction (§8.3): both produce the exact same
  `ImportError::Adapter("wrong password or corrupt file")`.
- A forward KDBX4 encoder (§8.9), gated behind `cfg(test)` plus the new
  `test-fixtures` Cargo feature (`test_support` module) — no real KeePass
  installation is available in this environment, so the test suite proves
  the decoder against fixtures this crate builds itself, cross-checked
  against the byte-level field layout and key-derivation formulas
  transcribed independently from third-party KDBX4 documentation rather
  than derived from this crate's own decoder. `test_support::build_single_
  login_fixture` is the one function of that encoder exposed under the
  feature flag, so `vault-pm-cli`'s own `import kdbx` tests can build a
  genuinely valid fixture without duplicating this byte-level encoder a
  second time; the feature is only pulled in via `vault-pm-cli`'s
  `[dev-dependencies]`, never for a production build.
- 42 unit tests: a full round-trip matrix (both outer ciphers × both KDFs ×
  compressed/uncompressed), entry-kind mapping (login, secure note, TOTP
  field under both recognized key names, custom fields, multiple entries),
  wrong-password/corruption message-equality (gate 11), a block-stream HMAC
  failure at a non-zero block index (gate 14), all three KDF cost ceilings
  proven to short-circuit before Argon2d/Argon2id is ever called via a
  thread-local call counter (gate 12), every named-rejection case
  (KDBX3/AES-KDF/AES128-CBC/Twofish-CBC/Salsa20/unsupported Argon2 version,
  gate 13), a structural/malformed-input matrix (oversized container,
  truncated header, truncated block stream, invalid UTF-8 in the
  decompressed XML, malformed base64 in a protected value, decompressed
  size at and past `MAX_DECOMPRESSED_BYTES`, gate 15), and the two bounds
  added in pre-merge security review (`MAX_PROTECTED_VALUES`, and
  `MAX_FIELD_LEN` enforced per-value during collection rather than only
  after decryption).
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Security hardening (pre-merge review)

A `/security-review` pass before pushing found two real gaps and one
documented (not code-fixable) limitation:

- **MEDIUM — inner stream key not zeroizing.** `InnerHeaderFields::stream_key`
  (the `InnerRandomStreamKey` parsed from the inner header — the direct
  SHA-512 preimage of the ChaCha20 key protecting every `Protected` XML
  value, including `Password`) was stored as a plain `Vec<u8>`, unlike
  every other key-derivation intermediate in this crate. Fixed: wrapped in
  `Zeroizing<Vec<u8>>`.
- **MEDIUM — protected-value collection unbounded before decryption.**
  `MAX_ENTRIES`/`MAX_CUSTOM_FIELDS_PER_ENTRY`/`MAX_FIELD_LEN` were enforced
  only in the second XML pass (`build_records`, over already-decrypted
  values); the first pass (`collect_protected_ciphertexts` +
  `decrypt_protected_values`) decrypted every `Protected` value in the
  document — with no per-value or total-count cap of its own — before that
  second pass ever ran, relying solely on the outer `MAX_DECOMPRESSED_BYTES`
  ceiling to bound peak memory. Requires a password-authenticated file, so
  this was a defense-in-depth gap, not an unauthenticated DoS. Fixed: added
  `MAX_PROTECTED_VALUES` (200,000) and a per-ciphertext `MAX_FIELD_LEN`
  check directly in `collect_protected_ciphertexts`, before
  `decrypt_protected_values` allocates its keystream buffer.
- **MEDIUM — plaintext residue in `title`/`username`/`url`/`notes` (not
  fixed; documented).** A KeePass entry can mark *any* `String` field
  `Protected="True"`, not only `Password`. A protected `UserName`/`URL`/
  `Notes`/`Title` value is correctly decrypted under `Zeroizing` and then
  necessarily copied into the corresponding `PortableRecord` field, which
  is a plain `String` by that shared type's own definition (every adapter
  in this workspace shares it; none of the other three ever puts
  secret-shaped data in those slots). This is a limitation of
  `PortableRecord` itself, not something this crate can fix without either
  diverging from the shared vocabulary or changing it for all four
  adapters — out of scope for this amendment, recorded here rather than
  silently left unmentioned. See the crate doc comment's "Plaintext
  residue" bullet and the README's Threat model section.

### Out of scope (documented, not silently dropped)

- KDBX3, legacy AES-KDF, AES128-CBC, Twofish-CBC, the Salsa20 inner stream,
  keyfile/hardware-key composite-key components, binary attachments, entry
  history, group/folder structure, and `PublicCustomData` interpretation.
