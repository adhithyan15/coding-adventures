//! # `coding_adventures_vault_import_csv` — VLT-PM49
//!
//! ## What this crate is
//!
//! A **format adapter** for the import/export tier defined by VLT15
//! (`coding_adventures_vault_import_export`). It decodes a header-keyed
//! login CSV — the shape every mainstream browser password manager
//! (Chrome, Edge, Brave, Firefox) and several third-party managers
//! (LastPass, Bitwarden's CSV export) all produce — into the shared
//! [`PortableRecord`] vocabulary every adapter in this tier produces.
//!
//! Real exports do not agree on column names:
//!
//! | Source | Header row |
//! |---|---|
//! | Chrome / Edge / Brave | `name,url,username,password` |
//! | Firefox | `url,username,password,httpRealm,formActionOrigin,guid,...` |
//! | LastPass | `url,username,password,totp,extra,name,grouping,fav` |
//! | Bitwarden CSV | `folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp` |
//!
//! Rather than ship one crate per vendor (VLT15's own README enumerates
//! `vault-import-lastpass`, `vault-import-chrome`, `vault-import-firefox`
//! as separate future crates), this adapter recognizes the union of
//! these header names, case-insensitively, and maps whichever ones are
//! present. That covers "browser CSV" as VLT-PM00 §23 item 13 names it
//! with one crate instead of three near-duplicates; per-vendor crates
//! remain available later if header drift ever makes that necessary.
//!
//! Only logins are in scope. Every export in the table above is
//! exclusively a login list — unlike Bitwarden's JSON export, none of
//! these CSV shapes carry secure notes or payment cards.
//!
//! ## Column resolution
//!
//! A column is matched by a case-insensitive, trimmed comparison against
//! a fixed alias list (see [`TITLE_ALIASES`] and friends). The **first**
//! header in a row that matches an alias set wins; a file with more than
//! one column matching the same alias set (e.g. both `name` and `title`
//! present) uses whichever [`coding_adventures_csv_parser`] happens to
//! keep — which, because CSV headers are turned into a
//! `HashMap<String, String>` per row upstream, is simply "the value
//! under that literal header text," so two *differently spelled* aliases
//! for the same concept in one file cannot collide; the CSV parser's own
//! row-shape rules (short rows padded, long rows truncated) govern
//! everything else.
//!
//! When a row has no title-shaped column at all (Firefox's export has
//! none), the title falls back to the URL, then the username, then a
//! generated `"Imported login N"` — never an empty string, because
//! vault-pm's own record validation rejects one and a fallback here is
//! far more useful than a whole-file rejection over one blank cell.
//!
//! ## Threat model
//!
//! * **Untrusted bytes.** [`MAX_SOURCE_BYTES`] bounds the whole input
//!   before parsing. [`MAX_ROWS`] and [`MAX_COLUMNS`] bound the decoded
//!   shape; [`MAX_FIELD_LEN`] bounds every string copied out. `MAX_COLUMNS`
//!   can only reject a wide row *after* `coding_adventures_csv_parser` has
//!   already fully materialized it into a `HashMap`, so
//!   [`MAX_SOURCE_BYTES`] is kept deliberately small (real exports are low
//!   single-digit megabytes) to directly bound how far that amplification
//!   can go, rather than claiming it is zero.
//! * **CSV structure attacks.** Parsing is delegated to this
//!   repository's existing RFC 4180 state-machine parser
//!   (`coding_adventures_csv_parser`), which already handles embedded
//!   quotes, embedded newlines and commas inside quoted fields, `""`
//!   escaping, and ragged rows. This adapter adds no CSV-syntax parsing
//!   of its own.
//! * **CSV formula injection.** A cell beginning with `=`, `+`, `-`, `@`,
//!   a tab, or a carriage return is a well-known attack when a CSV is
//!   later *opened* in a spreadsheet application — the leading character
//!   can make the cell evaluate as a formula. This adapter only ever
//!   **reads** CSV; it has no CSV-writing/export path, so there is
//!   nothing here for such a payload to trigger. It is decoded and
//!   stored as inert literal text — [`decode`]'s test suite proves a
//!   formula-shaped username/password/notes value round-trips byte-for-
//!   byte and is never interpreted. If a CSV **export** path is ever
//!   added to vault-pm, it must neutralize these leading characters
//!   (e.g. a leading `'`) on the way out per standard OWASP CSV-injection
//!   guidance; that responsibility does not exist yet because there is
//!   no writer to carry it.
//! * **Plaintext residue.** `password` and `totp_seed` are `Zeroizing` at
//!   the `PortableRecord` boundary, but the source cell values inside the
//!   parsed `rows: Vec<HashMap<String, String>>` are ordinary `String`s --
//!   `coding_adventures_csv_parser` has no reason to know any column is
//!   sensitive. [`decode`] zeroizes every cell value in `rows` in place
//!   once every record has been extracted from it, before `rows` drops.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_csv_parser::{parse_csv, CsvError};
use coding_adventures_vault_import_export::{
    ImportError, Importer, PortableRecord, PortableRecordKind,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::collections::{BTreeMap, HashMap};

// === Bounds =================================================================

/// Maximum accepted raw source bytes, checked before any parsing begins.
///
/// A real login-manager CSV export, even a large one, is low single-digit
/// megabytes; this ceiling is generous headroom over that, not an estimate
/// of a plausible file. Kept deliberately smaller than an earlier 32 MiB
/// draft: [`MAX_COLUMNS`] can only reject a wide row *after*
/// `coding_adventures_csv_parser` has already fully materialized it into a
/// `HashMap`, so a smaller byte ceiling directly bounds how large that
/// worst case can get rather than claiming the amplification factor is
/// zero.
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum accepted data rows (header excluded).
pub const MAX_ROWS: usize = 200_000;
/// Maximum accepted columns in the header row.
pub const MAX_COLUMNS: usize = 256;
/// Maximum accepted bytes for any single cell this adapter reads.
pub const MAX_FIELD_LEN: usize = 64 * 1024;

/// Case-insensitive header aliases this adapter recognizes as the title.
pub const TITLE_ALIASES: &[&str] = &["name", "title"];
/// Case-insensitive header aliases this adapter recognizes as the URL.
pub const URL_ALIASES: &[&str] = &[
    "url",
    "login_uri",
    "httprealm",
    "formactionorigin",
    "web site",
    "website",
];
/// Case-insensitive header aliases this adapter recognizes as the username.
pub const USERNAME_ALIASES: &[&str] = &["username", "login_username", "user name"];
/// Case-insensitive header aliases this adapter recognizes as the password.
pub const PASSWORD_ALIASES: &[&str] = &["password", "login_password"];
/// Case-insensitive header aliases this adapter recognizes as a TOTP seed.
pub const TOTP_ALIASES: &[&str] = &["totp", "login_totp"];
/// Case-insensitive header aliases this adapter recognizes as free-form notes.
pub const NOTES_ALIASES: &[&str] = &["notes", "note", "extra"];

/// Decodes a header-keyed browser/LastPass/Bitwarden-style login CSV into
/// [`PortableRecord`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct CsvLoginImporter;

impl Importer for CsvLoginImporter {
    fn name(&self) -> &str {
        "browser-csv"
    }

    fn import(&self, input: &[u8]) -> Result<Vec<PortableRecord>, ImportError> {
        decode(input)
    }
}

/// Decode a login CSV into [`PortableRecord`]s.
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
    let mut rows = parse_csv(text).map_err(|error| match error {
        CsvError::UnclosedQuote => ImportError::Decode("unclosed quoted CSV field"),
    })?;
    // Every secret-shaped *value* this adapter reads (password, TOTP
    // seed) is copied out into a `Zeroizing`-held `PortableRecord` field
    // below, but the source copy inside `rows` is an ordinary `String` --
    // `coding_adventures_csv_parser` has no reason to know any column is
    // sensitive. `outcome` captures every return path below (success
    // *and* every early `Err`, e.g. an over-`MAX_COLUMNS` row or an
    // over-`MAX_FIELD_LEN` cell reached partway through the file) so the
    // zeroize pass a few lines down is unconditional: a malformed row is
    // the normal case this crate's threat model exists to survive, and
    // it must not silently skip the wipe of every row already decoded
    // before that point just because it was also the input that
    // triggered the error.
    let outcome: Result<Vec<PortableRecord>, ImportError> = (|| {
        if rows.len() > MAX_ROWS {
            return Err(ImportError::TooLarge("MAX_ROWS"));
        }
        let mut records = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            if row.len() > MAX_COLUMNS {
                return Err(ImportError::TooLarge("MAX_COLUMNS"));
            }
            records.push(decode_row(row, index + 1)?);
        }
        Ok(records)
    })();
    // Scrub every cell value in place before `rows` drops, the same
    // property `read_external_import_source`'s `Zeroizing` buffer
    // already gives the raw bytes this table was parsed from -- run
    // unconditionally, before `outcome` is inspected. Keys are column
    // *names* (`"password"`, `"url"`, ...), never secret-shaped, and
    // `HashMap` only exposes them as `&String` through
    // `values_mut`/`iter_mut` regardless, so they are left alone.
    for row in rows.iter_mut() {
        for value in row.values_mut() {
            value.zeroize();
        }
    }
    outcome
}

