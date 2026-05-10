# `coding_adventures_vault_records` — VLT02

Typed record schemas for the Vault stack. Sits *above* canonical CBOR
and *below* the VLT01 sealed store: each record is a typed struct
that codecs to canonical CBOR bytes which are then wrapped in
envelope encryption.

## Why

VLT01 stores opaque `Vec<u8>` plaintext. Real apps need `Login`,
`SecureNote`, `Card`, `TotpSeed`, `ApiKey`, `DatabaseCredential`,
`SshKey`, etc. Without this layer every app reinvents the same
serialisation, with the same bugs.

## Quick example

```rust
use coding_adventures_vault_records::{Login, encode_record, decode_record_as};

let login = Login {
    title: "GitHub".into(),
    username: "ada".into(),
    password: "p455w0rd".into(),
    urls: vec!["https://github.com".into()],
    notes: None,
};
let bytes: Vec<u8> = encode_record(&login);
let back: Login = decode_record_as::<Login>(&bytes).unwrap();
assert_eq!(back, login);
```

## Wire format

```text
   record_bytes = canonical_cbor({
       "t": <text content_type, e.g. "vault/login/v1">,
       "d": <map of schema-specific fields>,
   })
```

Two top-level keys, both length-1 text. The canonical CBOR profile
sorts them deterministically (`"d" < "t"` lex), so the wire bytes
are stable regardless of which order the encoder builds them.

## First-party types

| Type                  | Content type              | Use case                                   |
|-----------------------|---------------------------|--------------------------------------------|
| `Login`               | `vault/login/v1`          | Username + password + URLs + notes         |
| `SecureNote`          | `vault/note/v1`           | Free-form encrypted note                   |
| `Card`                | `vault/card/v1`           | Credit / payment card                      |
| `TotpSeed`            | `vault/totp/v1`           | TOTP / HOTP shared secret                  |
| `ApiKey`              | `vault/api-key/v1`        | Static API token + scopes + expiry         |
| `DatabaseCredential`  | `vault/db-credential/v1`  | DB user/pass + host/port + lease metadata  |

App code can register additional types by implementing `VaultRecord`.
`decode_record` returns `AnyRecord::Opaque` for content types this
crate doesn't recognise, so older clients don't crash on records
produced by newer ones.

## Versioning

Each content type carries a `vN` suffix. Schema evolution = new
version. Decoders that only know v1 see v2 records as `Opaque`. A
migration helper (read v1, return a v2 struct) lives alongside the
new type.

## Forward compatibility

Decoders tolerate unknown extra fields in a payload map. So a v1
client can read records produced by a v1.1 client that added
optional fields without breaking; the v1 client just doesn't see
the new fields.

## Sensitive material handling

Every type that carries secrets (`Login.password`, `Card.cvv`,
`Card.number`, `TotpSeed.secret`, `ApiKey.token`,
`DatabaseCredential.password`) implements `Zeroize`. Higher layers
wrap held records in `Zeroizing<T>`.

## Errors are inert

`VaultRecordError`'s `Display` strings come exclusively from
literals in this crate. The `ContentTypeMismatch` variant
deliberately suppresses the attacker-controlled `actual` content
type from its Display output; callers that need it can match on
the variant.

## Where it fits

```text
                   ┌──────────────────────────────────────┐
                   │  application                         │
                   └────────────────┬─────────────────────┘
                                    │  Login { … }
                                    ▼
                   ┌──────────────────────────────────────┐
                   │  vault-records (VLT02)              ◄│  THIS CRATE
                   │  encode_record / decode_record       │
                   └────────────────┬─────────────────────┘
                                    │  canonical CBOR bytes
                                    ▼
                   ┌──────────────────────────────────────┐
                   │  canonical-cbor (RFC 8949 §4.2.3)    │
                   └────────────────┬─────────────────────┘
                                    │  bytes
                                    ▼
                   ┌──────────────────────────────────────┐
                   │  vault-sealed-store (VLT01)          │
                   │  envelope encryption + AAD binding   │
                   └────────────────┬─────────────────────┘
                                    │  ciphertext
                                    ▼
                   ┌──────────────────────────────────────┐
                   │  storage-core: opaque KV             │
                   └──────────────────────────────────────┘
```

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT02-vault-records.md`](../../../specs/VLT02-vault-records.md).
