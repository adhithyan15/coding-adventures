# Changelog

All notable changes to this package are documented here.

## [0.70.0] - 2026-08-25

### Changed

- **Fixed backlog item #23 (VLT-PM47 §9 acceptance gate 3), CI-cost only, no
  invariant change.** `attachment::tests::empty_and_oversized_inputs_are_
  refused_without_a_panic` used to materialize a real
  16 MiB (`MAX_ATTACHMENT_BYTES`) buffer and run it through the full
  two-layer seal — about 1.5s in a debug build, in a package with 250+
  otherwise-microsecond unit tests. Split into
  `the_boundary_is_exact_in_mechanism_and_in_arithmetic` (a 3-chunk
  representative call proving the offset-stepping loop's boundary mechanism —
  exact multiples produce no stray trailing chunk — plus a direct check that
  `expected_chunk_count(MAX_ATTACHMENT_BYTES)` equals `MAX_ATTACHMENT_CHUNKS`,
  the same arithmetic `chunk_attachment` itself derives its chunk count from)
  and `the_full_scale_ceiling_still_chunks_exactly_at_max_attachment_chunks`
  (`#[ignore]`d — the original full-scale assertion, kept as a periodic/manual
  check rather than deleted). The `empty`/`oversized` rejection assertions are
  unchanged and stay cheap (the bound is checked before a single chunk is
  sealed). Measured: this file's `attachment::` test group, 1.52s to 0.28s
  wall.

## [0.69.0] - 2026-08-25

### Added

