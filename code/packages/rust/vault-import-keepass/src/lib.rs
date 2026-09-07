//! # `coding_adventures_vault_import_keepass` — VLT-PM49 Amendment 2
//!
//! ## What this crate is
//!
//! A from-scratch **KDBX4** (KeePass 2.x / KeePassXC password database)
//! decoder into the shared [`PortableRecord`] vocabulary
//! (`coding_adventures_vault_import_export`, VLT15) that every import
//! adapter in this workspace produces. Unlike its plaintext siblings
//! (`vault-import-bitwarden`, `vault-import-csv`, `vault-import-otpauth`),
//! a `.kdbx` file is itself a real encrypted container, so this crate does
//! genuine cryptography — Argon2d/Argon2id key derivation, AES-256-CBC or
//! ChaCha20 decryption, HMAC-SHA256-authenticated block framing, and a
//! ChaCha20-obfuscated inner stream over protected XML values — composed
//! entirely from sibling primitive crates already in this workspace:
//! `argon2d`, `argon2id`, `aes-modes`, `chacha20-poly1305`, `hmac`,
//! `sha256`, `sha512`, `deflate`, and `xml-parser`. No new cryptographic
//! primitive is implemented here.
//!
//! ## Pipeline
//!
//! ```text
//! .kdbx file bytes
//!         |
//!         v
//! signature + version check ---------- major version must be 4 (KDBX3 out of scope)
//!         |
//!         v
//! outer header (cleartext TLV) -------- CipherID, CompressionFlags, MainSeed,
//!         |                             EncryptionIV, KdfParameters
//!         v
//! header SHA-256 check ---------------- corruption check only
//!         |
//!         v
//! KdfParameters bounds check ---------- MAX_KDF_MEMORY_KIB / ITERATIONS /
//!         |                             PARALLELISM, checked BEFORE the KDF runs
//!         v
//! Argon2d/Argon2id key derivation ----- password -> derived_key
//!         |
//!         v
//! header HMAC-SHA256 check ------------ the real "right password" check
//!         |
//!         v
//! AES-256-CBC / ChaCha20 body decrypt
//!         |
//!         v
//! HMAC-SHA256 block-stream reassembly - each block independently authenticated
//!         |
//!         v
//! (optional) gzip-wrapped deflate decompress, bounded by MAX_DECOMPRESSED_BYTES
//!         |
//!         v
//! inner header (cleartext TLV) -------- InnerRandomStreamID (ChaCha20 only),
//!         |                             InnerRandomStreamKey
//!         v
//! inner XML (xml-parser::parse_xml) --- Protected="True" values are
//!         |                             ChaCha20-obfuscated, decoded in
//!         |                             document order with one continuous
//!         |                             keystream
//!         v
//! Vec<PortableRecord>
//! ```
//!
//! ## Mapping (VLT-PM49 §8.4)
//!
//! Every `<Entry>` at any depth under `Root`/`Group*` becomes a record
//! (group/folder structure is not carried across, matching every other
//! adapter's "no collections" default):
//!
//! | KeePass `String/Key` | vault-pm outcome |
//! |---|---|
//! | `Title` | item title |
//! | `UserName` | `Login.username` |
//! | `Password` (always `Protected`) | `Login.password` |
//! | `URL` | `Login.url` |
//! | `Notes` | `Login.notes`, or the sole content of a `SecureNote` if `UserName`/`Password`/`URL` are all empty |
//! | `otp` / `TOTP Seed` | a second, separate [`PortableRecordKind::Totp`] record, `totp_seed` handed through unparsed for `vault-pm-cli`'s existing `decode_external_totp_field` to interpret |
//! | any other key | `custom_fields[key]`, up to [`MAX_CUSTOM_FIELDS_PER_ENTRY`] |
//!
//! Binary attachments, entry history (previous revisions), and
//! `IconID`/tags/expiry metadata have no [`PortableRecord`] slot and are
//! not carried across (VLT-PM49 §8.5) — the same "documented, not
//! silently dropped" discipline every other adapter in this workspace
//! follows.
//!
//! ## Threat model (VLT-PM49 §8.8)
//!
//! * **Untrusted container, bounded before any parsing.** [`MAX_SOURCE_BYTES`]
//!   caps the whole input up front, matching the shared
//!   `MAX_EXTERNAL_IMPORT_SOURCE_BYTES` ceiling `vault-pm-cli` already
//!   applies to every import format.
//! * **KDF cost as an attacker-controlled allocation lever.** Every
//!   `KdfParameters` value is read from the file before the master
//!   password is verified. [`MAX_KDF_MEMORY_KIB`], [`MAX_KDF_ITERATIONS`],
//!   and [`MAX_KDF_PARALLELISM`] bound the *raw declared* values —
//!   checked before any division and before Argon2d/Argon2id is ever
//!   called — so a crafted `M` near `u64::MAX` bytes cannot force a
//!   multi-gigabyte allocation.
//! * **Decompression amplification.** The reassembled block-stream body
//!   is decompressed through `deflate::inflate_counted`, which enforces
//!   [`MAX_DECOMPRESSED_BYTES`] *during* inflation rather than only
//!   checking the result afterward.
//! * **Deeply nested inner XML.** Parsed with this workspace's existing
//!   depth-capped `xml-parser`, not a new hand-rolled decoder.
//! * **Wrong password vs corrupt file.** KDBX gives no way to distinguish
//!   the two more precisely than "the HMAC didn't match," so every content
//!   HMAC/hash failure (header integrity, block stream) collapses to one
//!   fixed, non-distinguishing [`ImportError::Adapter`] message — this
//!   crate does not invent a finer-grained answer KDBX itself cannot give.
//! * **Constant-time comparison.** Every HMAC/hash comparison against
//!   attacker-influenced ciphertext uses `subtle::ConstantTimeEq`, not
//!   `==`.
//! * **Plaintext residue.** Every secret-shaped intermediate — the
//!   password, every KDF/HMAC key derivation step, and every decrypted
//!   protected value — is held under [`Zeroizing`] end to end.
//!
//! ## Usage
//!
//! ```ignore
//! use coding_adventures_vault_import_keepass::decode;
//! use coding_adventures_zeroize::Zeroizing;
//!
//! let container: Vec<u8> = std::fs::read("vault.kdbx")?;
//! let password = Zeroizing::new(b"correct horse battery staple".to_vec());
//! let records = decode(&container, &password)?;
//! # Ok::<(), coding_adventures_vault_import_export::ImportError>(())
//! ```
//!
//! ## Why a free function, not `Importer`
//!
//! Every other format in VLT-PM49 reads plaintext bytes and needs no
//! secret to decode them, so `vault-import-export::Importer::import`
//! takes only `&[u8]`. KDBX cannot implement that trait: decoding
//! requires the database's own master password. This crate exposes
//! [`decode`] as a free function instead (VLT-PM49 §8.7); `vault-pm-cli`
//! collects the password through the same `read_import_passphrase` host
//! method `portable_import`/`portable_restore` already use.
//!
//! ## Explicitly out of scope (VLT-PM49 §8.6)
//!
//! KDBX3, the legacy AES-KDF, AES128-CBC and Twofish-CBC outer ciphers,
//! the Salsa20 inner stream, and keyfile/hardware-key composite-key
//! components are all real, separable gaps recorded here rather than
//! silently misdecoded — each is refused by name with a distinct error
//! rather than a generic decode failure.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_vault_import_export::{ImportError, PortableRecord, PortableRecordKind};
use coding_adventures_xml_parser::{parse_xml, XmlDocument, XmlElement, XmlNode};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::collections::BTreeMap;
use subtle::ConstantTimeEq;

// ============================================================================
// Section 1: Bounds (VLT-PM49 §8.2, §8.8)
// ============================================================================

/// Maximum accepted raw container bytes, checked before any parsing begins.
///
/// Matches the shared `MAX_EXTERNAL_IMPORT_SOURCE_BYTES` ceiling
/// `vault-pm-cli` already applies to every `import FORMAT FILE` source
/// (VLT-PM49 §4/§8.7), so a file too large for the host to even hand this
/// crate is refused at the same size this crate would refuse it anyway.
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum accepted Argon2 memory cost, in KiB, checked against the
/// header's raw declared `M` (bytes) value *before* dividing by 1024 and
/// *before* Argon2d/Argon2id is ever called (VLT-PM49 §8.2). 1 GiB is a
/// large multiple of any real KeePass client's default (tens of MiB) while
/// still bounding the worst case a crafted file can force this process to
/// allocate.
pub const MAX_KDF_MEMORY_KIB: u64 = 1024 * 1024;
/// Maximum accepted Argon2 time cost (iteration count), checked before the
/// KDF is ever called.
pub const MAX_KDF_ITERATIONS: u64 = 64;
/// Maximum accepted Argon2 parallelism (lane count), checked before the KDF
/// is ever called.
pub const MAX_KDF_PARALLELISM: u32 = 16;
/// Maximum accepted decompressed inner-body size, enforced *during*
/// inflation (`deflate::inflate_counted`) rather than only checked
/// afterward, so a crafted small-file/huge-output decompression bomb never
/// materializes the full output.
pub const MAX_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;
/// Maximum accepted non-standard `String/Key` fields kept per `Entry` as
/// `custom_fields`.
pub const MAX_CUSTOM_FIELDS_PER_ENTRY: usize = 64;
/// Maximum accepted `<Entry>` elements walked across the whole document
/// (at any depth, including inside `<History>`), mirroring
/// `vault-import-bitwarden`'s `MAX_ITEMS`.
pub const MAX_ENTRIES: usize = 50_000;
/// Maximum accepted bytes for any single string field this adapter reads
/// (a `String/Key` name or its decoded value).
pub const MAX_FIELD_LEN: usize = 64 * 1024;

/// Maximum accepted entries in one `VariantDictionary` (`KdfParameters` or
/// `PublicCustomData`). Real files carry a handful; generous headroom over
/// that bounds a crafted dictionary padded with many tiny junk entries,
/// the same "bound the object, not just the array" discipline
/// `vault-import-bitwarden`'s `MAX_KEYS_PER_OBJECT` applies to JSON.
const MAX_VARIANT_ENTRIES: usize = 256;

// ============================================================================
// Section 2: Format constants (VLT-PM49 §8.1, §8.2, §8.6)
// ============================================================================

const SIGNATURE_1: u32 = 0x9AA2_D903;
const SIGNATURE_2: u32 = 0xB54B_FB67;
const REQUIRED_MAJOR_VERSION: u16 = 4;

const OUTER_FIELD_CIPHER_ID: u8 = 2;
const OUTER_FIELD_COMPRESSION: u8 = 3;
const OUTER_FIELD_MAIN_SEED: u8 = 4;
const OUTER_FIELD_ENCRYPTION_IV: u8 = 7;
const OUTER_FIELD_KDF_PARAMETERS: u8 = 11;
const OUTER_FIELD_PUBLIC_CUSTOM_DATA: u8 = 12;

const INNER_FIELD_STREAM_ID: u8 = 1;
const INNER_FIELD_STREAM_KEY: u8 = 2;
const INNER_FIELD_BINARY: u8 = 3;

const INNER_STREAM_SALSA20: u32 = 2;
const INNER_STREAM_CHACHA20: u32 = 3;

