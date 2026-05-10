//! # `coding_adventures_vault_import_export` — VLT15
//!
//! ## What this crate is
//!
//! The **import/export tier** of the Vault stack. Defines:
//!
//!   * a versioned [`PortableBundle`] format — the canonical
//!     interchange shape between vaults and external tools,
//!   * the [`Importer`] trait every adapter implements,
//!   * a [`PassthroughImporter`] reference that round-trips the
//!     portable bundle (so the test suite for upper layers has
//!     something to talk to without booting the real adapters),
//!   * an `export_to_bundle` writer and `import_from_bundle`
//!     reader that produce / consume the canonical JSON.
//!
//! Format adapters land as **sibling crates**:
//!
//!   - `vault-import-1password`  — `.1pux`
//!   - `vault-import-bitwarden`  — Bitwarden JSON
//!   - `vault-import-keepass`    — KeePassXC `.kdbx`
//!   - `vault-import-lastpass`   — LastPass CSV
//!   - `vault-import-chrome`     — Chrome / Edge CSV
//!   - `vault-import-firefox`    — Firefox CSV
//!   - `vault-import-age`        — age files
//!   - `vault-import-gpg`        — GnuPG files
//!   - `vault-import-sops`       — SOPS files
//!
//! Each adapter is its own dependency-light crate so a vault
//! using only Bitwarden import doesn't pull in the KDBX parser.
//!
//! ## Why this layer is "ceremony"
//!
//! Import/export is the one place in the Vault stack where
//! **plaintext crosses the trust boundary**:
//!
//!   * On *import*, plaintext arrives from the user (a file
//!     they exported from a competing product). The crate sees
//!     it before VLT01 sealing happens; the host is responsible
//!     for sealing immediately and then scrubbing the
//!     intermediate.
//!   * On *export*, plaintext leaves the vault. VLT01-sealed
//!     bytes are decrypted just to be repackaged into the
//!     portable format. Same scrub rule applies.
//!
//! Because of this, all import/export operations are documented
//! as **explicit user-driven ceremonies**: the host UI prompts,
//! warns, and times out. This crate gives you the byte
//! transformation; it does not decide whether to do it.
//!
//! ## Threat model
//!
//! * **Untrusted import bytes**. The reader is strict: bounded
//!   record count, bounded per-field byte length, bounded total
//!   bundle size, JSON parser is hand-rolled (no serde) and
//!   refuses unknown root keys. A malicious input cannot
//!   over-allocate by inflating any single number.
//! * **Plaintext residue**. Every secret-shaped field
//!   (`password`, `totp_seed`, `custom_fields` values) is held
//!   under [`Zeroizing`] so dropping a `PortableRecord`
//!   scrubs the bytes. Non-secret fields (`title`, `username`,
//!   `url`, `notes`) are not zeroizing — they're considered
//!   metadata.
//! * **Field-name confusion**. The portable JSON only carries a
//!   well-known set of field names (`kind`, `title`, `username`,
//!   `password`, `url`, `notes`, `totp_seed`, `tags`,
//!   `custom_fields`). Unknown keys are rejected so a peer
//!   producing a v2 bundle can't sneak data through a v1
//!   reader as opaque "extra" fields.
//! * **PortableRecordKind opacity**. The kind is an
//!   `#[non_exhaustive]` enum with `Custom(String)` for
//!   forward-compatibility; the kind string is bounded.
//! * **Round-trip determinism**. `export_to_bundle` produces
//!   deterministic JSON (sorted custom-field keys, fixed key
//!   order at every level) so a vault re-exported produces
//!   identical bytes to a previous export of the same content.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;

// === Section 1. Bounds =====================================================

/// Maximum number of records in one bundle.
pub const MAX_RECORDS: usize = 100_000;
/// Maximum bytes per individual field.
pub const MAX_FIELD_LEN: usize = 64 * 1024;
/// Maximum bytes for the entire bundle's serialized form.
pub const MAX_BUNDLE_LEN: usize = 256 * 1024 * 1024;
/// Maximum tags per record.
pub const MAX_TAGS_PER_RECORD: usize = 64;
/// Maximum custom fields per record.
pub const MAX_CUSTOM_FIELDS_PER_RECORD: usize = 64;
/// Bundle format version we produce. Readers accept this and
/// (in future) lower compatible versions.
pub const BUNDLE_VERSION: u32 = 1;

// === Section 2. Vocabulary types ===========================================

/// Versioned portable bundle. Top-level container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableBundle {
    /// Format version. Always [`BUNDLE_VERSION`] for newly
    /// produced bundles.
    pub version: u32,
    /// All records in the bundle. Order is significant for
    /// reproducible exports but not semantically meaningful;
    /// the host application sorts on display.
    pub records: Vec<PortableRecord>,
}

