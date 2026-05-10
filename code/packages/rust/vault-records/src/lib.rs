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
//!   Apps register custom types as they need.
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
//! let bytes = encode_record(&login);                  // canonical CBOR
//! let back  = decode_record(&bytes).unwrap();         // AnyRecord
//! match back {
//!     AnyRecord::Login(l) => assert_eq!(l, login),
//!     _ => unreachable!(),
//! }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_canonical_cbor::{decode, encode, CborError, CborValue};
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
#[derive(Debug)]
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

impl core::fmt::Display for VaultRecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VaultRecordError::Cbor(_) => write!(f, "vault-records: canonical-CBOR codec failed"),
            VaultRecordError::NotARecord => {
                write!(f, "vault-records: top-level item was not a {{t,d}} map")
            }
            VaultRecordError::BadEnvelope => {
                write!(f, "vault-records: envelope is missing or has wrong-typed t/d fields")
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
// 3. Top-level encode / decode
// ─────────────────────────────────────────────────────────────────────

/// Encode a `VaultRecord` to canonical CBOR bytes with its
/// content-type tag. Output is deterministic.
pub fn encode_record<T: VaultRecord>(rec: &T) -> Vec<u8> {
    let envelope = CborValue::Map(vec![
        (CborValue::text("t"), CborValue::text(T::CONTENT_TYPE)),
        (CborValue::text("d"), rec.encode_payload()),
    ]);
    encode(&envelope)
}

/// Decode any vault record. Returns an [`AnyRecord`] which
/// pattern-matches on the content type. Unknown types are returned
/// as `AnyRecord::Opaque` so old clients do not crash on records
/// produced by newer ones.
pub fn decode_record(bytes: &[u8]) -> Result<AnyRecord, VaultRecordError> {
    let v = decode(bytes)?;
    let (content_type, payload) = split_envelope(v)?;
    Ok(match content_type.as_str() {
        LOGIN_V1 => AnyRecord::Login(Login::decode_payload(&payload)?),
        SECURE_NOTE_V1 => AnyRecord::SecureNote(SecureNote::decode_payload(&payload)?),
        CARD_V1 => AnyRecord::Card(Card::decode_payload(&payload)?),
        TOTP_SEED_V1 => AnyRecord::TotpSeed(TotpSeed::decode_payload(&payload)?),
        API_KEY_V1 => AnyRecord::ApiKey(ApiKey::decode_payload(&payload)?),
        DATABASE_CREDENTIAL_V1 => {
            AnyRecord::DatabaseCredential(DatabaseCredential::decode_payload(&payload)?)
        }
        // Unknown / app-specific / future-version: re-encode the
        // payload bytes verbatim and return as opaque.
        _ => AnyRecord::Opaque {
            content_type,
            payload_bytes: encode(&payload),
        },
    })
}

/// Decode bytes as a specific known record type. Returns
/// [`VaultRecordError::ContentTypeMismatch`] if the content type
/// doesn't match `T::CONTENT_TYPE`.
pub fn decode_record_as<T: VaultRecord>(bytes: &[u8]) -> Result<T, VaultRecordError> {
    let v = decode(bytes)?;
    let (content_type, payload) = split_envelope(v)?;
    if content_type != T::CONTENT_TYPE {
        return Err(VaultRecordError::ContentTypeMismatch {
            expected: T::CONTENT_TYPE,
            actual: content_type,
        });
    }
    T::decode_payload(&payload)
}

/// Helper: peel off the `{t, d}` envelope. Returns `(content_type, payload)`.
fn split_envelope(v: CborValue) -> Result<(String, CborValue), VaultRecordError> {
    let entries = match v {
        CborValue::Map(e) => e,
        _ => return Err(VaultRecordError::NotARecord),
    };
    if entries.len() != 2 {
        return Err(VaultRecordError::NotARecord);
    }
    let mut t: Option<String> = None;
    let mut d: Option<CborValue> = None;
    for (k, val) in entries {
        match k {
            CborValue::Text(s) if s == "t" => match val {
                CborValue::Text(s) => t = Some(s),
                _ => return Err(VaultRecordError::BadEnvelope),
            },
            CborValue::Text(s) if s == "d" => d = Some(val),
            _ => return Err(VaultRecordError::BadEnvelope),
        }
    }
    match (t, d) {
        (Some(t), Some(d)) => Ok((t, d)),
        _ => Err(VaultRecordError::BadEnvelope),
    }
}

/// One of the known record types, or an opaque pass-through for
/// content types this crate doesn't recognise.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            AnyRecord::Opaque { content_type, payload_bytes } => {
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

/// Re-encode an [`AnyRecord::Opaque`] back to its full
/// envelope-wrapped canonical CBOR bytes. Useful for forwarding a
/// record of unknown type without losing it.
pub fn encode_opaque(content_type: &str, payload_bytes: &[u8]) -> Result<Vec<u8>, VaultRecordError> {
    let payload = decode(payload_bytes)?;
    let envelope = CborValue::Map(vec![
        (CborValue::text("t"), CborValue::text(content_type.to_string())),
        (CborValue::text("d"), payload),
    ]);
    Ok(encode(&envelope))
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
            (CborValue::text("title"), CborValue::text(self.title.clone())),
            (CborValue::text("username"), CborValue::text(self.username.clone())),
            (CborValue::text("password"), CborValue::text(self.password.clone())),
            (
                CborValue::text("urls"),
                CborValue::Array(self.urls.iter().map(|u| CborValue::text(u.clone())).collect()),
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
            (CborValue::text("title"), CborValue::text(self.title.clone())),
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
            (CborValue::text("title"), CborValue::text(self.title.clone())),
            (CborValue::text("holder"), CborValue::text(self.holder.clone())),
            (CborValue::text("number"), CborValue::text(self.number.clone())),
            (CborValue::text("month"), CborValue::Unsigned(self.expiry_month as u64)),
            (CborValue::text("year"), CborValue::Unsigned(self.expiry_year as u64)),
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
            return Err(VaultRecordError::SchemaMismatch { what: "Card.month not in 1..=12" });
        }
        Ok(Card {
            title: get_text(entries, "title")?,
            holder: get_text(entries, "holder")?,
            number: get_text(entries, "number")?,
            expiry_month: month as u8,
            expiry_year: u16::try_from(year)
                .map_err(|_| VaultRecordError::SchemaMismatch { what: "Card.year out of u16" })?,
            cvv: get_text(entries, "cvv")?,
            billing_zip: get_optional_text(entries, "zip")?,
        })
    }
}

