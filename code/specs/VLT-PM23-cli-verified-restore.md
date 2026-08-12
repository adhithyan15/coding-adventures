# VLT-PM23 — Audited CLI Verified Restore

## Status

Normative Phase 1A product contract for composing portable import with an
independent durable-target verification. This slice closes the local CLI
verified-restore loop without weakening the separately retryable import and
verification commands.

## 1. Grammar and safety boundary

```text
vault-pm --vault TARGET restore FILE
```

`TARGET` is mandatory and must name a configured non-default vault. `FILE` is
one non-empty Unicode path. The command has no passphrase flag, stdin mode, URL
mode, provider credential, overwrite switch, merge mode, implicit target,
mismatch-detail mode, or unsafe output option.

The explicit non-default selector prevents a recovery command from silently
mutating the source/default vault. The target must already exist through the
audit-first `vault create TARGET` ceremony. Target creation remains separate so
its independently confirmed passphrase, trace-before-config journal, and
provider namespace can be retried without reopening an artifact.

The existing commands remain available:

- `import FILE` retries an interrupted import into an eligible empty target;
- `restore verify FILE` retries verification after import committed; and
- `restore FILE` is the only surface allowed to claim one automatically
  composed import and independent verification.

## 2. Pre-authentication reservations

Before the first target authentication, the CLI validates the portable-open
Argon2id ceiling and reserves two complete, independent audit inputs: one for
`PortableImport` and one for `PortableRestoreVerify`. Each input contains one
advisory wall time and one `AUDITED_ACCESS_RANDOM_BYTES` block.

Reserving both inputs before mutation prevents a post-import host time or
entropy failure from making verification impossible to attempt. Reservation
failure occurs before target authentication and before artifact access.

## 3. Artifact opening and expectation preparation

The first target passphrase is collected through the fixed hidden
`Vault passphrase: ` prompt. The target must have an active audit epoch and be
logically eligible for portable import before the artifact is read.

The host reads `FILE` through the existing regular-file, non-empty, metadata,
streaming, and maximum-byte checks. It collects the artifact passphrase exactly
once through `Import passphrase: `. The application authenticates the complete
artifact, enforces the KDF resource ceiling, validates its signed bootstrap and
canonical snapshot, and derives an opaque `PortableRestoreExpectationV1`
before import consumes the opened snapshot.

The host can observe only aggregate item/candidate counts and can neither
inspect nor serialize the expectation. Path, passphrases, source identities,
semantic root, record metadata, and field values never enter audit detail,
configuration, output, or diagnostics.

## 4. Import publication

The CLI obtains the exact count-derived `PortableImportRandomnessV1` block from
the host CSPRNG. It then consumes the first unlocked target session through the
existing audited portable-import application boundary.

Import retains all VLT-PM18 guarantees: the target must be logically empty,
every source item/revision/object identity is replaced, source causal parents
are removed, the complete current candidate grouping and values are preserved,
and the new catalog plus succeeded `PortableImport` event publish atomically.
No intermediate success text is released.

Artifact read, prompt, authentication, expectation, entropy, eligibility, or
publication-preparation failures after authenticated target access publish a
failed itemless `PortableImport` event before the closed error is returned.
Ambiguous publication retains the exact ordinary recovery journal.

## 5. Independent durable-target verification

After import completes, the first target session and opened snapshot are gone.
The CLI collects `Vault passphrase: ` again and opens the selected target as a
new application session from durable owner state, bootstrap, and repository
objects. The artifact is not reopened: the opaque expectation prepared from
the already authenticated source is moved into this new session.

The application consumes the second session through
`audited_verify_portable_restore`. It proves exact normalized candidate-group
semantics, complete record/CRDT/tombstone value equality, cross-vault identity
disjointness, and absence of source causal parents. Match or mismatch publishes
the itemless `PortableRestoreVerify` event before aggregate proof or the closed
integrity error is observable.

A process interruption, second-unlock failure, or provider failure after the
import commit never rolls back or repeats import. The target remains imported,
and `vault-pm --vault TARGET restore verify FILE` is the explicit safe retry.

## 6. Output and claims

Only a durable succeeded verification may emit:

```text
Portable restore completed and verified: items=I candidates=C conflicts=K.
```

The command emits no separate import-success line and no partial counts on
failure. Output contains no path, target name, source/target identity, title,
URL, username, schema, timestamp, deletion time, semantic root, mismatch
position, provider, record body, or secret field.

This wording proves one imported artifact matched one independently reopened
target. It does not claim repository-backup coverage, provider durability,
rollback resistance, multi-replica safety, or that a backup policy is safe.

## 7. Error and audit policy

- invalid grammar, implicit/default target, or ineligible target: invalid;
- wrong target or artifact credentials: locked;
- semantic mismatch or authenticated corruption: integrity;
- source, repository, owner-state, time, or entropy unavailability: provider;
- unsupported platform, provider, or KDF policy: unsupported.

Every authenticated post-unlock import or verification outcome advances the
selected target's signed audit chain or fails closed at audit publication. The
source/default vault, configuration, artifact, and all other target audit chains
remain unchanged.

## 8. Acceptance gates

The slice is complete only when tests prove:

1. grammar requires exactly a leading named selector plus `restore FILE` and
   retains the exact standalone `restore verify FILE` grammar;
2. an implicit or explicitly selected default vault rejects before artifact
   access;
3. one artifact prompt prepares the expectation before the import consumes the
   snapshot;
4. import and its succeeded event publish atomically with no intermediate
   output;
5. verification uses a second target unlock and a newly reopened durable
   session;
6. success publishes ordered succeeded `PortableImport` and
   `PortableRestoreVerify` events, then emits aggregate counts only;
7. artifact/host failures publish only the failed import event, while semantic
   failure after import publishes a failed verification event;
8. a post-import interruption leaves the target eligible for standalone
   `restore verify FILE`, not for a duplicate import;
9. the source/default vault, configuration default, artifact, and unrelated
   named target remain byte-for-byte or semantically unchanged as applicable;
10. audit rows contain neither path, target name, nor any tested passphrase;
11. a real process creates a named target, performs the composed command
    through controlling-terminal prompts, restarts, verifies redacted restored
    items and both audit events, and finds no known plaintext in storage; and
12. formatting, Clippy, rustdoc, focused tests, and downstream executable tests
    pass.
