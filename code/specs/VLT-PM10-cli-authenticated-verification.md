# VLT-PM10 - Authenticated CLI Verification

Status: normative Phase 1A slice

Depends on: VLT-PM05, VLT-PM06, VLT-PM07, VLT-PM08, VLT-PM09

## 1. Purpose

This slice proves that the standalone `vault-pm` executable can open the
durable local vault through the production host boundaries, perform a complete
read-only repository verification, synchronously discard the live session, and
return only bounded public output.

It adds:

```text
vault-pm audit verify
vault-pm doctor --unlock
```

Plain `vault-pm doctor` remains the key-free health projection from VLT-PM09.
The explicit `--unlock` flag distinguishes a potentially expensive,
passphrase-prompting repository walk from a script-safe locked check.

This ordering is intentional. Authenticated read-only verification closes the
one-shot unlock composition before item creation, replacement, deletion,
history restore, export, or import can mutate durable state.

## 2. Locked properties

1. The master passphrase is read only from the controlling terminal through
   the fixed VLT-PM08 unlock prompt.
2. The passphrase is never accepted through argv, process stdin, environment,
   configuration, URL, or JSON.
3. Both commands use the configured storage-neutral application repository
   factory. They do not inspect filesystem objects or implement repository
   verification in the CLI.
4. The local process writer lock is acquired before configuration, owner state,
   bootstrap state, or repository state is read.
5. Unlock succeeds only after VLT-PM05 authenticates the local secret, validates
   exact active/bootstrap binding, opens the signed repository from durable
   pins, materializes the current catalog, and builds its wipe-on-drop search
   projection.
6. Audit success is rendered only after a second complete repository audit
   succeeds from the authenticated session.
7. Doctor success is rendered only after the unlocked doctor repeats the exact
   local/bootstrap binding and complete audit.
8. The live session is synchronously changed back to `Locked` before either
   success or doctor output is constructed.
9. Failures emit no partial counts, locators, paths, provider identifiers,
   cryptographic identifiers, item identifiers, or item fields.
10. This slice is read-only. It performs no recovery, repair, publication,
    garbage collection, configuration mutation, or provider migration.

## 3. Grammar

The parser accepts only these new exact forms:

```text
audit verify
doctor --unlock
```

It rejects:

- `audit` without the `verify` subcommand;
- extra audit or doctor positionals;
- `--flag=value` spellings;
- `--passphrase`, secret positionals, or secret-bearing JSON flags;
- unlock aliases, environment references, and file-descriptor flags; and
- non-Unicode arguments.

`doctor` without flags preserves VLT-PM09 behavior and never prompts.

## 4. Execution order

### 4.1 Shared host preparation

Both commands execute in this order:

1. resolve platform-standard roots;
2. prepare and permission-check the owner-private roots;
3. acquire the persistent non-blocking process writer lock;
4. load the exact configuration bytes;
5. strictly parse VLT-PM07 configuration;
6. select the default vault and its local store;
7. reject remote stores in Phase 1A;
8. prove that the configured filesystem location exactly equals the prepared
   object root and that the credential reference is `none`;
9. construct the durable owner-state/bootstrap adapter and storage-neutral
   repository factory; and
10. create a key-free `VaultAccessV1::Locked` boundary for the configured opaque
    locator.

An unconfigured invocation fails before a prompt. Unsupported or malformed
configuration likewise fails before secret collection.

### 4.2 One-shot unlock

After host preparation:

1. request the existing passphrase through the controlling terminal;
2. pass the owned zeroizing bytes directly into `VaultAccessV1::unlock`;
3. leave the lifecycle boundary locked on every authentication, storage,
   unsupported-format, integrity, or repository-open failure;
4. retain the resulting unlocked session only for this process action; and
5. never write the passphrase or derived keys to local state, configuration,
   repository objects, output, or diagnostics.

The command does not expose a standalone `unlock` operation. A completed
one-shot process has no resident key holder.

### 4.3 Audit verification

`audit verify` invokes `UnlockedVaultV1::audit_verify`. The application core:

- rediscovers announcements relative to durable local pins;
- verifies signed commits and complete bounded ancestry;
- proves the local counter, catalog root, and device certificate anchor;
- decrypts every distinct reachable catalog;
- decrypts every catalog-referenced revision and validates item binding;
- validates causal-parent item binding; and
- rejects partial, replayed, malformed, unsupported, or cross-vault state.

