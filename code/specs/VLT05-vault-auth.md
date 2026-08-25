# VLT05 — Vault Authentication

## Overview

The pluggable **authentication** layer of the Vault stack. Hosts
the `Authenticator` trait and ships two factors in v0.1:
`PasswordAuthenticator` (Argon2id, bind-mode) and
`TotpAuthenticator` (RFC 6238, gate-mode). Designed so additional
factors (WebAuthn / FIDO2-PRF / OPAQUE / OIDC / mTLS / SMS / Duo
/ AppRole / AWS-STS / Kubernetes-SA / …) slot in via the same
trait without touching the vault core.

Implementation lives at `code/packages/rust/vault-auth/`.

## Why pluggable

Both reference targets (Bitwarden-class password manager and
HashiCorp-Vault-class machine secrets) need a wide, varied set of
auth factors. Bitwarden alone supports password + TOTP +
WebAuthn + Duo + Email + FIDO2-PRF; HashiCorp Vault supports
tokens + AppRole + Userpass + LDAP + OIDC + AWS-STS + GCP-JWT +
Azure-MI + K8s-SA + GitHub + JWT + TLS + Kerberos. There is no
fixed set; this layer is a plugin host.

## Two operating modes

```rust
pub enum Mode { Gate, Bind }
```

- **`Gate`** — pass/fail, no key material contributed. Used by
  2FA-style factors that prove possession but don't widen the
  unlock-key derivation: TOTP, SMS-OTP, Email-OTP, classic
  WebAuthn signature-only flow, Duo push.
- **`Bind`** — the factor *also* contributes key material to the
  unlock derivation (KDF input set widens). 1Password's "Secret
  Key" is bind-mode, FIDO2-PRF is bind-mode, Shamir-quorum shares
  are bind-mode. Compromise of bind-mode storage doesn't unlock
  anything without the bind factor.

## Trait API

```rust
pub trait Authenticator {
    fn kind(&self) -> &'static str;
    fn mode(&self) -> Mode;
    fn verify(&self, credential: &[u8]) -> Result<AuthAssertion, AuthError>;
}

pub struct AuthAssertion {
    pub kind: &'static str,
    pub mode: Mode,
    pub key_contribution: Option<Zeroizing<Vec<u8>>>,
}

pub fn combine_key_contributions(
    vault_id: &[u8],
    factors: &[&AuthAssertion],
) -> Result<Zeroizing<[u8; 32]>, AuthError>;
```

`combine_key_contributions` performs:

```text
   ikm     = bind_factor_1.key || bind_factor_2.key || …          (ordered)
   unlock  = HKDF(salt = vault_id, ikm, info = "VLT05/key/v1",
                  length = 32, SHA-256)
```

So a vault's unlock key = HKDF over the bind-mode contributions
with the vault-id as salt. Different vaults with the same
factor set derive different unlock keys (vault-id binding).

## `PasswordAuthenticator` (bind-mode)

Argon2id-backed. Stored verifier `V = Argon2id(password, salt, t,
m, p, tag_len)`. Verify re-derives the candidate and compares
constant-time via `ct_eq`. On success, the candidate IS the
`key_contribution` (so apps can derive the unlock key from the
exact same Argon2id output that authenticated the user).

Construction:

- `with_verifier(salt, t, m, p, verifier)` — caller supplies the
  four pieces persisted at registration time.
- `derive_verifier(password, salt, t, m, p, tag_len)` —
  registration-time helper.

Validation: salt ≥ 8 bytes, verifier non-empty, `t ≥ 1`, `m ≥ 8
KiB`, `p ≥ 1`.

## `TotpAuthenticator` (gate-mode)

RFC 6238 — HOTP under a time-based counter, with the HMAC hash
selected per RFC 6238 §1.2.

```rust
TotpAuthenticator::new(secret, algorithm, period_sec, digits, window)
```

`algorithm` is `TotpAlgorithm::{Sha1, Sha256, Sha512}` and is
**required**, with no `Default`. SHA-1 remains what every
Google-Authenticator-clone expects, but a provisioned seed carries
its own algorithm with it, and a type that assumed SHA-1 would
produce six plausible, wrong digits for the other two — which is
indistinguishable from six right ones until a site rejects them.
The other parameters keep the universal baseline: `period = 30`,
`digits = 6`, `window = 1`.

### Verifier and generator

`verify(credential)` uses `SystemTime::now()`. `verify_at_time(
code, unix_time)` is the testable / replay-cache-integrating
variant — it returns the matched step counter, so callers can
pin "last-used step ≥ N" and reject replays.

The same type is also the **generator**, which is what a password
manager displaying the current code for a stored seed needs
(`VLT-PM45-cli-totp-code.md`):

- `code_at(unix_time)` — the code as an integer;
- `formatted_code_at(unix_time)` — the code as the decimal string a
  person types, zero-padded to `digits`, in a `Zeroizing<String>`;
- `remaining_seconds(unix_time)` — seconds until the step ends, in
  `1..=period`, never `0`.

