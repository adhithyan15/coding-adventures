# VLT-PM18 — Audited CLI Portable Import

## Status

Normative Phase 1A contract for bounded no-write artifact opening and atomic
import into a separately initialized target. This slice completes the first
usable restore path without making the source vault writable or conflating
target creation, configuration switching, and semantic field-by-field restore
comparison with the import mutation itself.

## 1. Purpose and safety boundary

`vault-pm import FILE` restores a canonical encrypted portable artifact into
the currently configured vault only when that target is logically empty and
has an active operation-audit epoch. The source is the artifact, not another
live vault session. The target has its own bootstrap, root key, item IDs,
revision IDs, encrypted object IDs, device identity, and audit chain.

Users create the target under an independent application root, run `init`, and
run `audit enable` before import. The command never opens or mutates the source
vault repository. This makes local-to-local recovery and provider-downloaded
artifact recovery possible while keeping source and target failure domains
separate.

## 2. Grammar

```text
vault-pm import FILE
```

V1 accepts exactly one non-empty Unicode path. It has no artifact-passphrase
flag, target-passphrase flag, stdin mode, URL mode, provider credential,
overwrite switch, merge mode, or implicit source. Unknown or additional tokens
fail before host preparation.

The target is the ordinary configured default vault resolved through the same
storage-neutral composition as every other command. V1 does not add a hidden
alternate-root or config-switching mechanism.

## 3. Required target state

The target must:

- be independently initialized and unlock with its own passphrase;
- have completed `audit enable`;
- contain no live, tombstone, or conflict candidate; and
- have no pending publication or repository divergence.

Audit-only events are allowed before import. This is required so a wrong
artifact passphrase, unavailable file, or interrupted retry can be recorded
without making the target permanently ineligible. Any prior logical item
mutation leaves a candidate and continues to fail closed.

A pre-audit target is rejected before artifact bytes are read. This deliberate
cutover rule makes every authenticated import attempt traceable.

## 4. Pre-authentication reservation

Before target authentication, the CLI reserves one advisory wall time and one
complete `AUDITED_ACCESS_RANDOM_BYTES` block. It also validates the host's
maximum portable-open Argon2id policy. These inputs are sufficient to publish
an audit-only failed `PortableImport` event if a later host or artifact step
fails.

The target passphrase is then collected through the ordinary hidden
`Vault passphrase: ` prompt. After a successful active-epoch unlock, no closed
import failure becomes observable before its event is durable.

## 5. Bounded artifact read and no-write opening

The native host reads `FILE` only after the target is authenticated and known
to have an audit epoch. The source must be a non-empty regular file no larger
than `MAX_PORTABLE_EXPORT_ARTIFACT_BYTES`. Metadata bounds allocation, and the
reader itself is capped at one byte beyond the maximum so concurrent growth
cannot force an unbounded read.

The artifact passphrase is collected once through the fixed hidden
`Import passphrase: ` prompt. It never enters argv, stdin, environment,
configuration, a URL, output, an audit event, or diagnostics.

The application authenticates header-bound AEAD before parsing plaintext,
enforces the host's Argon2id resource ceiling, verifies the canonical snapshot
hash and exact signed source bootstrap, and validates every bounded candidate.
This opening performs no target write. Its opaque result exposes only item and
candidate counts to CLI orchestration.

## 6. Failed-attempt auditing

After active-epoch target unlock, all of these failures publish an itemless,
revisionless, failed `PortableImport` event before the CLI error:

- source open, metadata, type, size, or read failure;
- artifact-passphrase prompt failure;
- wrong artifact passphrase or authenticated-format failure;
- KDF, snapshot, count, or publication bound failure;
- dynamic OS-entropy failure;
- target no-longer-empty, source-equals-target, identity collision, stale
  repository, or publication-preparation failure.

The event contains no path, artifact bytes, passphrase, source identity,
provider detail, candidate count, record metadata, or target item identity.
Audit publication ambiguity withholds the original error and retains the exact
recovery journal.

## 7. Atomic successful import

After no-write opening, the application computes the exact dynamic entropy
requirement from the bounded item and candidate counts. Host CSPRNG bytes are
owned by a wipe-on-drop `PortableImportRandomnessV1`.

One atomic publication:

1. assigns every source item a fresh target item ID;
2. assigns every retained live, tombstone, and conflict candidate a fresh
   encrypted target revision/object identity;
3. preserves validated schema, timestamps, record payload, CRDT field state,
   deletion state, and candidate grouping;
4. omits source causal-parent identities because cross-vault current-state
   import is a re-identification operation;
5. writes a new encrypted catalog and signed commit; and
6. writes the successful `PortableImport` audit event in that same commit and
   advances the target audit head.

The source vault, artifact, and target generation-zero bootstrap are never
reused as identities. Partial logical restore is forbidden. Provider or local
state ambiguity leaves the ordinary exact pending publication for recovery.

## 8. Output and errors

Success emits only aggregate source counts:

```text
Portable import complete: items=I candidates=C.
```

It emits no source path, title, URL, username, record body, item/revision ID,
vault identity, provider, or cryptographic detail. Subsequent ordinary list and
show commands reopen the target and use their existing audited redacted-read
boundaries.

Wrong target or artifact credentials use the closed locked class; malformed
input and ineligible target state use invalid; authenticated corruption uses
integrity; host/repository unavailability uses provider; unsupported platform
uses unsupported.

## 9. Acceptance gates

The slice is complete only when tests prove:

1. the parser accepts exactly one source and no passphrase argument;
2. a pre-audit empty target rejects import before artifact access;
3. a wrong artifact passphrase becomes a durable failed import event;
4. a later retry remains eligible and publishes one atomic successful event;
5. imported IDs differ from source IDs and the source remains unchanged;
6. the artifact reader rejects empty, non-file, and oversized sources under a
   hard read cap;
7. audit rows contain neither source path nor passphrase;
8. list/show after restart expose only the existing redacted projections; and
9. the real executable exports from one application root, initializes and
   audit-enables another, imports through hidden prompts, restarts, and observes
   the restored redacted items with no known plaintext in either storage tree.

## 10. Remaining restore work

This slice does not create or switch target configurations automatically and
does not claim a field-by-field semantic restore comparison in the CLI. The
next recovery slice should add an explicit target-creation/switch ceremony and
an application-owned verifier token that independently reopens the target and
compares item, candidate, conflict, schema, timestamp, deletion, and revealed
field values before reporting a fully verified restore.