/// One record. Field meanings:
///
/// | Field          | Sensitive? | Notes                                       |
/// |----------------|------------|---------------------------------------------|
/// | `kind`         | no         | typed-record kind (login, note, …)          |
/// | `title`        | no         | display name                                |
/// | `username`     | no         | account / email / handle                    |
/// | `password`     | YES        | held under `Zeroizing<String>`              |
/// | `url`          | no         | site URL                                    |
/// | `notes`        | no         | freeform notes                              |
/// | `totp_seed`    | YES        | held under `Zeroizing<String>`              |
/// | `tags`         | no         | flat string tags                            |
/// | `custom_fields`| YES        | each value held under `Zeroizing<String>`   |
///
/// `Debug` is hand-rolled to redact every sensitive field.
/// `Clone` is hand-rolled because `Zeroizing<String>` does not
/// derive `Clone` — cloning produces a fresh `Zeroizing<String>`
/// per sensitive field, so each copy is independently scrubbed
/// on drop.
pub struct PortableRecord {
    /// Typed-record kind.
    pub kind: PortableRecordKind,
    /// Display title.
    pub title: String,
    /// Optional username / handle.
    pub username: Option<String>,
    /// Optional password. Held under `Zeroizing` so a stray
    /// drop scrubs the bytes.
    pub password: Option<Zeroizing<String>>,
    /// Optional URL.
    pub url: Option<String>,
    /// Optional freeform notes (treated as non-sensitive).
    pub notes: Option<String>,
    /// Optional TOTP seed (Base32 / OTPAuth URI). Sensitive.
    pub totp_seed: Option<Zeroizing<String>>,
    /// Free-form tags (case-sensitive strings).
    pub tags: Vec<String>,
    /// Caller-defined extra fields. Each value is treated as
    /// sensitive (held under `Zeroizing`). Keys are sorted on
    /// export for deterministic output.
    pub custom_fields: BTreeMap<String, Zeroizing<String>>,
}

impl Clone for PortableRecord {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            title: self.title.clone(),
            username: self.username.clone(),
            password: self.password.as_ref().map(|p| Zeroizing::new((**p).clone())),
            url: self.url.clone(),
            notes: self.notes.clone(),
            totp_seed: self
                .totp_seed
                .as_ref()
                .map(|s| Zeroizing::new((**s).clone())),
            tags: self.tags.clone(),
            custom_fields: self
                .custom_fields
                .iter()
                .map(|(k, v)| (k.clone(), Zeroizing::new((**v).clone())))
                .collect(),
        }
    }
}

impl core::fmt::Debug for PortableRecord {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PortableRecord")
            .field("kind", &self.kind)
            .field("title", &self.title)
            .field("username", &self.username)
            .field(
                "password",
                &self
                    .password
                    .as_ref()
                    .map(|p| format!("<{}-char redacted>", p.len())),
            )
            .field("url", &self.url)
            .field("notes", &self.notes)
            .field(
                "totp_seed",
                &self
                    .totp_seed
                    .as_ref()
                    .map(|s| format!("<{}-char redacted>", s.len())),
            )
            .field("tags", &self.tags)
            .field(
                "custom_fields",
                &format_args!("<{} keys, values redacted>", self.custom_fields.len()),
            )
            .finish()
    }
}

impl PartialEq for PortableRecord {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.title == other.title
            && self.username == other.username
            && self.password.as_ref().map(|p| &**p) == other.password.as_ref().map(|p| &**p)
            && self.url == other.url
            && self.notes == other.notes
            && self.totp_seed.as_ref().map(|p| &**p)
                == other.totp_seed.as_ref().map(|p| &**p)
            && self.tags == other.tags
            && self.custom_fields.len() == other.custom_fields.len()
            && self
                .custom_fields
                .iter()
                .zip(other.custom_fields.iter())
                .all(|(a, b)| a.0 == b.0 && **a.1 == **b.1)
    }
}
impl Eq for PortableRecord {}

/// Typed-record kinds. `#[non_exhaustive]` so future kinds land
/// non-breakingly. Unknown wire kinds become `Custom(label)` so
/// a v1 reader doesn't crash on a v2 producer's new type.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PortableRecordKind {
    /// Login: username + password (+ optional TOTP).
    Login,
    /// Secure note (notes-only).
    SecureNote,
    /// Payment card.
    Card,
    /// SSH private key.
    SshKey,
    /// Standalone TOTP seed.
    Totp,
    /// Catch-all for kinds the v1 wire doesn't enumerate. The
    /// inner string is a short label.
    Custom(String),
}

/// Errors produced by the reader / writer.
#[derive(Debug)]
pub enum ImportError {
    /// JSON syntax error or malformed bundle structure.
    Decode(&'static str),
    /// Caller-supplied bundle exceeds a documented bound.
    TooLarge(&'static str),
    /// Caller passed a malformed value (oversize / control
    /// chars / empty required field).
    InvalidParameter(&'static str),
    /// Bundle version is not supported.
    UnsupportedVersion(u32),
    /// Adapter-specific failure (wrapped string).
    Adapter(String),
}

impl core::fmt::Display for ImportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(s) => write!(f, "decode error: {}", s),
            Self::TooLarge(s) => write!(f, "too large: {}", s),
            Self::InvalidParameter(s) => write!(f, "invalid parameter: {}", s),
            Self::UnsupportedVersion(v) => write!(f, "unsupported bundle version: {}", v),
            Self::Adapter(s) => write!(f, "adapter error: {}", s),
        }
    }
}

