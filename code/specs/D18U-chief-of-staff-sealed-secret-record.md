# D18U — Chief of Staff Sealed Secret Record

## Status

Specified 2026-10-07. Owner decisions taken in an attended session and
recorded on the D18S/D18R backlog (#13980, item P1.15). Implemented by
`code/packages/rust/chief-of-staff-vault-secret-store`.

## Why this exists

`ChiefVaultRuntime` (VLT06, "Per-secret admission policy") holds secrets in
memory and applies a per-secret policy to every lease and direct-delivery
request. `SealedStore` holds opaque encrypted bytes on disk. **Nothing
serialized between the two**, so the only thing that could ever populate the
vault was test code. A lease tool over a vault that cannot contain anything is
an entry point that can only fail, which is why the daemon's `[vault] kek_path`
is absent by default and no vault tool is offered today.

This spec defines the bytes that sit inside one sealed record, so that an owner
can provision a secret once, and the daemon can register it into the runtime at
startup with the policy the owner chose.

```text
owner ──stdin──▶ chief-of-staff vault put <name> --policy flags
                         │ encode (this spec)
                         ▼
                 SealedStore.put("chief-secrets", <name>, bytes)
                         │ XChaCha20-Poly1305, AAD = namespace ∥ 0x00 ∥ name
                         ▼
                    disk (vault storage_path)

daemon start ─▶ SealedStore.list + get ─▶ decode ─▶ ChiefVaultRuntime.register_secret
```

## Decisions

| # | Question | Decision |
|---|---|---|
| U-D1 | Is the record signed independently of the sealed store? | **No.** Integrity is the sealed store's AEAD alone. |
| U-D2 | Encoding | Binary envelope after the `oauth-credential-sealed-store` precedent: magic, version, length-prefixed fields, explicit bounds. |
| U-D3 | How does a secret get in? | `chief-of-staff vault put <name>` reads the value from **stdin, never argv**, and requires **explicit** policy flags. |
| U-D4 | How does the daemon see it? | It registers every record **at startup**. |
| U-D5 | Rotation | `put` a new value, then restart the daemon. Leases are in memory, so the restart revokes every outstanding lease (VLT06 P6). |

### Why no separate signature (U-D1)

`SealedStore` already binds each record's AEAD additional data to
`namespace ∥ 0x00 ∥ key`, so a ciphertext cannot be moved under a different
secret name, and any bit flip fails the tag. The only party that can produce a
record that verifies is one holding the KEK. A second signature would protect
against "holds the KEK but not the signing key" — a party who can already
decrypt every secret in the vault and so gains nothing by widening a policy
that they could not get by reading the payload directly. The cost would be a
second owner key to provision, rotate, and lose. Not worth it.

### Why startup-only (U-D4, U-D5)

A live write path would put `vault.put` on the daemon's control plane, which
is more surface to authenticate and harden for a write that happens a handful
of times a year. Startup registration composes with the property rotation
actually needs — *nothing minted against the old value survives* — for free,
because every lease dies with the process.

## The namespace and key

| Item | Value |
|---|---|
| Sealed-store namespace | `chief-secrets` |
| Record key | the secret name, verbatim |

**U-N1 — secret names are restricted.** A name is 1–128 bytes of ASCII,
starting with `[a-z0-9]`, continuing with `[a-z0-9._-]`, and containing no
`..`. The restriction exists because the name is used *verbatim* as a storage
key, which `storage-fs` maps to a path; a conservative charset rules out path
tricks, case-folding collisions on macOS and Windows, and Unicode confusables,
without an encoding layer. It is narrower than what the D18D vault tools accept
(`MAX_SECRET_NAME_BYTES = 512`, any string). That is safe in the direction it
errs: a name that cannot be stored cannot be leased, and the tool returns its
ordinary not-found denial.

## The envelope, version 1

All integers are big-endian. `string` is `u32 length ∥ UTF-8 bytes`.

| Offset | Field | Encoding | Bound |
|---|---|---|---|
| 0 | magic | 8 bytes `CHIEFSEC` | exact |
| 8 | version | `u8` | must be `1` |
| 9 | `privilege_tier` | `u8` | 0–3 |
| 10 | `allowed_mode` | `u8`: 0 Direct, 1 Leased, 2 Both | exact |
| 11 | `rotated_at_ms` | `u64` | any |
| 19 | `allowed_agents` tag | `u8`: 0 Any, 1 Only | exact |
| 20 | (Only) agent count | `u16` | 1–64 |
| … | (Only) each agent id | `string` | 1–256 bytes, no ASCII control characters, strictly ascending, no duplicates |
| … | payload | `u32 length ∥ bytes` | 1–65 536 bytes |

Nothing may follow the payload.

### Rules

**U-E1 — decode is total and closed.** Every byte is accounted for. A wrong
magic, an unknown version, an out-of-range tag or tier, a length past its
bound, a truncated field, or trailing bytes is a corruption error. Unknown
values are never mapped to a nearest neighbour — an unknown mode byte read as
`Both` would widen a policy silently.

**U-E2 — the policy precedes the payload.** A reader can reject a bad policy
before any secret bytes are copied out of the decrypted plaintext (VLT06 P4's
"refuse before materializing", applied to storage).

**U-E3 — `Only` with no agents is rejected.** A policy that admits nobody is a
provisioning mistake, not a way to disable a secret; disabling is deleting.
Rejecting it at both encode and decode makes it impossible to write one by
accident.

**U-E4 — agent ids are canonical.** Ids are written strictly ascending (which
is `BTreeSet` order), so one policy has exactly one encoding, and decode
rejects anything else. Without this, two encodings of one policy would be
indistinguishable to a reviewer comparing records and distinguishable to code.

**U-E5 — empty payloads are rejected.** A zero-length secret is never what the
owner meant, and accepting it would let a failed stdin read provision a
secret that leases successfully and authenticates nothing.

**U-E6 — secret bytes live only in zeroizing buffers**, allocated at their final
capacity up front. A growing `Vec` that reallocates leaves the old allocation
unzeroed, which the final zeroizing drop cannot reach.

**U-E7 — no secret in any error or `Debug` output.** Errors name the field
that failed and, where it is safe, the secret *name*; never the payload, never
an agent id read from a corrupt record.

### Versioning

The version byte gates the whole layout. A future version is a new decoder
arm, never a reinterpretation of version 1; a reader that does not know a
version refuses the record. Re-encoding happens only on `put`, so a vault can
hold mixed versions while the reader understands all of them.

## Loading

**U-L1 — load is all-or-nothing.** The startup loader lists the namespace,
pages through every record, and decodes each. If any record fails to decrypt
or decode, the load fails and names the offending secret. It does **not** skip
the record: a silently missing secret surfaces only as a refused legitimate
caller, far from its cause, and a vault that has been tampered with should stop
the daemon rather than serve whatever survived.

**U-L2 — the load is bounded.** At most 1 024 records. A vault past that bound
is refused rather than read into memory unbounded.

**U-L3 — `privilege_tier` is carried, not enforced.** VLT06 records that no
component reads it yet. This format stores it so that enforcing it later needs
no migration — and says so here, because a stored field reads as a control.

## What this does not do

- It does not decide which daemon tools are offered. That is P1.4's wiring:
  the vault tool source exists only when `[vault] kek_path` is configured and
  the load succeeded.
- It does not authenticate the person running `vault put`. Writing requires
  the KEK file, which is owner-only; that file *is* the authorization.
- It does not make `allowed_agents` finer than host granularity (VLT06,
  "What this does not do").
