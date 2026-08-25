# VLT-PM49 — CLI External Import: Bitwarden JSON, Browser CSV, and otpauth

## Status

Normative Phase 1B contract for VLT-PM00 §23 item 13, "Bitwarden/KDBX/
browser CSV import adapters." This slice ships the Bitwarden JSON and
browser/LastPass-style CSV adapters as a complete, well-tested pair.
KDBX is explicitly deferred; §8 records why and what closes it.

**Amendment.** §5.5 and the `otpauth-uri`/`otpauth-qr` grammar lines in
§3 were added later, at the user's explicit request for TOTP setup via a
QR code image and/or `otpauth://` URI instead of only manual Base32
secret entry through `item add totp` (`VLT-PM29-cli-totp-create.md`).
§1 explains why that request became an amendment to *this* spec rather
than a change to VLT-PM29's own closed grammar. `otpauth-uri` ships
complete in this amendment; `otpauth-qr` (QR *image* decoding) is
explicitly deferred, matching §8's existing precedent for KDBX — see the
new §9 for why and what closes it.

## 1. Purpose

`vault-pm import bitwarden FILE` and `vault-pm import csv FILE` read a
plaintext export produced by a *different* password manager and create
new vault-pm items from it, through the same audited, one-item-at-a-time
creation path `vault-pm item add` already uses. This is the "migrate in"
half of VLT-PM00 §2.1's import/export promise; `vault-pm import portable
FILE` (VLT-PM18) remains the disaster-recovery half, restoring a vault-pm
vault's own encrypted export into an empty target.

These two ceremonies look similar — both start with the word `import` —
and are deliberately *not* the same machinery, for reasons §2 below
explains from source, not by assertion.

### 1.1 Why `otpauth-uri`/`otpauth-qr` amend this spec, not VLT-PM29

`VLT-PM29-cli-totp-create.md` §1 states plainly that "QR scanning,
`otpauth://` parsing, code generation/display, HOTP counters, online
issuer discovery, clock correction, migration formats, and
non-interactive input are outside this command" — meaning `item add
totp`'s own closed grammar. That boundary is a deliberate scope
statement, not an oversight, and it is still accurate: this amendment
touches none of `item add totp`'s prompts, grammar, or code.

An `otpauth://` URI or a QR code encoding one is exactly the same *shape*
of thing this spec already handles: an external, untrusted, plaintext
artifact that becomes a new vault-pm item through the unmodified `item
add` publication path, read from a file named on the command line — not
typed at an interactive prompt. `PortableRecordKind::Totp` (§5) already
exists in `vault-import-export`'s vocabulary for exactly a standalone TOTP
seed, and §5.3's `decode_external_totp_field` /
`parse_otpauth_totp_uri` already parse the `otpauth://totp/...` query
string a Bitwarden/CSV TOTP field can carry. So the new work is a new
*format* this spec's existing `import FORMAT FILE` grammar names (§3), a
new sibling adapter crate (§5.5) implementing the same `Importer` trait
§2 already established as the reuse boundary, reusing §5.3's decoder
unchanged — not a new command, a new mutation path, or a new audit event.
Extending VLT-PM49 keeps that reuse honest; extending VLT-PM29 would mean
either duplicating this machinery next to `item add totp`'s interactive
prompts, or quietly widening a command whose own spec says in the same
paragraph that this exact capability is out of scope.

## 2. Reuse precedent: what this slice builds on, and what it does not

VLT-PM00 §6 lists `vault-import-export` as the reuse target for "actual
Bitwarden, KDBX, browser, CSV adapters," and that crate's own
description says format adapters ship as sibling crates implementing its
`Importer` trait. Before writing any adapter, this slice checked whether
`vault-pm import portable` (VLT-PM17/18) already consumes
`vault-import-export`'s types — because this campaign has repeatedly
found that vault-pm reimplements the generic crypto/envelope layers
independently rather than consuming the generic packages directly (its
own object format, its own commit DAG, its own Argon2id/AEAD wrapping),
and the reuse map's crypto-layer rows have been wrong about direct reuse
before.

