# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **VLT-PM51 slice 2 — real CTAP2 hardware I/O behind
  `WebAuthnPrfAuthenticator::verify()`.** `verify()` now performs a
  live CTAP2 `GetAssertion` (with the `hmac-secret` extension)
  through a new `Ctap2Transport` trait, and checks everything about
  the response that doesn't require an ECDSA P-256 signature
  verifier: the rpId hash, the credential id, the user-presence flag,
  and that `hmac-secret` actually fired. Only after all of that
  passes does `verify()` reach its final, still-unconditional
  `AuthError::Unimplemented` — because ECDSA P-256 verification still
  doesn't exist anywhere in this workspace. `Ctap2Transport` is
  defined here (protocol boundary only, zero native dependency); the
  real `ctap-hid-fido2`/`hidapi`-backed implementation is the new
  sibling crate `coding_adventures_vault_webauthn_ctap2_hid`, kept
  separate so this trust-sensitive crate never gains a native/
  hardware dependency. Full design in
  `code/specs/VLT-PM51-hardware-security-keys.md`.
- `Ctap2AssertionRequest`, `Ctap2AssertionResponse`,
  `Ctap2TransportError`, `Ctap2Transport` — the transport-agnostic
  CTAP2 request/response/error/trait types `WebAuthnPrfAuthenticator`
  is built against.
- `AuthError::HardwareUnavailable`, `AuthError::HardwareTimeout`,
  `AuthError::HardwareTransport { detail }` — new variants covering
  the transport's error taxonomy (no device / touch timeout / other
  transport failure).
- `WebAuthnPrfAuthenticator::new`/`with_touch_timeout` now take a
  `Ctap2Transport` at construction time, plus
  `DEFAULT_TOUCH_TIMEOUT`/`MIN_TOUCH_TIMEOUT`/`MAX_TOUCH_TIMEOUT`
  (30s default, 1s..=120s bounds) governing how long `verify()` waits
  for a physical touch before failing closed.
- **Breaking.** `WebAuthnPrfAuthenticator::new` gained a required
  fourth parameter (the transport). No external callers existed at
  the time of this change (confirmed by search).
