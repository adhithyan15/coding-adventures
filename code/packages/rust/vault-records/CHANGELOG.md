# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT02 (`code/specs/VLT02-vault-records.md`).
- `VaultRecord` trait — typed struct ↔ canonical CBOR payload.
- Wire envelope: `{"t": content_type, "d": payload}` — short keys
  for compactness; `"d"` precedes `"t"` deterministically per the
  canonical CBOR length-first / bytewise lex tiebreak.
- First-party record types covering both reference targets:
  - **Password manager**: `Login`, `SecureNote`, `Card`, `TotpSeed`.
  - **Machine secrets**: `ApiKey`, `DatabaseCredential`.
- Content type strings: `vault/login/v1`, `vault/note/v1`,
  `vault/card/v1`, `vault/totp/v1`, `vault/api-key/v1`,
  `vault/db-credential/v1`. Versioning via `vN` suffix; old clients
  see future versions as `Opaque` rather than crashing.
- `encode_record(&T)` / `decode_record(&[u8]) -> AnyRecord` /
  `decode_record_as::<T>(&[u8])` / `encode_opaque(...)`.
- Forward compatibility: unknown extra fields in a payload's CBOR
  map are tolerated by `decode_payload`; only required fields fail.
- All sensitive-byte-bearing types (`Login.password`, `Card.cvv`,
  `Card.number`, `TotpSeed.secret`, `ApiKey.token`,
  `DatabaseCredential.password`, etc.) implement `Zeroize` **and**
  `Drop` (Drop calls `zeroize`), so secrets wipe automatically on
  scope exit — callers don't have to remember to wrap in
  `Zeroizing<T>`. `AnyRecord` deliberately does NOT implement
  `Drop` (so callers can `match any { AnyRecord::Login(l) => l }`
  to move out a typed variant), but each typed variant's Drop
  fires automatically when the enum is dropped without
  move-destructuring; callers of `AnyRecord::Opaque` who consider
  the payload bytes sensitive should call `.zeroize()` explicitly.
- `VaultRecordError` typed enum: `Cbor`, `NotARecord`, `BadEnvelope`,
  `ContentTypeMismatch`, `SchemaMismatch`. `Display` strings are
  sourced exclusively from this crate's literals; the
  `ContentTypeMismatch` variant deliberately suppresses the
  attacker-controlled `actual` field from its Display output (the
  variant still carries it for callers that want to inspect via
  pattern matching).
- 21 unit tests covering: per-type round-trips for all six record
  types, `AnyRecord` dispatch, canonical idempotence (re-encoding a
  decoded record yields identical bytes; encoding the same struct
  twice yields identical bytes), content-type rejection, unknown-
  content-type opaque pass-through, opaque round-trip via
  `encode_opaque`, schema-mismatch rejection (missing required
  field, invalid month, invalid digit count), envelope rejection
  (top-level array, extra envelope field, `"t"` not text), forward
  compatibility (extra unknown payload fields tolerated), and
  Display-string-source-from-literals invariant.

### Security review

Round 1 found 1 MEDIUM + 2 LOW. All addressed:

- **MEDIUM** — typed records implemented `Zeroize` but not `Drop`,
  so plaintext secrets were not wiped on scope exit unless callers
  wrapped in `Zeroizing<T>`. **Fixed:** added `impl Drop` to all
  six typed records, each delegating to `self.zeroize()`. Wiping
  is now automatic.
- **LOW** — `Vec::clear()` on `Login.urls` and `ApiKey.scopes`
  wiped string contents but left the `Vec`'s own backing
  allocation in place. **Fixed:** replaced `clear()` with
  `Vec::new()`, which drops the backing allocation.
- **LOW** — `AnyRecord::Opaque` payload bytes were not
  zeroized. **Fixed:** added `Zeroize` impl on `AnyRecord` that
  wipes Opaque variant; documented that AnyRecord intentionally
  does NOT have `Drop` (because it would block
  move-destructuring) and instructed callers to call `.zeroize()`
  explicitly if Opaque bytes are sensitive.

Round 2 review: SECURITY REVIEW PASSED — no vulnerabilities found,
no new issues introduced by the Drop impls (verified: no double-
free, no panic paths, no soundness issues with move-destructuring,
clone semantics correct).
