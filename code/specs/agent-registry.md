# Agent Registry

## Overview

The Agent Registry is the orchestrator's allowlist of every agent
package that may be launched on this machine. Every entry in the
registry pins a specific signed package by its **hash** — not just
its signature — so that even an attacker who has stolen a valid
signing key cannot smuggle in a different binary under an
otherwise-trusted package name.

The registry is stored inside the vault under a dedicated
namespace (`vault://orchestrator/agent-registry/`), so:

- Modifying it requires Tier 3 authentication (hardware key) per
  the vault auth policy.
- The registry is encrypted at rest with the vault master key.
- The registry's contents are tamper-evident: any modification
  triggers the audit log.

This is a strictly stronger trust model than signature-only
verification:

| Layer                | Catches                                   |
|----------------------|-------------------------------------------|
| Ed25519 signature    | "this came from a valid signing key"      |
| Hash pin (registry)  | "this is the **exact bytes** we approved" |
| Manifest cage        | "this is what these bytes are allowed to do" |
| OS sandbox           | "and the OS will enforce that, too"       |

A signature alone catches packages from random third parties.
A hash pin additionally catches:

- A new build of an existing package whose source has changed
  (even if signed with the same key).
- A package replaced on disk between registration and launch (a
  TOCTOU attack against the orchestrator's filesystem).
- A signing-key compromise where the attacker has a valid key but
  no entry in the registry for the malicious bytes they want to
  run.
- Accidental "I just rebuilt my agent and forgot to re-register
  it" mistakes — the orchestrator refuses to launch an unfamiliar
  build until it is explicitly approved.

The registry is a **gate**, not a database. Adding to it is a
deliberate act that pages the user (Tier 3 challenge); removing
from it is the same. Listing and inspecting are unrestricted
read-only operations.

---

## Where It Fits

```
                    User (Tier 3 challenge for mutations)
                              │
                              ▼
   Agent Registry  ← THIS SPEC
   ┌──────────────────────────────────────────────────────────┐
   │  in-vault namespace: vault://orchestrator/agent-registry/│
   │  one record per registered agent                         │
   │  fields:                                                  │
   │    name, package_path, package_hash (SHA-256),           │
   │    signing_key_id, registered_at, registered_by,         │
   │    privilege_tier_cap, manifest_hash, status             │
   └──────────────────────────────────────────────────────────┘
                              │
                              │ consulted on every launch
                              ▼
   Orchestrator's launch_host()
   ┌──────────────────────────────────────────────────────────┐
   │  1. Read package on disk; compute hash                   │
   │  2. Look up the package name in the registry             │
   │  3. Verify package_hash matches                           │
   │  4. Verify signing_key_id matches                         │
   │  5. Verify status is Active                               │
   │  6. Verify privilege_tier_cap covers the requested tier  │
   │  7. THEN proceed to signature verify, manifest load, ... │
   └──────────────────────────────────────────────────────────┘
```

**Depends on:**
- `vault-records`, `vault-auth`, `vault-key-custody` — registry
  records live here.
- `vault-policy` — gates write access by Tier 3 for mutations.
- `vault-audit` — every mutation written to the audit log.
- `sha256` — for the package hash.
- `orchestrator` — the only consumer of the registry on launch.

**Used by:**
- `orchestrator.launch_host()` checks the registry before any
  signature verification.
- The orchestrator CLI (`orchestrator agent register`,
  `orchestrator agent revoke`, `orchestrator agent list`) is the
  user's interface.
- Audit log records every launch attempt, registered or not.

---

## Design Principles

1. **Default-deny.** A package on disk that is not in the
   registry cannot be launched, even if it has a valid signature
   from a trusted key. The registry is an explicit allowlist, not
   a deny-list.

2. **Hash pin per package.** Each registry entry binds a
   `(name, hash)` pair. A new build of the same package is a
   different hash and requires re-registration — a deliberate
   user action, not an automatic upgrade.

3. **Stored in the vault.** The registry inherits the vault's
   confidentiality, integrity, audit, and authentication
   guarantees. There is no separate trust-store file on disk.

4. **Mutations require Tier 3.** Adding, replacing, or revoking a
   registry entry triggers the vault's hardware-key challenge.
   Reading is unrestricted.

5. **The orchestrator does not author registrations.** Only the
   user does (via the CLI). The orchestrator is a consumer; it
   never adds entries on its own.

6. **Tamper-evident.** Every mutation is written to the
   orchestrator's audit log, which lives outside the registry
   and cannot be rewritten by a registry entry.

7. **Per-machine.** The registry is local to this orchestrator's
   vault. Cross-machine sync (so the registry on machine A
   matches machine B) is explicit future work, not v1.

---

## Registry Entry

Each entry is a single vault record at
`vault://orchestrator/agent-registry/<agent-name>`.

```rust
pub struct RegistryEntry {
    /// Stable agent name. Acts as the lookup key. Must match the
    /// `name` field in the agent's manifest.
    pub name:                String,

    /// Absolute or repo-root-relative path to the .agent
    /// directory on disk.
    pub package_path:        PathBuf,

    /// SHA-256 of the entire signed package (deterministic
    /// file-set ordering per D18). Exactly the hash that the
    /// signature was computed over.
    pub package_hash:        [u8; 32],

    /// SHA-256 of the manifest.json file alone. Stored
    /// separately so we can detect manifest-only mutations
    /// vs full-package replacements.
    pub manifest_hash:       [u8; 32],

    /// Identifier of the public key that signed this package.
    /// Must match a key in the trusted-keys namespace.
    pub signing_key_id:      KeyId,

    /// Maximum privilege tier this entry permits. The agent's
    /// effective tier from manifest must be <= this. Even if
    /// the signing key permits Tier 3, the registry can cap it
    /// lower.
    pub privilege_tier_cap:  PrivilegeTier,

    /// Lifecycle status of this entry.
    pub status:              EntryStatus,

    /// Unix ms timestamp of registration.
    pub registered_at_ms:    u64,

    /// Free-form label the user typed at registration. Shown in
    /// audit log and `agent list` output.
    pub registered_label:    String,

    /// Optional revocation timestamp; when set, status is
    /// Revoked and this is when.
    pub revoked_at_ms:       Option<u64>,

    /// Free-form reason the user typed at revocation. Shown in
    /// audit log.
    pub revoked_reason:      Option<String>,
}

pub enum EntryStatus {
    /// Allowed to launch.
    Active,

    /// Disabled by the user; refuses to launch but the entry
    /// remains for forensic reasons. Re-enable with `agent
    /// register --replace` or remove with `agent forget`.
    Disabled,

    /// Hard-revoked. Will not launch and cannot be re-enabled
    /// without removing-and-readding (Tier 3 twice).
    Revoked,
}
```

The registry as a whole is the set of all such records under the
`vault://orchestrator/agent-registry/` namespace. There is no
top-level "manifest of manifests" — listing is done by querying
the namespace.