The CLI does not receive object identifiers or decrypted documents. On success
it receives only aggregate counts.

### 4.4 Full doctor

`doctor --unlock` invokes the VLT-PM05 doctor on the authenticated lifecycle
boundary. The doctor:

- reloads and strictly decodes exact owner state;
- proves that it equals the active state retained by the session;
- reloads and verifies the latest signed bootstrap generation;
- repeats the full audit; and
- maps the result to one closed health classification.

Doctor never repairs a pending journal, replaces configuration, rewrites a
bootstrap, fetches an alternate provider, or marks corrupt data healthy.

## 5. Session disposal

The CLI stores the operation result, calls `VaultAccessV1::lock`, and only then
maps or renders the result. `lock` replaces the lifecycle enum and drops the
unlocked session, including live application keys, local private secrets,
materialized documents, and search projection, before the function returns.

Language-level drop remains a backstop for early unlock failures and process
termination. No success path relies solely on process exit for disposal.

## 6. Output contract

Successful audit output is exactly:

```text
Audit: verified (announcements=A commits=C catalogs=G revisions=R items=I)
```

The five values are non-negative aggregate counts from the fully verified
report. No output is produced until the complete audit succeeds.

Successful authenticated doctor output is exactly:

```text
Doctor: healthy
```

All existing locked doctor labels remain unchanged. Prompts are emitted by the
controlling-terminal adapter, not by standard input or the public renderer.

## 7. Failure mapping

| Condition | Exit | Public result |
|---|---:|---|
| success | 0 | exact audit or doctor line |
| invalid or unconfigured audit invocation | 2 | fixed invalid-command error |
| unconfigured doctor invocation | 2 | `Doctor: initialization_required` |
| wrong passphrase/authentication failure | 3 | fixed authentication-required error |
| pending local publication/concurrent writer | 5 | fixed recovery-or-conflict error |
| malformed, tampered, replayed, or cross-vault state | 6 | fixed integrity error |
| local repository unavailable | 7 | fixed storage-unavailable error |
| unsupported configuration/format/capability | 8 | fixed unsupported error |
| internal invariant failure | 10 | fixed internal error |

Audit failures have empty stdout. Unlocked doctor uses the existing coarse
doctor label mapping after authentication. No error includes its source value.

## 8. Acceptance tests

Package tests must prove:

1. the parser accepts only the exact new forms;
2. secret arguments and extra tokens are rejected without host side effects;
3. generation zero can be initialized and reopened for a complete audit;
4. an empty generation-zero vault reports exactly one announcement, one commit,
   one catalog, zero revisions, and zero items;
5. authenticated doctor reports healthy;
6. a wrong passphrase maps to exit 3 with empty stdout and fixed stderr; and
7. repository-object tampering maps to exit 6 with no partial audit counts; and
8. a later command observes the vault as locked, proving no reusable session is
   exported by the one-shot API.

The real executable PTY suite must additionally:

1. initialize under isolated platform roots;
2. run `audit verify` in a new process;
3. run `doctor --unlock` in another new process;
4. provide decoy secret lines through redirected stdin for both commands;
5. wait for the fixed controlling-terminal unlock prompt before providing the
   real passphrase through the pseudo-terminal;
6. observe exact successful public output;
7. prove neither the passphrase nor redirected-stdin decoy occurs in either
   transcript; and
8. recursively prove the plaintext passphrase is absent from the isolated
   filesystem tree.

Native CI compiles and tests on Linux, macOS, and Windows. Unix executes the PTY
suite; Windows retains the independently tested VLT-PM08 console adapter until
an equivalent ConPTY harness is available.

## 9. Explicit non-goals and backlog split

This slice does not implement item list/show/search/history, item mutation,
conflict resolution, portable export/import commands, the foreground shell,
clipboard ownership, password generation, TOTP, attachments, or fault repair.

VLT-PM00 item 9b is split after this slice into:

- 9b-2: redacted item list/show, search, and history reads;
- 9b-3: add, replace, delete, restore, and conflict-resolution mutations;
- 9b-4: portable export/import host commands and destination policy; and
- 9b-5: foreground interactive shell over the same command/use-case boundary.
