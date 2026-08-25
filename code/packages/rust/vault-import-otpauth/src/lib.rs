//! # `coding_adventures_vault_import_otpauth` — VLT-PM49 §5.5
//!
//! ## What this crate is
//!
//! A **format adapter**, in the sense `vault-import-export` (VLT15)
//! documents: a dependency-light sibling crate implementing its
//! [`Importer`] trait, sitting next to `vault-import-bitwarden` and
//! `vault-import-csv`. Where those two decode a *multi-record* export from
//! a different password manager, this one decodes the smallest possible
//! external TOTP source: **a file containing exactly one `otpauth://totp/
//! ...` URI** — the de facto "Google Authenticator Key URI Format" every
//! authenticator issuer's QR code and manual-setup page encodes today.
//!
//! It exists so `vault-pm import otpauth-uri FILE` can hand a person the
//! same "point the CLI at a file this issuer gave you" ergonomics
//! `import bitwarden FILE` already has, instead of requiring them to
//! retype a Base32 secret by hand into `item add totp`'s interactive
//! prompt. See `code/specs/VLT-PM49-cli-external-import.md` §5.5 for the
//! full command-surface rationale, including why this is an amendment to
//! VLT-PM49 rather than a change to VLT-PM29 (`item add totp`'s own closed
//! grammar, which this slice does not touch at all).
//!
//! ## What this crate deliberately does *not* parse
//!
//! Everything after the URI's `?` — `secret`, `issuer`, `algorithm`,
//! `digits`, `period` — is **not** interpreted here. This crate's only job
//! is to recognize the `otpauth://totp/` shape, pull the URI's *label*
//! segment out to use as the created item's title (there is no containing
//! Bitwarden/CSV record to borrow a title from, unlike §5.3's existing
//! decoder), and hand the **entire original URI string, byte-for-byte**,
//! to `PortableRecord::totp_seed`. `vault-pm-cli`'s existing
//! `decode_external_totp_field` / `parse_otpauth_totp_uri` — already
//! shipped and tested for VLT-PM49 §5.3's Bitwarden/CSV TOTP fields —
//! parses that query string exactly once, the same code path a Bitwarden
//! JSON export's `otpauth://` TOTP field already goes through. Splitting
//! the work this way means there is exactly one piece of code in the
//! whole workspace that decides what `secret=`/`algorithm=`/`digits=`/
//! `period=` mean, no matter which format handed it the URI.
//!
//! ## Threat model
//!
//! This is untrusted external input — a QR code or pasted URI can come
//! from anywhere, including an attacker (VLT-PM00 §7.1 adversary 6). This
//! crate is bounded and total, matching its sibling adapters' discipline:
//!
//! * **Whole-source ceiling** ([`MAX_SOURCE_BYTES`]) checked before any
//!   other work, so an oversized file cannot force wasted allocation.
//! * **No hardcoded byte-offset indexing into untrusted `&str`.** Every
//!   slice point this crate computes is either the return of `str::get`
//!   (which yields `None`, not a panic, for an out-of-range or
//!   boundary-splitting range) or the byte offset `str::find` returns for
//!   a single ASCII delimiter (`/`, `?`) — always a valid `char` boundary,
//!   because ASCII bytes are never a continuation byte of a multi-byte
//!   UTF-8 sequence. A crafted multi-byte character placed to straddle a
//!   naive fixed-offset slice cannot panic this decoder.
//! * **Closed rejection of `hotp` and every other type.** `otpauth://
//!   hotp/...` is a real, recognized shape this crate refuses on purpose
//!   ([`ImportError::Adapter`]) rather than guessing it means `totp` —
//!   matching VLT-PM29's TOTP-only scope and §5.3's existing
//!   `decode_external_totp_field` precedent.
//! * **Bounded, non-panicking percent-decoding** of the label segment
//!   only; the query string is passed through untouched (see above), so a
//!   crafted query cannot influence this crate's own control flow at all.
//! * **A label ceiling matching manual entry.** [`MAX_LABEL_BYTES`] equals
//!   VLT-PM29 §2's own `Label` prompt bound, so a title arriving through
//!   this importer can never exceed what a person typing one by hand would
//!   be allowed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_vault_import_export::{
    ImportError, Importer, PortableRecord, PortableRecordKind,
};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;

/// Maximum accepted bytes for the whole source file.
///
/// A real `otpauth://totp/...` URI is at most a few hundred bytes even
/// with a long issuer and a maximal-length Base32 secret; this is
/// generous headroom over that while still small enough that a crafted
/// file cannot force meaningful extra work before the first bound check.
pub const MAX_SOURCE_BYTES: usize = 8 * 1024;