---

## Operations

### Register

```
$ orchestrator agent register <package-path> [--label "<text>"] \
                              [--tier-cap 0|1|2|3]

Steps:
1. Parse package: read manifest.json, PUBKEY_ID, SIGNATURE.
2. Compute SHA-256 of the full package (deterministic file-set
   ordering per D18).
3. Compute SHA-256 of manifest.json alone.
4. Verify signature against the public key referenced by PUBKEY_ID
   in the trusted-keys namespace. If signature invalid → refuse.
5. Verify the manifest's effective tier <= --tier-cap (if
   provided). If --tier-cap omitted, default to the lower of the
   manifest's tier and the signing key's max tier.
6. RWS validation: load the manifest through the cage and
   confirm it is RWS-clean. If not → refuse with the cage's
   structured error.
7. Tier 3 challenge to the user. Display:
   - Package name, path, size
   - Hash (full + first 12 chars for confirmation)
   - Manifest summary (capabilities, requested tier)
   - Signing key (id + label if available)
   - Whether this name already exists in the registry (and if
     so, hash diff vs the existing entry — see "Replace" below)
8. On user approval (hardware key press):
   - Write the new RegistryEntry to vault:
     vault://orchestrator/agent-registry/<name>
   - Append audit record: { kind: "agent.registered", name,
     hash, signing_key, label, tier_cap, by: <user> }
9. Return success.
```

