# `coding_adventures_vault_pm_cli`

This crate is the first real composition layer for the local-first password
manager. It owns strict parsing, stable exit classes, redacted rendering, and
the bounded `init`, locked `status`/`doctor`, authenticated `audit enable` and
`audit verify`, and opt-in full `doctor --unlock` workflows. It now also owns the first usable item
vertical: authenticated login creation plus durable redacted list/show. The
same one-shot boundary now supports revision-safe login replacement. The
newest-first `history list ITEM` projection exposes canonical revision
selectors plus safe causal metadata without opening historical secrets. The
same selectors now support reversible `item delete ITEM` and
`history restore ITEM REVISION`. The executable is a thin caller of this
package.

The driver composes the existing storage-neutral application over separately
permission-checked application-state and encrypted-object filesystem roots.
It acquires the persistent cross-process writer lock before loading
configuration or constructing either backend. Configuration is parsed by the
closed VLT-PM07 codec, and the configured filesystem location must exactly
match the prepared platform object root before storage is touched.

Initialization collects a new passphrase only through the injected fixed
secret-input boundary, fills the complete generation-zero randomness block
from the injected OS entropy boundary, and uses a fixed production Argon2id
floor. It durably installs the exact `PreparedInit` owner journal before
publishing the configuration that makes the random locator discoverable. A
restart with a prepared journal collects the existing passphrase and resumes
the exact journal without generating new identities or ciphertext.

Status and plain doctor do not prompt or unlock. `audit enable`, `audit verify`, and
`doctor --unlock` collect the existing passphrase through the controlling
terminal, open the storage-neutral repository for one read-only action, and
synchronously drop the live session before rendering. Their projections contain
only closed labels or aggregate verification counts, including the number of
fully authenticated encrypted operation events (zero for pre-audit vaults),
with no paths, locators, providers, identities, or cryptographic details. No command accepts a
passphrase through argv, stdin, environment, configuration, or URL.

`audit enable` is the explicit one-time boundary between a vault's declared
unlogged historical prefix and its signed operation-event epoch. It consumes
the unlocked session through the existing crash-resumable audit-only journal
and returns only after the successful `AuditEpochStart` event and next owner
state are durable. Repeating the command is a no-write success. Once enabled,
the epoch cannot be disabled or silently bypassed by an authenticated item or
verification command.

When an audit epoch already exists, `audit verify`, `doctor --unlock`, `item
list`, `item show`, and `history list` consume the unlocked session through the
application's signed publish-before-release boundary. Output is constructed
only after the event and next owner state are durable. Pre-audit vaults retain
their prior read behavior until the all-command migration cutover is exposed.

`item add login` unlocks once, collects bounded fields from the controlling
terminal, obtains fresh mutation and metadata identities from OS entropy, and
consumes the session through the crash-resumable application mutation.
`item list` and `item show ITEM` reopen in separate one-shot sessions and
render only escaped redacted projections; the password and notes body are
never available to the renderer. In an active epoch, both commands first make
their exact signed access outcome durable.

`item edit ITEM` asks the application for an opaque edit preparation that owns
the authenticated current revision and wipe-on-drop secret document without
returning either to CLI orchestration. It preserves immutable identity and
unedited metadata, collects the complete bounded login form again, and consumes
the preparation through the crash-resumable replacement compare-and-swap. In an
active audit epoch, missing/conflicted/unsupported targets and prompt, entropy,
or document-validation failures publish a failed `ItemUpdate` event before the
CLI exposes their error; success publishes its event atomically with the new
revision.

`history list ITEM` reopens the authenticated repository, traverses at most the
application history bound, synchronously locks, and then renders each unique
revision as a canonical selector, live/deleted state, direct-parent count,
advisory timestamp, and—only for live revisions—the schema and escaped title.
Passwords and notes bodies never enter the redacted projection; usernames,
URLs, paths, object frames, and cryptographic details are never emitted by the
history renderer.

`item delete ITEM` passes only the stable item identity into an application
boundary that selects the sole current live revision and consumes the session
through an immutable causal-tombstone mutation. In an active audit epoch, a
successful delete publishes its signed event atomically with that tombstone;
missing, already-deleted, and conflicted attempts publish a failed delete event
before the CLI exposes the closed error. The exact optimistic revision
capability never crosses into CLI orchestration.
`history restore ITEM REVISION` passes both user selectors into one bounded
application boundary, which proves the revision belongs to that item's history
and copies it into a new current revision without returning the history
projection to CLI orchestration. In an active audit epoch, the successful
restore event and new revision publish atomically. Missing, cross-item,
tombstone, same-revision, and conflicted attempts publish a failed restore event
before their closed error is exposed. Neither operation erases history,
rewinds repository heads, or emits secret-bearing metadata.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli --no-deps
```
