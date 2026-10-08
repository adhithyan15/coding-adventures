# Chief of Staff Vault Secret Store

Persists Chief of Staff vault secrets, each with its admission policy, inside
`vault-sealed-store`. The format is specified in
[`code/specs/D18U-chief-of-staff-sealed-secret-record.md`](../../../specs/D18U-chief-of-staff-sealed-secret-record.md).

## Why it exists

`chief-of-staff-vault-runtime` decides, per secret, which agents may request it
and whether it may be leased or only delivered directly (VLT06). But it held
secrets only in memory, and `vault-sealed-store` holds only opaque bytes. Nothing
serialized between the two, so nothing outside a test could ever put a secret
into the vault. This crate fills that gap.

## Where it sits

```text
chief-of-staff vault put ──▶ ChiefSecretStore::put ──▶ SealedStore (disk, AEAD)
                                                              │
daemon startup ──▶ ChiefSecretStore::register_all ◀───────────┘
                          │
                          ▼
                 ChiefVaultRuntime  ──▶  D18D vault.request_lease
```

The crate opens nothing itself. The caller passes in an already-unsealed
`SealedStore`, and the CLI and the daemon each already know how to read the
owner-only KEK file. So this crate has no filesystem authority of its own.

## Usage

```rust
use chief_of_staff_vault_runtime::{AllowedAgents, ChiefVaultRuntime, SecretPolicy, VaultDeliveryMode};
use chief_of_staff_vault_secret_store::{ChiefSecretStore, SecretName};

// Provisioning (the CLI): the secret comes from stdin, never argv.
let store = ChiefSecretStore::new(unsealed_store);
store.put(
    &SecretName::parse("weather-api-key")?,
    &SecretPolicy {
        privilege_tier: 1,
        allowed_agents: AllowedAgents::only(["weather-agent"]),
        allowed_mode: VaultDeliveryMode::Leased,
        rotated_at_ms: now_ms,
    },
    &secret_bytes_from_stdin,
)?;

// Startup (the daemon): load every record or none, then register them.
let runtime = ChiefVaultRuntime::new();
let count = store.register_all(&runtime)?;
```

## What it guarantees

| Rule | Guarantee |
|---|---|
| U-D1 | Integrity comes from the sealed store's AEAD. The AAD binds `chief-secrets ∥ 0 ∥ name`, so a record cannot be renamed and cannot be edited. |
| U-N1 | Secret names are `[a-z0-9][a-z0-9._-]{0,119}` with no `..`. They are used verbatim as storage keys and can never form a path. 120 bytes hex-encodes to a 240-character file name, under `NAME_MAX`. |
| U-E1 | Decoding is total and closed. An unknown tag, an out-of-range bound, truncated input or trailing bytes is an error, never a nearest value. |
| U-E2 | The policy precedes the payload, so a bad policy is rejected before any secret byte is copied. |
| U-E3 | An `Only` allow-list with no agents is refused when writing and when reading. |
| U-E4 | Agent ids are strictly ascending, so each policy has exactly one encoding. |
| U-E5 | An empty payload is refused, so a failed stdin read cannot provision a secret. |
| U-E6 | Secret bytes live only in zeroizing buffers. Each is allocated once at its final capacity. |
| U-E7 | No error and no `Debug` output contains a payload or an agent id read from disk. |
| U-L1 | Loading is all-or-nothing. A corrupt record stops the load and is named, never skipped. Paging follows the backend's cursor, so a short page does not end the listing. |
| U-L2 | At most 1024 records are loaded. |
| U-E8 | Destinations are canonical `host:port`: a DNS name, never an IP literal, strictly ascending, at most 32. |
| U-E9 | Version 1 records decode with no destinations. Absent never means anywhere. |

**Rollback is detected for single files, not whole snapshots.** The sealed
store underneath keeps a sealed freshness index (VLT01 F1-F10). Restoring an
older record file, or one that was deleted, reads as `Tamper`, and
`register_all` refuses to load. An old index restored *with* an old record it
pins is not yet detected after a restart (P1.20b). So keep the vault directory
writable only by the owner.

`privilege_tier` is stored but not enforced. VLT06 records that nothing reads
it yet.

## Rotation

`put` overwrites. A running daemon sees the new value only after a restart. The
restart is the point: leases live in memory, so restarting revokes every lease
minted against the old value.

## Testing

```sh
cargo test -p chief-of-staff-vault-secret-store
```

The codec tests build envelopes byte by byte instead of round-tripping. A
round-trip test cannot catch an encoder and decoder that agree with each other
but disagree with the spec.
