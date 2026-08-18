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
let bytes: Vec<u8> = encode_record(&login).unwrap();
let back: Login = decode_record_as::<Login>(&bytes).unwrap();
assert_eq!(back, login);
```

## Encoding is fallible

`encode_record` returns a `Result`, which looks surprising for a
well-typed struct. It is fallible because two ceilings either side of
this crate disagree:

```text
   canonical-cbor  MAX_ENCODED_SIZE    =  1 MiB   per encoded value
   vault-pm        MAX_PLAINTEXT_BYTES = 16 MiB   per sealed object
```

The application's gate is sixteen times the codec's, and nothing
reconciles them — they answer different questions. Between them sits a
band of record sizes that are legal to hold and legal to decode but
**illegal to encode**.

That band is reachable. A peer device with a larger framing budget can
author a `Login` with a 2 MiB password; it seals, syncs, and decodes
here without complaint. The next command that re-serialises it — an
edit, a conflict merge, a restore, an export — arrives back at
`encode_record`. Reporting that closed loses one record; panicking
would lose the process, and because the record stays in the store the
abort would repeat on every later command against that vault.

The boundary is exact: a record encoding to exactly `MAX_ENCODED_SIZE`
succeeds, one byte more fails, and no partial output is ever returned.

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
produced by newer ones. `encode_opaque` puts such a record back into
its `{t, d}` envelope. Both directions re-encode the payload, and both
report an envelope the encoder declines to represent through their
`Result` rather than panicking: wrapping costs one level of nesting, so
a payload nested exactly as deep as the decoder permits does not fit,
and a caller's own framing bound need not be the encoder's size
ceiling. Failing closed there loses one record; panicking loses the
process.

`AnyRecord::summary()` exposes value-redacted inventory data for
host/store planning: record family, secret-field counts, optional/list
shape, lease/expiry flags, and opaque payload sizes without copying
the underlying record values.

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

Diagnostic formatting is closed and value-redacted. `Debug` for each typed
record emits only its type name and `<redacted>`; `AnyRecord` retains only the
variant name and the same marker. Opaque content types, opaque payload bytes,
display metadata, and secret fields are never formatted.

## Errors are inert

`VaultRecordError`'s `Display` and `Debug` strings come exclusively from
literals in this crate. The `ContentTypeMismatch` variant deliberately
suppresses the attacker-controlled `actual` content type from both outputs;
callers that need it can match on the variant.

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
