# VLT-PM06: Local Host Trust Boundary

Status: Phase 1A normative contract

## 1. Purpose

VLT-PM01 through VLT-PM05 deliberately accept bytes, storage traits, entropy,
and custody decisions from their caller. `storage-fs` accepts an arbitrary path
and `storage-core` conditions are atomic only within one live backend instance.
Those are reusable primitives, but they are not sufficient to let an executable
safely choose a user directory or coordinate independent processes.

This specification defines the smallest filesystem host boundary required
before the local CLI composes those primitives. It owns:

1. platform-standard per-user path resolution;
2. creation and validation of owner-private local roots;
3. separation of owner state from encrypted immutable objects;
4. rejection of linked or substituted filesystem objects;
5. non-blocking cross-process single-writer exclusion; and
6. bounded, owner-private, atomic configuration persistence.

It does not parse CLI arguments, prompt for a passphrase, generate entropy,
understand vault bytes, select a cloud provider, or repair permissions.

## 2. Security objective

Before a filesystem-backed command constructs any application or object-store
adapter, the host proves that its selected roots are ordinary directories owned
by the current user and unavailable to other users. Before a command opens any
backend capable of mutation, it also owns the installation's process lock.

The boundary prevents accidental disclosure through broad default permissions,
link traversal into attacker-selected locations, and concurrent independent
`FsStorageBackend` instances defeating the application adapter's exact-value
compare-and-exchange assumptions.

This boundary does not defend against another process already running as the
same operating-system user. Such a process is inside Phase 1A's local trust
boundary and can inspect the user's memory or replace user-owned files by other
means.

## 3. Filesystem layout

The platform resolver returns application-specific roots for configuration,
local data, and cache. The local-data root contains two disjoint subtrees:

```text
<config-root>/                    non-secret strict configuration
  vault-pm.toml                   exact VLT-PM07 configuration bytes
<data-root>/
  application-state/             bootstrap generations and owner-private state
  objects/                       encrypted immutable repository objects
  .writer.lock                   persistent process-lock identity
<cache-root>/                     disposable derived data only
```

The exact platform prefix is selected by the operating system's application
directory resolver. Callers must not concatenate a literal `$HOME`,
`USERPROFILE`, drive letter, or vendor-specific cloud path.

`application-state` and `objects` are separate so later storage selection can
move or replace the object backend without silently moving the owner-private
activation journal. Cache loss must never make the vault unrecoverable.

## 4. Path contract

Every resolved or explicitly injected root must be:

- absolute;
- non-empty;
- at most 4,096 encoded bytes or code units;
- free of `.` and `..` traversal components; and
- representable by the target platform's native path API.

The injectable constructor exists for tests and an eventual explicit
configuration override. It performs validation only. It never reads or creates
filesystem objects until `prepare` is called.

Path-bearing values have closed `Debug` output. No error string contains a
root, user name, volume, environment value, or lock-file path.

## 5. Directory preparation

Preparation is idempotent. For each required root it either creates a private
ordinary directory or validates the existing directory. It must not silently
replace, chmod, re-ACL, rename, delete, or migrate an existing object.

### 5.1 Unix

Unix traversal starts from an opened root-directory descriptor. Every path
component is opened relative to the previously opened descriptor with
`O_DIRECTORY`, `O_NOFOLLOW`, and `O_CLOEXEC`. Missing components are created
relative to that descriptor with mode `0700` and immediately reopened without
following links.

The final requested root must:

- be a directory according to `fstat` on the open descriptor;
- have `st_uid` equal to the effective user ID;
- grant no group or other permission bits; and
- grant the owner read, write, and execute permission.

This descriptor-relative sequence avoids check-then-open link substitution.
Existing intermediate platform directories may use ordinary platform access
modes; only newly created intermediates and each application root become
owner-private.

### 5.2 Windows

Windows traversal opens every ancestor with `FILE_FLAG_OPEN_REPARSE_POINT` and
requires the directory attribute without the reparse-point attribute. Missing
directories are created with an explicit protected DACL, owned by the current
process token's user SID, containing exactly one allow ACE for that SID.

The final requested root is reopened and must retain that same owner and DACL
shape. Inherited, null, absent, multi-ACE, foreign-owner, reparse-point, and
non-directory objects fail closed.

## 6. Existing insecure roots

An existing final root with broad permissions or a foreign owner returns a
coarse insecure-permissions or insecure-owner error. The library never repairs
it automatically because changing an existing ACL or mode can revoke intended
access, conceal compromise, or race another process.

The future CLI repair ceremony must identify the exact affected logical root,
require an interactive confirmation, revalidate through native handles, make
the smallest permission change, synchronize it, and rerun preparation. An
unsafe override, if ever offered, must be explicit for one invocation and may
not become the default.

## 7. Process lock

After preparation, a caller may open `<data-root>/.writer.lock`. The open uses
the same no-link and owner-only policy as the directories:

- Unix: regular file, current effective user, mode `0600` or stricter owner-only
  equivalent with owner read/write access;
- Windows: ordinary non-reparse file with the protected single-owner DACL.

The file is persistent and must not be deleted on normal unlock. Its filesystem
identity anchors advisory locking across process lifetimes and avoids an unlink
race in which two processes lock different inodes under the same name.

