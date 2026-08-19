# `coding_adventures_vault_import_csv` — VLT-PM49

Decodes a **header-keyed login CSV** — the shape Chrome, Edge, Brave,
Firefox, LastPass, and Bitwarden's own CSV export all produce — into the
shared `PortableRecord` vocabulary defined by
[`vault-import-export`](../vault-import-export) (VLT15).

This is a format adapter, consumed by `vault-pm`'s `import csv FILE`
ceremony (`VLT-PM49-cli-external-import.md`), which maps each
`PortableRecord` onto a real vault-pm item through the same `item add`
machinery a human typing at the CLI uses.

## Why one crate instead of one per vendor

VLT15's own README names `vault-import-lastpass`, `vault-import-chrome`,
and `vault-import-firefox` as separate future sibling crates. This
adapter instead recognizes the *union* of the column names those (and
Bitwarden's CSV export) all use, case-insensitively:

| Concept | Recognized headers |
|---|---|
| title | `name`, `title` |
| url | `url`, `login_uri`, `httpRealm`, `formActionOrigin`, `web site`, `website` |
| username | `username`, `login_username`, `user name` |
| password | `password`, `login_password` |
| totp | `totp`, `login_totp` |
| notes | `notes`, `note`, `extra` |

Only logins are in scope — every export shape in the table above is
exclusively a login list. When no title-shaped column is present at all
(Firefox's export has none), the title falls back to the URL, then the
username, then a generated `"Imported login N"`.

## Threat model

CSV structure (quoting, embedded commas/newlines, `""` escaping, ragged
rows) is handled entirely by this repository's existing RFC 4180
state-machine parser (`coding_adventures_csv_parser`); this adapter adds
no CSV-syntax parsing of its own. Untrusted-input bounds:
`MAX_SOURCE_BYTES` (16 MiB), `MAX_ROWS` (200,000), `MAX_COLUMNS` (256),
`MAX_FIELD_LEN` (64 KiB).

**CSV formula injection** — a cell starting with `=`, `+`, `-`, or `@`
can execute as a formula if the CSV is later opened in a spreadsheet
application. This crate only *reads* CSV; it has no export/writer path,
so there is nothing for such a payload to trigger here. It is decoded
and stored as inert literal text, proven by a dedicated test. If vault-pm
ever grows a CSV *export* path, that writer — not this reader — is where
OWASP's standard mitigation (prefixing a leading `'`) belongs.

## Usage

```rust
use coding_adventures_vault_import_csv::{decode, CsvLoginImporter};
use coding_adventures_vault_import_export::Importer;

let csv = "name,url,username,password\nGitHub,https://github.com,alice,hunter2\n";
let records = decode(csv.as_bytes()).unwrap();
assert_eq!(records.len(), 1);

let importer = CsvLoginImporter;
assert_eq!(importer.name(), "browser-csv");
```

## Out of scope

- Non-login rows — Bitwarden's CSV format can in principle carry other
  `type` values in a `type` column; this adapter does not read that
  column and treats every row as a login. (Bitwarden's *JSON* export,
  handled by the sibling `vault-import-bitwarden` crate, is the
  supported path for non-login items.)
- CSV export / writing.
