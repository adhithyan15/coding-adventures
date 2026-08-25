# VLT-PM17 — Audited CLI Portable Export

## Status

Normative Phase 1A contract for the first encrypted recovery-artifact command.
It composes the portable snapshot and operation-audit primitives already fixed
by VLT-PM00, VLT-PM05, and VLT-PM15. Portable import remains a separate
follow-up so creation, opening, target initialization, and cross-vault identity
allocation are not hidden inside one oversized ceremony.

**Amendment (VLT-PM05 §13.9).** The optional `--best-effort` flag in §2, the
exclusion report in §9, and gate 10 in §11 were added later, closing the
backlog item §13.2 opened and left deliberately unrepaired: a single item
this build cannot re-encode used to deny the export of an entire otherwise-
healthy vault. Without the flag, every byte of this spec's original text
below still describes this command's exact, unmodified behavior — the
amendment is strictly additive, reached only by a caller who asks for it by
name, the same "does not silently change what an existing caller already
depends on" posture VLT-PM49's own amendment took for its two new import
formats.

## 1. Purpose

The local CLI needs a backup users can place in a directory synchronized by a
storage provider they already trust or pay for. The CLI must not learn Google
Drive, Dropbox, WebDAV, S3, or any other provider protocol to provide that
value. It produces one provider-neutral, passphrase-encrypted artifact at an
explicit filesystem destination; the host or an ordinary sync client may move
those opaque bytes afterward.

This command is an authenticated bulk access. Its signed audit event must be
durable before the encrypted artifact is released to the filesystem adapter.
No plaintext record, passphrase, path, item identity, repository identity, or
provider detail enters command output or the audit projection.

## 2. Command grammar

The complete grammar, after the VLT-PM05 §13.9 amendment, is:

```text
vault-pm export FILE [--best-effort]
```

`FILE` is one explicit destination path. Missing, empty, additional, or
non-Unicode arguments fail as the stable invalid-command class before host
preparation. V1 has no passphrase flag, standard-input mode, standard-output
mode, overwrite flag, plaintext mode, provider flag, or implicit default
destination.

The parser treats a path beginning with `-` as a path value only because it is
the sole positional argument after the closed `export` command. It does not
interpret path contents, expand a shell token, resolve a provider, or render
the path back to the user. This rule is unchanged by the amendment and takes
precedence over it: `vault-pm export --best-effort` is one positional
argument — a destination literally named `--best-effort` — not the flag,
because the flag is recognized only in the two-token shape below.