They do not connect. `grep`ing `vault-pm-cli`'s and
`vault-pm-application`'s dependency graphs for `vault_import_export` or
`vault-import-export` finds nothing: zero references. `vault-pm import
FILE`'s actual implementation (`portable_import` in `vault-pm-cli`,
backed by `open_portable_with_passphrase` /
`audited_import_opened_portable_snapshot` in `vault-pm-application`) is
vault-pm's own passphrase-protected, Argon2id-KDF'd, AEAD-sealed,
signed-bootstrap-bound snapshot format — the same independent-
implementation pattern this campaign found at the crypto layer
elsewhere. It is not `PortableBundle` JSON. `PassthroughImporter` in
`vault-import-export` is not called by any vault-pm code today.

That is the right precedent to *not* copy: an external product's plain
JSON/CSV export needs no vault-pm cryptography to decode, so there is no
reason for a format adapter to depend on vault-pm's independent object
format the way `vault-pm import portable` does. But `vault-import-
export`'s `PortableRecord`/`Importer` vocabulary genuinely fits this
different job — decoding an external file into a typed, bounded,
zeroizing-secret record shape that has no vault-pm-specific content —
exactly as its own module documentation describes ("format adapters ship
as sibling crates"). This slice is the first consumer of that vocabulary
in the whole workspace:

- `code/packages/rust/vault-import-bitwarden` — implements `Importer`,
  decodes an unencrypted Bitwarden JSON export into
  `Vec<PortableRecord>`.
- `code/packages/rust/vault-import-csv` — implements `Importer`, decodes
  a header-keyed browser/LastPass/Bitwarden-CSV login export into
  `Vec<PortableRecord>`.
- `code/packages/rust/vault-import-otpauth` (§5.5, added by this
  amendment) — implements `Importer`, decodes a file holding exactly one
  `otpauth://totp/...` URI into a single `PortableRecord`.

Neither the original two crates nor this amendment's third one touch
vault-pm cryptography, item identity, or audit events. `vault-pm-cli` is
the only place `PortableRecord` values become real vault-pm items, and it
does that by mapping each one onto the *existing* `item add` machinery
(§4), not by inventing a second mutation/publication path next to the
one VLT-PM05 already specifies.

## 3. Grammar

```text
vault-pm [--vault NAME] import portable FILE
vault-pm [--vault NAME] import bitwarden FILE
vault-pm [--vault NAME] import csv FILE
vault-pm [--vault NAME] import kdbx FILE
vault-pm [--vault NAME] import otpauth-uri FILE
vault-pm [--vault NAME] import otpauth-qr FILE
```

`otpauth-uri` and `otpauth-qr` are this amendment's two new formats
(§5.5, §9). `otpauth-qr` parses the identical shape as `kdbx` — one
format keyword, one path — and, like `kdbx`, always fails closed with the
`unsupported` exit class before opening `FILE`: §9 records why QR *image*
decoding is deferred and what closes it, the same treatment §8 already
gives KDBX.

This supersedes VLT-PM18 §2's bare `vault-pm import FILE`, which is now
`vault-pm import portable FILE` — the format keyword is mandatory so the
grammar names every format VLT-PM00 §14.4 always documented, rather than
letting one format own the unqualified verb by historical accident.
Existing scripts written against the bare form must add `portable`; this
repository's stated policy is to break compatibility deliberately rather
than carry a silent default. `import kdbx FILE` is accepted by the
parser and always fails closed with the `unsupported` exit class before
opening `FILE` (§8) — present in the grammar so the command surface
matches what VLT-PM00 §14.4 documents, rather than a format silently
missing from `--help` with no explanation.

V1 accepts exactly one non-empty Unicode path per format, matching
VLT-PM18 §2's existing discipline: no source-passphrase flag (neither
external format is encrypted), no overwrite switch, no merge mode.

## 4. What "the same audited creation as `item add`" means, concretely

VLT-PM00's task brief for this slice requires imported items to satisfy
"the same audited/publish-before-release discipline... exactly like any
other item creation." `vault-pm-cli`'s existing single-item add path
already is that discipline: `prepare_item_create` reserves entropy and
authenticates the vault once, `ItemCreateContext::document` builds and
validates an `ItemDocument`, and `ItemCreateContext::complete` calls
`UnlockedVaultV1::add_item`, which VLT-PM05 §7a already specifies as
crash-resumable, entropy-bound, and audited — the exact function
`vault-pm item add login` calls today. `UnlockedVaultV1::add_item`'s own
doc comment states the session is consumed on every return path
specifically "so a successful caller cannot keep using stale pins,
catalog contents, or search state" — one authenticated session creates
*one* item, by construction, not a policy choice this slice could relax.