/// Maximum accepted bytes for the decoded label (this crate's `title`).
///
/// Matches VLT-PM29 §2's own `Label` prompt bound (256 bytes), so a title
/// arriving through this importer is held to the same ceiling a person
/// typing one by hand at `item add totp` would be.
pub const MAX_LABEL_BYTES: usize = 256;

/// Stable name every adapter names itself by (`vault-pm-cli` does not
/// currently dispatch on this string — it selects the adapter type
/// directly — but every sibling adapter implements it, and a future
/// name-keyed dispatch table gets it for free).
const IMPORTER_NAME: &str = "otpauth-uri";

/// The `Importer` for a file containing exactly one `otpauth://totp/...`
/// URI (VLT-PM49 §5.5).
#[derive(Debug, Default, Clone, Copy)]
pub struct OtpauthUriImporter;

impl Importer for OtpauthUriImporter {
    fn name(&self) -> &str {
        IMPORTER_NAME
    }

    fn import(&self, input: &[u8]) -> Result<Vec<PortableRecord>, ImportError> {
        decode(input).map(|record| vec![record])
    }
}

/// Decode one `otpauth://totp/...` URI, held as the entire contents of
/// `input`, into a single [`PortableRecord`] of kind
/// [`PortableRecordKind::Totp`].
///
/// Surrounding whitespace (a trailing newline from `echo URI > file` is
/// the common case) is trimmed before anything else. The record's
/// `totp_seed` field carries the trimmed URI **unmodified** — see the
/// module docs for why the query string is deliberately not parsed here.
pub fn decode(input: &[u8]) -> Result<PortableRecord, ImportError> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(ImportError::TooLarge("source"));
    }
    let text = core::str::from_utf8(input)
        .map_err(|_| ImportError::Decode("source is not valid UTF-8"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ImportError::Decode("source is empty"));
    }

    let label = parse_label(trimmed)?;

    Ok(PortableRecord {
        kind: PortableRecordKind::Totp,
        title: label,
        username: None,
        password: None,
        url: None,
        notes: None,
        totp_seed: Some(Zeroizing::new(trimmed.to_owned())),
        tags: Vec::new(),
        custom_fields: BTreeMap::new(),
    })
}

/// Validate the `otpauth://totp/` scheme and type, then extract and
/// percent-decode the label segment between the type and the `?` (or end
/// of string, if there is no query at all — an absent `secret` is a
/// mapping failure for the *downstream* decoder to report, not this
/// function's job to anticipate).
fn parse_label(uri: &str) -> Result<String, ImportError> {
    const SCHEME: &str = "otpauth://";

    // `str::get` rather than direct indexing: a source shorter than
    // `SCHEME.len()` bytes, or one where that byte offset does not land on
    // a `char` boundary, yields `None` here instead of panicking. Both are
    // exactly the shapes an adversarial or merely truncated file produces.
    let after_scheme = uri
        .get(..SCHEME.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(SCHEME))
        .map(|_| &uri[SCHEME.len()..])
        .ok_or(ImportError::Decode("source is not an otpauth:// URI"))?;

    // `find('/')` always returns a valid `char`-boundary offset when it
    // returns at all: `/` is single-byte ASCII, which is never the
    // continuation byte of a multi-byte UTF-8 sequence, so it cannot land
    // inside one. The two slices below are therefore panic-free regardless
    // of what multi-byte characters `after_scheme` contains elsewhere.
    let type_end = after_scheme.find('/').ok_or(ImportError::Decode(
        "otpauth URI is missing a /type/label segment",
    ))?;
    let otp_type = &after_scheme[..type_end];
    if !otp_type.eq_ignore_ascii_case("totp") {
        // A real, recognized shape (most commonly `hotp`) this crate
        // refuses on purpose rather than silently treating as `totp` —
        // VLT-PM29's TOTP-only scope, and the same answer §5.3's
        // `decode_external_totp_field` already gives a Bitwarden/CSV
        // TOTP field carrying an `otpauth://hotp/...` URI.
        return Err(ImportError::Adapter(format!(
            "unsupported otpauth type {otp_type:?}: only totp is supported"
        )));
    }

    // Safe: `type_end` is a valid boundary (see above) and `/` is one
    // byte, so `type_end + 1` is the boundary immediately after it.
    let after_type = &after_scheme[type_end + 1..];
    let label_raw = match after_type.find('?') {
        Some(position) => &after_type[..position],
        None => after_type,
    };

    let label = percent_decode_label(label_raw)?;
    if label.is_empty() {
        return Err(ImportError::Decode("otpauth URI has an empty label"));
    }
    if label.len() > MAX_LABEL_BYTES {
        return Err(ImportError::TooLarge("label"));
    }
    Ok(label)
}

