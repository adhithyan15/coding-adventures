# `coding_adventures_vault_pm_cli`

This crate is the first real composition layer for the local-first password
manager. It owns strict parsing, stable exit classes, redacted rendering, and
the bounded `init`, locked `status`/`doctor`, authenticated `audit verify`, and
opt-in full `doctor --unlock` workflows. It now also owns the first usable item
vertical: authenticated login creation plus durable redacted list/show. The
executable is a thin caller of this package.

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
only closed labels or aggregate verification counts, with no paths, locators,
providers, identities, or cryptographic details. No command accepts a
passphrase through argv, stdin, environment, configuration, or URL.

`item add login` unlocks once, collects bounded fields from the controlling
terminal, obtains fresh mutation and metadata identities from OS entropy, and
consumes the session through the crash-resumable application mutation.
`item list` and `item show ITEM` reopen in separate one-shot sessions and
render only escaped redacted projections; the password and notes body are
never available to the renderer.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli --no-deps
```
