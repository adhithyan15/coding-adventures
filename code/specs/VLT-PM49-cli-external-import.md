# VLT-PM49 — CLI External Import: Bitwarden JSON, Browser CSV, and otpauth

## Status

Normative Phase 1B contract for VLT-PM00 §23 item 13, "Bitwarden/KDBX/
browser CSV import adapters." This slice ships the Bitwarden JSON and
browser/LastPass-style CSV adapters as a complete, well-tested pair.
KDBX was originally deferred; Amendment 2 (§8) closes it.

**Amendment 1.** §5.5 and the `otpauth-uri`/`otpauth-qr` grammar lines in
§3 were added later, at the user's explicit request for TOTP setup via a
QR code image and/or `otpauth://` URI instead of only manual Base32
secret entry through `item add totp` (`VLT-PM29-cli-totp-create.md`).
§1 explains why that request became an amendment to *this* spec rather
than a change to VLT-PM29's own closed grammar. `otpauth-uri` shipped
complete in that amendment; `otpauth-qr` (QR *image* decoding) remains
explicitly deferred — see §9 for why and what closes it.

**Amendment 2.** §8 originally deferred KDBX entirely. This amendment
closes that gap: `import kdbx FILE` now decrypts a real KDBX4 database
and imports its entries through the same `item add` publication path
every other format in this spec uses. §8 below is rewritten in full to
specify the closed design; nothing in §1–§7 or §9 changes.

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

`otpauth-uri` and `otpauth-qr` are Amendment 1's two new formats
(§5.5, §9); `kdbx` (§8) went from always failing closed with the
`unsupported` exit class to a real decoder in Amendment 2. `otpauth-qr`
still parses its one-keyword-one-path shape and always fails closed
before opening `FILE`: §9 records why QR *image* decoding remains
deferred and what closes it, the same "real, separable follow-up work"
treatment §8 gave KDBX before Amendment 2 closed it.

This supersedes VLT-PM18 §2's bare `vault-pm import FILE`, which is now
`vault-pm import portable FILE` — the format keyword is mandatory so the
grammar names every format VLT-PM00 §14.4 always documented, rather than
letting one format own the unqualified verb by historical accident.
Existing scripts written against the bare form must add `portable`; this
repository's stated policy is to break compatibility deliberately rather
than carry a silent default. `import otpauth-qr FILE` is accepted by the
parser and always fails closed with the `unsupported` exit class before
opening `FILE` (§9) — present in the grammar so the command surface
matches what VLT-PM00 §14.4 documents, rather than a format silently
missing from `--help` with no explanation.

V1 accepts exactly one non-empty Unicode path per format for
`bitwarden`/`csv`/`otpauth-uri`, matching VLT-PM18 §2's existing
discipline: no overwrite switch, no merge mode. `import kdbx FILE` is
the one exception to "no source-passphrase flag" — a KDBX4 database is
itself an encrypted container (§8), so decrypting it needs the
database's own master password, prompted interactively exactly like any
other authenticated command (§8.3), never taken as a command-line
argument (a passphrase on the command line lands in shell history and
`ps` output — the same reasoning every other passphrase prompt in this
product already follows).

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

## 8. KDBX (KeePass) — closed by Amendment 2

KeePass's `.kdbx` (KDBX4) format is a real encrypted container in its
own right — Argon2d/Argon2id (or legacy AES-KDF) key derivation, then
AES-256-CBC or ChaCha20 decryption of an HMAC-SHA256-authenticated block
stream, holding an optionally-gzipped, structurally distinct inner XML
document — not a plaintext export like `bitwarden`/`csv`/`otpauth-uri`.
This amendment closes that gap with a new
`code/packages/rust/vault-import-keepass` crate plus `vault-pm-cli`
wiring, without adding any new cryptographic primitive: `argon2d`,
`argon2id`, `aes`, `aes-modes`, `chacha20-poly1305`, `hmac`, `sha256`,
`sha512`, `deflate`, and `xml-parser` all already exist as standalone
workspace crates and are reused unchanged. `vault-pm-repository` already
depends on `hmac` (which itself depends on `subtle`, a tiny external
crate providing a constant-time-comparison primitive `ct-compare` does
not) — so this amendment's dependency graph introduces no *new* external
crate into vault-pm's transitive closure, correcting §9's now-superseded
"zero external dependencies anywhere in vault-pm's import/crypto
surface" framing, written before that fact was checked here.

