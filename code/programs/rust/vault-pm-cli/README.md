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
vault-pm [--vault NAME] shell
vault-pm [--vault NAME] audit enable
vault-pm [--vault NAME] audit verify
vault-pm [--vault NAME] audit list
vault-pm [--vault NAME] audit show TRACE
vault-pm [--vault NAME] doctor [--unlock]
vault-pm [--vault NAME] passphrase rotate
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
vault-pm [--vault NAME] conflict merge database-credential ITEM BASE_REVISION
vault-pm [--vault NAME] conflict merge totp ITEM BASE_REVISION
vault-pm [--vault NAME] conflict merge opaque ITEM BASE_REVISION
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
payment-card, API-key, database-credential, TOTP, and opaque-record merges
before their secret forms, deletes to a causal
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

`vault-pm shell` is the same binary in its second host shape: one foreground
process that reads command lines from the controlling terminal and runs them
through the identical parser and application boundary. It keeps one wipe-on-drop
passphrase between commands and nothing else, so an operator can work a session
without answering a prompt — and paying an Argon2id derivation — for every
single command. Two further pseudo-terminal drills cover it. The first proves a
real process unlocks once, runs several commands with no second prompt, still
collects an item password through the hidden terminal ceremony inside the
session, forgets its authenticator on `lock`, re-authenticates afterwards, exits
cleanly, and leaves no passphrase or item password in the transcript or the
profile tree. The second proves `init`, `vault create`, and a leading `--vault`
are refused inside a session without ending it, and that `Ctrl-D` ends the
process cleanly. Both spawn the shell with the same injected piped stdin every
other drill uses, so a redirected stdin is proven unable to supply a command as
well as unable to supply a secret.

## The crash/fault drill

`tests/local_cli_e2e.rs` proves what this executable does when it is allowed to
finish. `code/programs/rust/vault-pm-cli-drill` proves what happens when it is
not: its `tests/crash_fault_matrix.rs` kills a real process with `SIGKILL` at a
deterministically chosen durable write and then asks the next real process what
it can see and what it can repair.

That drill lives in a separate crate on purpose, and this crate carries the
guard rail. VLT-PM41 needs a binary built with
`coding_adventures_vault_pm_cli`'s `crash-injection` feature, and the obvious
way to get one — enabling it through this crate's `dev-dependencies` — is a
trap. Cargo resolves features per package across a build graph, so
`cargo build --release --all-targets` pulls dev-dependencies in and uplifts the
instrumented binary to `target/release/vault-pm`, the exact path a packaging
step copies from. This crate therefore names that feature in no section, and
the instrumented twin is `vault-pm-drill` in its own workspace.

Naming no feature is necessary and *not sufficient*, because
`--features <dep>/<feature>` reaches a direct dependency's features regardless
of what the root package declares. So `src/main.rs` also carries a `const`
assertion on `CRASH_INJECTION_COMPILED` — a `vault-pm` with the instrumentation
in it does not compile — and
`the_shipped_executable_contains_no_crash_injection` reads the binary this
crate produced and fails if either injection variable name appears in it.

Two results from the drill are worth reading before trusting this executable
with anything:

- An interrupted `init` is always repairable by running `init` again, and the
  resumed vault passes authenticated `doctor --unlock`.
- An interrupted **mutation** is repairable by doing nothing but retrying. The
  tree is never torn, the durable journal is exact, and the next command that
  opens the vault replays it with the passphrase it already asks for, then says
  `vault-pm: recovered an interrupted write` on standard error. There is no
  recovery verb to learn and no flag to pass, because the person who needs the
  repair is by definition someone whose ordinary command just failed.

  This is a repair, not a discovery: until
  `code/specs/VLT-PM42-cli-pending-publication-recovery.md`, an interrupted
  mutation left a vault every later command refused, as exit 2
  `vault-pm: invalid command`. See
  `code/specs/VLT-PM41-cli-crash-fault-matrix.md` section 8 for the finding and
  VLT-PM00 §23 item 10a for its closure.

- An interrupted **passphrase rotation** leaves exactly one working passphrase,
  at every landing point of the ceremony. Never both, which would mean the
  retired wrap survived the rotation that was supposed to retire it; never
  neither, which would mean the vault was bricked. Which of the two it is
  depends on one durable fact — whether the rotation journal landed — and the
  next ordinary command finishes whatever remains without asking for a secret,
  so a person who types the passphrase they had before the crash still gets
  their vault repaired, and then an honest `authentication required`.

`status` and `doctor` are the exception, deliberately. Both report an
interrupted vault as `recovery_required` — `doctor` with exit class 5 — without
collecting a passphrase and without repairing anything, so looking before
leaping, and restoring a pre-mutation backup instead, both stay available.

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```

The crash drill has its own `BUILD`, next to its own crate.
