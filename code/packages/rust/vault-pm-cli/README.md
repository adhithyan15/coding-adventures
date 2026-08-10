# `coding_adventures_vault_pm_cli`

This crate is the first real composition layer for the local-first password
manager. It owns strict parsing, stable exit classes, redacted rendering, and
the bounded `init`, locked `status`, and locked `doctor` workflows. The
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

Status and doctor do not prompt or unlock. Their text and JSON projections are
closed labels with no paths, locators, providers, identities, or cryptographic
details. No command accepts a passphrase through argv, stdin, environment,
configuration, or URL.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli --no-deps
```
