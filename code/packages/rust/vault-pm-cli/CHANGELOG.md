# Changelog

## Unreleased

- Fixed backlog item #8 (VLT-PM05 §13.9 / `VLT-PM17-cli-portable-export.md`
  amendment): `vault-pm export FILE` gained an optional `--best-effort` flag.
  Without it, one item this build cannot re-encode still denies the whole
  export exactly as before — no behavior change for any existing caller or
  script. With it, such an item is excluded instead, and success reports the
  exact count and every excluded item's canonical id on standard output:
  ```text
  Portable export written.
  Excluded (too large to include): 2
  <item id>
  <item id>
  ```
  `--best-effort` is recognized only in the fixed `export FILE --best-effort`
  position — `export --best-effort FILE` is invalid, and a bare
  `export --best-effort` parses `--best-effort` as the destination, matching
  this command's pre-existing "a path beginning with `-` is a path value
  whenever it is the sole positional argument" rule. Reuses the existing
  `PortableExport` audit action; no new audit event kind. See VLT-PM05 §13.9
  for the complete design rationale, including the two alternatives
  (unconditional silent skip, and a persistent catalog-level quarantine
  state) that were weighed and rejected before this one.
- Fixed backlog item #16 (VLT-PM41 §8): `begin_init` and the fresh-target
  branch of `vault_create` now call
  `StorageCoreApplicationStore::reclaim_orphaned_preparations` immediately
  before installing their own new locator's `PreparedInit` journal, closing a
  storage leak where a crash strictly between that journal's write and the
  configuration write that would have named its locator left the journal
  durable forever under a locator nothing could ever discover again.
  `begin_init` passes an empty live-locator set, since no configuration exists
  yet; `vault_create` passes every locator the current configuration already
  names. See `VLT-PM05-application.md` §7.3 for the full argument.
- **`import otpauth-uri FILE`**, extending `VLT-PM49-cli-external-import.md`
  §5.5, at the user's explicit request for TOTP setup via `otpauth://` URI
  instead of only manual Base32 entry. Decodes a file containing exactly one
  `otpauth://totp/...` URI (the de facto Google Authenticator Key URI Format
  every authenticator issuer's QR code and manual-setup page encodes) into a
  single new `TOTP_SEED_V1` item, through the unmodified `item add`
  publication path — the same "new input into the existing audited creation
  ceremony" shape `import bitwarden`/`import csv` already established for
  Bitwarden/CSV TOTP fields (§5.3). A new sibling crate,
  `coding_adventures_vault_import_otpauth`, implements
  `vault-import-export`'s `Importer` trait and extracts only the URI's label
  segment for the item's title; the query string (`secret`/`issuer`/
  `algorithm`/`digits`/`period`) is handed unmodified to the existing,
  unchanged `decode_external_totp_field` / `parse_otpauth_totp_uri` decoder
  §5.3 already shipped and tested, so there remains exactly one place in the
  workspace that decodes an `otpauth://` query string. `otpauth://hotp/...`
  and every other non-`totp` type are refused with the invalid exit class,
  matching `VLT-PM29`'s TOTP-only scope — `item add totp`'s own closed
  grammar is untouched by this slice.
- **`import otpauth-qr FILE`** parses but always fails closed with the
  `unsupported` exit class before opening its file — turning a QR code
  *image* into its embedded `otpauth://` URI text is explicitly deferred
  (`VLT-PM49` §9), the same pattern §8 already established for
  `import kdbx`. `import otpauth-uri` above ships the URI-file half of this
  feature as a real, complete slice.
