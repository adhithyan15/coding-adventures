# `coding_adventures_vault_pm_cli`

This crate is the first real composition layer for the local-first password
manager. It owns strict parsing, stable exit classes, redacted rendering, and
the bounded `init`, locked `status`/`doctor`, authenticated `audit verify`, and
opt-in full `doctor --unlock` workflows. It now also owns the first usable item
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

Status and plain doctor do not prompt or unlock. `audit verify` and
`doctor --unlock` collect the existing passphrase through the controlling
terminal, open the storage-neutral repository for one read-only action, and
synchronously drop the live session before rendering. Their projections contain
only closed labels or aggregate verification counts, including the number of
fully authenticated encrypted operation events (zero for pre-audit vaults),
with no paths, locators, providers, identities, or cryptographic details. No command accepts a
passphrase through argv, stdin, environment, configuration, or URL.

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

`item edit ITEM` resolves the authenticated sole current revision, opens that
exact document only inside a wipe-on-drop wrapper, preserves immutable identity
and unedited metadata, collects the complete bounded login form again, and
consumes the session through the crash-resumable replacement compare-and-swap.

`history list ITEM` reopens the authenticated repository, traverses at most the
application history bound, synchronously locks, and then renders each unique
revision as a canonical selector, live/deleted state, direct-parent count,
advisory timestamp, and—only for live revisions—the schema and escaped title.
Passwords and notes bodies never enter the redacted projection; usernames,
URLs, paths, object frames, and cryptographic details are never emitted by the
history renderer.

`item delete ITEM` resolves the authenticated sole current live revision and
consumes the session through an immutable causal-tombstone mutation.
`history restore ITEM REVISION` first binds the exact canonical live revision
to that item's bounded redacted history, then consumes the session while the
application independently authenticates and copies it into a new current
revision. Neither operation erases history, rewinds repository heads, or emits
secret-bearing metadata.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli --no-deps
```
