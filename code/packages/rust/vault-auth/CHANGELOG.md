# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **VLT-PM51 slice 3 — real ECDSA P-256 assertion-signature
  verification.** `WebAuthnPrfAuthenticator::verify()` can now return
  `Ok(...)` for the first time: after the existing rpId-hash,
  credential-id, and user-presence checks pass, `verify()`
  cryptographically verifies the CTAP2 assertion signature
  (`response.signature`, ASN.1 DER-encoded ECDSA) over
  `response.auth_data || SHA-256(request.challenge)` against the
  credential's registered P-256 public key, using `ring`
  (`ring::signature::ECDSA_P256_SHA256_ASN1`). Only after that
  signature genuinely verifies does the assertion's `hmac-secret`
  output become the returned `AuthAssertion`'s `key_contribution` —
  the whole point of this factor being bind-mode. `ring` is a new
  direct dependency of this crate, but not a new one in this
  workspace's build: `vault-webauthn-ctap2-hid` already pulls it in
  transitively via `ctap-hid-fido2`'s own "ring" feature, so this
  promotes an already-resolved, already-audited dependency rather than
  adding a new crate to the tree. Full design, the re-verified
  dependency survey (`ring` vs. `p256`), and the exact check sequence
  in `code/specs/VLT-PM51-hardware-security-keys.md` §19–§22.
- `parse_es256_cose_public_key` (private) — decodes a COSE_Key
  (RFC 9053 §7) via `coding_adventures_canonical_cbor`, accepting only
  EC2/P-256 keys. A COSE key naming a different key type or curve is
  reported via `AuthError::Unimplemented` (a capability gap), not
  `AuthError::InvalidParameter` (malformed input) — the two are kept
  distinguishable the way every other refusal in this crate already
  keeps them. Parsed once, at `WebAuthnPrfAuthenticator`
  construction time, not on every `verify()` call.
- `verify_es256_signature` (private) — the ECDSA verification itself,
  via `ring::signature::UnparsedPublicKey`. Every failure mode (wrong
  key, wrong message, corrupted signature, an off-curve or otherwise
  malformed public key) collapses to the same
  `AuthError::InvalidCredential`, matching every other check in
  `WebAuthnPrfAuthenticator::verify`.
- `coding_adventures_canonical_cbor` is now a (path) dependency of this
  crate, used only for COSE_Key decoding above.
- 24 new tests: `parse_es256_cose_public_key` (valid key, non-canonical
  CBOR, non-map value, unsupported key type, unsupported curve, missing
  coordinates, wrong-length coordinates); `verify_es256_signature`
  against real `ring`-generated P-256 test vectors (genuine signature
  accepted; tampered message, tampered signature, signature from a
  different key, valid signature for different data, corrupted public
  key, and garbage signature bytes all rejected without panicking); and
  `WebAuthnPrfAuthenticator::verify` end-to-end (a genuinely signed
  assertion now succeeds and yields the `hmac-secret` output as
  `key_contribution`; wrong signing key, corrupted signature bytes,
  `authData` tampered after signing, and a signature valid for a
  different per-attempt challenge are all rejected; an unsupported
  registered COSE key is `Unimplemented` at construction; a malformed
  one is `InvalidParameter`).
- **VLT-PM51 slice 2 — real CTAP2 hardware I/O behind
  `WebAuthnPrfAuthenticator::verify()`.** `verify()` now performs a
  live CTAP2 `GetAssertion` (with the `hmac-secret` extension)
  through a new `Ctap2Transport` trait, and checks everything about
  the response that doesn't require an ECDSA P-256 signature
  verifier: the rpId hash, the credential id, the user-presence flag,
  and that `hmac-secret` actually fired. Only after all of that
  passes does `verify()` reach its final, still-unconditional
  `AuthError::Unimplemented` — because ECDSA P-256 verification still
  doesn't exist anywhere in this workspace. **Superseded above by
  slice 3**, which adds that ECDSA verifier and turns this same final
  step into a real `Ok(...)` for a genuinely valid assertion.
  `Ctap2Transport` is defined here (protocol boundary only, zero
  native dependency); the real `ctap-hid-fido2`/`hidapi`-backed
  implementation is the new sibling crate
  `coding_adventures_vault_webauthn_ctap2_hid`, kept separate so this
  trust-sensitive crate never gains a native/hardware dependency. Full
  design in `code/specs/VLT-PM51-hardware-security-keys.md`.
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

- **Breaking (in effect, not in signature).**
  `WebAuthnPrfAuthenticator::new`/`with_touch_timeout` now decode and
  validate `public_key_cose` at construction time (must be a canonical
  CBOR EC2/P-256 COSE_Key with 32-byte `x`/`y`), returning
  `AuthError::InvalidParameter` or `AuthError::Unimplemented` for
  anything else, instead of accepting any non-empty byte string and
  deferring all validation to a `verify()` call that could never
  succeed anyway. No external callers existed at the time of this
  change (confirmed by search) — nothing has ever registered a real
  `WebAuthnPrfAuthenticator`, since `verify()` could not return `Ok`
  until this slice.
- `AuthError::Unimplemented`'s doc comment updated: it is now reachable
  from `WebAuthnPrfAuthenticator::new`/`with_touch_timeout` (an
  unsupported COSE key type/curve) as well as from other still-fully-
  unimplemented backends (`vault-key-custody::TpmCustodian`), not only
  from `verify()`.
- **Breaking.** `TotpAuthenticator::new` now takes the algorithm as its
  second argument: `new(secret, algorithm, period, digits, window)`.
  The type was hard-wired to HMAC-SHA-1; a stored seed carries its own
  algorithm, so a generator or verifier that assumed SHA-1 would
  silently produce plausible, wrong digits for the other two. This
  crate had no external callers at the time of the change.

### Deferred

See `code/specs/VLT-PM51-hardware-security-keys.md` §22 for the full
reasoning.

- Signature-counter / cloned-authenticator detection. CTAP2's raw
  `sign_count` is not even plumbed through `Ctap2AssertionResponse`
  today (`vault-webauthn-ctap2-hid`'s `map_assertion` doesn't copy it),
  and this crate has no per-credential persistence story for any
  authenticator to compare against — the identical shape
  `TotpAuthenticator`'s own module doc already describes for
  replay-window state ("the caller's responsibility"). Real design
  work, not a few added lines: many resident-credential authenticators
  always report `sign_count == 0`, so a correct implementation needs
  "always zero" tracked per credential too, not just "did it go
  backwards."

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
