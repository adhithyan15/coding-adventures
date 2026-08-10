# VLT-PM09 — Local CLI bootstrap composition

Status: normative Phase 1A slice

## 1. Purpose

This specification defines the first runnable `vault-pm` product slice. It
composes the existing provider-neutral application, durable storage adapters,
secure local roots, strict configuration codec, controlling-terminal reader,
and OS entropy source into one local executable.

The slice proves three user-visible commands:

```text
vault-pm init [--vault NAME] [--storage NAME]
vault-pm status [--json]
vault-pm doctor
```

It does not redefine cryptography, repository semantics, storage wire formats,
configuration syntax, terminal handling, or filesystem permission checks.
Those remain owned by VLT-PM01 through VLT-PM08. This specification owns only
command grammar, composition order, stable rendering, and real-process
acceptance.

## 2. Package and executable boundaries

`code/packages/rust/vault-pm-cli` owns:

- the closed command grammar;
- stable exit-class mapping;
- payload-free text and JSON renderers;
- orchestration of `init`, locked `status`, and locked `doctor`;
- validation that configured storage is safe for this local-only slice; and
- injected path, clock, entropy, KDF-policy, and fixed-prompt secret seams.

`code/programs/rust/vault-pm-cli` owns only the `vault-pm` process entry point.
It passes `args_os().skip(1)` to the package, writes the returned bounded output,
and exits with the returned class. It MUST NOT parse commands, collect secrets,
construct providers, or implement application policy.

The package may be reused by a later installer or foreground shell. The
executable is replaceable without changing any persisted format.

## 3. Closed command grammar

### 3.1 General rules

- Non-Unicode arguments fail as invalid input.
- Unknown commands, flags, duplicates, missing values, and trailing arguments
  fail as invalid input.
- `--flag=value` forms are not accepted in V1.
- `--help`, `-h`, and `help` are the only help spellings and accept no other
  arguments.
- Help parsing performs no path resolution or filesystem access.
- No command accepts a passphrase, secret field, provider token, or encryption
  key through argv.

The following spellings MUST be rejected, rather than ignored:

```text
--passphrase
--password
--token
--unsafe-include-secrets
--vault=NAME
--storage=NAME
```

Future commands extend the closed grammar deliberately. Compatibility never
means silently accepting an unknown security-relevant option.

### 3.2 `init`

`init` accepts each of `--vault NAME` and `--storage NAME` at most once and in
either order. Names use the exact bounded VLT-PM07 `ConfigName` grammar.
Defaults are `personal` and `local`.

### 3.3 `status`

`status` accepts only an optional `--json`. It never prompts or unlocks in this
slice.

### 3.4 `doctor`

`doctor` accepts no arguments. It never repairs, prompts, unlocks, accepts new
pins, or changes configuration in this slice.

## 4. Host acquisition order

After successful parsing, every product command performs the following order:

1. resolve standard platform application directories through VLT-PM06;
2. create or validate owner-private config, data, state, object, and cache roots;
3. acquire the non-blocking persistent writer lock;
4. load exact configuration bytes while the lock is held;
5. strictly decode VLT-PM07 configuration when present; and
6. construct storage backends only after the configuration and local root have
   both passed validation.

The same lock is used for read-only commands in Phase 1A. `storage-core` does
not provide cross-instance CAS or durable reader snapshots, so a read-only CLI
must not race an independent writer while decoding owner state.

Errors and `Debug` values MUST NOT include resolved paths.

## 5. Local storage selection

This slice supports one configured default vault, no remote replicas, and one
filesystem local store. Configuration remains provider-neutral and retains a
typed storage selection; the CLI performs the Phase 1A adapter choice.

Before opening either filesystem backend, the CLI MUST prove:

- the selected vault exists;
- its local store declaration exists;
- the storage kind is `filesystem`;
- the credential reference is exactly `none`;
- the remote-store list is empty; and
- the configured filesystem location exactly equals the platform-resolved,
  permission-checked object root.

Any other valid VLT-PM07 storage selection returns `unsupported`, not a
fallback. The CLI MUST NOT reinterpret a Google Drive, WebDAV, S3, arbitrary
filesystem, or future storage declaration as the default local root.

Application/bootstrap owner state uses the distinct permission-checked
application-state root. Encrypted immutable repository objects use the object
root. Both are constructed through `FsStorageBackend`, but the application
continues to see only injected VLT-PM05 and VLT-PM02 traits.

## 6. New initialization

When no configuration exists, `init` performs this exact sequence while
holding the writer lock:

1. collect and constant-time confirm a new non-empty passphrase through the
   VLT-PM08 controlling-terminal boundary;
2. fill exactly `GENERATION_ZERO_RANDOM_BYTES` from the OS CSPRNG;
3. read an advisory Unix-millisecond wall-clock value;
4. construct the bounded production Argon2id policy;
5. call the no-write VLT-PM05 `prepare_generation_zero` boundary;
6. encode the exact returned `PreparedInit` owner state;
7. atomically create that state at its random bootstrap locator;
8. render and atomically create configuration containing that locator and the
   selected filesystem store; and
9. call `complete_generation_zero` over the injected application, bootstrap,
   and repository stores.

