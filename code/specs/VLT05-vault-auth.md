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

RFC 6238 — HMAC-SHA-1 HOTP under a time-based counter.

```rust
TotpAuthenticator::new(secret, period_sec, digits, window)
```

Defaults match the universal authenticator-app baseline:
`period = 30`, `digits = 6`, `window = 1`. SHA-1 is the RFC 6238
default and what every Google-Authenticator-clone expects;
SHA-256 / SHA-512 variants are a parameterisation away in a
follow-up.

`verify(credential)` uses `SystemTime::now()`. `verify_at_time(
code, unix_time)` is the testable / replay-cache-integrating
variant — it returns the matched step counter, so callers can
pin "last-used step ≥ N" and reject replays.

Tested against the published RFC 6238 Appendix B vectors
(T=59 → 287082, T=1111111109 → 081804, T=1111111111 → 050471).

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

## Out of scope (this PR)

- WebAuthn / passkeys / FIDO2-PRF.
- OPAQUE / SRP-6a aPAKE.
- OIDC / JWT / mTLS / AppRole / AWS-STS / GCP-JWT / Azure-MI /
  Kubernetes-SA.
- SMS / email-OTP / Duo push.
- Replay-cache integration: this crate exposes `verify_at_time`
  returning the matched step; persisting last-used-step per
  secret and rejecting replays is the caller's job.

## Citations

- RFC 4226 — *HOTP: An HMAC-Based One-Time Password Algorithm*.
  HOTP under HMAC-SHA-1; truncation per §5.3.
- RFC 6238 — *TOTP: Time-Based One-Time Password Algorithm*.
  Test vectors in Appendix B.
- RFC 9106 — *Argon2 Memory-Hard Function*. Used by
  `PasswordAuthenticator`.
- RFC 5869 — *HKDF*. Used by `combine_key_contributions`.
- VLT00-vault-roadmap.md — VLT05 layer purpose.
