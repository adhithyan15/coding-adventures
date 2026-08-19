//! # coding_adventures_vault_records — VLT02 typed record schemas
//!
//! ## What this crate does
//!
//! VLT01 (the sealed store) takes opaque `Vec<u8>` plaintext and
//! produces an envelope-encrypted record. That is intentional and
//! flexible — but every application built on the vault would
//! otherwise hand-roll the same struct-to-bytes serialisation, with
//! the same bugs. VLT02 is the typed record layer:
//!
//! * Define a `VaultRecord` trait: typed struct ↔ canonical CBOR
//!   bytes (via the sibling `coding_adventures_canonical_cbor`
//!   crate). Encoding is **deterministic** — the same logical record
//!   always produces the same bytes, which is what VLT01's AEAD AAD
//!   binding needs.
//! * Carry a `content_type` string with every encoded record (e.g.
//!   `"vault/login/v1"`, `"vault/note/v1"`, `"vault/card/v1"`). This
//!   lets the decoder dispatch to the right struct, lets unknown
//!   types pass through as opaque bytes (so an old client doesn't
//!   crash when it sees a `"vault/biometric/v1"` produced by a newer
//!   one), and lets schema migration be a codec concern rather than
//!   a storage concern.
//! * Ship a small set of "first-party" record types covering both
//!   reference targets:
//!     - **End-user password manager** (Bitwarden / 1Password class):
//!       `Login`, `SecureNote`, `Card`, `TotpSeed`.
//!     - **Machine-secret store** (HashiCorp Vault class):
//!       `ApiKey`, `DatabaseCredential`.
//! * Keep ordinary diagnostic formatting inert: typed records and
//!   [`AnyRecord`] retain only their type or variant name plus a fixed
//!   `<redacted>` marker, and [`VaultRecordError`] suppresses input values.
//!
//! Apps register custom types as they need.
//!
//! ## Wire format
//!
//! Every encoded record is a CBOR map of exactly two entries:
//!
//! ```text
//!   {
//!     "t" : <text>,    // content_type, e.g. "vault/login/v1"
//!     "d" : <map>,     // payload — schema-specific fields
//!   }
//! ```
//!
//! Why short keys (`"t"` / `"d"`)? Records are small and stored once
//! per user (often millions per organisation), so two-byte CBOR
//! headers per key matter for total disk + network use. Short keys
//! also mean the canonical-CBOR length-first ordering puts `"d"`
//! before `"t"` deterministically (both are length-1 text strings,
//! tied at length, so bytewise lex breaks the tie: `"d" < "t"`).
//!
//! Why `t` *outside* the payload rather than as a CBOR tag? Tags in
//! the canonical-CBOR profile pass through opaquely and are not
//! interpreted by us; using them for content-typing would mix
//! semantics with structure. Top-level fields are clearer.
//!
//! ## Versioning
//!
//! Content types are suffixed `vN` (e.g. `vault/login/v1`). When a
//! schema evolves, the new version gets a fresh tag. Decoders that
//! understand only v1 see v2 records as `Opaque`. Migration helpers
//! (read v1, return a v2 struct) live alongside the v2 type.
//!
//! ## What this crate does *not* do
//!
//! * No encryption — that's VLT01.
//! * No persistence — that's `storage-core`.
//! * No app-specific record schemas (TOTP timestepping, etc.) —
//!   those are interpretation concerns at the layer above.
//! * No schema validation beyond "decoded the right CBOR shape" —
//!   we don't enforce e.g. "URLs must be valid HTTPS." That's a
//!   higher layer's call.
//!
//! ## Example
//!
//! ```ignore
//! use coding_adventures_vault_records::{Login, encode_record, decode_record, AnyRecord};
//!
//! let login = Login {
//!     title: "GitHub".into(),
//!     username: "ada".into(),
//!     password: "p455w0rd".into(),
//!     urls: vec!["https://github.com".into()],
//!     notes: None,
//! };
//! let bytes = encode_record(&login).unwrap();        // canonical CBOR
//! let back  = decode_record(&bytes).unwrap();         // AnyRecord
//! match back {
//!     AnyRecord::Login(l) => assert_eq!(l, login),
//!     _ => unreachable!(),
//! }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

// The panicking `encode` is deliberately not imported here. Every
// encode on a production path in this crate goes through `try_encode`
// so that an oversized or overdeep value is reported rather than
// aborting the process; the tests below import `encode` separately to
// build fixture bytes they already know are in range.
use coding_adventures_canonical_cbor::{
    decode, decode_map_spanned, try_encode, CborError, CborValue,
};
use coding_adventures_zeroize::Zeroize;

// ─────────────────────────────────────────────────────────────────────
// 1. Content-type constants and the `VaultRecord` trait
// ─────────────────────────────────────────────────────────────────────

/// Content type for [`Login`] records.
pub const LOGIN_V1: &str = "vault/login/v1";
/// Content type for [`SecureNote`] records.
pub const SECURE_NOTE_V1: &str = "vault/note/v1";
/// Content type for [`Card`] records.
pub const CARD_V1: &str = "vault/card/v1";
/// Content type for [`TotpSeed`] records.
pub const TOTP_SEED_V1: &str = "vault/totp/v1";
/// Content type for [`ApiKey`] records.
pub const API_KEY_V1: &str = "vault/api-key/v1";
/// Content type for [`DatabaseCredential`] records.
pub const DATABASE_CREDENTIAL_V1: &str = "vault/db-credential/v1";

/// Implemented by every typed record. Defines how the struct maps to
/// the inner CBOR payload (`"d"`).
///
/// Implementors should be careful that `encode_payload(self)` is
/// **deterministic** in the values of `self` — the canonical CBOR
/// encoder takes care of map key ordering, so as long as the
/// `CborValue::Map` you build always carries the same set of keys
/// for a given populated record, encoded bytes will be byte-stable.
pub trait VaultRecord: Sized {
    /// The content-type string this record is tagged with on the
    /// wire, e.g. `"vault/login/v1"`.
    const CONTENT_TYPE: &'static str;

    /// Encode the record's fields into a CBOR value.
    fn encode_payload(&self) -> CborValue;

    /// Reconstruct the record from a decoded CBOR payload.
    /// Returns `Err(VaultRecordError::SchemaMismatch)` when the
    /// payload is missing required fields or has the wrong shape.
    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError>;
}

// ─────────────────────────────────────────────────────────────────────
// 2. Errors
// ─────────────────────────────────────────────────────────────────────

/// Errors returned by [`encode_record`] and [`decode_record`].
///
/// `Display` strings come from this crate's literals — never from
/// the input bytes — to avoid log-injection from malicious payloads.
pub enum VaultRecordError {
    /// Underlying canonical-CBOR codec failed.
    Cbor(CborError),
    /// Top-level structure was not the expected `{"t":…, "d":…}` map.
    NotARecord,
    /// `"t"` was not a text string, or `"d"` was missing.
    BadEnvelope,
    /// `decode_record_as::<T>` was called but the bytes' content
    /// type did not match `T::CONTENT_TYPE`.
    ContentTypeMismatch {
        /// What the caller asked for.
        expected: &'static str,
        /// What the bytes actually said.
        actual: String,
    },
    /// The payload didn't match the schema for the declared content
    /// type — missing required fields, wrong field types, etc.
    SchemaMismatch {
        /// Static description of the violation, e.g. `"Login.username missing"`.
        what: &'static str,
    },
}

impl core::fmt::Debug for VaultRecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variant = match self {
            Self::Cbor(_) => "VaultRecordError::Cbor",
            Self::NotARecord => "VaultRecordError::NotARecord",
            Self::BadEnvelope => "VaultRecordError::BadEnvelope",
            Self::ContentTypeMismatch { .. } => "VaultRecordError::ContentTypeMismatch",
            Self::SchemaMismatch { .. } => "VaultRecordError::SchemaMismatch",
        };
        f.write_str(variant)
    }
}

impl core::fmt::Display for VaultRecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VaultRecordError::Cbor(_) => write!(f, "vault-records: canonical-CBOR codec failed"),
            VaultRecordError::NotARecord => {
                write!(f, "vault-records: top-level item was not a {{t,d}} map")
            }
            VaultRecordError::BadEnvelope => {
                write!(
                    f,
                    "vault-records: envelope is missing or has wrong-typed t/d fields"
                )
            }
            VaultRecordError::ContentTypeMismatch { expected, .. } => {
                // Note: we DO show `expected` (a static literal) but not `actual`,
                // which could in principle contain attacker-controlled bytes.
                // Callers can match on the error variant to inspect `actual`.
                write!(
                    f,
                    "vault-records: content-type mismatch (expected {})",
                    expected
                )
            }
            VaultRecordError::SchemaMismatch { what } => {
                write!(f, "vault-records: schema mismatch — {}", what)
            }
        }
    }
}

impl std::error::Error for VaultRecordError {}

impl From<CborError> for VaultRecordError {
    fn from(e: CborError) -> Self {
        VaultRecordError::Cbor(e)
    }
}

// ─────────────────────────────────────────────────────────────────────
// 2a. Zeroizing CborValue trees
// ─────────────────────────────────────────────────────────────────────
//
// `encode_record` and `decode_record` both build an owned `CborValue`
// tree that, for the six typed record kinds plus opaque pass-through,
// *is* somebody's plaintext: `Login.password`, `Card.number`,
// `TotpSeed.secret`, and friends all pass through a `CborValue::Map`
// on the way to or from the wire. `Login` and its siblings already
// implement `Zeroize` + `Drop` (see section 4 below) so the *typed*
// struct wipes itself. The `CborValue` tree used to build or decode
// it did not: `encode_payload` clones every field into a fresh
// `CborValue::Text`/`CborValue::Bytes`, and `decode_payload` clones
// fields back out of a `CborValue` the codec already fully
// materialised — in both directions, an owned tree of plaintext sits
// in a local variable that dropped through the ordinary, non-wiping
// `Vec`/`String`/`CborValue` destructors.
//
// A round-trip security review once proposed patching this by wiping
// only `try_encode`'s own output buffer on its error path. That fix
// was correctly rejected as incomplete by construction: the same
// plaintext also sits in the *caller's* `CborValue` tree (`envelope`
// in `encode_record`, `payload` in `decode_record`), which the
// encoder's error path cannot reach and which is still fully built
// and dropped unwiped even when the encoder *succeeds*.
//
// The real fix has to live on the caller side, for a structural
// reason: `CborValue` is defined in `canonical-cbor`, which is
// deliberately zero-dependency (see CBR01 and that crate's own
// module doc) so it stays usable from arbitrarily constrained
// contexts, including the C/C++ reference oracles it ships alongside.
// It cannot depend on `coding_adventures_zeroize` to provide
// `impl Zeroize for CborValue` itself, and no third crate can supply
// that impl either — neither the `Zeroize` trait nor the `CborValue`
// type is local to this crate, so Rust's orphan rule forbids
// `impl coding_adventures_zeroize::Zeroize for CborValue` from
// showing up here. `zeroize_cbor_value` and `SecretCborValue` below
// sidestep the rule the ordinary way: the *wrapper type* is local to
// this crate, so its `Drop` impl needs no permission from either the
// trait's or `CborValue`'s home crate.
//
// `zeroize_cbor_value`'s match has **no wildcard arm** on purpose: if
// canonical-cbor ever grows a tenth `CborValue` variant (its own docs
// name floats as future work), this function fails to compile until
// the new arm is added here, rather than silently leaving a class of
// plaintext unwiped. See the `zeroize_cbor_value_wipes_every_variant`
// test below for the same exhaustiveness check applied at the call
// site, and `secret_cbor_value_drop_runs_even_on_panic_unwind` for
// proof `SecretCborValue`'s `Drop` fires on every exit path — the
// gap the rejected error-path-only quick fix left open.

/// Recursively wipe every owned `Text`/`Bytes` buffer reachable from a
/// `CborValue` tree, through any nesting of `Array`, `Map` (both keys
/// and values), and `Tag`.
///
/// `Unsigned`, `Negative`, `Bool`, and `Null` carry no owned heap
/// buffer — a `u64`/`bool` lives inline in the enum, nothing to wipe —
/// so those arms are no-ops kept only so the match stays exhaustive.
fn zeroize_cbor_value(value: &mut CborValue) {
    match value {
        CborValue::Text(s) => s.zeroize(),
        CborValue::Bytes(b) => b.zeroize(),
        CborValue::Array(items) => {
            for item in items.iter_mut() {
                zeroize_cbor_value(item);
            }
        }
        CborValue::Map(entries) => {
            for (k, v) in entries.iter_mut() {
                zeroize_cbor_value(k);
                zeroize_cbor_value(v);
            }
        }
        CborValue::Tag(_, inner) => zeroize_cbor_value(inner),
        CborValue::Unsigned(_) | CborValue::Negative(_) | CborValue::Bool(_) | CborValue::Null => {}
    }
}

