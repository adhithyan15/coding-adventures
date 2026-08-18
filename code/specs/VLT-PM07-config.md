# VLT-PM07: Storage-neutral Client Configuration

Status: Phase 1A normative contract

## 1. Purpose

Initialization creates a random bootstrap locator that is intentionally absent
from encrypted provider object names. A later process therefore needs durable
local configuration to find the same vault. Configuration also selects named
storage adapters without coupling application use cases to a filesystem or a
cloud SDK.

This specification defines the pure configuration value and byte contract. It
does not define filesystem persistence, platform path resolution, credential
custody, provider authentication, or storage-adapter construction.

## 2. Security properties

Configuration must never contain a master password, vault root key, decrypted
provider token, item metadata, or plaintext record. It may contain:

- a human-selected vault alias;
- the opaque random bootstrap locator;
- named local and remote storage selections;
- adapter-owned location strings;
- references to credentials held by an OS or provider credential store; and
- bounded secret-lifetime policy.

Locator, storage-location, and credential-reference diagnostics are redacted.
Parse failures are stable and payload blind.

## 3. Closed V1 schema

V1 is the TOML shape in VLT-PM00 section 14.3. The root contains exactly
`format_version` and `default_vault`. Every other declaration belongs to one
two-segment `[vaults.NAME]` or `[storage.NAME]` table. Array tables, deeper
tables, duplicate declarations, unknown fields, and missing fields fail.
Source input and the canonical rendering of a programmatically constructed
value are each bounded to 65,536 UTF-8 bytes.

Names are 1 through 64 ASCII bytes. The first byte is alphanumeric; remaining
bytes are alphanumeric, `_`, or `-`. There are at most 64 vaults, 64 storage
declarations, and 16 remote stores per vault.

Each vault table contains exactly:

- `vault_locator`: 64 lowercase hexadecimal characters encoding 32 random
  bytes; locator identity is unique across vault declarations;
- `local_store`: one existing storage alias;
- `remote_stores`: an ordered, unique array of existing aliases, none equal to
  the local store;
- `auto_lock_seconds`: 1 through 86,400; and
- `clipboard_clear_seconds`: 1 through 3,600.

`clipboard_clear_seconds` gained a reader in `VLT-PM46-cli-clipboard.md`: it is
how long `--copy` waits before verifying that the clipboard still holds the
value this product wrote, and clearing it if so. Until then it was a validated
value with nothing behind it. Its range is restated at the process boundary
that carries it to the detached clearer (VLT-PM46 §4.3), because a boundary
that trusts its input is not a boundary.

The default vault must exist.

Each storage table contains exactly:

- `kind`: `filesystem`, `gdrive`, `webdav`, or `s3`;
- `path`: a non-empty, trimmed, control-free UTF-8 adapter location of at most
  4,096 bytes; and
- `credential_ref`: a non-empty, trimmed, control-free UTF-8 reference of at
  most 256 bytes. `none` means no external credential.

`path` is a historical field name and is opaque to this contract. A filesystem
adapter interprets it as a native path; a provider adapter may interpret it as
a folder, bucket, prefix, or provider namespace. Only the selected adapter may
validate or use it.

## 4. Canonical rendering

The renderer emits decimal integers, lowercase locator hexadecimal, basic TOML
strings, vault tables in alias order, then storage tables in alias order. It
uses one final newline. Rendering a parsed value and parsing the result must be
idempotent.

Canonical rendering is not a wire-security primitive. It gives persistence
adapters deterministic exact bytes for atomic create and compare-exchange.

## 5. Dependency and composition rules

`vault-pm-config` is pure and owns no host capability. CLI, web, and desktop
composition roots may all parse the same value model. A host persistence
adapter supplies bytes separately. A storage factory receives only the selected
typed declaration and explicit host credentials; application and repository
packages never receive the complete config document.

For the first Phase 1A local CLI vault, the configured `local_store` is
`filesystem`, its location is the platform-resolved encrypted-object root from
VLT-PM06, and its credential reference is `none`. VLT-PM22 named targets each
receive a distinct filesystem declaration at a locator-derived child root; one
adapter root must not be shared across repository locators. Local owner state
remains fixed to VLT-PM06's application-state root and is not selected by this
config.

## 6. Acceptance tests

Automated tests must prove:

1. the VLT-PM00 example and an empty-remote variant parse;
2. every parsed value round-trips through canonical rendering;
3. unknown, duplicate, missing, wrongly typed, and unsupported declarations
   fail closed;
4. bounds, name grammar, locator encoding, locator uniqueness, and every
   cross-reference are enforced;
5. storage kinds remain typed while locations stay adapter-owned; and
6. errors and ordinary `Debug` output contain no locator, location, or
   credential-reference payload.
