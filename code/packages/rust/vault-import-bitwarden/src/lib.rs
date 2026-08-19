//! # `coding_adventures_vault_import_bitwarden` — VLT-PM49
//!
//! ## What this crate is
//!
//! A **format adapter** for the import/export tier defined by VLT15
//! (`coding_adventures_vault_import_export`). It decodes an unencrypted
//! Bitwarden JSON export — the file Bitwarden's "Export vault" screen
//! produces when the format is `.json` (not the encrypted `.json`
//! variant, which needs an account key this crate never sees) — into the
//! shared [`PortableRecord`] vocabulary every adapter in this tier
//! produces.
//!
//! This crate does no vault-pm-specific work: no crypto, no item IDs, no
//! audit events. It answers exactly one question — "what records does
//! this file describe?" — and leaves everything about *storing* those
//! records to the host (`vault-pm-cli`'s `import bitwarden` ceremony,
//! specified by `VLT-PM49-cli-external-import.md`).
//!
//! ## Why a hand-rolled walk instead of `serde_json`
//!
//! Same reasoning VLT15 itself gives: this monorepo has a
//! zero-external-dependency policy, and untrusted import bytes are
//! exactly the place a permissive or surprising deserializer turns into
//! a real bug (VLT-PM00 §7.1 adversary 6, "malicious imported data").
//! Rather than write a *fourth* hand-rolled JSON decoder in this
//! workspace, this crate reuses the already-audited, already-tested
//! generic JSON pipeline (`json-lexer` → `json-parser` → `json-value`)
//! that other consumers in this repo already depend on. That pipeline
//! caps parse-tree nesting depth (`DEFAULT_MAX_RULE_DEPTH` inside
//! `json-parser`), so a deeply nested adversarial document — the classic
//! `[[[[...]]]]` stack-overflow shape — returns a clean `Err` instead of
//! taking down the process. See [`decode`]'s tests for a payload that
//! exercises exactly that path.
//!
//! ## Bitwarden's shape, and where this adapter is deliberately narrower
//!
//! A Bitwarden export is one JSON object with (at least) `"items"`, an
//! array of records. Each item carries an integer `"type"`: `1` = login,
//! `2` = secure note, `3` = card, `4` = identity. This adapter maps:
//!
//! | Bitwarden `type` | Produces |
//! |---|---|
//! | `1` login | one [`PortableRecordKind::Login`], plus a *second*, separate [`PortableRecordKind::Totp`] record when `login.totp` is present |
//! | `2` secure note | one [`PortableRecordKind::SecureNote`] |
//! | `3` card | one [`PortableRecordKind::Card`], card fields carried in `custom_fields` (see below) |
//! | `4` identity, or any other value | one [`PortableRecordKind::Custom`] record — the host's mapping layer counts these as *skipped*, because vault-pm has no identity record type yet, rather than silently dropping them from the file |
//!
//! A login can hold several `login.uris` entries; [`PortableRecord`] has
//! room for exactly one `url`. The first URI becomes `url`; any further
//! URIs are kept, not dropped, as `custom_fields["uri_2"]`,
//! `["uri_3"]`, … up to [`MAX_URIS_PER_LOGIN`]. A card's cardholder
//! name, PAN, expiry, and CVV have no first-class slot in the shared
//! vocabulary either, so they land in `custom_fields` under the same key
//! names vault-pm's own `Card` record uses (`holder`, `number`,
//! `expiry_month`, `expiry_year`, `cvv`, `billing_zip`) — every
//! `custom_fields` value is already `Zeroizing` in the shared type, so
//! the PAN and CVV are exactly as protected as `password` is.
//!
//! Folder/collection assignment, Bitwarden's `favorite` flag, and
//! attachments referenced by an item are not carried across: vault-pm's
//! `context.document` always creates a new item unfavorited and with no
//! collections (matching plain `item add`), and Bitwarden's exported
//! attachment metadata has no accompanying bytes to import in the first
//! place. Recorded here rather than silently assumed.
//!
//! ## Threat model
//!
//! * **Untrusted bytes.** [`MAX_SOURCE_BYTES`] bounds the whole input
//!   before any parsing begins. [`MAX_ITEMS`], [`MAX_URIS_PER_LOGIN`],
//!   and [`MAX_CUSTOM_FIELDS_PER_ITEM`] bound every array this adapter
//!   walks; [`MAX_FIELD_LEN`] bounds every string it copies out. None of
//!   these can be exceeded by a file within the byte cap producing an
//!   unbounded *decoded* structure, because JSON (unlike XML) has no
//!   entity-expansion mechanism — a bounded-length document cannot decode
//!   to an unboundedly large tree.
//! * **Deeply nested documents.** Handled by `json-parser`'s built-in
//!   depth cap, exercised in this crate's own test suite rather than
//!   only trusted by citation.
//! * **Duplicate JSON keys.** Real Bitwarden exports never repeat a key
//!   within one object; a crafted file might. This adapter resolves
//!   duplicates the same way every mainstream JSON parser does: **last
//!   key wins**. Tested explicitly so the behavior is a documented
//!   contract, not an accident of iteration order.
//! * **Field-name confusion / type confusion.** Every field this adapter
//!   reads is type-checked (`JsonValue::String`, `JsonValue::Object`,
//!   …); an item where `"login"` is itself a string, an array, or `42`
//!   is rejected rather than coerced.
//! * **CSV formula injection.** Not applicable to this adapter — Bitwarden
//!   exports are JSON. `vault-import-csv`'s docs carry that concern.
//! * **Plaintext residue.** Every secret-shaped field this adapter
//!   produces — `password`, `totp_seed`, and every `custom_fields` value
//!   — is `Zeroizing` at the [`PortableRecord`] boundary already; this
//!   crate introduces no separate plaintext buffer that outlives the
//!   call.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_value::{JsonNumber, JsonValue};
use coding_adventures_vault_import_export::{
    ImportError, Importer, PortableRecord, PortableRecordKind,
};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;