/// Owns a `CborValue` tree that may hold plaintext secrets and wipes
/// it, recursively, when the guard drops.
///
/// This is `Zeroizing<CborValue>` in everything but name — `CborValue`
/// cannot itself implement the sibling crate's `Zeroize` trait (see
/// the section comment above), so this crate provides the equivalent
/// guarantee with a local newtype instead. `Deref` gives ordinary
/// read access (`&secret_cbor_value` coerces to `&CborValue`) so
/// callers pass it straight through to `expect_map`, `try_encode`,
/// and every `VaultRecord::decode_payload` without change.
struct SecretCborValue(CborValue);

impl SecretCborValue {
    fn new(value: CborValue) -> Self {
        Self(value)
    }
}

impl core::ops::Deref for SecretCborValue {
    type Target = CborValue;

    fn deref(&self) -> &CborValue {
        &self.0
    }
}

impl Drop for SecretCborValue {
    fn drop(&mut self) {
        #[cfg(test)]
        SECRET_CBOR_VALUE_DROPS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        zeroize_cbor_value(&mut self.0);
    }
}

/// Test-only counter proving `SecretCborValue::drop` actually ran,
/// without reading through a pointer into memory the real `Drop` has
/// already deallocated (which would be undefined behaviour, and this
/// crate is `#![forbid(unsafe_code)]` besides). See
/// `secret_cbor_value_drop_runs_even_on_panic_unwind` below.
#[cfg(test)]
static SECRET_CBOR_VALUE_DROPS: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

// ─────────────────────────────────────────────────────────────────────
// 3. Top-level encode / decode
// ─────────────────────────────────────────────────────────────────────

/// Encode a `VaultRecord` to canonical CBOR bytes with its
/// content-type tag. Output is deterministic.
///
/// # Why this returns a `Result`
///
/// A well-typed Rust struct looks like it should always encode, and
/// yet two ceilings sit on either side of this function and disagree:
///
/// ```text
///   canonical-cbor  MAX_ENCODED_SIZE  =  1 MiB   per encoded value
///   vault-pm        MAX_PLAINTEXT_BYTES = 16 MiB   per sealed object
/// ```
///
/// The application's gate is sixteen times the codec's, and nothing
/// reconciles them because they answer different questions — one
/// bounds an AEAD frame, the other bounds the encoder's own
/// denial-of-service exposure. Between them lies a band of record
/// sizes that are legal to hold and legal to decode but **illegal to
/// encode**.
///
/// That band is reachable, not theoretical. A peer device with a
/// larger framing budget can author a `Login` with a 2 MiB password;
/// it seals, syncs, and decodes here without complaint. The next
/// command that re-serialises it — `item edit`, any of the seven
/// authored conflict merges, `conflict choose`, `history restore`,
/// `export` — arrives back at this function.
///
/// So the encode is reported through an error channel rather than by
/// panicking. Failing closed loses one record; panicking loses the
/// process, and because the record stays in the store the abort
/// repeats on every later command against that vault. This mirrors
/// the rule already applied to [`encode_opaque`] and to
/// [`decode_record`]'s opaque arm.
///
/// No partial output escapes: `try_encode` builds into its own buffer
/// and returns `Err` without bytes, so a refused record can never be
/// mistaken for a truncated-but-whole one.
pub fn encode_record<T: VaultRecord>(rec: &T) -> Result<Vec<u8>, VaultRecordError> {
    // `envelope` holds a full plaintext clone of `rec`'s fields (see the
    // section 2a comment above) for exactly as long as it takes
    // `try_encode` to read it. `SecretCborValue`'s `Drop` wipes that
    // clone whether `try_encode` succeeds or the `?` below returns
    // early on failure — both are "the relevant caller-side scope
    // ends," and both used to leave the clone sitting in freed heap.
    let envelope = SecretCborValue::new(CborValue::Map(vec![
        (CborValue::text("t"), CborValue::text(T::CONTENT_TYPE)),
        (CborValue::text("d"), rec.encode_payload()),
    ]));
    Ok(try_encode(&envelope)?)
}

/// Decode any vault record. Returns an [`AnyRecord`] which
/// pattern-matches on the content type. Unknown types are returned
/// as `AnyRecord::Opaque` so old clients do not crash on records
/// produced by newer ones.
///
/// # Why a schema mismatch on a *known* content type does not fail
///
/// A payload tagged with a first-party content type (e.g.
/// `vault/login/v1`) but missing a required field, or carrying a
/// field of the wrong shape, used to make this function return
/// `Err(SchemaMismatch)`. That looks like the correct behaviour in
/// isolation — the bytes genuinely are not a `Login` — but one layer
/// up it repeats the exact defect this function's opaque arm was
/// fixed for (see the comment below): `decode_record` runs during
/// vault *open*, and the caller (`decode_live` in the application
/// layer) mapped every decode error, `SchemaMismatch` included, to
/// `IntegrityFailure`. One peer authoring a `Login` with a missing
/// `password` field denied the whole vault, not just that one item —
/// the identical blast radius as the oversized-opaque bug, reached
/// through a different door.
///
/// Unlike the oversized-opaque case, there is no "return the original
/// bytes as the same type" escape hatch here: the payload cannot be
/// materialised as a `Login` at all, by definition. What *is* available
/// is the same technique the opaque arm already uses — the payload's
/// own already-validated CBOR bytes, sliced rather than re-encoded —
/// just attached to a new [`AnyRecord::Quarantined`] variant instead of
/// a typed struct. The record is still identifiable (its declared
/// content type is retained) and still round-trips losslessly (its
/// bytes are retained verbatim); it just cannot be interpreted as the
/// schema it claims. That is enough for the vault to open, for the item
/// to be listed, and for it to be deleted — the same floor `Opaque`
/// already stands on.
///
/// Only [`VaultRecordError::SchemaMismatch`] is treated this way. Any
/// other error from a typed decoder still fails `decode_record` outright,
/// because `VaultRecord::decode_payload` is documented to return
/// `SchemaMismatch` and nothing else — this is a defensive branch, not a
/// currently-reachable one, kept so a future decoder that starts
/// returning some other error is not silently swallowed into quarantine.
pub fn decode_record(bytes: &[u8]) -> Result<AnyRecord, VaultRecordError> {
    let (content_type, payload, payload_span) = split_envelope(bytes)?;
    // Decode a known first-party type, quarantining a schema mismatch
    // instead of propagating it. See the function doc comment above.
    macro_rules! typed_or_quarantine {
        ($ctor:expr) => {
            match $ctor(&payload) {
                Ok(record) => record,
                Err(VaultRecordError::SchemaMismatch { what }) => {
                    return Ok(AnyRecord::Quarantined {
                        content_type,
                        payload_bytes: bytes[payload_span].to_vec(),
                        reason: what,
                    })
                }
                Err(other) => return Err(other),
            }
        };
    }
    Ok(match content_type.as_str() {
        LOGIN_V1 => AnyRecord::Login(typed_or_quarantine!(Login::decode_payload)),
        SECURE_NOTE_V1 => AnyRecord::SecureNote(typed_or_quarantine!(SecureNote::decode_payload)),
        CARD_V1 => AnyRecord::Card(typed_or_quarantine!(Card::decode_payload)),
        TOTP_SEED_V1 => AnyRecord::TotpSeed(typed_or_quarantine!(TotpSeed::decode_payload)),
        API_KEY_V1 => AnyRecord::ApiKey(typed_or_quarantine!(ApiKey::decode_payload)),
        DATABASE_CREDENTIAL_V1 => {
            AnyRecord::DatabaseCredential(typed_or_quarantine!(DatabaseCredential::decode_payload))
        }
        // Unknown / app-specific / future-version: hand back the
        // payload's own bytes and return as opaque.
        //
        // # Why this slices instead of re-encoding
        //
        // The obvious spelling is `try_encode(&payload)`, and this
        // function used to be written that way. It was the most severe
        // defect in the codec, because it could refuse a record the
        // decoder had just accepted:
        //
        // ```text
        //   canonical-cbor  MAX_ENCODED_SIZE    =  1 MiB   (encoder only)
        //   vault-pm        MAX_PLAINTEXT_BYTES = 16 MiB   (what we hold)
        // ```
        //
        // `decode` has no matching input-length bound, so an opaque
        // payload anywhere between those two numbers reads back fine and
        // then fails to re-encode. Every such record is undecodable
        // rather than merely unwritable — and this decode runs during
        // vault *open*, before any session exists, so a single synced
        // peer record denied the whole vault permanently, with no
        // session to delete it from and no export to escape through.
        //
        // Nothing needed re-encoding in the first place. The payload
        // arrived through the strict canonical decoder, which enforces
        // every rule the encoder applies, so its bytes already are the
        // one legal spelling of that value — exactly what `try_encode`
        // would have returned, byte for byte. Taking the sub-slice keeps
        // that guarantee (`encode_opaque` still round trips to the same
        // wire form) and drops the only failure mode, since slicing a
        // range the parser itself measured cannot fail on any input that
        // decoded at all.
        //
        // The record is still too large for this product to *write*.
        // That refusal stays where it belongs, in `encode_opaque` and
        // `encode_record`, where it costs one record rather than the
        // vault.
        _ => AnyRecord::Opaque {
            content_type,
            payload_bytes: bytes[payload_span].to_vec(),
        },
    })
}

/// Decode bytes as a specific known record type. Returns
/// [`VaultRecordError::ContentTypeMismatch`] if the content type
/// doesn't match `T::CONTENT_TYPE`.
pub fn decode_record_as<T: VaultRecord>(bytes: &[u8]) -> Result<T, VaultRecordError> {
    let (content_type, payload, _) = split_envelope(bytes)?;
    if content_type != T::CONTENT_TYPE {
        return Err(VaultRecordError::ContentTypeMismatch {
            expected: T::CONTENT_TYPE,
            actual: content_type,
        });
    }
    T::decode_payload(&payload)
}

/// Helper: peel off the `{t, d}` envelope.
///
/// Returns the content type, the decoded payload, and the byte range the
/// payload's encoding occupied inside `bytes`. Both public decoders go
/// through here, so there is exactly one answer to "which bytes are a
/// record" and the typed and opaque paths cannot drift into accepting
/// different inputs.
fn split_envelope(
    bytes: &[u8],
) -> Result<(String, SecretCborValue, core::ops::Range<usize>), VaultRecordError> {
    // Anything that is not a two-entry map is not a record envelope. A
    // shape mismatch is `Ok(None)` from the codec and a violation of the
    // canonical profile is `Err`, and the two stay distinct here.
    let Some(entries) = decode_map_spanned(bytes)? else {
        return Err(VaultRecordError::NotARecord);
    };
    if entries.len() != 2 {
        return Err(VaultRecordError::NotARecord);
    }
    let mut t: Option<String> = None;
    // Wrapped the instant the plaintext value under `"d"` is captured,
    // not after `split_envelope` returns successfully: `d` can also be
    // dropped un-returned, from inside this loop, if the envelope's
    // *other* entry turns out to be malformed (`BadEnvelope` below).
    // `Option<SecretCborValue>`'s own drop glue wipes it on that path
    // too, which a wipe placed only at the bottom of this function,
    // after the final `match`, would have missed.
    let mut d: Option<(SecretCborValue, core::ops::Range<usize>)> = None;
    for mut entry in entries {
        match entry.key {
            CborValue::Text(s) if s == "t" => match entry.value {
                CborValue::Text(s) => t = Some(s),
                mut other => {
                    // Malformed `"t"`: whatever this decoded to is
                    // still somebody's decrypted plaintext, just filed
                    // under the wrong key. Wipe it before the shape
                    // mismatch is reported.
                    zeroize_cbor_value(&mut other);
                    return Err(VaultRecordError::BadEnvelope);
                }
            },
            CborValue::Text(s) if s == "d" => {
                d = Some((SecretCborValue::new(entry.value), entry.value_span))
            }
            _ => {
                // Unrecognised key. The value under it is still
                // decrypted plaintext (this envelope's own siblings
                // decoded through the same call), so it gets the same
                // wipe as every other rejected shape here — including
                // when this is the *second* of the two entries and `d`
                // above is already `Some`, in which case `d`'s own
                // wrap-on-capture (see above) covers it when this
                // function returns.
                zeroize_cbor_value(&mut entry.value);
                return Err(VaultRecordError::BadEnvelope);
            }
        }
    }
    match (t, d) {
        (Some(t), Some((d, span))) => Ok((t, d, span)),
        _ => Err(VaultRecordError::BadEnvelope),
    }
}