**`--best-effort` (VLT-PM05 §13.9 amendment).** Optional, and recognized only
immediately after `FILE` — `export FILE --best-effort`, never
`export --best-effort FILE` — matching this grammar's existing
flags-after-positional order elsewhere in the command surface (VLT-PM00
§14.4's `item show ITEM [--field FIELD] [--copy|--reveal]`, for one). Absent,
this command's behavior is byte-for-byte what the rest of this spec already
describes. Present, an item this build cannot re-encode (an oversized
first-party, opaque, or quarantined record, or a conflict with one such
candidate — VLT-PM05 §13.1/§13.3/§13.5/§13.8) is excluded from the artifact
instead of denying the whole export; §9 states what changes in the command's
output when that happens, and VLT-PM05 §13.9 states the complete design.

## 3. Authorities and ownership

The composition uses only existing injected authorities plus two narrow host
operations:

1. local paths and the exclusive writer guard;
2. exact configuration, bootstrap, owner state, and immutable repository;
3. the controlling terminal for the vault and export passphrases;
4. OS CSPRNG and advisory wall time;
5. the application portable-export and audit boundaries; and
6. a create-new portable-artifact writer for the explicit destination.

The application owns decrypted candidate material, snapshot construction,
Argon2id, AEAD, bootstrap binding, and signed access publication. CLI
orchestration receives only the final encrypted artifact wrapper. The native
writer receives only those encrypted bytes and the user-selected path.

## 4. Input ceremony

The live vault passphrase is collected from the controlling terminal through
the ordinary `Vault passphrase: ` prompt. After successful unlock, the export
passphrase is collected twice with echo disabled:

The fixed prompts are `Export passphrase: ` and
`Confirm export passphrase: ` (each includes one trailing space).

The two independently read values are compared in constant time. They are
non-empty, at most 1,024 bytes, owned by wipe-on-drop wrappers, and never
accepted through argv, stdin, environment, configuration, URL, or destination
metadata. V1 intentionally uses a distinct passphrase rather than silently
reusing the live vault passphrase.

The export KDF policy is a separate injected policy seam even when the native
V1 host uses the same 64 MiB, three-iteration, one-lane Argon2id floor as new
local vaults. Tests may inject the valid 8 MiB minimum without changing the
production policy.

## 5. Pre-authentication reservation

Before requesting the live vault passphrase, the CLI must successfully reserve:

- one advisory wall-clock value;
- the exact `PORTABLE_EXPORT_RANDOM_BYTES` salt/nonce block;
- the exact `AUDITED_ACCESS_RANDOM_BYTES` event/publication block; and
- one validated portable-export KDF policy.

It also loads the exact signed bootstrap selected by the configured opaque
locator before authentication. Failure here is not an authenticated vault
access and does not fabricate an event.

Reservation before authentication guarantees that every failure after a
successful active-epoch unlock can advance the existing audit chain without
asking a failing entropy or clock authority for replacement material.

## 6. Audit ordering

### 6.1 Active audit epoch

After successful unlock, export-passphrase collection or confirmation failure
must consume the session through a failed `PortableExport` event. The event is
itemless and revisionless, contains a fresh trace, and is durably published
before the CLI exposes its closed error.

For valid input, the application builds and authenticates the canonical
encrypted artifact, then publishes a successful `PortableExport` event through
the audit-only journal. The artifact is withheld until the event and next
owner state are durable. Invalid snapshot or export-policy input is published
as a failed attempt by the same application boundary.

If audit publication is unavailable, ambiguous, conflicting, or corrupt, the
artifact is not released and the exact pending recovery state is retained.

### 6.2 Pre-audit vault

A vault that has not run `audit enable` retains the backward-compatible
ordinary portable-export boundary. This exception exists only for migration;
after epoch activation the CLI may not bypass audited export.

### 6.3 Destination failure

A successful `PortableExport` event means the authenticated encrypted artifact
was produced and released across the application boundary. Destination
persistence happens afterward and is not allowed to rewrite that signed fact.
If create, write, or sync fails, the CLI returns a closed host failure. A later
retry is another independently audited export access.

## 7. Artifact contents

The artifact delegates to the canonical portable V1 application format. It
contains every verified current live, tombstone, and conflict candidate and is
bound to the exact signed source bootstrap. It excludes local owner-private
state, provider credentials, configuration, cache/search projections, pins,
and raw repository frames.

Without `--best-effort`, that "every" is unconditional: one candidate this
build cannot re-encode denies the artifact entirely (§9). With
`--best-effort` (VLT-PM05 §13.9 amendment), "every" instead means every item
this build *could* re-encode; an item it could not is left out in its
entirety — never a partial candidate set — and its identity is carried, still
inside the same passphrase-encrypted plaintext as everything else, in the
snapshot's own `excluded_item_ids` field, so the gap is a recorded fact of
the artifact rather than something indistinguishable from a smaller vault
that never had the item.

The complete plaintext snapshot is encrypted under the separately collected
passphrase using a fresh Argon2id salt and XChaCha20-Poly1305 nonce. The host
cannot parse record fields and may copy the resulting bytes through any
storage backend without learning vault contents.

## 8. Destination policy

The native V1 writer must:

- reject an empty destination or empty artifact;
- use create-new semantics for the final path;
- never follow or replace an existing final file, directory, or symbolic link;
- request owner-read/write mode (`0600`) when creating a Unix file;
- write the complete artifact and synchronize the file before success; and
- best-effort remove the newly created incomplete file after write/sync
  failure.

The parent is intentionally user-selected. It may be a normal local directory,
mounted removable media, or a folder synchronized by a cloud client. V1 does
not create parent directories, weaken their permissions, resolve provider
credentials, claim remote durability, or verify that an external sync client
uploaded the file.

Create-new rather than replacement is mandatory. Rotation and retention are
explicit user operations; a typo must not destroy the only known-good backup.

## 9. Rendering and exit behavior

Success without `--best-effort`, or success with `--best-effort` and nothing
excluded, emits exactly:

```text
Portable export written.
```

**With `--best-effort` and at least one item excluded (VLT-PM05 §13.9
amendment)**, success instead emits:

```text
Portable export written.
Excluded (too large to include): N
<item id>
...
```

one item id per line, in this build's own canonical id rendering (the same
form `item show`/`item delete` accept). This is dynamic, operator-actionable
content and belongs on standard output — the same "aggregate counts on
stdout" shape the `import` command's `created=/skipped=/failed=` report
already uses (`VLT-PM49-cli-external-import.md` §6) — not a fixed,
payload-free sentence on standard error the way the VLT-PM42 recovery notice
and the VLT-PM47 attachments-left-behind notice are: unlike those two, a bare
count here would leave the operator with nothing to act on. Every id printed
is already visible through this vault's own `item list`; nothing about this
line discloses anything the operator's own already-open session could not
already show them.