impl std::error::Error for ImportError {}

// === Section 3. Importer trait ==============================================

/// What every adapter implements. `Send + Sync` so a single
/// adapter instance can be shared across threads.
pub trait Importer: Send + Sync {
    /// Stable name used by the CLI to dispatch:
    /// `vault import --from <name> <file>`.
    fn name(&self) -> &str;
    /// Convert source-format bytes into a list of
    /// `PortableRecord`s. The host then either passes the
    /// list straight to the vault (sealing each one) or
    /// re-serialises it as a portable bundle for the user.
    fn import(&self, input: &[u8]) -> Result<Vec<PortableRecord>, ImportError>;
}

/// Reference implementation: reads a portable bundle and
/// returns its records. Useful as the round-trip path the
/// other tests exercise.
pub struct PassthroughImporter;

impl Importer for PassthroughImporter {
    fn name(&self) -> &str {
        "vault-portable"
    }
    fn import(&self, input: &[u8]) -> Result<Vec<PortableRecord>, ImportError> {
        let bundle = import_from_bundle(input)?;
        Ok(bundle.records)
    }
}

// === Section 4. Validation =================================================

fn validate_field_str(s: &str, what: &'static str) -> Result<(), ImportError> {
    if s.len() > MAX_FIELD_LEN {
        return Err(ImportError::TooLarge(what));
    }
    Ok(())
}

fn validate_record(rec: &PortableRecord) -> Result<(), ImportError> {
    validate_field_str(&rec.title, "title")?;
    if rec.title.is_empty() {
        return Err(ImportError::InvalidParameter("title must not be empty"));
    }
    if let Some(u) = &rec.username {
        validate_field_str(u, "username")?;
    }
    if let Some(p) = &rec.password {
        validate_field_str(p, "password")?;
    }
    if let Some(u) = &rec.url {
        validate_field_str(u, "url")?;
    }
    if let Some(n) = &rec.notes {
        validate_field_str(n, "notes")?;
    }
    if let Some(t) = &rec.totp_seed {
        validate_field_str(t, "totp_seed")?;
    }
    if rec.tags.len() > MAX_TAGS_PER_RECORD {
        return Err(ImportError::TooLarge("tags per record"));
    }
    for tag in &rec.tags {
        validate_field_str(tag, "tag")?;
    }
    if rec.custom_fields.len() > MAX_CUSTOM_FIELDS_PER_RECORD {
        return Err(ImportError::TooLarge("custom fields per record"));
    }
    for (k, v) in &rec.custom_fields {
        validate_field_str(k, "custom field key")?;
        validate_field_str(v, "custom field value")?;
    }
    if let PortableRecordKind::Custom(label) = &rec.kind {
        validate_field_str(label, "kind label")?;
        if label.is_empty() {
            return Err(ImportError::InvalidParameter("Custom kind label must not be empty"));
        }
    }
    Ok(())
}

// === Section 5. Hand-rolled JSON encoder ===================================
//
// We avoid pulling in serde so the crate stays dep-light and
// the wire format is auditable in one file. The encoder is a
// total function over the bounded vocabulary; the decoder is
// strict (no unknown keys).

fn json_escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_record(rec: &PortableRecord, out: &mut String) {
    out.push('{');
    // kind
    out.push_str("\"kind\":");
    write_kind(&rec.kind, out);
    // title
    out.push_str(",\"title\":");
    json_escape(&rec.title, out);
    // username
    if let Some(u) = &rec.username {
        out.push_str(",\"username\":");
        json_escape(u, out);
    }
    if let Some(p) = &rec.password {
        out.push_str(",\"password\":");
        json_escape(p, out);
    }
    if let Some(u) = &rec.url {
        out.push_str(",\"url\":");
        json_escape(u, out);
    }
    if let Some(n) = &rec.notes {
        out.push_str(",\"notes\":");
        json_escape(n, out);
    }
    if let Some(t) = &rec.totp_seed {
        out.push_str(",\"totp_seed\":");
        json_escape(t, out);
    }
    if !rec.tags.is_empty() {
        out.push_str(",\"tags\":[");
        for (i, t) in rec.tags.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_escape(t, out);
        }
        out.push(']');
    }
    if !rec.custom_fields.is_empty() {
        out.push_str(",\"custom_fields\":{");
        // BTreeMap iteration order is sorted by key, which gives
        // us deterministic output.
        for (i, (k, v)) in rec.custom_fields.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_escape(k, out);
            out.push(':');
            json_escape(v, out);
        }
        out.push('}');
    }
    out.push('}');
}

