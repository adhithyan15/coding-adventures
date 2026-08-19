# `coding_adventures_vault_import_bitwarden` — VLT-PM49

Decodes an **unencrypted Bitwarden JSON export** into the shared
`PortableRecord` vocabulary defined by
[`vault-import-export`](../vault-import-export) (VLT15).

This is a format adapter, one of the sibling crates VLT15's own README
names as future work. It does no vault-pm-specific work — no crypto, no
item identity, no audit events — and is consumed by `vault-pm`'s
`import bitwarden FILE` ceremony (`VLT-PM49-cli-external-import.md`),
which maps each `PortableRecord` onto a real vault-pm item through the
same `item add` machinery a human typing at the CLI uses.

## Where this fits in the vault-pm stack

```text
Bitwarden "Export vault" (json format)
            |
            v
  vault-import-bitwarden::decode()   <- this crate
            |
            v
   Vec<PortableRecord>               <- VLT15's shared vocabulary
            |
            v
  vault-pm-cli's `import bitwarden`  <- maps to AnyRecord, calls
                                         the existing audited
                                         add_item path once per record
```

## Mapping

| Bitwarden `type` | Produces |
|---|---|
| `1` login | one `Login` record, plus a separate `Totp` record when `login.totp` is present |
| `2` secure note | one `SecureNote` record |
| `3` card | one `Card` record; cardholder/number/expiry/CVV land in `custom_fields` under vault-pm's own `Card` field names (`holder`, `number`, `expiry_month`, `expiry_year`, `cvv`) |
| `4` identity, or any unrecognized type | one `Custom("bitwarden-type-N")` record — kept, not dropped, so the host can report an honest skipped count |

A login's first `uris[]` entry becomes `url`; any further URIs are kept
as `custom_fields["uri_2"]`, `["uri_3"]`, … rather than silently lost.
Bitwarden's per-item `fields` (the person's own custom fields) are merged
into `custom_fields` last, so a same-named custom field wins over a
kind-specific one.

Not carried across: folder/collection assignment, the `favorite` flag,
and attachment metadata (there are no attachment bytes in a Bitwarden
JSON export to import in the first place).

## Threat model

Untrusted bytes in, so every array this adapter walks is bounded
(`MAX_ITEMS`, `MAX_URIS_PER_LOGIN`, `MAX_CUSTOM_FIELDS_PER_ITEM`), every
object it destructures is bounded (`MAX_KEYS_PER_OBJECT`), every string it
copies is bounded (`MAX_FIELD_LEN`), and the whole source is bounded
before parsing even starts (`MAX_SOURCE_BYTES`, kept to 16 MiB precisely
because the underlying parser has no per-object key-count limit of its
own, only a nesting-depth cap — a smaller byte ceiling directly bounds
how far a crafted object's junk-key amplification can go before
`MAX_KEYS_PER_OBJECT` rejects it). JSON parsing reuses this repo's
existing depth-capped `json-lexer`/`json-parser`/`json-value` pipeline
rather than a new hand-rolled decoder, so an adversarial deeply-nested
document is refused instead of overflowing the stack. Duplicate JSON
object keys resolve last-write-wins, matching every mainstream JSON
parser, and are covered by an explicit regression test rather than left
as an accident of iteration order. Every secret-shaped source string
inside the parsed JSON tree — not just the copies extracted into
`Zeroizing` `PortableRecord` fields — is recursively zeroized in place
before the tree drops, on every return path including a decode error
partway through the file.

## Usage

```rust
use coding_adventures_vault_import_bitwarden::{decode, BitwardenJsonImporter};
use coding_adventures_vault_import_export::Importer;

let json = br#"{"items":[{"type":1,"name":"GitHub",
    "login":{"username":"alice","password":"hunter2","uris":[]}}]}"#;

let records = decode(json).unwrap();
assert_eq!(records.len(), 1);

// Or through the trait object every adapter implements:
let importer = BitwardenJsonImporter;
assert_eq!(importer.name(), "bitwarden-json");
```

## Bounds

`MAX_SOURCE_BYTES = 16 MiB`, `MAX_ITEMS = 50_000`,
`MAX_KEYS_PER_OBJECT = 128`, `MAX_URIS_PER_LOGIN = 32`,
`MAX_CUSTOM_FIELDS_PER_ITEM = 64`, `MAX_FIELD_LEN = 64 KiB`.

## Out of scope

- The Bitwarden *encrypted* JSON export format (needs an account
  encryption key this crate never has access to).
- Identity records — mapped to `Custom(...)` and skipped by the host,
  because vault-pm has no identity item type yet.
- Attachments — Bitwarden's export carries only attachment metadata, not
  bytes.
