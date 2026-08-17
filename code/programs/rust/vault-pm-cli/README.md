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
vault-pm [--vault NAME] item add card
vault-pm [--vault NAME] item add api-key
vault-pm [--vault NAME] item add database-credential
vault-pm [--vault NAME] item add totp
vault-pm [--vault NAME] item edit ITEM
vault-pm [--vault NAME] item delete ITEM
vault-pm [--vault NAME] item list
vault-pm [--vault NAME] item show ITEM
vault-pm [--vault NAME] item reveal ITEM FIELD
vault-pm [--vault NAME] search QUERY
vault-pm [--vault NAME] history list ITEM
vault-pm [--vault NAME] history restore ITEM REVISION
vault-pm [--vault NAME] conflict list ITEM
vault-pm [--vault NAME] conflict reveal ITEM REVISION FIELD
vault-pm [--vault NAME] conflict choose ITEM REVISION
vault-pm [--vault NAME] conflict merge login ITEM BASE_REVISION
vault-pm [--vault NAME] conflict merge secure-note ITEM BASE_REVISION
vault-pm [--vault NAME] conflict merge card ITEM BASE_REVISION
vault-pm [--vault NAME] conflict merge api-key ITEM BASE_REVISION
```

`init` and every authenticated command require a controlling terminal even
when stdin is redirected. No passphrase flag, environment variable, config
field, URL, or stdin path exists. Unix integration tests launch this exact
binary under fresh pseudo-terminals. The primary drill verifies passphrases and
item passwords are not echoed, restarts the process for durable item
add/edit/list/show with ordered multi-URL fields and optional hidden login
notes, searches redacted URL metadata without echoing the query, explicitly
confirms audited current-password and login-notes reveals
directly on `/dev/tty` while captured process stdout remains empty, injects decoy
bytes through stdin, verifies redacted canonical history across another fresh
process, proves candidate-reveal denial and unconflicted failure advance the
audit chain without entering stdout or disclosing a secret, proves an
unconflicted authored-login merge fails before form collection and advances
the merge audit action, repeats that gate for authored secure-note,
payment-card, and API-key merges before their secret forms, deletes to a causal
tombstone, restores an exact live ancestor into a new revision, activates the
signed audit epoch, forces an invalid edit prompt in
a later process, verify that failure event from another process, inspect the
same verified history in newest-first order, select the failed edit by its
canonical trace in another process, verify both history accesses became
durable, produce a separately passphrase-encrypted portable artifact through
two hidden prompts, create a separately keyed named target in the same profile,
select it without changing the source default, open the artifact through
another hidden prompt, publish import with no intermediate output, independently
reopen the target in the same command for audited semantic verification,
restart into redacted restored items, reopen the untouched source, and inspect
the shared profile tree for password and notes plaintext bytes.

A separate payment-card drill creates a card through fixed metadata plus hidden
PAN/CVV prompts, restarts into holder/last-four/expiry-only rendering, reveals
PAN and CVV only through independently confirmed direct-terminal accesses,
verifies the advanced audit chain and its closed event-field grammar, and scans
the profile tree for the collision-resistant full PAN bytes. Exact typed
round-trips and redacted rendering cover the necessarily short CVV value
without relying on a collision-prone raw substring scan of random ciphertext.

A separate API-key drill collects a token only through the hidden controlling
terminal prompt, restarts into label/service/scope/expiry-only rendering,
reveals the token only through the existing separately confirmed audited
terminal ceremony, verifies the closed audit-row grammar, and scans the
profile tree for the collision-resistant full token bytes.

A database-credential drill repeats those gates for canonical static
connection metadata and a hidden password, including restart-backed redaction,
separate audited password reveal, and full-password plaintext-tree exclusion.

A TOTP drill accepts the seed only through a hidden canonical Base32 prompt,
restarts into algorithm/digits/period metadata with explicit secret redaction,
reveals canonical Base32 only after a separate audited confirmation, verifies
the closed audit-row grammar, and excludes both encoded and raw seed bytes from
the profile tree.

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```
