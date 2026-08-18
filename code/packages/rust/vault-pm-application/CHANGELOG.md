# Changelog

All notable changes to this package are documented here.

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