### Replace (re-registration after a code change)

The user has rebuilt the agent. The new package has the same
`name` but a different `package_hash`. The orchestrator detects
this when `agent register` is called with a path whose package's
name already exists in the registry:

```
$ orchestrator agent register ./my-agent.agent --replace
[diff] Existing entry hash:   a5f3...8b91
[diff] New package hash:      c129...4f72
[diff] Manifest changed:      yes (added net:connect:api.openai.com:443)
[diff] Signing key:           dev-key-7C8 (unchanged)
[Tier 3 challenge: insert YubiKey and press button]
... user presses ...
[ok] Replaced "my-agent" entry. Old hash a5f3...8b91 archived.
```

A replace is logically a revoke + new register, but the registry
keeps the old hash in an `archived_hashes` array on the new
entry so forensic reviewers can see the lineage.

`--replace` is required when the name already exists; without
it, the registration fails to prevent accidental overwrites.

### Revoke

```
$ orchestrator agent revoke <name> --reason "<text>"

Steps:
1. Look up the entry. If not found → error.
2. Tier 3 challenge.
3. On approval:
   - Set status = Revoked, revoked_at_ms, revoked_reason.
   - Append audit record: { kind: "agent.revoked", name, hash,
     reason, by: <user> }.
4. Return success.

A revoked entry remains in the registry (for audit) but
launch_host refuses it.
```

### Forget

Permanently remove an entry. Requires the entry to already be
Revoked (defensive: cannot delete an active or merely-disabled
agent in one step).

```
$ orchestrator agent forget <name>

Tier 3 challenge; remove the vault record; audit-log the deletion.
```

### Disable / Enable

Disable: temporarily refuse to launch without revoking. Tier 3.
Enable: undo Disable. Tier 3.

Used when the user wants to pause an agent while triaging a
suspicious behavior, without losing the registration.

### List / Inspect

```
$ orchestrator agent list

NAME              STATUS    HASH         TIER  SIGNED_BY        LABEL
weather-fetcher   Active    c129...4f72  0     dev-key-7C8      "v1 PoC fetcher"
weather-classifier Active   8a3b...2e10  0     dev-key-7C8      "v1 PoC classifier"
file-writer       Active    f7d9...9a45  0     dev-key-7C8      "v1 PoC writer"
gmail-reader      Disabled  bb02...c134  1     prod-key-2026    "old build, awaiting review"


$ orchestrator agent inspect weather-fetcher

name:                weather-fetcher
status:              Active
package_path:        ./agents/weather-fetcher.agent
package_hash:        c129...4f72
manifest_hash:       6f81...0a3d
signing_key:         dev-key-7C8 (Adhi's dev key)
privilege_tier_cap:  0
registered_at:       2026-05-06T10:43:11Z
registered_label:    "v1 PoC fetcher"
manifest_summary:
  - net:connect:api.weather.gov:443 (ingestion, untrusted)
  - channel:write:weather-snapshots (internal)
last_launched:       2026-05-06T11:25:00Z (success)
launch_count_24h:    288
```

These commands are unrestricted; they read from the vault but
do not require Tier 3.

---

## Launch Path Integration

`orchestrator.launch_host()` from `orchestrator.md` is amended
to consult the registry **before** signature verification.
Updated launch sequence:

```
launch_host(package_path)
    │
    ▼
1. Read package: manifest.json, PUBKEY_ID, SIGNATURE.
   Compute SHA-256 of full package.
   Compute SHA-256 of manifest.json.
   On any read error → return Error::PackageRead.

2. **Look up registry entry by manifest.name.**
   On not-found → return Error::NotRegistered.

3. **Verify package_hash matches registry.entry.package_hash.**
   On mismatch → return Error::HashMismatch and emit a
   PanicSignal (this is suspicious — the bytes on disk differ
   from what the user approved).

4. **Verify status is Active.**
   On Disabled → return Error::Disabled.
   On Revoked  → return Error::Revoked + emit a PanicSignal.

5. **Verify signing_key_id matches.**
   On mismatch → return Error::SigningKeyChanged + PanicSignal.

6. **Verify manifest_hash matches.**
   On mismatch → return Error::ManifestChanged + PanicSignal.
   (A manifest change without a hash change of the whole package
   is impossible; this check is belt-and-suspenders.)

7. **Verify privilege tier cap.**
   effective_tier = manifest.effective_tier()
   If effective_tier > entry.privilege_tier_cap →
       return Error::TierExceedsCap.

8. Now do the existing signature verification (orchestrator.md
   step 2). The hash pin already proved the bytes are what we
   approved; signature verify confirms cryptographic provenance.

9. Continue with the rest of the existing launch sequence
   (trust check, manifest load, supervisor.start_child, channel
   bootstrap, etc.).
```

Each new error is structured with full diagnostic data; the
orchestrator's audit log captures the rejection reason and the
relevant hash pair.

The PanicSignal emissions on hash/key/manifest mismatch escalate
to the panic-broadcast root because these are signals of
tampering (not just configuration errors). The root may decide
to enter tree-wide quarantine if multiple agents start failing
verification — a coordinated attack on the package directory is
exactly the kind of thing tree-wide quarantine is for.

---

## Storage Layout

Inside the vault, under `vault://orchestrator/agent-registry/`:

```
vault://orchestrator/agent-registry/<name>          RegistryEntry (JSON-encoded)
vault://orchestrator/agent-registry/_index           {"names": [...]} (kept in sync on every mutation)
```

The `_index` is an optimization for `agent list`; it lets the
orchestrator enumerate names without scanning the namespace.
Mutation operations write to the entry, then update the index;
the writes are sequenced atomically per the vault's own
guarantees.

The `vault-policy` for this namespace:

```
vault://orchestrator/agent-registry/<name>:
  read:  no-challenge (orchestrator startup, list, inspect)
  write: tier-3 (register, replace, revoke, forget, enable, disable)
```

The orchestrator's identity (the same identity used for the
secure-host-channel) is the principal that reads from the
registry. Writes are user-initiated and require the user's
hardware key.

---

## CLI Reference

```
orchestrator agent register <package-path>
                            [--label <text>]
                            [--tier-cap 0|1|2|3]
                            [--replace]
orchestrator agent revoke   <name> --reason <text>
orchestrator agent forget   <name>
orchestrator agent disable  <name> [--reason <text>]
orchestrator agent enable   <name>
orchestrator agent list     [--status active|disabled|revoked|all]
orchestrator agent inspect  <name>
orchestrator agent diff     <name> <new-package-path>
                            (preview the hash/manifest diff before --replace)
orchestrator agent verify   <package-path>
                            (verify signature + check registry without launching)
```

All mutation commands trigger the Tier 3 challenge. All read
commands are immediate.

---

## Test Strategy

### Unit Tests

1. **Hash determinism.** A given .agent directory hashes to the
   same SHA-256 across two runs (same file-set order, same byte
   reads).
2. **Entry round-trip.** A RegistryEntry serializes to vault and
   deserializes back identically.
3. **Replace lineage.** After --replace, the new entry's
   archived_hashes contains the old hash.

