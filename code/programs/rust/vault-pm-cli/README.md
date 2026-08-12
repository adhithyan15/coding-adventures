# `vault-pm`

The first installable local password-manager executable. It intentionally has
no product logic: it forwards process arguments to
`coding_adventures_vault_pm_cli`, writes that package's bounded public output,
and exits with its stable VLT-PM00 class.

The current command surface is:

```text
vault-pm init [--vault NAME] [--storage NAME]
vault-pm status [--json]
vault-pm audit enable
vault-pm audit verify
vault-pm audit list
vault-pm audit show TRACE
vault-pm doctor [--unlock]
vault-pm export FILE
vault-pm import FILE
vault-pm item add login
vault-pm item add secure-note
vault-pm item edit ITEM
vault-pm item delete ITEM
vault-pm item list
vault-pm item show ITEM
vault-pm history list ITEM
vault-pm history restore ITEM REVISION
```

`init` and every authenticated command require a controlling terminal even
when stdin is redirected. No passphrase flag, environment variable, config
field, URL, or stdin path exists. Unix integration tests launch this exact binary
under fresh pseudo-terminals, verify passphrases and item passwords are not
echoed, restart the process for durable item add/edit/list/show, inject decoy
bytes through stdin, verify redacted canonical history across another fresh
process, delete to a causal tombstone, restore an exact live ancestor into a
new revision, activate the signed audit epoch, force an invalid edit prompt in
a later process, verify that failure event from another process, inspect the
same verified history in newest-first order, select the failed edit by its
canonical trace in another process, verify both history accesses became
durable, produce a separately passphrase-encrypted portable artifact through
two hidden prompts, initialize and audit-enable an independent application
root, import the artifact through another hidden prompt, restart into redacted
restored items, and inspect both isolated filesystem trees for plaintext secret
bytes.

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```
