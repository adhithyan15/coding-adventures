# VLT-PM22: Audited Named Targets and Command-Scoped Selection

Status: Phase 1A normative product contract

## 1. Purpose

Portable restore must never overwrite the source vault. The local CLI therefore
needs a way to create a separately keyed empty target and direct later commands
to that target. This contract adds named target creation and command-scoped
selection without coupling the product to the filesystem backend or mutating a
global selection as a side effect of access.

This slice does not add a storage provider, change the VLT-PM07 configuration
format, delete or rename vault declarations, or combine import and semantic
verification into one command. It establishes the safe target lifecycle that
that final composition can use.

## 2. Grammar and selection

The closed CLI grammar gains:

```text
vault-pm vault create NAME
vault-pm --vault NAME COMMAND ...
```

`NAME` is a VLT-PM07 `ConfigName`. The leading selector is accepted exactly
once and only before a command that operates on an existing vault. It is
rejected for `init`, `vault create`, and `help`. Existing `init --vault NAME`
syntax continues to name the first declaration and is not a command selector.

When the leading selector is absent, commands use `default_vault` exactly as
before. When present, they select that exact configured declaration. Selection
is command-scoped: it never rewrites `default_vault` or any other configuration
byte. An unknown name fails with the stable not-found class. A declaration that
references a storage capability the local CLI cannot construct fails with the
stable unsupported class.

All authenticated edit, disclosure, history, audit, export, import, and restore
verification commands run against the selected vault and retain their existing
publish-before-release audit rules. Locked `status` and locked `doctor` inspect
only redacted local owner state and do not decrypt or release vault content.

## 3. Named target creation

`vault create NAME` requires an existing valid configuration and creates one
new empty vault declaration. V1 reuses the default vault's supported adapter
kind and policy values while allocating a distinct provider namespace and
storage declaration. A storage adapter is bound to one opaque repository
locator, so two vaults must never share one adapter root. The new target
receives a fresh root, signing key, device identity, bootstrap locator,
repository object set, and passphrase collected and confirmed through the
controlling terminal. The existing default declaration and every existing
vault remain unchanged.

The implementation must perform these steps while holding the local writer
lease:

1. load and retain the exact current configuration bytes;
2. collect and confirm the new target passphrase, fill the complete audited
   generation-zero randomness block, and prepare audited generation zero;
3. atomically install the exact `PreparedInit` owner state at the fresh
   locator before publishing that locator in configuration;
4. construct a validated configuration containing the new vault and its
   distinct storage declaration, then compare-exchange it against the exact
   bytes loaded in step 1;
5. complete generation zero through the application repository boundary; and
6. report success only after the target is durable and active.

The prepared journal installed before the configuration edit contains the
signed encrypted `VaultInitialize` trace, its initial commit, and the intended
active audit head. Target creation is therefore trace-first even across a crash:
the discoverability edit never precedes the exact encrypted trace that explains
it.

An existing active declaration is not replaced. If the declaration points to
an exact `PreparedInit` state, repeating `vault create NAME` collects the
existing passphrase, rehydrates that journal without new randomness or
replacement bytes, and resumes its original completion. `PendingPublication`,
malformed owner state, locator mismatch, or a stale configuration
compare-exchange fails closed.

A crash before configuration publication may leave unreachable opaque prepared
bytes. It must not make the locator discoverable or change any existing vault.
Opaque orphan reclamation is a separate maintenance concern. A crash after
configuration publication is recoverable by the exact-name retry above.

## 4. Storage neutrality

The command manipulates typed VLT-PM07 values and uses the application storage
and repository interfaces. It does not inspect provider object layouts or add a
filesystem path to an application use case. Phase 1A accepts the composed local
filesystem adapter and allocates a locator-derived child root beneath the
platform object root. Its storage alias is an alphabetic prefix plus 252 bits
of the locator spelling; an alias collision fails closed. Later Google Drive,
WebDAV, S3, and other adapter factories allocate a distinct folder, prefix, or
provider namespace through the same typed configuration boundary.

Every provider sees only immutable encrypted repository objects and encrypted
owner/bootstrap state permitted by the lower-layer contracts. Vault aliases,
adapter locations, and credential references remain local configuration and
must never enter encrypted record payloads or audit details.

## 5. Output and secret safety

Successful creation emits one fixed public line. Selection emits no independent
output. Errors remain bounded and payload blind. Passphrases, keys, locators,
provider locations, credential references, item identifiers, trace details,
and portable artifact contents must not appear in arguments, ordinary output,
errors, or `Debug` values.

## 6. Acceptance tests

Automated tests must prove:

1. the new grammar accepts one valid leading selector and one valid creation
   command while rejecting duplicates, misplaced selectors, secret-like extra
   arguments, and selectors on `init`, creation, or help;
2. creation adds exactly one vault and one distinct supported storage
   declaration, preserves the default and existing declarations, and produces
   an independently openable empty active vault;
3. the target's first commit contains the succeeded `VaultInitialize` genesis
   event and its active state binds that event as the audit head;
4. a failure before configuration compare-exchange leaves the old exact config
   unchanged, while a crash after it is resumed from the original prepared
   bytes without generating a replacement trace;
5. duplicate active names, unknown selected names, unsupported selected storage,
   stale config, malformed state, and pending publication fail closed;
6. two named vaults isolate items, revisions, identities, and audit chains while
   selected authenticated commands advance only the chosen chain; and
7. a real process can create a target, select it for portable import and restore
   verification, restart between commands, and observe no source-vault change.

## 10. Verified-restore composition

`VLT-PM23-cli-verified-restore.md` uses the explicit named selector to compose
portable import with an independently reopened target verification. It retains
the commands defined here and by VLT-PM20 as interruption-safe retry surfaces.
