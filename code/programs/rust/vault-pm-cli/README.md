# `vault-pm`

The first installable local password-manager executable. It intentionally has
no product logic: it forwards process arguments to
`coding_adventures_vault_pm_cli`, writes that package's bounded public output,
and exits with its stable VLT-PM00 class.

The current command surface is:

```text
vault-pm init [--vault NAME] [--storage NAME]
vault-pm vault create NAME
vault-pm [--vault NAME] status [--json]
vault-pm [--vault NAME] audit enable
vault-pm [--vault NAME] audit verify
vault-pm [--vault NAME] audit list
vault-pm [--vault NAME] audit show TRACE
vault-pm [--vault NAME] doctor [--unlock]
vault-pm [--vault NAME] export FILE
vault-pm [--vault NAME] import FILE
vault-pm --vault NAME restore FILE
vault-pm [--vault NAME] restore verify FILE
vault-pm [--vault NAME] item add login
vault-pm [--vault NAME] item add secure-note
vault-pm [--vault NAME] item edit ITEM
vault-pm [--vault NAME] item delete ITEM
vault-pm [--vault NAME] item list
vault-pm [--vault NAME] item show ITEM
vault-pm [--vault NAME] history list ITEM
vault-pm [--vault NAME] history restore ITEM REVISION
vault-pm [--vault NAME] conflict list ITEM
vault-pm [--vault NAME] conflict choose ITEM REVISION
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
two hidden prompts, create a separately keyed named target in the same profile,
select it without changing the source default, open the artifact through
another hidden prompt, publish import with no intermediate output, independently
reopen the target in the same command for audited semantic verification,
restart into redacted restored items, reopen the untouched source, and inspect
the shared profile tree for plaintext secret bytes.

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```