fn write_kind(k: &PortableRecordKind, out: &mut String) {
    match k {
        PortableRecordKind::Login => out.push_str("\"login\""),
        PortableRecordKind::SecureNote => out.push_str("\"secure_note\""),
        PortableRecordKind::Card => out.push_str("\"card\""),
        PortableRecordKind::SshKey => out.push_str("\"ssh_key\""),
        PortableRecordKind::Totp => out.push_str("\"totp\""),
        PortableRecordKind::Custom(label) => {
            // Wire shape: {"custom":"<label>"}. Two-level so a
            // v1 reader doesn't confuse it with a known kind.
            out.push_str("{\"custom\":");
            json_escape(label, out);
            out.push('}');
        }
    }
}

/// Serialize a bundle to its canonical JSON form.
pub fn export_to_bundle(records: &[PortableRecord]) -> Result<Vec<u8>, ImportError> {
    if records.len() > MAX_RECORDS {
        return Err(ImportError::TooLarge("MAX_RECORDS"));
    }
    for rec in records {
        validate_record(rec)?;
    }
    let mut out = String::with_capacity(64 + records.len() * 128);
    out.push_str("{\"version\":");
    out.push_str(&BUNDLE_VERSION.to_string());
    out.push_str(",\"records\":[");
    for (i, rec) in records.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_record(rec, &mut out);
    }
    out.push_str("]}");
    if out.len() > MAX_BUNDLE_LEN {
        return Err(ImportError::TooLarge("MAX_BUNDLE_LEN"));
    }
    Ok(out.into_bytes())
}

// === Section 6. Hand-rolled JSON decoder ===================================
//
// The decoder is *strict*: unknown root keys, unknown record
// keys, and trailing input are all rejected. Bounded inputs at
// every step.

struct Reader<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(s: &'a [u8]) -> Self {
        Self { s, pos: 0 }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }
    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn expect_byte(&mut self, b: u8, msg: &'static str) -> Result<(), ImportError> {
        self.skip_ws();
        if self.bump() != Some(b) {
            return Err(ImportError::Decode(msg));
        }
        Ok(())
    }
    fn read_string(&mut self) -> Result<String, ImportError> {
        self.skip_ws();
        if self.bump() != Some(b'"') {
            return Err(ImportError::Decode("expected `\"`"));
        }
        // Accumulate raw bytes (with escapes already decoded
        // into their UTF-8 byte form). At the end we run a
        // single `String::from_utf8` over the whole buffer so
        // multi-byte UTF-8 codepoints are decoded correctly —
        // a previous byte-by-byte `b as char` push would
        // mis-decode `é` (UTF-8 0xC3 0xA9) as two Latin-1
        // chars `U+00C3 U+00A9` instead of one `U+00E9`,
        // silently corrupting any imported plaintext that
        // contained non-ASCII characters not pre-escaped by
        // the producer.
        let mut buf: Vec<u8> = Vec::new();
        loop {
            let b = self.bump().ok_or(ImportError::Decode("unterminated string"))?;
            if buf.len() > MAX_FIELD_LEN {
                return Err(ImportError::TooLarge("string field"));
            }
            match b {
                b'"' => {
                    return String::from_utf8(buf)
                        .map_err(|_| ImportError::Decode("invalid UTF-8 in string"))
                }
                b'\\' => {
                    let c = self.bump().ok_or(ImportError::Decode("bad escape"))?;
                    match c {
                        b'"' => buf.push(b'"'),
                        b'\\' => buf.push(b'\\'),
                        b'/' => buf.push(b'/'),
                        b'n' => buf.push(b'\n'),
                        b'r' => buf.push(b'\r'),
                        b't' => buf.push(b'\t'),
                        b'b' => buf.push(0x08),
                        b'f' => buf.push(0x0C),
                        b'u' => {
                            let mut hex = [0u8; 4];
                            for slot in hex.iter_mut() {
                                *slot = self.bump().ok_or(ImportError::Decode("short \\u escape"))?;
                            }
                            let cp = u32_from_hex4(&hex)
                                .ok_or(ImportError::Decode("bad \\u hex"))?;
                            let c = char::from_u32(cp)
                                .ok_or(ImportError::Decode("bad codepoint"))?;
                            // Encode the codepoint as UTF-8
                            // bytes into the buffer.
                            let mut tmp = [0u8; 4];
                            let s = c.encode_utf8(&mut tmp);
                            buf.extend_from_slice(s.as_bytes());
                        }
                        _ => return Err(ImportError::Decode("unknown escape")),
                    }
                }
                b => {
                    // Reject raw control characters; they MUST
                    // be \u-escaped per JSON.
                    if b < 0x20 {
                        return Err(ImportError::Decode("unescaped control char in string"));
                    }
                    // Push the raw byte; UTF-8 validation is
                    // deferred until the closing quote so
                    // multi-byte codepoints survive intact.
                    buf.push(b);
                }
            }
        }
    }
    fn read_u32(&mut self) -> Result<u32, ImportError> {
        self.skip_ws();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(ImportError::Decode("expected digits"));
        }
        // RFC 8259 forbids leading zeros on integer literals
        // (`0123` is not valid JSON, even though `u32::parse`
        // would accept it). Reject so two parsers can't disagree
        // about field values.
        if self.pos - start > 1 && self.s[start] == b'0' {
            return Err(ImportError::Decode("leading zero in number"));
        }
        let s = core::str::from_utf8(&self.s[start..self.pos])
            .map_err(|_| ImportError::Decode("non-ASCII number"))?;
        s.parse::<u32>().map_err(|_| ImportError::Decode("bad u32"))
    }
    fn at_eof(&mut self) -> bool {
        self.skip_ws();
        self.pos >= self.s.len()
    }
}