/// AES256-CBC outer cipher UUID `31c1f2e6-bf71-4350-be58-05216afc5aff`.
const CIPHER_AES256_CBC: [u8; 16] = [
    0x31, 0xc1, 0xf2, 0xe6, 0xbf, 0x71, 0x43, 0x50, 0xbe, 0x58, 0x05, 0x21, 0x6a, 0xfc, 0x5a, 0xff,
];
/// ChaCha20 outer cipher UUID `d6038a2b-8b6f-4cb5-a524-339a31dbb59a`.
const CIPHER_CHACHA20: [u8; 16] = [
    0xd6, 0x03, 0x8a, 0x2b, 0x8b, 0x6f, 0x4c, 0xb5, 0xa5, 0x24, 0x33, 0x9a, 0x31, 0xdb, 0xb5, 0x9a,
];
/// AES128-CBC outer cipher UUID `61ab05a1-9464-41c3-8d74-3a563df8dd35`.
/// Rejected by name (VLT-PM49 §8.6): legal but rare in practice.
const CIPHER_AES128_CBC: [u8; 16] = [
    0x61, 0xab, 0x05, 0xa1, 0x94, 0x64, 0x41, 0xc3, 0x8d, 0x74, 0x3a, 0x56, 0x3d, 0xf8, 0xdd, 0x35,
];
/// Twofish-CBC outer cipher UUID `ad68f29f-576f-4bb9-a36a-d47af965346c`.
/// Rejected by name (VLT-PM49 §8.6): legal but rare in practice.
const CIPHER_TWOFISH_CBC: [u8; 16] = [
    0xad, 0x68, 0xf2, 0x9f, 0x57, 0x6f, 0x4b, 0xb9, 0xa3, 0x6a, 0xd4, 0x7a, 0xf9, 0x65, 0x34, 0x6c,
];

/// Legacy AES-KDF UUID `c9d9f39a-628a-4460-bf74-0d08c18a4fea`. Rejected by
/// name (VLT-PM49 §8.6): superseded by Argon2d/Argon2id since KeePass 2.35.
const KDF_AES_KDF: [u8; 16] = [
    0xc9, 0xd9, 0xf3, 0x9a, 0x62, 0x8a, 0x44, 0x60, 0xbf, 0x74, 0x0d, 0x08, 0xc1, 0x8a, 0x4f, 0xea,
];
/// Argon2d KDF UUID `ef636ddf-8c29-444b-91f7-a9a403e30a0c`.
const KDF_ARGON2D: [u8; 16] = [
    0xef, 0x63, 0x6d, 0xdf, 0x8c, 0x29, 0x44, 0x4b, 0x91, 0xf7, 0xa9, 0xa4, 0x03, 0xe3, 0x0a, 0x0c,
];
/// Argon2id KDF UUID `9e298b19-56db-4773-b23d-fc3ec6f0a1e6`.
const KDF_ARGON2ID: [u8; 16] = [
    0x9e, 0x29, 0x8b, 0x19, 0x56, 0xdb, 0x47, 0x73, 0xb2, 0x3d, 0xfc, 0x3e, 0xc6, 0xf0, 0xa1, 0xe6,
];

// ============================================================================
// Section 3: A minimal cursor reader over the (already bounded) container
// ============================================================================

/// A cursor over a byte slice that never allocates and never reads past
/// what is actually present. Every multi-byte read is checked against the
/// remaining length *before* slicing, so a crafted field declaring a huge
/// length simply runs out of buffer and returns a structural [`ImportError`]
/// rather than attempting to materialize an oversized read.
struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ImportError> {
        if n > self.remaining() {
            return Err(truncated());
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, ImportError> {
        Ok(self.take(1)?[0])
    }

    fn u16_le(&mut self) -> Result<u16, ImportError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32_le(&mut self) -> Result<u32, ImportError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes(b.try_into().expect("take(4) returns 4 bytes")))
    }
}

fn truncated() -> ImportError {
    ImportError::Decode("truncated KDBX container")
}

/// The one, deliberately non-distinguishing failure for "the password is
/// wrong" and "the file is corrupt" (VLT-PM49 §8.3): a KDBX file gives no
/// way to tell these apart more precisely than "an authenticated hash
/// didn't match," so this crate does not invent a finer-grained answer.
fn wrong_password_or_corrupt() -> ImportError {
    ImportError::Adapter("wrong password or corrupt file".to_owned())
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

// ============================================================================
// Section 4: VariantDictionary (VLT-PM49 §8.2)
// ============================================================================

// Bool/Int32/Int64/String's inner values are never read by production code
// (no field this crate consumes uses those type tags), only reconstructed
// by the test-only encoder and asserted on by `variant_dictionary_round_
// trips_every_type_tag` -- but a real file's `KdfParameters`/
// `PublicCustomData` may still legally contain one, and the parser must
// accept (not choke on) every tag in the VariantDictionary type-tag space
// (VLT-PM49 §8.2), so all seven tags stay modeled here.
#[derive(Clone, Debug)]
#[allow(dead_code)]
enum VariantValue {
    UInt32(u32),
    UInt64(u64),
    Bool(bool),
    Int32(i32),
    Int64(i64),
    String(String),
    ByteArray(Vec<u8>),
}

type VariantDictionary = BTreeMap<String, VariantValue>;

fn parse_variant_dictionary(data: &[u8]) -> Result<VariantDictionary, ImportError> {
    let mut r = ByteReader::new(data);
    let version = r.u16_le()?;
    if (version >> 8) != 1 {
        return Err(ImportError::Decode("unsupported VariantDictionary version"));
    }
    let mut map = BTreeMap::new();
    loop {
        let type_tag = r.u8()?;
        if type_tag == 0 {
            break;
        }
        if map.len() >= MAX_VARIANT_ENTRIES {
            return Err(ImportError::TooLarge("MAX_VARIANT_ENTRIES"));
        }
        let key_len = r.u32_le()? as usize;
        let key_bytes = r.take(key_len)?;
        let key = core::str::from_utf8(key_bytes)
            .map_err(|_| ImportError::Decode("VariantDictionary key is not valid UTF-8"))?
            .to_owned();
        let value_len = r.u32_le()? as usize;
        let value_bytes = r.take(value_len)?;
        let value = match type_tag {
            0x04 => VariantValue::UInt32(u32::from_le_bytes(
                value_bytes
                    .try_into()
                    .map_err(|_| ImportError::Decode("VariantDictionary UInt32 must be 4 bytes"))?,
            )),
            0x05 => VariantValue::UInt64(u64::from_le_bytes(
                value_bytes
                    .try_into()
                    .map_err(|_| ImportError::Decode("VariantDictionary UInt64 must be 8 bytes"))?,
            )),
            0x08 => {
                if value_len != 1 {
                    return Err(ImportError::Decode("VariantDictionary Bool must be 1 byte"));
                }
                VariantValue::Bool(value_bytes[0] != 0)
            }
            0x0C => VariantValue::Int32(i32::from_le_bytes(
                value_bytes
                    .try_into()
                    .map_err(|_| ImportError::Decode("VariantDictionary Int32 must be 4 bytes"))?,
            )),
            0x0D => VariantValue::Int64(i64::from_le_bytes(
                value_bytes
                    .try_into()
                    .map_err(|_| ImportError::Decode("VariantDictionary Int64 must be 8 bytes"))?,
            )),
            0x18 => VariantValue::String(
                String::from_utf8(value_bytes.to_vec())
                    .map_err(|_| ImportError::Decode("VariantDictionary string is not valid UTF-8"))?,
            ),
            0x42 => VariantValue::ByteArray(value_bytes.to_vec()),
            _ => return Err(ImportError::Decode("unrecognized VariantDictionary type tag")),
        };
        map.insert(key, value);
    }
    Ok(map)
}

// ============================================================================
// Section 5: Outer header (VLT-PM49 §8.1, §8.2)
// ============================================================================

struct OuterHeaderFields {
    cipher_id: [u8; 16],
    compression: u32,
    main_seed: [u8; 32],
    iv: Vec<u8>,
    kdf_params: VariantDictionary,
}

fn parse_outer_header_fields(r: &mut ByteReader<'_>) -> Result<OuterHeaderFields, ImportError> {
    let mut cipher_id: Option<[u8; 16]> = None;
    let mut compression: Option<u32> = None;
    let mut main_seed: Option<[u8; 32]> = None;
    let mut iv: Option<Vec<u8>> = None;
    let mut kdf_params: Option<VariantDictionary> = None;

    loop {
        let field_id = r.u8()?;
        let len = r.u32_le()? as usize;
        let value = r.take(len)?;
        match field_id {
            0 => break,
            OUTER_FIELD_CIPHER_ID => {
                if len != 16 {
                    return Err(ImportError::Decode("CipherID must be 16 bytes"));
                }
                cipher_id = Some(value.try_into().expect("checked len == 16"));
            }
            OUTER_FIELD_COMPRESSION => {
                if len != 4 {
                    return Err(ImportError::Decode("CompressionFlags must be 4 bytes"));
                }
                compression = Some(u32::from_le_bytes(value.try_into().expect("checked len == 4")));
            }
            OUTER_FIELD_MAIN_SEED => {
                if len != 32 {
                    return Err(ImportError::Decode("MainSeed must be 32 bytes"));
                }
                main_seed = Some(value.try_into().expect("checked len == 32"));
            }
            OUTER_FIELD_ENCRYPTION_IV => {
                iv = Some(value.to_vec());
            }
            OUTER_FIELD_KDF_PARAMETERS => {
                kdf_params = Some(parse_variant_dictionary(value)?);
            }
            // PublicCustomData: parsed enough to skip over (already consumed
            // via `take(len)` above), never interpreted (VLT-PM49 §8.6) --
            // no first-party KeePass plugin data has a vault-pm equivalent.
            OUTER_FIELD_PUBLIC_CUSTOM_DATA => {}
            // Any other/unknown field ID: forward-compatible skip.
            _ => {}
        }
    }

    Ok(OuterHeaderFields {
        cipher_id: cipher_id.ok_or(ImportError::Decode("outer header missing CipherID"))?,
        compression: compression.ok_or(ImportError::Decode("outer header missing CompressionFlags"))?,
        main_seed: main_seed.ok_or(ImportError::Decode("outer header missing MainSeed"))?,
        iv: iv.ok_or(ImportError::Decode("outer header missing EncryptionIV"))?,
        kdf_params: kdf_params.ok_or(ImportError::Decode("outer header missing KdfParameters"))?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OuterCipher {
    Aes256Cbc,
    ChaCha20,
}

impl OuterCipher {
    fn iv_len(self) -> usize {
        match self {
            OuterCipher::Aes256Cbc => 16,
            OuterCipher::ChaCha20 => 12,
        }
    }
}

fn resolve_outer_cipher(id: &[u8; 16]) -> Result<OuterCipher, ImportError> {
    if *id == CIPHER_AES256_CBC {
        Ok(OuterCipher::Aes256Cbc)
    } else if *id == CIPHER_CHACHA20 {
        Ok(OuterCipher::ChaCha20)
    } else if *id == CIPHER_AES128_CBC {
        Err(ImportError::Adapter(
            "AES128-CBC cipher is not supported; only AES256-CBC and ChaCha20 are accepted".to_owned(),
        ))
    } else if *id == CIPHER_TWOFISH_CBC {
        Err(ImportError::Adapter(
            "Twofish-CBC cipher is not supported; only AES256-CBC and ChaCha20 are accepted".to_owned(),
        ))
    } else {
        Err(ImportError::Decode("unrecognized outer CipherID"))
    }
}

// ============================================================================
// Section 6: KDF parameters, bounded before they are trusted (VLT-PM49 §8.2)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KdfVariant {
    Argon2d,
    Argon2id,
}

struct KdfConfig {
    variant: KdfVariant,
    salt: Vec<u8>,
    iterations: u32,
    memory_kib: u32,
    parallelism: u32,
    version: u32,
}

fn variant_bytes<'a>(vd: &'a VariantDictionary, key: &str) -> Option<&'a [u8]> {
    match vd.get(key) {
        Some(VariantValue::ByteArray(b)) => Some(b.as_slice()),
        _ => None,
    }
}

fn variant_u64(vd: &VariantDictionary, key: &str) -> Option<u64> {
    match vd.get(key) {
        Some(VariantValue::UInt64(v)) => Some(*v),
        _ => None,
    }
}

fn variant_u32(vd: &VariantDictionary, key: &str) -> Option<u32> {
    match vd.get(key) {
        Some(VariantValue::UInt32(v)) => Some(*v),
        _ => None,
    }
}

fn parse_kdf_config(vd: &VariantDictionary) -> Result<KdfConfig, ImportError> {
    let uuid_bytes =
        variant_bytes(vd, "$UUID").ok_or(ImportError::Decode("KdfParameters missing $UUID"))?;
    let uuid: [u8; 16] = uuid_bytes
        .try_into()
        .map_err(|_| ImportError::Decode("KdfParameters $UUID must be 16 bytes"))?;

    let variant = if uuid == KDF_ARGON2D {
        KdfVariant::Argon2d
    } else if uuid == KDF_ARGON2ID {
        KdfVariant::Argon2id
    } else if uuid == KDF_AES_KDF {
        return Err(ImportError::Adapter(
            "AES-KDF is not supported; re-save the database with Argon2d or Argon2id".to_owned(),
        ));
    } else {
        return Err(ImportError::Decode("unrecognized KDF $UUID"));
    };

    let salt = variant_bytes(vd, "S")
        .ok_or(ImportError::Decode("KdfParameters missing S"))?
        .to_vec();
    let memory_bytes = variant_u64(vd, "M").ok_or(ImportError::Decode("KdfParameters missing M"))?;
    let iterations = variant_u64(vd, "I").ok_or(ImportError::Decode("KdfParameters missing I"))?;
    let parallelism = variant_u32(vd, "P").ok_or(ImportError::Decode("KdfParameters missing P"))?;
    let version = variant_u32(vd, "V").ok_or(ImportError::Decode("KdfParameters missing V"))?;

    // The load-bearing security property (VLT-PM49 §8.2): every one of
    // these is attacker-controlled and read before the master key is
    // verified. Bound the *raw declared* value before any division or KDF
    // call -- a crafted `M` near `u64::MAX` bytes must never reach the
    // division below, let alone Argon2d/Argon2id.
    if memory_bytes > MAX_KDF_MEMORY_KIB * 1024 {
        return Err(ImportError::TooLarge("MAX_KDF_MEMORY_KIB"));
    }
    if iterations > MAX_KDF_ITERATIONS {
        return Err(ImportError::TooLarge("MAX_KDF_ITERATIONS"));
    }
    if parallelism > MAX_KDF_PARALLELISM {
        return Err(ImportError::TooLarge("MAX_KDF_PARALLELISM"));
    }
    if iterations == 0 || parallelism == 0 {
        return Err(ImportError::InvalidParameter(
            "KDF iterations and parallelism must each be at least 1",
        ));
    }

    // Safe now: `memory_bytes` is bounded above by MAX_KDF_MEMORY_KIB * 1024,
    // which itself fits comfortably in a u32 once divided by 1024.
    let memory_kib = (memory_bytes / 1024) as u32;

    Ok(KdfConfig {
        variant,
        salt,
        iterations: iterations as u32,
        memory_kib,
        parallelism,
        version,
    })
}

// ============================================================================
// Section 7: Key derivation (VLT-PM49 §8.3)
// ============================================================================

// Hook for tests to prove Argon2d/Argon2id is never invoked when a
// `KdfConfig` bound is violated (VLT-PM49 §10 gate 12), without needing a
// multi-gigabyte fixture to actually observe the effect. Thread-local (not
// a shared global) because `cargo test` runs each `#[test]` fn on its own
// thread, so a global counter would be racy across concurrently running
// tests.
#[cfg(test)]
thread_local! {
    static KDF_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_kdf_call() {
    KDF_CALLS.with(|c| c.set(c.get() + 1));
}

#[cfg(not(test))]
fn record_kdf_call() {}

struct DerivedKeys {
    encryption_key: Zeroizing<[u8; 32]>,
    hmac_key_base: Zeroizing<[u8; 64]>,
}

fn derive_keys(
    password: &Zeroizing<Vec<u8>>,
    main_seed: &[u8; 32],
    kdf: &KdfConfig,
) -> Result<DerivedKeys, ImportError> {
    let password_hash: Zeroizing<[u8; 32]> = Zeroizing::new(coding_adventures_sha256::sha256(password));
    let composite_key: Zeroizing<[u8; 32]> =
        Zeroizing::new(coding_adventures_sha256::sha256(&password_hash[..]));

    const TAG_LENGTH: u32 = 32;
    record_kdf_call();
    let derived_bytes: Vec<u8> = match kdf.variant {
        KdfVariant::Argon2d => {
            let opts = coding_adventures_argon2d::Options {
                key: None,
                associated_data: None,
                version: Some(kdf.version),
            };
            coding_adventures_argon2d::argon2d(
                &composite_key[..],
                &kdf.salt,
                kdf.iterations,
                kdf.memory_kib,
                kdf.parallelism,
                TAG_LENGTH,
                &opts,
            )
            .map_err(|e| ImportError::Adapter(format!("Argon2d key derivation failed: {e}")))?
        }
        KdfVariant::Argon2id => {
            let opts = coding_adventures_argon2id::Options {
                key: None,
                associated_data: None,
                version: Some(kdf.version),
            };
            coding_adventures_argon2id::argon2id(
                &composite_key[..],
                &kdf.salt,
                kdf.iterations,
                kdf.memory_kib,
                kdf.parallelism,
                TAG_LENGTH,
                &opts,
            )
            .map_err(|e| ImportError::Adapter(format!("Argon2id key derivation failed: {e}")))?
        }
    };
    let derived_key: Zeroizing<Vec<u8>> = Zeroizing::new(derived_bytes);

    let mut enc_input: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(32 + derived_key.len()));
    enc_input.extend_from_slice(main_seed);
    enc_input.extend_from_slice(&derived_key);
    let encryption_key: Zeroizing<[u8; 32]> =
        Zeroizing::new(coding_adventures_sha256::sha256(&enc_input));

    let mut hmac_input: Zeroizing<Vec<u8>> =
        Zeroizing::new(Vec::with_capacity(32 + derived_key.len() + 1));
    hmac_input.extend_from_slice(main_seed);
    hmac_input.extend_from_slice(&derived_key);
    hmac_input.push(0x01);
    let hmac_key_base: Zeroizing<[u8; 64]> =
        Zeroizing::new(coding_adventures_sha512::sum512(&hmac_input));

    Ok(DerivedKeys {
        encryption_key,
        hmac_key_base,
    })
}

/// `block_hmac_key(N) = SHA-512(N as u64 LE || hmac_key_base)` (VLT-PM49
/// §8.3), the per-block (and, at `N = u64::MAX`, per-header) HMAC key.
fn block_hmac_key(index: u64, hmac_key_base: &[u8; 64]) -> Zeroizing<[u8; 64]> {
    let mut input: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(8 + 64));
    input.extend_from_slice(&index.to_le_bytes());
    input.extend_from_slice(hmac_key_base);
    Zeroizing::new(coding_adventures_sha512::sum512(&input))
}

