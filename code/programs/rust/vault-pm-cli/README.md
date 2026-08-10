# `vault-pm`

The first installable local password-manager executable. It intentionally has
no product logic: it forwards process arguments to
`coding_adventures_vault_pm_cli`, writes that package's bounded public output,
and exits with its stable VLT-PM00 class.

The current command surface is:

```text
vault-pm init [--vault NAME] [--storage NAME]
vault-pm status [--json]
vault-pm doctor
```

`init` requires a controlling terminal even when stdin is redirected. No
passphrase flag, environment variable, config field, URL, or stdin path exists.
Unix integration tests launch this exact binary under a fresh pseudo-terminal,
verify the passphrase is not echoed, restart the process for status and doctor,
and inspect the isolated filesystem tree for plaintext passphrase bytes.

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```
