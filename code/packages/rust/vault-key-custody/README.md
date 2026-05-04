# `coding_adventures_vault_key_custody` — VLT03

Pluggable **Key Custody** for the Vault stack. Abstracts where the
master KEK lives — passphrase, TPM, Secure Enclave, HSM, Cloud KMS,
YubiKey-PRF — behind one trait so the rest of the vault can
compose against the trait, not against any specific backend.

**Hardware custodians are FIRST-CLASS preferred.** When a TPM /
Secure Enclave is available on the host, the vault refuses to fall
back to a software passphrase custodian unless the caller passes an
explicit `force_software` flag. Side-channel attack surface (cold-
boot RAM scraping, debugger attaches, swap files, core dumps) is
correspondingly reduced.

## Quick example

```rust
use coding_adventures_vault_key_custody::{
    PassphraseCustodian, TpmCustodian, KeyCustodian, select_custodian, fresh_random_key,
};

// 1. Build candidate custodians the host supports.
let pw = PassphraseCustodian::with_default_params(b"correct horse battery staple".to_vec())?;
let tpm = TpmCustodian::detected("tpm-2.0");
let candidates: Vec<&dyn KeyCustodian> = vec![&pw, &tpm];

// 2. The selector picks the hardware-bound one if present.
let chosen = select_custodian(&candidates, /* force_software = */ false)?;

// 3. Wrap the vault's master KEK and persist the wrapped blob.
let label = b"vault/master".to_vec();
let master_kek = fresh_random_key()?;
let wrapped = chosen.wrap(&label, &master_kek)?;
// wrapped.0 is opaque bytes you can store anywhere.

// 4. Later, on unseal:
let unwrapped = chosen.unwrap(&label, &wrapped)?;
// Use unwrapped (Zeroizing<[u8; 32]>) as the KEK input to VLT01.
```

## What's in this version (v0.1)

- `KeyCustodian` trait + `CustodianCaps`.
- `PassphraseCustodian` — Argon2id KDF + XChaCha20-Poly1305 AEAD
  wrap. 90-byte wrapped blob. AAD-bound to its label.
- `TpmCustodian` scaffold — reports the right capability shape so
  `select_custodian` prefers it; `wrap`/`unwrap` return
  `Unimplemented` until the platform-specific TPM 2.0 backend
  lands in a follow-up PR.
- `select_custodian` + `assert_no_hardware_bypass` — TPM-first
  policy helpers.

## Future work (separate PRs)

- Real `TpmCustodian` backends:
  - Linux: `/dev/tpmrm0` via `tpm2-tss` / `tss-esapi`.
  - Windows: TBS (TPM Base Services).
  - macOS: Secure Enclave via `LocalAuthentication` /
    `SecKeyCreateWithData`.
- `OSKeystoreCustodian` — macOS Keychain / Windows DPAPI /
  Linux libsecret.
- `Pkcs11Custodian` — generic HSM via PKCS#11.
- `AwsKmsCustodian` / `GcpKmsCustodian` / `AzureKvCustodian` —
  Cloud KMS auto-unseal.
- `YubikeyPrfCustodian` — FIDO2 hmac-secret / PRF as a wrapping
  primitive (matches Bitwarden / 1Password's hardware-bound
  unlock).
- `DistributedFragmentCustodian` — Shamir-split KEK held by N
  operators (uses the already-shipped `coding_adventures_shamir`
  crate).

## Where it fits in the Vault stack

```text
                            ┌─────────────────────────────┐
                            │  application                │
                            └──────────────┬──────────────┘
                                           │
                            ┌──────────────▼──────────────┐
                            │  vault-sealed-store (VLT01) │
                            │  envelope encryption        │
                            └──────────────┬──────────────┘
                                           │  needs a 32-byte KEK
                                           ▼
                            ┌─────────────────────────────┐
                            │  vault-key-custody (VLT03) ◄│  THIS CRATE
                            │  pluggable wrap/unwrap      │
                            └──┬─────────┬───────┬────────┘
                               │         │       │
                       ┌───────▼──┐ ┌────▼───┐ ┌─▼────────┐ ┌───────────────┐
                       │ Passphr. │ │  TPM   │ │ Cloud KMS│ │ YubiKey-PRF…  │
                       └──────────┘ └────────┘ └──────────┘ └───────────────┘
```

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT03-vault-key-custody.md`](../../../specs/VLT03-vault-key-custody.md).