/// A TOTP / HOTP seed (the shared secret an authenticator app stores
/// for one account). Useful when the vault is also acting as the
/// user's authenticator.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            (CborValue::text("label"), CborValue::text(self.label.clone())),
            (CborValue::text("secret"), CborValue::Bytes(self.secret.clone())),
            (CborValue::text("alg"), CborValue::text(self.algorithm.clone())),
            (CborValue::text("digits"), CborValue::Unsigned(self.digits as u64)),
            (CborValue::text("period"), CborValue::Unsigned(self.period as u64)),
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
            return Err(VaultRecordError::SchemaMismatch { what: "TotpSeed.digits not in 4..=10" });
        }
        Ok(TotpSeed {
            label: get_text(entries, "label")?,
            issuer: get_optional_text(entries, "issuer")?,
            secret: get_bytes(entries, "secret")?,
            algorithm: get_text(entries, "alg")?,
            digits: digits as u8,
            period: u32::try_from(period).map_err(|_| {
                VaultRecordError::SchemaMismatch { what: "TotpSeed.period out of u32" }
            })?,
        })
    }
}

/// An API key — the machine-secret-store equivalent of `Login`.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            (CborValue::text("label"), CborValue::text(self.label.clone())),
            (CborValue::text("service"), CborValue::text(self.service.clone())),
            (CborValue::text("token"), CborValue::text(self.token.clone())),
            (
                CborValue::text("scopes"),
                CborValue::Array(self.scopes.iter().map(|s| CborValue::text(s.clone())).collect()),
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
            (CborValue::text("label"), CborValue::text(self.label.clone())),
            (CborValue::text("engine"), CborValue::text(self.engine.clone())),
            (CborValue::text("host"), CborValue::text(self.host.clone())),
            (CborValue::text("port"), CborValue::Unsigned(self.port as u64)),
            (CborValue::text("username"), CborValue::text(self.username.clone())),
            (CborValue::text("password"), CborValue::text(self.password.clone())),
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
            port: u16::try_from(port).map_err(|_| {
                VaultRecordError::SchemaMismatch { what: "DatabaseCredential.port out of u16" }
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
        _ => Err(VaultRecordError::SchemaMismatch { what: "payload not a CBOR map" }),
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
        Some(_) => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
        None => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
    }
}

fn get_optional_text(entries: &Entries, key: &'static str) -> Result<Option<String>, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Text(s)) => Ok(Some(s.clone())),
        Some(_) => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
        None => Ok(None),
    }
}

