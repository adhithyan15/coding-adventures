# `coding_adventures_vault_auth` — VLT05

Pluggable **authentication** for the Vault stack. The trait host
that lets a vault require any combination of factors — password,
TOTP, WebAuthn, FIDO2-PRF, OPAQUE, SMS, OIDC, mTLS, AppRole,
Kubernetes-SA, etc. — without the vault core caring which.

This v0.1 ships **PasswordAuthenticator** and **TotpAuthenticator**.

## Two operating modes

- **Gate** — pass/fail, no key material contributed (TOTP, SMS,
  classic WebAuthn).
- **Bind** — contributes key material to the unlock derivation
  (password, FIDO2-PRF, 1Password-style Secret Key, Shamir
  shares).

The vault calls `combine_key_contributions(vault_id, factors)`
which HKDF-extracts over the ordered concatenation of bind-mode
factor outputs.

## Quick example

```rust
use coding_adventures_vault_auth::{
    Authenticator, AuthError, Mode, PasswordAuthenticator,
    TotpAuthenticator, combine_key_contributions,
};

// Registration: derive verifier and store (salt, params, verifier).
let salt = b"saltsaltsaltsalt".to_vec();
let verifier = PasswordAuthenticator::derive_verifier(
    b"correct horse battery staple",
    &salt,
    /* t */ 3, /* m_kib */ 64*1024, /* p */ 4, /* tag_len */ 32,
)?;
let pw = PasswordAuthenticator::with_verifier(salt, 3, 64*1024, 4, verifier)?;

// Verification at unlock time.
let assertion = pw.verify(b"correct horse battery staple")?;

// Combine bind-mode contributions into a 32-byte unlock key.
let unlock_key = combine_key_contributions(b"vault-id-abcdef", &[&assertion])?;
// `unlock_key` is Zeroizing<[u8; 32]>; pass it to VLT01 as the master KEK.
```

For TOTP-as-2FA on top:

```rust
let totp = TotpAuthenticator::new(seed.into(), /* period */ 30, /* digits */ 6, /* window */ 1)?;
let _gate = totp.verify(b"123456")?;  // gate-mode, no key contribution
```

## RFC 6238 conformance

`TotpAuthenticator` is tested against the published RFC 6238
Appendix B vectors:

| T (s)         | Step          | 6-digit code |
|---------------|---------------|--------------|
| 59            | 1             | `287082`     |
| 1 111 111 109 | 37 037 036    | `081804`     |
| 1 111 111 111 | 37 037 037    | `050471`     |

`verify_at_time(code, unix_time)` returns the matched step counter
so callers can pin "last-used step" into a per-secret cache and
reject replays at the layer above.

## Where it fits

```text
                    ┌──────────────────────────────────────┐
                    │  application                         │
                    └──────────────┬───────────────────────┘
                                   │
                    ┌──────────────▼───────────────────────┐
                    │  vault-auth (VLT05)               ◄  │  THIS CRATE
                    │  Authenticator trait + Pwd + TOTP    │
                    └──────────────┬───────────────────────┘
                                   │  bind-mode key_contribution
                                   ▼
                    ┌──────────────────────────────────────┐
                    │  vault-policy (VLT06)                │
                    │  decides "did the right factors      │
                    │  pass for this action?"              │
                    └──────────────┬───────────────────────┘
                                   │ unlock_key
                                   ▼
                    ┌──────────────────────────────────────┐
                    │  vault-key-custody (VLT03)           │
                    │  uses unlock_key as KEK input        │
                    └──────────────────────────────────────┘
```

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT05-vault-auth.md`](../../../specs/VLT05-vault-auth.md).
