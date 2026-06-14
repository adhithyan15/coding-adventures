# VLT03 — Vault Key Custody

## Overview

The **key custody** layer of the Vault stack. Abstracts where the
master KEK lives — passphrase, TPM 2.0 / Secure Enclave, OS keystore,
PKCS#11 HSM, Cloud KMS, YubiKey-PRF, distributed-fragment Shamir
quorum — behind one trait so the rest of the vault composes against
that trait, not against any specific backend.

This document specifies the trait, the capability model, the
TPM-first / hardware-preferred policy, the `PassphraseCustodian`
wire format, and what's deliberately deferred to follow-up PRs.
Implementation lives at `code/packages/rust/vault-key-custody/`.

## Why this layer exists

VLT01 (sealed store) needs a 32-byte master KEK to wrap per-record
DEKs. The original VLT01 spec derived that KEK from a single
passphrase via Argon2id, baked in. That's fine for a local-only
file-based vault but leaves the rest of the world unaddressed:

- **TPM-bound deployments** want the wrapping key never to leave
  the TPM — every unwrap crosses the hardware boundary.
- **Secure Enclave / Touch ID** flows want biometric-gated
  unwrap.
- **Cloud auto-unseal** wants the wrapping key to live in AWS KMS
  / GCP Cloud KMS / Azure Key Vault Managed HSM — the vault
  process just sees opaque wrap/unwrap operations.
- **YubiKey-PRF** (Bitwarden / 1Password / KeePassXC pattern)
  derives the wrapping key from a hardware-bound HMAC.
- **Quorum unseal** (HashiCorp Vault style) splits the wrapping
  key into Shamir shares held by N operators, K of whom must
  collaborate.

VLT03 makes all of these pluggable.

## TPM-first / hardware-preferred policy

### The rule (refinement, 2026-05-04)

When the host machine has a hardware custodian available, the
vault refuses to instantiate a software (passphrase) custodian
unless the caller passes an explicit `force_software` flag. The
helper functions `select_custodian` and `assert_no_hardware_bypass`
encode this rule.

### Why

A user with a TPM in their laptop expects their vault key not to
live in process heap in extractable form. If the application
silently falls back to a software custodian (because the TPM
binding wasn't wired up, or because the deployment forgot to
enable hardware mode), the threat model changes invisibly:

| Threat                              | Hardware custodian | Software custodian |
|-------------------------------------|--------------------|--------------------|
| Cold-boot RAM scraping              | Resists            | Vulnerable         |
| Debugger / `ptrace` attach          | Resists            | Vulnerable         |
| Swap to disk, core dump             | Resists            | Vulnerable         |
| Kernel-mode adversary               | Resists most cases | Vulnerable         |
| Lost laptop, no passphrase typed    | Resists            | Resists            |

The TPM-first rule preserves the stronger guarantee by default,
and forces the caller to consciously waive it.

### How

`CustodianCaps` reports whether each candidate is `hardware_bound`
and whether its key material is `extractable`. `select_custodian(
candidates, force_software)` walks the list:

1. If any candidate is hardware-bound → pick the first
   hardware-bound one.
2. Else → pick the first candidate (software).
3. If `force_software` is set → pick the first non-hardware
   candidate, even when hardware is present (for tests / migration
   paths).

`assert_no_hardware_bypass(candidates, host_has_hw,
force_software)` is an advisory boot-time check — it returns
`HardwareAvailableButSoftwareRequested` when a software-only
candidate list is presented on a host that the application has
detected has hardware support, and `force_software` isn't set.

The "host has hardware" detection itself is delegated to the
caller — it depends on OS / vendor APIs (TPM 2.0 device probe,
`SecKeyCreateWithData` test, etc.) that this crate doesn't try to
encapsulate.

## `KeyCustodian` trait