### 8.1 Format, byte for byte

KDBX4's layered structure, derived from the file format directly (no
official machine-readable spec exists; this project's own README carries
the full citation list, primarily Wladimir Palant's reverse-engineered
documentation and the KeePassXC/kdbxweb/keepass-rs implementations,
cross-checked against each other):

1. **Signature + version** (12 bytes, cleartext): `0x9AA2D903`,
   `0xB54BFB67`, then a `u16` minor and `u16` major version. Major
   version must be exactly `4`; any other value (including KDBX3's `3`)
   is `ImportError::UnsupportedVersion` — KDBX3 is explicitly out of
   scope (§8.6).
2. **Outer header** (cleartext, TLV: `u8` field ID, `u32` LE length,
   value bytes, terminated by field ID `0`): `CipherID` (16-byte UUID —
   only AES256-CBC and ChaCha20 accepted, §8.6), `CompressionFlags`
   (`u32`: `0`=none, `1`=gzip), `MainSeed` (32 bytes), `EncryptionIV`
   (16 bytes for AES-CBC, 12 for ChaCha20), `KdfParameters` (a
   `VariantDictionary` — its own nested TLV format, §8.2).
3. **Header integrity**, immediately after the header's terminator
   field: 32 bytes `SHA-256(header bytes)` (corruption check only, not
   authentication — verified but never trusted as proof of a correct
   password), then 32 bytes `HMAC-SHA256(header_hmac_key, header bytes)`
   where `header_hmac_key` is the per-block key formula (§8.3) evaluated
   at block index `0xFFFF_FFFF_FFFF_FFFF` — this is the real "is this
   the right master key" check, evaluated *before* touching the rest of
   the file.
4. **Body**: the outer cipher (AES-256-CBC or ChaCha20, keyed by
   `encryption_key`, §8.3) decrypts to a sequence of HMAC-SHA256-
   authenticated blocks (§8.3), each independently verified before its
   data is trusted. Reassembled block data is gzip-decompressed if
   `CompressionFlags == 1`.
5. **Inner header** (cleartext once decrypted, same TLV shape as the
   outer header but a disjoint field-ID space): `InnerRandomStreamID`
   (`u32`: `2`=Salsa20, `3`=ChaCha20 — only ChaCha20 accepted, §8.6),
   `InnerRandomStreamKey` (32 or 64 bytes), zero or more `Binary` entries
   (attachment bytes — not carried across, §8.5), terminated by field ID
   `0`.
6. **Inner XML document** — the rest of the decompressed body, parsed
   with `xml-parser` (§8.4). `Value` elements with `Protected="True"`
   hold Base64 ciphertext, decrypted with the inner stream cipher
   (§8.3) advancing statefully across the document in field order.

### 8.2 KDF parameters, bounded before they are trusted

`KdfParameters` is a `VariantDictionary`: `u16` version (`0x0100`), then
repeated entries (`u8` type tag, `u32` LE key-name length, key name
UTF-8, `u32` LE value length, value bytes), terminated by a zero byte.
The dictionary's `$UUID` entry selects the KDF (Argon2d, Argon2id, or
legacy AES-KDF — only the two Argon2 variants are accepted, §8.6); its
remaining entries (`S`=salt, `M`=memory in **bytes**, `I`=iterations,
`P`=parallelism, `V`=Argon2 version) are exactly the parameters
`argon2d`/`argon2id` already require.

This is the load-bearing security property this amendment adds that
neither existing adapter needed: **every one of these parameters is
attacker-controlled and read before the master key is verified.**
`argon2d`/`argon2id` validate their own internal invariants (salt
length, parallelism range, `memory_cost >= 8 * parallelism`) but do not
themselves cap `memory_cost`'s *absolute* size — a crafted file
declaring `M` near `u64::MAX` bytes would otherwise drive an allocation
of that size inside the KDF before any of those checks even run out of
memory gracefully. This crate adds its own ceiling, checked against the
raw header-declared values *before* calling either KDF function:

```rust
pub const MAX_KDF_MEMORY_KIB: u64 = 1024 * 1024;   // 1 GiB
pub const MAX_KDF_ITERATIONS: u64 = 64;
pub const MAX_KDF_PARALLELISM: u32 = 16;
```