/// One of the known record types, an opaque pass-through for content
/// types this crate doesn't recognise, or a quarantined record whose
/// declared type it recognises but whose payload doesn't parse as
/// that type's schema.
#[derive(Clone, PartialEq, Eq)]
pub enum AnyRecord {
    /// `vault/login/v1`
    Login(Login),
    /// `vault/note/v1`
    SecureNote(SecureNote),
    /// `vault/card/v1`
    Card(Card),
    /// `vault/totp/v1`
    TotpSeed(TotpSeed),
    /// `vault/api-key/v1`
    ApiKey(ApiKey),
    /// `vault/db-credential/v1`
    DatabaseCredential(DatabaseCredential),
    /// Any other content type — the bytes here are the canonical-
    /// CBOR re-encoding of the inner payload (so a roundtrip through
    /// `decode_record` then re-emitting via `encode_opaque` is
    /// byte-stable).
    Opaque {
        /// The content_type string from the wire.
        content_type: String,
        /// The canonical-CBOR-encoded payload bytes.
        payload_bytes: Vec<u8>,
    },
    /// A first-party content type (`vault/login/v1` and friends) whose
    /// payload does not decode as that type's schema — a required field
    /// missing, or present with the wrong shape.
    ///
    /// This differs from `Opaque` in *why* the type could not be
    /// materialised: `Opaque` means "this crate doesn't recognise the
    /// content type at all," which is an ordinary forward-compatibility
    /// case (an older client seeing a newer peer's record). `Quarantined`
    /// means "this crate recognises the content type and the payload is
    /// still wrong," which only happens if a peer authored a malformed
    /// record — by bug or by malice. The two get the same downstream
    /// treatment (visible in `item list`, redacted in `item show`,
    /// deletable, inert in search) because a caller's remedy is the same
    /// either way — it cannot repair the record, only remove it — but
    /// they are kept as distinct variants because *why* a record is
    /// unreadable is worth preserving for diagnostics, and conflating
    /// the two would make "how many records has this vault received
    /// that this client cannot even parse against its own claimed type"
    /// unanswerable.
    Quarantined {
        /// The content_type string from the wire (one of this crate's
        /// own `*_V1` constants — otherwise decoding would have taken
        /// the `Opaque` arm instead).
        content_type: String,
        /// The payload's own canonical-CBOR bytes, sliced rather than
        /// re-encoded for the same reason `Opaque`'s are: the bytes came
        /// from the strict canonical decoder, so they already are the
        /// one legal spelling of that value, and slicing a
        /// parser-measured range cannot fail on input that decoded.
        payload_bytes: Vec<u8>,
        /// Static description of what went wrong, e.g.
        /// `"Login.password missing"`. Never attacker-controlled text —
        /// it is always one of a fixed set of literals defined by this
        /// crate's own typed decoders.
        reason: &'static str,
    },
}

impl core::fmt::Debug for AnyRecord {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variant = match self {
            Self::Login(_) => "AnyRecord::Login(<redacted>)",
            Self::SecureNote(_) => "AnyRecord::SecureNote(<redacted>)",
            Self::Card(_) => "AnyRecord::Card(<redacted>)",
            Self::TotpSeed(_) => "AnyRecord::TotpSeed(<redacted>)",
            Self::ApiKey(_) => "AnyRecord::ApiKey(<redacted>)",
            Self::DatabaseCredential(_) => "AnyRecord::DatabaseCredential(<redacted>)",
            Self::Opaque { .. } => "AnyRecord::Opaque(<redacted>)",
            Self::Quarantined { .. } => "AnyRecord::Quarantined(<redacted>)",
        };
        f.write_str(variant)
    }
}

/// Known high-level record kind without carrying record values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultRecordKind {
    /// `vault/login/v1`
    Login,
    /// `vault/note/v1`
    SecureNote,
    /// `vault/card/v1`
    Card,
    /// `vault/totp/v1`
    TotpSeed,
    /// `vault/api-key/v1`
    ApiKey,
    /// `vault/db-credential/v1`
    DatabaseCredential,
    /// Unknown, app-specific, or future-version content type.
    Opaque,
    /// A first-party content type whose payload failed schema decode.
    Quarantined,
}

impl VaultRecordKind {
    /// Static content type for first-party records.
    ///
    /// Returns `None` for `Quarantined` even though the record's *wire*
    /// content type is one of this crate's own `*_V1` constants — this
    /// method returns a `&'static str` and a quarantined record's content
    /// type is only known at runtime (it is read straight off `AnyRecord`,
    /// same as `Opaque`'s). Use `AnyRecord`'s own content-type accessor
    /// (in the application layer) when the dynamic string is needed.
    pub fn content_type(self) -> Option<&'static str> {
        match self {
            Self::Login => Some(LOGIN_V1),
            Self::SecureNote => Some(SECURE_NOTE_V1),
            Self::Card => Some(CARD_V1),
            Self::TotpSeed => Some(TOTP_SEED_V1),
            Self::ApiKey => Some(API_KEY_V1),
            Self::DatabaseCredential => Some(DATABASE_CREDENTIAL_V1),
            Self::Opaque | Self::Quarantined => None,
        }
    }

    /// True for first-party records understood by this crate.
    ///
    /// `Quarantined` is `false` here even though its wire content type
    /// names a first-party schema: this predicate means "successfully
    /// decoded as one of the six typed schemas," and a quarantined
    /// record by definition did not.
    pub fn is_first_party(self) -> bool {
        self.content_type().is_some()
    }

    /// True for password-manager-shaped first-party records.
    pub fn is_password_manager_record(self) -> bool {
        matches!(
            self,
            Self::Login | Self::SecureNote | Self::Card | Self::TotpSeed
        )
    }

    /// True for machine-secret-store first-party records.
    pub fn is_machine_secret_record(self) -> bool {
        matches!(self, Self::ApiKey | Self::DatabaseCredential)
    }
}

/// Value-redacted record inventory data for host/store planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultRecordSummary {
    /// Record kind, or [`VaultRecordKind::Opaque`] for unknown content types.
    pub kind: VaultRecordKind,
    /// Number of fields that this crate treats as secret-bearing.
    pub secret_field_count: usize,
    /// Number of present optional fields.
    pub optional_field_count: usize,
    /// Number of repeated string items, such as URLs or scopes.
    pub list_item_count: usize,
    /// Whether the record carries an expiry timestamp.
    pub has_expiry: bool,
    /// Whether the record carries a lease reference.
    pub has_lease: bool,
    /// Length of an opaque or quarantined record's content type. Zero for
    /// records decoded as one of the six typed schemas.
    pub opaque_content_type_len: usize,
    /// Canonical-CBOR payload byte length for opaque or quarantined
    /// records. Zero for records decoded as one of the six typed schemas.
    pub opaque_payload_bytes: usize,
}

impl VaultRecordSummary {
    /// True when the record is decoded as a first-party schema.
    pub fn is_first_party(&self) -> bool {
        self.kind.is_first_party()
    }

    /// True when the record belongs to the password-manager schema family.
    pub fn is_password_manager_record(&self) -> bool {
        self.kind.is_password_manager_record()
    }

    /// True when the record belongs to the machine-secret schema family.
    pub fn is_machine_secret_record(&self) -> bool {
        self.kind.is_machine_secret_record()
    }

    /// True when at least one secret-bearing field is present.
    pub fn carries_sensitive_fields(&self) -> bool {
        self.secret_field_count > 0
    }

    /// True when the record is opaque to this crate.
    pub fn is_opaque(&self) -> bool {
        self.kind == VaultRecordKind::Opaque
    }

    /// True when the record's declared type is known but its payload
    /// failed schema decode.
    pub fn is_quarantined(&self) -> bool {
        self.kind == VaultRecordKind::Quarantined
    }
}

impl AnyRecord {
    /// Return the value-redacted kind for this record.
    pub fn kind(&self) -> VaultRecordKind {
        match self {
            Self::Login(_) => VaultRecordKind::Login,
            Self::SecureNote(_) => VaultRecordKind::SecureNote,
            Self::Card(_) => VaultRecordKind::Card,
            Self::TotpSeed(_) => VaultRecordKind::TotpSeed,
            Self::ApiKey(_) => VaultRecordKind::ApiKey,
            Self::DatabaseCredential(_) => VaultRecordKind::DatabaseCredential,
            Self::Opaque { .. } => VaultRecordKind::Opaque,
            Self::Quarantined { .. } => VaultRecordKind::Quarantined,
        }
    }

    /// Return value-redacted inventory data without cloning secret values.
    pub fn summary(&self) -> VaultRecordSummary {
        match self {
            Self::Login(record) => VaultRecordSummary {
                kind: VaultRecordKind::Login,
                secret_field_count: 1,
                optional_field_count: usize::from(record.notes.is_some()),
                list_item_count: record.urls.len(),
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            },
            Self::SecureNote(_) => VaultRecordSummary {
                kind: VaultRecordKind::SecureNote,
                secret_field_count: 1,
                optional_field_count: 0,
                list_item_count: 0,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            },
            Self::Card(record) => VaultRecordSummary {
                kind: VaultRecordKind::Card,
                secret_field_count: 2,
                optional_field_count: usize::from(record.billing_zip.is_some()),
                list_item_count: 0,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            },
            Self::TotpSeed(record) => VaultRecordSummary {
                kind: VaultRecordKind::TotpSeed,
                secret_field_count: 1,
                optional_field_count: usize::from(record.issuer.is_some()),
                list_item_count: 0,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            },
            Self::ApiKey(record) => VaultRecordSummary {
                kind: VaultRecordKind::ApiKey,
                secret_field_count: 1,
                optional_field_count: usize::from(record.expires_at.is_some()),
                list_item_count: record.scopes.len(),
                has_expiry: record.expires_at.is_some(),
                has_lease: false,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            },
            Self::DatabaseCredential(record) => VaultRecordSummary {
                kind: VaultRecordKind::DatabaseCredential,
                secret_field_count: 1,
                optional_field_count: usize::from(record.database.is_some())
                    + usize::from(record.lease_id.is_some())
                    + usize::from(record.expires_at.is_some()),
                list_item_count: 0,
                has_expiry: record.expires_at.is_some(),
                has_lease: record.lease_id.is_some(),
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            },
            Self::Opaque {
                content_type,
                payload_bytes,
            } => VaultRecordSummary {
                kind: VaultRecordKind::Opaque,
                secret_field_count: 0,
                optional_field_count: 0,
                list_item_count: 0,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: content_type.len(),
                opaque_payload_bytes: payload_bytes.len(),
            },
            Self::Quarantined {
                content_type,
                payload_bytes,
                reason: _,
            } => VaultRecordSummary {
                kind: VaultRecordKind::Quarantined,
                secret_field_count: 0,
                optional_field_count: 0,
                list_item_count: 0,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: content_type.len(),
                opaque_payload_bytes: payload_bytes.len(),
            },
        }
    }
}

impl Zeroize for AnyRecord {
    fn zeroize(&mut self) {
        match self {
            AnyRecord::Login(r) => r.zeroize(),
            AnyRecord::SecureNote(r) => r.zeroize(),
            AnyRecord::Card(r) => r.zeroize(),
            AnyRecord::TotpSeed(r) => r.zeroize(),
            AnyRecord::ApiKey(r) => r.zeroize(),
            AnyRecord::DatabaseCredential(r) => r.zeroize(),
            AnyRecord::Opaque {
                content_type,
                payload_bytes,
            } => {
                content_type.zeroize();
                payload_bytes.zeroize();
            }
            AnyRecord::Quarantined {
                content_type,
                payload_bytes,
                reason: _,
            } => {
                // `reason` is a `&'static str` literal owned by this
                // crate's own source, never attacker-controlled, so
                // there is nothing to wipe there. `content_type` and
                // `payload_bytes` came off the wire and get the same
                // treatment as `Opaque`'s.
                content_type.zeroize();
                payload_bytes.zeroize();
            }
        }
    }
}