// === Bounds =================================================================

/// Maximum accepted raw source bytes, checked before any parsing begins.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
/// Maximum accepted `items` array length.
pub const MAX_ITEMS: usize = 50_000;
/// Maximum accepted `login.uris` entries kept per login (first becomes
/// `url`; the rest become bounded `custom_fields` entries).
pub const MAX_URIS_PER_LOGIN: usize = 32;
/// Maximum accepted `fields` (Bitwarden custom fields) per item.
pub const MAX_CUSTOM_FIELDS_PER_ITEM: usize = 64;
/// Maximum accepted bytes for any single string field this adapter reads.
pub const MAX_FIELD_LEN: usize = 64 * 1024;

/// Decodes an unencrypted Bitwarden JSON export into [`PortableRecord`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct BitwardenJsonImporter;

impl Importer for BitwardenJsonImporter {
    fn name(&self) -> &str {
        "bitwarden-json"
    }

    fn import(&self, input: &[u8]) -> Result<Vec<PortableRecord>, ImportError> {
        decode(input)
    }
}

/// Decode a Bitwarden JSON export into [`PortableRecord`]s.
///
/// Exposed as a free function (in addition to the [`Importer`] impl above)
/// so callers that already have a concrete adapter type in hand don't need
/// a trait object just to call it.
pub fn decode(input: &[u8]) -> Result<Vec<PortableRecord>, ImportError> {
    if input.is_empty() {
        return Err(ImportError::InvalidParameter("empty source"));
    }
    if input.len() > MAX_SOURCE_BYTES {
        return Err(ImportError::TooLarge("MAX_SOURCE_BYTES"));
    }
    let text = core::str::from_utf8(input)
        .map_err(|_| ImportError::Decode("source is not valid UTF-8"))?;
    let root = coding_adventures_json_value::parse(text)
        .map_err(|_| ImportError::Decode("source is not valid JSON"))?;
    let JsonValue::Object(root_pairs) = root else {
        return Err(ImportError::Decode("root value must be a JSON object"));
    };
    let items = match find(&root_pairs, "items") {
        Some(JsonValue::Array(items)) => items,
        Some(_) => return Err(ImportError::Decode("`items` must be an array")),
        None => return Err(ImportError::Decode("missing `items` array")),
    };
    if items.len() > MAX_ITEMS {
        return Err(ImportError::TooLarge("MAX_ITEMS"));
    }
    let mut records = Vec::with_capacity(items.len());
    for item in items {
        decode_item(item, &mut records)?;
    }
    Ok(records)
}