// ============================================================================
// Section 8: Outer decryption and the HMAC-SHA256 block stream (§8.3)
// ============================================================================

fn decrypt_body(
    body: &[u8],
    cipher: OuterCipher,
    encryption_key: &[u8; 32],
    iv: &[u8],
) -> Result<Zeroizing<Vec<u8>>, ImportError> {
    match cipher {
        OuterCipher::Aes256Cbc => coding_adventures_aes_modes::cbc_decrypt(body, encryption_key, iv)
            .map(Zeroizing::new)
            .map_err(|_| wrong_password_or_corrupt()),
        OuterCipher::ChaCha20 => {
            let nonce: [u8; 12] = iv
                .try_into()
                .expect("ChaCha20 IV length already validated to be 12 bytes");
            Ok(Zeroizing::new(coding_adventures_chacha20_poly1305::chacha20_encrypt(
                body,
                encryption_key,
                &nonce,
                0,
            )))
        }
    }
}

/// Reassemble the HMAC-SHA256-authenticated block stream (VLT-PM49 §8.3).
/// Each block is `[32-byte HMAC][4-byte LE size N][N bytes data]`; a
/// terminal `N == 0` block ends the stream. Every block's HMAC is verified
/// in constant time *before* its data is appended, so a single corrupted
/// block anywhere -- not only the first -- is caught before any of the
/// reassembled buffer is trusted.
fn read_hmac_block_stream(
    decrypted: &[u8],
    hmac_key_base: &[u8; 64],
) -> Result<Zeroizing<Vec<u8>>, ImportError> {
    let mut r = ByteReader::new(decrypted);
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    let mut index: u64 = 0;
    loop {
        let stored_hmac = r.take(32)?;
        let size = r.u32_le()? as usize;
        let data = r.take(size)?;

        let key = block_hmac_key(index, hmac_key_base);
        let mut message: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(12 + data.len()));
        message.extend_from_slice(&index.to_le_bytes());
        message.extend_from_slice(&(size as u32).to_le_bytes());
        message.extend_from_slice(data);
        let computed = coding_adventures_hmac::hmac_sha256(&key[..], &message)
            .map_err(|_| wrong_password_or_corrupt())?;
        if !ct_eq(&computed, stored_hmac) {
            return Err(wrong_password_or_corrupt());
        }

        if size == 0 {
            break;
        }
        out.extend_from_slice(data);
        index += 1;
    }
    Ok(out)
}

// ============================================================================
// Section 9: gzip container stripping + bounded inflate (§8.1, §8.8)
// ============================================================================

/// Strip a minimal RFC 1952 gzip container down to its raw DEFLATE body.
/// KDBX's `CompressionFlags == 1` wraps the reassembled block-stream body
/// in gzip, not bare deflate, so this crate strips the container itself
/// (a small, bounded, in-house task) before handing the raw deflate stream
/// to `deflate::inflate_counted`. The trailing CRC32/ISIZE are not
/// verified: they are advisory in the original gzip design, and here the
/// body has already passed the HMAC-SHA256 block-stream authentication
/// (§8.3) -- a CRC32 check would add no security property this crate does
/// not already have.
fn strip_gzip_container(data: &[u8]) -> Result<&[u8], ImportError> {
    // 10-byte minimum header + 8-byte trailer, even for an empty payload.
    if data.len() < 18 {
        return Err(ImportError::Decode("gzip container too short"));
    }
    if data[0] != 0x1f || data[1] != 0x8b {
        return Err(ImportError::Decode("not a gzip container"));
    }
    if data[2] != 8 {
        return Err(ImportError::Decode("unsupported gzip compression method"));
    }
    let flags = data[3];
    let mut pos = 10usize;

    if flags & 0x04 != 0 {
        // FEXTRA
        if pos + 2 > data.len() {
            return Err(ImportError::Decode("truncated gzip FEXTRA length"));
        }
        let xlen = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        if pos + xlen > data.len() {
            return Err(ImportError::Decode("truncated gzip FEXTRA field"));
        }
        pos += xlen;
    }
    if flags & 0x08 != 0 {
        // FNAME
        pos = skip_null_terminated(data, pos)?;
    }
    if flags & 0x10 != 0 {
        // FCOMMENT
        pos = skip_null_terminated(data, pos)?;
    }
    if flags & 0x02 != 0 {
        // FHCRC
        if pos + 2 > data.len() {
            return Err(ImportError::Decode("truncated gzip FHCRC"));
        }
        pos += 2;
    }
    if pos + 8 > data.len() {
        return Err(ImportError::Decode("gzip container too short for its trailer"));
    }
    Ok(&data[pos..data.len() - 8])
}

fn skip_null_terminated(data: &[u8], mut pos: usize) -> Result<usize, ImportError> {
    loop {
        if pos >= data.len() {
            return Err(ImportError::Decode("truncated gzip container (unterminated field)"));
        }
        if data[pos] == 0 {
            return Ok(pos + 1);
        }
        pos += 1;
    }
}

