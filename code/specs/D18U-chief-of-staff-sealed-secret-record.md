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

**What the AEAD does not stop: rollback.** The AEAD proves a record was
written by a KEK holder, but not that it is the latest record. A party who can
*write* the storage directory but holds no KEK could once restore an older
ciphertext file for the same name, from a backup, a filesystem snapshot or a
sync folder, and it still verified. On the next restart the daemon would
register the old policy and the old value. That undoes a narrowed allow-list,
the rotation of a leaked secret, or a delete. A second *signature* would not
fix this either, because an old record is just as validly signed.

`vault-sealed-store` now binds freshness (VLT01 F1-F10, P1.20). A sealed
per-namespace index records each key's newest generation and AEAD tag, and
each record binds its generation into its AAD. These now read as `Tamper`,
and `register_all` fails closed on them:

- restoring one record file;
- resurrecting a deleted one;
- deleting the index.

The Chief vault also opens its store with a freshness anchor (VLT01 F11). It is
kept in `<kek_path>.freshness/`, next to the owner-only KEK file and outside
the storage directory. The anchor closes F10's remaining cases, so restoring
an old index together with an old record is also `Tamper`, across restarts.
The trust assumption moves from the storage directory to the KEK's own
directory, which the KEK file's owner-only check already relies on. The
operating requirement is now that **the KEK's directory is writable only by
the owner**. If someone else can write the storage directory, they still
cannot roll the Chief vault back.

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

**U-N1 — secret names are restricted.** A name is 1–120 bytes of ASCII,
starting with `[a-z0-9]`, continuing with `[a-z0-9._-]`, and containing no
`..`. The restriction exists because the name is used *verbatim* as a storage
key, which `storage-fs` maps to a path; a conservative charset rules out path
tricks, case-folding collisions on macOS and Windows, and Unicode confusables,
without an encoding layer. The length bound comes from the same mapping:
`storage-fs` hex-encodes the key into the file name, doubling it, so 120 bytes
is a 240-character file name against Linux's 255-byte `NAME_MAX` (a
128-byte bound, as first drafted, would have failed on write). It is narrower than what the D18D vault tools accept
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

## The envelope, version 2 (P1.21)

Version 2 is version 1 with one section added between the agent list and the
payload: the destinations the secret may be sent to (D18V V-S7, VLT06 P9).

| Field | Encoding | Bound |
|---|---|---|
| magic … agent ids | as in version 1, with version byte `2` | as in version 1 |
| destination count | `u16` | 0–32 |
| each destination | `string` | `host:port`, see U-E8; strictly ascending, no duplicates |
| payload | `u32 length ∥ bytes` | 1–65 536 bytes |

**U-E8 — a destination is a DNS name and a port, canonical.** The host is 1–253
bytes of lowercase labels (`[a-z0-9-]`, each 1–63 bytes, not starting or ending
with `-`), separated by single dots, with no trailing dot. It is never an IP
literal, because `net.fetch` refuses IP-literal hosts (D18V V-A1), so an IP
destination could never be used. The port is decimal 1–65 535 with no leading
zeros. Destinations are written strictly ascending, which is U-E4's rule
applied to a second list.

**U-E9 — an empty destination list means "nowhere".** A secret with no
destinations can still be leased or delivered directly, but `net.fetch` will
never write it into a request. **A version 1 record decodes with no
destinations**, so a secret provisioned before version 2 is not usable with
`net.fetch` until it is re-`put`. This is the conservative direction: the
alternative, treating "absent" as "anywhere", is the exact mistake VLT06 P3
exists to forbid.

The encoder always writes version 2. The decoder reads versions 1 and 2.

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

**U-L1a — paging follows the backend's cursor.** The loader pages with
`SealedStore::list_page` and stops only when `next_cursor` is `None`. A short
page is **not** the end: `storage-fs` drops a key deleted between its directory
scan and its read, so a page can come back short with more to come, and a
loader that stopped there would miss every later record — the silent gap U-L1
exists to rule out.

**U-L2 — the load is bounded.** At most 1 024 records. A vault past that bound
is refused rather than read into memory unbounded.

**U-L3 — `privilege_tier` is carried, not enforced.** VLT06 records that no
component reads it yet. This format stores it so that enforcing it later needs
no migration — and says so here, because a stored field reads as a control.