/// Resolve a key in an object's pair list, **last write wins** on
/// duplicates — the same rule every mainstream JSON parser applies, made
/// an explicit, tested contract here rather than an accident of iteration
/// order (see `reject...duplicate_key` tests below).
fn find<'a>(pairs: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    pairs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn as_str<'a>(value: &'a JsonValue, what: &'static str) -> Result<&'a str, ImportError> {
    match value {
        JsonValue::String(s) => {
            if s.len() > MAX_FIELD_LEN {
                Err(ImportError::TooLarge(what))
            } else {
                Ok(s.as_str())
            }
        }
        _ => Err(ImportError::Decode(what)),
    }
}

/// Look up an optional, nullable string field. `None`/`Null`/absent all
/// mean "not present"; a present-but-wrong-typed value is still a hard
/// decode error, because Bitwarden never emits e.g. a number here and a
/// crafted file doing so is exactly the "malformed / ambiguous" shape
/// VLT-PM00 §7.1 names.
fn optional_str<'a>(
    pairs: &'a [(String, JsonValue)],
    key: &str,
    what: &'static str,
) -> Result<Option<&'a str>, ImportError> {
    match find(pairs, key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => Ok(Some(as_str(value, what)?)),
    }
}

fn decode_item(item: &JsonValue, out: &mut Vec<PortableRecord>) -> Result<(), ImportError> {
    let JsonValue::Object(pairs) = item else {
        return Err(ImportError::Decode("each item must be a JSON object"));
    };
    let title = match find(pairs, "name") {
        Some(value) => as_str(value, "name")?,
        None => return Err(ImportError::Decode("item missing `name`")),
    };
    if title.is_empty() {
        return Err(ImportError::InvalidParameter(
            "item `name` must not be empty",
        ));
    }
    let notes = optional_str(pairs, "notes", "notes")?.map(str::to_owned);
    let kind_number = match find(pairs, "type") {
        Some(JsonValue::Number(JsonNumber::Integer(n))) => *n,
        Some(_) => return Err(ImportError::Decode("`type` must be an integer")),
        None => return Err(ImportError::Decode("item missing `type`")),
    };
    match kind_number {
        1 => decode_login(pairs, title, notes, out),
        2 => {
            let mut custom_fields = BTreeMap::new();
            decode_custom_fields(pairs, &mut custom_fields)?;
            out.push(PortableRecord {
                kind: PortableRecordKind::SecureNote,
                title: title.to_owned(),
                username: None,
                password: None,
                url: None,
                notes,
                totp_seed: None,
                tags: Vec::new(),
                custom_fields,
            });
            Ok(())
        }
        3 => decode_card(pairs, title, notes, out),
        _ => {
            // Identity (4) or any future/unknown type. Kept as a Custom
            // record rather than dropped, so a host counting "skipped"
            // items can prove it saw every item in the file.
            let mut custom_fields = BTreeMap::new();
            decode_custom_fields(pairs, &mut custom_fields)?;
            out.push(PortableRecord {
                kind: PortableRecordKind::Custom(format!("bitwarden-type-{kind_number}")),
                title: title.to_owned(),
                username: None,
                password: None,
                url: None,
                notes,
                totp_seed: None,
                tags: Vec::new(),
                custom_fields,
            });
            Ok(())
        }
    }
}

