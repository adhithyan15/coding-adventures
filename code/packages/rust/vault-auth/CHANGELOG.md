# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
