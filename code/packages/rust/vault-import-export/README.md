# `coding_adventures_vault_import_export` — VLT15

The import/export tier of the Vault stack. Defines the
**versioned portable JSON bundle** that vaults emit on
`vault export` and read on `vault import`, plus an `Importer`
trait that format adapters implement and a hand-rolled JSON
codec.

Format adapters land as **sibling crates** (per VLT00 §VLT15):

- `vault-import-1password`  — `.1pux`
- `vault-import-bitwarden`  — Bitwarden JSON
- `vault-import-keepass`    — KeePassXC `.kdbx`
- `vault-import-lastpass`   — LastPass CSV
- `vault-import-chrome`     — Chrome / Edge CSV
- `vault-import-firefox`    — Firefox CSV
- `vault-import-age`        — age files
- `vault-import-gpg`        — GnuPG files
- `vault-import-sops`       — SOPS files

## Quick example

```rust
use coding_adventures_vault_import_export::{
    export_to_bundle, import_from_bundle, PassthroughImporter,
    PortableRecord, PortableRecordKind,
};
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;

let records = vec![PortableRecord {
    kind: PortableRecordKind::Login,
    title: "GitHub".into(),
    username: Some("alice".into()),
    password: Some(Zeroizing::new("hunter2".into())),
    url: Some("https://github.com".into()),
    notes: None,
    totp_seed: None,
    tags: vec!["work".into()],
    custom_fields: BTreeMap::new(),
}];

let bytes = export_to_bundle(&records)?;     // canonical JSON
let bundle = import_from_bundle(&bytes)?;
assert_eq!(bundle.records, records);
```

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
      "tags": ["work"]
    },
    {
      "kind": {"custom": "workflow"},
      "title": "deploy steps",
      "notes": "ssh ; deploy ; bless"
    }
  ]
}
```

`kind` is a string for known kinds (`login`, `secure_note`,
`card`, `ssh_key`, `totp`) or `{"custom": "<label>"}` for the
escape hatch. The reader rejects unknown root keys, unknown
record keys, unknown `kind` strings, and trailing input.

## Threat model

- **Plaintext crosses the trust boundary here.** Every
  import/export operation is a user-driven ceremony — host
  prompts, warns, and times out. This crate provides the byte
  transformation; it does not decide whether to do it.
- **Sensitive fields are zeroized.** `password`, `totp_seed`,
  and every `custom_fields` value are held under
  `Zeroizing<String>` so dropping a `PortableRecord` scrubs
  the bytes. `Debug` for `PortableRecord` is hand-rolled to
  redact each sensitive field.
- **Strict reader.** Unknown root keys → `Decode`; unknown
  record keys → `Decode`; trailing bytes → `Decode`; unknown
  bundle version → `UnsupportedVersion(v)`. A peer producing
  a future bundle cannot smuggle data through a v1 reader.
- **Bounded sizes.** `MAX_RECORDS = 100_000`,
  `MAX_FIELD_LEN = 64 KiB`, `MAX_BUNDLE_LEN = 256 MiB`,
  `MAX_TAGS_PER_RECORD = 64`,
  `MAX_CUSTOM_FIELDS_PER_RECORD = 64`. Both reader and writer
  enforce all five.
- **Deterministic export.** Same input → same bytes
  (`BTreeMap` sorted custom-field keys, fixed key order at
  every level, U+2028 / U+2029 escaped). Reproducible exports
  are a documented property.

## What this crate is NOT

- Not a sealing layer — VLT01 seals the records before they
  reach the vault.
- Not a recipient list — VLT04 wraps DEKs at export time.
- Not a CSV parser — sibling crates handle each external
  format.
- Not a `serde_json` user — the codec is hand-rolled in one
  file so the wire format is auditable in one place.

## Capabilities

None — pure parser + writer. See `required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT15-vault-import-export.md`](../../../specs/VLT15-vault-import-export.md).