The destination is not echoed either way. Failures use the existing bounded
CLI classes:

- malformed grammar, passphrase mismatch, or existing destination: invalid;
- wrong vault passphrase: locked;
- bootstrap/repository/audit corruption: integrity;
- terminal, entropy, repository, or destination write failure: provider;
- unsupported target: unsupported.

`--best-effort` changes none of these classes and introduces no new one: it
changes only whether an otherwise-`BoundExceeded` outcome is reachable at
all for the item(s) it excludes. A `BoundExceeded` from the array-level
encode ceiling on an entirely healthy, merely very large vault (VLT-PM05
§13.9's own "what this does not fix") is unaffected by the flag and still
surfaces as the ordinary invalid-command-adjacent failure this command
already had before the amendment.

No diagnostic includes OS error text, a path, a passphrase, record data,
provider metadata, or cryptographic material. The exclusion report above is
the one deliberate, narrow exception to "record data never enters command
output" — item ids are identifiers, not record fields, and VLT-PM05 §13.9
states why exposing them here does not weaken that guarantee.

## 10. Recovery and interruption

An interruption before successful audit publication releases no artifact. An
ambiguous audit publication is recovered by the ordinary owner-state journal
before a later command can proceed. An interruption while writing the already
released encrypted artifact may leave either no destination (after cleanup) or
an incomplete encrypted file; the command did not report success, and a retry
must use a different absent destination.

The artifact format remains self-authenticating. The later import command must
open, authenticate, and fully validate it under a host resource ceiling before
creating or mutating any target vault.

## 11. Acceptance gates

The slice is complete only when tests prove:

1. the parser accepts exactly one destination and no secret-bearing flag;
2. live and export passphrases are collected from the controlling terminal,
   both export entries are hidden, and redirected stdin cannot inject them;
3. an active-epoch prompt failure is visible later as failed
   `portable_export` before the process error;
4. success is visible later as succeeded `portable_export` and the artifact
   can be authenticated with the distinct passphrase;
5. the artifact and storage tree contain none of the known plaintext test
   passphrases or record secrets;
6. an existing destination is byte-for-byte unchanged after a retry;
7. Unix creation requests mode `0600`;
8. audit output contains neither destination nor passphrase;
9. the real executable completes the hidden-prompt ceremony across a fresh
   pseudo-terminal and restart-backed vault; and
10. **(VLT-PM05 §13.9 amendment)** `--best-effort` is recognized only in the
    fixed `export FILE --best-effort` position, and a bare
    `export --best-effort` parses `--best-effort` as the destination, not the
    flag; without `--best-effort`, standard output for both a clean and a
    would-have-excluded export is byte-for-byte identical to the pre-
    amendment text of §9; with it and at least one exclusion, standard
    output carries the exact count and every excluded item's canonical id,
    and none of the excluded items' record data — title, username, password,
    or any other field.

## 12. Non-goals and backlog

This slice does not add portable import, plaintext interoperability export,
repository mirroring, backup scheduling, retention, provider upload status,
Google Drive APIs, or a restore-completed claim. The prioritized follow-up is
the bounded audited import ceremony in `VLT-PM18-cli-portable-import.md`.
Explicit target creation/configuration switching and application-owned
field-by-field semantic restore comparison remain later recovery work.

The VLT-PM05 §13.9 amendment does not extend `--best-effort` (or an
equivalent) to `vault-pm import portable`, nor does it address the
array-level encode ceiling on a large, entirely healthy vault (VLT-PM05
§13.9's own stated exclusion) or VLT-PM05 §13.2's separate, still-open
ingest-gating residual. Each remains exactly as open as it was before this
amendment.