fn decompress_body(data: &[u8], max_output: usize) -> Result<Zeroizing<Vec<u8>>, ImportError> {
    match deflate::inflate_counted(data, max_output as i64) {
        Ok(result) => Ok(Zeroizing::new(result.output)),
        Err(e)
            if e.code == deflate::InflateErrorCode::OutputLimitExceeded
                || e.code == deflate::InflateErrorCode::InvalidOutputLimit =>
        {
            Err(ImportError::TooLarge("MAX_DECOMPRESSED_BYTES"))
        }
        Err(_) => Err(ImportError::Decode("inner body failed to decompress")),
    }
}

// ============================================================================
// Section 10: Inner header (§8.1, §8.5, §8.6)
// ============================================================================

struct InnerHeaderFields {
    stream_id: u32,
    stream_key: Vec<u8>,
}

/// Parse the inner header, returning it plus the byte offset of the first
/// byte *after* its terminator -- i.e. where the inner XML document
/// begins.
fn parse_inner_header(data: &[u8]) -> Result<(InnerHeaderFields, usize), ImportError> {
    let mut r = ByteReader::new(data);
    let mut stream_id: Option<u32> = None;
    let mut stream_key: Option<Vec<u8>> = None;

    loop {
        let field_id = r.u8()?;
        let len = r.u32_le()? as usize;
        let value = r.take(len)?;
        match field_id {
            0 => break,
            INNER_FIELD_STREAM_ID => {
                if len != 4 {
                    return Err(ImportError::Decode("InnerRandomStreamID must be 4 bytes"));
                }
                stream_id = Some(u32::from_le_bytes(value.try_into().expect("checked len == 4")));
            }
            INNER_FIELD_STREAM_KEY => {
                if len != 32 && len != 64 {
                    return Err(ImportError::Decode(
                        "InnerRandomStreamKey must be 32 or 64 bytes",
                    ));
                }
                stream_key = Some(value.to_vec());
            }
            // Binary entries (attachment bytes): parsed past, never
            // surfaced (VLT-PM49 §8.5) -- already consumed via `take(len)`.
            INNER_FIELD_BINARY => {}
            _ => {}
        }
    }

    let stream_id = stream_id.ok_or(ImportError::Decode("inner header missing InnerRandomStreamID"))?;
    let stream_key =
        stream_key.ok_or(ImportError::Decode("inner header missing InnerRandomStreamKey"))?;
    Ok((InnerHeaderFields { stream_id, stream_key }, r.pos))
}

// ============================================================================
// Section 11: Base64 (VLT-PM49: no base64 crate anywhere in this workspace)
// ============================================================================

fn base64_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decode standard-alphabet Base64 with `+`/`/`/`=` padding. `Protected="True"`
/// `<Value>` text content is the only place this format uses Base64
/// (`MainSeed` and friends are raw bytes in the binary header).
fn decode_base64(s: &str) -> Result<Vec<u8>, ImportError> {
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.len().is_multiple_of(4) {
        return Err(ImportError::Decode("malformed base64 (length not a multiple of 4)"));
    }
    let chunk_count = bytes.len() / 4;
    let mut out = Vec::with_capacity(chunk_count * 3);
    for (chunk_index, chunk) in bytes.chunks(4).enumerate() {
        let is_last = chunk_index + 1 == chunk_count;
        if (chunk[0] == b'=') || (chunk[1] == b'=') {
            return Err(ImportError::Decode("malformed base64 (invalid padding)"));
        }
        let mut quad = [0u8; 4];
        let mut pad_count = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                if !is_last {
                    return Err(ImportError::Decode("malformed base64 (padding in a non-final block)"));
                }
                pad_count += 1;
            } else {
                if pad_count > 0 {
                    return Err(ImportError::Decode("malformed base64 (data after padding)"));
                }
                quad[i] = base64_value(b).ok_or(ImportError::Decode(
                    "malformed base64 (invalid character)",
                ))?;
            }
        }
        let n = ((quad[0] as u32) << 18)
            | ((quad[1] as u32) << 12)
            | ((quad[2] as u32) << 6)
            | (quad[3] as u32);
        out.push((n >> 16) as u8);
        if pad_count < 2 {
            out.push((n >> 8) as u8);
        }
        if pad_count == 0 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

// ============================================================================
// Section 12: Inner stream cipher key + protected-value decryption (§8.3)
// ============================================================================

struct InnerStreamKey {
    key: [u8; 32],
    nonce: [u8; 12],
}

impl Zeroize for InnerStreamKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
        self.nonce.zeroize();
    }
}

fn derive_inner_stream_key(raw_key: &[u8]) -> Zeroizing<InnerStreamKey> {
    let hash: Zeroizing<[u8; 64]> = Zeroizing::new(coding_adventures_sha512::sum512(raw_key));
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash[0..32]);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&hash[32..44]);
    Zeroizing::new(InnerStreamKey { key, nonce })
}

fn is_protected(value_el: &XmlElement) -> bool {
    value_el.get_attr(None, "Protected") == Some("True")
}

/// Walk every `<Entry>` at any depth (VLT-PM49 §8.4), collecting each
/// `Protected="True"` `<Value>`'s Base64-decoded ciphertext bytes in
/// document order. Must visit entries in exactly the same order
/// [`build_records`] does (including inside `<History>`, which is walked
/// for keystream-position purposes even though its content is never kept)
/// so the two passes' cursor positions agree.
fn collect_protected_ciphertexts(el: &XmlElement, out: &mut Vec<Vec<u8>>) -> Result<(), ImportError> {
    if el.local_name == "Entry" {
        for string_el in el.get_children(None, "String") {
            if let Some(value_el) = string_el.get_child(None, "Value") {
                if is_protected(value_el) {
                    out.push(decode_base64(&value_el.text_content())?);
                }
            }
        }
    }
    for child in &el.children {
        if let XmlNode::Element(child_el) = child {
            collect_protected_ciphertexts(child_el, out)?;
        }
    }
    Ok(())
}

/// Decrypt every collected protected-value ciphertext with one continuous
/// ChaCha20 keystream (VLT-PM49 §8.3): generate exactly as much keystream
/// as the cumulative ciphertext length requires by encrypting an all-zero
/// buffer of that length (`chacha20_encrypt` XORs its input against the
/// keystream, so an all-zero input *is* the keystream), then XOR each
/// ciphertext against its slice in order.
fn decrypt_protected_values(
    ciphertexts: Vec<Vec<u8>>,
    inner_key: &InnerStreamKey,
) -> Result<Vec<Zeroizing<String>>, ImportError> {
    let total: usize = ciphertexts.iter().map(|c| c.len()).sum();
    let zeros = vec![0u8; total];
    let keystream: Zeroizing<Vec<u8>> = Zeroizing::new(coding_adventures_chacha20_poly1305::chacha20_encrypt(
        &zeros,
        &inner_key.key,
        &inner_key.nonce,
        0,
    ));

    let mut out = Vec::with_capacity(ciphertexts.len());
    let mut offset = 0usize;
    for mut ciphertext in ciphertexts {
        let slice = &keystream[offset..offset + ciphertext.len()];
        for (byte, k) in ciphertext.iter_mut().zip(slice) {
            *byte ^= k;
        }
        offset += ciphertext.len();
        match String::from_utf8(ciphertext) {
            Ok(s) => out.push(Zeroizing::new(s)),
            Err(e) => {
                let mut leftover = e.into_bytes();
                leftover.zeroize();
                return Err(ImportError::Decode("protected value is not valid UTF-8"));
            }
        }
    }
    Ok(out)
}

// ============================================================================
// Section 13: Inner XML -> PortableRecord (VLT-PM49 §8.4, §8.5)
// ============================================================================

/// Walk every `<Entry>` at any depth, building a [`PortableRecord`] (or two,
/// for a TOTP field) per entry not nested inside `<History>`. Consumes the
/// decrypted protected values in the exact document order
/// [`collect_protected_ciphertexts`] produced them in, including walking
/// (but discarding the result of) entries inside `<History>` so the two
/// passes' positions never drift apart.
#[allow(clippy::too_many_arguments)]
fn build_records(
    el: &XmlElement,
    in_history: bool,
    cursor: &mut usize,
    decrypted: &[Zeroizing<String>],
    entries_seen: &mut usize,
    out: &mut Vec<PortableRecord>,
) -> Result<(), ImportError> {
    if el.local_name == "Entry" {
        *entries_seen += 1;
        if *entries_seen > MAX_ENTRIES {
            return Err(ImportError::TooLarge("MAX_ENTRIES"));
        }

        let mut title = String::new();
        let mut username: Option<String> = None;
        let mut password: Option<Zeroizing<String>> = None;
        let mut url: Option<String> = None;
        let mut notes: Option<String> = None;
        let mut totp_seed: Option<Zeroizing<String>> = None;
        let mut custom_fields: BTreeMap<String, Zeroizing<String>> = BTreeMap::new();

        for string_el in el.get_children(None, "String") {
            let (Some(key_el), Some(value_el)) =
                (string_el.get_child(None, "Key"), string_el.get_child(None, "Value"))
            else {
                continue;
            };
            let key = key_el.text_content();
            if key.len() > MAX_FIELD_LEN {
                return Err(ImportError::TooLarge("MAX_FIELD_LEN"));
            }

            let value: Zeroizing<String> = if is_protected(value_el) {
                let v = decrypted
                    .get(*cursor)
                    .map(|z| Zeroizing::new((**z).clone()))
                    .ok_or(ImportError::Decode("protected value stream underrun"))?;
                *cursor += 1;
                v
            } else {
                Zeroizing::new(value_el.text_content())
            };
            if value.len() > MAX_FIELD_LEN {
                return Err(ImportError::TooLarge("MAX_FIELD_LEN"));
            }

            match key.as_str() {
                "Title" => title = value.as_str().to_owned(),
                "UserName" => username = Some(value.as_str().to_owned()),
                "Password" => password = Some(value),
                "URL" => url = Some(value.as_str().to_owned()),
                "Notes" => notes = Some(value.as_str().to_owned()),
                "otp" | "TOTP Seed" => totp_seed = Some(value),
                _ => {
                    if custom_fields.len() >= MAX_CUSTOM_FIELDS_PER_ENTRY
                        && !custom_fields.contains_key(&key)
                    {
                        return Err(ImportError::TooLarge("MAX_CUSTOM_FIELDS_PER_ENTRY"));
                    }
                    custom_fields.insert(key, value);
                }
            }
        }

        if !in_history {
            let is_login_shaped = username.as_deref().is_some_and(|s| !s.is_empty())
                || password.as_deref().is_some_and(|s| !s.is_empty())
                || url.as_deref().is_some_and(|s| !s.is_empty());
            let entry_title = title.clone();
            if is_login_shaped {
                out.push(PortableRecord {
                    kind: PortableRecordKind::Login,
                    title: entry_title,
                    username,
                    password,
                    url,
                    notes,
                    totp_seed: None,
                    tags: Vec::new(),
                    custom_fields,
                });
            } else {
                out.push(PortableRecord {
                    kind: PortableRecordKind::SecureNote,
                    title: entry_title,
                    username: None,
                    password: None,
                    url: None,
                    notes,
                    totp_seed: None,
                    tags: Vec::new(),
                    custom_fields,
                });
            }
            if let Some(seed) = totp_seed {
                out.push(PortableRecord {
                    kind: PortableRecordKind::Totp,
                    title,
                    username: None,
                    password: None,
                    url: None,
                    notes: None,
                    totp_seed: Some(seed),
                    tags: Vec::new(),
                    custom_fields: BTreeMap::new(),
                });
            }
        }
    }

    for child in &el.children {
        if let XmlNode::Element(child_el) = child {
            let nested_in_history = in_history || el.local_name == "History";
            build_records(child_el, nested_in_history, cursor, decrypted, entries_seen, out)?;
        }
    }
    Ok(())
}