Acquisition uses the operating system's exclusive advisory file lock in
non-blocking mode. Contention returns `AlreadyLocked`; it never waits, sleeps,
steals, truncates, or parses the lock file. The returned guard owns the open
file and lock. Dropping the guard releases the lock even during ordinary error
unwinding or process exit.

Phase 1A commands acquire the guard before constructing `FsStorageBackend`
instances and hold it through all reads, recovery, publication, and final
rendering. Read-only commands may be relaxed later only after every involved
adapter has a proven cross-process snapshot contract.

## 8. Configuration persistence

Configuration is stored as exact non-empty bytes at
`<config-root>/vault-pm.toml`, bounded to 65,536 bytes. The local-host package
does not parse or render those bytes; the storage-neutral `vault-pm-config`
package owns the VLT-PM07 schema. Load, initial create, and replacement are
available only through the live writer guard, so every cooperating process has
already passed native root validation and acquired cross-process exclusion.

Load returns `None` only when the file is absent. An existing object must be a
regular, non-link, current-owner file with owner-only access. Empty, oversized,
linked, reparse, wrong-type, foreign-owner, and broadly accessible objects fail
closed before bytes are returned.

Initial create stages an owner-only file in the configuration directory,
writes and synchronizes all bytes, and publishes it without replacing an
existing name. Replacement is exact-value compare-and-exchange: the durable
bytes must equal the caller's expected bytes before a same-directory staged
file atomically replaces the name. A stale or absent value returns
`ConfigConflict`. Unix publishes initial state with a descriptor-relative hard
link, replaces with descriptor-relative rename, and synchronizes the directory.
Windows uses a protected single-owner DACL and write-through move, adding the
replace flag only for compare-and-exchange.

A failed operation never truncates or partially writes the named configuration.
A crash may leave an owner-private `.vault-pm.toml.tmp.*` staging file; loaders
ignore such files, and a later operation uses a unique name. Automatic scanning
or deletion of unknown residue is outside this contract.

## 9. Closed failures

The stable local-host failure classes are:

| Failure | Meaning |
|---|---|
| `PlatformUnavailable` | per-user standard directories could not be resolved |
| `InvalidPath` | an injected or resolved path violated the bounded contract |
| `ParentUnavailable` | a parent could not be traversed without following links |
| `AccessFailed` | a native create, open, inspection, or lock operation failed |
| `UnsafeObjectType` | a link, reparse point, or wrong object type was found |
| `InsecureOwner` | an existing private object belongs to another user |
| `InsecurePermissions` | an existing private object grants broader access |
| `AlreadyLocked` | another process currently owns the writer lock |
| `ConfigAlreadyExists` | initial creation found existing configuration |
| `ConfigConflict` | expected bytes did not match durable configuration |
| `InvalidConfigBytes` | configuration was empty or exceeded 65,536 bytes |
| `UnsupportedPlatform` | no audited native implementation exists |

The executable maps invalid injected paths and duplicate initialization to exit
class 2, lock or compare-and-exchange contention to exit class 5,
integrity/security-policy failures (including invalid durable configuration) to
exit class 6, unsupported targets to exit class 8, and unexpected native access
failures to exit class 10. Human diagnostics remain low resolution.

## 10. Composition rule

The initial filesystem composition is:

```text
LocalVaultPaths::resolve
  -> prepare
  -> try_acquire_writer
  -> load/create/compare-exchange strict VLT-PM07 configuration bytes
  -> FsStorageBackend(application-state)
     -> StorageCoreApplicationStore
  -> FsStorageBackend(objects)
     -> StorageCoreObjectStore
  -> V1ApplicationRepositoryFactory
  -> VaultAccessV1 / command driver
```

The application and object adapters do not receive the config or cache root.
The local-host package receives only opaque configuration bytes; it does not
receive parsed vault IDs, locators, passphrases, plaintext documents, provider
credentials, or serialized owner state.

Replacing `FsStorageBackend(objects)` with Google Drive, WebDAV, S3, or an
in-memory test backend does not change path preparation for local owner state
and does not change the application contract.

## 11. Acceptance tests

The package is complete for Phase 1A when automated tests prove:

1. resolved roots are absolute and layout subtrees are disjoint;
2. preparation creates all required roots and is idempotent;
3. created Unix roots are mode `0700` and Windows roots use a protected
   single-owner DACL;
4. existing broad-permission roots fail without mutation;
5. final links, reparse points, and wrong object types fail closed;
6. one guard excludes a second independently opened guard;
7. dropping the first guard permits a later acquisition;
8. the lock file itself is owner-only and persistent;
9. invalid injected paths fail before filesystem access;
10. errors and `Debug` output contain no resolved paths;
11. configuration creation never replaces an existing safe or unsafe object;
12. configuration load returns exact bounded bytes and rejects unsafe objects;
13. compare-and-exchange rejects stale or absent expected bytes without change;
14. successful create and replacement survive close and reopen; and
15. created configuration uses owner-only native security metadata.

The real CLI pseudo-terminal suite must additionally run two executable
processes against the same injected roots and observe one deterministic lock
winner before Phase 1A is declared complete.