fn lookup<'a>(
    row: &'a HashMap<String, String>,
    aliases: &[&str],
) -> Result<Option<&'a str>, ImportError> {
    for alias in aliases {
        if let Some(value) = row
            .iter()
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(alias))
            .map(|(_, value)| value.as_str())
        {
            if value.len() > MAX_FIELD_LEN {
                return Err(ImportError::TooLarge("CSV field"));
            }
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}

fn decode_row(
    row: &HashMap<String, String>,
    ordinal: usize,
) -> Result<PortableRecord, ImportError> {
    let url = lookup(row, URL_ALIASES)?;
    let username = lookup(row, USERNAME_ALIASES)?;
    let password = lookup(row, PASSWORD_ALIASES)?;
    let totp = lookup(row, TOTP_ALIASES)?;
    let notes = lookup(row, NOTES_ALIASES)?;
    let title = lookup(row, TITLE_ALIASES)?
        .or(url)
        .or(username)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Imported login {ordinal}"));

    Ok(PortableRecord {
        kind: PortableRecordKind::Login,
        title,
        username: username.map(str::to_owned),
        password: password.map(|s| Zeroizing::new(s.to_owned())),
        url: url.map(str::to_owned),
        notes: notes.map(str::to_owned),
        totp_seed: totp.map(|s| Zeroizing::new(s.to_owned())),
        tags: Vec::new(),
        custom_fields: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_chrome_style_csv() {
        let csv = "name,url,username,password\nGitHub,https://github.com,alice,hunter2\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.kind, PortableRecordKind::Login);
        assert_eq!(r.title, "GitHub");
        assert_eq!(r.url.as_deref(), Some("https://github.com"));
        assert_eq!(r.username.as_deref(), Some("alice"));
        assert_eq!(r.password.as_deref().map(String::as_str), Some("hunter2"));
    }

    #[test]
    fn decodes_firefox_style_csv_without_title_column() {
        let csv = "url,username,password,httpRealm,formActionOrigin,guid\n\
                   https://example.com,bob,pw,,https://example.com,abc123\n";
        let records = decode(csv.as_bytes()).unwrap();
        // No name/title column at all -- falls back to the URL.
        assert_eq!(records[0].title, "https://example.com");
        assert_eq!(records[0].username.as_deref(), Some("bob"));
    }

    #[test]
    fn decodes_lastpass_style_csv_with_totp() {
        let csv = "url,username,password,totp,extra,name,grouping,fav\n\
                   https://aws.amazon.com,root,pw,JBSWY3DPEHPK3PXP,,AWS,,0\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "AWS");
        assert_eq!(
            records[0].totp_seed.as_deref().map(String::as_str),
            Some("JBSWY3DPEHPK3PXP")
        );
    }

    #[test]
    fn decodes_bitwarden_csv_style_headers() {
        let csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
                   ,0,login,Work Email,,,0,https://mail.example.com,carol,pw,\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "Work Email");
        assert_eq!(records[0].url.as_deref(), Some("https://mail.example.com"));
        assert_eq!(records[0].username.as_deref(), Some("carol"));
    }

    #[test]
    fn falls_back_to_generated_title_when_nothing_else_present() {
        let csv = "password\nonlyapassword\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "Imported login 1");
    }

    #[test]
    fn header_matching_is_case_insensitive_and_trims_whitespace() {
        let csv = " Name , URL \nSite,https://example.com\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "Site");
        assert_eq!(records[0].url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn embedded_comma_and_newline_in_quoted_field_are_preserved() {
        let csv = "name,notes\n\"Site\",\"line one, with comma\nline two\"\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(
            records[0].notes.as_deref(),
            Some("line one, with comma\nline two")
        );
    }

    #[test]
    fn multiple_rows_decode_independently() {
        let csv = "name,url,username,password\n\
                   A,https://a.example,ua,pa\n\
                   B,https://b.example,ub,pb\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].title, "A");
        assert_eq!(records[1].title, "B");
    }

    // --- CSV formula injection: stored as inert literal text ---

    #[test]
    fn formula_injection_payloads_are_stored_literally_not_interpreted() {
        let payloads = [
            "=cmd|'/c calc'!A1",
            "+1+1",
            "-2+3+cmd|' /C calc'!A0",
            "@SUM(1+1)",
            "=HYPERLINK(\"http://evil.example\",\"click\")",
        ];
        for payload in payloads {
            // Quoted per RFC 4180 since some payloads contain commas --
            // this is what a real CSV writer would do, and the point of
            // the test is what *this adapter* does with the decoded
            // value, not CSV-quoting mechanics (already covered above).
            let quoted = payload.replace('"', "\"\"");
            let csv =
                format!("name,url,username,password\nSite,https://x.example,\"{quoted}\",pw\n");
            let records = decode(csv.as_bytes()).unwrap();
            // Stored exactly as given -- no character stripped, no
            // interpretation, and decoding itself never fails on it.
            assert_eq!(records[0].username.as_deref(), Some(payload));
        }
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
    fn rejects_unclosed_quote() {
        let csv = "name,url\n\"unterminated,https://x.example\n";
        assert!(matches!(
            decode(csv.as_bytes()),
            Err(ImportError::Decode(_))
        ));
    }

    #[test]
    fn header_only_csv_decodes_to_zero_rows() {
        let csv = "name,url,username,password\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn ragged_short_row_is_padded_with_empty_and_still_decodes() {
        let csv = "name,url,username,password\nSite,https://x.example\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "Site");
        assert_eq!(records[0].username, None);
        assert!(records[0].password.is_none());
    }

    #[test]
    fn ragged_long_row_extra_fields_are_discarded_by_the_parser() {
        let csv = "name,url\nSite,https://x.example,unexpected,extra\n";
        // The upstream RFC 4180 parser truncates extra fields on a long
        // row; this adapter just proves it does not error or panic.
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "Site");
    }

    #[test]
    fn rejects_too_many_rows() {
        let mut csv = String::from("name,url\n");
        for i in 0..(MAX_ROWS + 1) {
            csv.push_str(&format!("Site{i},https://x.example\n"));
        }
        assert!(matches!(
            decode(csv.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn rejects_too_many_columns() {
        let mut header = String::new();
        let mut row = String::new();
        for i in 0..(MAX_COLUMNS + 1) {
            if i > 0 {
                header.push(',');
                row.push(',');
            }
            header.push_str(&format!("col{i}"));
            row.push('v');
        }
        let csv = format!("{header}\n{row}\n");
        assert!(matches!(
            decode(csv.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn rejects_oversize_field() {
        let big = "x".repeat(MAX_FIELD_LEN + 1);
        let csv = format!("name,url\n{big},https://x.example\n");
        assert!(matches!(
            decode(csv.as_bytes()),
            Err(ImportError::TooLarge(_))
        ));
    }

    #[test]
    fn importer_trait_name_and_import() {
        let importer = CsvLoginImporter;
        assert_eq!(importer.name(), "browser-csv");
        let csv = "name,url\nSite,https://x.example\n";
        let records = importer.import(csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn importer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CsvLoginImporter>();
    }

    #[test]
    fn unicode_fields_round_trip() {
        let csv = "name,url,username,password\n\
                   Café,https://café.example,alice@éxample.com,naïve-Päßwôrd-🔑\n";
        let records = decode(csv.as_bytes()).unwrap();
        assert_eq!(records[0].title, "Café");
        assert_eq!(
            records[0].password.as_deref().map(String::as_str),
            Some("naïve-Päßwôrd-🔑")
        );
    }
}
