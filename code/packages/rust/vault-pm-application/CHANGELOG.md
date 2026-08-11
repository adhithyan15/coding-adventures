# Changelog

All notable changes to this package are documented here.

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