fn u32_from_hex4(bytes: &[u8; 4]) -> Option<u32> {
    let mut out = 0u32;
    for b in bytes {
        let v = match *b {
            b'0'..=b'9' => (b - b'0') as u32,
            b'a'..=b'f' => (b - b'a' + 10) as u32,
            b'A'..=b'F' => (b - b'A' + 10) as u32,
            _ => return None,
        };
        out = (out << 4) | v;
    }
    Some(out)
}

fn read_kind(r: &mut Reader<'_>) -> Result<PortableRecordKind, ImportError> {
    r.skip_ws();
    match r.peek() {
        Some(b'"') => {
            let s = r.read_string()?;
            Ok(match s.as_str() {
                "login" => PortableRecordKind::Login,
                "secure_note" => PortableRecordKind::SecureNote,
                "card" => PortableRecordKind::Card,
                "ssh_key" => PortableRecordKind::SshKey,
                "totp" => PortableRecordKind::Totp,
                _ => return Err(ImportError::Decode("unknown kind value")),
            })
        }
        Some(b'{') => {
            // {"custom":"<label>"}
            r.bump(); // consume '{'
            let key = r.read_string()?;
            if key != "custom" {
                return Err(ImportError::Decode("unknown object kind key"));
            }
            r.expect_byte(b':', "expected `:` after `custom`")?;
            let label = r.read_string()?;
            r.expect_byte(b'}', "expected `}` after Custom kind")?;
            Ok(PortableRecordKind::Custom(label))
        }
        _ => Err(ImportError::Decode("kind must be a string or `{custom:...}`")),
    }
}

fn read_record(r: &mut Reader<'_>) -> Result<PortableRecord, ImportError> {
    r.expect_byte(b'{', "expected `{` at record start")?;
    let mut kind: Option<PortableRecordKind> = None;
    let mut title: Option<String> = None;
    let mut username: Option<String> = None;
    let mut password: Option<Zeroizing<String>> = None;
    let mut url: Option<String> = None;
    let mut notes: Option<String> = None;
    let mut totp_seed: Option<Zeroizing<String>> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut custom_fields: BTreeMap<String, Zeroizing<String>> = BTreeMap::new();
    let mut first = true;
    loop {
        r.skip_ws();
        if r.peek() == Some(b'}') {
            r.bump();
            break;
        }
        if !first {
            // Strict JSON: between fields we require a comma
            // and only a comma. Permissive parsers (accepting
            // missing commas) are a "JSON parsing differences"
            // attack surface — two consumers seeing different
            // record contents.
            r.expect_byte(b',', "expected `,` or `}` between record fields")?;
            r.skip_ws();
            // Trailing comma before `}` is also forbidden.
            if r.peek() == Some(b'}') {
                return Err(ImportError::Decode("trailing comma before `}`"));
            }
        }
        first = false;
        let key = r.read_string()?;
        r.expect_byte(b':', "expected `:` after key")?;
        match key.as_str() {
            "kind" => kind = Some(read_kind(r)?),
            "title" => title = Some(r.read_string()?),
            "username" => username = Some(r.read_string()?),
            "password" => password = Some(Zeroizing::new(r.read_string()?)),
            "url" => url = Some(r.read_string()?),
            "notes" => notes = Some(r.read_string()?),
            "totp_seed" => totp_seed = Some(Zeroizing::new(r.read_string()?)),
            "tags" => {
                r.expect_byte(b'[', "expected `[` for tags array")?;
                loop {
                    r.skip_ws();
                    if r.peek() == Some(b']') {
                        r.bump();
                        break;
                    }
                    if !tags.is_empty() {
                        r.expect_byte(b',', "expected `,` between tags")?;
                        r.skip_ws();
                    }
                    if tags.len() >= MAX_TAGS_PER_RECORD {
                        return Err(ImportError::TooLarge("tags per record"));
                    }
                    tags.push(r.read_string()?);
                }
            }
            "custom_fields" => {
                r.expect_byte(b'{', "expected `{` for custom_fields")?;
                loop {
                    r.skip_ws();
                    if r.peek() == Some(b'}') {
                        r.bump();
                        break;
                    }
                    if !custom_fields.is_empty() {
                        r.expect_byte(b',', "expected `,` between custom fields")?;
                        r.skip_ws();
                    }
                    if custom_fields.len() >= MAX_CUSTOM_FIELDS_PER_RECORD {
                        return Err(ImportError::TooLarge("custom fields per record"));
                    }
                    let k = r.read_string()?;
                    r.expect_byte(b':', "expected `:` in custom field")?;
                    let v = Zeroizing::new(r.read_string()?);
                    custom_fields.insert(k, v);
                }
            }
            _ => return Err(ImportError::Decode("unknown record key")),
        }
        // Loop top will require a comma or `}` next.
    }
    let kind = kind.ok_or(ImportError::Decode("record missing `kind`"))?;
    let title = title.ok_or(ImportError::Decode("record missing `title`"))?;
    let rec = PortableRecord {
        kind,
        title,
        username,
        password,
        url,
        notes,
        totp_seed,
        tags,
        custom_fields,
    };
    validate_record(&rec)?;
    Ok(rec)
}