### Integration Tests

4. **Launch happy path.** A registered Active package launches
   successfully.
5. **Launch with hash mismatch.** Modify a byte in the package
   on disk after registration; launch fails with HashMismatch
   and a PanicSignal is emitted.
6. **Launch unregistered package.** Drop a valid signed package
   in `./agents/` without registering it; launch fails with
   NotRegistered.
7. **Launch disabled package.** Disable an entry; launch fails
   with Disabled (no PanicSignal — disabled is operator intent,
   not tampering).
8. **Launch revoked package.** Revoke an entry; launch fails
   with Revoked and a PanicSignal is emitted.
9. **Tier cap enforcement.** Manifest declares Tier 2; registry
   entry caps at Tier 1; launch fails with TierExceedsCap.
10. **Replace requires --replace.** Calling `register` with a
    name that already exists fails without --replace.
11. **Tier 3 challenge.** `register`, `revoke`, `forget`,
    `disable`, `enable` all trigger the challenge; denial cancels
    the operation; timeout cancels the operation.

### Forensic Tests

12. **Audit log captures every mutation.** A complete
    register/replace/revoke/forget cycle produces 4 audit
    records with correct fields.
13. **PanicSignal aggregation.** 5 hash-mismatch failures within
    60s cause the root to enter tree-wide quarantine.

### Coverage Target

`>=95%` line coverage on the registry logic. The vault and
audit-log dependencies have their own coverage targets.

---

## Trade-Offs

**Hash pin requires re-registration on every code change.** A
developer who rebuilds an agent must run `agent register
--replace` before launching it. We accept the friction; the
alternative (auto-upgrade on hash change) defeats the entire
purpose. Future ergonomics (a `--watch` mode that prompts for
re-registration on a detected change) can soften it.

**No automatic key rotation.** If a signing key is rotated,
every entry signed by the old key shows up with a stale
`signing_key_id`. The user must explicitly re-register or revoke
each affected entry. A future `agent rekey` operation that
re-verifies-and-updates the signing_key_id for an entry whose
hash hasn't changed would help; v1 doesn't include it.

**Per-machine.** Two machines running the same agent suite each
maintain their own registry. Sync via `vault-sync` is theoretically
possible but requires careful design (a malicious sync source
could push tampered hashes); we defer.

**The vault must be unlocked to launch.** If the vault is locked,
the orchestrator cannot read the registry, and no agents can
launch. This is intentional: the vault is the trust root, and
operations on locked-vault data are forbidden. The vault unlock
is part of orchestrator startup (per orchestrator.md).

**Index can drift.** A bug or partial write that updates an
entry without updating `_index` would cause the affected agent
to disappear from `agent list` even though it still launches.
The audit log will show the divergence; an `orchestrator agent
reindex` command (Tier 3) rebuilds the index from the namespace
contents.

**Hash-mismatch as PanicSignal may be noisy in development.** A
developer who frequently rebuilds without re-registering will
trigger panic signals on every launch. The default `auto-quarantine
threshold` is 5 distinct alerts in 60s; a developer hitting that
just from forgetting to re-register will need to clear quarantine
manually. We document this; an opt-in "developer mode" might
suppress hash-mismatch panics in the future, but v1 keeps the
strict default.

---

## Future Extensions

- **Cross-machine registry sync** with cryptographic provenance
  for the sync source.
- **Auto-rekey on signing-key rotation** that doesn't change
  package hashes.
- **TUF-style metadata roles** (root, snapshot, targets) for
  larger deployments.
- **Hash-pin policy variations** — e.g., "any hash signed by
  this key" for trusted CI environments where the user is
  confident every CI build is acceptable.
- **Watch mode** that detects on-disk changes and prompts for
  re-registration.
- **Reproducible-build verification** — given source + build
  environment, verify the package hash matches what a
  re-run of the build would produce.

These are deliberately out of scope for v1.
