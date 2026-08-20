# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed

- **Caller-side `CborValue` trees are now wiped end to end.** `encode_record`
  and `decode_record`/`decode_record_as` (via the shared `split_envelope`
  helper) and `encode_opaque` each build an owned `CborValue` tree that, for
  all seven record kinds, *is* somebody's plaintext — `encode_payload` clones
  every field into fresh `CborValue::Text`/`Bytes` leaves, and the typed
  `decode_payload`s clone fields back out of a tree the codec already fully
  materialised. The typed records themselves already wipe on drop (`Login`
  and its five siblings implement `Zeroize` + `Drop`), but this intermediate
  tree did not: it dropped through ordinary, non-wiping `Vec`/`String`/
  `CborValue` destructors, once per encode or decode.

  A prior security review's proposed quick fix — wipe only `try_encode`'s
  output buffer on its error path — was correctly rejected as incomplete:
  the caller's own tree is built and dropped regardless of whether that
  encode call succeeds, and `canonical-cbor` itself cannot own the fix
  (it is deliberately zero-dependency; see CBR01's "Non-goals"). The real
  fix is here instead: a new `zeroize_cbor_value` recursively wipes every
  `Text`/`Bytes` leaf through any nesting of `Array`/`Map`/`Tag` (its match
  has no wildcard arm, so a future `CborValue` variant fails to compile here
  until the new arm is added), and a new local `SecretCborValue` wrapper —
  `Zeroizing<CborValue>` in everything but name, since `CborValue` cannot
  itself implement the sibling crate's `Zeroize` trait without violating
  Rust's orphan rule — calls it from `Drop`, so the wipe runs on every exit
  path (success, an early `?`-propagated error, or a panic unwind), not only
  the ones a hand-placed wipe call would remember to cover. See VLT-PM05
  §13.6 for the full design account and CBR01's "Non-goals" for why this
  could not live inside canonical-cbor.

- `decode_record`'s opaque arm no longer re-encodes the payload it has just
  decoded, which closes the most severe defect this codec had: an opaque record
  that could be **read but not read back**.

  The arm produced `AnyRecord::Opaque::payload_bytes` by calling `try_encode` on
  the decoded payload. `decode` has no input-length bound and `try_encode` has
  `MAX_ENCODED_SIZE` (1 MiB), so any opaque payload between 1 MiB and this
  layer's 16 MiB plaintext gate decoded cleanly and then failed
  `EncodeTooLarge`. That is a different class of failure from the encode-side
  ones fixed before it: this decode runs underneath `decode_item_revision`,
  which runs during vault *open*, so the error reached `open_active_vault` and
  denied the entire vault — permanently, with no session from which to delete
  the record and no export to escape through. One record synced from a peer
  with a larger framing budget was enough. Recovery meant deleting local state
  or hand-editing the store outside the product.

  The re-encode was never needed. The payload arrived through the strict
  canonical decoder, which enforces every rule the encoder applies, so its bytes
  already are the one legal spelling of that value — byte for byte what
  `try_encode` would have returned. The arm now takes that sub-slice, using the
  span reported by canonical-CBOR's new `decode_map_spanned`, and slicing a
  range the parser itself measured cannot fail on any input that decoded at all.

  Unchanged by design: `encode_opaque` and `encode_record` still refuse an
  oversized record with `EncodeTooLarge`. The band of records that are legal to
  hold and illegal to write is not narrowed — the refusal simply now costs one
  record rather than the vault. Also unchanged: `encode_opaque(content_type,
  payload_bytes)` still reproduces the original wire bytes exactly, so the
  `decode_record` → `encode_opaque` round trip remains the identity, which is
  the property VLT-PM39's authored opaque merge validates its input against.

  Tests: an oversized record that decodes and whose payload equals the wire
  sub-slice; sliced-and-re-encoded agreement across eight payload lengths
  spanning every header width; span correctness across nine payload *shapes*, so
  an off-by-one cannot hide in a value type the size tests never build; and, in
  `vault-pm-application`, a 1.5 MiB record delivered through a shared object
  store that leaves the vault openable and the item deletable, with a
  sub-ceiling control alongside. Specified in VLT02 *Decoding never re-encodes*
  and VLT-PM05 §13.3.

### Changed

- `decode_record` and `decode_record_as` now peel the `{t, d}` envelope through
  one shared byte-level routine rather than each destructuring an
  already-decoded value, so there is a single answer to which bytes are a record
  and the typed and opaque paths cannot drift into accepting different inputs.
  Accepted inputs and returned errors are unchanged.

- **Breaking:** `encode_record` now returns `Result<Vec<u8>, VaultRecordError>`
  rather than `Vec<u8>`. See *Fixed* below for why the encode is genuinely
  fallible. Callers that know their record is small can `.expect(…)`, but the
  only in-tree caller — `vault-pm-application`'s `encode_any_record` — maps the
  failure to a closed `BoundExceeded`.

### Fixed

- `encode_record` now reports a record too large to encode through a new
  `Result`, instead of panicking. This is the last of the three panicking
  encodes this crate had; the other two — `encode_opaque` and `decode_record`'s
  opaque arm — were closed just before it, and needed no signature change.
  The two ceilings either side of this layer disagree:
  vault-pm's `MAX_PLAINTEXT_BYTES` is 16 MiB while canonical-CBOR's
  `MAX_ENCODED_SIZE` is 1 MiB, so records between them are legal to hold and
  legal to decode but illegal to encode. A peer device with a larger framing
  budget can author a `Login` with a 2 MiB password that seals, syncs, and
  decodes here without complaint, and every later command that re-serialises
  it — `item edit`, all seven authored conflict merges, `conflict choose`,
  `history restore`, `export` — used to abort the process. Because the record
  stays in the store, that abort repeated on every subsequent command against
  the same vault: a local denial of service from one synced record. This
  applies to all six first-party types, not just `Login`.
  Known limitation, recorded in VLT02 *Encoding is fallible*: a refused encode
  does not wipe what it had already serialised — `try_encode` drops its partial
  output buffer unzeroized. That is not new to the `Result` (the panicking
  wrapper reached the same state, since `encode` is `try_encode(…).expect(…)`
  and the buffer was already dropped before the panic); what is new is that the
  process survives to keep running with that heap freed but unwiped. Wiping
  only that buffer would mislead rather than protect, because the identical
  plaintext sits in the `CborValue` tree the caller still owns, and
  canonical-CBOR's value types implement neither `Zeroize` nor a wiping `Drop`
  on any path. Closing it properly means making canonical-CBOR zeroize-aware
  end to end, which adds a dependency to a deliberately zero-dependency
  foundational crate, so it is tracked as its own change.
- `encode_opaque` now reports an envelope that is one level too deep to encode
  through its existing `Result`, instead of panicking. Wrapping a payload in the
  `{t, d}` envelope costs one level of nesting, so a payload nested exactly as
  deep as the decoder permits used to abort the process on the way back out.
  That was unreachable while every payload came off the wire, and becomes
  reachable as soon as a caller authors one.
- `decode_record` now reports an opaque payload it cannot re-encode through its
  existing `Result` too, instead of panicking. Its unknown-content-type arm
  re-encodes the decoded payload, and a caller's own framing bound need not be
  the encoder's, so a record from a peer with a larger frame budget could decode
  and then abort the process on the way back out. Failing closed there loses one
  record; panicking loses the process, including every later command against the
  same store.

### Added

- `VaultRecordKind`, `VaultRecordSummary`, `AnyRecord::kind()`, and
  `AnyRecord::summary()` for value-redacted record inventory data covering
  record family, secret-field counts, optional/list shape, lease/expiry flags,
  and opaque content/payload lengths.

### Security

- Replaced derived `Debug` on all secret-bearing record types and `AnyRecord`
  with closed value-redacted implementations. Opaque content types and payload
  bytes are also suppressed.
- Added a closed `VaultRecordError::Debug` implementation so diagnostic and
  assertion formatting cannot emit attacker-controlled content types.

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