/// Parse a portable bundle from bytes. Strict: unknown
/// top-level keys → `Decode`; unknown record keys → `Decode`;
/// trailing bytes → `Decode`.
pub fn import_from_bundle(bytes: &[u8]) -> Result<PortableBundle, ImportError> {
    if bytes.len() > MAX_BUNDLE_LEN {
        return Err(ImportError::TooLarge("MAX_BUNDLE_LEN"));
    }
    let mut r = Reader::new(bytes);
    r.expect_byte(b'{', "expected `{` at bundle start")?;
    let mut version: Option<u32> = None;
    let mut records: Option<Vec<PortableRecord>> = None;
    let mut first = true;
    loop {
        r.skip_ws();
        if r.peek() == Some(b'}') {
            r.bump();
            break;
        }
        if !first {
            r.expect_byte(b',', "expected `,` or `}` between bundle keys")?;
            r.skip_ws();
            if r.peek() == Some(b'}') {
                return Err(ImportError::Decode("trailing comma before `}`"));
            }
        }
        first = false;
        let key = r.read_string()?;
        r.expect_byte(b':', "expected `:` after key")?;
        match key.as_str() {
            "version" => version = Some(r.read_u32()?),
            "records" => {
                r.expect_byte(b'[', "expected `[` for records")?;
                let mut recs: Vec<PortableRecord> = Vec::new();
                loop {
                    r.skip_ws();
                    if r.peek() == Some(b']') {
                        r.bump();
                        break;
                    }
                    if !recs.is_empty() {
                        r.expect_byte(b',', "expected `,` between records")?;
                        r.skip_ws();
                    }
                    if recs.len() >= MAX_RECORDS {
                        return Err(ImportError::TooLarge("MAX_RECORDS"));
                    }
                    recs.push(read_record(&mut r)?);
                }
                records = Some(recs);
            }
            _ => return Err(ImportError::Decode("unknown bundle key")),
        }
        // Loop top will require a comma or `}` next.
    }
    let version = version.ok_or(ImportError::Decode("bundle missing `version`"))?;
    if version != BUNDLE_VERSION {
        return Err(ImportError::UnsupportedVersion(version));
    }
    let records = records.ok_or(ImportError::Decode("bundle missing `records`"))?;
    if !r.at_eof() {
        return Err(ImportError::Decode("trailing bytes after bundle"));
    }
    Ok(PortableBundle { version, records })
}