- **`storage add|list|check|migrate`**, `VLT-PM00` §23 item 14, the last item
  of Phase 1B. Specified by `VLT-PM50-cli-storage-migration.md`.

  `storage add filesystem|removable NAME PATH` registers a new named
  filesystem-family storage location in configuration; it neither creates
  the directory (the backend does that lazily on first use, exactly as
  `init` already relies on) nor opens any vault.
  `gdrive|webdav|s3` still parse and are refused with the `unsupported`
  exit class, the same closed-grammar answer `VLT-PM49` §8 gave `import
  kdbx` — Phase 2's job. `storage list` lists every configured location.
  `storage check NAME` reports one location's reachability, runs
  `vault-pm-storage-removable`'s third-party sync-interference scan for a
  local-directory kind, and reports any configured mirror's coarse
  object-count health.

  `storage migrate SOURCE TARGET [--mirror]` implements the
  filesystem-family slice of `VLT-PM00` §19.1's seven steps:
  `vault-pm-storage-removable::copy_object_tree` copies and read-back
  verifies every committed object (steps 2-4), then the freshly collected
  passphrase is used to independently unlock `TARGET` over a repository
  factory pointed only at its objects (step 6) — a wrong passphrase or a
  corrupt copy both fail this exact step before configuration is ever
  touched, which *is* the "explicit confirmation" step 7 asks for, rather
  than a second invented ceremony. Only then does configuration switch:
  without `--mirror`, `local_store` moves to `TARGET`; with it, `TARGET`
  joins `remote_stores` and `local_store` is unchanged. `SOURCE` is never
  modified or deleted (step 8's default; no `--delete-source` flag exists in
  this slice).

  `configured_vault`'s storage-location check, previously hardcoded to the
  exact two paths this composition root itself creates, now accepts any
  registered `filesystem`/`removable` location and any `remote_stores` that
  resolve the same way — the restriction predates `storage add` existing at
  all and would otherwise make the new command pointless.

  Every repository this composition root opens now goes through
  `vault-pm-storage::ReplicaSetObjectStore` (with zero configured mirrors by
  default, a verified no-op pass-through) rather than the bare
  `StorageCoreObjectStore` — so a vault mirrored via `storage migrate
  --mirror` gets real, ongoing, best-effort mirror-write propagation on
  every subsequent mutation, not just at migration time.

  **Deferred**, per `VLT-PM00` §23 item 14's own scoping: the explicit `sync
  --wait` ceremony with a configurable `one`/`all`/quorum durability target,
  and treating a change feed rather than write-time propagation and
  directory-scan counts as the source of replica truth. `storage check`'s
  replica status line is a labeled structural heuristic (an object-file
  count comparison), not a cryptographic guarantee, and says so in its own
  documentation.

  `configured_vault`'s cross-vault storage-collision check now falls back
  to comparing canonicalized paths when two locations' raw strings differ
  (`same_local_directory`), so a relative path or a symlink cannot make
  two different-looking `storage add`ed locations silently alias one real
  directory — found in this PR's own security review, since the previous
  exact-string comparison predates any location but the two this
  composition root itself created ever being possible.

  Tests: closed-grammar coverage for all four verbs including `--mirror`
  and the `--vault` selector split (`add`/`list`/`check` refuse one,
  `migrate` accepts one); registering and listing a location, duplicate and
  cloud-kind rejection; `storage check` across unreachable, healthy,
  sync-interference, missing-name, and cloud-kind-injected states, with an
  assertion that a conflict-copy filename never appears in the report;
  `storage migrate` switching primary storage while leaving the source
  directory untouched and the previously created item still readable;
  `storage migrate --mirror` adding a replica and proving a *subsequent*
  item creation actually propagates to it, then that `storage check` reports
  it `in_sync`; a wrong passphrase leaving configuration unchanged even
  though the (harmless, verified) copy already happened; and source/target
  identity and reference-integrity rejections.

- **`import portable|bitwarden|csv|kdbx FILE`**, `VLT-PM00` §23 item 13.
  Specified by `VLT-PM49-cli-external-import.md`. The bare `import FILE`
  grammar (VLT-PM18) is now `import portable FILE`; `import bitwarden FILE`
  and `import csv FILE` join it, each decoding an unencrypted external
  export with a new dependency-light adapter crate
  (`vault-import-bitwarden`, `vault-import-csv`) implementing
  `vault-import-export`'s (VLT15) `Importer` trait — the first real
  consumer of that trait in this workspace. `import kdbx FILE` still parses
  but always fails closed with the `unsupported` exit class before opening
  its file: KDBX's own encrypted-container format is explicitly deferred
  (VLT-PM49 §8), not silently missing from the documented command surface.

  Every mapped record is created through the exact same audited `item add`
  publication path a person typing at the CLI uses, once per record —
  `add_item`'s session-consuming design already makes "one authenticated
  session creates one item" structural, so this reuses that path N times
  rather than inventing a new bulk-mutation primitive. No new audit event
  kind was introduced: each created item carries the same `ItemCreate`
  event `item add` already produces. Imports always create brand-new items
  with fresh identities and never merge or conflict-resolve against an
  existing item, because an external format's records carry no vault-pm
  item ID to collide with in the first place — the same "always new
  identity" answer VLT-PM18 §7 gives portable restore, reached here for a
  simpler reason.

  A Bitwarden login carrying a TOTP seed becomes two vault-pm items (a
  `LOGIN_V1` and a separate `TOTP_SEED_V1`), because vault-pm's own `Login`
  record has no TOTP slot. TOTP fields are decoded from either raw
  (possibly padded/lowercase) Base32 or an `otpauth://totp/...` URI,
  through the same `decode_totp_base32` the interactive `item add totp`
  form already uses. Output is aggregate-only —
  `Import complete: created=C skipped=S failed=F` — with no source path,
  title, username, URL, or secret ever printed.

  Adds `read_external_import_source` to the `CliHost` trait, backed by a
  new bounded, `Zeroizing`-returning reader in `vault-pm-cli-host`
  (`read_external_import_source`), modeled on the existing attachment
  reader rather than the portable-export one: unlike a vault-pm portable
  artifact (already ciphertext), a Bitwarden/CSV export *is* the person's
  plaintext secrets.

- **`agent start|stop|status|unlock|lock`**, `VLT-PM00` §23 item 12.
  Specified by `VLT-PM48-local-agent-ipc.md`. `agent start` re-executes this
  same binary, detached, as the hidden `agent run-foreground` verb, which
  binds a permission-checked Unix domain socket
  (`coding_adventures_vault_pm_agent_host`) and retains one passphrase per
  vault name in memory until an explicit `agent lock`, `agent stop`, or its
  own `auto_lock_seconds` idle bound elapses — enforced by a real background
  sweep thread, the pre-emptive auto-lock timer `VLT-PM40-cli-interactive-
  shell.md` §3.5 named as this slice's own deferred work.

  `agent unlock` authenticates exactly once, through the same
  `open_authenticated_access` unlock step every other command uses, and hands
  the agent a passphrase only after that open already succeeded against the
  real vault — the agent itself verifies nothing and cannot, since
  `vault-pm-agent-host` has no dependency on `vault-pm-application` at all.

  Every authenticated command now funnels its passphrase collection through
  one new seam, `agent::passphrase_for`: a running, unlocked agent removes
  the terminal prompt; anything else — no agent, an expired bound, a
  different vault — falls back to the unmodified one-shot prompt
  unconditionally. `passphrase rotate` is the one exception and always
  prompts fresh, for the same reason `vault-pm shell` refuses to delegate
  `passphrase` at all (`VLT-PM43-cli-passphrase-rotation.md` §3.1); a
  successful rotation also forgets that vault's cached passphrase
  immediately, and any command that comes back `Locked` triggers the same
  best-effort forget, mirroring `ShellSession`'s in-process self-heal.

  The interactive shell refuses the whole `agent` noun, not verb by verb:
  `agent run-foreground` inline would block the session's own prompt forever,
  the same mistake a nested `shell` already is.

  Windows named-pipe support is explicitly deferred (`VLT-PM48` §9); every
  agent command reports the closed `unsupported` exit class there rather than
  silently doing nothing.