## Provisioning commands (P1.4b)

The owner writes records with the operator CLI. The commands run **locally**.
They open the same vault storage root the daemon uses, with the same
owner-only KEK file, and never contact the daemon. Writing a secret is a
filesystem operation that needs the KEK, and the KEK file is the
authorization.

```text
chief-of-staff vault put <NAME> --mode direct|leased|both --tier 0..3
                               (--allow-agent HOST)... | --any-agent
                               [--destination HOST:PORT]...
                               [--raw]                  < secret-on-stdin
chief-of-staff vault delete <NAME>
chief-of-staff vault list
```

**U-C1 — the secret comes from stdin, never argv.** argv is visible in `ps`,
shell history and process accounting. The command reads at most
`MAX_PAYLOAD_BYTES + 3` bytes: room for a maximum-size secret, the `\r\n`
that U-C3 strips, and one byte more, so an oversized input is refused rather
than truncated. It reads through an unbuffered duplicate of the stdin
descriptor, because the standard library's shared stdin buffer would keep a
copy of the secret that nothing zeroes.

Errors printed to stderr never quote an argv value. A secret pasted onto
argv by mistake is refused as an extra argument, and the parser error for that
names only its kind, not the token.

**U-C2 — an interactive terminal on stdin is refused.** Reading a secret from
a terminal without disabling echo would print it to the screen. The command
says to pipe the value in, for example from a password manager. Echo-disabled
prompting is a possible later addition, not a fallback.

**U-C3 — one trailing newline is stripped unless `--raw` is given.**
`printf '%s\n' "$TOKEN" | chief-of-staff vault put …` and `echo` both append
`\n`. Keeping it would provision a value that never matches. Exactly one
`\n`, or one `\r\n`, is removed. `--raw` keeps the bytes exactly, for
binary secrets. A value that is empty after stripping is refused (U-E5).

**U-C4 — every policy flag is mandatory.** `--mode` and `--tier` have no
default. Exactly one of `--allow-agent` (repeatable) or `--any-agent` is
required. This is VLT06 P5 carried to the command line: a secret must not
become permissive because a flag was omitted, and the owner must type
`--any-agent` to say a secret is unguarded.

**U-C4a — a leasable secret names its destinations.** `--mode leased` and
`--mode both` require at least one `--destination HOST:PORT` (repeatable, at
most 32). `--mode direct` refuses `--destination`, because a direct-only
secret is never leased and so never reaches `net.fetch`. Destinations follow
U-E8 and are lowercased before validation.

**U-C5 — agent names are host names.** The daemon attests a caller as its
registered host name (`ToolContext::agent_id` is the registration's
`host_name`). So `--allow-agent` values are validated with the service
registry's host-name grammar. That makes it impossible to write an allow-list
entry that no caller could ever match.

**U-C6 — output never contains the value.** `put` and `delete` print the
action, the secret name, and that the change takes effect at the next daemon
restart (U-D5). `list` prints names only and decrypts nothing.

**U-C7 — the vault must be configured.** Without `[vault] kek_path` the
commands refuse and name the missing setting. They do not invent a key.

### Shared storage root

`[vault] storage_path` is also the storage root of the six smart-home pairing
vaults. A `SealedStore` root has one KEK manifest, so when `[vault] kek_path`
and a pairing `kek_path` are both configured **they must name the same KEK
file**, or whichever opens second fails with `InvalidKek`.

`storage-fs` serializes writers inside one process only. A `vault put` while
the daemon is running is safe for the record itself: the write is an atomic
rename, and the last writer wins. The one cross-process race is the first
write to a namespace, which records the namespace in the store's registry. If
the daemon registers a pairing namespace at the same instant, one registration
can be lost, and a later `rotate_kek` would skip that namespace. The race
window is a single first write, so it is documented here rather than locked
against.

## What this does not do

- It does not decide which daemon tools are offered. That is P1.4's wiring:
  the vault tool source exists only when `[vault] kek_path` is configured and
  the load succeeded.
- It does not authenticate the person running `vault put`. Writing requires
  the KEK file, which is owner-only; that file *is* the authorization.
- It does not make `allowed_agents` finer than host granularity (VLT06,
  "What this does not do").