fn get_array<'a>(entries: &'a Entries, key: &'static str) -> Result<&'a [CborValue], VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Array(a)) => Ok(a.as_slice()),
        Some(_) => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
        None => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
    }
}

fn get_unsigned(entries: &Entries, key: &'static str) -> Result<u64, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Unsigned(n)) => Ok(*n),
        Some(_) => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
        None => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
    }
}

fn get_bytes(entries: &Entries, key: &'static str) -> Result<Vec<u8>, VaultRecordError> {
    match find_entry(entries, key) {
        Some(CborValue::Bytes(b)) => Ok(b.clone()),
        Some(_) => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
        None => Err(VaultRecordError::SchemaMismatch { what: missing_or_wrong(key) }),
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

    fn sample_login() -> Login {
        Login {
            title: "GitHub".into(),
            username: "ada".into(),
            password: "p455w0rd".into(),
            urls: vec!["https://github.com".into(), "https://github.com/login".into()],
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

    // --- Per-type round-trips ---

    #[test]
    fn login_roundtrip() {
        let r = sample_login();
        let bytes = encode_record(&r);
        let back = decode_record_as::<Login>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn note_roundtrip() {
        let r = sample_note();
        let bytes = encode_record(&r);
        let back = decode_record_as::<SecureNote>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn card_roundtrip() {
        let r = sample_card();
        let bytes = encode_record(&r);
        let back = decode_record_as::<Card>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn totp_roundtrip() {
        let r = sample_totp();
        let bytes = encode_record(&r);
        let back = decode_record_as::<TotpSeed>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn api_key_roundtrip() {
        let r = sample_api_key();
        let bytes = encode_record(&r);
        let back = decode_record_as::<ApiKey>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn db_credential_roundtrip() {
        let r = sample_db();
        let bytes = encode_record(&r);
        let back = decode_record_as::<DatabaseCredential>(&bytes).unwrap();
        assert_eq!(back, r);
    }

    // --- AnyRecord dispatch ---

    #[test]
    fn any_record_dispatches_login() {
        let r = sample_login();
        let bytes = encode_record(&r);
        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::Login(l) => assert_eq!(l, r),
            _ => panic!("expected AnyRecord::Login"),
        }
    }

    #[test]
    fn any_record_dispatches_db_credential() {
        let r = sample_db();
        let bytes = encode_record(&r);
        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::DatabaseCredential(d) => assert_eq!(d, r),
            _ => panic!("expected AnyRecord::DatabaseCredential"),
        }
    }

    // --- Canonical idempotence ---

    #[test]
    fn encode_is_byte_stable_across_struct_orderings() {
        // Build "the same login" twice — the struct is the same so
        // canonical-CBOR must produce identical bytes.
        let bytes_a = encode_record(&sample_login());
        let bytes_b = encode_record(&sample_login());
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn decode_then_reencode_is_byte_stable() {
        let bytes = encode_record(&sample_card());
        let any = decode_record(&bytes).unwrap();
        let card = match any {
            AnyRecord::Card(c) => c,
            _ => unreachable!(),
        };
        let bytes2 = encode_record(&card);
        assert_eq!(bytes, bytes2);
    }

    // --- Content-type rejection ---

    #[test]
    fn decode_record_as_rejects_wrong_content_type() {
        let bytes = encode_record(&sample_login());
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
                CborValue::Map(vec![
                    (CborValue::text("hash"), CborValue::Bytes(vec![1, 2, 3, 4])),
                ]),
            ),
        ]);
        let bytes = encode(&envelope);

        let any = decode_record(&bytes).unwrap();
        match any {
            AnyRecord::Opaque { content_type, payload_bytes } => {
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
            AnyRecord::Opaque { content_type, payload_bytes } => (content_type, payload_bytes),
            _ => unreachable!(),
        };
        let bytes2 = encode_opaque(&ct, &payload).unwrap();
        assert_eq!(bytes, bytes2);
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
                    (CborValue::text("username"), CborValue::text("y".to_string())),
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
        let bytes = encode_record(&c);
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
        let bytes = encode_record(&t);
        let err = decode_record_as::<TotpSeed>(&bytes).unwrap_err();
        assert!(matches!(err, VaultRecordError::SchemaMismatch { .. }));
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
                    (CborValue::text("username"), CborValue::text("y".to_string())),
                    (CborValue::text("password"), CborValue::text("z".to_string())),
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
}