// === Section 7. Tests ======================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn login(title: &str, user: &str, pw: &str) -> PortableRecord {
        PortableRecord {
            kind: PortableRecordKind::Login,
            title: title.into(),
            username: Some(user.into()),
            password: Some(Zeroizing::new(pw.into())),
            url: Some("https://example.com".into()),
            notes: None,
            totp_seed: None,
            tags: vec!["work".into()],
            custom_fields: BTreeMap::new(),
        }
    }

    // --- Round-trip ---

    #[test]
    fn roundtrip_one_login() {
        let r = login("GitHub", "alice", "hunter2");
        let bytes = export_to_bundle(&[r.clone()]).unwrap();
        let bundle = import_from_bundle(&bytes).unwrap();
        assert_eq!(bundle.version, BUNDLE_VERSION);
        assert_eq!(bundle.records.len(), 1);
        assert_eq!(bundle.records[0], r);
    }

    #[test]
    fn roundtrip_all_kinds() {
        let mut card_fields = BTreeMap::new();
        card_fields.insert("number".to_string(), Zeroizing::new("4111-1111-1111-1111".to_string()));
        card_fields.insert("cvv".to_string(), Zeroizing::new("123".to_string()));
        let recs = vec![
            login("github", "alice", "hunter2"),
            PortableRecord {
                kind: PortableRecordKind::SecureNote,
                title: "wifi password".into(),
                username: None,
                password: None,
                url: None,
                notes: Some("LongAndStrong-2024".into()),
                totp_seed: None,
                tags: vec![],
                custom_fields: BTreeMap::new(),
            },
            PortableRecord {
                kind: PortableRecordKind::Card,
                title: "amex".into(),
                username: None,
                password: None,
                url: None,
                notes: None,
                totp_seed: None,
                tags: vec![],
                custom_fields: card_fields,
            },
            PortableRecord {
                kind: PortableRecordKind::Totp,
                title: "AWS root".into(),
                username: None,
                password: None,
                url: None,
                notes: None,
                totp_seed: Some(Zeroizing::new("JBSWY3DPEHPK3PXP".into())),
                tags: vec![],
                custom_fields: BTreeMap::new(),
            },
            PortableRecord {
                kind: PortableRecordKind::Custom("workflow".into()),
                title: "deploy steps".into(),
                username: None,
                password: None,
                url: None,
                notes: Some("ssh ; deploy ; bless".into()),
                totp_seed: None,
                tags: vec!["ops".into()],
                custom_fields: BTreeMap::new(),
            },
        ];
        let bytes = export_to_bundle(&recs).unwrap();
        let bundle = import_from_bundle(&bytes).unwrap();
        assert_eq!(bundle.records, recs);
    }

    #[test]
    fn deterministic_export() {
        // Same input → identical output bytes (BTreeMap iteration
        // and the writer's fixed key order make this true).
        let r = login("github", "alice", "hunter2");
        let bytes_a = export_to_bundle(&[r.clone()]).unwrap();
        let bytes_b = export_to_bundle(&[r]).unwrap();
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn passthrough_importer_works() {
        let r = login("github", "alice", "hunter2");
        let bytes = export_to_bundle(&[r.clone()]).unwrap();
        let imp = PassthroughImporter;
        assert_eq!(imp.name(), "vault-portable");
        let recs = imp.import(&bytes).unwrap();
        assert_eq!(recs, vec![r]);
    }

    // --- Validation ---

    #[test]
    fn reject_empty_title() {
        let mut r = login("github", "alice", "hunter2");
        r.title.clear();
        let res = export_to_bundle(&[r]);
        assert!(matches!(res, Err(ImportError::InvalidParameter(_))));
    }

    #[test]
    fn reject_oversize_field() {
        let mut r = login("github", "alice", "hunter2");
        r.notes = Some("x".repeat(MAX_FIELD_LEN + 1));
        let res = export_to_bundle(&[r]);
        assert!(matches!(res, Err(ImportError::TooLarge(_))));
    }

    #[test]
    fn reject_too_many_records() {
        let r = login("github", "alice", "hunter2");
        let recs = vec![r; MAX_RECORDS + 1];
        let res = export_to_bundle(&recs);
        assert!(matches!(res, Err(ImportError::TooLarge(_))));
    }

    #[test]
    fn reject_too_many_tags() {
        let mut r = login("github", "alice", "hunter2");
        r.tags = (0..(MAX_TAGS_PER_RECORD + 1)).map(|i| format!("t{}", i)).collect();
        let res = export_to_bundle(&[r]);
        assert!(matches!(res, Err(ImportError::TooLarge(_))));
    }

    #[test]
    fn reject_too_many_custom_fields() {
        let mut r = login("github", "alice", "hunter2");
        r.custom_fields = (0..(MAX_CUSTOM_FIELDS_PER_RECORD + 1))
            .map(|i| (format!("k{}", i), Zeroizing::new("v".to_string())))
            .collect();
        let res = export_to_bundle(&[r]);
        assert!(matches!(res, Err(ImportError::TooLarge(_))));
    }

    #[test]
    fn reject_empty_custom_kind_label() {
        let r = PortableRecord {
            kind: PortableRecordKind::Custom("".into()),
            title: "x".into(),
            username: None,
            password: None,
            url: None,
            notes: None,
            totp_seed: None,
            tags: vec![],
            custom_fields: BTreeMap::new(),
        };
        let res = export_to_bundle(&[r]);
        assert!(matches!(res, Err(ImportError::InvalidParameter(_))));
    }

    // --- Decoder strictness ---

    #[test]
    fn reject_unknown_root_key() {
        let bytes = b"{\"version\":1,\"records\":[],\"zomg\":42}";
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_unknown_record_key() {
        let bytes = b"{\"version\":1,\"records\":[{\"kind\":\"login\",\"title\":\"x\",\"frobnicate\":\"y\"}]}";
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_unsupported_version() {
        let bytes = b"{\"version\":999,\"records\":[]}";
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::UnsupportedVersion(999))));
    }

    #[test]
    fn reject_trailing_bytes() {
        let r = login("github", "alice", "hunter2");
        let mut bytes = export_to_bundle(&[r]).unwrap();
        bytes.extend_from_slice(b"\n//garbage");
        let res = import_from_bundle(&bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_unknown_kind_value() {
        let bytes = b"{\"version\":1,\"records\":[{\"kind\":\"frobnicate\",\"title\":\"x\"}]}";
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_oversize_bundle_bytes() {
        // Build a payload that exceeds MAX_BUNDLE_LEN by being
        // a giant string field (cheap to construct via the
        // import path).
        let big = vec![0u8; MAX_BUNDLE_LEN + 1];
        let res = import_from_bundle(&big);
        assert!(matches!(res, Err(ImportError::TooLarge(_))));
    }

    #[test]
    fn reject_unescaped_control_char() {
        let bytes = b"{\"version\":1,\"records\":[{\"kind\":\"login\",\"title\":\"a\nb\"}]}";
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn round_trip_preserves_escaped_characters() {
        let r = PortableRecord {
            kind: PortableRecordKind::Login,
            title: "weird \"quoted\" \n title".into(),
            username: Some("alice\\bob".into()),
            password: Some(Zeroizing::new("p\"a's".into())),
            url: None,
            notes: Some("line1\nline2".into()),
            totp_seed: None,
            tags: vec![],
            custom_fields: BTreeMap::new(),
        };
        let bytes = export_to_bundle(&[r.clone()]).unwrap();
        let bundle = import_from_bundle(&bytes).unwrap();
        assert_eq!(bundle.records[0], r);
    }

    #[test]
    fn round_trip_preserves_u2028_u2029() {
        let r = PortableRecord {
            kind: PortableRecordKind::Login,
            title: "a\u{2028}b\u{2029}c".into(),
            username: None,
            password: None,
            url: None,
            notes: None,
            totp_seed: None,
            tags: vec![],
            custom_fields: BTreeMap::new(),
        };
        let bytes = export_to_bundle(&[r.clone()]).unwrap();
        // U+2028 / U+2029 escaped on the wire (so JS embedding
        // is safe) but read back to original chars.
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.contains("\\u2028"));
        let bundle = import_from_bundle(&bytes).unwrap();
        assert_eq!(bundle.records[0], r);
    }

    // --- Redaction ---

    #[test]
    fn record_debug_redacts_password_and_totp() {
        let mut fields = BTreeMap::new();
        fields.insert("api_key".to_string(), Zeroizing::new("super-secret-key".to_string()));
        let r = PortableRecord {
            kind: PortableRecordKind::Login,
            title: "github".into(),
            username: Some("alice".into()),
            password: Some(Zeroizing::new("hunter2".into())),
            url: None,
            notes: None,
            totp_seed: Some(Zeroizing::new("JBSWY3DPEHPK3PXP".into())),
            tags: vec![],
            custom_fields: fields,
        };
        let s = format!("{:?}", r);
        assert!(!s.contains("hunter2"));
        assert!(!s.contains("JBSWY3DPEHPK3PXP"));
        assert!(!s.contains("super-secret-key"));
        assert!(s.contains("redacted"));
    }

    // --- Send + Sync ---

    #[test]
    fn importer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PassthroughImporter>();
        assert_send_sync::<Box<dyn Importer>>();
    }

    // --- UTF-8 / strictness ---

    #[test]
    fn round_trip_preserves_unicode_passwords() {
        // Regression test for the silent UTF-8 corruption bug:
        // a previous byte-by-byte `b as char` push would split
        // `é` (UTF-8 0xC3 0xA9) into two Latin-1 chars.
        let r = PortableRecord {
            kind: PortableRecordKind::Login,
            title: "Bücher / 日本語 / Café 🦀".into(),
            username: Some("alice@éxample.com".into()),
            password: Some(Zeroizing::new("naïve-Päßwôrd-日本-🔑".into())),
            url: None,
            notes: Some("ñoño / αβγ".into()),
            totp_seed: None,
            tags: vec!["密码".into(), "café".into()],
            custom_fields: BTreeMap::new(),
        };
        let bytes = export_to_bundle(&[r.clone()]).unwrap();
        let bundle = import_from_bundle(&bytes).unwrap();
        assert_eq!(bundle.records[0], r);
    }

    #[test]
    fn reject_missing_comma_between_record_fields() {
        let bytes = br#"{"version":1,"records":[{"kind":"login" "title":"x"}]}"#;
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_missing_comma_between_bundle_keys() {
        let bytes = br#"{"version":1 "records":[]}"#;
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_trailing_comma_in_record() {
        let bytes = br#"{"version":1,"records":[{"kind":"login","title":"x",}]}"#;
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_trailing_comma_in_bundle() {
        let bytes = br#"{"version":1,"records":[],}"#;
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_leading_zero_in_version() {
        let bytes = br#"{"version":01,"records":[]}"#;
        let res = import_from_bundle(bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    #[test]
    fn reject_invalid_utf8_in_string() {
        // Build raw bytes with a malformed UTF-8 sequence
        // (continuation byte without a leading byte).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(br#"{"version":1,"records":[{"kind":"login","title":""#);
        bytes.push(0xC3); // start of 2-byte UTF-8
        // missing continuation byte; instead close the string
        bytes.extend_from_slice(br#""}]}"#);
        let res = import_from_bundle(&bytes);
        assert!(matches!(res, Err(ImportError::Decode(_))));
    }

    // --- Empty bundle ---

    #[test]
    fn empty_bundle_round_trips() {
        let bytes = export_to_bundle(&[]).unwrap();
        let bundle = import_from_bundle(&bytes).unwrap();
        assert!(bundle.records.is_empty());
    }
}