- `WebAuthnPrfAuthenticator` (bind-mode) — original scaffold for a
  FIDO2 hardware security key (YubiKey and other CTAP2-compliant
  authenticators) unlock factor via the CTAP2 `hmac-secret` extension
  (WebAuthn's `prf` extension), shipped in VLT-PM51 slice 1 with the
  registration-time shape (`relying_party_id`, `credential_id`,
  `public_key_cose`) and trait plumbing (`kind() == "webauthn-prf"`,
  `mode() == Mode::Bind`); superseded by the real hardware I/O above.
- `AuthError::Unimplemented { backend: &'static str }` — new variant
  used by `WebAuthnPrfAuthenticator::verify`.
- Built-in versus extension factor counts and gate/bind contribution
  consistency helpers on `AuthAssertionSetSummary`.
- `TotpAlgorithm` — `Sha1`, `Sha256`, or `Sha512`, per RFC 6238 §1.2.
  Deliberately has no `Default`: six wrong digits look exactly like six
  right ones, so the parameter that decides which is which is never
  chosen on a caller's behalf.
- `TotpAuthenticator::formatted_code_at` — the code as the decimal
  string a person types, zero-padded to the configured width and held
  in a `Zeroizing<String>`. Roughly one code in ten has a leading zero,
  and `042311` is not `42311`; returning the integer and asking every
  caller to remember the padding is an invitation for one of them to
  forget.
- `TotpAuthenticator::remaining_seconds` — seconds until the current
  step ends, in `1..=period`. Never `0`, because a code with zero
  seconds left has already been replaced by the next one.
- `TotpAuthenticator::digits` accessor.
- The full RFC 6238 Appendix B table as tests: all six published
  timestamps against all three algorithms, each with its own published
  seed (20/32/64 bytes), at the published 8-digit width and at the
  6-digit truncation. Plus period-boundary behaviour (constant across a
  step, different across the transition, identical one period later),
  zero-padding, and a cross-algorithm test proving the selector
  actually changes the hash rather than being ignored.

### Changed

- **Breaking.** `TotpAuthenticator::new` now takes the algorithm as its
  second argument: `new(secret, algorithm, period, digits, window)`.
  The type was hard-wired to HMAC-SHA-1; a stored seed carries its own
  algorithm, so a generator or verifier that assumed SHA-1 would
  silently produce plausible, wrong digits for the other two. This
  crate had no external callers at the time of the change.

### Fixed

- Ten-digit codes no longer overflow the modulus. It was computed as
  `10u32.pow(digits)` while `digits` is permitted up to 10, and `10^10`
  exceeds `u32::MAX` — so a legal argument panicked in debug builds and
  wrapped in release ones. The modulus is now computed in `u64`. RFC
  4226 dynamic truncation yields only 31 bits, so at ten digits the
  modulus is correctly a no-op.
- Dynamic truncation indexes the offset nibble from the *last* byte of
  the tag rather than from a hard-wired byte 19, which is what makes it
  correct for the 32- and 64-byte tags of the wider hashes.
- The HMAC tag no longer escapes its wipe-on-drop wrapper. `TotpAlgorithm::mac`
  copies each fixed-size tag into the returned buffer and then zeroizes the
  array; the shorter `hmac_sha1(...)?.into()` would have left the *first* copy
  on the stack untouched while `Zeroizing` owned the second. A TOTP tag is not
  merely secret-adjacent — the code is read straight out of it.
- Dynamic truncation now uses checked lookups (`last()` and `get(o..o+4)`)
  returning `Crypto` instead of indexing. The panic was unreachable — a nibble
  is at most 15 and the narrowest tag is 20 bytes — but "unreachable" rested on
  a fact about three hash functions declared in another crate, which this
  function cannot see. A refusal is a better answer than a panic in a password
  manager.

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT05
  (`code/specs/VLT05-vault-auth.md`).
- `Authenticator` trait + `Mode` (`Gate` / `Bind`) +
  `AuthAssertion`. Bind-mode factors contribute key material to
  the unlock derivation; gate-mode factors only pass/fail.
- `PasswordAuthenticator` (bind-mode) — Argon2id-derived tag is
  the key contribution. `with_verifier(salt, t, m, p, verifier)`
  takes the four pieces persisted at registration time;
  `derive_verifier(...)` is the registration-time helper. Verify
  uses `ct_eq` for constant-time tag comparison.
- `TotpAuthenticator` (gate-mode) — RFC 6238, HMAC-SHA-1, 6-digit
  default, 30-second period, ±1-step window default. Tested
  against the published RFC 6238 Appendix B vectors (T=59,
  T=1111111109, T=1111111111). `verify_at_time(code,
  unix_time)` returns the matched step counter so callers can
  pin a per-secret last-used step into a replay-rejection cache.
  `code_at_counter` is internal but accessible via
  `code_at(unix_time)` for diagnostics.
- `combine_key_contributions(vault_id, factors)` — HKDF-Extract
  over the ordered concatenation of bind-mode contributions, with
  the vault-id as salt and `"VLT05/key/v1"` as info, producing a
  32-byte unlock key. Different vault-ids derive distinct unlock
  keys from the same factor set.
- `AuthError` typed enum: `InvalidCredential`,
  `MalformedCredential`, `InvalidParameter`, `Crypto`,
  `NoBindFactors`. `Display` strings sourced exclusively from
  this crate's literals.
- Credential-safe `AuthAssertionSummary` and `AuthAssertionSetSummary`
  read models for policy/audit layers that need factor coverage without
  exposing key-contribution bytes.
- All key material is held in `Zeroizing<…>` and wiped on drop;
  `AuthAssertion::Drop` zeroes the contained key contribution.
- 16 unit tests covering: password verify success with bind
  contribution; wrong password rejected as `InvalidCredential`;
  empty credential malformed; constructor parameter validation
  (short salt, empty verifier, weak Argon2id params); password
  key-contribution determinism (same password ⇒ same bytes);
  RFC 6238 known-answer vectors (T=59 → 287082, T=1111111109 →
  081804, T=1111111111 → 050471); TOTP window accepts ±1 step;
  TOTP outside-window rejection; TOTP parameter validation
  (empty secret, period 0, digit count > 10); TOTP malformed
  credential (wrong digit count, non-decimal); TOTP gate-mode
  has no key contribution; combine yields deterministic unlock
  key; combine on distinct vault-ids yields distinct keys;
  combine skips gate-mode factors and refuses on no-bind-factors;
  combine refuses empty list; error-display-from-literals.

### Out of scope (future PRs)

- WebAuthn (signature-only, gate-mode).
- WebAuthn-PRF / FIDO2 hmac-secret (bind-mode hardware factor —
  the YubiKey-as-key-derivation flow Bitwarden / 1Password use).
- OPAQUE / SRP-6a aPAKE flows.
- OIDC / JWT / mTLS / AppRole / AWS-STS / GCP-JWT / Azure-MI /
  Kubernetes-SA — the machine-auth side.
- SMS / email OTP / Duo push.
- Replay-cache integration: TOTP `verify_at_time` returns the
  matched step so apps can store-and-reject; the cache itself is
  application-level concern.