```rust
pub trait KeyCustodian {
    fn name(&self) -> &str;                                        // "passphrase", "tpm-2.0", …
    fn capabilities(&self) -> CustodianCaps;
    fn wrap(&self, label: &Label, key: &Key) -> Result<WrappedKey, CustodyError>;
    fn unwrap(&self, label: &Label, wrapped: &WrappedKey) -> Result<Key, CustodyError>;
}

pub struct CustodianCaps {
    pub hardware_bound:         bool,    // bound to a hardware secret
    pub extractable:            bool,    // can the wrapping key escape the custodian?
    pub requires_user_presence: bool,    // touch / biometric on every unwrap
    pub remote:                 bool,    // network round-trip on every unwrap
}
```

`Label` is `Vec<u8>` — opaque to this crate. The custodian uses it
to find the right wrapping key (passphrase: AAD-binds the wrapped
blob to its label; TPM: persistent handle; Cloud KMS: CMK ARN).

`Key` is `Zeroizing<[u8; 32]>` — wrapped at the type level so it
wipes on every drop, including early returns.

## `PassphraseCustodian` (the software baseline)

The first concrete implementation. Used by:

- File-based vaults (KeePassXC class).
- Test environments.
- Hosts without hardware support.

### Construction

- `with_default_params(passphrase)` — uses Argon2id `time=3,
  memory=64 MiB, parallelism=4` (RFC 9106 baseline).
- `with_params(passphrase, time_cost, memory_cost, parallelism)` —
  caller-controlled.

Validates: passphrase non-empty; `time_cost >= 1`; `memory_cost
>= 8` (KiB); `parallelism >= 1`. Rejects with `InvalidParameter`.

### Wire format

```text
   wrapped_blob = magic(2) || salt(16) || nonce(24) || ct(32) || tag(16)
   total = 90 bytes
   magic = b"P1"  (PassphraseCustodian, version 1)
   AAD   = magic || label
```

The `magic` lets the decoder fail-fast on a blob produced by a
different custodian (e.g. a TPM custodian's blob handed to a
passphrase custodian). The AAD-binds-to-label property means a
blob saved under one slot can't be replayed under another.

### Algorithm

- **Wrap**: fresh CSPRNG salt + nonce. Derive KEK =
  `Argon2id(passphrase, salt, t, m, p, 32 bytes)`. AEAD-encrypt
  the inner key with `XChaCha20-Poly1305(KEK, nonce, AAD,
  inner_key)`. Compose blob.
- **Unwrap**: parse blob, reject on length mismatch or magic
  mismatch (`MalformedWrappedKey`). Re-derive KEK from passphrase
  + salt. AEAD-decrypt; failure (any cause) returns
  `InvalidPassphrase`. The `InvalidPassphrase` variant is
  intentionally undifferentiated — wrong passphrase, wrong label,
  body tamper, salt tamper all collapse to the same error so
  there's no oracle distinguishing them.

### Drop semantics

`PassphraseCustodian` holds `passphrase: Zeroizing<Vec<u8>>` and
implements `Drop` calling `zeroize()` for belt-and-braces.

Inside `wrap` / `unwrap`, the derived KEK is `Zeroizing<[u8; 32]>`
so it wipes on every return path including errors. Heap byte
buffers from `argon2id` and `xchacha20_poly1305_aead_decrypt` are
explicitly zeroed before drop.

## `TpmCustodian` (scaffold)

Reports `CustodianCaps::HARDWARE_LOCAL` so `select_custodian`
correctly prefers it. Returns `CustodyError::Unimplemented {
backend: "TPM 2.0 / Secure Enclave" }` from `wrap` / `unwrap`
until the platform-specific backend lands in a follow-up PR.

This lets:

- Boot-time detection / fallback logic compile and run today.
- Tests for `select_custodian` use a real `TpmCustodian` rather
  than mocks.