- **`attachment add`, `attachment list`, and `attachment export`**, the last
  piece of `VLT-PM00` §23 item 11. Specified by
  `VLT-PM47-cli-attachments.md`. The verbs are spelled as §14.4 already
  published them.

  One deviation from that table, recorded there: the export destination is
  required rather than bracketed. The only available default was the stored
  attachment name resolved against the working directory, and in a synced
  vault that name is authored by whoever attached the file. Nothing in this
  product turns a stored name into a filesystem path.

  `add` validates the base name and reads the source *before* the passphrase
  prompt, so a missing file, a directory, or one over the ceiling costs no
  terminal interaction — the position `VLT-PM44` §2.3, `VLT-PM45` §2.3, and
  `VLT-PM46` §3.2 all put a pre-flight check in, for the same reason. The
  entropy block is sized from the file's length and reserved before
  authentication, like every other mutation's.

  `export` runs `VLT-PM25`'s ceremony with a third confirmation sentence, for
  `VLT-PM46` §3.1's reason: neither existing prompt describes writing vault
  content into an ordinary unencrypted file this product will not track,
  clear, or know about again. The intent stays `InteractiveReveal`
  (`VLT-PM46` §3.0). Refusal, or a host failure collecting the answer,
  publishes `Denied` and writes nothing.

  `attachment remove` is deferred to `gc run` by that document's §2.2:
  removing a reference while every byte stays in the store is not the removal
  the word promises.

- `DurableStep::AttachmentArtifact` is bracketed around the exported file, the
  one durable write this ceremony makes outside the storage backend.

- **`attachment list` renders the stored name through `quoted`**, like every
  other stored string this CLI prints. The name is the most peer-authorable
  string in the product — in a synced vault it was typed on another device —
  and it is what an operator reads to choose which attachment to export, so it
  gets the escape as well as the application layer's validation.

- **`export`, `import`, and `restore` now write a fixed notice to standard
  error when attachments were left behind**:
  `vault-pm: portable export does not carry attachments`. Same shape as the
  VLT-PM42 recovery notice — payload-free, standard output unchanged, exit
  class unchanged. A snapshot carries records and not blobs, so without this an
  operator was told an export succeeded and later told a restore was
  *verified*, with nothing anywhere saying their attachments had not travelled.
  `restore` matters most of the three, because *verified* is the word a person
  reads as "everything came back".

- **`--copy` now works** on `password generate` and `totp code`, the third
  piece of `VLT-PM00` §23 item 11. Specified by `VLT-PM46-cli-clipboard.md`.
  Both commands have parsed `--copy` and then refused it with the unsupported
  class since they shipped, because no clipboard adapter existed anywhere in
  the product. The adapter is `coding_adventures_vault_pm_cli_host::clipboard`;
  this crate is the wiring.

  **`--copy` is not a new disclosure path.** Every step of both ceremonies
  happens in the same order with the same audit consequences; only the final
  delivery differs. `totp code --copy` still reserves its audit inputs before
  authentication, still unlocks, still reads the clock a second time, and still
  publishes one `ItemRead` event before releasing anything, with VLT-PM45 §3.1's
  outcome table unchanged. Its non-secret validity line still goes to standard
  output. `password generate --copy` still publishes nothing, because minting a
  password no vault stores is not a vault access.

  **The confirmation prompt is new text**, because the old one would be a false
  statement: "Copy secret to this system's clipboard?" rather than "Reveal
  secret on this terminal?". A consent ceremony that misdescribes what it is
  consenting to is worse than none, since it manufactures a record of an
  agreement nobody made.

  **Clipboard availability is checked first**, before any prompt, unlock, clock
  reading, entropy reservation, or audit event — exactly where the old blanket
  refusal sat, and for the same reason. Only the condition narrowed, from
  "always" to "when this host has no clipboard", so a headless runner still
  gets `unsupported` (exit 8) without being asked for a passphrase first.

  **Which timeout is used differs by command, and that is forced rather than
  chosen.** `totp code` opens a vault, so the selected vault's
  `clipboard_clear_seconds` is already in hand and is used. `password generate`
  may not read config at all — VLT-PM44 §1 requires it to resolve no platform
  layout and to work where `init` has never run — so it uses the product
  default of 30. VLT-PM46 §6 states the consequence rather than hiding it.
