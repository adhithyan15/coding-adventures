# `coding_adventures_vault_import_keepass` — VLT-PM49 Amendment 2

Decodes a **KDBX4** (KeePass 2.x / KeePassXC) password database into the
shared `PortableRecord` vocabulary defined by
[`vault-import-export`](../vault-import-export) (VLT15).

Unlike its plaintext siblings (`vault-import-bitwarden`, `vault-import-csv`,
`vault-import-otpauth`), a `.kdbx` file is itself a real encrypted
container — Argon2d/Argon2id key derivation, then AES-256-CBC or ChaCha20
decryption of an HMAC-SHA256-authenticated block stream, holding an
optionally-gzipped inner XML document. This crate implements that whole
stack from scratch, composed entirely from sibling primitive crates already
in this workspace (`argon2d`, `argon2id`, `aes-modes`,
`chacha20-poly1305`, `hmac`, `sha256`, `sha512`, `deflate`, `xml-parser`).
No new cryptographic primitive is added.

This amendment closes the KDBX deferral `VLT-PM49-cli-external-import.md`
§8 originally recorded, consumed by `vault-pm`'s `import kdbx FILE`
ceremony.

## Where this fits in the vault-pm stack

```text
a .kdbx file
        |
        v
  vault_import_keepass::decode(bytes, password)   <- this crate
        |
        v
   Vec<PortableRecord>                             <- VLT15's shared vocabulary
        |
        v
  vault-pm-cli's `import kdbx`                      <- maps to AnyRecord, calls
                                                         the existing audited
                                                         add_item path once per
                                                         record
```

## Format sources

No official machine-readable KDBX4 specification exists. This crate's byte
layout is transcribed from Wladimir Palant's reverse-engineered
documentation and cross-checked against the KeePassXC and `kdbxweb`/
`keepass-rs` open-source implementations — the same citation list
`VLT-PM49-cli-external-import.md` §8.1 carries.

## Mapping

Every `<Entry>` at any depth under `Root`/`Group*` (group/folder structure
is not carried across — no vault-pm collection to put it in) becomes a
record:

| KeePass `String/Key` | Produces |
|---|---|
| `Title` | item title |
| `UserName` | `Login.username` |
| `Password` (always `Protected`) | `Login.password` |
| `URL` | `Login.url` |
| `Notes` | `Login.notes`, or the sole content of a `SecureNote` if `UserName`/`Password`/`URL` are all empty |
| `otp` / `TOTP Seed` | a second, separate `Totp` record — `totp_seed` is handed through unparsed for `vault-pm-cli`'s existing `decode_external_totp_field` to decode identically to a Bitwarden/CSV TOTP field |
| any other key | `custom_fields[key]`, up to `MAX_CUSTOM_FIELDS_PER_ENTRY` |

Not carried across (documented, not silently dropped): binary attachments,
entry history (previous revisions), `IconID`/tags/expiry metadata, and
group/folder structure.

## Why a free function, not `Importer`

Every other VLT-PM49 format reads plaintext and needs no secret to decode
it, so `vault-import-export::Importer::import` takes only `&[u8]`. A KDBX
database needs its own master password, which that trait has no slot for.
This crate exposes a free function instead:

```rust,ignore
pub fn decode(
    container: &[u8],
    password: &Zeroizing<Vec<u8>>,
) -> Result<Vec<PortableRecord>, ImportError>;
```

## Threat model

- **Bounded before any parsing.** `MAX_SOURCE_BYTES` (16 MiB) caps the
  whole container up front, matching the shared
  `MAX_EXTERNAL_IMPORT_SOURCE_BYTES` ceiling `vault-pm-cli` already applies
  to every import format.
- **KDF cost as an attacker-controlled allocation lever.** Every
  `KdfParameters` value is attacker-controlled and read *before* the
  master password is verified. `MAX_KDF_MEMORY_KIB` (1 GiB),
  `MAX_KDF_ITERATIONS` (64), and `MAX_KDF_PARALLELISM` (16) bound the raw
  declared header values, checked before any division and before
  Argon2d/Argon2id is ever called — a crafted `M` near `u64::MAX` bytes
  never reaches the KDF.
