# `coding_adventures_vault_auth` — VLT05

Pluggable **authentication** for the Vault stack. The trait host
that lets a vault require any combination of factors — password,
TOTP, WebAuthn, FIDO2-PRF, OPAQUE, SMS, OIDC, mTLS, AppRole,
Kubernetes-SA, etc. — without the vault core caring which.

This v0.1 ships **PasswordAuthenticator** and **TotpAuthenticator**,
plus a **`WebAuthnPrfAuthenticator` scaffold** — the trait plumbing
and registration-time shape (relying-party id, credential id, COSE
public key) for a FIDO2 hardware security key (YubiKey and other
CTAP2-compliant authenticators) as a bind-mode unlock factor via the
CTAP2 `hmac-secret` extension. `verify()` always returns
`AuthError::Unimplemented` until a follow-up PR adds real hardware
I/O and ECDSA P-256 signature verification — see
[`VLT-PM51-hardware-security-keys.md`](../../../specs/VLT-PM51-hardware-security-keys.md)
for the full design, the protocol/dependency survey, and why real
hardware support is deferred rather than partially built.
Successful assertions can also be projected into credential-safe
`AuthAssertionSummary` / `AuthAssertionSetSummary` read models so
policy and audit layers can inspect factor coverage without touching
key-contribution bytes. Set summaries include built-in versus
extension factor counts and gate/bind contribution consistency flags
for host-side policy checks.

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
let totp = TotpAuthenticator::new(
    seed.into(),
    TotpAlgorithm::Sha1,
    /* period */ 30,
    /* digits */ 6,
    /* window */ 1,
)?;
let _gate = totp.verify(b"123456")?;  // gate-mode, no key contribution
```

The algorithm is a required argument with no `Default`. RFC 6238 §1.2
names three HMAC variants, a provisioned seed carries its own, and six
wrong digits look exactly like six right ones — so the parameter that
decides which is which is never chosen on a caller's behalf.

### Generating, not only verifying

The same type is the generator a password manager needs to display the
current code for a stored seed:

```rust
let code = totp.formatted_code_at(unix_time_sec)?;   // Zeroizing<String>, zero-padded
let left = totp.remaining_seconds(unix_time_sec);    // 1..=period
```

`formatted_code_at` pads to the configured width because roughly one code
in ten has a leading zero and `042311` is not `42311`. Both the code and
the buffer holding it are wipe-on-drop. `remaining_seconds` is never `0`:
a code with zero seconds left has already been replaced by the next one.

## RFC 6238 conformance

`TotpAuthenticator` is tested against **every** published RFC 6238
Appendix B vector — all six timestamps against all three algorithms, at
the published 8-digit width, plus the 6-digit truncation most apps
render:

| T (s)          | SHA-1      | SHA-256    | SHA-512    |
|----------------|------------|------------|------------|
| 59             | `94287082` | `46119246` | `90693936` |
| 1 111 111 109  | `07081804` | `68084774` | `25091201` |
| 1 111 111 111  | `14050471` | `67062674` | `99943326` |
| 1 234 567 890  | `89005924` | `91819424` | `93441116` |
| 2 000 000 000  | `69279037` | `90698825` | `38618901` |
| 20 000 000 000 | `65353130` | `77737706` | `47863826` |

Each algorithm uses its own Appendix B seed (20, 32, and 64 ASCII bytes
of repeating `1234567890`), which is what makes the table a real test of
the algorithm selector rather than of one hash three times.

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

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md),
[`VLT05-vault-auth.md`](../../../specs/VLT05-vault-auth.md), and
[`VLT-PM51-hardware-security-keys.md`](../../../specs/VLT-PM51-hardware-security-keys.md).