- **Added `vault-pm clipboard clear`**, the detached half of `--copy`: the
  process `vault-pm` re-executes itself as so that a clear scheduled thirty
  seconds out survives the exit of a one-shot process. It is listed in `USAGE`
  rather than hidden, because a closed grammar with a secret verb in it is not
  a closed grammar. It opens no vault, resolves no platform layout, takes no
  writer lock, prompts for nothing, and publishes no audit event; typed by hand
  it reads zero bytes and exits 2. Its parameters arrive on standard input
  precisely because argv is world-readable through `ps`. An interactive session
  refuses the verb: a shell's standard input is a person's terminal, not the
  pipe a parent wrote.
- Added `CliHost::confirm_secret_copy`, `CliHost::ensure_clipboard_available`,
  `CliHost::copy_revealed_text`, and `CliHost::run_scheduled_clipboard_clear`,
  so every clipboard effect crosses one injected seam and the CLI-level tests
  need no display server.
- Factored `authenticated_access` into `open_authenticated_access`, which also
  returns the selected vault's `clipboard_clear_seconds`. The twenty-nine
  existing call sites are unchanged.

- **Added `vault-pm [--vault NAME] totp code ITEM (--reveal|--copy)`**, the
  second of `VLT-PM00` §23 item 11. Specified by
  `VLT-PM45-cli-totp-code.md`.

  `item add totp` could store a seed and `item reveal ITEM totp-secret` could
  hand it back for re-provisioning; neither is the reason anyone puts a TOTP
  seed in a password manager. This command computes the six digits that are
  valid right now.

  It is the opposite of `password generate` in nearly every respect: it opens a
  vault, requires the passphrase, resolves an item, and publishes an audit
  event. `VLT-PM15` §2 already names "TOTP display" in its list of accesses, so
  the ceremony is `item reveal`'s unchanged — the same exact-`yes` terminal
  prompt, the same `Denied`/`Failed`/`Succeeded` outcomes on the same
  `ItemRead` action, the same publish-before-release ordering. VLT-PM45 §3
  records the argument for a lighter treatment (a code lives ~30 seconds and
  does not yield the next one) and rejects it as an argument about the
  consequence of a disclosure rather than the fact of one.

  **The clock is read twice.** The audit timestamp is reserved before
  authentication as usual; the code time is read again after unlock and after
  the confirmation answer. An Argon2id derivation and a human reading a prompt
  sit between the two, so several seconds is ordinary and a whole period is
  reachable — a command that reused the reserved reading would routinely return
  the *previous* code, correct-looking and rejected by the site. There is no
  NTP query and no drift correction; TOTP correctness depends on the host
  clock, as it does for every TOTP client.

  Output is split by sensitivity, so unlike `item reveal` this command's
  standard output is not empty: the code goes only to the controlling terminal
  through the §14.6 adapter, while one non-secret line — `Code valid for N more
  seconds` — goes to standard output, because it is a function of the clock and
  the stored period that anyone with a watch can reproduce. When `N` is small
  the command reports the small number and returns rather than sleeping (which
  would hold an unlocked vault open for a duration nobody chose) or handing
  back the next step's code (which is not valid yet).

  The code is computed inside `vault-pm-application`; the decoded seed never
  reaches this crate. Building the command as "reveal the seed, then compute"
  would have materialized the seed in the outermost layer, next to the argument
  parser and the terminal.

  `--copy` is recognized and refused with the unsupported class before any
  prompt, unlock, clock reading, or entropy reservation — identical to the
  generator's refusal, so both stop refusing on the day a clipboard adapter
  lands. A live refreshing display is deferred by VLT-PM45 §8.

  The verb is delegated inside the interactive shell *with* the session's bound
  vault prefix, unlike `password generate`, because it does have a target.

  Tests cover the closed grammar (thirteen rejected spellings, including a
  missing output flag, both flags, a repeated flag, and a lowercase selector);
  the exact RFC 6238 answer `921300` against the frozen test clock, which sits
  at a step the RFC publishes for the Appendix B seed; that the code comes from
  the *fresh* reading rather than the reserved one, using a host whose clock
  advances a whole period per reading; the `--copy` refusal with no scripted
  passphrase or confirmation at all, so a prompt would have failed differently;
  four refused confirmations each publishing `Denied`; a login and a missing
  item; and one shell session.

  A failure of the *fresh* clock reading is folded into the same channel as a
  failure to collect the confirmation, rather than returned from an early `?`.
  An early return would have been the one path through this command on which an
  authenticated attempt reached the confirmation prompt and then left no audit
  row — precisely the "an access that happened and left no trace" outcome the
  ceremony exists to prevent. The attempt instead proceeds as unconfirmed,
  publishes `Denied`, and only then returns the payload-free provider error.
  Failing the *first* reading is unchanged and still leaves no row, because
  `VLT-PM25` §3 requires that a pre-authentication failure not claim an item
  access occurred. A test asserts both, since they return the same class to the
  caller and differ only in the audit trail.

- **Renamed `PasswordOutputMode` to `SecretOutputMode`.** It is now shared by
  the two commands whose entire output is a live credential.