// NOTE: `AnyRecord` does NOT implement `Drop`. Adding Drop to the
// enum would prevent callers from move-destructuring its variants
// (`match any { AnyRecord::Login(l) => l }` — which moves the inner
// Login out — would fail to compile). Instead, each typed variant
// (`Login`, `Card`, etc.) implements `Drop` *itself*, so when an
// `AnyRecord` is dropped, the typed inner record drops in the
// normal enum-drop order and its Drop wipes. The one exception is
// `AnyRecord::Opaque { content_type, payload_bytes }` — by
// definition we don't know what type those bytes encode, so this
// crate does not assume they are sensitive. If a caller decides
// they ARE sensitive (because they came from a known but
// future-version content type), the caller should call
// `.zeroize()` explicitly via the `Zeroize` trait impl above
// before letting the value drop.
//
// `AnyRecord::Quarantined { content_type, payload_bytes, reason }`
// gets the same non-`Drop` treatment for the same structural reason
// (no typed struct to hold a per-type `Drop`), but unlike `Opaque` its
// bytes usually ARE known to be sensitive: the declared content type
// names one of this crate's own secret-bearing schemas (a `Login`
// missing its `password` field, say, still has a *username* sitting in
// `payload_bytes`). Callers that materialise a `Quarantined` record —
// today, only `decode_record`'s typed-dispatch arms — should treat its
// `payload_bytes` as sensitive by default and hold it the same way they
// would hold the typed record it failed to become (e.g. wrapped in
// `Zeroizing<_>`), rather than assuming safety the way `Opaque`'s
// genuinely-unknown-type bytes get to.

/// Re-encode an [`AnyRecord::Opaque`] back to its full
/// envelope-wrapped canonical CBOR bytes. Useful for forwarding a
/// record of unknown type without losing it.
///
/// Wrapping costs one level of nesting, so a payload that decodes at
/// exactly the decoder's depth limit is one level too deep to encode
/// inside the envelope. That is reported through this function's
/// existing error channel rather than by panicking: `payload_bytes`
/// can come from a caller that authored it, not only from the wire,
/// and no input should be able to abort the process.
pub fn encode_opaque(
    content_type: &str,
    payload_bytes: &[u8],
) -> Result<Vec<u8>, VaultRecordError> {
    // `payload_bytes` here is frequently a `Quarantined`/`Opaque`
    // record's own plaintext (see that variant's doc comment on
    // `AnyRecord`), so `payload` and the `envelope` it moves into get
    // the same `SecretCborValue` wipe-on-drop treatment as
    // `encode_record`'s tree.
    let payload = decode(payload_bytes)?;
    let envelope = SecretCborValue::new(CborValue::Map(vec![
        (
            CborValue::text("t"),
            CborValue::text(content_type.to_string()),
        ),
        (CborValue::text("d"), payload),
    ]));
    Ok(try_encode(&envelope)?)
}

// ─────────────────────────────────────────────────────────────────────
// 4. Concrete record types
// ─────────────────────────────────────────────────────────────────────
//
// Pattern: each struct holds plain Rust types. `encode_payload`
// builds a canonical CborValue::Map with explicit keys; the encoder
// will sort them. `decode_payload` walks the entries by key name and
// materialises the struct. Unknown extra keys are tolerated (forward-
// compat: a v1.1 might add fields). Required-but-missing keys raise
// SchemaMismatch with a static `what`.
//
// Sensitive fields (passwords, secrets, seeds) implement Zeroize via
// the sibling crate. Drop is triggered by the typical Vec/String
// drop chain; the higher Vault layer wraps records in Zeroizing<…>
// when it holds them in memory.

/// A login (username + password + URLs) in the password-manager use
/// case. Reference shape: Bitwarden's `Login` / 1Password's
/// `LoginItem`.
#[derive(Clone, PartialEq, Eq)]
pub struct Login {
    /// Display title for the entry.
    pub title: String,
    /// Username / email / handle that goes in the username field.
    pub username: String,
    /// The password. Sensitive; zeroized on drop via the wrapping
    /// `Zeroize` impl.
    pub password: String,
    /// One or more URLs the credential is for. Not validated;
    /// scheme matching is the application's job.
    pub urls: Vec<String>,
    /// Free-form notes. Optional.
    pub notes: Option<String>,
}

impl Zeroize for Login {
    fn zeroize(&mut self) {
        self.title.zeroize();
        self.username.zeroize();
        self.password.zeroize();
        for u in self.urls.iter_mut() {
            u.zeroize();
        }
        // After wiping each URL string's heap buffer, replace the
        // Vec itself with a fresh empty one so the Vec's *own*
        // backing allocation (an array of String header triples) is
        // freed, not just element-cleared.
        self.urls = Vec::new();
        if let Some(n) = self.notes.as_mut() {
            n.zeroize();
        }
        self.notes = None;
    }
}

impl Drop for Login {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl VaultRecord for Login {
    const CONTENT_TYPE: &'static str = LOGIN_V1;

    fn encode_payload(&self) -> CborValue {
        let mut entries = vec![
            (
                CborValue::text("title"),
                CborValue::text(self.title.clone()),
            ),
            (
                CborValue::text("username"),
                CborValue::text(self.username.clone()),
            ),
            (
                CborValue::text("password"),
                CborValue::text(self.password.clone()),
            ),
            (
                CborValue::text("urls"),
                CborValue::Array(
                    self.urls
                        .iter()
                        .map(|u| CborValue::text(u.clone()))
                        .collect(),
                ),
            ),
        ];
        if let Some(n) = &self.notes {
            entries.push((CborValue::text("notes"), CborValue::text(n.clone())));
        }
        CborValue::Map(entries)
    }

    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError> {
        let entries = expect_map(payload)?;
        Ok(Login {
            title: get_text(entries, "title")?,
            username: get_text(entries, "username")?,
            password: get_text(entries, "password")?,
            urls: {
                let arr = get_array(entries, "urls")?;
                let mut out = Vec::with_capacity(arr.len());
                for item in arr {
                    out.push(text_or_err(item, "Login.urls[*]")?);
                }
                out
            },
            notes: get_optional_text(entries, "notes")?,
        })
    }
}

/// A free-form encrypted note.
#[derive(Clone, PartialEq, Eq)]
pub struct SecureNote {
    /// Display title.
    pub title: String,
    /// Body text.
    pub body: String,
}

impl Zeroize for SecureNote {
    fn zeroize(&mut self) {
        self.title.zeroize();
        self.body.zeroize();
    }
}

impl Drop for SecureNote {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl VaultRecord for SecureNote {
    const CONTENT_TYPE: &'static str = SECURE_NOTE_V1;

    fn encode_payload(&self) -> CborValue {
        CborValue::Map(vec![
            (
                CborValue::text("title"),
                CborValue::text(self.title.clone()),
            ),
            (CborValue::text("body"), CborValue::text(self.body.clone())),
        ])
    }

    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError> {
        let entries = expect_map(payload)?;
        Ok(SecureNote {
            title: get_text(entries, "title")?,
            body: get_text(entries, "body")?,
        })
    }
}

/// A credit-card / payment-method record.
#[derive(Clone, PartialEq, Eq)]
pub struct Card {
    /// Display title (e.g. "Personal Visa").
    pub title: String,
    /// Cardholder name as it appears on the card.
    pub holder: String,
    /// PAN — primary account number. Sensitive.
    pub number: String,
    /// MM / YY expiration month, 1..=12.
    pub expiry_month: u8,
    /// YYYY expiration year (e.g. 2030).
    pub expiry_year: u16,
    /// CVV / CSC. Sensitive.
    pub cvv: String,
    /// Optional billing ZIP / postcode.
    pub billing_zip: Option<String>,
}

impl Zeroize for Card {
    fn zeroize(&mut self) {
        self.title.zeroize();
        self.holder.zeroize();
        self.number.zeroize();
        self.expiry_month = 0;
        self.expiry_year = 0;
        self.cvv.zeroize();
        if let Some(z) = self.billing_zip.as_mut() {
            z.zeroize();
        }
        self.billing_zip = None;
    }
}

impl Drop for Card {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl VaultRecord for Card {
    const CONTENT_TYPE: &'static str = CARD_V1;

    fn encode_payload(&self) -> CborValue {
        let mut entries = vec![
            (
                CborValue::text("title"),
                CborValue::text(self.title.clone()),
            ),
            (
                CborValue::text("holder"),
                CborValue::text(self.holder.clone()),
            ),
            (
                CborValue::text("number"),
                CborValue::text(self.number.clone()),
            ),
            (
                CborValue::text("month"),
                CborValue::Unsigned(self.expiry_month as u64),
            ),
            (
                CborValue::text("year"),
                CborValue::Unsigned(self.expiry_year as u64),
            ),
            (CborValue::text("cvv"), CborValue::text(self.cvv.clone())),
        ];
        if let Some(z) = &self.billing_zip {
            entries.push((CborValue::text("zip"), CborValue::text(z.clone())));
        }
        CborValue::Map(entries)
    }

    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError> {
        let entries = expect_map(payload)?;
        let month = get_unsigned(entries, "month")?;
        let year = get_unsigned(entries, "year")?;
        if !(1..=12).contains(&month) {
            return Err(VaultRecordError::SchemaMismatch {
                what: "Card.month not in 1..=12",
            });
        }
        Ok(Card {
            title: get_text(entries, "title")?,
            holder: get_text(entries, "holder")?,
            number: get_text(entries, "number")?,
            expiry_month: month as u8,
            expiry_year: u16::try_from(year).map_err(|_| VaultRecordError::SchemaMismatch {
                what: "Card.year out of u16",
            })?,
            cvv: get_text(entries, "cvv")?,
            billing_zip: get_optional_text(entries, "zip")?,
        })
    }
}

/// A TOTP / HOTP seed (the shared secret an authenticator app stores
/// for one account). Useful when the vault is also acting as the
/// user's authenticator.
#[derive(Clone, PartialEq, Eq)]
pub struct TotpSeed {
    /// Display label (e.g. "GitHub : ada@example.com").
    pub label: String,
    /// Issuer (e.g. "GitHub"). Optional.
    pub issuer: Option<String>,
    /// Shared-secret bytes. Sensitive.
    pub secret: Vec<u8>,
    /// HMAC algorithm — `"SHA1"`, `"SHA256"`, `"SHA512"`.
    pub algorithm: String,
    /// Number of digits in the generated code (typically 6 or 8).
    pub digits: u8,
    /// Time-step in seconds (typically 30).
    pub period: u32,
}

impl Zeroize for TotpSeed {
    fn zeroize(&mut self) {
        self.label.zeroize();
        if let Some(i) = self.issuer.as_mut() {
            i.zeroize();
        }
        self.issuer = None;
        self.secret.zeroize();
        self.algorithm.zeroize();
        self.digits = 0;
        self.period = 0;
    }
}

impl Drop for TotpSeed {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl VaultRecord for TotpSeed {
    const CONTENT_TYPE: &'static str = TOTP_SEED_V1;