- **Decompression amplification.** The reassembled body is decompressed
  through `deflate::inflate_counted`, which enforces
  `MAX_DECOMPRESSED_BYTES` (256 MiB) *during* inflation, not only checked
  against the result afterward.
- **Deeply nested inner XML.** Parsed with this workspace's existing
  depth-capped `xml-parser`, not a new hand-rolled decoder.
- **Wrong password vs corrupt file.** KDBX gives no way to distinguish the
  two more precisely than "an HMAC/hash didn't match." Every content
  integrity failure (header hash, header HMAC, any block-stream HMAC)
  collapses to one fixed, non-distinguishing message rather than inventing
  a finer-grained answer.
- **Constant-time comparison.** Every HMAC/hash check against
  attacker-influenced ciphertext uses `subtle::ConstantTimeEq`.
- **Plaintext residue.** The password and every KDF/HMAC key-derivation
  intermediate (`password_hash`, `composite_key`, `derived_key`,
  `encryption_key`, `hmac_key_base`, every per-block/header HMAC key, the
  decompressed inner buffer, every decrypted protected value) is held
  under `Zeroizing`.

## Usage

```rust,ignore
use coding_adventures_vault_import_keepass::decode;
use coding_adventures_zeroize::Zeroizing;

let container = std::fs::read("vault.kdbx")?;
let password = Zeroizing::new(b"correct horse battery staple".to_vec());
let records = decode(&container, &password)?;
```

## Test fixtures for downstream consumers

No real KeePass installation is available in this environment (§8.9), so
this crate's own test suite proves `decode` against a `.kdbx` fixture it
builds itself with a `#[cfg(test)]`-only forward encoder. That encoder's
one function meant for outside use, `test_support::build_single_login_
fixture`, is also reachable behind the `test-fixtures` Cargo feature, so a
downstream crate's own tests (`vault-pm-cli`'s `import kdbx` tests) can
build a genuinely valid fixture without duplicating this byte-level
encoder. Enable it only as a `[dev-dependencies]` feature — it is never
part of a production build.

## Bounds

`MAX_SOURCE_BYTES = 16 MiB`, `MAX_KDF_MEMORY_KIB = 1 GiB`,
`MAX_KDF_ITERATIONS = 64`, `MAX_KDF_PARALLELISM = 16`,
`MAX_DECOMPRESSED_BYTES = 256 MiB`, `MAX_CUSTOM_FIELDS_PER_ENTRY = 64`,
`MAX_ENTRIES = 50_000`, `MAX_FIELD_LEN = 64 KiB`.

## Out of scope (documented, not silently dropped)

- **KDBX3** — a materially different outer format (2-byte header field
  lengths, no HMAC block stream, gzip applied before AES-CBC rather than
  after). Rejected by major version number.
- **Legacy AES-KDF** — superseded by Argon2d/Argon2id since KeePass 2.35
  (2019). Rejected by `$UUID`, by name.
- **AES128-CBC and Twofish-CBC outer ciphers** — legal but rare (AES256-CBC
  is KeePass's own default; ChaCha20 is the only other cipher its UI
  exposes). Rejected by `CipherID`, by name.
- **Salsa20 inner stream** — ChaCha20 has been KDBX4's default inner
  stream since its introduction. Rejected by `InnerRandomStreamID`, by
  name.
- **Keyfiles and hardware-key (YubiKey challenge-response) composite-key
  components.** Only the password-only case is implemented; a database
  requiring one of these simply fails the header HMAC check like any other
  wrong master key.
- **Binary attachments, entry history, group/folder structure, `IconID`,
  tags, and expiry metadata.**
- **`PublicCustomData`** — parsed enough to skip over, never interpreted.