- **Added `vault-pm password generate`**, the first of `VLT-PM00` §23 item 11
  and the first command in this grammar that opens no vault. Specified by
  `VLT-PM44-cli-password-generate.md`.

  It is dispatched beside `help`, before `execute`: no platform layout is
  resolved, no cross-process writer lock is taken, no passphrase is collected,
  and no audit event is published. It works on a machine where `init` has never
  run, which is the most common moment to want a generated password. VLT-PM44
  §1 records the three reasons that scoping is deliberate — `VLT-PM15` §2
  already exempts operations that reveal no vault content, a vault-scoped event
  would correlate an instant with whichever item is created next, and requiring
  the master passphrase for an operation that never opens the vault would train
  a person to type it at prompts that do not need it.

  The command surface is `[--length N] [--no-lowercase] [--no-uppercase]
  [--no-digits] [--no-symbols] [--exclude-ambiguous] (--reveal|--copy)`. This
  is the crate's first real flag parser; every other command's arguments are
  positional or a single boolean and are matched as exhaustive slice patterns.
  Duplicate rejection needs no bookkeeping flags because the state being built
  *is* the record — a class starts selected and can only be cleared once, an
  output mode starts absent and can only be set once — so a second occurrence
  has nowhere to go and falls into the same `_` arm as an unknown flag.

  `--length` accepts one to three ASCII decimal digits with no sign, no
  separator, no whitespace, and no leading zero. `str::parse` alone would take
  `+24` and `007`, and a grammar with two spellings of one number is a grammar
  where a typo can silently mean something else.

- **Exactly one output mode is required**, and there is no plain-stdout mode.
  VLT-PM00 §14.6 makes ordinary output redacted, and this command has nothing
  but a secret to say: a default stdout mode would put a live credential into
  shell history, terminal scrollback, `tee` pipelines, and CI logs the first
  time anyone redirected it.

  `--reveal` reuses `item reveal`'s fixed confirmation prompt and terminal
  adapter unchanged rather than inventing a second ceremony, so the value is
  quoted, control-escaped, written to the reopened controlling terminal, and
  never enters `CliOutput`, stdout, stderr, argv, or a `Debug` rendering.
  Confirmation runs *before* generation, which buys a property `item reveal`
  cannot have: on refusal no password is ever created, so there is no secret to
  wipe. Nothing forces the other order here, because unlike a stored-secret
  reveal there is no audit event that must be published before the answer is
  known.

  `--copy` is recognized and refused with the unsupported class. No clipboard
  adapter exists anywhere in this product — `clipboard_clear_seconds` is a
  configuration value with no writer behind it — and §14.4 documents the flag,
  so someone who reads the specification and types it deserves "not yet" rather
  than "invalid command". It is refused before any prompt, since confirming a
  reveal nobody asked for and then failing would be worse than failing at once.

- **Added `CliFailure::WeakPasswordPolicy`**, carrying the invalid exit class
  with its own fixed message, `vault-pm: password policy below the minimum
  entropy floor`. It is the one rejection in this grammar a person will hit
  while doing something entirely reasonable — asking for a shorter password, or
  narrowing the alphabet for a site that refuses symbols — and "invalid
  command" would send them hunting for a typo that is not there. The message
  names no length, alphabet, or bit count.

- **The `--vault` selector is refused for `password generate`**, joining
  `init`, `vault create`, and `help`. The selector names a target for a command
  to operate on, and this command operates on none; accepting and ignoring it
  would let someone believe they had aimed a command that was never aimable.

  The interactive shell prefixes every delegated command with its bound vault,
  so it now asks `takes_no_vault_selector` first and delegates this one verb
  unprefixed. Refusing the verb in the shell instead would have been the
  smaller change and the worse one: a generator is exactly what you reach for
  mid-session.

- **Randomness comes from the operating-system CSPRNG** through the existing
  `CliHost::fill_entropy` → `OsEntropy::fill` → `csprng::fill_random` path, in
  one reservation, into a wipe-on-drop buffer. No new host method, no new
  adapter, and no general-purpose RNG. The policy and sampler live in the new
  pure `coding_adventures_vault_pm_password_policy` crate, which sources no
  randomness at all.

- Added ten unit tests and 25 new closed-parser rows, covering the flag surface
  and its rejections, the entropy floor on both sides of three documented
  policies, terminal-only delivery with empty stdout and stderr, determinism
  under a fixed host and divergence under a different one, narrowed alphabets,
  five confirmation answers that must not authorize a reveal, `--copy`,
  an unavailable CSPRNG, an untouched vault root, and a full shell session that
  runs the verb without ever unlocking.

- **Added `vault-pm [--vault NAME] passphrase rotate`**, the ceremony
  `VLT-PM00` §14.8 has required since before there was any code and that
  nothing performed. Specified by `VLT-PM43-cli-passphrase-rotation.md`; closes
  §23 item 10b.

  The order of the two prompts is the whole safety argument. The *current*
  passphrase comes first, because it is the authentication and because someone
  who cannot produce it must be told so before being asked to invent a
  replacement. The *new* one is collected and confirmed second, against an
  already unlocked vault, so a typo is caught while the old passphrase is still
  the only one that means anything. Nothing durable happens until both are in
  hand and the next bootstrap is built and signed, so every failure up to that
  point leaves a vault the current passphrase still opens.

  The verb takes no arguments at all. §14.5 forbids a passphrase reaching this
  process through argv, an environment variable, command history, a URL, or
  config, and a flag naming a file or a policy would be the first step toward
  one that named a secret.

- **The interactive shell refuses `passphrase`**, joining `init`, `vault`,
  `shell`, and `--vault`. The reason is sharper than for the others: a session's
  entire premise is that the authenticator it collected once still opens the
  vault, and a successful rotation is precisely the event that makes that
  false. Permitting it would leave two bad options — keep using a passphrase
  that no longer works, turning every later command into an authentication
  failure the person cannot explain, or silently adopt the new one, which is a
  retained secret the session never prompted for and cannot re-confirm.

