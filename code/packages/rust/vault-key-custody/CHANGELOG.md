# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT03
  (`code/specs/VLT03-vault-key-custody.md`).
- `KeyCustodian` trait — three-method interface (`name`,
  `capabilities`, `wrap`, `unwrap`) abstracting where the master
  KEK lives.
- `CustodianCaps` — capability flags reported per custodian:
  `hardware_bound`, `extractable`, `requires_user_presence`,
  `remote`. Two ready-made constants: `SOFTWARE` (passphrase) and
  `HARDWARE_LOCAL` (TPM / Secure Enclave).
- `PassphraseCustodian` — full implementation. Argon2id KDF
  (RFC 9106; default time=3, memory=64 MiB, parallelism=4) +
  XChaCha20-Poly1305 AEAD wrap. Wire format:
  `magic(2) || salt(16) || nonce(24) || ct(32) || tag(16)` =
  90 bytes. AAD = `magic || label` so blobs can't be replayed
  under a different label. `Drop` wipes the held passphrase via
  `Zeroizing<Vec<u8>>`. Argon2id parameters validated at
  construction time (rejects empty passphrase, zero time-cost,
  memory < 8 KiB, zero parallelism).
- `TpmCustodian` scaffold — reports
  `CustodianCaps::HARDWARE_LOCAL` so `select_custodian` correctly
  prefers it; `wrap` / `unwrap` return
  `CustodyError::Unimplemented { backend: "TPM 2.0 / Secure
  Enclave" }` until the platform-specific backend lands in a
  follow-up PR. Capability reporting is wired now so downstream
  TPM-first decisions can already be made.
- `select_custodian(candidates, force_software)` — the policy
  helper that enforces TPM-first / hardware-preferred:
  - If any candidate is hardware-bound, it wins.
  - If `force_software = true`, picks the first software candidate
    even when hardware is available (test / migration path).
  - Empty candidate list → `NoCandidates`.
- `assert_no_hardware_bypass(candidates, host_has_hw,
  force_software)` — boot-time advisory check that rejects a
  software-only candidate list when the host actually has a
  hardware custodian and `force_software` is unset, returning
  `HardwareAvailableButSoftwareRequested`.
- `CustodyError` typed enum: `InvalidPassphrase`,
  `MalformedWrappedKey`, `InvalidParameter`, `Csprng`, `Kdf`,
  `Aead`, `HardwareAvailableButSoftwareRequested`, `NoCandidates`,
  `Unimplemented`. `Display` strings sourced exclusively from this
  crate's literals; attacker-controlled bytes never appear in
  error output.
- 24 unit tests covering: passphrase round-trip, wrap-produces-
  distinct-blobs (fresh salt + nonce), wrong-passphrase rejection
  (fail-closed), wrong-label rejection (AAD binding works),
  body-tamper rejection (AEAD detects), magic-tamper / truncated-
  blob → MalformedWrappedKey, parameter validation (empty
  passphrase, zero time-cost), capability reporting (passphrase
  vs TPM), `TpmCustodian::wrap` returns Unimplemented,
  `select_custodian` picks hardware when available, picks software
  when only software, force_software override, empty candidates,
  `assert_no_hardware_bypass` refusals and allowances, error-
  message-from-literals invariant, deterministic wrap-output
  length, custodian Drop is safe.

### Security review

- TPM-first / hardware-preferred is the headline guarantee:
  software custodians cannot silently fall back when hardware is
  available. `assert_no_hardware_bypass` makes the refusal
  explicit at boot time.
- All wrap/unwrap operations hold the derived KEK in
  `Zeroizing<[u8; 32]>` so it wipes on every return path.
- `PassphraseCustodian` `Drop` wipes the held passphrase; the
  `Zeroizing<Vec<u8>>` wrapper does this automatically, the
  explicit `zeroize()` call is belt-and-braces.
- Wrong passphrase / wrong label / body tamper all return the
  same `InvalidPassphrase` variant (no oracle distinguishing
  these conditions), preserving the VLT01 discipline.

Round 1 security review found 2 MEDIUM. Both fixed inline before
push:

- **MEDIUM** — `with_params` accepted `passphrase: impl
  Into<Vec<u8>>` and held it as a bare `Vec<u8>` until after
  parameter validation. On rejection (empty / bad Argon2id
  parameters) the heap buffer was freed without zeroing.
  **Fixed:** wrap into `Zeroizing<Vec<u8>>` immediately on entry,
  before any validation that could fail.
- **MEDIUM** — `select_custodian(force_software = true)` on a
  hardware-only candidate list silently returned a hardware
  custodian (caller asked for software, got hardware — a
  policy/identity confusion footgun). **Fixed:** added
  `CustodyError::NoSoftwareCandidate`; the function now fails
  closed when `force_software` is set but no software candidate
  exists. New test:
  `select_with_force_software_on_hardware_only_list_fails_closed`.