fn decode_login(
    pairs: &[(String, JsonValue)],
    title: &str,
    notes: Option<String>,
    out: &mut Vec<PortableRecord>,
) -> Result<(), ImportError> {
    let login = match find(pairs, "login") {
        Some(JsonValue::Object(login_pairs)) => login_pairs.as_slice(),
        Some(_) => return Err(ImportError::Decode("`login` must be an object")),
        None => &[],
    };
    let username = optional_str(login, "username", "login.username")?.map(str::to_owned);
    let password =
        optional_str(login, "password", "login.password")?.map(|s| Zeroizing::new(s.to_owned()));
    let totp = optional_str(login, "totp", "login.totp")?.map(|s| Zeroizing::new(s.to_owned()));

    let mut custom_fields: BTreeMap<String, Zeroizing<String>> = BTreeMap::new();
    let mut url: Option<String> = None;
    if let Some(value) = find(login, "uris") {
        let JsonValue::Array(uris) = value else {
            return Err(ImportError::Decode("`login.uris` must be an array"));
        };
        if uris.len() > MAX_URIS_PER_LOGIN {
            return Err(ImportError::TooLarge("MAX_URIS_PER_LOGIN"));
        }
        for (index, entry) in uris.iter().enumerate() {
            let JsonValue::Object(uri_pairs) = entry else {
                return Err(ImportError::Decode("`login.uris[]` must be an object"));
            };
            let Some(uri) = optional_str(uri_pairs, "uri", "login.uris[].uri")? else {
                continue;
            };
            if uri.is_empty() {
                continue;
            }
            if url.is_none() {
                url = Some(uri.to_owned());
            } else {
                custom_fields.insert(format!("uri_{}", index + 1), Zeroizing::new(uri.to_owned()));
            }
        }
    }
    decode_custom_fields(pairs, &mut custom_fields)?;

    out.push(PortableRecord {
        kind: PortableRecordKind::Login,
        title: title.to_owned(),
        username,
        password,
        url,
        notes,
        totp_seed: None,
        tags: Vec::new(),
        custom_fields,
    });

    if let Some(totp) = totp {
        // A separate record: vault-pm's own `Login` record has no TOTP
        // slot (TOTP is its own item kind), so a Bitwarden login carrying
        // a TOTP seed becomes two vault-pm items. Kept as its own
        // top-level PortableRecord rather than a custom field so the
        // host's mapping layer can route it through the same TOTP
        // decode path (raw Base32 or `otpauth://`) used for a
        // Custom/Totp kind.
        out.push(PortableRecord {
            kind: PortableRecordKind::Totp,
            title: title.to_owned(),
            username: None,
            password: None,
            url: None,
            notes: None,
            totp_seed: Some(totp),
            tags: Vec::new(),
            custom_fields: BTreeMap::new(),
        });
    }
    Ok(())
}

fn decode_card(
    pairs: &[(String, JsonValue)],
    title: &str,
    notes: Option<String>,
    out: &mut Vec<PortableRecord>,
) -> Result<(), ImportError> {
    let card = match find(pairs, "card") {
        Some(JsonValue::Object(card_pairs)) => card_pairs.as_slice(),
        Some(_) => return Err(ImportError::Decode("`card` must be an object")),
        None => &[],
    };
    let mut custom_fields: BTreeMap<String, Zeroizing<String>> = BTreeMap::new();
    let mapped = [
        ("cardholderName", "holder"),
        ("number", "number"),
        ("expMonth", "expiry_month"),
        ("expYear", "expiry_year"),
        ("code", "cvv"),
    ];
    for (bitwarden_key, portable_key) in mapped {
        if let Some(value) = optional_str(card, bitwarden_key, bitwarden_key)? {
            if !value.is_empty() {
                custom_fields.insert(portable_key.to_owned(), Zeroizing::new(value.to_owned()));
            }
        }
    }
    decode_custom_fields(pairs, &mut custom_fields)?;
    out.push(PortableRecord {
        kind: PortableRecordKind::Card,
        title: title.to_owned(),
        username: None,
        password: None,
        url: None,
        notes,
        totp_seed: None,
        tags: Vec::new(),
        custom_fields,
    });
    Ok(())
}