- `init` and `vault create` resume an interrupted *rotation* as well as an
  interrupted publication, through the renamed `resume_interrupted_write`. A
  person reaching that path after a crashed rotation must type the **new**
  passphrase — the one they had just confirmed when the machine stopped —
  because the roll-forward itself consumes none.

- **Fixed the availability defect `VLT-PM41-cli-crash-fault-matrix.md` §8
  found.** A process killed inside the shared mutation publication path left a
  vault that was intact and one journal replay from healthy, and that every
  subsequent command refused — as exit 2, `vault-pm: invalid command`. The
  person was told their command was wrong, over and over, about a vault that
  was fine. `VLT-PM42-cli-pending-publication-recovery.md` is the repair.

  It adds no verb, no flag, no file format, no on-disk artifact, and no
  environment variable. The vault-open path finishes the interrupted
  publication with the passphrase it has already collected, through the
  application's new `unlock_recovering_pending_publication`, and then opens the
  repaired vault through the ordinary strict open. Every authenticated command
  (`item` CRUD/list/show/reveal, `search`, `history`, `conflict`, `audit
  enable`/`list`/`show`, `import`, `restore`), `export`, and `audit verify`
  take that path, so the repair reaches a person on whatever command they
  happened to retry.
- `init` and `vault create` resume paths now finish a `PendingPublication`
  instead of refusing it with the conflict class, and report
  `Vault recovered.`. "Finish what was interrupted" is what those paths already
  meant for a `PreparedInit` journal; a pending publication is the same promise
  one generation later, and `init` is the verb a stuck person retries. A vault
  that is merely already initialized is still refused, unchanged.
- `doctor` is deliberately **not** a repair, and `--unlock` does not make it
  one. A wedged vault now short-circuits the authenticated half entirely — no
  passphrase is collected and nothing is published — and the read-only
  diagnostic answers `recovery_required` with exit class 5. Only the
  classification changes: this case used to inherit the refused open's
  misleading exit 2. `status` is untouched and still reports without repairing,
  which is what keeps restoring a pre-mutation file-level backup a real option
  rather than a race against an eager repair.
- Added one fixed, payload-free notice on standard error,
  `vault-pm: recovered an interrupted write`, emitted exactly when a command
  moved the durable state out of `recovery_required`. The composition root
  observes that transition across the command, both reads inside the
  cross-process writer lock the command already holds — which is what makes the
  inference sound, since no other local writer can move the state between them.
  *Both* observations must succeed for the claim to be made, and the security
  review of this change is why that sentence is worth writing down: an
  unobservable after-state satisfies "not `RecoveryRequired`" while proving
  nothing, so reading it that way would announce a repair on a vault that is
  still wedged. `observed_a_repair` states the whole truth table and is pinned
  by a test. The second reading is also *conditional* on the first having found
  `recovery_required`, which is a requirement rather than an optimization:
  reading owner state initializes its backend, a backend initialization is a
  durable step VLT-PM41 kills processes at, and reading after every command
  would append durable writes past each ceremony's own last one. An observation
  about a command must not move the command. Standard output and every exit class are unchanged, and the notice
  is attached to a failing command too, because a repair is worth saying even
  when the verb that triggered it went on to report `not found`.
- No change to the crash-injection isolation. The `crash-injection` feature is
  still named in no section of the product crate, still enabled only by
  `code/programs/rust/vault-pm-cli-drill`, and the product executable still
  fails to compile with it. The new tests reach a wedged vault through
  `vault-pm-storage`'s ordinary fault-injecting object store — a plain
  dev-dependency that enables no feature — rather than through that seam.

- Added the composition root's durable-write seam for
  `VLT-PM41-cli-crash-fault-matrix.md`. The new `crash` module names every
  point at which this package makes something durable, so a drill can kill the
  real process at a chosen one: backend writes through a `LocalBackend` type
  alias, plus the two writes that do not go through a backend — the first
  creation of the client configuration file and the creation of an encrypted
  portable-export artifact. The seam lives here rather than in
  `vault-pm-application` because the application layer is deliberately
  storage-agnostic and owns no filesystem authority, so it is not the layer
  that knows what "durable" means.
- Added the non-default `crash-injection` feature that selects the instrumented
  half of that module. With the feature off — the only configuration the
  product executable is ever built in — `LocalBackend` is exactly
  `FsStorageBackend`, each combinator is an `#[inline]` function whose body is
  `action()`, and the crash-injection package is an optional dependency that is
  not compiled at all. No behavior, output, exit class, file, or on-disk format
  changes in either configuration. Only `code/programs/rust/vault-pm-cli-drill`
  enables the feature; the product crate names it in no section, because Cargo
  resolves features per package and naming it even in `dev-dependencies` would
  let `cargo build --all-targets` uplift an instrumented binary to
  `target/release/vault-pm`.
- Added `CRASH_INJECTION_COMPILED`, a public `const` a composition root can
  assert on to turn "this build must not contain crash injection" into a
  compile error. Declaring no feature is necessary and not sufficient: cargo's
  `--features <dep>/<feature>` syntax reaches a direct dependency's features
  even when the root package declares none of its own, so the product
  executable asserts on this constant as well.