- **`export_portable_with_passphrase_best_effort` / `audited_export_
  portable_with_passphrase_best_effort`: an explicit, opt-in export that
  excludes a poisoned item instead of failing the whole export** (VLT-PM05
  §13.9). Closes the backlog item VLT-PM05 §13.2 opened and deliberately
  left unrepaired: "one poisoned record blocks the whole export... partial
  export is a format and verification change, not a local one." This is
  that format change.

  - **The bug, confirmed with a real reproduction.**
    `export_portable_with_passphrase` walks every current candidate of
    every item and calls `encode_item_revision` on each one; the first
    `BoundExceeded` (an oversized first-party, opaque, or quarantined
    record, or a conflict with one such candidate — §13.1/§13.3/§13.5/
    §13.8) propagates through the surrounding `?` and denies the *whole*
    export, not just the one poisoned item. `portable_export_best_effort_
    excludes_a_synced_oversized_item_and_keeps_the_rest` pins this
    directly: a real peer-synced 1.5 MiB oversized-opaque item, alongside
    one perfectly ordinary item, denies `Strict` export entirely before
    this change and after it.

  - **Options weighed and rejected before this one.** Silently skipping the
    poisoned item unconditionally (the shape §13.3 used for vault open) was
    rejected as a *default* or *unconditional* behavior: unlike open,
    export has a genuine completeness tradeoff a silent default cannot make
    safely on the operator's behalf — an artifact missing an item, with no
    record of that fact, is indistinguishable from a smaller vault that
    never had it. A persistent "quarantine" mutation that replaces the
    poisoned payload with an inert catalog marker was also rejected: it
    requires authenticating and publishing a new commit *before* the
    operator can get a backup at all, which is strictly more friction than
    the escape hatch this codebase already shipped (`item delete`, then
    `export`), and it duplicates that working mechanism across a large new
    surface (search, list, six conflict-merge preconditions, audit
    rendering) for no benefit over it.

  - **The fix.** `PortableExportCompletenessV1::{Strict, BestEffort}` picks
    the behavior; `Strict` is the unmodified default every existing caller
    still gets (`export_portable_with_passphrase`'s signature and return
    type — `PortableExportArtifactV1` — are unchanged). `BestEffort` is
    reached only through the two new, additive methods, and returns
    `PortableExportOutcomeV1 { artifact, excluded_item_ids }`. An item is
    excluded *whole*, never as a partial candidate set: if any one current
    candidate cannot be re-encoded, the item's entire scratch buffer of
    already-encoded sibling candidates is zeroized and discarded rather
    than partially folded in — keeping only the small candidate of a mixed
    conflict would silently hand the target an unconflicted item where the
    source actually had one, a correctness defect worse than the omission
    itself. `portable_export_best_effort_excludes_a_whole_mixed_conflict_
    not_just_its_oversized_half` pins this directly.

  - **The wire format.** The plaintext snapshot gains one new field,
    `6: excluded_item_ids Array<Bytes[16]>`, emitted only when non-empty —
    an ordinary export, or a `BestEffort` export that excluded nothing,
    stays the exact five-field shape every prior version of this module
    already writes and reads, so no existing backup's readability changes
    and no `VERSION` bump was needed. Field 6 is outside `snapshot_hash`'s
    own domain (the same shape `candidate_count` already has) and is
    instead structurally cross-checked on read: bounded length
    (`MAX_CATALOG_ENTRIES`), no duplicate id, and disjoint from every item
    id actually present among `entries` — a producer must never claim one
    item both present and excluded.
    `excluded_item_ids_field_is_optional_and_self_consistency_checked`
    pins all three checks directly against a real, hand-edited artifact.

  - **`OpenedPortableSnapshotV1::excluded_item_count()`** exposes the
    aggregate on the import side, the same shape
    `attachment_bearing_item_count` (VLT-PM47 §8.3) already uses — a count,
    not the identities, because those belong to the *source* vault the
    importing operator does not otherwise have visibility into.
    `PortableExportOutcomeV1::excluded_item_ids()` is deliberately wider on
    the *export* side: every id it returns is already visible to that same
    operator through this exact vault's own `item list`.

  - **CLI.** `vault-pm export FILE [--best-effort]`
    (`vault-pm-cli`'s CHANGELOG has the corresponding entry).

## [0.68.0] - 2026-08-24

### Fixed

- **`LocalSecretV1::encode`/`decode` and `encode_item_revision` now wipe
  their own intermediate `CborValue` trees** (VLT-PM05 §13.7), closing a
  HIGH pre-existing gap in this crate's own codec — the same root pattern
  §13.6 fixed in `vault-records`, but explicitly left un-swept in this
  crate by that earlier PR.

  - `LocalSecretV1::encode` built `CborValue::Map(vec![... bytes(&self.
    authority_seed) ...])` — fresh heap copies of the vault's entire local
    root key hierarchy, the Ed25519 authority seed and both device seeds —
    and handed it to the *panicking* `encode` wrapper. The map dropped,
    unwiped, on every return. `decode` used plain `take_fixed` for the
    three seed fields, which converts the decoded `Vec<u8>` into a
    `[u8; 32]` and drops the vector; `value_fields`'s existing wipe-on-
    structural-failure gate never covered this, because decoding
    succeeded.

  - `encode_item_revision` folds `encode_live`'s output — a `CborValue`
    tree whose eighth field is the item's own record bytes (a `Login`'s
    password, a `Card`'s PAN and CVV, a `TotpSeed`'s secret, ...) — into
    its own outer map and calls `try_encode` directly. That call's
    `BoundExceeded` failure for an oversized record is a routine, expected
    outcome here, not a rare edge case, so the failure path this left open
    is one real vaults reach in ordinary use, not only a theoretical one.
    Every caller already wraps the returned `Vec<u8>` in `Zeroizing`, but
    that only ever protected the final bytes, never the intermediate tree
    they were built from.

  - Both instances were found by sweeping every `CborValue::Map(vec![...])`
    construction in this crate for the same pattern. `state.rs`,
    `export.rs`, and `vault-pm-format` were checked and confirmed clear —
    each only ever handles already-sealed ciphertext, public identifiers,
    or KDF/AEAD parameters, never plaintext secret material.
    `decode_live`/`decode_item_revision` (this gap's decode side) were
    already correctly hardened and needed no change.

  - Fixed by reusing machinery this module already had for the identical
    `AttachmentManifestV1` case (`zeroize_cbor_secrets`, `take_secret_fixed`)
    rather than introducing a new type or reaching into `vault-records`'s
    `SecretCborValue` — which is `struct`-private to that crate's own
    module and was never part of its public surface. Both encoders now
    build into a local, call `try_encode` (never the panicking `encode`),
    and wipe with `zeroize_cbor_secrets` regardless of outcome before the
    local drops. `LocalSecretV1::decode` now takes its three seeds through
    `take_secret_fixed` instead of `take_fixed`; `vault_id`/`device_id`
    stay on plain `take_fixed`, matching `AttachmentManifestV1`'s existing
    secret/identifier distinction.

  - **Round-1 security review finding, fixed before push:** the first draft
    of `decode_fields` dereferenced each `take_secret_fixed` result into a
    plain `[u8; 32]` local immediately on the line that took it. A plain
    array's `Drop` is a no-op, so if an earlier seed (say, `authority_seed`)
    had already been taken and dereferenced by the time a *later* field (5
    or 6) failed to decode, that early `?` return left the earlier seed's
    already-extracted copy sitting unwiped — one field later than
    `take_secret_fixed`'s own per-call guarantee reaches. Fixed by keeping
    every seed `Zeroizing`-wrapped until the function's one, final,
    infallible `Ok(Self { ... })` literal, so an early return anywhere
    before that point still wipes every seed already taken via its own
    still-live wrapper.

  - New tests: `local_secret_encode_wipes_its_own_scaffolding`,
    `local_secret_decode_wipes_its_own_scaffolding_on_success`,
    `local_secret_decode_wipes_an_earlier_seed_when_a_later_field_fails`
    (pins the round-1 finding above),
    `encode_item_revision_wipes_the_records_plaintext_on_success`, and
    `encode_item_revision_wipes_the_records_plaintext_on_bound_exceeded`,
    each watching a new `#[cfg(test)]` process-wide call counter on
    `zeroize_cbor_secrets` move forward around a real production call —
    the same shape as `vault-records`'s `SECRET_CBOR_VALUE_DROPS` — so the
    tests prove the real code path reached the wipe, not just that the
    wipe function is correct in isolation.

## [0.67.0] - 2026-08-19

### Fixed

- **`MAX_CATALOG_ENTRIES` is now derived from `MAX_ENCODED_SIZE`, not an
  eyeballed `100,000`** (VLT-PM05 §13.4), closing a MEDIUM pre-existing bug
  where an ordinary vault — no hostile peer needed — froze every mutation
  including `item delete` once its catalog crossed roughly nineteen thousand
  items, well inside the old, fictional 100,000-entry admission ceiling.

  - The old bound described what `validate_catalog` admitted, not what
    `CatalogV1::encode` could carry: canonical-CBOR's 1 MiB `MAX_ENCODED_SIZE`
    refused to re-emit a catalog long before entry count reached 100,000, so
    the number the catalog advertised was never the number it could hold.
    Because catalog entries are never removed — a deleted item becomes a
    tombstone *entry*, the same size on the wire, not a smaller one — a vault
    that crossed the real ceiling had no recourse from inside the product at
    all: every later mutation that had to re-encode the full catalog failed
    the same way, forever.

  - `CATALOG_ENTRY_BYTES` (55, the exact cost of one item id plus one
    candidate revision id in canonical CBOR) and `CATALOG_FRAME_OVERHEAD_BYTES`
    are now `const`s pinned against the real encoder, and
    `MAX_ENCODABLE_CATALOG_ENTRIES = (MAX_ENCODED_SIZE - overhead) /
    CATALOG_ENTRY_BYTES = 19,064` is a *proven* ceiling: no encode of a
    `CatalogV1`, on this device or any other honouring the wire format, can
    ever exceed it.

  - Admission and decode now use two different bounds, deliberately.
    `validate_catalog` (and therefore every local mutation that builds a new
    catalog) applies the tight, margined `MAX_CATALOG_ENTRIES = 19,064 -
    1,000 = 18,064`, refusing the item that would make the catalog
    unencodable *before* that catalog is ever built. `CatalogV1::decode` and
    `materialize_current_catalog`'s cross-head merge instead apply the
    looser, unmargined `MAX_ENCODABLE_CATALOG_ENTRIES`, so a catalog this
    device's past self (or any honest peer) could have legitimately produced
    under the old admission check stays openable, while a catalog no honest
    encoder could ever have produced — a peer hand-crafting bytes past its
    own claimed framing budget — is still refused. Applying the tight bound
    uniformly would have repeated, at the catalog level, the open-denies-the-
    whole-vault mistake VLT-PM05 §13.3 already corrected once for individual
    records.

  - `CatalogV1::decode` no longer routes through `CatalogV1::new` for its
    final construction, because that would silently re-apply the tight
    admission bound after the intentionally looser decode-time check. The
    per-record encode is factored into `encode_catalog_entries`, reused by
    both `CatalogV1::encode` and the tests that construct catalog bytes past
    the admission ceiling to model an honest-but-looser peer.

  - Tests: `codec.rs` pins the byte-cost derivation and both bounds directly
    against the real encoder — cheap, exact, no crypto needed. `open.rs`
    reproduces the bug end to end against a real, unlocked vault:
    `a_synced_catalog_at_the_proven_ceiling_opens` and
    `a_synced_catalog_past_the_proven_ceiling_denies_open` bracket 19,064
    exactly through `open_active_vault` against a peer-authored catalog
    delivered the way a real sync would (many small publications, since one
    commit cannot itself carry more than `MAX_ADDED_OBJECTS` new objects);
    `a_catalog_at_the_admission_ceiling_can_still_be_deleted_from` reproduces
    the named symptom directly — a real `delete_current_item` call succeeds
    on a catalog sitting at this device's own admission ceiling.

  - `MAX_CATALOG_ENTRIES` dropped from a nominal 100,000 to 18,064. No
    honestly-produced catalog could ever have exceeded roughly 19,064 entries
    regardless of that nominal figure, so this is not a new restriction on
    anything that could actually have existed — it is the admission ceiling
    catching up to the encode ceiling that was always the real one.

  - **Caught in security review before merge:** applying the tight
    `MAX_CATALOG_ENTRIES` unconditionally to *every* catalog rebuild —
    including a delete, edit, or restore that does not add an entry —
    reopened this exact bug at a narrower band: a catalog synced from a
    peer, or grown under this device's own pre-fix admission policy, with an
    entry count anywhere in `(MAX_CATALOG_ENTRIES,
    MAX_ENCODABLE_CATALOG_ENTRIES]` would decode and open fine, then fail
    *every* subsequent mutation — delete included — because rebuilding that
    same, unchanged entry count still ran into the tight admission ceiling.
    `CatalogV1::new_for_mutation` fixes this: it only applies the tight
    ceiling when a mutation's entry count actually grows past what it
    already was, and the looser, proven `MAX_ENCODABLE_CATALOG_ENTRIES`
    otherwise. `CatalogV1::encode` no longer re-runs `validate_catalog`
    either, for the same reason — that re-check does not know whether the
    catalog was built as new growth or as a non-growing mutation, and
    applying the tight bound there unconditionally would undo
    `new_for_mutation`'s fix at the encode step. Regression tests:
    `mutation_of_an_above_admission_catalog_succeeds_when_it_does_not_grow`
    (`codec.rs`) and `a_catalog_above_the_admission_ceiling_can_still_be_
    deleted_from` (`open.rs`, a real `delete_current_item` call against a
    real synced vault whose catalog already exceeds the admission ceiling).

## [0.66.0] - 2026-08-18

### Added

- **Chunked encrypted attachments**, the last of `VLT-PM00` §23 item 11.
  Specified by `VLT-PM47-cli-attachments.md`.

  - A new `attachment` module splits one plaintext into fixed 64 KiB pieces,
    seals each with VLT14's chunk AEAD under a per-attachment key, and
    reassembles them again. `ATTACHMENT_CHUNK_BYTES`, `MAX_ATTACHMENT_BYTES`,
    `MAX_ATTACHMENT_CHUNKS`, and `MAX_ATTACHMENT_NAME_BYTES` are the bounds,
    and three of the four are *derived* rather than written: the chunk size is
    VLT14's, the attachment ceiling is `MAX_PLAINTEXT_BYTES` itself, and the
    chunk cap is the quotient. A second literal is a second thing that can
    drift, which is the failure this whole design is about.

  - The chunk size is chosen against `canonical-cbor`'s 1 MiB
    `MAX_ENCODED_SIZE`, not against the 16 MiB frame bound. §13.1 of
    `VLT-PM05-application.md` records why that is the ceiling that binds: the
    gap between the two is a range of values legal to hold, legal to decode,
    and illegal to re-encode, and every encode in it used to abort the
    process. One sealed chunk object encodes to about 65,600 bytes; two
    compile-time assertions hold the sixteen-fold margin under
    `MAX_ENCODED_SIZE` and hold the chunk cap plus four under
    `MAX_PUBLICATION_OBJECTS`, so neither relation can drift silently.

  - `MAX_ATTACHMENT_BYTES` equals `MAX_PLAINTEXT_BYTES` deliberately, and the
    equality is the argument: an attachment can never be larger than a
    plaintext this product already accepts in one sealed frame, so attachments
    do not become a bigger door than records.

  - `ObjectKind` gains `AttachmentManifest` (6) and `AttachmentChunk` (7).
    `AttachmentManifestV1` carries one attachment's name, plaintext length,
    content hash, VLT14 key, and ordered chunk object references; the item
    revision's live-state map gains an optional tenth field mapping each
    retained `AttachmentId` to its manifest object. That field is emitted only
    when the attachment set is non-empty, so a revision without attachments
    encodes exactly the nine keys it encoded before this change, byte for
    byte, and every revision written earlier still decodes. Storing manifests
    inline instead would have cost up to 570 KiB of revision and put the
    revision encode back in the range §13.1 is about.

  - The attachment identity *is* the VLT14 blob id — one 128-bit value with
    both meanings — so the chunk AEAD's associated data binds each chunk to
    the attachment identity a person sees rather than to a private alias, and
    there is no state in which the two disagree.

  - `UnlockedVaultV1::audited_attach_attachment`, `audited_list_attachments`,
    and `audited_export_attachment`. Attaching publishes `ItemUpdate` inside
    the mutation's own commit, with failed preconditions publishing a failed
    `ItemUpdate` first; listing and exporting publish `ItemRead`. Export runs
    `VLT-PM25`'s disclosure ceremony unchanged — the `InteractiveReveal`
    intent, the same outcome table, and publish-before-release — with the
    reassembled plaintext held in a local binding across publication so a
    publication failure drops and wipes it without the caller ever seeing it.

  - `AttachmentRandomnessV1` and `attachment_random_bytes`, the
    variable-length entropy block one attach requires. Its size depends on the
    chunk count, the same situation `PortableImportRandomnessV1` is in, and it
    is validated at construction so a short or long block is an `InvalidInput`
    before the vault is touched rather than a partition that reads the wrong
    offsets.

  - Every peer-authorable malformation is a closed error and none is a panic:
    an oversized declared length is refused before it can size a buffer, and
    reordered, cross-blob, promoted-final, tampered, and truncated chunks are
    refused by VLT14's associated data and tag. The reassembled length and
    SHA-256 are re-derived, because VLT14 v1 commits `0` rather than the real
    total in chunk associated data and names verifying it as the caller's duty.

### Security

- **`validate_attachment_name` now rejects Unicode Cf and Zl/Zp**, not only
  Cc. It runs on decode as well as ingest, so the name it is checking may have
  been authored by a synchronising peer, and `char::is_control` covers only
  category Cc: a name carrying U+202E RIGHT-TO-LEFT OVERRIDE passed, and
  rendered as a *different* name in the listing an operator uses to choose
  which attachment to export. The enumeration covers the soft hyphen, the
  Arabic and Syriac formatting marks, the Mongolian vowel separator, the
  zero-width and bidirectional controls, the interlinear annotation controls,
  and the deprecated tag block. Ordinary non-ASCII names are unaffected, and
  the doc says plainly that an enumeration is a statement about the code points
  in it rather than a categorical guarantee — the total gate is the CLI's
  escape at the render site, and this is defence in depth on top of it.

- **`AttachmentManifestV1::encode` returns `Zeroizing<Vec<u8>>`** and wipes its
  own intermediate CBOR tree, and the manifest's key is decoded through a new
  helper that wipes the decoder's copy. The manifest is the one value in this
  module whose *encoded output* is a secret; `bytes(...)` copied the key into a
  heap buffer the encoder then freed unwiped, and `take_fixed` did the same on
  the way back. VLT02 already records that the CBOR value types are not
  zeroize-aware and that making them so is a change to a deliberately
  zero-dependency crate; this closes the half that does not need it.

- **`UnlockedVaultV1::attachment_bearing_item_count`** and
  **`OpenedPortableSnapshotV1::attachment_bearing_item_count`**, the aggregates
  the CLI uses to say that a portable export, import, or restore left
  attachments behind. A backup an operator believes carries their recovery
  codes and does not is worse than no backup, and before this the export
  ceremony reported success and the restore ceremony reported *verified* — a
  true statement about what it compared, which reads to a person as "everything
  came back".

- **`value_fields` wipes a rejected map before dropping it.** It is the gate
  every object in this crate decodes through, and two of those objects are made
  of key material — the attachment manifest's key, and `LocalSecretV1`'s
  authority and device seeds. Its four failure paths, all reachable from a
  peer-authored value, dropped the entries decoded so far and left them in
  freed heap. Both halves of the work in progress are now wiped on every exit,
  and `zeroize_cbor_secrets` reaches text as well as byte strings so the
  guarantee does not depend on which variant a future field happens to use.

- **`AttachmentManifestV1::decode` wipes on every exit, not just the
  successful one.** Three checks run before the key is taken — wrong version,
  wrong kind, malformed attachment id — and each dropped the field map, and the
  key with it, unwiped. The body is split out and funnelled through one place
  that wipes, so a check added tomorrow is covered without anyone remembering
  to cover it.

### Changed

- **`prepare_item_publication` gained a content-object parameter** rather than
  a third copy of the commit-and-announce sequence. Sealed chunk and manifest
  frames are simply more objects in the same `PendingPublication` journal and
  the same commit, which is what makes an attachment write inherit
  `VLT-PM41`'s crash matrix and `VLT-PM42`'s recovery instead of needing a
  resumable-upload protocol beside them. An interrupted attach leaves only the
  unreachable immutable objects `VLT-PM00` §10.4 already describes.

- **Portable import now drops attachments.** `VLT-PM17`'s snapshot carries
  records and not blobs, so carrying an attachment *reference* across would
  produce an item claiming attachments no export in the target could ever
  find. `portable_semantic_root` normalises source and target through the same
  function, so restore verification compares the same closure on both sides
  and is unweakened.

- Randomness partitioning now works on slices throughout; the fixed-array
  helpers it duplicated were removed.

## [0.65.0] - 2026-08-18

### Changed

- **`SecretDisclosureIntentV1::Clipboard` now carries and enforces
  `confirmed`.** It previously authorized *unconditionally* and was reachable
  only from tests, which made it a trap rather than a policy: the first caller
  to reach for it would naturally be the first slice to implement `--copy`
  (`VLT-PM46-cli-clipboard.md`), and reaching for it would have silently
  deleted the application-layer confirmation gate while appearing to describe
  the delivery channel more accurately. A destination is not an authorization,
  and putting a secret somewhere every process in a session can read is not the
  disclosure that needs *less* consent than one on a private terminal.

  No shipped behaviour changes, because no production caller ever constructed
  the variant. VLT-PM46 §3.0 deliberately keeps `--copy` on
  `SecretDisclosureIntentV1::InteractiveReveal`: that contract's whole claim is
  that `--copy` runs the same ceremony as `--reveal` and differs only in the
  final output channel, and the audit event records the fact of an access
  rather than the door it left by.

## [0.64.0] - 2026-08-18

### Added

- **Current TOTP code computation**, the second of `VLT-PM00` §23 item 11.
  Specified by `VLT-PM45-cli-totp-code.md`.

  - `UnlockedVaultV1::audited_current_item_totp_code` reads the sole current
    live revision of a `TOTP_SEED_V1` item and returns the current RFC 6238
    code. It publishes the same `ItemRead` event, with the same
    `Denied`/`Failed`/`Succeeded` outcomes and the same publish-before-release
    ordering, as `audited_reveal_current_item_field` — because `VLT-PM15` §2
    already names "TOTP display" in its list of accesses. A six-digit code
    lives about thirty seconds and does not let its holder produce the next
    one, which makes its blast radius smaller than the seed's; it does not
    make the read less of a read.

  - The computation happens *inside* this crate, so the decoded seed never
    crosses the boundary. That is the whole reason the new `totp` module
    exists rather than the CLI reusing the existing seed reveal and computing
    for itself: a command whose purpose is to avoid showing anyone the seed
    should not materialize the seed in the process's outermost layer.

  - `TotpCodeV1` carries the zero-padded digits and the seconds they remain
    valid. It implements no `Debug`, `Display`, or `Clone` — a `Debug` derive
    would put a valid second factor into every future `{:?}` of anything
    containing one — and wipes the code on drop. The countdown is readable
    through an ordinary accessor because it is a function of the clock and the
    stored period, reproducible by anyone with a watch, and therefore safe to
    print to ordinary standard output.

  - The instant is a caller-supplied `code_time_ms`, deliberately separate from
    the `wall_time_ms` reserved for the audit event. `VLT-PM45` §4.1: an
    Argon2id unlock and a human typing `yes` sit between the pre-authentication
    reservation and the computation, so the reserved reading is stale by
    construction and a whole period is easily reachable in the gap. One reading
    used for both would routinely return the *previous* code — six digits,
    correct-looking, and rejected by the site.

  - Stored parameters this build cannot compute fail closed as `Unsupported`
    with a `Failed` event: an unrecognized or differently-spelled algorithm, a
    digit count the engine cannot render, a zero period, an empty secret. The
    codec does not validate these (only `VLT-PM29`'s CLI input boundary does),
    so a portable import can carry them. There is no fallback to SHA-1 and no
    clamped digit count, because six wrong digits are indistinguishable from
    six right ones until a login fails.

  - The RFC 6238 engine is `coding_adventures_vault_auth`, a new dependency,
    reused per `VLT-PM00` §6's reuse map rather than reimplemented here.

### Changed

- The current-item disclosure ceremony is now one generic code path shared by
  the secret-field and TOTP-code boundaries. The mapping from situation to
  audit outcome *is* the security property, and a second copy of it would be a
  second thing to keep correct — with "an access that happened and left no
  trace" as the failure mode.

## [0.63.0] - 2026-08-18

### Added

- **Passphrase rotation**, closing `VLT-PM00` §14.8's "password rotation
  rewraps the VRK without re-encrypting every item body" and §23 item 10b.
  Specified by `VLT-PM43-cli-passphrase-rotation.md`.

  The new `rotate` module is a workflow, not a new cryptographic capability.
  The passphrase already protected exactly one thing — the 32 bytes of
  `BootstrapV1.passphrase_root_wrap` that hold the vault root key — so a
  rotation is one Argon2id derivation, one AEAD open, and one AEAD seal on
  those 32 bytes, plus a re-signature by the unchanged vault authority.
  Nothing below the root key is read, opened, or rewritten.

  - `prepare_passphrase_rotation` is pure and performs no external write. It
    takes the current passphrase a second time, because an `UnlockedVaultV1`
    deliberately retains derived subkeys and not the root: rotation is the one
    operation that needs the root, so it is the one operation that pays for it,
    rather than every session holding it longer. It then proves the unwrapped
    root is the session's root by opening `ActiveStateV1::local_secret` with
    keys derived from it and requiring the identical owner secret — a binding
    check that compares no key bytes.
  - `commit_passphrase_rotation` and `recover_pending_rotation` perform the
    durable half through a new `LocalVaultStateV1::PendingRotation` journal.
  - `UnlockedVaultV1::rotate_passphrase`,
    `::audited_rotate_passphrase`, and
    `::record_audited_passphrase_rotation_host_failure` are the session
    boundaries, with the audited one publishing its event *before* the effect
    through the existing audit-only publication path.

- `BootstrapStore::supersede_generation`, a **required** trait method.
  Advancing the latest pointer is not enough to retire a passphrase: the
  superseded generation still wraps the *same, unchanged* root key under the
  *old* passphrase-derived key, so anyone who later obtained a copy of the
  state directory and the retired passphrase could open the vault through it.
  Implementations must refuse to remove the generation the latest pointer names
  — deleting the live bootstrap would leave a vault no passphrase opens — and
  must treat an already-absent record as success, because recovery replays the
  call.

- `PendingRotationV1` and the `LocalVaultStateV1::PendingRotation` variant
  (state tag 4). It holds the last stable owner state and the exact signed next
  bootstrap; the intended next state is *derived* rather than stored, so the
  journal cannot disagree with itself. `UnlockRecoveryV1` gains
  `RecoveredPendingRotation`, and both read-only projections report the new
  state as `recovery_required` without distinguishing which journal it is.

### Changed

- `VaultAccessV1::unlock_recovering_pending_publication` now finishes an
  interrupted *rotation* as well. Its name is kept because the contract is
  unchanged from a caller's point of view — finish whatever an interrupted
  process left, then open — but the two journals are finished differently, and
  the difference is the point: **a rotation roll-forward consumes no
  passphrase.** Every step after the journal is a pure function of the journal,
  so a person who types the pre-crash passphrase still gets their vault
  repaired, and then an honest `AuthenticationFailed` from the open that
  follows, which names the state the vault is actually in.

  Rolling *back* was considered and rejected: it would have to decide without a
  passphrase whether the person still knows the old one, un-install an
  immutable generation record, and explain why someone who just confirmed a new
  passphrase twice ended up with the old one.

### Security

- The KEK derived from the *retired* passphrase is wiped inside
  `unwrap_root_key` before the new one is derived, and the root key exists only
  between that unwrap and the re-wrap. The prepared rotation value carries the
  next bootstrap's public signed bytes and no live secret, which is why it is
  safe to journal.
- A rotation is floored at the vault's existing Argon2id memory and iteration
  cost. It may raise the cost and may never lower it: without the floor, the
  same parameter path that lets someone rotating on a faster machine get
  stronger parameters would also let a caller hand them a *weaker* credential
  than the one they already had, in the ceremony they ran to improve their
  security. The shipped CLI passes a fixed production policy; this guards
  embedders and any future host that calibrates at run time.
- `generate_keypair` returns the Ed25519 secret **by value**, and `[u8; 64]` is
  `Copy`, so wrapping it in `Zeroizing` silently left the original array on the
  stack un-wiped. The authority secret is now bound mutably and the original
  wiped once the owned copy exists.
- The rotation roll-forward **never** undoes anything, and the reason is now
  stated where it can be checked. A partial roll-back — withdrawing the journal
  when the store refused the install and still served the generation the
  rotation meant to retire — was written, reviewed, and removed. It reads like
  proof that nothing happened and it is not: `put_generation` installs the
  generation record *before* it advances the latest pointer, and, decisively,
  the observation reads the bootstrap store while the withdrawal writes the
  local state store, with nothing making those atomic together. A second host
  finishing the rotation inside that window would find the first committing
  `Active(old)` on top of an already-retired generation — a vault pinning a
  bootstrap the provider no longer has, openable by *no* passphrase, with no
  journal left to say so. What makes concurrent and repeated recovery safe is
  convergence: every host performs the same writes in the same order. A store
  that has moved somewhere the journal did not put it is a tampered or forked
  provider, fails closed as `IntegrityFailure` with the journal intact, and
  leaves the file-level backup restore VLT-PM41 §5 proves available.

## [0.62.0] - 2026-08-18

### Added

- `VaultAccessV1::unlock_recovering_pending_publication` and the closed
  `UnlockRecoveryV1` outcome it returns (`AlreadyActive` or
  `RecoveredPendingPublication`). This is the second half of
  `VLT-PM05-application.md` §8 step 2 — "resume a prepared initialization or
  **pending publication** when present" — which had been specified from the
  beginning and never wired to anything.

  `recover_pending_publication` was already here, already correct, and already
  covered: it replays an exact durable journal idempotently and advances the
  owner state only after the repository returns the heads that journal
  expected. Nothing called it. `VLT-PM41-cli-crash-fault-matrix.md` §8 measured
  what that cost by killing a real process: a vault interrupted mid-publication
  was intact, exactly journalled, correctly diagnosed as `recovery_required` by
  both read-only projections — and refused by every command that opened it,
  because `open_active_vault` accepts only `Active`. That refusal surfaced as
  the invalid-input class, so a person whose machine died mid-write was told
  their *command* was wrong, indefinitely.

  The new entry point replays the journal and then performs the ordinary strict
  `open_active_vault` against the repaired durable state, discarding the
  `ActiveStateV1` the recovery returned. That is deliberate: every check a
  later, uninvolved process would perform — bootstrap signature, root-wrap
  authentication, seed-to-pinned-identity reproduction, repository open against
  non-empty pins — runs on the repaired bytes. A repair that produced a session
  only the repairing process could reproduce would be worth less than no repair.

  It costs one extra Argon2id derivation, and only when a repair happens: the
  recovery and the reopen each consume a passphrase by value. Since `Zeroizing`
  implements neither `Clone` nor `Debug` on purpose, the recovering branch
  constructs its duplicate by name; both copies are wiped on drop, including
  while unwinding.

### Unchanged, deliberately

- `VaultAccessV1::unlock` still accepts only `Active` and still refuses a
  `PendingPublication` with the invalid-input class, so a host that wants a
  crash refused rather than repaired keeps exactly that. The recovering unlock
  is a second named door, not a change of behavior behind the existing one.
- `open_active_vault` and `recover_pending_publication` are untouched. This
  release composes them; it does not modify either.
- A `PreparedInit` owner state is still refused by the recovering unlock. It
  belongs to initialization, which is the only caller that knows a vault is
  being created.

## [0.61.2] - 2026-08-17

### Fixed

- `open_active_vault` no longer fails because of one item's payload size. The
  repair itself is in `vault-records` — `decode_record`'s opaque arm no longer
  re-encodes the payload it just decoded — but the damage landed here, and this
  is where it is pinned.

  An opaque payload between canonical-CBOR's 1 MiB encode ceiling and this
  crate's 16 MiB plaintext gate decoded fine and then failed `EncodeTooLarge`
  on the re-encode. The error rose through `decode_item_revision` (as
  `IntegrityFailure`), `read_candidate`, and `materialize_current_catalog` to
  deny `open_active_vault`. Since `materialize_current_catalog` reads every
  candidate of every item, one poisoned revision anywhere in the catalog denied
  the whole vault — and because the failure is *during* open, there was no
  session to delete the item from, no export to evacuate through, and no
  ceremony of any kind. Recovery meant deleting local state or editing the store
  by hand outside the product. This was the residual exposure recorded in
  VLT-PM05 §13.2 as worse than anything the two preceding changes fixed: those
  turned process aborts into closed errors on *write* paths, where a closed
  error costs one record; on the open path a closed error is the loss.

  The invariant this crate may now rely on is stated in the new VLT-PM05 §13.3:
  any revision whose plaintext is within `MAX_PLAINTEXT_BYTES` and which decodes
  materialises into the current catalog, whatever the encoder would say about
  re-emitting it. It is narrow on purpose. Open still fails closed on everything
  it should — corrupt frames, broken pins, failed signatures, a catalog past
  `MAX_CATALOG_ENTRIES` — and it is about *size*, not about every per-item
  failure: a peer-authored first-party record whose payload does not match its
  schema still yields `SchemaMismatch` → `IntegrityFailure` and still denies
  open. That residual is pre-existing, unchanged here, and deliberately deferred:
  the size failure could be removed because the re-encode was never needed,
  whereas a schema-invalid payload has to be *represented*, which means deciding
  what a partly-unreadable item looks like to search, list, show, conflict
  resolution, export, and restore.

- `encode_any_record`'s opaque arm now routes through `map_record_encode_error`
  like the six typed arms, instead of folding every `encode_opaque` failure into
  `IntegrityFailure`. The wildcard was defensible while an oversized opaque
  record could never be materialised — the arm's only reachable failure was
  genuinely an integrity one — but the fix above makes such a record openable,
  so the arm can now see `EncodeTooLarge` from stored bytes, most visibly on
  `export`. Reporting that as `IntegrityFailure` tells an operator their store is
  corrupt, which invites destructive recovery, when the store is intact and the
  remedy is to delete one large item. That remedy is the escape hatch this
  change exists to restore, so it must not be described in the vocabulary of
  corruption — and `doctor`, which never calls `encode_any_record`, would have
  reported the same vault `Healthy`, so the operator got two contradictory
  answers. VLT-PM39's dependency is unaffected: a payload that is not valid CBOR
  yields a non-size `CborError`, which still maps to `IntegrityFailure`, and the
  existing fixture pins it — `encode_opaque` decodes the stored payload before it
  encodes the envelope, so the two size variants are unreachable unless the
  payload was already valid canonical CBOR, and a corrupt-but-large payload
  cannot be laundered into a size error. Found by the round-1 security review of
  this change.

  One user-visible consequence: `vault-pm` exits 2 (`InvalidCommand`) rather than
  6 (`Integrity`) when a command is refused for an oversized opaque record. That
  is the exit code the six typed arms have produced since the preceding change,
  so this makes the seventh consistent with them rather than introducing a new
  behaviour. Stderr remains fixed and payload-blind.

  The escape hatch of §13.2 now covers the opaque case on the same terms as the
  first-party one, and both halves are pinned by test rather than inferred:
  `a_synced_oversized_opaque_record_leaves_the_vault_openable` and
  `a_synced_oversized_opaque_item_can_be_deleted` drive a real 1.5 MiB opaque
  record delivered the only way such a record can arrive — published straight
  into the shared object store by a device with a larger framing budget, since
  the local mutation path stages the whole publication journal in local state
  and will not write a 1.5 MiB frame. A sub-ceiling control
  (`a_synced_opaque_record_under_the_encode_ceiling_opens`) uses the identical
  fixture and delivery, so a failure is attributable to the size band rather
  than to how the record was authored, and both regression tests were confirmed
  red against the unfixed opaque arm before the fix landed.

## [0.61.1] - 2026-08-17

### Fixed

- Report every serialisation that exceeds canonical-CBOR's 1 MiB
  `MAX_ENCODED_SIZE` as a closed `BoundExceeded` instead of aborting the
  process. This crate's own `MAX_PLAINTEXT_BYTES` is 16 MiB and its local-state
  bound 32 MiB, both looser than the codec ceiling beneath them, so six encodes
  were reachable with values the encoder refuses to emit:
  - `encode_any_record` — a first-party record (all six schemas) larger than
    1 MiB, which a peer device with a larger framing budget can author and sync.
  - `encode_item_revision` — the revision framing around that record. Reachable
    even when the record itself fits, because the item id, schema tag,
    timestamps, favourite register, three observed sets, and causal-parent list
    are added on top of the record bytes.
  - `CatalogV1::encode` — needs no hostile peer at all. `validate_catalog`
    admits 100,000 entries at roughly sixty bytes each, so an ordinary vault
    crossed the codec ceiling somewhere below twenty thousand items, and the
    catalog is re-encoded by *every* mutation.
  - The portable export snapshot and artifact encodes — also no hostile peer:
    a vault holding two ~600 KiB entries produced an export larger than 1 MiB
    and aborted `vault-pm export`.
  - The portable *import* re-encode of entries decoded from someone else's
    artifact, the same shape as `decode_record`'s opaque arm.
  - `encode_signed_object`, whose wrapped exact bytes are caller-sized.

  `BoundExceeded` rather than `IntegrityFailure` because the cause is a fixed
  serialisation bound being exceeded, not a corrupt store. The pre-existing
  opaque arm of `encode_any_record` keeps `IntegrityFailure`, which VLT-PM39
  relies on. `LocalSecretV1::encode` and the export AAD header stay infallible
  and are documented as provably fixed-size.

- Keep size faults and integrity faults distinct when mapping a CBOR encode
  failure. The nine call sites above previously used a `|_|` wildcard that
  collapsed every `CborError` into `BoundExceeded`, including `DuplicateMapKey`
  — a canonicality fault meaning the value would encode to bytes the strict
  decoder rejects as ambiguous. Unreachable today, because every map this crate
  builds has distinct literal keys, but a trap for any later refactor: a real
  integrity fault would have been reported as the benign size error an operator
  is told to fix by shrinking a record. `map_encode_error` now matches
  `EncodeTooLarge`/`EncodeTooDeep` explicitly and sends everything else to
  `IntegrityFailure`.

### Changed

- Track `vault-records`' `encode_record` signature change to `Result`.

### Known limitations

The change bounds the blast radius of an unencodable record; it does not stop
one entering the catalog. Recorded in VLT-PM05 section 13.2 and tracked as
follow-on work rather than widened into this change:

- **Ingest is still ungated.** `decode_item_revision` gates on the 16 MiB
  plaintext bound and canonical-CBOR caps decode depth but not length, so a
  peer with a larger framing budget can still hand this device a record that
  decodes and can never be re-encoded. Matching the ingest gate to
  `MAX_ENCODED_SIZE` changes what the product accepts, and done naively turns a
  partly-degraded vault into an unopenable one — a worse failure than the one
  it prevents — so it needs its own spec.
- **One unencodable record blocks the whole export.** The export walks every
  current candidate and propagates the first failure, so a single bad record
  denies evacuating the entire vault. Skip-and-report is a format and
  verification change, not a local one: `candidate_count` and the signed
  `snapshot_hash` assert completeness, and VLT-PM19/VLT-PM20
  restore-verification depends on that assertion.
- **A narrow escape hatch.** Deleting the offending item works, because a
  tombstone revision carries only the item id and a timestamp, so the `Live`
  arm that reaches `encode_any_record` is never taken — now pinned by
  `deleting_an_oversized_item_stays_possible`. It covers exactly the
  single-candidate, first-party-record case. It does *not* cover an item that
  is also conflicted (deletion asserts one live candidate and otherwise returns
  `ConflictRequired`), nor an oversized *opaque* record, which is undecodable
  rather than merely unwritable — `decode_record`'s opaque arm re-encodes the
  payload, so the failure lands during vault open and no session is ever
  established to delete from. Both are pre-existing, both need a peer with a
  larger framing budget to reach, and both are tracked separately.

## [0.61.0] - 2026-08-17

### Added

- Add an opaque audited authored opaque-record conflict merge preparation with a
  wipe-on-drop replacement payload line, exact-current opaque base, and
  application-owned closed hexadecimal and CBOR-canonicality validation. The
  payload is decoded behind the audited boundary and accepted only when its
  canonical re-encoding inside the record envelope reproduces the typed bytes.
  The record's content type is inherited from the base rather than authored,
  because an item's schema is immutable across its history, so this is the one
  merge where a single field is authored and everything else carries over.

### Security

- Keep the payload line, decoded payload bytes, envelope round-trip
  intermediates, and prior candidate documents inside application ownership,
  decode into a wipe-on-drop buffer that cannot reallocate, wipe the partial
  nibble and the round-tripped record on every exit including the refused
  non-opaque one, publish host and closed form-validation failures before
  returning them, and publish success atomically with the all-current-parent
  revision.
- Refuse a payload that round trips as a first-party record, so the one merge
  command that validates no fields cannot be used to author a login, note,
  card, API key, database credential, or TOTP seed around its own closed rules.

## [0.60.0] - 2026-08-17

### Added

- Add an opaque audited authored TOTP conflict merge preparation with a
  complete wipe-on-drop replacement form, exact-current TOTP base, and
  application-owned closed seed, algorithm, digit, and period validation. The
  Base32 seed line is decoded behind the audited boundary and accepted only in
  its canonical unpadded spelling. `TOTP_SEED_V1` has no issuance-only field,
  so every schema field is authored and nothing is inherited from the base
  candidate.

### Security

- Keep the seed line, decoded seed bytes, and prior candidate documents inside
  application ownership, decode into a wipe-on-drop buffer that cannot
  reallocate and wipe the partial bit accumulator on every exit, publish host
  and closed form-validation failures before returning them, and publish
  success atomically with the all-current-parent revision.

## [0.59.0] - 2026-08-17

### Added

- Add an opaque audited authored database-credential conflict merge preparation
  with a complete wipe-on-drop replacement form, exact-current
  database-credential base, and application-owned closed engine and port
  validation. An authored merge result is always a static credential: the
  merged record carries no lease ID and no lease expiry.

### Security

- Keep the password, connection metadata, and prior candidate documents inside
  application ownership, publish host and closed form-validation failures
  before returning them, and publish success atomically with the
  all-current-parent revision.

## [0.58.0] - 2026-08-17

### Added

- Add an opaque audited authored API-key conflict merge preparation with a
  complete wipe-on-drop replacement form, exact-current API-key base, and
  application-owned closed scope-line and expiry validation.

### Security

- Keep the token, scope line, and prior candidate documents inside application
  ownership, publish host and closed form-validation failures before returning
  them, and publish success atomically with the all-current-parent revision.

## [0.57.0] - 2026-08-12

### Added

- Add an opaque audited authored payment-card conflict merge preparation with
  a complete wipe-on-drop replacement form and exact-current card base.

### Security

- Keep PAN/CVV and prior candidate documents inside application ownership,
  publish host and closed form-validation failures before returning them, and
  publish success atomically with the all-current-parent revision.

## [0.56.0] - 2026-08-12

### Added

- Add an opaque audited authored secure-note conflict merge preparation with a
  complete title/body input and shared exact-current base validation.

### Security

- Publish wrong-schema and host failures before release and publish success
  atomically with an all-current-parent revision that retains base non-form
  metadata without returning the prior note body to the host.

## [0.55.0] - 2026-08-12

### Added

- Add an opaque audited preparation for authored login conflict merges using
  one exact current live login as the non-form metadata base.

### Security

- Publish item-scoped `ItemConflictMerge` failures for invalid bases, host
  failures, and invalid authored forms before returning them, while publishing
  success atomically with an all-current-parent merged revision.
- Enforce the closed sixteen-URL ceiling inside the application-owned login
  replacement builder as well as at the terminal host.

## [0.54.0] - 2026-08-12

### Added

- Add an audited item-bound secret disclosure boundary that accepts only an
  exact member of the authenticated current conflict set.

### Security

- Publish denial without candidate traversal, reject unconflicted and
  historical noncandidate revisions, and bind the exact revision only after
  current membership is authenticated.

## [0.53.0] - 2026-08-11

### Added

- Add audited typed disclosure of optional login notes.

### Changed

- Make login replacement own and replace the complete ordered URL list and
  optional notes while retaining immutable identity and unrelated metadata.
- Accept existing multi-URL logins during edit preparation.

## [0.52.0] - 2026-08-11

### Added

- Add item-bound audited disclosure of one typed field from the sole current
  live candidate without returning its revision capability to the host.

### Security

- Publish denied confirmation, missing/conflicted item, field mismatch, and
  successful exact-revision outcomes before releasing a non-printable owned
  secret.

## [0.51.0] - 2026-08-11

### Added

- Add item-bound audited choose-candidate conflict resolution.

### Security

- Publish missing, unconflicted, and wrong-selector failures before their
  closed errors, while binding successful selected revisions atomically to the
  all-current-parent resolution mutation.

## [0.50.0] - 2026-08-11

### Added

- Add a distinct audit-first generation-zero boundary with an exact owned
  randomness block and encrypted `VaultInitialize` genesis event.

### Security

- Bind the signed initialization event into the initial commit, retry journal,
  and intended active owner state so new product vaults are auditable before
  the first post-initialization operation.
- Retain the legacy pre-audit preparation boundary only for migration,
  recovery, and fail-closed compatibility verification.

## [0.49.0] - 2026-08-11

### Added

- Add a session-consuming host-failure boundary for portable restore
  verification input and artifact-opening failures.

### Security

- Publish a failed itemless `PortableRestoreVerify` event before CLI
  composition can expose a post-unlock source-read, prompt, or open failure.

## [0.48.0] - 2026-08-11

### Added

- Add an opaque portable-restore expectation over canonical normalized
  candidate-group semantics and source identity sets.
- Add independently reopened target verification with aggregate-only results.

### Security

- Compare exact live/tombstone values, timestamps, schemas, CRDT state,
  candidate grouping, parent removal, and cross-vault identity disjointness.
- Publish dedicated succeeded or failed `PortableRestoreVerify` events before
  releasing the aggregate proof or integrity failure.

## [0.47.0] - 2026-08-11

### Added

- Add audited portable-import host-failure and target-validation boundaries.
- Allow an empty target to retain audit-only attempts before its first atomic
  import.

### Security

- Publish failed `PortableImport` events before returning host, artifact, or
  target errors, while keeping the success event atomic with re-identified
  candidates and the new target catalog.

## [0.46.0] - 2026-08-11

### Added

- Add a session-consuming boundary for an authenticated portable-export host
  input failure.

### Security

- Publish a failed itemless `PortableExport` event before CLI composition can
  expose a post-unlock distinct-passphrase prompt failure.

## [0.45.0] - 2026-08-11

### Added

- Add a session-consuming boundary that records an authenticated host-side
  item-create failure against its already-reserved item identity.

### Security

- Publish the failed `ItemCreate` event and its fresh trace through the
  audit-only journal before the caller can expose a prompt failure.

## [0.44.0] - 2026-08-11

### Added

- Add bounded newest-first projections for complete verified operation-audit
  history and exact trace lookup.
- Expose only the trace, counter, action, outcome, optional item/revision
  selectors, and advisory time required by an explicit audit surface.

### Security

- Publish one successful `AuditRead` event before re-verifying and returning
  either history result, so the authorizing access can appear in its own view
  without recursive self-auditing.
- Reject invalid list bounds without writing, and keep stable identities out of
  default debug output.

## [0.43.0] - 2026-08-11

### Added

- Add an opaque login-edit preparation that retains the current revision and
  secret-bearing document entirely inside the application boundary.
- Add audited preparation, completion, and host-failure paths for missing,
  conflicted, unsupported, invalid-input, prompt, and entropy failures.

### Security

- Publish successful `ItemUpdate` events atomically with replacement revisions
  and publish closed edit failures before the host can expose their outcome.
- Preserve notes and immutable metadata without releasing the existing login
  document or optimistic revision capability to CLI orchestration.

## [0.42.0] - 2026-08-11

### Added

- Add item-bound bounded-history restore boundaries that validate the stable
  item ID, historical revision, tombstone state, current conflict state, and
  same-revision guard without exposing a history projection to the host.
- Add an audited restore variant that durably records failed authenticated
  selection outcomes before returning their closed error.

### Security

- Publish successful restore events atomically with their new live revision;
  failed events bind the selected revision only after repository history proves
  that it belongs to the attempted item.

## [0.41.0] - 2026-08-11

### Added

- Add one application-selected current-item delete boundary that keeps the
  exact optimistic revision capability out of transitional CLI hosts.
- Add an audited variant that returns successful deletion only after its
  atomic mutation event is durable and returns missing, tombstoned, or
  conflicted failures only after a failed `ItemDelete` event is durable.

### Security

- Permit audit-only item-mutation events only for failed or denied outcomes;
  successful item mutations must still bind their result revision atomically
  to the causal mutation publication.

## [0.40.0] - 2026-08-11

### Added

- Expose a payload-free unlocked-session predicate so transitional hosts can
  require audited access whenever a durable audit epoch already exists.

## [0.39.0] - 2026-08-11

### Added

- Add a production session-consuming audit-epoch activation boundary for
  explicit migration of a pre-audit vault.

### Security

- Publish the one permitted successful `AuditEpochStart` through the durable
  audit-only journal before returning the next owner state.
- Reject repeat activation without changing owner state, and prove exact
  pending-journal recovery after ambiguous provider success.

## [0.38.0] - 2026-08-11

### Added

- Add session-consuming audited access to exact current revision capabilities,
  whole secret-bearing revisions, and schema-specific secret fields.
- Represent refused interactive and unsafe non-interactive disclosure
  ceremonies as durable `Denied` item-read events.

### Security

- Withhold every revision capability, owned document, revealed secret, and
  closed operation failure until its signed event and next owner state are
  durable.
- Bind successful reads and authorized field-selection failures to the exact
  item and reachable revision without traversing revisions for denied requests.

## [0.37.0] - 2026-08-11

### Added

- Extend the session-consuming audited access boundary to complete repository
  verification, coarse unlocked diagnostics, and encrypted portable export.
- Preserve the existing aggregate verification report, identity-free doctor
  result, and owned export-secret handling behind the common durable-result
  wrapper.

### Security

- Withhold verification, diagnosis, and encrypted export results until their
  signed access events and next owner states are durable.
- Record invalid portable-export inputs as failed authenticated attempts, while
  audit-publication failure supersedes and withholds either result or error.

## [0.36.0] - 2026-08-11

### Added

- Extend the session-consuming audited access boundary to one-item redacted
  reads, redacted search, bounded redacted history, and current conflict
  candidate inspection.
- Bind successful item reads to their exact selected live revision and map
  missing or tombstoned items to a closed audited `NotFound` result.

### Security

- Route list, show, search, history, and conflict inspection through one common
  publish-before-release completion path.
- Record invalid queries, invalid history bounds, missing items, and
  unconflicted candidate requests as failed authenticated attempts; audit
  publication failure still supersedes and withholds every operation result.

## [0.35.0] - 2026-08-10

### Added

- Add a session-consuming audited item-list boundary and a redacted result
  wrapper that carries both the durable next owner state and the original
  closed operation result.
- Add an exact wipe-on-drop entropy container for one trace, encrypted audit
  event, and audit-only repository commit.

### Security

- Refuse audited access on pre-epoch vaults, and publish either a succeeded or
  failed `ItemList` event before releasing any redacted list or authenticated
  conflict result.
- Withhold the underlying operation result when event publication fails, while
  retaining the exact pending journal for recovery after ambiguous provider
  success.

## [0.34.0] - 2026-08-10

### Added

- Add a dedicated crash-resumable audit-only publication journal that advances
  the repository commit, device counter, and encrypted event head while reusing
  the exact active catalog root.

### Security

- Permit catalog reuse only when the publication supplies a distinct new audit
  event, and require pending-state validation to bind an omitted catalog frame
  to the exact prior active catalog.
- Prove canonical pending-state replay after an ambiguous provider success and
  complete verification of epoch and non-mutating access events without
  manufacturing replacement catalog ciphertext.

## [0.33.0] - 2026-08-10

### Added

- Extend complete unlocked audit verification through the durable encrypted
  operation-event head and report the verified event count without identities.

### Security

- Decrypt every linked event, verify its certified-device signature, and bind
  its vault, device counter, exact basis heads, commit timestamp, event-object
  membership, selected revision, and mutation result to reachable signed
  commits.
- Reject cycles, gaps, skipped durable heads, wrong signers, wrong basis heads,
  missing results, and non-genesis chain roots. Pre-audit vaults remain
  backward-compatible and explicitly report zero verified events.

## [0.32.0] - 2026-08-10

### Added

- Add encrypted, device-signed audit events to item create, update, delete,
  restore, conflict resolution/merge, and portable-import publications whenever
  the durable audit epoch is active.
- Reserve an independent 32-byte trace ID and encrypted-event randomness in
  every mutation entropy container so hosts cannot silently omit audit entropy.

### Security

- Bind each successful mutation event to the same device counter, exact parent
  heads, affected item, selected revision where applicable, resulting revision,
  prior per-device event, and advisory time as its repository commit.
- Publish the event in the same crash-resumable journal and advance the durable
  audit head only with the mutation commit. Pre-audit vault behavior remains
  compatible, and public activation stays deferred until access paths also fail
  closed.

## [0.31.0] - 2026-08-10

### Added

- Add an optional owner-private per-device audit-event head to active state and
  pending publication journals, with backward-compatible decoding of pre-audit
  state records.

### Security

- Require every publication after audit activation to advance to a distinct
  newly published event object, preventing a journal from silently skipping or
  reusing the durable audit chain head.
- Keep event identities redacted from diagnostics; this dormant state boundary
  does not activate auditing or claim current command enforcement by itself.

## [0.30.0] - 2026-08-10

### Added

- Add the stable authenticated object kind and strict canonical wrapper for a
  signed VLT-PM15 operation-audit event.

### Security

- Encrypt operation events under a distinct object-kind AAD domain and require
  callers to verify the decoded event against its certified device key; this
  slice does not yet claim repository publication or CLI enforcement.

## [0.29.0] - 2026-08-10

### Added

- Add an authenticated current-item revision capability for optimistic host
  mutations without putting revision identities in ordinary redacted views.

### Security

- Return no revision for absent/tombstoned items and fail closed on current
  conflicts, so a CLI edit cannot select an arbitrary candidate.

## [0.28.0] - 2026-08-10

### Added

- Add one atomic cross-vault import that consumes an authenticated opaque
  portable snapshot into an untouched empty generation-zero target.
- Add exact count-derived host-CSPRNG sizing and a non-printable wipe-on-drop
  entropy container for new item identities plus every revision, catalog, and
  commit frame.

### Security

- Reject source-vault reuse, mutated or non-empty targets, stale pins,
  source/target identity collisions, malformed candidate bindings, and imports
  exceeding the repository's 4,096-object publication bound before activation.
- Re-identify every item and revision under independent target encryption while
  preserving complete validated live, tombstone, conflict, CRDT, timestamp, and
  secret state; source causal identities are deliberately not copied.
- Reuse the exact write-ahead pending-publication journal for all-or-nothing
  crash recovery, consume the opaque snapshot on every path, and independently
  reopen the target to prove source/target identity separation and restored
  content equality.

## [0.27.0] - 2026-08-10

### Added

- Add strict authenticated opening for an untrusted canonical portable export,
  returning one opaque non-cloneable secret-bearing snapshot with aggregate
  item and candidate counts only.
- Add an explicit host-approved Argon2id memory, iteration, and lane ceiling
  checked before artifact-controlled KDF work.

### Security

- Bound the artifact before canonical decode, authenticate the complete
  header-bound ciphertext before plaintext parsing, and make wrong credentials
  indistinguishable from valid-shape tag tampering.
- Verify the exact snapshot count/hash, embedded authority-signed bootstrap,
  strict source item/revision ordering, per-item candidate bounds, canonical
  revisions, and entry/document item binding before returning success.
- Keep opening free of target-vault or provider writes; expose no source
  identity or document accessors, redact diagnostics, and wipe all intermediate
  secret-bearing buffers and CBOR trees on every return path.

## [0.26.0] - 2026-08-10

### Added

- Add canonical authenticated passphrase-encrypted portable export from an
  unlocked session, retaining every current live, tombstone, and conflict
  candidate in deterministic source item/revision order.
- Bind the exact active signed bootstrap, candidate count, and
  domain-separated snapshot hash inside the encrypted 512 MiB-bounded
  plaintext; authenticate all artifact header parameters as AEAD associated
  data.

### Security

- Require a separately collected non-empty bounded passphrase, caller-validated
  Argon2id policy, and fresh host-supplied salt and XChaCha20 nonce rather than
  implicitly reusing the live VRK or unlock credential.
- Exclude owner-private state, private seeds, provider credentials, local pins,
  journals, and search data; redact public diagnostics and wipe temporary
  passphrase, key, plaintext, revision, and hash-preimage buffers on drop.
- Return only encrypted bytes and leave path choice, overwrite policy, and
  persistence authority to the host. Authenticated import and cross-vault
  re-identification remain a separate follow-up workflow.

## [0.25.0] - 2026-08-10

### Added

- Add a read-only locked/unlocked `doctor` workflow with one closed,
  identity-free coarse report.
- Distinguish initialization required, recovery required, local-state,
  bootstrap, and repository unavailability, unsupported versions or suites,
  authentication required, integrity failure, and authenticated health.

### Security

- Require exact durable active-state and signed-bootstrap binding before an
  unlocked complete audit can report healthy.
- Never repair owner state, accept new pins, expose provider detail, or return
  counts and identities through doctor diagnostics.

## [0.24.0] - 2026-08-10

### Added

- Add unlocked `audit_verify` with a secret-free aggregate report covering
  announcements, commits, catalogs, item revisions, and item identities.

### Security

- Repeat repository discovery from durable pins, require an exact local
  counter/catalog/certificate anchor, traverse complete bounded ancestry, and
  authenticate/decrypt every distinct reachable catalog and referenced item
  revision before returning a successful report.
- Return no partial report on failure and keep ordinary diagnostics free of
  vault, device, item, revision, object, locator, and provider identities.

## [0.23.0] - 2026-08-10

### Added

- Add user-authored merged-document conflict resolution for a current item
  with two or more retained candidates.

### Security

- Require at least one live candidate and preserve the schema and creation time
  of every retained live candidate before any local write.
- Publish the complete current candidate set as causal parents, consume the
  secret-bearing document and session on every path, and retain all immutable
  candidate bytes as reachable history.

## [0.22.0] - 2026-08-10

### Added

- Add a closed five-state `VaultStatusV1` projection for absent, prepared,
  locked, unlocked, and recovery-required lifecycle states.
- Add authenticated item, candidate, and conflicted-item counts while unlocked.

### Security

- Locked status strictly decodes only bounded owner-private state and never
  opens the bootstrap or repository.
- Omit all counts outside an authenticated live session, keep diagnostics free
  of identities and provider details, and translate store failures through the
  closed application error taxonomy.

## [0.21.0] - 2026-08-10

### Added

- Add the stable payload-free `Locked` application error class.
- Add a compact `VaultAccessV1` lifecycle boundary with explicit locked and
  unlocked states, in-place authenticated unlock, session access, and lock.

### Security

- Failed unlock attempts leave the lifecycle object locked and immediately
  drop temporary key material.
- Locking synchronously replaces and drops the live session, wiping its keys,
  local secrets, decrypted records, and search projection before returning.
- Lifecycle diagnostics reveal only locked or unlocked state and omit the
  locator, vault identity, item metadata, and live-session counts.

## [0.20.0] - 2026-08-10

### Added

- Add schema-specific selection for every first-party secret field, returning
  one owned binary-or-UTF-8 `RevealedSecretV1` value.
- Add explicit clipboard, confirmed interactive reveal, and unsafe
  non-interactive reveal policy inputs for host adapters.

### Security

- Validate disclosure policy before repository traversal; non-interactive
  reveal requires both explicit unsafe opt-in and a host-emitted warning.
- Reject wrong-schema and opaque field selection, and expose secret bytes only
  through a non-printable, non-cloneable, wipe-on-drop allocation.

## [0.19.0] - 2026-08-09

### Added

- Add explicit reveal for one exact reachable live revision, returning its
  authenticated document in a non-printable owned zeroizing wrapper.

### Security

- Reuse bounded verified current-head history traversal, reject tombstones and
  unreachable revisions, and never select a current or conflict candidate
  implicitly.

## [0.18.0] - 2026-08-09

### Added

- Add deterministic redacted inspection for every retained current conflict
  candidate, including typed live metadata and tombstone markers.
- Add a session-consuming choose-candidate resolution workflow and owned
  wipe-on-drop randomness for its three encrypted frames.

### Security

- Publish the selected authenticated live document or tombstone as a new
  revision whose direct parents include the complete current conflict set.
- Preserve every losing candidate as immutable reachable history, reject
  missing and unconflicted selections before local persistence, and never ask
  the host to round-trip secret plaintext.

## [0.17.0] - 2026-08-09

### Added

- Add a session-consuming `restore_item` workflow that locates one selected
  revision through bounded reachable history and republishes its live document
  as a new current revision.
- Add owned wipe-on-drop randomness for the three encrypted restoration frames.

### Security

- Require the selected revision to be reachable from a current head, live,
  different from the sole current candidate, and bound to the same item before
  the local write-ahead compare-exchange.
- Make only the selected historical revision the new revision's causal parent,
  while preserving all current commit heads and historical immutable bytes.
- Reject unreachable revisions with `NotFound`, tombstones/current selections
  with `InvalidInput`, and unresolved current candidates with
  `ConflictRequired`, without asking a host to resupply secret plaintext.

## [0.16.0] - 2026-08-09

### Added

- Add bounded item-history reads over verified ancestry from every current
  repository head, with a default limit of 100 and a hard limit of 4,096.
- Return typed historical views containing redacted live metadata or a
  tombstone marker, direct-parent count, advisory revision time, and an
  explicitly requested revision ID.

### Security

- Decrypt historical catalogs and revisions only inside the unlocked session,
  verify vault, object, item, and direct-parent bindings, and deduplicate shared
  ancestry without weakening the provider-neutral repository seam.
- Order results by ancestry depth, commit object ID, and revision object ID
  rather than advisory wall time, while redacting revision and item metadata
  from ordinary diagnostics.

## [0.15.0] - 2026-08-09

### Added

- Add a session-consuming `delete_item` workflow that locates the caller's
  expected current live revision and publishes a one-parent tombstone.
- Accept separate advisory deletion and commit timestamps plus an owned
  wipe-on-drop entropy block for the three encrypted deletion frames.

### Security

- Reject absent, conflicted, and already-tombstoned targets before the local
  write-ahead compare-exchange without exposing item identity in errors.
- Preserve unrelated catalog candidates, make every current repository head a
  commit parent, and reuse the exact crash-resumable publication journal.
- Retain tombstones as current causal state while ordinary get/list/search
  views omit the deleted item and its former secret-bearing document.

## [0.14.0] - 2026-08-09

### Added

- Add a session-consuming `replace_item` workflow that creates a live revision
  directly descended from the caller's sole expected current revision.
- Add the specified payload-free `NotFound` application error and owned
  wipe-on-drop replacement randomness for exactly three encrypted frames.

### Security

- Reject absent, stale, tombstoned, or conflicted replacement targets before
  local persistence, and preserve item identity, schema, and creation time.
- Reuse the exact write-ahead `PendingPublication` state machine for
  replacement, including byte-identical ambiguous winners and crash recovery.
- Rewrite the complete bounded catalog and make all current repository heads
  commit parents without using advisory timestamps for causality.

## [0.13.0] - 2026-08-09

### Added

- Add one new item from an unlocked session using a caller-filled 256-byte
  CSPRNG block for its item identity and three encrypted object frames.
- Construct a parentless revision, complete rewritten catalog, signed commit
  over every current head, signed announcement, and exact expected new pins.

### Security

- Consume the unlocked session and owned document/randomness on every return
  path so stale pins and mutation inputs cannot remain available to callers.
- Compare-exchange the exact `Active -> PendingPublication -> Active` owner
  states around repository publication, accept only byte-identical ambiguous
  winners, and retain a replayable journal across provider or final-local-write
  interruption.
- Reject mismatched generated identities, existing IDs, stale repository
  heads, invalid documents, and counter overflow before publication.

## [0.12.0] - 2026-08-09

### Added

- Build a rebuildable in-memory search projection during authenticated reopen
  and expose bounded `search_items` reads with optional exact collection
  filtering.
- Normalize indexed metadata and owned queries with Unicode lowercase followed
  by NFC, support valid one- and two-byte queries, and deterministically order
  results by normalized display title, schema, then item-ID bytes.

### Security

- Admit only display titles/labels, usernames, URLs, services, database hosts,
  and present tags; passwords, note bodies, seeds, tokens, card data, lease IDs,
  opaque payloads, and non-allowlisted metadata never enter the projection.
- Hold normalized searchable text and query terms in wipe-on-drop containers,
  clear the trigram accelerator with the unlocked session, fail closed on any
  current conflict, and omit indexed values and identities from diagnostics.
- Intersect whitespace-delimited token candidates, post-filter every trigram
  candidate for exact normalized substring matches, and
  safely scan short queries or entries too large for the accelerator without
  truncating searchable metadata.

## [0.11.0] - 2026-08-09

### Added

- Add deterministic `get_item` and `list_items` session reads that return only
  typed, wipe-on-drop `RedactedItemView` values.
- Treat a current tombstone as absent while retaining it internally for
  history and restore.

### Security

- Fail every multi-candidate current-item read with the closed
  `ConflictRequired` error instead of selecting an arbitrary winner.
- Abort conflicted list reads without returning a partial result, while
  retaining every materialized candidate for explicit resolution.

## [0.10.0] - 2026-08-09

### Added

- Materialize every distinct verified-head catalog and referenced current item
  revision into the authenticated unlocked session.
- Expose payload-free item, candidate, and conflicted-item counts for later
  status, redacted-view, and search workflows.

### Security

- Bound the union across head catalogs to 100,000 item identities and 16
  candidates per item, deduplicate identical references, and fail closed on
  dangling, missing direct-parent, or cross-item revision references.
- Keep decrypted domain candidates inside the wipe-on-drop session boundary
  and omit item IDs and metadata from ordinary diagnostics.

## [0.9.0] - 2026-08-09

### Added

- Add exact `PendingPublication` replay through authenticated repository
  composition and durable advancement to the journal's intended active pins,
  counter, and catalog root.
- Accept only the identical intended active bytes when a concurrent local
  writer wins the final compare-exchange.

### Security

- Preserve the exact already-randomized and signed journal across unavailable
  and ambiguous provider failures, preventing counter reuse or self-equivocation.
- Require the exact expected repository receipt before committing new durable
  local pins.

## [0.8.0] - 2026-08-09

### Added

- Add authenticated active-vault reopen from injected local/bootstrap stores,
  including exact signed-bootstrap pin checks, passphrase root unwrap, private
  identity re-derivation, opaque repository connection, and verified open from
  non-empty local head pins.
- Add a wipe-on-drop `UnlockedVaultV1` session boundary with redacted ordinary
  diagnostics and payload-free verified-open metadata.

### Security

- Refuse missing, malformed, unsigned, rolled-back, cross-vault, or locally
  unpinned bootstrap state before repository access.
- Anchor complete repository discovery to durable non-empty head pins and keep
  provider-specific failures outside the closed application error surface.

## [0.7.0] - 2026-08-09

### Added

- One injected, provider-neutral generation-zero completion workflow for both
  first execution and restart recovery.
- Exact idempotent bootstrap installation/readback, repository initialization
  and publication, receipt verification, and final local activation.
- An explicit bootstrap-store contract requiring exact generation replay to
  succeed idempotently while different bytes conflict.

### Security

- The exact `PreparedInit` journal is atomically durable before the first
  bootstrap or repository effect.
- Every failure retains the same randomized and signed bytes for retry; only an
  exact expected receipt can advance local state to `Active`.
- Exact already-active replay succeeds idempotently, while conflicting local,
  bootstrap, repository, and compare-exchange state fails closed.

## [0.6.0] - 2026-08-09

### Added

- No-write passphrase rehydration of a durable generation-zero `PreparedInit`
  journal into its repository address and authority-anchored verifier.

### Security

- Wrong passphrases and unauthenticatable root wraps share the closed
  `AuthenticationFailed` result.
- Rehydration proves the decrypted authority, device-signing, and device-wrap
  private seeds reproduce the identities pinned in the signed bootstrap and
  authority-signed certificate before repository access.

## [0.5.0] - 2026-08-09

### Added

- Pure deterministic generation-zero preparation from an owned zeroizing
  passphrase, bounded KDF policy, advisory timestamp, and caller-filled CSPRNG
  block.
- Exact construction of the signed bootstrap, encrypted certificate and empty
  catalog, initial commit and announcement, repository address, recovery
  journal, intended active state, and authority-anchored verifier.

### Security

- Root wrapping uses Argon2id and XChaCha20-Poly1305 with exact AAD binding to
  the suite and vault ID.
- The passphrase, VRK, KEK, authority/device seeds and signing keys, X25519
  secret, local-secret plaintext, object randomness, and source CSPRNG block
  are held in wipe-on-drop containers.
- Preparation performs no external writes, so the complete exact
  `PreparedInit` journal can be atomically persisted before any remote effect.

## [0.4.0] - 2026-08-09

### Added

- An object-safe application repository and factory over any injected
  VLT-PM02 object store.
- Complete delegation for initialization, verified open, by-value publication,
  encrypted-object reads, commit reads, and bounded history.

### Security

- Production construction requires a caller-supplied unlocked
  `RepositoryVerifier`; there is no unchecked repository path.
- Repository and provider failures are translated to a closed payload-free
  application error taxonomy.
- Exact randomized and signed publication batches are consumed by value,
  preserving the crash journal's single-byte-sequence invariant.

## [0.3.0] - 2026-08-09

### Added

- Exact canonical `PreparedInit`, `Active`, and `PendingPublication` owner-state
  codecs with retry-stable repository journals.
- Byte-oriented injected bootstrap and atomic local-state store contracts.
- Domain-separated local-secret XChaCha20-Poly1305 sealing and opening.
- Random bootstrap locators and domain-separated authority fingerprints.

### Security

- State decoding cross-checks bootstrap, vault, authority, device,
  certificate-frame ID, announcement, commit ID, catalog, head, and counter
  relationships before recovery can use persisted bytes.
- Prepared initialization verifies the embedded-authority generation-zero
  bootstrap signature; pending publication rebinds announcement identity to
  the last active vault, device, and certificate.
- Pending mutations retain exact randomized and signed publication bytes, so a
  retry cannot equivocate at one reserved device counter.
- Store and state diagnostics remain closed and payload-free.

## [0.2.0] - 2026-08-09

### Added

- Exact canonical application wrappers for authority-signed device
  certificates and device-signed commits.
- An authority-anchored VLT-PM04 verifier for the single authorized Phase 1A
  device.

### Security

- Commit frames are ID-checked, AEAD-opened as the commit kind, strictly
  decoded, identity-bound, and Ed25519-verified before repository use.
- Verifier construction requires the locally pinned certificate frame to
  reproduce its expected object ID before authority verification.
- Announcements must match the authorized vault, device, certificate object,
  and signing key; all verifier failures remain payload-free.

## [0.1.0] - 2026-08-09

### Added

- Closed canonical codecs for local device secrets, item revisions, and
  bounded catalog snapshots.
- Lossless observed-set persistence, including retained add operations and
  removal tombstones.
- V1 HKDF subkey derivation and domain-separated XChaCha20-Poly1305 object
  framing over caller-provided randomness.

### Security

- Strict kind, vault, suite, bound, and AEAD checks before plaintext parsing.
- Zeroizing live keys, local secret state, object DEKs, and opened plaintext.
- Closed payload-free diagnostics and a capability-free package boundary.
