# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial implementation — VLT-PM51 slice 2
  (`code/specs/VLT-PM51-hardware-security-keys.md`).
- `HidCtap2Transport` — real `Ctap2Transport` (defined in
  `coding_adventures_vault_auth`) backed by `ctap-hid-fido2` +
  `hidapi`. Enumerates USB HID FIDO2 devices, performs a CTAP2
  `GetAssertion` request with the `hmac-secret` extension, and bounds
  the wait for a physical touch via a worker thread raced against the
  request's `touch_timeout`.
- `HID_ACCESS_LOCK` — a process-wide `Mutex` serializing every native
  HID call this crate makes, acquired with `try_lock` (never a
  blocking `lock`). Added after this crate's own test suite reliably
  crashed the process with `SIGTRAP` under the default parallel test
  runner — `hidapi`'s enumeration is not safe to call concurrently
  within one process. Matters for production too: two concurrent
  `verify()` calls in one process (e.g. once this transport is wired
  into `vault-pm-agent-host`) would hit the same crash without it. A
  contended attempt fails fast with `Ctap2TransportError::Failed {
  detail: "another hardware operation is already in progress" }`
  rather than queuing — see Security section below for why a blocking
  acquire here was itself a bug, not just a trade-off.
- This is the first package anywhere in the Vault stack
  (`vault-auth`, `vault-key-custody`, `vault-pm-*`) with a native,
  hardware-touching, non-workspace `Cargo.toml` dependency
  (`ctap-hid-fido2`). Dependency choice re-verified against slice 1's
  recommendation (crates.io metadata, actual API surface, transitive
  dependency tree, license, and unsafe-code footprint) before adding
  it — see `VLT-PM51-hardware-security-keys.md`.
- Deliberately a separate crate from `vault-auth` — `vault-auth`
  defines only the `Ctap2Transport` trait boundary and stays free of
  this native dependency; this crate is the one real implementation
  of it, mirroring `VLT-PM48`'s protocol-crate/transport-crate split
  for the local agent.
- 13 unit tests: pure-logic coverage of extension-request building,
  CTAP2 error classification (mapped to a small static taxonomy that
  never echoes device-controlled text), and response mapping against
  real `ctap-hid-fido2` types built by hand (no device needed); one
  real, non-mocked test exercising `ctap_hid_fido2::
  get_fidokey_devices`, `FidoKeyHidFactory::create`, and this crate's
  own `HidCtap2Transport` end-to-end, proving the real dependency
  wiring compiles and calls the right APIs, and that the "no hardware
  attached" path fails fast rather than waiting out the touch
  timeout; and one test pinning the `HID_ACCESS_LOCK` contention fix
  (below) without touching real `hidapi`.

### Security

Findings from this PR's own `/security-review` round, fixed before
merge:

- **`HID_ACCESS_LOCK` blocking acquire could accumulate unboundedly
  many stuck threads (Medium).** The lock was originally acquired with
  a blocking `lock()`. If one attempt's worker thread got stuck (a
  device that never answers `GetAssertion` — already a known,
  documented trade-off of not being able to cancel a blocking native
  HID read), every *subsequent* `verify()` call still spawned a fresh
  worker thread that also blocked acquiring the same lock — one
  permanently-stuck thread per attempt, unboundedly, for as long as
  attempts kept coming against the same stuck or malicious device.
  Fixed by acquiring with `try_lock` instead: a contended attempt now
  fails fast with `Ctap2TransportError::Failed { detail: "another
  hardware operation is already in progress" }`, so at most one
  thread is ever blocked on real (or stuck) hardware I/O at a time.
- **Un-zeroized stack copy of the `hmac-secret` output (Low).**
  `map_assertion` copied the extracted `hmac-secret` bytes into a
  `Zeroizing` wrapper but never wiped the original local variable,
  leaving a duplicate, unwiped copy of the one genuinely secret value
  in this feature on the stack. Fixed by explicitly zeroizing the
  local before it drops, matching the discipline
  `coding_adventures_vault_auth::TotpAlgorithm::mac` already documents
  and applies for the identical reason.
- **Missing `#![forbid(unsafe_code)]` (Info).** This crate has zero
  `unsafe` blocks of its own (verified by inspection, and asserted
  above under "Dependency footprint"); the lint now enforces that as
  a compiler-checked invariant rather than a claim resting on manual
  review, matching `vault-auth`'s posture.

### Known limitations (documented, not hidden)

- No cross-platform way exists to cancel a blocking native HID read
  from safe Rust. When `touch_timeout` elapses, the worker thread
  performing the CTAP2 exchange is left running rather than killed;
  it exits on its own once the device eventually answers (touched or
  not), at which point its result is silently discarded. A single
  abandoned attempt's thread is bounded and self-terminating. Unlike
  before the `try_lock` fix above, this no longer compounds across
  *subsequent* attempts against the same stuck device — each of those
  now fails fast instead of spawning another blocked thread — but the
  one thread tied up by the original stuck attempt still only exits
  once that device eventually answers or its own internal handling
  gives up.
- CTAP2-over-HID has no signal on every authenticator that reliably
  distinguishes "touched and declined" from "never touched" — both
  collapse to `Ctap2TransportError::TouchTimedOut`. A small number of
  authenticators that do report an explicit denial are classified
  precisely (best-effort, via `classify_ctap_error`'s text matching on
  `ctap-hid-fido2`'s free-text `anyhow::Error` messages, since that
  crate does not expose a typed CTAP2 status-code enum).
- An actual physical touch cannot be exercised in CI. Everything up to
  that boundary is tested for real; the touch itself is manual/
  hardware-in-the-loop only.