- Corrected the `crash` module's own documentation, which still described the
  rejected `dev-dependencies` design as the live one. That matters more there
  than anywhere else: it is the module implementing the seam, so its doc
  comment was telling the next maintainer to do the unsafe thing.
- Added `vault-pm [--vault NAME] shell`, the foreground interactive session
  host specified by `VLT-PM40-cli-interactive-shell.md`. It adds no capability:
  every command inside a session runs through the same parser, the same
  application use-case boundary, the same publish-before-release audit
  ordering, and the same closed exit classes as its one-shot invocation. A
  session binds one vault at start, collects one wipe-on-drop authenticator
  lazily on the first command that unlocks, and thereafter runs commands
  without re-prompting. Each command still performs its own verified open,
  consumes its own session, and acquires the cross-process writer lock only for
  its own duration, so no pinned repository head is reused and an idle prompt
  blocks no other process. `lock` wipes the authenticator, a rejected
  passphrase or an unreadable clock wipes it, the configured
  `auto_lock_seconds` bound wipes it when a command is submitted and again when
  the value is handed to an unlock — never merely before the prompt was
  printed, which would let an unattended session serve a stale authenticator to
  whoever types next. An unreadable clock and a clock that has stepped
  *backwards* since collection both expire the value, since advisory wall time
  is not monotonic and a saturating comparison would otherwise suspend the
  bound for exactly as long as the machine's clock was wrong. `exit`, `quit`,
  or end of input ends the session. `init`, `vault`, a nested `shell`, and a
  leading `--vault` are refused inside a session. Command lines are read from
  the controlling terminal, never from process standard input, so a redirected
  stdin can supply neither a secret nor a command.
- Added the `shell` module's public surface: `ShellTerminal`, the injected
  boundary a session reads command lines from and renders results to;
  `NativeShellTerminal`, the production adapter that reads `/dev/tty` and
  writes the process standard streams; and `run_with_terminal`, which is `run`
  with that boundary supplied so a session can be driven by a test script.

- Added audit-required `conflict merge opaque ITEM BASE_REVISION`, the last
  authored merge ceremony, which retains the exact current opaque record
  together with the content type it must keep, collects the whole
  canonical-CBOR payload as one hidden lowercase hexadecimal line, forwards that
  line verbatim for application-owned closed validation, durably records host
  and validation failures, and publishes one authored all-current-parent record
  without exposing prior candidate values.
- Added audit-required `conflict merge totp ITEM BASE_REVISION`, which retains
  the exact current TOTP seed opaquely, collects the Base32 seed through a
  hidden prompt, forwards the seed and parameter lines verbatim for
  application-owned closed validation, durably records host and validation
  failures, and publishes one authored all-current-parent seed without exposing
  prior candidate values.
- Added audit-required `conflict merge database-credential ITEM BASE_REVISION`,
  which retains the exact current database credential opaquely, collects the
  password through a hidden prompt, forwards the engine and port lines verbatim
  for application-owned closed validation, durably records host and validation
  failures, and publishes one authored all-current-parent static credential
  without exposing prior candidate values.
- Added audit-required `conflict merge api-key ITEM BASE_REVISION`, which
  retains the exact current API key opaquely, collects the token through a
  hidden prompt, forwards the scope and expiry lines verbatim for
  application-owned closed validation, durably records host and validation
  failures, and publishes one authored all-current-parent result without
  exposing prior candidate values.
- Added audit-required `conflict merge card ITEM BASE_REVISION`, which retains
  the exact current card opaquely, collects PAN/CVV through hidden prompts,
  durably records host and validation failures, and publishes one authored
  all-current-parent result without exposing prior candidate values.
- Added audit-required `conflict merge secure-note ITEM BASE_REVISION`, with an
  opaque exact-current note base, hidden complete body input, durable
  precondition/host failures, and atomic all-current-parent success.
- Added audit-required `conflict merge login ITEM BASE_REVISION`, which keeps
  the exact current login base opaque, collects a complete bounded terminal
  form, durably records precondition/prompt/entropy/validation failures, and
  publishes one all-current-parent authored revision on success.
- Added audit-required `conflict reveal ITEM REVISION FIELD`, which accepts
  only an exact current conflict candidate, reuses the exact-`yes` ceremony,
  publishes denial/failure/success before release, and writes the selected
  secret only to the controlling terminal.
- Added audited `search QUERY` over the application-owned wipe-on-lock
  projection, with zeroizing/redacted query ownership, a fixed 100-result cap,
  deterministic list-row rendering, and durable failed semantic queries.
- Extended login add/edit to collect zero-to-sixteen ordered URLs plus optional
  hidden notes, accept existing multi-URL records, replace the complete form,
  redact notes presence, audit invalid counts before returning, and expose
  notes only through the separate audited reveal ceremony.
- Added audited `item add totp` with canonical hidden Base32 seed input, closed
  algorithm/digits/period validation, metadata-only rendering, durable failure
  events, and separately authorized publish-before-Base32 reveal.
- Added audited `item add database-credential` with canonical static engine and
  port validation, hidden password input, metadata-only rendering, durable
  failure events, and separate VLT-PM25 password reveal reuse.
- Added audited `item add api-key` with a hidden token prompt, closed scope and
  expiry validation, redacted metadata rendering, durable failure events, and
  separate VLT-PM25 token reveal reuse.
- Added audited `item add card` with hidden PAN/CVV prompts, closed offline
  validation, redacted holder/last-four/expiry rendering, durable failure
  events, and separate VLT-PM25 reveal reuse.
