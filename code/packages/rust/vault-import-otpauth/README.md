# `coding_adventures_vault_import_otpauth` — VLT-PM49 §5.5

Decodes a file containing exactly **one `otpauth://totp/...` URI** — the
de facto "Google Authenticator Key URI Format" every authenticator
issuer's QR code and manual TOTP setup page encodes today — into the
shared `PortableRecord` vocabulary defined by
[`vault-import-export`](../vault-import-export) (VLT15).

This is a format adapter, the same shape as its siblings
[`vault-import-bitwarden`](../vault-import-bitwarden) and
[`vault-import-csv`](../vault-import-csv), consumed by `vault-pm`'s
`import otpauth-uri FILE` ceremony
(`code/specs/VLT-PM49-cli-external-import.md` §5.5). It maps the one URI
onto a real vault-pm item through the same `item add` machinery a human
typing at the interactive `item add totp` prompt uses — see that spec
section for why this is a `vault-pm import` verb rather than a change to
`item add totp`'s own closed grammar (VLT-PM29), which this slice does
not touch.

## Where this fits in the vault-pm stack

```text
issuer's QR code / "can't scan?" manual setup text
            |
            v (photographed → decoded elsewhere, or copy/pasted to a file)
otpauth://totp/Issuer:alice?secret=...&issuer=...
            |
            v
  vault-import-otpauth::decode()      <- this crate: recognizes the
            |                            otpauth://totp/ shape, extracts
            |                            + percent-decodes the label only
            v
     PortableRecord                   <- VLT15's shared vocabulary;
       { totp_seed: Some(<the full   ,    totp_seed carries the ORIGINAL
         URI, byte for byte>) }            URI untouched
            |
            v
vault-pm-cli's `import otpauth-uri`  <- decode_external_totp_field /
                                         parse_otpauth_totp_uri (VLT-PM49
                                         §5.3) parses secret/issuer/
                                         algorithm/digits/period from the
                                         query string -- the SAME code a
                                         Bitwarden/CSV TOTP field already
                                         goes through -- then calls the
                                         existing audited add_item path
```

## Why this crate parses the label but not the query string

`vault-pm-cli` already has a complete, tested `otpauth://totp/...` query
decoder (`decode_external_totp_field` / `parse_otpauth_totp_uri`,
VLT-PM49 §5.3) for the case where a Bitwarden JSON or CSV row's TOTP field
is itself an `otpauth://` URI. That decoder takes its record's *title*
from the containing login record — there is no such containing record
here, since this crate's whole input *is* one bare URI with nothing else
around it. So this crate's only original work is extracting a title:
pull the label segment (`otpauth://totp/<LABEL>?...`) out and
percent-decode it, then hand the **entire original URI, byte for byte**
to `PortableRecord::totp_seed` so the existing, already-reviewed query
decoder runs exactly once, regardless of which format produced the URI.

## Threat model

Untrusted bytes in (VLT-PM00 §7.1 adversary 6 — a QR code or pasted URI
can come from anywhere), so:

- **Whole-source ceiling** (`MAX_SOURCE_BYTES = 8 KiB`) checked before any
  other work — generous for any real URI, small enough that a crafted
  file cannot force meaningful extra work.
- **No fixed-byte-offset indexing into untrusted `&str`.** Every slice
  boundary this crate computes is either the result of `str::get` (which
  returns `None` rather than panicking on an out-of-range or
  boundary-splitting range) or the byte offset of a single-byte ASCII
  delimiter (`/`, `?`) found via `str::find`, which is always a valid
  `char` boundary because an ASCII byte can never be a continuation byte
  of a multi-byte UTF-8 sequence.
- **`hotp` and every other non-`totp` type is refused, not guessed at**
  (`ImportError::Adapter`), matching VLT-PM29's TOTP-only scope and
  §5.3's existing behavior for the same shape embedded in a Bitwarden/CSV
  field.
- **A label ceiling matching manual entry** (`MAX_LABEL_BYTES = 256`,
  VLT-PM29 §2's own `Label` prompt bound), so an imported title can never
  exceed what a person typing one by hand at `item add totp` would be
  allowed.
- **The query string is never interpreted here.** It reaches
  `PortableRecord::totp_seed` exactly as received — duplicate keys,
  unknown parameters, an oversize or missing `secret`, all of it — so
  there is exactly one place in the workspace, §5.3's existing decoder,
  that decides what any of it means.

## Usage

```rust
use coding_adventures_vault_import_otpauth::{decode, OtpauthUriImporter};
use coding_adventures_vault_import_export::Importer;

let uri = b"otpauth://totp/GitHub:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub";

let record = decode(uri).unwrap();
assert_eq!(record.title, "GitHub:alice@example.com");

// Or through the trait object every adapter implements:
let importer = OtpauthUriImporter;
assert_eq!(importer.name(), "otpauth-uri");
assert_eq!(importer.import(uri).unwrap().len(), 1);
```

## Bounds

`MAX_SOURCE_BYTES = 8 KiB`, `MAX_LABEL_BYTES = 256 bytes`.

## Out of scope

- **Everything after `?`** — `secret`, `issuer`, `algorithm`, `digits`,
  `period` — parsed by `vault-pm-cli`'s existing VLT-PM49 §5.3 decoder,
  not this crate.
- **`otpauth://hotp/...`** — a real, recognized shape this crate refuses
  on purpose, matching VLT-PM29's TOTP-only scope.
- **QR image decoding.** This crate consumes a URI as text; turning a QR
  code *image* into that URI text is `import otpauth-qr FILE`'s job, and
  is explicitly deferred — see
  `code/specs/VLT-PM49-cli-external-import.md` §9 for what was checked
  and what closes it.