fn map_document_to_records(
    doc: &XmlDocument,
    inner_key: &InnerStreamKey,
) -> Result<Vec<PortableRecord>, ImportError> {
    let mut ciphertexts: Vec<Vec<u8>> = Vec::new();
    collect_protected_ciphertexts(&doc.root, &mut ciphertexts)?;
    let decrypted = decrypt_protected_values(ciphertexts, inner_key)?;

    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut entries_seen = 0usize;
    build_records(&doc.root, false, &mut cursor, &decrypted, &mut entries_seen, &mut out)?;
    Ok(out)
}

// ============================================================================
// Section 14: The public entry point (VLT-PM49 §8.7)
// ============================================================================

/// Decode a KDBX4 container into [`PortableRecord`]s.
///
/// `password` is the KDBX database's own master password (not vault-pm's).
/// Keyfile- or hardware-key-protected databases are out of scope
/// (VLT-PM49 §8.6): an empty password is passed through unchanged, which
/// simply fails the header HMAC check like any other wrong master key.
///
/// A wrong password and a byte-corrupted-but-structurally-valid file are
/// indistinguishable by construction (VLT-PM49 §8.3) and return the exact
/// same [`ImportError::Adapter`] message.
pub fn decode(container: &[u8], password: &Zeroizing<Vec<u8>>) -> Result<Vec<PortableRecord>, ImportError> {
    if container.len() > MAX_SOURCE_BYTES {
        return Err(ImportError::TooLarge("MAX_SOURCE_BYTES"));
    }
    if container.len() < 12 {
        return Err(ImportError::Decode("source is too short to be a KDBX file"));
    }

    let mut r = ByteReader::new(container);
    let sig1 = r.u32_le()?;
    let sig2 = r.u32_le()?;
    let _minor_version = r.u16_le()?;
    let major_version = r.u16_le()?;
    if sig1 != SIGNATURE_1 || sig2 != SIGNATURE_2 {
        return Err(ImportError::Decode("not a KDBX file (bad signature)"));
    }
    if major_version != REQUIRED_MAJOR_VERSION {
        // KDBX3 (major version 3) is explicitly out of scope (VLT-PM49
        // §8.6): a materially different outer format.
        return Err(ImportError::UnsupportedVersion(major_version as u32));
    }

    let outer = parse_outer_header_fields(&mut r)?;
    let header_end = r.pos;
    let header_bytes = &container[..header_end];

    let stored_header_hash = r.take(32)?;
    let computed_header_hash = coding_adventures_sha256::sha256(header_bytes);
    if !ct_eq(&computed_header_hash, stored_header_hash) {
        return Err(wrong_password_or_corrupt());
    }
    let stored_header_hmac = r.take(32)?.to_vec();

    let cipher = resolve_outer_cipher(&outer.cipher_id)?;
    if outer.iv.len() != cipher.iv_len() {
        return Err(ImportError::Decode(
            "EncryptionIV has the wrong length for the selected cipher",
        ));
    }

    let kdf = parse_kdf_config(&outer.kdf_params)?;
    let derived = derive_keys(password, &outer.main_seed, &kdf)?;

    let header_hmac_key = block_hmac_key(u64::MAX, &derived.hmac_key_base);
    let computed_header_hmac = coding_adventures_hmac::hmac_sha256(&header_hmac_key[..], header_bytes)
        .map_err(|_| wrong_password_or_corrupt())?;
    if !ct_eq(&computed_header_hmac, &stored_header_hmac) {
        return Err(wrong_password_or_corrupt());
    }

    let body = &container[r.pos..];
    let decrypted_body = decrypt_body(body, cipher, &derived.encryption_key, &outer.iv)?;
    let reassembled = read_hmac_block_stream(&decrypted_body, &derived.hmac_key_base)?;

    let inner_plain: Zeroizing<Vec<u8>> = match outer.compression {
        0 => reassembled,
        1 => {
            let deflate_body = strip_gzip_container(&reassembled)?;
            decompress_body(deflate_body, MAX_DECOMPRESSED_BYTES)?
        }
        _ => return Err(ImportError::Decode("unrecognized CompressionFlags value")),
    };

    let (inner_header, xml_start) = parse_inner_header(&inner_plain)?;
    if inner_header.stream_id == INNER_STREAM_SALSA20 {
        return Err(ImportError::Adapter(
            "Salsa20 inner stream is not supported; only ChaCha20 is accepted".to_owned(),
        ));
    }
    if inner_header.stream_id != INNER_STREAM_CHACHA20 {
        return Err(ImportError::Decode("unrecognized InnerRandomStreamID"));
    }

    let xml_bytes = &inner_plain[xml_start..];
    let xml_text = core::str::from_utf8(xml_bytes)
        .map_err(|_| ImportError::Decode("inner XML is not valid UTF-8"))?;
    let doc = parse_xml(xml_text).map_err(|_| ImportError::Decode("inner XML failed to parse"))?;

    let inner_key = derive_inner_stream_key(&inner_header.stream_key);
    map_document_to_records(&doc, &inner_key)
}

// ============================================================================
// Section 15: Tests
// ============================================================================

// ============================================================================
// Section 16: Test-only forward encoder (VLT-PM49 §8.9)
// ============================================================================