    fn encode_payload(&self) -> CborValue {
        let mut entries = vec![
            (
                CborValue::text("label"),
                CborValue::text(self.label.clone()),
            ),
            (
                CborValue::text("secret"),
                CborValue::Bytes(self.secret.clone()),
            ),
            (
                CborValue::text("alg"),
                CborValue::text(self.algorithm.clone()),
            ),
            (
                CborValue::text("digits"),
                CborValue::Unsigned(self.digits as u64),
            ),
            (
                CborValue::text("period"),
                CborValue::Unsigned(self.period as u64),
            ),
        ];
        if let Some(i) = &self.issuer {
            entries.push((CborValue::text("issuer"), CborValue::text(i.clone())));
        }
        CborValue::Map(entries)
    }

    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError> {
        let entries = expect_map(payload)?;
        let digits = get_unsigned(entries, "digits")?;
        let period = get_unsigned(entries, "period")?;
        if !(4..=10).contains(&digits) {
            return Err(VaultRecordError::SchemaMismatch {
                what: "TotpSeed.digits not in 4..=10",
            });
        }
        Ok(TotpSeed {
            label: get_text(entries, "label")?,
            issuer: get_optional_text(entries, "issuer")?,
            secret: get_bytes(entries, "secret")?,
            algorithm: get_text(entries, "alg")?,
            digits: digits as u8,
            period: u32::try_from(period).map_err(|_| VaultRecordError::SchemaMismatch {
                what: "TotpSeed.period out of u32",
            })?,
        })
    }
}

/// An API key — the machine-secret-store equivalent of `Login`.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey {
    /// Display label.
    pub label: String,
    /// Service the key is for (e.g. `"github.com"`).
    pub service: String,
    /// The token / API key. Sensitive.
    pub token: String,
    /// Scopes / permissions assigned to this key. Free-form strings.
    pub scopes: Vec<String>,
    /// Expiry as UNIX seconds. None = no expiry.
    pub expires_at: Option<u64>,
}

impl Zeroize for ApiKey {
    fn zeroize(&mut self) {
        self.label.zeroize();
        self.service.zeroize();
        self.token.zeroize();
        for s in self.scopes.iter_mut() {
            s.zeroize();
        }
        // Drop the Vec's own backing allocation (not just clear length).
        self.scopes = Vec::new();
        self.expires_at = None;
    }
}

impl Drop for ApiKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl VaultRecord for ApiKey {
    const CONTENT_TYPE: &'static str = API_KEY_V1;

    fn encode_payload(&self) -> CborValue {
        let mut entries = vec![
            (
                CborValue::text("label"),
                CborValue::text(self.label.clone()),
            ),
            (
                CborValue::text("service"),
                CborValue::text(self.service.clone()),
            ),
            (
                CborValue::text("token"),
                CborValue::text(self.token.clone()),
            ),
            (
                CborValue::text("scopes"),
                CborValue::Array(
                    self.scopes
                        .iter()
                        .map(|s| CborValue::text(s.clone()))
                        .collect(),
                ),
            ),
        ];
        if let Some(e) = self.expires_at {
            entries.push((CborValue::text("exp"), CborValue::Unsigned(e)));
        }
        CborValue::Map(entries)
    }

    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError> {
        let entries = expect_map(payload)?;
        Ok(ApiKey {
            label: get_text(entries, "label")?,
            service: get_text(entries, "service")?,
            token: get_text(entries, "token")?,
            scopes: {
                let arr = get_array(entries, "scopes")?;
                let mut out = Vec::with_capacity(arr.len());
                for item in arr {
                    out.push(text_or_err(item, "ApiKey.scopes[*]")?);
                }
                out
            },
            expires_at: match find_entry(entries, "exp") {
                None => None,
                Some(CborValue::Unsigned(n)) => Some(*n),
                Some(_) => {
                    return Err(VaultRecordError::SchemaMismatch {
                        what: "ApiKey.exp not unsigned",
                    });
                }
            },
        })
    }
}

/// A database credential — username/password plus connection
/// metadata. Often a *dynamic* credential issued by VLT08; the
/// record schema is identical whether the credential is static or
/// dynamic.
#[derive(Clone, PartialEq, Eq)]
pub struct DatabaseCredential {
    /// Display label.
    pub label: String,
    /// e.g. `"postgres"`, `"mysql"`, `"mongodb"`.
    pub engine: String,
    /// Hostname or IP.
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Database / catalog name. Optional.
    pub database: Option<String>,
    /// Username for the credential.
    pub username: String,
    /// Password. Sensitive.
    pub password: String,
    /// Lease ID (if VLT08-issued); for static creds, None.
    pub lease_id: Option<String>,
    /// Lease expiry as UNIX seconds. None = no expiry.
    pub expires_at: Option<u64>,
}

impl Zeroize for DatabaseCredential {
    fn zeroize(&mut self) {
        self.label.zeroize();
        self.engine.zeroize();
        self.host.zeroize();
        self.port = 0;
        if let Some(d) = self.database.as_mut() {
            d.zeroize();
        }
        self.database = None;
        self.username.zeroize();
        self.password.zeroize();
        if let Some(l) = self.lease_id.as_mut() {
            l.zeroize();
        }
        self.lease_id = None;
        self.expires_at = None;
    }
}

impl Drop for DatabaseCredential {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl VaultRecord for DatabaseCredential {
    const CONTENT_TYPE: &'static str = DATABASE_CREDENTIAL_V1;

    fn encode_payload(&self) -> CborValue {
        let mut entries = vec![
            (
                CborValue::text("label"),
                CborValue::text(self.label.clone()),
            ),
            (
                CborValue::text("engine"),
                CborValue::text(self.engine.clone()),
            ),
            (CborValue::text("host"), CborValue::text(self.host.clone())),
            (
                CborValue::text("port"),
                CborValue::Unsigned(self.port as u64),
            ),
            (
                CborValue::text("username"),
                CborValue::text(self.username.clone()),
            ),
            (
                CborValue::text("password"),
                CborValue::text(self.password.clone()),
            ),
        ];
        if let Some(d) = &self.database {
            entries.push((CborValue::text("db"), CborValue::text(d.clone())));
        }
        if let Some(l) = &self.lease_id {
            entries.push((CborValue::text("lease"), CborValue::text(l.clone())));
        }
        if let Some(e) = self.expires_at {
            entries.push((CborValue::text("exp"), CborValue::Unsigned(e)));
        }
        CborValue::Map(entries)
    }

    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError> {
        let entries = expect_map(payload)?;
        let port = get_unsigned(entries, "port")?;
        Ok(DatabaseCredential {
            label: get_text(entries, "label")?,
            engine: get_text(entries, "engine")?,
            host: get_text(entries, "host")?,
            port: u16::try_from(port).map_err(|_| VaultRecordError::SchemaMismatch {
                what: "DatabaseCredential.port out of u16",
            })?,
            database: get_optional_text(entries, "db")?,
            username: get_text(entries, "username")?,
            password: get_text(entries, "password")?,
            lease_id: get_optional_text(entries, "lease")?,
            expires_at: match find_entry(entries, "exp") {
                None => None,
                Some(CborValue::Unsigned(n)) => Some(*n),
                Some(_) => {
                    return Err(VaultRecordError::SchemaMismatch {
                        what: "DatabaseCredential.exp not unsigned",
                    });
                }
            },
        })
    }
}

macro_rules! impl_redacted_record_debug {
    ($($record:ident),+ $(,)?) => {
        $(
            impl core::fmt::Debug for $record {
                fn fmt(
                    &self,
                    f: &mut core::fmt::Formatter<'_>,
                ) -> core::fmt::Result {
                    f.write_str(concat!(stringify!($record), "(<redacted>)"))
                }
            }
        )+
    };
}

impl_redacted_record_debug!(
    Login,
    SecureNote,
    Card,
    TotpSeed,
    ApiKey,
    DatabaseCredential
);

// ─────────────────────────────────────────────────────────────────────
// 5. Map-walking helpers
// ─────────────────────────────────────────────────────────────────────
//
// These are the small accessors `decode_payload` impls use to look
// up named fields in the decoded CBOR map. They centralise the
// "missing required field → SchemaMismatch" logic so each record
// type's decoder reads cleanly.

type Entries = [(CborValue, CborValue)];

fn expect_map(v: &CborValue) -> Result<&Entries, VaultRecordError> {
    match v {
        CborValue::Map(e) => Ok(e),
        _ => Err(VaultRecordError::SchemaMismatch {
            what: "payload not a CBOR map",
        }),
    }
}

fn find_entry<'a>(entries: &'a Entries, key: &str) -> Option<&'a CborValue> {
    for (k, v) in entries {
        if let CborValue::Text(s) = k {
            if s == key {
                return Some(v);
            }
        }
    }
    None
}

fn get_text(entries: &Entries, key: &'static str) -> Result<String, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Text(s)) => Ok(s.clone()),
        Some(_) => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
        None => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
    }
}

fn get_optional_text(
    entries: &Entries,
    key: &'static str,
) -> Result<Option<String>, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Text(s)) => Ok(Some(s.clone())),
        Some(_) => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
        None => Ok(None),
    }
}

fn get_array<'a>(
    entries: &'a Entries,
    key: &'static str,
) -> Result<&'a [CborValue], VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Array(a)) => Ok(a.as_slice()),
        Some(_) => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
        None => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
    }
}

fn get_unsigned(entries: &Entries, key: &'static str) -> Result<u64, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Unsigned(n)) => Ok(*n),
        Some(_) => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
        None => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
    }
}

fn get_bytes(entries: &Entries, key: &'static str) -> Result<Vec<u8>, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Bytes(b)) => Ok(b.clone()),
        Some(_) => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
        None => Err(VaultRecordError::SchemaMismatch {
            what: missing_or_wrong(key),
        }),
    }
}

fn text_or_err(v: &CborValue, what: &'static str) -> Result<String, VaultRecordError> {
    match v {
        CborValue::Text(s) => Ok(s.clone()),
        _ => Err(VaultRecordError::SchemaMismatch { what }),
    }
}

