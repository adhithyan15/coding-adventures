# VLT15 — Vault Import / Export

## Overview

The import/export tier of the Vault stack. Defines:

- a versioned **portable JSON bundle** — the canonical
  interchange shape between vaults and external tools,
- the [`Importer`] trait every adapter implements,
- a [`PassthroughImporter`] reference that reads the bundle
  itself,
- `export_to_bundle` / `import_from_bundle` — hand-rolled JSON
  writer/reader that produces / consumes the canonical bytes.

Format adapters land as **sibling crates** (per VLT00 §VLT15):

- `vault-import-1password` — `.1pux`
- `vault-import-bitwarden` — Bitwarden JSON
- `vault-import-keepass` — KeePassXC `.kdbx`
- `vault-import-lastpass` — LastPass CSV
- `vault-import-chrome` / `vault-import-firefox` — browser CSV
- `vault-import-age` / `vault-import-gpg` / `vault-import-sops`

Each is dependency-light so a vault using only Bitwarden
import doesn't pull in the KDBX parser.

Implementation lives at `code/packages/rust/vault-import-export/`.

## Why this layer is "ceremony"

Import/export is the one place in the Vault stack where
**plaintext crosses the trust boundary**:

- On *import*, plaintext arrives from the user (a file they
  exported from a competing product). The crate sees it
  before VLT01 sealing happens; the host is responsible for
  sealing immediately and scrubbing the intermediate.
- On *export*, plaintext leaves the vault. VLT01-sealed bytes
  are decrypted just to be repackaged into the portable
  format. Same scrub rule applies.

All import/export operations are documented as
**explicit user-driven ceremonies**: the host UI prompts,
warns, and times out. This crate gives you the byte
transformation; it does not decide whether to do it.

## Wire format (versioned)

```json
{
  "version": 1,
  "records": [
    {
      "kind": "login",
      "title": "GitHub",
      "username": "alice",
      "password": "hunter2",
      "url": "https://github.com",
      "notes": null,
      "totp_seed": null,
      "tags": ["work"],
      "custom_fields": {}
    },
    {
      "kind": {"custom": "workflow"},
      "title": "deploy steps",
      "notes": "ssh ; deploy ; bless"
    }
  ]
}
```

Known `kind` values: `login`, `secure_note`, `card`,
`ssh_key`, `totp`. Unknown wire kinds become
`{"custom": "<label>"}`.

Optional fields are simply omitted on the wire (and `None` in
the Rust type). The reader rejects unknown root keys,
unknown record keys, raw control chars in strings, and
trailing input.

## Public API

```rust
pub struct PortableBundle { pub version: u32, pub records: Vec<PortableRecord> }
pub struct PortableRecord {
    pub kind: PortableRecordKind,
    pub title: String,
    pub username: Option<String>,
    pub password: Option<Zeroizing<String>>,    // sensitive
    pub url: Option<String>,
    pub notes: Option<String>,
    pub totp_seed: Option<Zeroizing<String>>,   // sensitive
    pub tags: Vec<String>,
    pub custom_fields: BTreeMap<String, Zeroizing<String>>,  // sensitive values
}

pub enum PortableRecordKind {                   // #[non_exhaustive]
    Login, SecureNote, Card, SshKey, Totp, Custom(String),
}

pub trait Importer: Send + Sync {
    fn name(&self) -> &str;
    fn import(&self, input: &[u8]) -> Result<Vec<PortableRecord>, ImportError>;
}

pub struct PassthroughImporter;

pub fn export_to_bundle(records: &[PortableRecord]) -> Result<Vec<u8>, ImportError>;
pub fn import_from_bundle(bytes: &[u8]) -> Result<PortableBundle, ImportError>;

pub enum ImportError {
    Decode(&'static str),
    TooLarge(&'static str),
    InvalidParameter(&'static str),
    UnsupportedVersion(u32),
    Adapter(String),
}
```

## Bounds

| Constant                          | Value         |
|-----------------------------------|---------------|
| `MAX_RECORDS`                     | 100,000       |
| `MAX_FIELD_LEN`                   | 64 KiB        |
| `MAX_BUNDLE_LEN`                  | 256 MiB       |
| `MAX_TAGS_PER_RECORD`             | 64            |
| `MAX_CUSTOM_FIELDS_PER_RECORD`    | 64            |
| `BUNDLE_VERSION`                  | 1             |

## Threat model & test coverage

| Threat                                                | Defence                                              | Test                                                |
|------------------------------------------------------|-------------------------------------------------------|-----------------------------------------------------|
| Untrusted bytes inflate any single number             | bounded reader at every step                          | `reject_oversize_bundle_bytes`, `reject_oversize_field`, `reject_too_many_tags`, `reject_too_many_custom_fields`, `reject_too_many_records` |
| Peer producer adds new top-level key                  | strict reader: unknown root key → `Decode`            | `reject_unknown_root_key`                           |
| Peer producer adds new record key                     | strict reader: unknown record key → `Decode`          | `reject_unknown_record_key`                         |
| Bundle version mismatch                               | `UnsupportedVersion(v)`                                | `reject_unsupported_version`                        |
| Trailing-bytes injection                              | reader requires EOF                                    | `reject_trailing_bytes`                             |
| Raw control chars in string fields                    | reader requires JSON-escape                            | `reject_unescaped_control_char`                     |
| `dbg!(record)` leaks password / TOTP / custom values  | hand-rolled redacted `Debug`                           | `record_debug_redacts_password_and_totp`            |
| Plaintext residue after drop                          | `Zeroizing<String>` on every sensitive field           | structural — `Drop` runs scrub                      |
| JSON output broken in `<script>` (U+2028 / U+2029)    | `json_escape` emits ` ` / ` `                | `round_trip_preserves_u2028_u2029`                  |
| Determinism for reproducible exports                  | `BTreeMap` sorted keys + fixed top-level key order     | `deterministic_export`                              |
| **Multi-byte UTF-8 silently corrupted on import**       | accumulate bytes through the loop; `String::from_utf8` at close | `round_trip_preserves_unicode_passwords`           |
| **Permissive comma handling (parser differential)**    | strict separator: comma between fields, no trailing comma | `reject_missing_comma_between_record_fields`, `reject_missing_comma_between_bundle_keys`, `reject_trailing_comma_in_record`, `reject_trailing_comma_in_bundle` |
| Leading zeros on number literals                       | RFC 8259 — rejected by `read_u32`                       | `reject_leading_zero_in_version`                    |
| Malformed UTF-8 in input string                        | `String::from_utf8` validation at close                 | `reject_invalid_utf8_in_string`                     |

## Out of scope (future PRs)

- Format adapters (sibling crates per external format).
- Recipient list at export — VLT04 wraps DEKs.
- Streaming reader for very large bundles.
- Schema validation — VLT02 runs on the host's import path
  after `import_from_bundle`.
- Compression — host's responsibility.

## Citations

- VLT00-vault-roadmap.md — VLT15 placement.
- 1Password 1PUX, Bitwarden JSON, KeePassXC KDBX, LastPass
  CSV — external formats covered by sibling adapter crates.
- VLT01-vault-sealed-store — the layer the host runs each
  imported record through before persistence.
- VLT02-vault-records — typed-record schema validation runs
  here.
- VLT04-vault-recipients — wraps DEKs at export time.