/// Test-only forward KDBX4 encoder (VLT-PM49 §8.9).
///
/// No real KeePass installation is available in this environment, so this
/// crate proves its own decoder against a fixture it builds itself,
/// composing the header, deriving the same keys, HMAC-framing the same
/// blocks, and ChaCha20-obfuscating the same protected values -- cross-
/// checked against the byte-level field layout, cipher UUIDs, and key-
/// derivation formulas transcribed independently in §8.1-§8.3 (not derived
/// from this file's own decoder).
///
/// Exposed behind the `test-fixtures` Cargo feature (in addition to this
/// crate's own `#[cfg(test)]` build) so a *consumer* of this crate --
/// `vault-pm-cli`'s own test suite -- can build a genuinely valid `.kdbx`
/// fixture to test its `import kdbx` wiring against, without duplicating
/// this byte-level encoder a second time. Everything below except
/// [`build_single_login_fixture`] is `pub(crate)`, reachable only from
/// inside this crate.
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_support {
    use super::*;

    pub(crate) fn encode_base64(data: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            out.push(ALPHABET[(n >> 18 & 0x3f) as usize] as char);
            out.push(ALPHABET[(n >> 12 & 0x3f) as usize] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6 & 0x3f) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[(n & 0x3f) as usize] as char
            } else {
                '='
            });
        }
        out
    }

    pub(crate) fn write_variant_dictionary(entries: &[(&str, VariantValue)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0x0100u16.to_le_bytes());
        for (key, value) in entries {
            let (tag, value_bytes): (u8, Vec<u8>) = match value {
                VariantValue::UInt32(v) => (0x04, v.to_le_bytes().to_vec()),
                VariantValue::UInt64(v) => (0x05, v.to_le_bytes().to_vec()),
                VariantValue::Bool(v) => (0x08, vec![u8::from(*v)]),
                VariantValue::Int32(v) => (0x0C, v.to_le_bytes().to_vec()),
                VariantValue::Int64(v) => (0x0D, v.to_le_bytes().to_vec()),
                VariantValue::String(v) => (0x18, v.as_bytes().to_vec()),
                VariantValue::ByteArray(v) => (0x42, v.clone()),
            };
            out.push(tag);
            out.extend_from_slice(&(key.len() as u32).to_le_bytes());
            out.extend_from_slice(key.as_bytes());
            out.extend_from_slice(&(value_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&value_bytes);
        }
        out.push(0);
        out
    }

    pub(crate) fn write_tlv_field(out: &mut Vec<u8>, field_id: u8, value: &[u8]) {
        out.push(field_id);
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
    }

    /// Compose the outer header (signature + version + TLV fields +
    /// terminator), independent of any key derivation, so bound-violation
    /// tests can build a header with a crafted `KdfParameters` without
    /// ever running a real (potentially huge) Argon2 call.
    pub(crate) fn write_outer_header(
        major_version: u16,
        cipher_id: [u8; 16],
        compression: u32,
        main_seed: [u8; 32],
        iv: &[u8],
        kdf_params: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&SIGNATURE_1.to_le_bytes());
        out.extend_from_slice(&SIGNATURE_2.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // minor version, unchecked
        out.extend_from_slice(&major_version.to_le_bytes());
        write_tlv_field(&mut out, OUTER_FIELD_CIPHER_ID, &cipher_id);
        write_tlv_field(&mut out, OUTER_FIELD_COMPRESSION, &compression.to_le_bytes());
        write_tlv_field(&mut out, OUTER_FIELD_MAIN_SEED, &main_seed);
        write_tlv_field(&mut out, OUTER_FIELD_ENCRYPTION_IV, iv);
        write_tlv_field(&mut out, OUTER_FIELD_KDF_PARAMETERS, kdf_params);
        write_tlv_field(&mut out, 0, &[]);
        out
    }

    /// One entry's fields, as source data for the encoder (not yet
    /// encrypted/base64'd).
    pub(crate) struct TestEntry {
        pub(crate) title: &'static str,
        pub(crate) username: Option<&'static str>,
        pub(crate) password: Option<&'static str>,
        pub(crate) url: Option<&'static str>,
        pub(crate) notes: Option<&'static str>,
        pub(crate) otp: Option<&'static str>,
        pub(crate) custom_fields: Vec<(&'static str, &'static str)>,
    }

    impl Default for TestEntry {
        fn default() -> Self {
            Self {
                title: "Untitled",
                username: None,
                password: None,
                url: None,
                notes: None,
                otp: None,
                custom_fields: Vec::new(),
            }
        }
    }

    /// Render one `<Entry>` XML fragment, base64-encoding and XOR-ing any
    /// protected value against the next slice of `keystream`, advancing
    /// `keystream_offset` as it goes -- so entries can be concatenated and
    /// the whole document decrypts against one continuous keystream,
    /// exactly like the real inner stream cipher (VLT-PM49 §8.3).
    pub(crate) fn render_entry_xml(
        entry: &TestEntry,
        keystream: &[u8],
        keystream_offset: &mut usize,
    ) -> String {
        let mut xml = String::from("<Entry>");
        let mut push_string = |xml: &mut String, key: &str, value: &str, protected: bool| {
            xml.push_str("<String><Key>");
            xml.push_str(key);
            xml.push_str("</Key><Value");
            if protected {
                xml.push_str(" Protected=\"True\">");
                let mut bytes = value.as_bytes().to_vec();
                let slice = &keystream[*keystream_offset..*keystream_offset + bytes.len()];
                for (b, k) in bytes.iter_mut().zip(slice) {
                    *b ^= k;
                }
                *keystream_offset += value.len();
                xml.push_str(&encode_base64(&bytes));
            } else {
                xml.push('>');
                xml.push_str(value);
            }
            xml.push_str("</Value></String>");
        };
        push_string(&mut xml, "Title", entry.title, false);
        if let Some(v) = entry.username {
            push_string(&mut xml, "UserName", v, false);
        }
        if let Some(v) = entry.password {
            push_string(&mut xml, "Password", v, true);
        }
        if let Some(v) = entry.url {
            push_string(&mut xml, "URL", v, false);
        }
        if let Some(v) = entry.notes {
            push_string(&mut xml, "Notes", v, false);
        }
        if let Some(v) = entry.otp {
            push_string(&mut xml, "otp", v, true);
        }
        for (key, value) in &entry.custom_fields {
            push_string(&mut xml, key, value, false);
        }
        xml.push_str("</Entry>");
        xml
    }

    /// Full options for the test encoder. `Default` produces a genuinely
    /// valid, decodable AES256-CBC + Argon2id + uncompressed, one-login-
    /// entry fixture; individual tests override just the field(s) they
    /// need to deviate on.
    pub(crate) struct EncodeOptions {
        pub(crate) password: Vec<u8>,
        pub(crate) major_version: u16,
        pub(crate) cipher_id: [u8; 16],
        pub(crate) compression: u32,
        pub(crate) kdf_uuid: [u8; 16],
        pub(crate) kdf_memory_bytes: u64,
        pub(crate) kdf_iterations: u64,
        pub(crate) kdf_parallelism: u32,
        pub(crate) kdf_argon_version: u32,
        pub(crate) inner_stream_id: u32,
        pub(crate) entries: Vec<TestEntry>,
        /// Bytes per real HMAC block (deliberately small so a typical
        /// fixture spans multiple blocks, letting a test corrupt a
        /// non-zero block index).
        pub(crate) block_size: usize,
    }

    impl Default for EncodeOptions {
        fn default() -> Self {
            Self {
                password: b"correct horse battery staple".to_vec(),
                major_version: 4,
                cipher_id: CIPHER_AES256_CBC,
                compression: 0,
                kdf_uuid: KDF_ARGON2ID,
                kdf_memory_bytes: 8 * 1024,
                kdf_iterations: 1,
                kdf_parallelism: 1,
                kdf_argon_version: 0x13,
                inner_stream_id: INNER_STREAM_CHACHA20,
                entries: vec![TestEntry {
                    title: "GitHub",
                    username: Some("alice"),
                    password: Some("hunter2-super-secret"),
                    url: Some("https://github.example"),
                    ..Default::default()
                }],
                block_size: 24,
            }
        }
    }

    /// The fully encoded container, plus enough layout metadata for tests
    /// that need to corrupt a specific byte precisely (VLT-PM49 §10 gates
    /// 11 and 14).
    // `body_offset`/`second_block_hmac_plaintext_offset` are only read by
    // this crate's own `#[cfg(test)] mod tests` (byte-precise corruption
    // tests, VLT-PM49 §10 gates 11/14); a `feature = "test-fixtures"`-only
    // build (no `cfg(test)`) never reads them, since
    // `build_single_login_fixture` only needs `bytes`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) struct EncodedFixture {
        pub(crate) bytes: Vec<u8>,
        /// Offset in `bytes` where the outer-encrypted body begins.
        pub(crate) body_offset: usize,
        /// Offset, within the *plaintext* reassembled block stream, of the
        /// second real (non-terminal) block's 32-byte HMAC field. `None`
        /// if fewer than two real blocks were produced.
        pub(crate) second_block_hmac_plaintext_offset: Option<usize>,
    }

    pub(crate) fn encode_kdbx4(opts: &EncodeOptions) -> EncodedFixture {
        let main_seed = [0x11u8; 32];
        let iv_len = if opts.cipher_id == CIPHER_CHACHA20 { 12 } else { 16 };
        let iv: Vec<u8> = (0u8..iv_len as u8).collect();
        let kdf_salt = [0x22u8; 16];

        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(opts.kdf_uuid.to_vec())),
            ("S", VariantValue::ByteArray(kdf_salt.to_vec())),
            ("M", VariantValue::UInt64(opts.kdf_memory_bytes)),
            ("I", VariantValue::UInt64(opts.kdf_iterations)),
            ("P", VariantValue::UInt32(opts.kdf_parallelism)),
            ("V", VariantValue::UInt32(opts.kdf_argon_version)),
        ]);

        let header_bytes = write_outer_header(
            opts.major_version,
            opts.cipher_id,
            opts.compression,
            main_seed,
            &iv,
            &kdf_params,
        );
        let header_hash = coding_adventures_sha256::sha256(&header_bytes);

        let kdf = KdfConfig {
            variant: if opts.kdf_uuid == KDF_ARGON2D {
                KdfVariant::Argon2d
            } else {
                KdfVariant::Argon2id
            },
            salt: kdf_salt.to_vec(),
            iterations: opts.kdf_iterations as u32,
            memory_kib: (opts.kdf_memory_bytes / 1024) as u32,
            parallelism: opts.kdf_parallelism,
            version: opts.kdf_argon_version,
        };
        let password = Zeroizing::new(opts.password.clone());
        let derived = derive_keys(&password, &main_seed, &kdf).expect("valid test fixture KDF params");

        let header_hmac_key = block_hmac_key(u64::MAX, &derived.hmac_key_base);
        let header_hmac = coding_adventures_hmac::hmac_sha256(&header_hmac_key[..], &header_bytes)
            .expect("hmac_key_base is always 64 bytes, never empty");

        // Inner stream key + a big-enough keystream to protect every
        // Password/otp field across every entry.
        let inner_stream_key_raw = [0x33u8; 64];
        let inner_key = derive_inner_stream_key(&inner_stream_key_raw);
        let total_protected_len: usize = opts
            .entries
            .iter()
            .flat_map(|e| [e.password, e.otp])
            .flatten()
            .map(str::len)
            .sum();
        let keystream = coding_adventures_chacha20_poly1305::chacha20_encrypt(
            &vec![0u8; total_protected_len],
            &inner_key.key,
            &inner_key.nonce,
            0,
        );

        let mut inner_header = Vec::new();
        write_tlv_field(&mut inner_header, INNER_FIELD_STREAM_ID, &opts.inner_stream_id.to_le_bytes());
        write_tlv_field(&mut inner_header, INNER_FIELD_STREAM_KEY, &inner_stream_key_raw);
        write_tlv_field(&mut inner_header, 0, &[]);

        let mut xml = String::from(
            "<KeePassFile><Meta><Generator>vault-import-keepass-test-encoder</Generator></Meta><Root><Group><Name>Root</Name>",
        );
        let mut keystream_offset = 0usize;
        for entry in &opts.entries {
            xml.push_str(&render_entry_xml(entry, &keystream, &mut keystream_offset));
        }
        xml.push_str("</Group></Root></KeePassFile>");

        let mut inner_plain = inner_header;
        inner_plain.extend_from_slice(xml.as_bytes());

        let reassembled = if opts.compression == 1 {
            gzip_wrap(&deflate::compress(&inner_plain).expect("deflate::compress on a small fixture"))
        } else {
            inner_plain
        };

        let (block_stream, second_block_hmac_plaintext_offset) =
            write_hmac_block_stream(&reassembled, &derived.hmac_key_base, opts.block_size);

        let body = match resolve_outer_cipher(&opts.cipher_id) {
            Ok(OuterCipher::Aes256Cbc) => {
                coding_adventures_aes_modes::cbc_encrypt(&block_stream, &derived.encryption_key[..], &iv)
                    .expect("valid CBC key/iv in test fixture")
            }
            Ok(OuterCipher::ChaCha20) => {
                let nonce: [u8; 12] = iv.as_slice().try_into().expect("chacha20 iv is 12 bytes");
                coding_adventures_chacha20_poly1305::chacha20_encrypt(
                    &block_stream,
                    &derived.encryption_key,
                    &nonce,
                    0,
                )
            }
            // Rejected-cipher fixtures (AES128/Twofish) never reach a real
            // decrypt in the tests that use them -- decode() fails at
            // cipher resolution first. Fall back to a fixed-size all-zero
            // ciphertext of plausible shape so `bytes` is at least
            // structurally well-formed.
            Err(_) => vec![0u8; block_stream.len().div_ceil(16) * 16],
        };

        let mut bytes = header_bytes;
        bytes.extend_from_slice(&header_hash);
        bytes.extend_from_slice(&header_hmac);
        let body_offset = bytes.len();
        bytes.extend_from_slice(&body);

        EncodedFixture {
            bytes,
            body_offset,
            second_block_hmac_plaintext_offset,
        }
    }

    pub(crate) fn write_hmac_block_stream(
        data: &[u8],
        hmac_key_base: &[u8; 64],
        block_size: usize,
    ) -> (Vec<u8>, Option<usize>) {
        let mut out = Vec::new();
        let mut second_block_hmac_offset = None;
        let mut index: u64 = 0;
        for (block_index, chunk) in data.chunks(block_size.max(1)).enumerate() {
            if block_index == 1 {
                second_block_hmac_offset = Some(out.len());
            }
            let key = block_hmac_key(index, hmac_key_base);
            let mut message = Vec::with_capacity(12 + chunk.len());
            message.extend_from_slice(&index.to_le_bytes());
            message.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            message.extend_from_slice(chunk);
            let hmac = coding_adventures_hmac::hmac_sha256(&key[..], &message)
                .expect("hmac_key_base is always non-empty");
            out.extend_from_slice(&hmac);
            out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            out.extend_from_slice(chunk);
            index += 1;
        }
        // Terminal empty block.
        let key = block_hmac_key(index, hmac_key_base);
        let message = index.to_le_bytes().to_vec(); // size=0, no data
        let mut full_message = message;
        full_message.extend_from_slice(&0u32.to_le_bytes());
        let hmac = coding_adventures_hmac::hmac_sha256(&key[..], &full_message)
            .expect("hmac_key_base is always non-empty");
        out.extend_from_slice(&hmac);
        out.extend_from_slice(&0u32.to_le_bytes());
        (out, second_block_hmac_offset)
    }

    /// Wrap a raw deflate stream in a minimal RFC 1952 gzip container. The
    /// CRC32/ISIZE trailer is never verified by [`strip_gzip_container`],
    /// so this writes zeroed placeholders rather than a real CRC32 (no
    /// CRC32 implementation exists in this workspace, and none is needed:
    /// see that function's doc comment).
    pub(crate) fn gzip_wrap(deflate_body: &[u8]) -> Vec<u8> {
        let mut out = vec![0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0, 0xff];
        out.extend_from_slice(deflate_body);
        out.extend_from_slice(&0u32.to_le_bytes()); // CRC32 placeholder
        out.extend_from_slice(&0u32.to_le_bytes()); // ISIZE placeholder
        out
    }

    /// Build a minimal, genuinely valid, single-login-entry KDBX4 fixture
    /// (AES256-CBC, Argon2id, uncompressed) for a caller -- such as
    /// `vault-pm-cli`'s own test suite -- that needs a real decodable
    /// `.kdbx` file but not this crate's own internal cipher/KDF/
    /// compression test matrix. The narrowest public surface this module
    /// exposes: everything else above stays `pub(crate)`, reachable only
    /// from this crate's own `#[cfg(test)] mod tests`.
    pub fn build_single_login_fixture(
        password: &[u8],
        title: &'static str,
        username: &'static str,
        entry_password: &'static str,
    ) -> Vec<u8> {
        let opts = EncodeOptions {
            password: password.to_vec(),
            entries: vec![TestEntry {
                title,
                username: Some(username),
                password: Some(entry_password),
                ..Default::default()
            }],
            ..Default::default()
        };
        encode_kdbx4(&opts).bytes
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    fn pw(bytes: &[u8]) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(bytes.to_vec())
    }

    // --- Happy path: full matrix -----------------------------------------

    #[test]
    fn round_trips_across_the_full_cipher_kdf_compression_matrix() {
        for cipher_id in [CIPHER_AES256_CBC, CIPHER_CHACHA20] {
            for kdf_uuid in [KDF_ARGON2D, KDF_ARGON2ID] {
                for compression in [0u32, 1u32] {
                    let opts = EncodeOptions {
                        cipher_id,
                        kdf_uuid,
                        compression,
                        ..Default::default()
                    };
                    let fixture = encode_kdbx4(&opts);
                    let records = decode(&fixture.bytes, &pw(b"correct horse battery staple"))
                        .unwrap_or_else(|e| {
                            panic!("cipher={cipher_id:?} kdf={kdf_uuid:?} compression={compression}: {e}")
                        });
                    assert_eq!(records.len(), 1);
                    assert_eq!(records[0].kind, PortableRecordKind::Login);
                    assert_eq!(records[0].title, "GitHub");
                    assert_eq!(records[0].username.as_deref(), Some("alice"));
                    assert_eq!(
                        records[0].password.as_deref().map(String::as_str),
                        Some("hunter2-super-secret")
                    );
                    assert_eq!(records[0].url.as_deref(), Some("https://github.example"));
                }
            }
        }
    }

    // --- Entry-kind mapping ------------------------------------------------

    #[test]
    fn notes_only_entry_becomes_a_secure_note() {
        let opts = EncodeOptions {
            entries: vec![TestEntry {
                title: "Wifi",
                notes: Some("network password: hunter2"),
                ..Default::default()
            }],
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let records = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, PortableRecordKind::SecureNote);
        assert_eq!(records[0].notes.as_deref(), Some("network password: hunter2"));
    }

    #[test]
    fn otp_field_becomes_a_second_totp_record() {
        let opts = EncodeOptions {
            entries: vec![TestEntry {
                title: "AWS Root",
                username: Some("root"),
                password: Some("pw"),
                otp: Some("JBSWY3DPEHPK3PXP"),
                ..Default::default()
            }],
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let records = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].kind, PortableRecordKind::Login);
        assert_eq!(records[1].kind, PortableRecordKind::Totp);
        assert_eq!(records[1].title, "AWS Root");
        assert_eq!(
            records[1].totp_seed.as_deref().map(String::as_str),
            Some("JBSWY3DPEHPK3PXP")
        );
    }

    #[test]
    fn totp_seed_field_name_variant_is_also_recognized() {
        let opts = EncodeOptions {
            entries: vec![TestEntry {
                title: "Legacy",
                username: Some("bob"),
                password: Some("pw"),
                custom_fields: vec![("TOTP Seed", "JBSWY3DPEHPK3PXP")],
                ..Default::default()
            }],
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let records = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].kind, PortableRecordKind::Totp);
    }

    #[test]
    fn unrecognized_string_keys_become_custom_fields() {
        let opts = EncodeOptions {
            entries: vec![TestEntry {
                title: "Service",
                username: Some("carol"),
                password: Some("pw"),
                custom_fields: vec![("api_key", "sk-abc123"), ("region", "us-east-1")],
                ..Default::default()
            }],
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let records = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].custom_fields.get("api_key").map(|z| (**z).clone()),
            Some("sk-abc123".to_owned())
        );
        assert_eq!(
            records[0].custom_fields.get("region").map(|z| (**z).clone()),
            Some("us-east-1".to_owned())
        );
    }

    #[test]
    fn multiple_entries_all_decode() {
        let opts = EncodeOptions {
            entries: vec![
                TestEntry {
                    title: "One",
                    username: Some("a"),
                    password: Some("pw1"),
                    ..Default::default()
                },
                TestEntry {
                    title: "Two",
                    username: Some("b"),
                    password: Some("pw2"),
                    ..Default::default()
                },
                TestEntry {
                    title: "Three",
                    notes: Some("just a note"),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let records = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].title, "One");
        assert_eq!(records[1].title, "Two");
        assert_eq!(records[2].kind, PortableRecordKind::SecureNote);
    }

    // --- Wrong password / corruption indistinguishability (gate 11) -------

    #[test]
    fn wrong_password_and_bit_corruption_produce_the_identical_error_message() {
        let fixture = encode_kdbx4(&EncodeOptions::default());

        let wrong_password_err =
            decode(&fixture.bytes, &pw(b"not the right password")).unwrap_err();

        let mut corrupted = fixture.bytes.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0xFF;
        let corrupted_err = decode(&corrupted, &pw(b"correct horse battery staple")).unwrap_err();

        let ImportError::Adapter(wrong_password_message) = wrong_password_err else {
            panic!("expected Adapter error for wrong password, got {wrong_password_err:?}");
        };
        let ImportError::Adapter(corrupted_message) = corrupted_err else {
            panic!("expected Adapter error for corruption, got {corrupted_err:?}");
        };
        assert_eq!(wrong_password_message, corrupted_message);
        assert_eq!(wrong_password_message, "wrong password or corrupt file");
    }

    #[test]
    fn header_integrity_hash_mismatch_is_wrong_password_or_corrupt() {
        let mut fixture = encode_kdbx4(&EncodeOptions::default());
        // The header hash immediately follows the header bytes; corrupt
        // its first byte.
        let header_len = fixture.body_offset - 64;
        fixture.bytes[header_len] ^= 0xFF;
        let err = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m == "wrong password or corrupt file"));
    }

    // --- Block-stream HMAC failure at a non-zero index (gate 14) ----------

    #[test]
    fn block_stream_hmac_failure_at_a_non_zero_index_is_rejected() {
        let opts = EncodeOptions {
            cipher_id: CIPHER_CHACHA20, // stream cipher: ciphertext offset == plaintext offset
            entries: vec![TestEntry {
                title: "Long Enough Entry To Span Multiple Blocks",
                username: Some("someone-with-a-fairly-long-username-value"),
                password: Some("a-fairly-long-password-value-to-force-more-blocks"),
                url: Some("https://example.test/a/pretty/long/path/value"),
                notes: Some("some notes text that adds more bytes to the body"),
                ..Default::default()
            }],
            block_size: 24,
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let plaintext_offset = fixture
            .second_block_hmac_plaintext_offset
            .expect("fixture must contain at least two real blocks");

        let mut corrupted = fixture.bytes.clone();
        let ciphertext_offset = fixture.body_offset + plaintext_offset;
        corrupted[ciphertext_offset] ^= 0xFF;

        let err = decode(&corrupted, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m == "wrong password or corrupt file"));
    }

    // --- KDF cost ceilings, proven never-called (gate 12) ------------------

    fn kdf_calls() -> usize {
        KDF_CALLS.with(|c| c.get())
    }

    #[test]
    fn kdf_memory_ceiling_is_enforced_before_the_kdf_is_ever_called() {
        let before = kdf_calls();
        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(KDF_ARGON2ID.to_vec())),
            ("S", VariantValue::ByteArray(vec![0x22u8; 16])),
            ("M", VariantValue::UInt64(MAX_KDF_MEMORY_KIB * 1024 + 1)),
            ("I", VariantValue::UInt64(1)),
            ("P", VariantValue::UInt32(1)),
            ("V", VariantValue::UInt32(0x13)),
        ]);
        let header = write_outer_header(4, CIPHER_AES256_CBC, 0, [0x11; 32], &[0u8; 16], &kdf_params);
        let mut bytes = header.clone();
        bytes.extend_from_slice(&coding_adventures_sha256::sha256(&header));
        bytes.extend_from_slice(&[0u8; 32]); // never-checked placeholder HMAC

        let err = decode(&bytes, &pw(b"anything")).unwrap_err();
        assert!(matches!(err, ImportError::TooLarge("MAX_KDF_MEMORY_KIB")));
        assert_eq!(kdf_calls(), before, "Argon2 must never be invoked");
    }

    #[test]
    fn kdf_iterations_ceiling_is_enforced_before_the_kdf_is_ever_called() {
        let before = kdf_calls();
        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(KDF_ARGON2ID.to_vec())),
            ("S", VariantValue::ByteArray(vec![0x22u8; 16])),
            ("M", VariantValue::UInt64(8 * 1024)),
            ("I", VariantValue::UInt64(MAX_KDF_ITERATIONS + 1)),
            ("P", VariantValue::UInt32(1)),
            ("V", VariantValue::UInt32(0x13)),
        ]);
        let header = write_outer_header(4, CIPHER_AES256_CBC, 0, [0x11; 32], &[0u8; 16], &kdf_params);
        let mut bytes = header.clone();
        bytes.extend_from_slice(&coding_adventures_sha256::sha256(&header));
        bytes.extend_from_slice(&[0u8; 32]);

        let err = decode(&bytes, &pw(b"anything")).unwrap_err();
        assert!(matches!(err, ImportError::TooLarge("MAX_KDF_ITERATIONS")));
        assert_eq!(kdf_calls(), before, "Argon2 must never be invoked");
    }

    #[test]
    fn kdf_parallelism_ceiling_is_enforced_before_the_kdf_is_ever_called() {
        let before = kdf_calls();
        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(KDF_ARGON2ID.to_vec())),
            ("S", VariantValue::ByteArray(vec![0x22u8; 16])),
            ("M", VariantValue::UInt64(8 * 1024)),
            ("I", VariantValue::UInt64(1)),
            ("P", VariantValue::UInt32(MAX_KDF_PARALLELISM + 1)),
            ("V", VariantValue::UInt32(0x13)),
        ]);
        let header = write_outer_header(4, CIPHER_AES256_CBC, 0, [0x11; 32], &[0u8; 16], &kdf_params);
        let mut bytes = header.clone();
        bytes.extend_from_slice(&coding_adventures_sha256::sha256(&header));
        bytes.extend_from_slice(&[0u8; 32]);

        let err = decode(&bytes, &pw(b"anything")).unwrap_err();
        assert!(matches!(err, ImportError::TooLarge("MAX_KDF_PARALLELISM")));
        assert_eq!(kdf_calls(), before, "Argon2 must never be invoked");
    }

    // --- Named rejections (gate 13) ----------------------------------------

    #[test]
    fn kdbx3_major_version_is_rejected_by_version_number() {
        let opts = EncodeOptions {
            major_version: 3,
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let err = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::UnsupportedVersion(3)));
    }

    #[test]
    fn aes_kdf_is_rejected_by_name() {
        let opts = EncodeOptions {
            kdf_uuid: KDF_AES_KDF,
            ..Default::default()
        };
        // encode_kdbx4 defaults its internal KdfVariant selection to
        // Argon2id whenever kdf_uuid != KDF_ARGON2D, which would derive
        // real keys under the wrong label for an AES-KDF fixture -- but
        // decode() rejects the file by its declared $UUID before any of
        // that matters, so the mismatch is harmless here.
        let fixture = encode_kdbx4(&opts);
        let err = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m.contains("AES-KDF")));
    }

    #[test]
    fn aes128_cbc_outer_cipher_is_rejected_by_name() {
        let opts = EncodeOptions {
            cipher_id: CIPHER_AES128_CBC,
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let err = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m.contains("AES128-CBC")));
    }

    #[test]
    fn twofish_cbc_outer_cipher_is_rejected_by_name() {
        let opts = EncodeOptions {
            cipher_id: CIPHER_TWOFISH_CBC,
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let err = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m.contains("Twofish-CBC")));
    }

    #[test]
    fn salsa20_inner_stream_is_rejected_by_name() {
        let opts = EncodeOptions {
            inner_stream_id: INNER_STREAM_SALSA20,
            ..Default::default()
        };
        let fixture = encode_kdbx4(&opts);
        let err = decode(&fixture.bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m.contains("Salsa20")));
    }

    #[test]
    fn unsupported_argon2_version_is_rejected() {
        // Built from the low-level header pieces directly (like the KDF
        // bound-violation tests) rather than through `encode_kdbx4`'s full
        // pipeline: that pipeline calls the real `derive_keys` to build a
        // genuinely decodable fixture, which would itself panic on a
        // rejected Argon2 version before `decode()` ever got a chance to
        // reject it. `decode()` fails at key derivation here (after the
        // header hash check, before ever reaching the body), so no real
        // body is needed.
        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(KDF_ARGON2ID.to_vec())),
            ("S", VariantValue::ByteArray(vec![0x22u8; 16])),
            ("M", VariantValue::UInt64(8 * 1024)),
            ("I", VariantValue::UInt64(1)),
            ("P", VariantValue::UInt32(1)),
            ("V", VariantValue::UInt32(0x10)),
        ]);
        let header = write_outer_header(4, CIPHER_AES256_CBC, 0, [0x11; 32], &[0u8; 16], &kdf_params);
        let mut bytes = header.clone();
        bytes.extend_from_slice(&coding_adventures_sha256::sha256(&header));
        bytes.extend_from_slice(&[0u8; 32]); // never-checked placeholder HMAC

        let err = decode(&bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m.contains("only Argon2 v1.3")));
    }

    // --- Structural / malformed-input matrix (gate 15) ---------------------

    #[test]
    fn rejects_oversize_container() {
        let big = vec![0u8; MAX_SOURCE_BYTES + 1];
        let err = decode(&big, &pw(b"x")).unwrap_err();
        assert!(matches!(err, ImportError::TooLarge("MAX_SOURCE_BYTES")));
    }

    #[test]
    fn rejects_truncated_source() {
        let err = decode(&[0u8; 4], &pw(b"x")).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn rejects_bad_signature() {
        let bytes = vec![0u8; 20];
        let err = decode(&bytes, &pw(b"x")).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn rejects_truncated_header() {
        let fixture = encode_kdbx4(&EncodeOptions::default());
        // Cut off partway through the outer header, well before its
        // terminator.
        let truncated_bytes = &fixture.bytes[..20];
        let err = decode(truncated_bytes, &pw(b"correct horse battery staple")).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn rejects_truncated_block_stream() {
        let fixture = encode_kdbx4(&EncodeOptions::default());
        // Cut off a few bytes from the very end -- inside the encrypted
        // body, after the header is fully intact.
        let truncated_bytes = &fixture.bytes[..fixture.bytes.len() - 3];
        let err = decode(truncated_bytes, &pw(b"correct horse battery staple"));
        assert!(err.is_err());
    }

    #[test]
    fn rejects_invalid_utf8_in_decompressed_xml() {
        // Build a fixture the normal way, then splice invalid UTF-8 bytes
        // directly into the plaintext XML region before HMAC-framing and
        // encrypting it, so the corruption survives to the XML-decode
        // step rather than being caught earlier by an HMAC mismatch.
        let opts = EncodeOptions::default();
        let main_seed = [0x11u8; 32];
        let iv: Vec<u8> = (0u8..16).collect();
        let kdf_salt = [0x22u8; 16];
        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(KDF_ARGON2ID.to_vec())),
            ("S", VariantValue::ByteArray(kdf_salt.to_vec())),
            ("M", VariantValue::UInt64(8 * 1024)),
            ("I", VariantValue::UInt64(1)),
            ("P", VariantValue::UInt32(1)),
            ("V", VariantValue::UInt32(0x13)),
        ]);
        let header_bytes =
            write_outer_header(4, CIPHER_AES256_CBC, 0, main_seed, &iv, &kdf_params);
        let header_hash = coding_adventures_sha256::sha256(&header_bytes);
        let kdf = KdfConfig {
            variant: KdfVariant::Argon2id,
            salt: kdf_salt.to_vec(),
            iterations: 1,
            memory_kib: 8,
            parallelism: 1,
            version: 0x13,
        };
        let password = pw(b"correct horse battery staple");
        let derived = derive_keys(&password, &main_seed, &kdf).unwrap();
        let header_hmac_key = block_hmac_key(u64::MAX, &derived.hmac_key_base);
        let header_hmac = coding_adventures_hmac::hmac_sha256(&header_hmac_key[..], &header_bytes).unwrap();

        let inner_stream_key_raw = [0x33u8; 64];
        let mut inner_header = Vec::new();
        write_tlv_field(&mut inner_header, INNER_FIELD_STREAM_ID, &INNER_STREAM_CHACHA20.to_le_bytes());
        write_tlv_field(&mut inner_header, INNER_FIELD_STREAM_KEY, &inner_stream_key_raw);
        write_tlv_field(&mut inner_header, 0, &[]);
        let mut inner_plain = inner_header;
        // Invalid UTF-8: a lone continuation byte.
        inner_plain.extend_from_slice(&[0xC3, 0x28]);

        let (block_stream, _) = write_hmac_block_stream(&inner_plain, &derived.hmac_key_base, 24);
        let body =
            coding_adventures_aes_modes::cbc_encrypt(&block_stream, &derived.encryption_key[..], &iv)
                .unwrap();

        let mut bytes = header_bytes;
        bytes.extend_from_slice(&header_hash);
        bytes.extend_from_slice(&header_hmac);
        bytes.extend_from_slice(&body);
        let _ = opts; // silence unused-field warning if defaults change

        let err = decode(&bytes, &password).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn rejects_malformed_base64_in_a_protected_value() {
        // Same technique as the invalid-UTF-8 test: hand-build the inner
        // plaintext so the corruption lands inside the XML, past every
        // HMAC/hash check.
        let main_seed = [0x11u8; 32];
        let iv: Vec<u8> = (0u8..16).collect();
        let kdf_salt = [0x22u8; 16];
        let kdf_params = write_variant_dictionary(&[
            ("$UUID", VariantValue::ByteArray(KDF_ARGON2ID.to_vec())),
            ("S", VariantValue::ByteArray(kdf_salt.to_vec())),
            ("M", VariantValue::UInt64(8 * 1024)),
            ("I", VariantValue::UInt64(1)),
            ("P", VariantValue::UInt32(1)),
            ("V", VariantValue::UInt32(0x13)),
        ]);
        let header_bytes =
            write_outer_header(4, CIPHER_AES256_CBC, 0, main_seed, &iv, &kdf_params);
        let header_hash = coding_adventures_sha256::sha256(&header_bytes);
        let kdf = KdfConfig {
            variant: KdfVariant::Argon2id,
            salt: kdf_salt.to_vec(),
            iterations: 1,
            memory_kib: 8,
            parallelism: 1,
            version: 0x13,
        };
        let password = pw(b"correct horse battery staple");
        let derived = derive_keys(&password, &main_seed, &kdf).unwrap();
        let header_hmac_key = block_hmac_key(u64::MAX, &derived.hmac_key_base);
        let header_hmac = coding_adventures_hmac::hmac_sha256(&header_hmac_key[..], &header_bytes).unwrap();

        let inner_stream_key_raw = [0x33u8; 64];
        let mut inner_header = Vec::new();
        write_tlv_field(&mut inner_header, INNER_FIELD_STREAM_ID, &INNER_STREAM_CHACHA20.to_le_bytes());
        write_tlv_field(&mut inner_header, INNER_FIELD_STREAM_KEY, &inner_stream_key_raw);
        write_tlv_field(&mut inner_header, 0, &[]);

        let xml = "<KeePassFile><Root><Group><Entry><String><Key>Title</Key><Value>x</Value></String>\
                   <String><Key>Password</Key><Value Protected=\"True\">not-valid-base64!!</Value></String>\
                   </Entry></Group></Root></KeePassFile>";
        let mut inner_plain = inner_header;
        inner_plain.extend_from_slice(xml.as_bytes());

        let (block_stream, _) = write_hmac_block_stream(&inner_plain, &derived.hmac_key_base, 24);
        let body =
            coding_adventures_aes_modes::cbc_encrypt(&block_stream, &derived.encryption_key[..], &iv)
                .unwrap();

        let mut bytes = header_bytes;
        bytes.extend_from_slice(&header_hash);
        bytes.extend_from_slice(&header_hmac);
        bytes.extend_from_slice(&body);

        let err = decode(&bytes, &password).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn decompressed_size_at_and_past_the_ceiling_is_rejected_before_materializing() {
        // Exercises the private `decompress_body` helper directly with a
        // tiny ceiling, rather than the full pipeline with a real
        // MAX_DECOMPRESSED_BYTES-sized (256 MiB) fixture -- proving the
        // bound is enforced *during* inflation without needing gigabytes
        // of test data.
        let payload = vec![b'a'; 1_000];
        let compressed = deflate::compress(&payload).unwrap();
        // Exactly at the ceiling: still fits.
        let at_ceiling = decompress_body(&compressed, payload.len());
        assert!(at_ceiling.is_ok());
        // One byte past: rejected.
        let past_ceiling = decompress_body(&compressed, payload.len() - 1);
        assert!(matches!(past_ceiling, Err(ImportError::TooLarge("MAX_DECOMPRESSED_BYTES"))));
    }

    // --- gzip container stripper -------------------------------------------

    #[test]
    fn gzip_wrap_and_strip_round_trips() {
        let payload = b"hello, kdbx compressed body";
        let compressed = deflate::compress(payload).unwrap();
        let wrapped = gzip_wrap(&compressed);
        let stripped = strip_gzip_container(&wrapped).unwrap();
        assert_eq!(stripped, compressed.as_slice());
        let inflated = deflate::decompress(stripped).unwrap();
        assert_eq!(inflated, payload);
    }

    #[test]
    fn strip_gzip_container_rejects_bad_magic() {
        let err = strip_gzip_container(&[0u8; 32]).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn strip_gzip_container_rejects_too_short_input() {
        let err = strip_gzip_container(&[0x1f, 0x8b, 0x08, 0x00]).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    // --- base64 --------------------------------------------------------

    #[test]
    fn base64_round_trips() {
        for payload in [
            &b""[..],
            &b"f"[..],
            &b"fo"[..],
            &b"foo"[..],
            &b"foob"[..],
            &b"fooba"[..],
            &b"foobar"[..],
            &[0u8, 1, 2, 3, 4, 5, 255, 254][..],
        ] {
            let encoded = encode_base64(payload);
            let decoded = decode_base64(&encoded).unwrap();
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn base64_rejects_bad_length() {
        assert!(decode_base64("abc").is_err());
    }

    #[test]
    fn base64_rejects_invalid_character() {
        assert!(decode_base64("ab!=").is_err());
    }

    #[test]
    fn base64_rejects_padding_in_a_non_final_block() {
        assert!(decode_base64("ab==abcd").is_err());
    }

    #[test]
    fn base64_accepts_whitespace_between_groups() {
        let decoded = decode_base64("Zm9v\nYmFy").unwrap();
        assert_eq!(decoded, b"foobar");
    }

    // --- VariantDictionary ---------------------------------------------

    #[test]
    fn variant_dictionary_round_trips_every_type_tag() {
        let entries: Vec<(&str, VariantValue)> = vec![
            ("u32", VariantValue::UInt32(42)),
            ("u64", VariantValue::UInt64(1_000_000_000_000)),
            ("bool", VariantValue::Bool(true)),
            ("i32", VariantValue::Int32(-7)),
            ("i64", VariantValue::Int64(-9_000_000_000)),
            ("str", VariantValue::String("hello".to_owned())),
            ("bytes", VariantValue::ByteArray(vec![1, 2, 3])),
        ];
        let encoded = write_variant_dictionary(&entries);
        let decoded = parse_variant_dictionary(&encoded).unwrap();
        assert_eq!(decoded.len(), 7);
        assert!(matches!(decoded.get("u32"), Some(VariantValue::UInt32(42))));
        assert!(matches!(decoded.get("bool"), Some(VariantValue::Bool(true))));
        assert!(matches!(decoded.get("str"), Some(VariantValue::String(s)) if s == "hello"));
    }

    #[test]
    fn variant_dictionary_rejects_unknown_type_tag() {
        let mut bytes = 0x0100u16.to_le_bytes().to_vec();
        bytes.push(0xEE); // not a recognized type tag
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.push(b'x');
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.push(0);
        let err = parse_variant_dictionary(&bytes).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    #[test]
    fn variant_dictionary_rejects_truncated_entry() {
        let mut bytes = 0x0100u16.to_le_bytes().to_vec();
        bytes.push(0x04); // UInt32
        bytes.extend_from_slice(&100u32.to_le_bytes()); // claims a huge key length
        let err = parse_variant_dictionary(&bytes).unwrap_err();
        assert!(matches!(err, ImportError::Decode(_)));
    }

    // --- Empty password (keyfile-only databases fail the same way, §8.6) --

    #[test]
    fn empty_password_is_passed_through_and_fails_like_any_wrong_password() {
        let fixture = encode_kdbx4(&EncodeOptions::default());
        let err = decode(&fixture.bytes, &pw(b"")).unwrap_err();
        assert!(matches!(err, ImportError::Adapter(m) if m == "wrong password or corrupt file"));
    }

    // --- Send + Sync of the produced records (sanity) -----------------

    #[test]
    fn decode_result_records_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Vec<PortableRecord>>();
    }
}