/// Bounded percent-decoder for the otpauth URI's label segment.
///
/// `+` decodes to a literal space, the same `application/
/// x-www-form-urlencoded` convention real `otpauth://` producers use for
/// a label containing spaces (matching `vault-pm-cli`'s existing
/// `percent_decode` for the query string, VLT-PM49 §5.3).
fn percent_decode_label(value: &str) -> Result<String, ImportError> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let high = bytes.get(index + 1).copied().and_then(hex_nibble);
                let low = bytes.get(index + 2).copied().and_then(hex_nibble);
                match (high, low) {
                    (Some(high), Some(low)) => {
                        out.push((high << 4) | low);
                        index += 3;
                    }
                    _ => {
                        return Err(ImportError::Decode(
                            "otpauth label has a malformed % escape",
                        ))
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out)
        .map_err(|_| ImportError::Decode("otpauth label is not valid UTF-8 after decoding"))
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Happy path ---

    #[test]
    fn decode_accepts_a_minimal_totp_uri() {
        let uri = "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP";
        let record = decode(uri.as_bytes()).expect("minimal URI must decode");
        assert_eq!(record.kind, PortableRecordKind::Totp);
        assert_eq!(record.title, "Example:alice@example.com");
        assert_eq!(record.totp_seed.as_deref().map(String::as_str), Some(uri));
        assert!(record.username.is_none());
        assert!(record.password.is_none());
    }

    #[test]
    fn decode_percent_decodes_the_label_only() {
        let uri = "otpauth://totp/My%20Bank%3AAlice+Smith?secret=JBSWY3DPEHPK3PXP&issuer=My+Bank";
        let record = decode(uri.as_bytes()).expect("percent-escaped label must decode");
        assert_eq!(record.title, "My Bank:Alice Smith");
        // The query string is passed through byte-for-byte, `+` and all —
        // decoding it is `vault-pm-cli`'s job, not this crate's.
        assert_eq!(record.totp_seed.as_deref().map(String::as_str), Some(uri));
    }

    #[test]
    fn decode_is_case_insensitive_on_scheme_and_type() {
        let uri = "OTPAuth://TOTP/Example:alice?secret=JBSWY3DPEHPK3PXP";
        let record = decode(uri.as_bytes()).expect("case-insensitive scheme/type must decode");
        assert_eq!(record.title, "Example:alice");
    }

    #[test]
    fn decode_trims_surrounding_whitespace() {
        let uri = "  otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP\n";
        let record = decode(uri.as_bytes()).expect("surrounding whitespace must be trimmed");
        assert_eq!(
            record.totp_seed.as_deref().map(String::as_str),
            Some(uri.trim())
        );
    }

    #[test]
    fn decode_preserves_the_full_query_string_untouched_for_downstream_parsing() {
        // Deliberately pathological query content (duplicate `secret`,
        // an unknown parameter): this crate's job is only to recognize
        // the shape and extract the label, never to interpret or
        // normalize the query. Whether this query is ultimately accepted
        // is `vault-pm-cli`'s `parse_otpauth_totp_uri`'s decision alone.
        let uri = "otpauth://totp/Example:alice?secret=AAAA&secret=BBBB&future_param=xyz";
        let record = decode(uri.as_bytes()).expect("decode does not itself reject a bad query");
        assert_eq!(record.totp_seed.as_deref().map(String::as_str), Some(uri));
    }

    #[test]
    fn decode_accepts_a_uri_with_no_query_at_all() {
        // No `?` present: the whole remainder becomes the label, and this
        // crate does not require a query to exist -- an absent `secret`
        // is `parse_otpauth_totp_uri`'s rejection to make downstream.
        let uri = "otpauth://totp/Example:alice";
        let record = decode(uri.as_bytes()).expect("a query-less URI must still decode here");
        assert_eq!(record.title, "Example:alice");
    }

    // --- Closed rejection of unsupported / malformed shapes ---

    #[test]
    fn decode_rejects_hotp_with_a_distinct_error() {
        let uri = "otpauth://hotp/Example:alice?secret=JBSWY3DPEHPK3PXP&counter=0";
        let error = decode(uri.as_bytes()).expect_err("hotp must be refused, not guessed at");
        assert!(matches!(error, ImportError::Adapter(_)), "{error}");
    }

    #[test]
    fn decode_rejects_an_unrelated_type() {
        let uri = "otpauth://motp/Example:alice?secret=JBSWY3DPEHPK3PXP";
        assert!(matches!(
            decode(uri.as_bytes()),
            Err(ImportError::Adapter(_))
        ));
    }

    #[test]
    fn decode_rejects_a_non_otpauth_scheme() {
        for uri in [
            "https://example.com/totp/Example:alice?secret=JBSWY3DPEHPK3PXP",
            "not a uri at all",
            "otpauth:totp/no-slash-slash",
        ] {
            assert!(
                matches!(decode(uri.as_bytes()), Err(ImportError::Decode(_))),
                "{uri}"
            );
        }
    }

    #[test]
    fn decode_rejects_a_scheme_with_no_type_segment() {
        // "otpauth://" present, but no further `/` to delimit a type.
        assert!(matches!(
            decode(b"otpauth://totp-with-no-trailing-slash"),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn decode_rejects_an_empty_label() {
        assert!(matches!(
            decode(b"otpauth://totp/?secret=JBSWY3DPEHPK3PXP"),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn decode_rejects_an_oversize_label() {
        let label = "a".repeat(MAX_LABEL_BYTES + 1);
        let uri = format!("otpauth://totp/{label}?secret=JBSWY3DPEHPK3PXP");
        assert!(matches!(
            decode(uri.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn decode_rejects_an_oversize_source() {
        let padding = "a".repeat(MAX_SOURCE_BYTES + 1);
        let uri = format!("otpauth://totp/{padding}?secret=JBSWY3DPEHPK3PXP");
        assert!(matches!(
            decode(uri.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn decode_rejects_invalid_utf8_source() {
        let mut bytes = b"otpauth://totp/".to_vec();
        bytes.push(0xC3); // start of a 2-byte UTF-8 sequence, no continuation
        bytes.extend_from_slice(b"?secret=JBSWY3DPEHPK3PXP");
        assert!(matches!(decode(&bytes), Err(ImportError::Decode(_))));
    }

    #[test]
    fn decode_rejects_an_empty_or_whitespace_only_source() {
        assert!(matches!(decode(b""), Err(ImportError::Decode(_))));
        assert!(matches!(decode(b"   \n\t  "), Err(ImportError::Decode(_))));
    }

    #[test]
    fn decode_rejects_a_malformed_percent_escape_in_the_label() {
        for uri in [
            "otpauth://totp/bad%2?secret=JBSWY3DPEHPK3PXP",
            "otpauth://totp/bad%zz?secret=JBSWY3DPEHPK3PXP",
            "otpauth://totp/bad%?secret=JBSWY3DPEHPK3PXP",
        ] {
            assert!(
                matches!(decode(uri.as_bytes()), Err(ImportError::Decode(_))),
                "{uri}"
            );
        }
    }

    #[test]
    fn decode_rejects_a_percent_escape_that_decodes_to_invalid_utf8() {
        // %C3 alone is the first byte of a 2-byte UTF-8 sequence with no
        // continuation byte -- valid ASCII hex, invalid resulting text.
        let uri = "otpauth://totp/bad%C3?secret=JBSWY3DPEHPK3PXP";
        assert!(matches!(
            decode(uri.as_bytes()),
            Err(ImportError::Decode(_))
        ));
    }

    /// A multi-byte UTF-8 character placed to straddle the fixed 10-byte
    /// `"otpauth://".len()` offset the scheme check compares against must
    /// be refused, not panic (VLT-PM00 §7.1 adversary 6) -- the same class
    /// of regression `vault-pm-cli`'s own otpauth parser guards against.
    #[test]
    fn decode_does_not_panic_on_a_boundary_splitting_character_at_the_scheme_check() {
        // "otpauth:/" is 9 ASCII bytes; "é" is 2 bytes starting at byte 9,
        // so byte offset 10 -- `"otpauth://".len()` -- lands on its second
        // byte, not a boundary.
        assert!(matches!(
            decode("otpauth:/\u{e9}".as_bytes()),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn decode_does_not_panic_on_a_short_source_shorter_than_the_scheme() {
        assert!(matches!(decode(b"otp"), Err(ImportError::Decode(_))));
    }

    #[test]
    fn decode_does_not_panic_on_multibyte_characters_throughout_the_uri() {
        // Multi-byte characters in the label and (untouched, pass-through)
        // query must not trip any boundary computation.
        let uri = "otpauth://totp/日本語:ユーザー?secret=JBSWY3DPEHPK3PXP&issuer=会社";
        let record = decode(uri.as_bytes()).expect("multibyte content must decode cleanly");
        assert_eq!(record.title, "日本語:ユーザー");
    }

    // --- Trait wiring ---

    #[test]
    fn importer_name_is_stable() {
        assert_eq!(OtpauthUriImporter.name(), "otpauth-uri");
    }

    #[test]
    fn importer_returns_exactly_one_record() {
        let uri = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP";
        let records = OtpauthUriImporter
            .import(uri.as_bytes())
            .expect("a well-formed URI must import");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Example:alice");
    }

    #[test]
    fn importer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OtpauthUriImporter>();
        assert_send_sync::<Box<dyn Importer>>();
    }
}