/// Build a `&'static str` describing "field foo is missing or wrong type."
///
/// We need a static lifetime to plug into `SchemaMismatch.what`. Rust
/// can do this with a small lookup on known field names — but for
/// our handful of record types, we leak the formatted string into a
/// per-key static via a `match`. (Building a literal-only string
/// keeps the "no attacker-controlled bytes in errors" rule.)
fn missing_or_wrong(key: &'static str) -> &'static str {
    match key {
        "title" => "field 'title' missing or not text",
        "username" => "field 'username' missing or not text",
        "password" => "field 'password' missing or not text",
        "urls" => "field 'urls' missing or not array",
        "notes" => "field 'notes' wrong type (expected text)",
        "body" => "field 'body' missing or not text",
        "holder" => "field 'holder' missing or not text",
        "number" => "field 'number' missing or not text",
        "month" => "field 'month' missing or not unsigned",
        "year" => "field 'year' missing or not unsigned",
        "cvv" => "field 'cvv' missing or not text",
        "zip" => "field 'zip' wrong type (expected text)",
        "label" => "field 'label' missing or not text",
        "issuer" => "field 'issuer' wrong type (expected text)",
        "secret" => "field 'secret' missing or not bytes",
        "alg" => "field 'alg' missing or not text",
        "digits" => "field 'digits' missing or not unsigned",
        "period" => "field 'period' missing or not unsigned",
        "service" => "field 'service' missing or not text",
        "token" => "field 'token' missing or not text",
        "scopes" => "field 'scopes' missing or not array",
        "exp" => "field 'exp' wrong type (expected unsigned)",
        "engine" => "field 'engine' missing or not text",
        "host" => "field 'host' missing or not text",
        "port" => "field 'port' missing or not unsigned",
        "db" => "field 'db' wrong type (expected text)",
        "lease" => "field 'lease' wrong type (expected text)",
        _ => "required field missing or wrong type",
    }
}

// ─────────────────────────────────────────────────────────────────────
// 6. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_canonical_cbor::encode;

    fn sample_login() -> Login {
        Login {
            title: "GitHub".into(),
            username: "ada".into(),
            password: "p455w0rd".into(),
            urls: vec![
                "https://github.com".into(),
                "https://github.com/login".into(),
            ],
            notes: Some("personal account".into()),
        }
    }

    fn sample_card() -> Card {
        Card {
            title: "Personal Visa".into(),
            holder: "Ada Lovelace".into(),
            number: "4111111111111111".into(),
            expiry_month: 12,
            expiry_year: 2030,
            cvv: "123".into(),
            billing_zip: Some("OX1 4AR".into()),
        }
    }

    fn sample_totp() -> TotpSeed {
        TotpSeed {
            label: "GitHub: ada".into(),
            issuer: Some("GitHub".into()),
            secret: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34],
            algorithm: "SHA1".into(),
            digits: 6,
            period: 30,
        }
    }

    fn sample_api_key() -> ApiKey {
        ApiKey {
            label: "ci-prod".into(),
            service: "github.com".into(),
            token: "ghp_abc1234567890def".into(),
            scopes: vec!["repo".into(), "workflow".into()],
            expires_at: Some(1_900_000_000),
        }
    }

    fn sample_db() -> DatabaseCredential {
        DatabaseCredential {
            label: "prod-postgres-readonly".into(),
            engine: "postgres".into(),
            host: "db.internal".into(),
            port: 5432,
            database: Some("warehouse".into()),
            username: "ro_xyz".into(),
            password: "ephemeral_ro_password".into(),
            lease_id: Some("lease/abc-123".into()),
            expires_at: Some(1_800_000_000),
        }
    }

    fn sample_note() -> SecureNote {
        SecureNote {
            title: "WiFi password".into(),
            body: "SSID: HomeNet\nKey: hunter2".into(),
        }
    }

    #[test]
    fn record_debug_is_value_redacted() {
        let cases = [
            (format!("{:?}", sample_login()), "Login(<redacted>)"),
            (format!("{:?}", sample_note()), "SecureNote(<redacted>)"),
            (format!("{:?}", sample_card()), "Card(<redacted>)"),
            (format!("{:?}", sample_totp()), "TotpSeed(<redacted>)"),
            (format!("{:?}", sample_api_key()), "ApiKey(<redacted>)"),
            (
                format!("{:?}", sample_db()),
                "DatabaseCredential(<redacted>)",
            ),
        ];

        for (actual, expected) in cases {
            assert_eq!(actual, expected);
        }
        assert_eq!(format!("{:#?}", sample_login()), "Login(<redacted>)");
    }

    #[test]
    fn any_record_debug_is_value_redacted() {
        let cases = [
            (
                AnyRecord::Login(sample_login()),
                "AnyRecord::Login(<redacted>)",
            ),
            (
                AnyRecord::SecureNote(sample_note()),
                "AnyRecord::SecureNote(<redacted>)",
            ),
            (
                AnyRecord::Card(sample_card()),
                "AnyRecord::Card(<redacted>)",
            ),
            (
                AnyRecord::TotpSeed(sample_totp()),
                "AnyRecord::TotpSeed(<redacted>)",
            ),
            (
                AnyRecord::ApiKey(sample_api_key()),
                "AnyRecord::ApiKey(<redacted>)",
            ),
            (
                AnyRecord::DatabaseCredential(sample_db()),
                "AnyRecord::DatabaseCredential(<redacted>)",
            ),
            (
                AnyRecord::Opaque {
                    content_type: "vault/private-token/v1".into(),
                    payload_bytes: b"raw opaque secret".to_vec(),
                },
                "AnyRecord::Opaque(<redacted>)",
            ),
        ];

        for (record, expected) in cases {
            assert_eq!(format!("{record:?}"), expected);
            assert_eq!(format!("{record:#?}"), expected);
        }
    }

    // --- Per-type round-trips ---

    #[test]
    fn login_roundtrip() {
        let r = sample_login();
        let bytes = encode_record(&r).unwrap();
        let back = decode_record_as::<Login>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn note_roundtrip() {
        let r = sample_note();
        let bytes = encode_record(&r).unwrap();
        let back = decode_record_as::<SecureNote>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn card_roundtrip() {
        let r = sample_card();
        let bytes = encode_record(&r).unwrap();
        let back = decode_record_as::<Card>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn totp_roundtrip() {
        let r = sample_totp();
        let bytes = encode_record(&r).unwrap();
        let back = decode_record_as::<TotpSeed>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn api_key_roundtrip() {
        let r = sample_api_key();
        let bytes = encode_record(&r).unwrap();
        let back = decode_record_as::<ApiKey>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn db_credential_roundtrip() {
        let r = sample_db();
        let bytes = encode_record(&r).unwrap();
        let back = decode_record_as::<DatabaseCredential>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    // --- AnyRecord dispatch ---

    #[test]
    fn any_record_dispatches_login() {
        let r = sample_login();
        let bytes = encode_record(&r).unwrap();
        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::Login(l) => assert_eq!(l, r),
            _ => panic!("expected AnyRecord::Login"),
        }
    }

    #[test]
    fn any_record_dispatches_db_credential() {
        let r = sample_db();
        let bytes = encode_record(&r).unwrap();
        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::DatabaseCredential(d) => assert_eq!(d, r),
            _ => panic!("expected AnyRecord::DatabaseCredential"),
        }
    }

    #[test]
    fn any_record_summary_counts_login_shape_without_values() {
        let r = sample_login();
        let bytes = encode_record(&r).unwrap();
        let any = decode_record(&bytes).unwrap();
        let summary = any.summary();

        assert_eq!(any.kind(), VaultRecordKind::Login);
        assert_eq!(VaultRecordKind::Login.content_type(), Some(LOGIN_V1));
        assert_eq!(
            summary,
            VaultRecordSummary {
                kind: VaultRecordKind::Login,
                secret_field_count: 1,
                optional_field_count: 1,
                list_item_count: 2,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            }
        );
        assert!(summary.is_first_party());
        assert!(summary.is_password_manager_record());
        assert!(!summary.is_machine_secret_record());
        assert!(summary.carries_sensitive_fields());

        let debug = format!("{summary:?}");
        assert!(!debug.contains("p455w0rd"));
        assert!(!debug.contains("GitHub"));
        assert!(!debug.contains("personal account"));
    }

    #[test]
    fn any_record_summary_marks_machine_secret_lease_and_expiry() {
        let r = sample_db();
        let bytes = encode_record(&r).unwrap();
        let any = decode_record(&bytes).unwrap();
        let summary = any.summary();

        assert_eq!(
            summary,
            VaultRecordSummary {
                kind: VaultRecordKind::DatabaseCredential,
                secret_field_count: 1,
                optional_field_count: 3,
                list_item_count: 0,
                has_expiry: true,
                has_lease: true,
                opaque_content_type_len: 0,
                opaque_payload_bytes: 0,
            }
        );
        assert!(summary.is_first_party());
        assert!(!summary.is_password_manager_record());
        assert!(summary.is_machine_secret_record());
        assert!(summary.carries_sensitive_fields());

        let debug = format!("{summary:?}");
        assert!(!debug.contains("ephemeral_ro_password"));
        assert!(!debug.contains("lease/abc-123"));
        assert!(!debug.contains("db.internal"));
    }

    // --- Canonical idempotence ---

    #[test]
    fn encode_is_byte_stable_across_struct_orderings() {
        // Build "the same login" twice — the struct is the same so
        // canonical-CBOR must produce identical bytes.
        let bytes_a = encode_record(&sample_login()).unwrap();
        let bytes_b = encode_record(&sample_login()).unwrap();
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn decode_then_reencode_is_byte_stable() {
        let bytes = encode_record(&sample_card()).unwrap();
        let any = decode_record(&bytes).unwrap();
        let card = match any {
            AnyRecord::Card(c) => c,
            _ => unreachable!(),
        };
        let bytes2 = encode_record(&card).unwrap();
        assert_eq!(bytes, bytes2);
    }

    // --- Content-type rejection ---

    #[test]
    fn decode_record_as_rejects_wrong_content_type() {
        let bytes = encode_record(&sample_login()).unwrap();
        let err = decode_record_as::<SecureNote>(&bytes).unwrap_err();
        match err {
            VaultRecordError::ContentTypeMismatch { expected, actual } => {
                assert_eq!(expected, SECURE_NOTE_V1);
                assert_eq!(actual, LOGIN_V1);
            }
            other => panic!("expected ContentTypeMismatch, got {:?}", other),
        }
    }

    // --- Unknown content type → opaque pass-through ---

    #[test]
    fn unknown_content_type_decodes_as_opaque() {
        // Hand-build a record with an unknown content type.
        let envelope = CborValue::Map(vec![
            (
                CborValue::text("t"),
                CborValue::text("vault/biometric-prf-blob/v1".to_string()),
            ),
            (
                CborValue::text("d"),
                CborValue::Map(vec![(
                    CborValue::text("hash"),
                    CborValue::Bytes(vec![1, 2, 3, 4]),
                )]),
            ),
        ]);
        let bytes = encode(&envelope);

        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::Opaque {
                content_type,
                payload_bytes,
            } => {
                assert_eq!(content_type, "vault/biometric-prf-blob/v1");
                // payload_bytes is the canonical CBOR of {"hash":h'01020304'}.
                let payload = decode(&payload_bytes).unwrap();
                if let CborValue::Map(m) = payload {
                    assert_eq!(m.len(), 1);
                } else {
                    panic!("expected map");
                }
            }
            other => panic!("expected Opaque, got {:?}", other),
        }
    }

    #[test]
    fn opaque_summary_preserves_only_lengths() {
        let envelope = CborValue::Map(vec![
            (
                CborValue::text("t"),
                CborValue::text("vault/custom-secret/v9".to_string()),
            ),
            (
                CborValue::text("d"),
                CborValue::Map(vec![(
                    CborValue::text("token"),
                    CborValue::Bytes(vec![9, 8, 7]),
                )]),
            ),
        ]);
        let any = decode_record(&encode(&envelope)).unwrap();
        let summary = any.summary();

        let payload_bytes = match any {
            AnyRecord::Opaque { payload_bytes, .. } => payload_bytes,
            _ => unreachable!(),
        };
        assert_eq!(
            summary,
            VaultRecordSummary {
                kind: VaultRecordKind::Opaque,
                secret_field_count: 0,
                optional_field_count: 0,
                list_item_count: 0,
                has_expiry: false,
                has_lease: false,
                opaque_content_type_len: "vault/custom-secret/v9".len(),
                opaque_payload_bytes: payload_bytes.len(),
            }
        );
        assert!(!summary.is_first_party());
        assert!(summary.is_opaque());
        assert_eq!(VaultRecordKind::Opaque.content_type(), None);

        let debug = format!("{summary:?}");
        assert!(!debug.contains("vault/custom-secret/v9"));
        assert!(!debug.contains("token"));
    }

    #[test]
    fn opaque_roundtrip_via_encode_opaque() {
        let envelope = CborValue::Map(vec![
            (
                CborValue::text("t"),
                CborValue::text("vault/custom-app/v1".to_string()),
            ),
            (
                CborValue::text("d"),
                CborValue::Map(vec![
                    (CborValue::text("k1"), CborValue::Unsigned(42)),
                    (CborValue::text("k2"), CborValue::text("hello".to_string())),
                ]),
            ),
        ]);
        let bytes = encode(&envelope);
        let any = decode_record(&bytes).unwrap();
        let (ct, payload) = match any {
            AnyRecord::Opaque {
                content_type,
                payload_bytes,
            } => (content_type, payload_bytes),
            _ => unreachable!(),
        };
        let bytes2 = encode_opaque(&ct, &payload).unwrap();
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn encode_opaque_reports_an_undepictable_payload_instead_of_panicking() {
        use coding_adventures_canonical_cbor::MAX_DECODE_DEPTH;

        // `0x81` is a one-element array header, so a run of them is a chain of
        // singleton arrays around the final `0x00`. A chain exactly as deep as
        // the decoder allows still decodes on its own, but the envelope adds
        // one more level, which the encoder does not allow.
        let mut deepest_decodable = vec![0x81_u8; MAX_DECODE_DEPTH];
        deepest_decodable.push(0x00);
        assert!(decode(&deepest_decodable).is_ok());
        assert!(matches!(
            encode_opaque("vault/custom-app/v1", &deepest_decodable),
            Err(VaultRecordError::Cbor(CborError::EncodeTooDeep))
        ));

        // One level shallower leaves exactly enough room for the envelope, so
        // the boundary is proven from both sides.
        let mut deepest_wrappable = vec![0x81_u8; MAX_DECODE_DEPTH - 1];
        deepest_wrappable.push(0x00);
        assert!(encode_opaque("vault/custom-app/v1", &deepest_wrappable).is_ok());
    }

    /// The wire bytes a peer with a larger framing budget writes for one
    /// opaque record holding a byte string of `payload_len` bytes.
    ///
    /// Built directly rather than through `encode_opaque`, because past
    /// 1 MiB this product's own encoder will not write them. Canonical map
    /// order puts the equal-length key "d" before "t".
    fn peer_authored_opaque_wire(payload_len: usize) -> Vec<u8> {
        // Smallest-form length header for a byte string, the same choice
        // the canonical encoder makes -- a longer form would be rejected
        // by the decoder before any of this mattered.
        let header = match payload_len as u64 {
            0..=23 => vec![0x40 | payload_len as u8],
            24..=0xFF => vec![0x58, payload_len as u8],
            0x100..=0xFFFF => {
                let mut header = vec![0x59];
                header.extend_from_slice(&(payload_len as u16).to_be_bytes());
                header
            }
            _ => {
                let mut header = vec![0x5a];
                header.extend_from_slice(&(payload_len as u32).to_be_bytes());
                header
            }
        };
        let mut wire = vec![0xa2, 0x61, b'd'];
        wire.extend_from_slice(&header);
        wire.extend(std::iter::repeat_n(0x5a_u8, payload_len));
        wire.extend_from_slice(&[0x61, b't', 0x73]);
        wire.extend_from_slice(b"vault/custom-app/v1");
        wire
    }

    #[test]
    fn an_opaque_payload_too_large_to_re_encode_still_decodes() {
        use coding_adventures_canonical_cbor::MAX_ENCODED_SIZE;

        // The record class that used to deny the whole vault. A caller's
        // framing bound need not match the encoder's, so a record can
        // arrive that decodes and that the encoder would refuse to re-emit.
        // Because this decode runs during vault open, refusing it is not a
        // survivable failure -- so the arm must not re-encode at all.
        let wire = peer_authored_opaque_wire(MAX_ENCODED_SIZE);
        assert!(decode(&wire).is_ok(), "the peer's record must decode");

        let AnyRecord::Opaque {
            content_type,
            payload_bytes,
        } = decode_record(&wire).unwrap()
        else {
            panic!("an unknown content type must decode as opaque")
        };
        assert_eq!(content_type, "vault/custom-app/v1");
        // The payload is exactly the sub-slice from the wire: the four-byte
        // header plus the body, and nothing of the envelope around it.
        assert_eq!(payload_bytes.len(), MAX_ENCODED_SIZE + 5);
        assert_eq!(payload_bytes.as_slice(), &wire[3..wire.len() - 22]);

        // Writing it back is still refused, which is the correct place for
        // the ceiling: it costs one record rather than the vault.
        assert!(matches!(
            encode_opaque(&content_type, &payload_bytes),
            Err(VaultRecordError::Cbor(CborError::EncodeTooLarge))
        ));
    }

    #[test]
    fn opaque_payload_bytes_are_what_re_encoding_would_have_produced() {
        // Slicing replaced a re-encode, so the two have to agree wherever
        // the re-encode was possible at all. This is the property
        // `encode_opaque` round trips on, and the one the application's
        // authored opaque merge checks its input against.
        for payload_len in [0, 1, 23, 24, 255, 256, 65_535, 65_536] {
            let wire = peer_authored_opaque_wire(payload_len);
            let AnyRecord::Opaque { payload_bytes, .. } = decode_record(&wire).unwrap() else {
                panic!("an unknown content type must decode as opaque")
            };
            let payload = decode(&payload_bytes).unwrap();
            assert_eq!(
                payload_bytes,
                try_encode(&payload).unwrap(),
                "sliced and re-encoded payloads must agree at length {payload_len}",
            );
            assert_eq!(
                encode_opaque("vault/custom-app/v1", &payload_bytes).unwrap(),
                wire,
                "the record must round trip to the same wire bytes at length {payload_len}",
            );
        }
    }

    #[test]
    fn opaque_payload_bytes_are_sliced_for_every_payload_shape() {
        // The span the decoder reports has to be right for every value
        // shape, not just the byte strings a large payload happens to use:
        // an off-by-one would silently store a different payload.
        for payload in [
            CborValue::Unsigned(0),
            CborValue::Unsigned(u64::MAX),
            CborValue::Negative(41),
            CborValue::Bytes(Vec::new()),
            CborValue::text("text"),
            CborValue::Array(vec![CborValue::Null, CborValue::Bool(true)]),
            CborValue::Map(vec![(CborValue::text("k"), CborValue::Unsigned(1))]),
            CborValue::Tag(7, Box::new(CborValue::text("tagged"))),
            CborValue::Null,
        ] {
            let expected = try_encode(&payload).unwrap();
            let wire = encode_opaque("vault/custom-app/v1", &expected).unwrap();
            let AnyRecord::Opaque { payload_bytes, .. } = decode_record(&wire).unwrap() else {
                panic!("an unknown content type must decode as opaque")
            };
            assert_eq!(
                payload_bytes, expected,
                "payload {payload:?} must slice out"
            );
        }
    }

    /// A `Login` whose password is `n` bytes long and whose every other
    /// field is as short as the schema permits, so the encoded length is
    /// `n` plus a constant.
    fn login_with_password_len(n: usize) -> Login {
        Login {
            title: String::new(),
            username: String::new(),
            password: "a".repeat(n),
            urls: Vec::new(),
            notes: None,
        }
    }

    #[test]
    fn encode_record_reports_an_oversized_record_instead_of_panicking() {
        use coding_adventures_canonical_cbor::MAX_ENCODED_SIZE;

        // The boundary is derived, not guessed, so it stays exact if the
        // schema or the ceiling ever moves.
        //
        // CBOR text headers are length-prefixed, and the prefix widens in
        // steps: 1 byte below 24, then 2, 3, 5, 9. Measuring the constant
        // overhead at a password length that is already in the same
        // 5-byte-header bracket as `MAX_ENCODED_SIZE` (anything in
        // 65_536 ..= 4_294_967_295) means the constant really is constant
        // across the arithmetic below.
        const PROBE: usize = 65_536;
        let probe_len = encode_record(&login_with_password_len(PROBE))
            .expect("the probe is far below the ceiling")
            .len();
        let overhead = probe_len - PROBE;

        // Exactly at the ceiling: accepted, and it really is exact.
        let largest = MAX_ENCODED_SIZE - overhead;
        let at_ceiling = encode_record(&login_with_password_len(largest))
            .expect("a record encoding to exactly MAX_ENCODED_SIZE is legal");
        assert_eq!(at_ceiling.len(), MAX_ENCODED_SIZE);

        // One single byte more: refused, and refused *without panicking*,
        // which is the whole point. This is the record a peer device with
        // a 16 MiB plaintext budget can legally author and sync to us.
        assert!(matches!(
            encode_record(&login_with_password_len(largest + 1)),
            Err(VaultRecordError::Cbor(CborError::EncodeTooLarge))
        ));

        // And far past it, the way a real poisoned record looks: 2 MiB is
        // comfortably inside vault-pm's 16 MiB plaintext gate and
        // comfortably outside the encoder's 1 MiB one.
        assert!(matches!(
            encode_record(&login_with_password_len(2 * 1024 * 1024)),
            Err(VaultRecordError::Cbor(CborError::EncodeTooLarge))
        ));
    }

    #[test]
    fn every_first_party_record_type_reports_oversize() {
        // The `Result` sits on the generic entry point rather than inside
        // any one schema, so the guarantee has to hold for all six. Each
        // record below is built from the crate's own sample and then has a
        // single 2 MiB field substituted, which is the shape a peer record
        // between the two ceilings actually takes.
        let huge = "a".repeat(2 * 1024 * 1024);
        let too_large = |result: Result<Vec<u8>, VaultRecordError>| {
            matches!(
                result,
                Err(VaultRecordError::Cbor(CborError::EncodeTooLarge))
            )
        };

        let mut login = sample_login();
        login.password = huge.clone();
        assert!(too_large(encode_record(&login)));

        let mut note = sample_note();
        note.body = huge.clone();
        assert!(too_large(encode_record(&note)));

        let mut card = sample_card();
        card.number = huge.clone();
        assert!(too_large(encode_record(&card)));

        let mut totp = sample_totp();
        // The TOTP seed is raw bytes rather than text, so the same
        // oversize arrives through a different CBOR major type.
        totp.secret = huge.clone().into_bytes();
        assert!(too_large(encode_record(&totp)));

        let mut api_key = sample_api_key();
        api_key.token = huge.clone();
        assert!(too_large(encode_record(&api_key)));

        let mut db = sample_db();
        db.password = huge;
        assert!(too_large(encode_record(&db)));
    }

    // --- Schema mismatch rejection ---

    #[test]
    fn login_missing_password_is_schema_mismatch() {
        // Hand-build a login record with no password field.
        let envelope = CborValue::Map(vec![
            (CborValue::text("t"), CborValue::text(LOGIN_V1.to_string())),
            (
                CborValue::text("d"),
                CborValue::Map(vec![
                    (CborValue::text("title"), CborValue::text("x".to_string())),
                    (
                        CborValue::text("username"),
                        CborValue::text("y".to_string()),
                    ),
                    (CborValue::text("urls"), CborValue::Array(vec![])),
                ]),
            ),
        ]);
        let bytes = encode(&envelope);
        let err = decode_record_as::<Login>(&bytes).unwrap_err();
        assert!(matches!(err, VaultRecordError::SchemaMismatch { .. }));
    }

    #[test]
    fn card_with_invalid_month_is_schema_mismatch() {
        let mut c = sample_card();
        c.expiry_month = 13;
        let bytes = encode_record(&c).unwrap();
        let err = decode_record_as::<Card>(&bytes).unwrap_err();
        match err {
            VaultRecordError::SchemaMismatch { what } => {
                assert!(what.contains("month"));
            }
            other => panic!("expected SchemaMismatch, got {:?}", other),
        }
    }

    #[test]
    fn totp_with_invalid_digits_is_schema_mismatch() {
        let mut t = sample_totp();
        t.digits = 100;
        let bytes = encode_record(&t).unwrap();
        let err = decode_record_as::<TotpSeed>(&bytes).unwrap_err();
        assert!(matches!(err, VaultRecordError::SchemaMismatch { .. }));
    }

    // --- Schema mismatch through `decode_record` is quarantined, not denied ---
    //
    // The three tests above prove `decode_record_as::<T>` — a caller asking
    // for one specific type — still fails closed on schema mismatch. These
    // prove `decode_record` — the general decoder `decode_live` in the
    // application layer calls at vault-open time — does not: it must
    // materialise something rather than propagate the error, because
    // propagating it here is exactly the bug this quarantine variant
    // exists to close (see the `decode_record` doc comment).

    #[test]
    fn login_missing_password_via_decode_record_is_quarantined_not_denied() {
        // Same malformed bytes as `login_missing_password_is_schema_mismatch`,
        // but through the general decoder instead of the type-specific one.
        let envelope = CborValue::Map(vec![
            (CborValue::text("t"), CborValue::text(LOGIN_V1.to_string())),
            (
                CborValue::text("d"),
                CborValue::Map(vec![
                    (CborValue::text("title"), CborValue::text("x".to_string())),
                    (
                        CborValue::text("username"),
                        CborValue::text("y".to_string()),
                    ),
                    (CborValue::text("urls"), CborValue::Array(vec![])),
                ]),
            ),
        ]);
        let bytes = encode(&envelope);

        let any = decode_record(&bytes).expect(
            "a schema-mismatched but well-formed-envelope record must still decode \
             (as Quarantined), never deny the whole decode",
        );
        match any {
            AnyRecord::Quarantined {
                content_type,
                payload_bytes,
                reason,
            } => {
                assert_eq!(content_type, LOGIN_V1);
                assert!(reason.contains("password"));
                // The quarantined bytes are exactly the inner "d" payload's
                // own canonical CBOR — the same slice-not-re-encode contract
                // the Opaque arm already gives.
                let payload = decode(&payload_bytes).unwrap();
                if let CborValue::Map(entries) = payload {
                    assert_eq!(entries.len(), 3);
                } else {
                    panic!("expected map");
                }
            }
            other => panic!("expected Quarantined, got {:?}", other),
        }
    }

    #[test]
    fn card_with_invalid_month_via_decode_record_is_quarantined() {
        let mut c = sample_card();
        c.expiry_month = 13;
        let bytes = encode_record(&c).unwrap();
        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::Quarantined {
                content_type,
                reason,
                ..
            } => {
                assert_eq!(content_type, CARD_V1);
                assert!(reason.contains("month"));
            }
            other => panic!("expected Quarantined, got {:?}", other),
        }
    }

    #[test]
    fn quarantined_record_kind_and_summary_are_distinct_from_opaque() {
        let mut t = sample_totp();
        t.digits = 100;
        let bytes = encode_record(&t).unwrap();
        let any = decode_record(&bytes).unwrap();

        assert_eq!(any.kind(), VaultRecordKind::Quarantined);
        assert_ne!(any.kind(), VaultRecordKind::Opaque);
        assert!(!any.kind().is_first_party());

        let summary = any.summary();
        assert_eq!(summary.kind, VaultRecordKind::Quarantined);
        assert!(summary.is_quarantined());
        assert!(!summary.is_opaque());
        assert!(!summary.is_first_party());
        assert_eq!(summary.opaque_content_type_len, TOTP_SEED_V1.len());
        assert!(summary.opaque_payload_bytes > 0);
    }

    #[test]
    fn quarantined_record_debug_is_value_redacted() {
        let mut c = sample_card();
        c.expiry_month = 13;
        let bytes = encode_record(&c).unwrap();
        let any = decode_record(&bytes).unwrap();
        assert_eq!(format!("{any:?}"), "AnyRecord::Quarantined(<redacted>)");
    }

    #[test]
    fn a_malformed_envelope_still_denies_decode_record_entirely() {
        // Quarantine only widens the *schema-mismatch* case. A record whose
        // envelope itself is broken (not a {t,d} map, "t" the wrong type,
        // etc.) is not something any content type can be attributed to, and
        // must still fail outright rather than being silently accepted as
        // some fabricated quarantine record.
        let not_a_record = encode(&CborValue::Array(vec![CborValue::Unsigned(1)]));
        assert!(matches!(
            decode_record(&not_a_record),
            Err(VaultRecordError::NotARecord)
        ));

        let bad_envelope = CborValue::Map(vec![
            (CborValue::text("t"), CborValue::Unsigned(1)),
            (CborValue::text("d"), CborValue::Map(vec![])),
        ]);
        assert!(matches!(
            decode_record(&encode(&bad_envelope)),
            Err(VaultRecordError::BadEnvelope)
        ));
    }

    // --- Envelope rejection ---

    #[test]
    fn decode_rejects_top_level_array() {
        let bytes = encode(&CborValue::Array(vec![CborValue::Unsigned(1)]));
        let err = decode_record(&bytes).unwrap_err();
        assert!(matches!(err, VaultRecordError::NotARecord));
    }

    #[test]
    fn decode_rejects_envelope_with_extra_field() {
        let envelope = CborValue::Map(vec![
            (CborValue::text("t"), CborValue::text(LOGIN_V1.to_string())),
            (CborValue::text("d"), CborValue::Map(vec![])),
            (CborValue::text("x"), CborValue::Unsigned(42)),
        ]);
        let bytes = encode(&envelope);
        let err = decode_record(&bytes).unwrap_err();
        assert!(matches!(err, VaultRecordError::NotARecord));
    }

    #[test]
    fn decode_rejects_a_two_entry_map_whose_keys_are_not_t_and_d() {
        // Two entries is the right *count*, so this reaches the key match
        // rather than the arity check -- the one branch of the shared
        // envelope routine that neither arity nor value-type tests cover.
        let envelope = CborValue::Map(vec![
            (CborValue::text("a"), CborValue::text(LOGIN_V1.to_string())),
            (CborValue::text("b"), CborValue::Map(vec![])),
        ]);
        let bytes = encode(&envelope);
        assert!(matches!(
            decode_record(&bytes).unwrap_err(),
            VaultRecordError::BadEnvelope
        ));
        assert!(matches!(
            decode_record_as::<Login>(&bytes).unwrap_err(),
            VaultRecordError::BadEnvelope
        ));
    }

    #[test]
    fn decode_rejects_envelope_with_t_not_text() {
        let envelope = CborValue::Map(vec![
            (CborValue::text("t"), CborValue::Unsigned(1)),
            (CborValue::text("d"), CborValue::Map(vec![])),
        ]);
        let bytes = encode(&envelope);
        let err = decode_record(&bytes).unwrap_err();
        assert!(matches!(err, VaultRecordError::BadEnvelope));
    }

    // --- Forward compatibility: extra unknown fields are tolerated ---

    #[test]
    fn extra_unknown_fields_in_payload_are_ignored() {
        // Take a Login and inject an extra field "future_field".
        // (Build the CBOR manually to bypass encode_payload.)
        let envelope = CborValue::Map(vec![
            (CborValue::text("t"), CborValue::text(LOGIN_V1.to_string())),
            (
                CborValue::text("d"),
                CborValue::Map(vec![
                    (CborValue::text("title"), CborValue::text("x".to_string())),
                    (
                        CborValue::text("username"),
                        CborValue::text("y".to_string()),
                    ),
                    (
                        CborValue::text("password"),
                        CborValue::text("z".to_string()),
                    ),
                    (CborValue::text("urls"), CborValue::Array(vec![])),
                    (
                        CborValue::text("future_field"),
                        CborValue::Bytes(vec![0xAA, 0xBB]),
                    ),
                ]),
            ),
        ]);
        let bytes = encode(&envelope);
        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::Login(_) => {} // succeeded
            other => panic!("expected Login, got {:?}", other),
        }
    }

    // --- Display strings come from literals only ---

    #[test]
    fn error_display_strings_are_static() {
        let errs: Vec<VaultRecordError> = vec![
            VaultRecordError::Cbor(CborError::UnexpectedEof),
            VaultRecordError::NotARecord,
            VaultRecordError::BadEnvelope,
            VaultRecordError::ContentTypeMismatch {
                expected: "vault/login/v1",
                actual: "ATTACKER\u{0}\u{1}\u{2}".into(),
            },
            VaultRecordError::SchemaMismatch { what: "x" },
        ];
        for e in &errs {
            let s = e.to_string();
            assert!(s.starts_with("vault-records:"));
            // The Display for ContentTypeMismatch must NOT include
            // the attacker-controlled `actual` value.
            if let VaultRecordError::ContentTypeMismatch { .. } = e {
                assert!(!s.contains("ATTACKER"));
            }
        }
    }

    #[test]
    fn error_debug_strings_are_static() {
        let cases = [
            (
                VaultRecordError::Cbor(CborError::UnexpectedEof),
                "VaultRecordError::Cbor",
            ),
            (VaultRecordError::NotARecord, "VaultRecordError::NotARecord"),
            (
                VaultRecordError::BadEnvelope,
                "VaultRecordError::BadEnvelope",
            ),
            (
                VaultRecordError::ContentTypeMismatch {
                    expected: LOGIN_V1,
                    actual: "ATTACKER-CONTROLLED-CONTENT-TYPE".into(),
                },
                "VaultRecordError::ContentTypeMismatch",
            ),
            (
                VaultRecordError::SchemaMismatch {
                    what: "static schema detail",
                },
                "VaultRecordError::SchemaMismatch",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(format!("{error:?}"), expected);
            assert_eq!(format!("{error:#?}"), expected);
        }
    }

    // --- CborValue zeroization (item #9) ---

    /// Every leaf of a `CborValue` tree, no matter how deeply nested
    /// under `Array`/`Map`/`Tag`, is empty after `zeroize_cbor_value`.
    /// The checker below has its own exhaustive match (no wildcard),
    /// so this test is a second, independent enforcement point beyond
    /// `zeroize_cbor_value`'s own: if canonical-cbor ever grows a new
    /// variant, *this* match fails to compile too, not just the one in
    /// the production code it's checking.
    #[test]
    fn zeroize_cbor_value_wipes_every_variant() {
        fn assert_no_plaintext_remains(v: &CborValue) {
            match v {
                CborValue::Text(s) => assert!(s.is_empty(), "unwiped Text leaf: {s:?}"),
                CborValue::Bytes(b) => assert!(b.is_empty(), "unwiped Bytes leaf: {b:?}"),
                CborValue::Array(items) => items.iter().for_each(assert_no_plaintext_remains),
                CborValue::Map(entries) => entries.iter().for_each(|(k, v)| {
                    assert_no_plaintext_remains(k);
                    assert_no_plaintext_remains(v);
                }),
                CborValue::Tag(_, inner) => assert_no_plaintext_remains(inner),
                CborValue::Unsigned(_)
                | CborValue::Negative(_)
                | CborValue::Bool(_)
                | CborValue::Null => {}
            }
        }

        let mut value = CborValue::Map(vec![
            (
                CborValue::text("password"),
                CborValue::text("hunter2-super-secret"),
            ),
            (
                CborValue::text("totp-seed"),
                CborValue::bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            ),
            (
                CborValue::text("nested"),
                CborValue::Array(vec![
                    CborValue::text("nested-secret-one"),
                    CborValue::bytes(b"nested-secret-two".to_vec()),
                    CborValue::Tag(
                        61,
                        Box::new(CborValue::Map(vec![(
                            CborValue::text("k"),
                            CborValue::text("tagged-and-mapped-secret"),
                        )])),
                    ),
                ]),
            ),
            (CborValue::text("count"), CborValue::Unsigned(42)),
            (CborValue::text("delta"), CborValue::Negative(7)),
            (CborValue::text("flag"), CborValue::Bool(true)),
            (CborValue::text("absent"), CborValue::Null),
        ]);

        zeroize_cbor_value(&mut value);

        // The map's own keys are plaintext field names, not secrets —
        // but they went through the same recursive walk as everything
        // else, so confirming they're wiped too proves the walk really
        // is unconditional rather than special-casing "looks secret."
        assert_no_plaintext_remains(&value);
    }

    #[test]
    fn zeroize_cbor_value_on_scalars_is_a_harmless_no_op() {
        for mut v in [
            CborValue::Unsigned(9),
            CborValue::Negative(3),
            CborValue::Bool(false),
            CborValue::Null,
        ] {
            let before = v.clone();
            zeroize_cbor_value(&mut v);
            assert_eq!(v, before);
        }
    }

    #[test]
    fn secret_cbor_value_derefs_for_read_access_before_drop() {
        let secret = SecretCborValue::new(CborValue::Map(vec![(
            CborValue::text("password"),
            CborValue::text("still-readable"),
        )]));
        // `expect_map`/`try_encode`/`VaultRecord::decode_payload` all
        // take `&CborValue`; this is the coercion `encode_record`,
        // `encode_opaque`, and every typed decode rely on.
        let entries = expect_map(&secret).unwrap();
        assert_eq!(get_text(entries, "password").unwrap(), "still-readable");
    }

    /// Proves `SecretCborValue::drop` actually runs the wipe — on the
    /// ordinary success path, on early return, *and* on panic unwind —
    /// without reading through a pointer into memory the real `Drop`
    /// has already deallocated (unsound, and this crate forbids
    /// `unsafe` outright). Instead this observes the one side effect
    /// `Drop::drop` produces that's safe to check after the fact: the
    /// `#[cfg(test)]` counter it increments. `SECRET_CBOR_VALUE_DROPS`
    /// is a single process-wide counter shared with every other test
    /// that (directly or via `encode_record`/`decode_record`) drops a
    /// `SecretCborValue` concurrently, so this only ever asserts the
    /// counter moved *forward*, never an exact value — the one
    /// assertion that stays true no matter what else is running.
    #[test]
    fn secret_cbor_value_drop_runs_even_on_panic_unwind() {
        use core::sync::atomic::Ordering;
        use std::panic::{catch_unwind, AssertUnwindSafe};

        // Ordinary scope exit.
        let before = SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed);
        {
            let _secret = SecretCborValue::new(CborValue::text("wiped-on-scope-exit"));
        }
        assert!(
            SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed) > before,
            "SecretCborValue::drop did not run on ordinary scope exit"
        );

        // Panic unwind mid-operation, the case the rejected
        // error-path-only quick fix (see the section 2a comment above
        // `zeroize_cbor_value`) would not have covered.
        let before = SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _secret = SecretCborValue::new(CborValue::text("wiped-on-panic-unwind"));
            panic!("simulated mid-operation failure");
        }));
        assert!(result.is_err(), "panic did not propagate");
        assert!(
            SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed) > before,
            "SecretCborValue::drop did not run during unwind"
        );
    }

    /// End-to-end: `encode_record` and `decode_record` each build
    /// exactly one caller-side `SecretCborValue` around real record
    /// plaintext, and each one drops (wiping it) by the time the
    /// function returns.
    #[test]
    fn encode_and_decode_record_each_wipe_their_own_cbor_tree() {
        use core::sync::atomic::Ordering;

        let login = sample_login();

        let before = SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed);
        let bytes = encode_record(&login).unwrap();
        assert!(
            SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed) > before,
            "encode_record did not wipe its envelope"
        );

        let before = SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed);
        let decoded = decode_record(&bytes).unwrap();
        assert!(
            SECRET_CBOR_VALUE_DROPS.load(Ordering::Relaxed) > before,
            "decode_record did not wipe its payload"
        );
        assert!(matches!(decoded, AnyRecord::Login(_)));
    }
}
