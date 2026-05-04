# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT09
  (`code/specs/VLT09-vault-audit-log.md`).
- `AuditEvent` — cleartext input the caller hands to the
  chain. Carries `principal`, `action: AuditAction`,
  `resource: Option<String>`, `detail: Option<Vec<u8>>`.
- `AuditAction` — `#[non_exhaustive]` enum with explicit
  variants for the most common cross-tier events
  (`AuthSucceed`/`AuthFail`, `PolicyAllow`/`PolicyDeny`,
  `EngineMint`/`EngineRevoke`/`EngineRotateRoot`,
  `LeaseConsume`/`LeaseRevoke`, `SealedWrite`/`SealedRead`)
  plus `Other(label)` as an escape hatch. New variants land
  non-breakingly.
- `AuditEntry` — a sequenced + linked + timestamped event.
  Fields: `seq`, `timestamp_ms`, `prev_hash`, `event`. Genesis
  entry's `prev_hash` is 32 zero bytes.
- `SignedAuditEntry` — an `AuditEntry` plus its embedded
  `signer_pub` and Ed25519 signature over the canonical bytes.
- `canonical_bytes()` — total, deterministic encoder. Tagged
  length-prefixed framing with a 4-byte magic `"AUD1"` and a
  1-byte version. Bounded-size guarantees from
  `validate_event`.
- `AuditSigningKey` — wraps the Ed25519 secret in
  `Zeroizing<[u8; 64]>` and a redacted `Debug` so a stray
  `dbg!(key)` cannot leak the secret bytes. Constructor
  `from_seed(&[u8; 32])` derives the keypair via
  `coding_adventures_ed25519::generate_keypair`.
- `AuditSink` trait — `Send + Sync`, append-only contract:
  `append`, `len`, `is_empty` (default), `entries`, `last`
  (default).
- `InMemoryAuditSink` — reference implementation backed by a
  `Mutex<VecDeque<SignedAuditEntry>>`.
- `AuditChain<S>` — writer. Caches the head (`next_seq`,
  `prev_canonical_bytes`) under a `Mutex` so concurrent
  `record()` calls serialize cleanly. `attach(key, sink)`
  reads the sink's existing tail to pick up where a previous
  process left off (so an audit chain survives restarts).
- `record(event, timestamp_ms)` — validates, allocates seq,
  links `prev_hash`, signs, appends. The order is
  *append-then-bump-head*, so a sink failure leaves the head
  unchanged and a retry reuses the same sequence number.
- `verify_chain(entries, expected_signer_pub)` — walks the
  chain in append order and checks: dense `seq` starting at 0,
  `prev_hash` recomputes to `blake2b-256(prev.canonical ||
  this.body)`, every Ed25519 signature verifies against the
  embedded `signer_pub`, and (optionally) every entry was
  signed by the pinned key.
- `validate_event` — bounds: `principal` non-empty and
  ≤ `MAX_PRINCIPAL_LEN` (256), `resource` ≤ `MAX_RESOURCE_LEN`
  (512), `detail` ≤ `MAX_DETAIL_LEN` (1024),
  `Other(label)` non-empty and ≤ `MAX_OTHER_ACTION_LEN` (64).
  Bounds keep verification linear-in-entries, not
  linear-in-bytes-of-detail.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.
- 24 unit tests covering: genesis-zero-prev_hash,
  second-entry-links-to-first, full-chain-verify, pinned
  issuer (matches and mismatches),
  tampered-event-breaks-current-entry,
  tampered-first-entry-breaks-second-via-prev_hash,
  truncation visible as resequence, forged signature breaks
  verification, empty chain verifies vacuously, all four
  validate_event rejection paths (empty principal, oversize
  detail, oversize principal, empty/oversize Other label),
  attach picks up existing chain across simulated restart,
  Other action round-trip, signing key Debug redaction,
  Send+Sync compile-time check, 32-thread concurrent record,
  canonical_bytes magic + version, `signed.signer_pub ==
  chain.signer_public()`.

### Security hardening (pre-merge review)

Three findings flagged before push, all fixed inline:

- **MEDIUM** — `attach()` did not verify the existing chain
  before extending it: a tampered or forged sink tail would
  silently get extended by new `record()` calls. Now `attach()`
  runs `verify_chain(.., Some(&key.public()))` end-to-end on
  the existing entries and fails closed with
  `VerificationFailed`. An `attach_unverified()` escape hatch
  exists for callers that have already validated the sink
  out-of-band (e.g. a Trillian inclusion proof).
- **LOW** — Mutex poisoning panicked the audit log on any prior
  thread panic, silently DoS'ing security-critical recording.
  Replaced `lock().expect("...")` with a poison-recovering
  helper (`PoisonError::into_inner`). Audit-log invariants
  remain coherent across the recovery (verified by inspection:
  no panic site between lock and structural-invariant
  restoration).
- **INFO** — `AuditEvent::detail` documentation now warns
  callers not to put secrets there: detail bytes flow through
  non-zeroizing `Vec<u8>` intermediates by design (the crate
  does not seal at rest — VLT01 does, in a higher tier).

### Out of scope (future PRs)

- **Persistent file sink** — `vault-audit-fs`.
- **Transparency log** — `vault-audit-trillian` / `vault-audit-sigsum`.
- **Cloud sinks** — `vault-audit-syslog`, `vault-audit-s3`,
  `vault-audit-splunk`.
- **Sealed-at-rest** — production deployments will route
  entries through VLT01 sealed-store before they reach a sink;
  the chain still verifies because hashes are over the
  cleartext canonical bytes.
- **Indexed queries** — a higher tier will provide search by
  principal / action / time range over the chain.