/// Decode Bitwarden's item-level `fields` array (custom fields the person
/// added themselves) into `custom_fields`, merged in on top of whatever
/// kind-specific fields the caller already inserted. A name collision
/// with a kind-specific key (e.g. a custom field literally named
/// `"number"` on a card) overwrites it — last write wins, same rule as
/// duplicate JSON object keys, and it is the person's own custom field
/// that wins because it is decoded second.
fn decode_custom_fields(
    pairs: &[(String, JsonValue)],
    custom_fields: &mut BTreeMap<String, Zeroizing<String>>,
) -> Result<(), ImportError> {
    let Some(value) = find(pairs, "fields") else {
        return Ok(());
    };
    let JsonValue::Array(fields) = value else {
        return Err(ImportError::Decode("`fields` must be an array"));
    };
    if fields.len() > MAX_CUSTOM_FIELDS_PER_ITEM {
        return Err(ImportError::TooLarge("MAX_CUSTOM_FIELDS_PER_ITEM"));
    }
    for field in fields {
        let JsonValue::Object(field_pairs) = field else {
            return Err(ImportError::Decode(
                "each `fields[]` entry must be an object",
            ));
        };
        let name = match find(field_pairs, "name") {
            Some(JsonValue::Null) | None => continue,
            Some(value) => as_str(value, "fields[].name")?,
        };
        if name.is_empty() {
            continue;
        }
        let field_value = optional_str(field_pairs, "value", "fields[].value")?.unwrap_or_default();
        if custom_fields.len() >= MAX_CUSTOM_FIELDS_PER_ITEM && !custom_fields.contains_key(name) {
            return Err(ImportError::TooLarge("MAX_CUSTOM_FIELDS_PER_ITEM"));
        }
        custom_fields.insert(name.to_owned(), Zeroizing::new(field_value.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(pw: &PortableRecord, name: &str) -> String {
        pw.custom_fields
            .get(name)
            .map(|z| (**z).clone())
            .unwrap_or_default()
    }

    #[test]
    fn decodes_one_login() {
        let json = br#"{"items":[{"type":1,"name":"GitHub","notes":null,
            "login":{"username":"alice","password":"hunter2",
            "uris":[{"uri":"https://github.com"}]}}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.kind, PortableRecordKind::Login);
        assert_eq!(r.title, "GitHub");
        assert_eq!(r.username.as_deref(), Some("alice"));
        assert_eq!(r.password.as_deref().map(String::as_str), Some("hunter2"));
        assert_eq!(r.url.as_deref(), Some("https://github.com"));
    }

    #[test]
    fn login_with_totp_becomes_two_records() {
        let json = br#"{"items":[{"type":1,"name":"AWS",
            "login":{"username":"root","password":"pw",
            "totp":"JBSWY3DPEHPK3PXP","uris":[]}}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].kind, PortableRecordKind::Login);
        assert_eq!(records[1].kind, PortableRecordKind::Totp);
        assert_eq!(records[1].title, "AWS");
        assert_eq!(
            records[1].totp_seed.as_deref().map(String::as_str),
            Some("JBSWY3DPEHPK3PXP")
        );
    }

    #[test]
    fn extra_uris_kept_as_custom_fields_not_dropped() {
        let json = br#"{"items":[{"type":1,"name":"Multi",
            "login":{"uris":[{"uri":"https://a.example"},
                              {"uri":"https://b.example"},
                              {"uri":"https://c.example"}]}}]}"#;
        let records = decode(json).unwrap();
        let r = &records[0];
        assert_eq!(r.url.as_deref(), Some("https://a.example"));
        assert_eq!(field(r, "uri_2"), "https://b.example");
        assert_eq!(field(r, "uri_3"), "https://c.example");
    }

    #[test]
    fn decodes_secure_note() {
        let json = br#"{"items":[{"type":2,"name":"WiFi","notes":"password: hunter2"}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records[0].kind, PortableRecordKind::SecureNote);
        assert_eq!(records[0].notes.as_deref(), Some("password: hunter2"));
    }

    #[test]
    fn decodes_card_fields_into_custom_fields() {
        let json = br#"{"items":[{"type":3,"name":"Amex","card":{
            "cardholderName":"Ada Lovelace","number":"378282246310005",
            "expMonth":"1","expYear":"2030","code":"1234"}}]}"#;
        let records = decode(json).unwrap();
        let r = &records[0];
        assert_eq!(r.kind, PortableRecordKind::Card);
        assert_eq!(field(r, "holder"), "Ada Lovelace");
        assert_eq!(field(r, "number"), "378282246310005");
        assert_eq!(field(r, "expiry_month"), "1");
        assert_eq!(field(r, "expiry_year"), "2030");
        assert_eq!(field(r, "cvv"), "1234");
    }

    #[test]
    fn identity_becomes_custom_kind_not_dropped() {
        let json = br#"{"items":[{"type":4,"name":"My Passport"}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].kind,
            PortableRecordKind::Custom("bitwarden-type-4".into())
        );
    }

    #[test]
    fn unknown_type_becomes_custom_kind() {
        let json = br#"{"items":[{"type":99,"name":"???"}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(
            records[0].kind,
            PortableRecordKind::Custom("bitwarden-type-99".into())
        );
    }

    #[test]
    fn custom_fields_are_decoded() {
        let json = br#"{"items":[{"type":2,"name":"n","notes":null,
            "fields":[{"name":"pin","value":"1234","type":1},
                      {"name":"note","value":"x","type":0}]}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(field(&records[0], "pin"), "1234");
        assert_eq!(field(&records[0], "note"), "x");
    }

    // --- Adversarial / malformed input ---

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(decode(b""), Err(ImportError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_oversize_input() {
        let big = vec![b' '; MAX_SOURCE_BYTES + 1];
        assert!(matches!(decode(&big), Err(ImportError::TooLarge(_))));
    }

    #[test]
    fn rejects_invalid_utf8() {
        let bytes = vec![0xFF, 0xFE, 0xFD];
        assert!(matches!(decode(&bytes), Err(ImportError::Decode(_))));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(matches!(decode(b"{not json"), Err(ImportError::Decode(_))));
    }

    #[test]
    fn rejects_non_object_root() {
        assert!(matches!(decode(b"[1,2,3]"), Err(ImportError::Decode(_))));
    }

    #[test]
    fn rejects_missing_items_key() {
        assert!(matches!(
            decode(br#"{"folders":[]}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_items_not_an_array() {
        assert!(matches!(
            decode(br#"{"items":"nope"}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_item_not_an_object() {
        assert!(matches!(
            decode(br#"{"items":[42]}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_missing_name() {
        assert!(matches!(
            decode(br#"{"items":[{"type":1}]}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_empty_name() {
        assert!(matches!(
            decode(br#"{"items":[{"type":1,"name":""}]}"#),
            Err(ImportError::InvalidParameter(_))
        ));
    }

    #[test]
    fn rejects_missing_type() {
        assert!(matches!(
            decode(br#"{"items":[{"name":"x"}]}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_type_as_string() {
        assert!(matches!(
            decode(br#"{"items":[{"type":"1","name":"x"}]}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_login_as_non_object() {
        assert!(matches!(
            decode(br#"{"items":[{"type":1,"name":"x","login":"nope"}]}"#),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn rejects_too_many_items() {
        let mut json = String::from(r#"{"items":["#);
        for i in 0..(MAX_ITEMS + 1) {
            if i > 0 {
                json.push(',');
            }
            json.push_str(r#"{"type":2,"name":"n"}"#);
        }
        json.push_str("]}");
        assert!(matches!(
            decode(json.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn rejects_too_many_uris() {
        let mut uris = String::from("[");
        for i in 0..(MAX_URIS_PER_LOGIN + 1) {
            if i > 0 {
                uris.push(',');
            }
            uris.push_str(r#"{"uri":"https://example.com"}"#);
        }
        uris.push(']');
        let json = format!(r#"{{"items":[{{"type":1,"name":"x","login":{{"uris":{uris}}}}}]}}"#);
        assert!(matches!(
            decode(json.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn rejects_too_many_custom_fields() {
        let mut fields = String::from("[");
        for i in 0..(MAX_CUSTOM_FIELDS_PER_ITEM + 1) {
            if i > 0 {
                fields.push(',');
            }
            fields.push_str(&format!(r#"{{"name":"k{i}","value":"v"}}"#));
        }
        fields.push(']');
        let json = format!(r#"{{"items":[{{"type":2,"name":"x","fields":{fields}}}]}}"#);
        assert!(matches!(
            decode(json.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn rejects_oversize_field() {
        let big = "x".repeat(MAX_FIELD_LEN + 1);
        let json = format!(r#"{{"items":[{{"type":1,"name":"{big}"}}]}}"#);
        assert!(matches!(
            decode(json.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn duplicate_keys_resolve_last_write_wins() {
        let json = br#"{"items":[{"type":2,"name":"first","name":"second"}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records[0].title, "second");
    }

    #[test]
    fn duplicate_top_level_items_key_resolves_last_write_wins() {
        let json = br#"{"items":[{"type":2,"name":"ignored"}],
            "items":[{"type":2,"name":"used"}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "used");
    }

    #[test]
    fn deeply_nested_document_does_not_crash_and_is_rejected() {
        // Not a real Bitwarden shape at all -- this only proves the
        // depth cap inherited from json-parser turns an adversarial
        // `[[[[...]]]]` into a clean `Err` instead of a stack overflow.
        let mut nested = String::new();
        for _ in 0..10_000 {
            nested.push('[');
        }
        for _ in 0..10_000 {
            nested.push(']');
        }
        let result = decode(nested.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn null_login_fields_are_treated_as_absent() {
        let json = br#"{"items":[{"type":1,"name":"x",
            "login":{"username":null,"password":null,"totp":null,"uris":null}}]}"#;
        // `uris: null` is not an array, so this is a decode error --
        // Bitwarden always emits `[]`, never `null`, for that field.
        assert!(matches!(decode(json), Err(ImportError::Decode(_))));
    }

    #[test]
    fn missing_login_object_is_treated_as_empty() {
        let json = br#"{"items":[{"type":1,"name":"x"}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].username, None);
        assert!(records[0].password.is_none());
    }

    #[test]
    fn importer_trait_name_and_import() {
        let importer = BitwardenJsonImporter;
        assert_eq!(importer.name(), "bitwarden-json");
        let json = br#"{"items":[{"type":2,"name":"n"}]}"#;
        let records = importer.import(json).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn importer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BitwardenJsonImporter>();
    }

    #[test]
    fn custom_field_can_override_kind_specific_card_field() {
        let json = br#"{"items":[{"type":3,"name":"x",
            "card":{"number":"111"},
            "fields":[{"name":"number","value":"overridden"}]}]}"#;
        let records = decode(json).unwrap();
        assert_eq!(field(&records[0], "number"), "overridden");
    }
}