So the ceremony for N external records is N calls to that exact
existing pipeline, once per record, not one new bulk-mutation primitive:

1. **Read the source file** through a new bounded host method,
   `read_external_import_source`, modeled on VLT-PM47's
   `read_attachment_source` rather than VLT-PM18's
   `read_portable_export`: the buffer is `Zeroizing`, because unlike a
   vault-pm portable artifact (already ciphertext) a Bitwarden/CSV export
   *is* the person's plaintext secrets. No vault is opened yet, so a
   missing file, wrong permissions, or empty file is refused before any
   authentication prompt and needs no audit event — nothing vault-side
   has happened.
2. **Decode** the bytes with the format's adapter crate
   (`vault-import-bitwarden::decode` or `vault-import-csv::decode`). Any
   `ImportError` here is `CliFailure::InvalidCommand` (malformed source),
   still before any vault access.
3. **Map** each `PortableRecord` to zero or one vault-pm `(content_type,
   AnyRecord)` pair (§5). A record whose kind has no vault-pm equivalent
   (`PortableRecordKind::SshKey`, `Custom(_)`) is *skipped*, counted, and
   not silently dropped from the reported outcome.
4. If mapping produces **zero** creatable records (an empty file, or a
   file whose every record is unsupported), report the outcome and open
   no vault at all — an import that creates nothing is not a mutation,
   and `vault-pm password generate`'s Phase 1B precedent already
   established that not every command needs a vault (VLT-PM44 §2.2).
5. Otherwise, for **each** mapped record in turn: call
   `prepare_item_create` (authenticate — through the agent cache first,
   VLT-PM48, then falling back to the ordinary prompt, exactly like
   every other authenticated command), build the document, and call
   `context.complete` on success or `context.fail` on a validation
   failure. Each of those calls is the unmodified VLT-PM05/VLT-PM15 path:
   its own `ItemCreate` audit event, its own crash-resumable publication,
   its own entropy reservation. This slice adds no new audit event kind.
6. **Aggregate and report** (§6). No item title, URL, username, or
   secret ever reaches CLI output.

### 4.1 Why re-authentication per record, and its cost

Because step 5 calls the unmodified single-item path once per record,
importing N records re-derives the vault's Argon2id KEK N times unless a
running `vault-pm agent` (VLT-PM48) is caching the passphrase — in which
case only the KDF derivation itself repeats locally, not a passphrase
prompt. This is the same cost anyone incurs today by running `vault-pm
item add login` N times from a script; this slice introduces no new
cost model, and choosing it over a new bulk-session API keeps every
crash/audit guarantee exactly what VLT-PM41/42 already proved for
single-item creation, rather than opening a second, unproven mutation
surface. A bulk-session `add_items` primitive that authenticates once
and publishes one commit per batch is legitimate future work — starting
`vault-pm agent start` first is the documented mitigation until it
lands.

### 4.2 Why no merge/conflict resolution

VLT-PM18's restore path requires its target to be logically empty
specifically because it is reconstructing *the same* vault-pm identity
space the export snapshot came from — a source item ID could otherwise
collide with a live target item. An external Bitwarden/CSV record has no
vault-pm item ID at all; there is no identity for a target item to
collide with. So, exactly like `item add`, every imported record becomes
a brand-new item with a freshly generated `ItemId`, unconditionally.
This is the same "no merge, always new identity" answer VLT-PM18 §7
already gives for the *portable* restore path, arrived at for a
different, simpler reason: there is nothing here to merge against in the
first place.

## 5. Field mapping

### 5.1 Bitwarden JSON (`vault-import-bitwarden`)

| Bitwarden `type` | vault-pm outcome |
|---|---|
| `1` login | one `LOGIN_V1` item (`username`, `password`, first `uris[]` entry as the sole URL, `notes`); a second, separate `TOTP_SEED_V1` item when `login.totp` is present (§5.3) |
| `2` secure note | one `SECURE_NOTE_V1` item (`notes` becomes the body) |
| `3` card | one `CARD_V1` item, fields taken from the adapter's `custom_fields` (`holder`, `number`, `expiry_month`, `expiry_year`, `cvv`) |
| `4` identity, or any other value | **skipped** — vault-pm has no identity item type yet |

A login's extra `uris[]` entries beyond the first, and Bitwarden's
per-item custom `fields[]`, are preserved on the `PortableRecord` (as
`custom_fields`) but have no vault-pm-side destination in V1 and are
therefore also not created as separate fields on the mapped item —
recorded as a known gap in §8, not silently discarded by the adapter
(the adapter crate keeps them; the CLI mapping layer is what does not
yet have anywhere to put them).

