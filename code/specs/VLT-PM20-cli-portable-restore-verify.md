# VLT-PM20 — Audited CLI Portable Restore Verification

## Status

Normative Phase 1A CLI contract for retrying application-owned semantic
verification against the currently configured target. This slice composes
VLT-PM19 without creating or switching target configurations.

## 1. Grammar and purpose

```text
vault-pm restore verify FILE
```

V1 accepts exactly the `verify` action and one non-empty Unicode path. It has
no passphrase flag, stdin mode, URL mode, provider credential, target selector,
import mode, overwrite switch, mismatch-detail mode, or unsafe output option.

The command is deliberately separate from `import FILE`. If import committed
but its later host process was interrupted, verification can be retried without
attempting a second mutation against the now non-empty target.

## 2. Pre-authentication reservation

Before target authentication, the CLI:

1. validates the host portable-open Argon2id ceiling;
2. reserves one advisory wall time; and
3. reserves one complete `AUDITED_ACCESS_RANDOM_BYTES` block.

The target passphrase is then collected through the ordinary hidden
`Vault passphrase: ` prompt. A target without an active audit epoch is rejected
before artifact bytes are read. Every distinct post-unlock outcome can
therefore advance the target audit chain or fail closed at publication.

## 3. Bounded source reopening

After active-epoch unlock, the host reads `FILE` through the same non-empty
regular-file and metadata/streaming byte ceilings as portable import. It then
collects the artifact passphrase through the fixed hidden
`Import passphrase: ` prompt.

The application authenticates and validates the entire artifact without a
target write, then derives `PortableRestoreExpectationV1`. Host orchestration
receives neither the opened source candidates nor the semantic root.

## 4. Host and artifact failure auditing

After target unlock, any source-read, passphrase-prompt, artifact-authentication,
format, KDF, snapshot-validation, or expectation-preparation failure consumes
the target session through
`record_audited_portable_restore_verify_host_failure`.

The CLI returns the original closed error only after a failed itemless,
revisionless `PortableRestoreVerify` event is durable. The event contains no
path, provider detail, passphrase, source/target identity, semantic root,
candidate count, record metadata, or mismatch detail. Audit publication
ambiguity withholds the original error and retains the exact recovery journal.

## 5. Independent comparison

The command opens the target in a new command/session, independently of the
session that imported the artifact. It passes the opaque expectation to
`audited_verify_portable_restore`.

VLT-PM19 comparison proves exact normalized candidate-group semantics,
source/target vault-item-revision identity disjointness, and absence of source
causal parents. Match or mismatch publishes its succeeded or failed itemless
event before releasing the aggregate report or integrity error.

## 6. Output

Success emits exactly one aggregate line:

```text
Portable restore verified: items=I candidates=C conflicts=K.
```

It emits no source path, title, URL, username, schema, timestamp, deletion
time, item/revision/vault identity, semantic root, mismatch position, provider,
record body, or secret field. Failure emits no partial comparison result.

This wording is “restore verified,” not “restore completed” or “backup safe.”
Explicit target-creation/configuration switching and automatic import-to-verify
composition remain outstanding product requirements.

## 7. Exit policy

- wrong target or artifact credentials: locked;
- malformed grammar or pre-audit target: invalid;
- semantic mismatch or authenticated corruption: integrity;
- source/repository/owner-state unavailability: provider;
- unsupported platform or KDF policy: unsupported.

The command never reveals which semantic field differed.

## 8. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly `restore verify FILE` and no secret argument;
2. a pre-audit target rejects before artifact access;
3. wrong artifact credentials publish a failed verification event;
4. a later retry publishes a succeeded event and aggregate counts only;
5. audit rows contain neither path nor either tested artifact passphrase;
6. the source remains unchanged and the target can still perform ordinary
   audited redacted reads;
7. the real executable imports under one application root, starts another
   process, collects both secrets through controlling-terminal prompts,
   independently verifies, and leaves no known plaintext in either storage
   tree; and
8. formatting, Clippy, rustdoc, focused tests, and downstream executable tests
   pass.

## 9. Remaining recovery work

The next slice should add explicit in-root target creation and configuration
switching, then compose import and verification automatically while retaining
this standalone command as the safe retry path after interruption.