- Added audit-required `item reveal ITEM FIELD` with exact-`yes` controlling
  terminal confirmation, application-owned current-revision selection, durable
  denied/failed/succeeded outcomes, and direct escaped terminal delivery that
  never enters ordinary CLI output.
- Added audit-required `conflict list ITEM` and `conflict choose ITEM REVISION`
  with redacted candidate rows, item-bound selection, durable failed attempts,
  and atomic choose-existing resolution.
- Added explicit-named-target `restore FILE`, which opens the artifact once,
  publishes audited import without intermediate output, independently reopens
  the durable target, and claims completed-and-verified only after its audited
  semantic comparison succeeds.
- Reserve both audit traces before restore mutation and retain standalone
  `restore verify FILE` as the safe retry after a post-import interruption.
- Added audit-first `vault create NAME` with a distinct adapter namespace,
  trace-before-config ordering, exact prepared-journal retry, and no replacement
  of active targets.
- Added command-scoped `--vault NAME` selection across existing vault commands;
  it preserves `default_vault` and routes authenticated operations only through
  the selected vault's independent state, repository, and audit chain.
- New `init` operations use audit-first generation zero, making the encrypted
  signed `VaultInitialize` event the first repository commit and audit head.
- `audit enable` is an idempotent no-write success on new vaults while the
  explicit epoch-start migration remains available for legacy pre-audit state.
- Added retryable audit-required `restore verify FILE`, which authenticates the
  current target and encrypted artifact independently, prepares the opaque
  source expectation, and releases aggregate verified counts only after a
  succeeded `PortableRestoreVerify` event is durable.
- Record source-read, prompt, artifact-open, expectation, and semantic mismatch
  failures as failed itemless verification events without path or mismatch
  detail.
- Added audit-required `import FILE` with bounded artifact reads, hidden
  artifact-passphrase input, no-write authentication, count-derived entropy,
  and atomic cross-vault re-identification into an empty target.
- Record artifact/host failures as failed itemless `PortableImport` events and
  retain retry eligibility across audit-only attempts.
- Added `export FILE` with a separately confirmed hidden passphrase, canonical
  encrypted portable artifact, publish-before-release audit ordering, and an
  explicit create-new destination that never overwrites an existing path.
- Reserve export and audit entropy before unlock so active-epoch export prompt
  failures become durable itemless `PortableExport` events before their CLI
  error is returned.
- Added audited `item add secure-note` with a hidden bounded body prompt and
  explicit list/show rendering that never receives or prints body plaintext.
- Centralized login and secure-note creation on one preflight, durable failure,
  document, and completion path so future record kinds inherit the same audit
  ordering.
- Reserve create time, identities, and audit-failure entropy before unlock so
  active-epoch item prompt failures become durable traceable `ItemCreate`
  events before their CLI error is returned.
- Added authenticated `audit list` and canonical `audit show TRACE`; both
  publish one durable `AuditRead` before rendering verified trace-aware rows.
- Added closed canonical trace parsing, bounded newest-first output, audited
  missing-trace results, tamper rejection, and ambiguous-provider recovery
  coverage for the explicit audit surface.
- Exposed idempotent authenticated `audit enable`, installing the one durable
  `AuditEpochStart` migration event before any active-epoch command can run.
- Route `item edit ITEM` through an opaque application-owned preparation so
  active-epoch precondition, prompt, entropy, and document-validation failures
  become durable before their CLI errors, while success stays one atomic
  `ItemUpdate` mutation.
- Collapse active-epoch `history restore ITEM REVISION` into one item-bound
  audited application mutation, including durable missing, cross-item,
  tombstone, same-revision, and conflict failures.
- Collapse active-epoch `item delete ITEM` into one application-selected
  audited mutation: successful tombstones and failed authenticated
  preconditions now become durable before the CLI reveals their outcome.
- Route list, show, history list, audit verify, and unlocked doctor through
  signed publish-before-render access events whenever the vault audit epoch is
  active, while retaining backward-compatible pre-audit behavior.
- Added reversible authenticated `item delete ITEM` and
  `history restore ITEM REVISION` mutations with strict item-bound selectors,
  causal tombstones, and restore-as-new-revision semantics.
- Added authenticated `history list ITEM` with canonical revision selectors,
  newest-first causal metadata, and redacted record titles.
- Added revision-safe `item edit ITEM` for complete login-field replacement
  while preserving identity, metadata, notes, and causal history.
- Added strict `item add login`, `item list`, and `item show ITEM` commands.
- Added controlling-terminal item input, fresh mutation identities, durable
  application publication, escaped redacted rendering, and restart coverage.
- Added one-shot authenticated `audit verify` with aggregate-only output.
- Extended that output with a secret-free count of fully authenticated
  encrypted operation-audit events; pre-audit vaults report zero.
- Added opt-in full repository health verification through `doctor --unlock`.
- Added strict parser, wrong-passphrase, synchronous re-lock, and real-process
  controlling-terminal coverage for authenticated verification.

## 0.1.0

- Added the closed `init`, `status`, and `doctor` command grammar.
- Added stable exit classes and payload-free text/JSON rendering.
- Composed secure local roots, exact configuration, durable application state,
  immutable filesystem storage, fixed terminal prompts, and OS entropy.
- Added crash-resumable generation-zero activation and restart tests.