- Downstream code (the vault's KEK initialisation flow) can branch
  on `caps.hardware_bound` already.

Future PRs:

- **Linux**: `/dev/tpmrm0` via `tss-esapi`. Uses a TPM persistent
  handle as the wrapping key (sealed under the SRK); `wrap` is
  `Tss2_Sys_Create + Tss2_Sys_Load`-style; `unwrap` is
  `TPM2_Unseal`. Label = persistent handle.
- **Windows**: TBS (TPM Base Services) via `tbs.h`.
- **macOS**: Secure Enclave via `LocalAuthentication` and
  `SecKeyCreateWithData(kSecAttrTokenIDSecureEnclave, …)`. Wrap =
  `SecKeyCreateEncryptedData`. Biometric-gated via
  `LAContext` (sets `requires_user_presence = true`).

## `CustodyError`

```rust
pub enum CustodyError {
    InvalidPassphrase,
    MalformedWrappedKey,
    InvalidParameter { what: &'static str },
    Csprng,
    Kdf,
    Aead,
    HardwareAvailableButSoftwareRequested,
    NoCandidates,
    Unimplemented { backend: &'static str },
}
```

`Display` strings are `&'static str` literals; attacker-controlled
bytes never appear in error output.

## Threat model & test coverage

| Threat                                                                             | Defence                                                                                                  | Test                                                            |
|------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------|
| Wrong passphrase silently produces garbage                                         | AEAD; fail-closed `InvalidPassphrase`                                                                    | `wrong_passphrase_fails_closed`                                 |
| Slot blob replayed under a different label                                         | AAD = magic ‖ label; fail-closed                                                                         | `wrong_label_fails_closed`                                      |
| Body tamper / salt tamper / nonce tamper                                           | AEAD detects; fail-closed                                                                                | `body_tamper_fails_closed`                                      |
| Wrong-custodian blob handed in (e.g. TPM blob → passphrase custodian)              | Magic prefix; `MalformedWrappedKey`                                                                      | `magic_tamper_is_malformed`                                     |
| Truncated / corrupted blob length                                                  | Length pre-check; `MalformedWrappedKey`                                                                  | `truncated_blob_is_malformed`                                   |
| Empty / weak passphrase                                                            | Constructor validation                                                                                   | `empty_passphrase_rejected`, `zero_time_cost_rejected`          |
| **Software custodian silently used when hardware available**                       | TPM-first / `select_custodian` + `assert_no_hardware_bypass` refuse                                      | `select_picks_hardware_when_available`, `assert_no_hardware_bypass_*` |
| Forced software path (test / migration) needs to bypass intentionally              | `force_software` flag                                                                                    | `select_with_force_software_picks_software_when_both_present`   |
| Empty candidate list                                                               | `NoCandidates`                                                                                           | `select_rejects_empty_candidates`                               |
| Distinguishing oracle: wrong passphrase vs wrong label vs body tamper              | All collapse to `InvalidPassphrase`                                                                      | three tests above all expect same variant                       |
| KEK lingering in process heap after unwrap                                         | `Zeroizing<[u8; 32]>` at every step; explicit `zeroize` of intermediate Vec buffers                      | covered by `Zeroizing` upstream tests + visual review            |
| Passphrase lingering in `PassphraseCustodian` heap after drop                      | `Drop` impl + `Zeroizing<Vec<u8>>` field                                                                 | `custodian_drop_is_safe` (smoke)                                 |
| Attacker-controlled bytes in error logs                                            | All `Display` strings are static literals                                                                | `error_messages_are_static_literals`                            |

## Out of scope (this PR)

- Real TPM 2.0 backends (Linux/Windows/macOS) — separate PRs.
- `OSKeystoreCustodian` (Keychain / DPAPI / libsecret).
- `Pkcs11Custodian` (HSMs, smartcards).
- `AwsKmsCustodian` / `GcpKmsCustodian` / `AzureKvCustodian`.
- `YubikeyPrfCustodian` (FIDO2 hmac-secret).
- `DistributedFragmentCustodian` (Shamir quorum unseal — built on
  the already-shipped `coding_adventures_shamir` crate).

## Citations

- RFC 9106 — *Argon2 Memory-Hard Function*. Default parameters
  (`time=3, memory=64 MiB, parallelism=4`) follow §4.
- TCG TPM 2.0 Library Specification — `TPM2_Create`, `TPM2_Unseal`
  for the future TPM backend.
- Apple Platform Security guide — Secure Enclave + biometric gating.
- HashiCorp Vault internals — `auto-unseal` and `awskms` /
  `gcpckms` / `azurekeyvault` seal types — model for the Cloud KMS
  custodians.
- VLT00-vault-roadmap.md — VLT03 layer purpose; VLT01 — the
  consumer of the master KEK.