The production KDF floor for this slice is Argon2id with 65,536 KiB memory,
three iterations, and one lane. Calibration and persisted policy upgrades are
future work; a test host may inject another VLT-PM01-valid policy without
changing production defaults.

The passphrase exists only in a `Zeroizing<Vec<u8>>`. Entropy is written into a
caller-owned fixed-size block. Neither value is cloneable through the CLI
driver, rendered, logged, or included in an error.

## 7. Crash and retry semantics

The exact prepared journal is installed before configuration makes its random
locator discoverable. This order creates two acceptable crash classes:

- before config publication, no configured vault exists; any unreachable
  opaque journal is not a partial logical vault and may be garbage-collected by
  the Phase 1A fault/restore slice; or
- after config publication, the configured locator resolves to the complete
  exact `PreparedInit` journal and initialization is resumable.

On a retry with existing configuration:

1. the configured default vault and selected storage names must exactly match
   the requested `init` names;
2. the selected filesystem declaration must pass section 5;
3. an `Active` owner state returns `already initialized` without prompting;
4. a `PendingPublication` owner state returns `conflict/recovery required`;
5. a missing, malformed, or mismatched owner state fails integrity; and
6. only `PreparedInit` collects the existing passphrase, calls
   `rehydrate_prepared_init`, and completes the exact journal.

Retry MUST NOT generate a new locator, identity, key, salt, nonce, object,
commit, or announcement. VLT-PM05 idempotence decides whether exact already
published effects are success.

## 8. Status projection

With no configuration, status is `uninitialized`. With configuration, the CLI
uses a key-free `VaultAccessV1::Locked` boundary and the exact local-state store.
It renders only these labels:

| VLT-PM05 state | CLI label |
|---|---|
| absent | `uninitialized` |
| prepared | `initializing` |
| active, no live session | `locked` |
| live session | `unlocked` |
| pending publication | `recovery_required` |

Text output is exactly `Status: LABEL\n`. JSON output is exactly one object,
`{"state":"LABEL"}\n`. Locked JSON never gains counts, identifiers, paths,
or provider details merely because `--json` was supplied.

## 9. Doctor projection

The locked VLT-PM05 doctor report maps to one coarse label and exit class:

| Doctor state | Label | Exit |
|---|---|---:|
| healthy | `healthy` | 0 |
| initialization required | `initialization_required` | 2 |
| recovery required | `recovery_required` | 5 |
| any store unavailable | `unavailable` | 7 |
| unsupported version/capability | `unsupported` | 8 |
| authentication required | `authentication_required` | 3 |
| integrity failure | `integrity_failure` | 6 |

Output is exactly `Doctor: LABEL\n` on stdout, including nonzero health states.
Stderr remains empty because this is the requested diagnostic result rather
than an execution error.

## 10. Stable execution failures

Parser and orchestration failures emit one fixed line on stderr and no stdout.
They use VLT-PM00 exit classes:

| Exit | Meaning in this slice |
|---:|---|
| 2 | invalid command/input or already initialized |
| 3 | passphrase authentication required/failed |
| 4 | requested object absent |
| 5 | writer contention, CAS conflict, or recovery required |
| 6 | unsafe permissions/object type or persisted integrity failure |
| 7 | local storage, terminal, entropy, clock, or platform authority unavailable |
| 8 | unsupported platform, provider, or storage selection |
| 10 | internal invariant failure |

Failures MUST NOT interpolate an argument, name, path, OS error, passphrase,
locator, provider identifier, or persisted payload.

## 11. Real-process acceptance

The executable integration suite MUST:

1. create isolated platform roots;
2. launch the exact `vault-pm` binary as a new session with a controlling
   pseudo-terminal;
3. connect process stdin to a pipe containing decoy secret lines and prove the
   controlling-terminal prompts ignore them;
4. wait for each fixed new-passphrase prompt before writing terminal input;
5. prove the passphrase and decoy bytes do not occur in the terminal transcript;
6. require successful generation-zero initialization;
7. start a new process and observe exact locked JSON status;
8. start another process and observe locked doctor authentication-required
   output and exit 3; and
9. recursively inspect the isolated files and prove the plaintext passphrase
   byte string is absent.

Package tests additionally cover the closed parser, unconfigured projections,
restart behavior over injected roots, and repeated-init refusal. Native CI must
compile the package and executable on Linux, macOS, and Windows. Unix runs the
pseudo-terminal E2E; Windows relies on the independently tested VLT-PM08
console adapter until the repository supplies an equivalent ConPTY harness.

## 12. Explicit non-goals

This slice does not implement:

- an `unlock` command or resident key process;
- item CRUD/list, search, history, conflict resolution, or restore;
- full authenticated doctor or audit verification from a one-shot command;
- portable export/import command parsing;
- interactive shell, clipboard, TOTP, attachment, or password generation;
- configuration mutation after initial creation;
- arbitrary filesystem roots, removable-folder mode, or remote providers;
- multiple configured vault creation; or
- repair, garbage collection, backup, or restore drills.

Those remain backlog items 9b and 10 in VLT-PM00. They must reuse the same
application and storage seams rather than branching on filesystem or future
Google Drive details inside command handlers.