### 5.2 Browser/LastPass CSV (`vault-import-csv`)

Every recognized CSV row maps to one `LOGIN_V1` item
(`username`/`password`/`url`/`notes` from the matched columns), plus a
separate `TOTP_SEED_V1` item when a `totp`/`login_totp` column is present
and non-empty (§5.3). CSV carries no secure-note or card rows in any of
the vendor shapes this adapter recognizes (see the adapter's own
README), so §5.1's card/note mapping does not apply here.

### 5.3 TOTP field decoding

Both formats can carry a TOTP seed as either raw Base32 (Bitwarden's
`login.totp`, LastPass CSV's `totp` column) or an `otpauth://totp/...`
URI (both formats accept either shape in practice). `vault-pm-cli` adds
one shared decoder, `decode_external_totp_field`, tried in this order:

1. **`otpauth://totp/...` URI** — scheme and type checked exactly
   (`hotp` is refused, matching VLT-PM29's TOTP-only scope), label and
   query percent-decoded under a fixed length bound, `secret` required,
   `issuer`/`algorithm`/`digits`/`period` optional with VLT-PM29's
   existing defaults (`SHA1`, 6 digits, 30 seconds) when absent.
2. **Raw Base32** — normalized (uppercased, padding `=` stripped,
   internal whitespace stripped) and then decoded by the same
   `decode_totp_base32` the interactive `item add totp` form already
   uses, so a seed accepted here decodes identically to one a person
   typed by hand.

A field that is neither is a mapping failure for that one record (does
not abort the whole import; counted in `failed`, §6).

### 5.4 Not carried across (documented, not silently dropped)

- Folder/collection assignment, and Bitwarden's `favorite` flag: every
  created item starts with no collections and `favorite = false`,
  identical to `item add`'s existing default (`ItemCreateContext::
  document` always builds `LwwRegister::new(false, ...)`).
- Attachment bytes: neither Bitwarden's JSON export nor any CSV shape
  here carries attachment content, only (for Bitwarden) metadata this
  slice does not read.
- Bitwarden's item-level custom `fields[]` and a login's extra `uris[]`
  beyond the first (§5.1) — kept by the adapter, no destination on the
  mapped vault-pm item yet.

### 5.5 Standalone otpauth URI (`vault-import-otpauth`) — added by this amendment

`vault-pm import otpauth-uri FILE` reads `FILE` as the entire contents of
one `otpauth://totp/...` URI — the de facto "Google Authenticator Key URI
Format" every authenticator issuer's QR code and manual TOTP setup page
encodes — and creates exactly one `TOTP_SEED_V1` item from it, through
the same `item add` publication path as every other format in this spec.

Unlike §5.1/§5.2's login/note/card records, there is no containing record
to take a title from, so `vault-import-otpauth::decode` does two, and
only two, things of its own:

1. **Validates the scheme and type.** Exactly `otpauth://totp/...`
   (case-insensitive on the scheme and type only). Any other type —
   `hotp` is the real-world case — is refused with a distinct error
   rather than guessed at, matching §5.3's existing answer for the same
   shape embedded in a Bitwarden/CSV field, and VLT-PM29's TOTP-only
   scope.
2. **Extracts and percent-decodes the label** (`otpauth://totp/<LABEL>?
   ...`) to use as the created item's title, bounded to
   `VLT-PM29-cli-totp-create.md` §2's own 256-byte `Label` bound.

Everything after `?` — `secret`, `issuer`, `algorithm`, `digits`,
`period` — is **not** parsed by the new crate. The entire original URI,
byte for byte, becomes the produced `PortableRecord`'s `totp_seed` field,
which is exactly the field §5.3's existing, unmodified
`decode_external_totp_field` / `parse_otpauth_totp_uri` already knows how
to decode (a Bitwarden JSON export's `login.totp` field carrying an
`otpauth://` URI takes the identical path today). So the query string is
decoded by exactly one piece of code in the workspace regardless of which
format handed it the URI — this amendment reuses that decoder unchanged
rather than duplicating its RFC 4648 Base32 handling, algorithm/digit/
period validation, or percent-decoding a second time. Absent `issuer`,
`algorithm`, `digits`, or `period` fall back to the same defaults §5.3
and VLT-PM29 already define (`none`/SHA1/6/30).

`vault-import-otpauth`'s own README and test suite carry the full
adversarial-input matrix for this shape (oversized source, invalid UTF-8,
non-otpauth scheme, missing type segment, empty/oversize label, malformed
percent escapes, multi-byte-boundary panics), the same "each crate's own
broad test matrix rather than asserted here" discipline §7 states for the
other two adapters.

## 6. Output and errors

Success reports only aggregate counts, matching VLT-PM18 §8's existing
style:

```text
Import complete: created=C skipped=S failed=F
```

`created` counts items actually published; `skipped` counts records
whose kind has no vault-pm equivalent (§5); `failed` counts records that
were mappable but whose `item add` publication itself returned an
application error (e.g. a bound violation) — the ordinary
`context.fail` path already publishes that failure's own audited event.
No source path, title, username, URL, secret, or record body is ever
printed. Source-file and decode failures (before any vault is opened)
use the invalid exit class; an authentication failure partway through
uses the locked class exactly like any other authenticated command, and
whatever items were already durably created before that point remain —
this is an ordinary sequence of independent, already-audited mutations,
not one atomic operation, precisely because §4.2 established there is no
shared identity space to make atomic in the first place.

## 7. Threat model — VLT-PM00 §7.1 adversary 6, "malicious imported data"

Both adapter crates are designed against oversized, malformed,
ambiguous, and structurally adversarial input, verified by each crate's
own broad test matrix rather than asserted here:

- **Bounded everything, before decode.** Whole-source byte ceilings
  (`MAX_SOURCE_BYTES`), and bounded arrays/fields/rows once inside
  (`MAX_ITEMS`, `MAX_URIS_PER_LOGIN`, `MAX_CUSTOM_FIELDS_PER_ITEM`,
  `MAX_ROWS`, `MAX_COLUMNS`, `MAX_FIELD_LEN` — see each crate's README).
  JSON has no entity-expansion mechanism, so a byte-bounded document
  cannot decode to an unboundedly large tree.
- **Deeply nested JSON.** `vault-import-bitwarden` reuses this
  workspace's existing depth-capped `json-lexer`/`json-parser`/
  `json-value` pipeline rather than a new hand-rolled decoder; its test
  suite includes a 10,000-deep nested array proving the inherited cap
  turns an adversarial `[[[[...]]]]` into a clean `Err` rather than a
  stack overflow.
- **Duplicate/ambiguous keys.** JSON duplicate object keys resolve
  last-write-wins, the same rule every mainstream JSON parser applies,
  tested explicitly (both a duplicate field inside one item and a
  duplicate top-level `"items"` key).
- **Type confusion.** Every field the Bitwarden adapter reads is
  type-checked; a crafted file where `"login"` is a string, number, or
  array is rejected rather than coerced.
- **CSV structure.** Delegated entirely to this workspace's existing
  RFC 4180 state-machine `csv-parser` (embedded quotes/commas/newlines,
  `""` escaping, ragged rows); this slice adds no CSV-syntax parsing.
- **CSV formula injection** (`=cmd|...`, `+`, `-`, `@`-prefixed cells) —
  named explicitly by this slice's task brief as a known class. This
  import-only path never writes a CSV, so there is no spreadsheet a
  crafted cell could later detonate in; such a value is decoded and
  stored as inert literal text, proven by a dedicated round-trip test.
  If vault-pm ever grows a CSV *export* path, neutralizing a leading
  `=`/`+`/`-`/`@` on the way out is that writer's responsibility, not
  retroactively this reader's — recorded in `vault-import-csv`'s README
  so the obligation is not lost.
- **Log-injection.** §6 already gives the answer this adversary needs:
  no imported field ever reaches CLI stdout/stderr or an audit event: an
  attacker who puts a fake syslog line, ANSI escape, or `Set-Cookie`-
  shaped string in an item title cannot get it echoed anywhere this
  product controls.
- **`otpauth-uri` (added by this amendment).** A QR code or pasted URI
  can come from anywhere. `vault-import-otpauth` bounds the whole source
  before any other work (`MAX_SOURCE_BYTES`), never indexes an untrusted
  `&str` at a fixed byte offset (every slice boundary is either
  `str::get`-checked or a single-byte ASCII delimiter offset from
  `str::find`, so a crafted multi-byte character cannot trigger a
  boundary panic — the same class of regression §5.3's own
  `parse_otpauth_totp_uri` already guards against), refuses `hotp` and
  every other non-`totp` type with a closed error instead of guessing,
  and bounds the decoded label to VLT-PM29's own 256-byte `Label` limit.
  See that crate's own README and test suite for the full matrix.

## 8. Explicitly deferred: KDBX

KeePass's `.kdbx` (KDBX4) format is a real encrypted container in its
own right — Argon2d or AES-256-KDF key derivation, then AES-256 or
ChaCha20 authenticated decryption of an inner XML document — not a
plaintext export like the other two formats in this slice. Before
assuming that needs a new dependency, this slice checked what this
workspace already has: `argon2d`, `aes`, `aes-modes`, and
`chacha20-poly1305` all exist as standalone crates
(`code/packages/rust/argon2d`, `.../aes`, `.../aes-modes`,
`.../chacha20-poly1305`), and vault-pm's *own* vault unlock already
composes Argon2id with an AEAD cipher for the same "password-derived key
opens an encrypted container" shape (VLT-PM00 §8.1). So closing this gap
would not need a new cryptographic primitive — it would need: the KDBX4
binary container framing (a distinct format from vault-pm's own `VPO1`
envelope, VLT-PM00 §10.2), an Argon2d parameter block reader with the
same hard-bounded-before-allocation discipline VLT-PM00 §8.2 requires of
vault-pm's own KDF parameters, and a hand-rolled bounded KeePass-flavored
XML reader for the decrypted inner document (a fourth structurally
different untrusted-input parser, on top of the JSON and CSV parsers
this slice already reviews).

That is real, separable work at least as large as the Bitwarden and CSV
adapters combined, and reviewing it well alongside them in one PR was
judged to cost more than it would return — the same call this campaign
made for Windows named-pipe support in VLT-PM48 §6, deferred there and
documented rather than silently missing. `import kdbx FILE` therefore
stays in the grammar (§3) — VLT-PM00 §14.4 always documented it as part
of this command surface — and every invocation fails closed with the
`unsupported` exit class (VLT-PM00 §14.7 code 8) before opening `FILE`,
the same pattern VLT-PM48 §6 uses for every agent verb on Windows. A
follow-up slice owns: a `vault-import-keepass` adapter crate producing
the same `PortableRecord` vocabulary as the other two, and wiring
`import kdbx` in `vault-pm-cli` to it exactly as this slice wires
`bitwarden`/`csv`.

## 9. Explicitly deferred: QR image decoding — added by this amendment

`import otpauth-uri FILE` (§5.5) closes the "type or paste an `otpauth://`
URI into a file" half of the user's request. The other half — point the
CLI at a **QR code image** (a screenshot or export from an issuer's setup
page, or another password manager's export; this is a CLI product with
no camera, so "scanning" always means an image file someone already has)
and decode its embedded `otpauth://` URI — needs turning pixels into that
URI text first, which is a materially different and larger problem than
§5.5's text parsing.

**What was checked before assuming a new dependency was needed.** Every
Rust package under `code/packages/rust/` and `code/programs/rust/` was
searched for existing QR handling: `code/packages/rust/qr-code` exists,
but it is an **encoder only** (ISO/IEC 18004 string → `ModuleGrid` →
pixels, for `barcode-2d` to render) with no decode direction at all — it
cannot be pointed at an image and asked what it says. The `barcode-2d`,
`gf256`, `reed-solomon`, `aztec-code`, `data-matrix`, `pdf417`, and
`micro-qr` sibling packages are the same shape: encoders and their
supporting field/error-correction arithmetic, not decoders. No `paint-vm`
family package reads a QR code either — they render vector scenes, not
recognize codes in raster images. Two actively-maintained crates.io
crates were also checked: `rqrr` (pure-Rust QR-grid detection and
decoding from a raw luma buffer; its own runtime dependencies are just
`g2p` and `lru` — the heavier `image` crate is only an *optional* feature
this project would not need to enable) and `png` (pure-Rust PNG
decoding). Together they would be lighter than pulling in the general
`image` crate, and PNG-only is a reasonable restriction for this use
case: QR exports are typically PNG already, and JPEG's lossy compression
actively works against reliable QR decoding.

**Why this is still deferred rather than shipped alongside §5.5.** Every
other dependency this whole vault-pm stack composes today —
`vault-pm-cli`'s own `Cargo.toml`, and every crate in its dependency
closure including this amendment's own `vault-import-otpauth` — is a
workspace-internal path dependency; none of vault-pm's import/export or
cryptographic surface depends on an external (crates.io) crate today.
Adding the first two would be a real, deliberate architectural decision
about this product's untrusted-input surface, and it deserves its own
focused review rather than riding as a second concern inside this
amendment's URI-parsing review — the same "cost more to review well
together than it would return" judgment §8 already made for KDBX, for an
analogous reason (a structurally different untrusted-input format, here
a compressed raster image rather than a binary container). Decompression-
bomb-style guarding (an attacker-supplied PNG with a tiny file size but
enormous decoded pixel dimensions) needs its own adversarial test
fixtures, and correctness needs real QR images to decode against —
`code/packages/rust/qr-code`'s own `encode_and_layout`/`render_png` can
generate those fixtures in-house, which is real, separable follow-up
work of its own.

`import otpauth-qr FILE` therefore stays in the grammar (§3) — named so
it is not silently missing from `--help` with no explanation — and every
invocation fails closed with the `unsupported` exit class before opening
`FILE`, identical in shape to `import kdbx` (§8). A follow-up slice owns:
adding `rqrr` and `png` to a new crate (or extending
`vault-import-otpauth`), decoding the PNG header first to cap width ×
height before any full pixel decode, running `rqrr` detection/decoding to
recover the embedded string, and feeding that string through §5.5's
existing `otpauth://` label/query pipeline unchanged — plus real
QR-image test fixtures generated from `qr-code`'s own encoder, covering a
corrupt file, a non-QR image, an oversized image, and a QR code encoding
something other than an `otpauth://` URI.

## 10. Acceptance gates

1. `import bitwarden`/`import csv`/`import portable`/`import otpauth-uri`
   each parse exactly one source path and nothing else; `import kdbx` and
   `import otpauth-qr` parse the same shape but fail closed with the
   `unsupported` exit class before any file access, in a unit test that
   also proves the file was never opened.
2. A well-formed Bitwarden JSON export containing one of each mapped
   kind (login with URIs and a TOTP seed, secure note, card) creates the
   expected vault-pm items, each independently reachable by `item show`
   after restart, and the login's TOTP seed becomes a separately
   reachable `totp code` item.
3. A well-formed CSV export in each of the four documented column
   shapes (Chrome, Firefox, LastPass, Bitwarden CSV) creates the
   expected login items.
4. A file whose every record is an unsupported kind reports
   `created=0 skipped=N failed=0` and never authenticates a vault (a
   fake `CliHost` asserting `read_existing_passphrase`/`fill_entropy`
   are never called for that case); the equivalent proof for
   `import otpauth-uri` is that a malformed/`hotp`/empty source is
   refused with the invalid exit class without ever authenticating.
5. Both original adapter crates' own malformed-input matrices (§7) are
   green, plus a `vault-pm-cli`-level test importing a
   truncated/malformed file of each format and observing the invalid
   exit class with no vault opened; `vault-import-otpauth`'s own matrix
   (§5.5) — oversized source, invalid UTF-8, non-otpauth scheme, missing
   type segment, `hotp`, empty/oversize label, malformed percent
   escapes, multi-byte-boundary panics — is equally green.
6. Imported secrets never appear in stdout, stderr, or an audit-event
   field — verified the same way VLT-PM18 §9 verifies it for portable
   import: grep the real CLI's captured output and the durable audit
   rows for known fixture plaintext after a real end-to-end run through
   the actual executable.
7. Each created item's audit trail is indistinguishable in shape from
   one created by `item add` — same `ItemCreate` event kind, same
   crash-resumable publication — because no new mutation or audit path
   was introduced for this slice.
8. `import otpauth-uri` on a well-formed URI carrying every optional
   query parameter (`issuer`, `algorithm`, `digits`, `period`) creates an
   item whose `item show` fields match the URI exactly, and on a URI
   carrying only `secret` falls back to VLT-PM29's documented defaults
   (`none`/SHA1/6/30) — proven through the real executable with a real
   PTY by independently recomputing the expected TOTP code from the same
   RFC 6238 engine `totp code` uses and checking the executable agrees,
   the same style of proof VLT-PM45 §9 already uses for the interactive
   path.
9. `otpauth://hotp/...` and every other non-`totp` type is refused by
   `import otpauth-uri` with the invalid exit class, not silently
   imported as if it were `totp`.