`1024 * 1024` KiB (1 GiB) is a large multiple of any memory cost a real
KeePass client sets by default (KeePass 2.x's own Argon2id default is
tens of MiB; even a deliberately hardened personal database rarely
exceeds a few hundred MiB) while still bounding the worst case a crafted
file can force this process to allocate. `MAX_KDF_ITERATIONS`/
`MAX_KDF_PARALLELISM` are generous over any real default for the same
reason. `Argon2 version` (`V`) must be exactly `0x13` — both KDF crates
already hard-require this (`Argon2Error::UnsupportedVersion` otherwise),
so no separate check is needed; a file declaring the older `0x10` is
rejected, a real but narrow compatibility gap recorded in §8.6.

### 8.3 Key derivation, HMAC keys, and the block stream

```text
password_hash    = SHA-256(password)                     (keyfiles: out of scope, §8.6)
composite_key    = SHA-256(password_hash)
derived_key      = Argon2d|Argon2id(composite_key, S, I, memory_kib, P, tag_len=32)  (§8.2)
encryption_key   = SHA-256(MainSeed || derived_key)
hmac_key_base    = SHA-512(MainSeed || derived_key || 0x01)
block_hmac_key(N)= SHA-512(N as u64 LE || hmac_key_base)
```

`password` is `Zeroizing<Vec<u8>>` end to end; `password_hash`,
`composite_key`, `derived_key`, `encryption_key`, and `hmac_key_base`
are every one of them `Zeroizing` too — each is real key material, not
merely a value derived from one.

Block stream (the outer-decrypted body, before inner-header parsing):
each block is `[32-byte HMAC-SHA256][4-byte LE size N][N bytes data]`,
verified as `HMAC-SHA256(block_hmac_key(index), index_LE8 || N_LE4 ||
data)` — checked with `subtle`'s constant-time comparison (already a
transitive dependency via `hmac`, §8 preamble) before that block's data
is appended to the reassembled buffer. `index` starts at `0` and
increments per block; `N == 0` marks the terminal (empty) block and
ends the stream. A single failed block HMAC — including the header's
own block-`0xFFFF...FFFF` check in §8.1 step 3 — is
`ImportError::Adapter("wrong password or corrupt file".into())`: KDBX
gives no way to distinguish "wrong password" from "corrupted file" any
more precisely than this, and this crate does not invent one.

Inner stream cipher (ChaCha20 only, §8.6): `stream_key_hash =
SHA-512(InnerRandomStreamKey)`; `key = stream_key_hash[0..32]`; `nonce =
stream_key_hash[32..44]`. Every `Protected="True"` value's Base64-
decoded ciphertext is decrypted by the *same* running ChaCha20 keystream
in document order — this crate walks the parsed `XmlDocument` tree
depth-first in document order exactly once, generating exactly as much
keystream as the cumulative protected-ciphertext length seen so far
requires (bounded by `MAX_SOURCE_BYTES`, so one bounded keystream buffer
generated up front is simpler and no less safe than a streaming
byte-position tracker here).

### 8.4 Inner XML → `PortableRecord`

The decompressed inner body is parsed with `xml-parser::parse_xml`
(shared, depth-capped, already-audited — the same reuse-over-hand-roll
call §7 already makes for JSON and CSV, extended here to a third,
structurally distinct untrusted-input format). No new XML parser is
written by this crate.

KeePass's tree is `KeePassFile > Root > Group* > Entry*` (groups nest
groups; this crate walks every `Entry` at any depth, ignoring group
structure entirely — vault-pm has no folder/collection concept for
imported items either, matching §5.4's existing "folder assignment not
carried across" precedent). Each `Entry` holds `String` elements with a
`Key`/`Value` child pair, `Value` optionally carrying `Protected="True"`:

| KeePass `String/Key` | vault-pm outcome |
|---|---|
| `Title` | item title |
| `UserName` | `Login.username` |
| `Password` (always `Protected`) | `Login.password` |
| `URL` | `Login.url` |
| `Notes` | `Login.notes`, or the sole content of a `SecureNote` if `UserName`/`Password`/`URL` are all empty (KeePass has no distinct "secure note" entry type; an entry that is nothing but a title and notes is the closest real-world equivalent, the same convention other KDBX importers use) |
| `otp` (KeePass's own 2.x-native TOTP field; also recognizes the older `TOTP Seed`) | fed through this spec's existing, unmodified `decode_external_totp_field` (§5.3) — raw Base32 or `otpauth://` URI, identical handling to a Bitwarden/CSV TOTP field — producing a second, separate `TOTP_SEED_V1` item exactly like §5.1/§5.2 |
| any other `String/Key` (custom fields) | `custom_fields[key]`, `Protected` or not, up to `MAX_CUSTOM_FIELDS_PER_ENTRY` |

Binary attachments (`Value Ref="N"` referencing an inner-header `Binary`
entry) have no `PortableRecord` slot, matching §5.4's existing treatment
of Bitwarden attachment metadata — recorded here, not silently dropped.
KeePass's own "card"/"identity" entry types don't exist as a distinct
KDBX concept (unlike Bitwarden's `type` field) — every `Entry` is
structurally the same, so §5.1's Bitwarden `Card` mapping has no KDBX
analogue; a KeePass user's card-shaped entries import as plain logins or
notes depending on which standard fields they populated, which is
already the correct, lossless outcome given KDBX carries no card-typed
metadata to map from.

### 8.5 Not carried across (documented, not silently dropped)

- **Binary attachments** — no vault-pm-side destination, same as §5.4.
- **Group/folder structure** — every entry becomes a top-level item, no
  collections, matching `item add`'s own default (§4.2's "no merge,
  always new identity" reasoning applies identically: a KDBX entry has
  no vault-pm item ID to collide with).
- **`IconID`/custom icons, entry history (previous revisions), and
  KeePass's own "expires"/tags metadata** — vault-pm's `Login`/
  `SecureNote`/`Totp` records have no matching field.

### 8.6 Explicitly out of scope for this amendment

Each of these is a real, separable gap, not an oversight — recorded so
none is silently assumed unsupported without reason, matching this
spec's own §8/§9 precedent for the two prior deferrals:

- **KDBX3** (`import kdbx` on a KDBX3 file) — a materially different
  outer format (2-byte header field lengths, no HMAC block stream, gzip
  applied *before* the AES-CBC wrapper rather than after it). Major
  version `!= 4` is `ImportError::UnsupportedVersion(major as u32)`.
- **Legacy AES-KDF** — KDBX4 still permits the pre-Argon2 KDF (iterated
  AES-256-ECB keyed by a random seed) for backward compatibility, but
  every KeePass release since 2.35 (2019) defaults new databases to
  Argon2d/Argon2id, and this amendment's task is closing the common
  case, not the full historical matrix. A `$UUID` naming the AES-KDF UUID
  is `ImportError::Adapter("AES-KDF is not supported; re-save the
  database with Argon2d or Argon2id".into())`.
- **AES128-CBC and Twofish-CBC outer ciphers** — both legal `CipherID`
  values, both rare in practice (AES256-CBC is KeePass's own default;
  ChaCha20 is the only other cipher exposed in its UI). Rejected the
  same way as AES-KDF, by name, not silently misdecoded.
- **Salsa20 inner stream** — KDBX4 still permits it for the same
  backward-compatibility reason AES-KDF is permitted; ChaCha20 has been
  KDBX4's default inner stream since its introduction (§8 preamble). No
  `salsa20` crate exists anywhere in this workspace today (checked
  before assuming otherwise); adding one is real, separable follow-up
  work, the same class of gap §9 already documents for QR decoding.
- **Keyfiles and hardware-key (YubiKey challenge-response) composite-key
  components** — `composite_key` (§8.3) is specified generally as
  `SHA-256(password_hash || keyfile_hash || provider_response)`; this
  amendment implements the password-only case (`keyfile_hash` and
  `provider_response` both empty). A database protected by a keyfile or
  hardware key *and* a password fails closed with
  `ImportError::Adapter("password-only KDBX databases are supported;
  this database requires a keyfile or hardware key".into())` rather than
  silently deriving the wrong key from the password alone — detected by
  checking the composite key against the header integrity HMAC (§8.1
  step 3) and reporting this specific failure only when a *keyfile-only*
  database (no password at all) is what was attempted, which this
  amendment does not attempt in the first place (a password prompt is
  always shown; an empty password is passed through unchanged as
  `password_hash = SHA-256(b"")`, which is what a keyfile-only database
  actually expects, so this failure mode is simply "wrong master key" —
  no special-cased detection needed beyond §8.3's existing HMAC check).
- **`PublicCustomData`** (outer header field ID `12`, a plugin-storage
  `VariantDictionary`) — parsed enough to skip over (its TLV framing is
  identical to `KdfParameters`'s), never interpreted; no first-party
  KeePass plugin data has a vault-pm equivalent.

### 8.7 Passphrase, dispatch, and why this format cannot reuse `Importer::import`

Every other format in this spec reads plaintext bytes and needs no
secret to decode them, so `vault-import-export::Importer::import(&self,
input: &[u8])` (§2) never carries a passphrase parameter. KDBX cannot
implement that trait: decoding requires the database's own master
password. `vault-import-keepass` therefore exposes a free function
instead of an `Importer`:

```rust
pub fn decode(
    container: &[u8],
    password: &Zeroizing<Vec<u8>>,
) -> Result<Vec<PortableRecord>, ImportError>;
```

`vault-pm-cli` wires `Command::ImportKdbx` through a new
`import_kdbx` function, parallel to but not reusing `import_external`
end to end: it reads the source file through the same
`host.read_external_import_source` (same `MAX_EXTERNAL_IMPORT_SOURCE_BYTES`
ceiling every other format already uses, §8.8), then prompts for the
KDBX database's password through the same `host.read_import_passphrase()`
method `portable_import`/`portable_restore` already use for vault-pm's
own passphrase (VLT-PM18) — reused as-is because its contract ("prompt
for the passphrase protecting an externally-supplied encrypted blob,
return it zeroizing") is already exactly this job, not a vault-pm-
specific one, before calling `vault_import_keepass::decode`. From the
resulting `Vec<PortableRecord>` onward, `import_kdbx` calls the same
shared "map, authenticate, create, aggregate" loop `import_external`
already implements (§4) — factored into a shared
`create_items_from_portable_records` helper both functions call, so the
per-record `prepare_item_create`/`context.complete`/`context.fail`
ceremony (§4, §4.1, §4.2) is written once, not duplicated for a fourth
time.

### 8.8 Threat model addendum

§7's adversary (VLT-PM00 §7.1 adversary 6, "malicious imported data")
gains one new capability against this format specifically: **a crafted
KDBX file can try to force excessive memory allocation via its own KDF
parameters before any password is checked** (§8.2) — the property no
prior format in this spec has, addressed by `MAX_KDF_MEMORY_KIB`/
`MAX_KDF_ITERATIONS`/`MAX_KDF_PARALLELISM`, checked first, always.
Everything else §7 already establishes carries over unchanged: the
16 MiB `MAX_EXTERNAL_IMPORT_SOURCE_BYTES` ceiling (shared across every
format, §8.7) bounds the whole container before any parsing begins —
including its gzip-compressed body, so this crate also caps the
*decompressed* size (`MAX_DECOMPRESSED_BYTES`, generous over any real
database at this container-size ceiling) before trusting `deflate`'s
output, the same "bound the amplification, not just the input" posture
§7 already states for JSON; the inner XML inherits `xml-parser`'s
existing depth cap against a deeply-nested adversarial document, the
same protection §7 credits `json-parser` with for Bitwarden; every HMAC
comparison (block stream, header integrity) is constant-time via
`subtle`, already a transitive dependency (§8 preamble); no imported
field — including a crafted `Title`/`UserName` containing a fake syslog
line or ANSI escape — ever reaches CLI stdout/stderr or an audit event,
identical to §6's existing guarantee, verified the same way (grep
captured output and durable audit rows for fixture plaintext, VLT-PM18
§9's style, §10 gate 6).

A wrong password and a corrupted file are indistinguishable by
construction (§8.3) — both report as one `Adapter` error, mapped by
`vault-pm-cli` to the `invalid` exit class, never revealing which of the
two occurred (a KDBX file's HMAC check gives an attacker probing for
"which password is closer to correct" no signal beyond pass/fail either
way, so this crate adds no new oracle by collapsing the two cases).

### 8.9 Test fixtures: why this crate carries its own encoder

Unlike the Bitwarden and CSV adapters — where a hand-written fixture
file is trivially "well-formed" because those formats are plaintext —
proving this decoder correct needs a genuinely valid, correctly-
encrypted KDBX4 file, and no real KeePass installation exists in this
environment to produce one from a GUI. `vault-import-keepass`'s test
suite therefore includes a `#[cfg(test)]`-only encoder implementing
§8.1–§8.3 in the forward direction (compose the header, derive the same
keys, HMAC-frame the same blocks, ChaCha20-obfuscate the same protected
values) — the same "generate fixtures in-house from an existing
first-party encoder" pattern §9 already commits to for future QR-image
fixtures via `qr-code`'s own encoder, applied here from the start
because no external reference tool is available at all. Every field the
encoder writes is read back by the real `decode` function it is testing
against, so a bug shared between "how this crate writes a header" and
"how this crate reads one" is a real risk this suite alone cannot rule
out — mitigated by cross-checking the encoder's output against the
byte-level field layout, cipher UUIDs, and key-derivation formulas
transcribed from independent third-party documentation of the real
KDBX4 format in §8.1–§8.3 (not derived from this crate's own decoder),
and by the RFC 4231/FIPS 180-4/RFC 9106 test vectors every one of the
underlying primitive crates (`hmac`, `sha256`, `sha512`, `argon2d`,
`argon2id`) already carries in its own test suite, which this crate
does not re-derive. A real KeePass-produced fixture remains valuable
follow-up verification the moment one is available in this environment,
recorded here rather than silently assumed equivalent to the in-house
one.

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
together than it would return" judgment §8 originally made for KDBX
before Amendment 2 gave it that focused review on its own, for an
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
`FILE`, the same shape `import kdbx` had before Amendment 2. A follow-up
slice owns:
adding `rqrr` and `png` to a new crate (or extending
`vault-import-otpauth`), decoding the PNG header first to cap width ×
height before any full pixel decode, running `rqrr` detection/decoding to
recover the embedded string, and feeding that string through §5.5's
existing `otpauth://` label/query pipeline unchanged — plus real
QR-image test fixtures generated from `qr-code`'s own encoder, covering a
corrupt file, a non-QR image, an oversized image, and a QR code encoding
something other than an `otpauth://` URI.

## 10. Acceptance gates

1. `import bitwarden`/`import csv`/`import portable`/`import otpauth-uri`/
   `import kdbx` each parse exactly one source path and nothing else;
   `import otpauth-qr` parses the same shape but fails closed with the
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
10. A synthetically-constructed, well-formed KDBX4 fixture (built by this
    crate's own test-only encoder, §8.9 — no real KeePass installation is
    available in this environment) covering AES256-CBC and ChaCha20 outer
    ciphers, Argon2d and Argon2id KDFs, gzip-compressed and uncompressed
    bodies, and a mix of a plain login entry, a notes-only entry, an entry
    with an `otp` TOTP field, and an entry with unrecognized custom
    fields, round-trips through `vault_import_keepass::decode` to the
    expected `PortableRecord`s, and through the real `vault-pm-cli`
    executable to the expected created items, each independently
    reachable by `item show` after restart.
11. A wrong password and a byte-corrupted (but structurally well-formed)
    KDBX4 fixture both fail `import kdbx` with the invalid exit class and
    an identical, non-distinguishing error message (§8.3) — proven by a
    test asserting the two error strings are equal, not merely both
    present.
12. `MAX_KDF_MEMORY_KIB`/`MAX_KDF_ITERATIONS`/`MAX_KDF_PARALLELISM` (§8.2)
    are each independently violated by exactly one in a crafted
    `KdfParameters` block and rejected before Argon2d/Argon2id is ever
    called — proven by a fake KDF hook (or a wall-clock/allocation bound
    in the test itself) showing the real KDF function is never invoked
    for these three cases.
13. A KDBX4 fixture naming AES-KDF, AES128-CBC, Twofish-CBC, Salsa20 (inner
    stream), or KDBX3's major version number is refused by name (§8.6),
    each with a distinct, specific error rather than a generic decode
    failure — proven by asserting on the returned `ImportError` variant
    or message per case, not merely that each one is an `Err`.
14. A block-stream HMAC failure at any block index (not only the first)
    and a header-integrity HMAC failure are both refused before any
    inner-header or XML parsing begins — proven by a fake `CliHost`
    assertion that `read_import_passphrase` is called exactly once (no
    retry loop) and no partial item is ever created.
15. `vault-import-keepass`'s own malformed-input matrix (§8.8, modeled on
    §7's existing per-adapter discipline) is green: oversized container,
    oversized declared block size inside the container-size ceiling,
    truncated header, truncated block stream, invalid UTF-8 inside a
    `String` XML value, a `Protected="True"` value whose Base64 fails to
    decode, and a decompressed-body size at and past
    `MAX_DECOMPRESSED_BYTES`.
