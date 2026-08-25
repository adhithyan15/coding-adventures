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

## `WebAuthnPrfAuthenticator` (scaffold)

Added by `VLT-PM51-hardware-security-keys.md`. Bind-mode: a FIDO2
hardware security key (YubiKey and other CTAP2-compliant
authenticators — a standards-based factor, not vendor-specific)
contributes key material via the CTAP2 `hmac-secret` extension
(WebAuthn's `prf` extension).

```rust
pub struct WebAuthnPrfAuthenticator { /* relying_party_id, credential_id, public_key_cose */ }

impl WebAuthnPrfAuthenticator {
    pub fn new(relying_party_id: impl Into<String>,
               credential_id: impl Into<Vec<u8>>,
               public_key_cose: impl Into<Vec<u8>>) -> Result<Self, AuthError>;
}
```

`kind()` is `"webauthn-prf"`, `mode()` is `Mode::Bind`. `verify()`
always returns `AuthError::Unimplemented { backend: "FIDO2 CTAP2
hmac-secret (WebAuthn PRF)" }`, regardless of input — the identical
pattern `vault-key-custody::TpmCustodian` uses for `wrap`/`unwrap`.
Two pieces are missing before `verify()` can do anything else: a real
CTAP2/WebAuthn hardware transport, and an ECDSA P-256 signature
verifier (no elliptic-curve primitive exists anywhere in this
workspace today). `VLT-PM51` §6 explains at length why `verify()`
refuses unconditionally rather than validating the parts of an
assertion that don't need the missing signature check.

This lets:

- Code that composes authenticators (a future VLT06 policy, e.g.
  `all_of { password, webauthn_prf }`) be written against a real type
  today.
- `combine_key_contributions` and `summarize_auth_assertions` be
  exercised against a real bind-mode `"webauthn-prf"` assertion now
  (both dispatch on `Mode`, not `kind`, so neither needs a change once
  `verify()` is real).

Future PR: a CTAP2/WebAuthn hardware transport (`VLT-PM51` §5
recommends `ctap-hid-fido2`) plus ECDSA P-256 verification, landing
together since neither is independently useful without the other.

## `AuthError`

```rust
pub enum AuthError {
    InvalidCredential,
    MalformedCredential,
    InvalidParameter { what: &'static str },
    Crypto,
    NoBindFactors,
    Unimplemented { backend: &'static str },
}
```

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
| A `webauthn-prf` assertion validated only "shaped like" real | `verify()` refuses unconditionally rather than partially validating | `webauthn_prf_verify_always_returns_unimplemented` (tried against both empty and plausible-looking input) |
| Hardware-key construction with missing registration data   | Constructor validation (empty rp id / credential id / public key)   | `webauthn_prf_rejects_empty_relying_party_id`, `webauthn_prf_rejects_empty_credential_id`, `webauthn_prf_rejects_empty_public_key` |

## Out of scope (this PR)

- OPAQUE / SRP-6a aPAKE.
- OIDC / JWT / mTLS / AppRole / AWS-STS / GCP-JWT / Azure-MI /
  Kubernetes-SA.
- SMS / email-OTP / Duo push.
- Replay-cache integration: this crate exposes `verify_at_time`
  returning the matched step; persisting last-used-step per
  secret and rejecting replays is the caller's job.
- Real WebAuthn-PRF hardware I/O and ECDSA P-256 signature
  verification — `WebAuthnPrfAuthenticator` ships as a scaffold
  (see above); `VLT-PM51-hardware-security-keys.md` covers the full
  design and the follow-up work that completes it.
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