The `window` parameter is a *verifier's* tolerance and takes no part
in generation: there is exactly one current step, and offering a
neighbouring one would offer a code that is spent or not yet live.

### Conformance

Tested against **every** published RFC 6238 Appendix B vector — all
six timestamps against all three algorithms, each with its own
published seed (20, 32, and 64 ASCII bytes of repeating
`1234567890`), at the published 8-digit width and at the 6-digit
truncation. Using each algorithm's own seed is what makes the table
a test of the algorithm selector rather than of one hash three
times.

Dynamic truncation (RFC 4226 §5.3) takes its offset nibble from the
*last* byte of the tag, not from a hard-wired byte 19, so it is
correct for the 32- and 64-byte tags of the wider hashes. The
`10^digits` modulus is computed in `u64`, because `digits` is legal
up to 10 and `10^10` exceeds `u32::MAX`.

## `WebAuthnPrfAuthenticator`

Added by `VLT-PM51-hardware-security-keys.md` (slice 1); real CTAP2
hardware I/O added by that same document's slice 2. Bind-mode: a
FIDO2 hardware security key (YubiKey and other CTAP2-compliant
authenticators — a standards-based factor, not vendor-specific)
contributes key material via the CTAP2 `hmac-secret` extension
(WebAuthn's `prf` extension).

```rust
pub struct WebAuthnPrfAuthenticator { /* relying_party_id, credential_id, public_key_cose, transport, touch_timeout */ }

impl WebAuthnPrfAuthenticator {
    pub fn new(relying_party_id: impl Into<String>,
               credential_id: impl Into<Vec<u8>>,
               public_key_cose: impl Into<Vec<u8>>,
               transport: impl Ctap2Transport + Send + Sync + 'static) -> Result<Self, AuthError>;
    pub fn with_touch_timeout(/* as above, plus */ touch_timeout: Duration) -> Result<Self, AuthError>;
}
```

`kind()` is `"webauthn-prf"`, `mode()` is `Mode::Bind`. `verify()`
performs a real CTAP2 `GetAssertion` (with `hmac-secret`) through the
`Ctap2Transport` this instance was built with, checks the response's
rpId hash, credential id, user-presence flag, and `hmac-secret`
presence, and *only after all of that passes* still returns
`AuthError::Unimplemented { backend: "ECDSA P-256 assertion-signature
verification (WebAuthn PRF)" }` — the one remaining piece is an ECDSA
P-256 signature verifier, which no primitive in this workspace
implements yet. `VLT-PM51` §13 covers the full check sequence; §6/§7
(slice 1) explain the original reasoning for never partially
validating an assertion, which still holds — this slice moved *where*
the refusal happens, not *whether* it happens.

`Ctap2Transport`, `Ctap2AssertionRequest`, `Ctap2AssertionResponse`,
and `Ctap2TransportError` are the transport-agnostic types this
authenticator is built against; the real, `ctap-hid-fido2`/`hidapi`-
backed implementation (`HidCtap2Transport`) lives in the sibling crate
`coding_adventures_vault_webauthn_ctap2_hid`, kept separate so this
crate never gains a native/hardware dependency (`VLT-PM51` §12).
Unit tests exercise `verify()`'s full logic against an in-process fake
transport; `vault-webauthn-ctap2-hid`'s own tests exercise the real
dependency for real, with no hardware attached (`VLT-PM51` §15).

This lets:

- Code that composes authenticators (a future VLT06 policy, e.g.
  `all_of { password, webauthn_prf }`) be written against a real type
  today.
- `combine_key_contributions` and `summarize_auth_assertions` be
  exercised against a real bind-mode `"webauthn-prf"` assertion now
  (both dispatch on `Mode`, not `kind`, so neither needs a change once
  `verify()` is real).

Still deferred: ECDSA P-256 verification itself — `VLT-PM51` §16 has
the full list of what remains after slice 2.

## `AuthError`

```rust
pub enum AuthError {
    InvalidCredential,
    MalformedCredential,
    InvalidParameter { what: &'static str },
    Crypto,
    NoBindFactors,
    Unimplemented { backend: &'static str },
    HardwareUnavailable,
    HardwareTimeout,
    HardwareTransport { detail: &'static str },
}
```

The last three variants are `WebAuthnPrfAuthenticator`-specific,
covering "no device attached," "no touch within the timeout," and
"transport failed for any other reason" respectively — see `VLT-PM51`
§14 for the full taxonomy and why `HardwareTimeout` also covers an
authenticator that affirmatively declines the request.

## Threat model & test coverage

| Threat                                                     | Defence                                                             | Test                                                              |
|------------------------------------------------------------|---------------------------------------------------------------------|--------------------------------------------------------------------|
| Wrong password                                             | `ct_eq` of Argon2id tag; fail-closed                                | `password_wrong_password_rejected`                                |
| Empty / malformed credential                               | Up-front malformed rejection                                        | `password_empty_credential_is_malformed`, `totp_malformed_credential_rejected` |
| Constructor parameter validation                           | Argon2id params, salt length, TOTP digits / period                  | `password_with_verifier_rejects_short_salt`, `totp_invalid_parameters_rejected` |
| TOTP code from outside the configured window               | `verify_at_time` rejects                                            | `totp_verify_at_time_rejects_outside_window`                      |
| Same vault-id + same factors derive different unlock keys  | HKDF-Extract over ordered ikm with vault-id salt — deterministic    | `combine_yields_deterministic_unlock_key`                         |
| Cross-vault unlock-key reuse                               | vault-id is HKDF salt → different vaults yield different keys       | `combine_distinct_vault_ids_yield_distinct_keys`                  |
| Gate-mode factor accidentally contributes key material     | `combine_key_contributions` skips Mode::Gate                        | `combine_skips_gate_mode_factors`                                 |
| Caller forgets to supply any bind factor                   | `NoBindFactors`                                                     | `combine_no_factors_rejected`                                     |
| Argon2id timing leak on tag compare                        | `ct_eq` constant-time                                               | implicit via the upstream `ct-compare` crate's tests              |
| Attacker-controlled bytes in error logs                    | All `Display` strings are static literals                           | `error_messages_are_static_literals`                              |
| A `webauthn-prf` assertion accepted on partial (non-cryptographic) validation alone | `verify()` still refuses (`Unimplemented`) even after a real hardware round trip passes every structural check | `webauthn_prf_verify_still_refuses_after_a_correct_hardware_round_trip` |
| Hardware key answering for the wrong relying party / wrong credential | rpId hash and credential id compared against the registered values | `webauthn_prf_verify_rejects_wrong_relying_party_hash`, `webauthn_prf_verify_rejects_credential_id_mismatch` |
| Hardware key assertion without a physical touch | `user_present` flag checked | `webauthn_prf_verify_rejects_missing_user_presence` |
| Hardware key without `hmac-secret` support | Extension-output presence checked | `webauthn_prf_verify_rejects_missing_hmac_secret_extension` |
| No hardware key attached blocking the unlock path | `HardwareUnavailable`, returned fast via cheap enumeration | `webauthn_prf_verify_maps_no_device_to_hardware_unavailable` (fake transport); wall-clock timing test in `vault-webauthn-ctap2-hid` (real transport) |
| Hardware key never touched / declined hanging `verify()` | Bounded `touch_timeout` (default 30s, 1s..=120s) | `webauthn_prf_verify_maps_touch_timeout`, `webauthn_prf_rejects_touch_timeout_below_minimum`, `webauthn_prf_rejects_touch_timeout_above_maximum` |
| `hmac-secret` key derivation drifting across unlock attempts | Salt derived only from registration-time data (rpId + credential id), never from the per-attempt challenge | `webauthn_prf_hmac_secret_salt_is_stable_across_attempts_and_independent_of_credential_bytes` |
| Hardware-key construction with missing registration data   | Constructor validation (empty rp id / credential id / public key)   | `webauthn_prf_rejects_empty_relying_party_id`, `webauthn_prf_rejects_empty_credential_id`, `webauthn_prf_rejects_empty_public_key` |

## Out of scope (this PR)

- OPAQUE / SRP-6a aPAKE.
- OIDC / JWT / mTLS / AppRole / AWS-STS / GCP-JWT / Azure-MI /
  Kubernetes-SA.
- SMS / email-OTP / Duo push.
- Replay-cache integration: this crate exposes `verify_at_time`
  returning the matched step; persisting last-used-step per
  secret and rejecting replays is the caller's job.
- ECDSA P-256 signature verification — the one piece still standing
  between `WebAuthnPrfAuthenticator::verify()`'s current final `Err`
  and a real `Ok(...)`; no elliptic-curve signature primitive exists
  anywhere in this workspace yet. `VLT-PM51-hardware-security-keys.md`
  §16 covers the full list of what remains after real CTAP2 hardware
  I/O landed in that document's slice 2.
- Plain-signature (gate-mode) `WebAuthnAuthenticator` — not needed
  until a factor without `hmac-secret` support is required.

## Citations

- RFC 4226 — *HOTP: An HMAC-Based One-Time Password Algorithm*.
  HOTP under HMAC-SHA-1; truncation per §5.3.
- RFC 6238 — *TOTP: Time-Based One-Time Password Algorithm*.
  Test vectors in Appendix B.
- RFC 9106 — *Argon2 Memory-Hard Function*. Used by
  `PasswordAuthenticator`.
- RFC 5869 — *HKDF*. Used by `combine_key_contributions`.
- FIDO Alliance CTAP2 `hmac-secret` extension / W3C WebAuthn `prf`
  extension — used by `WebAuthnPrfAuthenticator`. See
  `VLT-PM51-hardware-security-keys.md` for the full protocol survey.
- VLT00-vault-roadmap.md — VLT05 layer purpose.
