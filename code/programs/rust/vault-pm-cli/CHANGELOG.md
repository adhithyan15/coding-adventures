# Changelog

## Unreleased

- Fixed the actual cause of PR #12042's "Build and test affected packages"
  step running 2h46m before its 150-minute job timeout fired, with the build
  tool's progress line frozen and zero log output for over two hours:
  `run_attachment_command_in_pty` in `tests/local_cli_e2e.rs` — the helper
  behind the `attachment add` / `attachment list` / `attachment export`
  end-to-end drills this PR adds — was the one PTY-driven `run_*_in_pty`
  helper in this file that never dropped its `Command` after `spawn`. Every
  other helper here does, immediately, and a comment now explains why:
  `Command` keeps its own copy of the `Stdio` handed to `.stderr(...)` — the
  pty slave — alive for as long as the `Command` value lives, and without an
  explicit early drop that is the end of the function, *after* `drain_pty`.
  A pty master only reports EOF once every open reference to its slave is
  closed, so the un-dropped `Command` was a second, silent holder of the
  slave: the real `vault-pm attachment add` process would run to completion
  and exit cleanly, but `drain_pty`'s read loop kept waiting for a hang-up
  the terminal could never produce while the test's own leaked descriptor
  held it open. Reproduced deterministically — in isolation, not from
  parallel-test contention — under a 2-CPU-constrained Docker container
  (matching GitHub Actions' runner shape) both with 13 tests running at
  once and with `the_real_cli_round_trips_a_multi_chunk_attachment_byte_identically`
  run completely alone; fixed by adding the same `drop(command);` every
  sibling helper already carries. This crate's own product sources are
  unchanged — `vault-pm attachment add` itself was never stuck.

- The pseudo-terminal helpers (`read_until`, `read_until_from`, `drain_pty`)
  also now bound every blocking `read` on the PTY master with a 60-second
  `libc::poll` idle timeout, panicking with the pattern that never arrived
  and the transcript captured so far instead of blocking indefinitely. This
  is what turned the leaked-descriptor bug above from a genuine hang into a
  loud, fast, diagnosable failure in the first place — the panic that named
  `drain_pty` and an empty trailing transcript is what led to finding it —
  and it now guards every other `run_*_in_pty` helper against the same class
  of mistake, or any other single unexpected byte from the real child
  process (a race, a PTY-vs-interactive-terminal buffering difference, a
  changed prompt string, or a genuine product deadlock), since they all
  funnel through these three functions.

  Locally (native, unconstrained) the full `local_cli_e2e` suite, including
  the giant `real_cli_initializes_through_a_hidden_tty_and_survives_restart`
  test that drives ~15-20 sequential real CLI invocations through a PTY,
  passes cleanly in ~137s with no timeouts hit. A 2-CPU-constrained Docker
  run — the closer approximation of the CI runner used to find and confirm
  the leaked-descriptor bug above — also passes cleanly, in ~157s.

- The shipped executable now stores, lists, and exports file attachments:
  `attachment add ITEM FILE`, `attachment list ITEM`, and
  `attachment export ITEM ATTACHMENT FILE`. This crate's own sources are
  unchanged; the storage lives in `coding_adventures_vault_pm_application`,
  the file I/O in `coding_adventures_vault_pm_cli_host`, and the grammar in
  `coding_adventures_vault_pm_cli`. Specified by
  `VLT-PM47-cli-attachments.md`.

- A real-process end-to-end test round-trips a payload of two 64 KiB chunks
  plus 1,234 bytes — deliberately not a chunk multiple, so the short final
  chunk is exercised — through a pseudo-terminal and compares the exported
  file byte for byte, checks that neither an interior run of the plaintext nor
  the file's name appears anywhere under the platform roots, and proves a
  refused export writes no file while still leaving a denied row in the audit
  chain.

- The shipped executable can now deliver a secret to the system clipboard:
  `password generate --copy` and `totp code ITEM --copy` are no longer refused.
  This crate's own sources are unchanged; the adapter lives in
  `coding_adventures_vault_pm_cli_host` and the wiring in
  `coding_adventures_vault_pm_cli`, under
  `code/specs/VLT-PM46-cli-clipboard.md`. It is recorded here because it
  changes what this binary does — including that the binary now re-executes
  *itself*, as `vault-pm clipboard clear`, to perform the configured timed
  clear after the original one-shot process has exited.

  The real-process drills now strip `DISPLAY` and `WAYLAND_DISPLAY` from every
  test environment. That makes each `--copy` run deterministically
  clipboard-free on a Linux developer machine and on CI alike, so the
  fail-closed path is what is actually exercised, and — just as important — it
  means the suite never reaches out and overwrites the developer's own
  clipboard with a generated password. macOS reaches its pasteboard through
  `pbcopy`, which no environment variable can take away, so the fail-closed
  assertions are skipped there rather than made to pass by clobbering it. The
  real platform round trip is proved in `vault-pm-cli-host` behind an explicit
  `VAULT_PM_CLIPBOARD_E2E` opt-in, for the same reason.

  New assertions cover the `clipboard clear` verb end to end from the real
  binary: run by hand with nothing on standard input it exits 2, and
  `clipboard`, `clipboard wipe`, `clipboard clear 30`, and a `--vault`-prefixed
  form are all invalid commands.

- The shipped executable can now show a stored TOTP item's current code:
  `vault-pm [--vault NAME] totp code ITEM (--reveal|--copy)`. This crate's own
  sources are unchanged; the ceremony lives in
  `coding_adventures_vault_pm_cli` under `code/specs/VLT-PM45-cli-totp-code.md`,
  and it is recorded here because it changes what this binary does.

  Unlike `password generate`, this command opens a vault, requires the
  passphrase, and publishes an audit event — `VLT-PM15` §2 names TOTP display
  as an access. The code goes only to the controlling terminal; ordinary
  standard output carries one non-secret line, `Code valid for N more seconds`.
  `--copy` is refused with the unsupported class before any prompt, exactly as
  the generator's is.

  A new pseudo-terminal drill covers it end to end. It cannot hard-code the
  expected code, because the real binary reads the real clock, so it brackets
  the run between two of its own clock readings, recomputes the code for every
  second the process could have been in — against the RFC 6238 Appendix B seed,
  so the comparison is with the published algorithm — and requires the
  executable's answer to be one of them. It also proves the two output channels
  never swap, that `--copy` needs no terminal at all, that two runs inside one
  step agree, that a refusal releases nothing, and that the audit chain gains
  one `item_read` row per disclosure while carrying neither the code nor the
  seed.

- The shipped executable can now mint a password: `vault-pm password generate
  [policy flags] (--reveal|--copy)`. This crate's own sources are unchanged;
  the ceremony lives in `coding_adventures_vault_pm_cli` under
  `code/specs/VLT-PM44-cli-password-generate.md`, and it is recorded here
  because it changes what this binary does.

  It is the first verb this executable accepts that opens no vault: it takes no
  `--vault` selector, collects no passphrase, and runs on a home directory
  where `init` has never happened.

  Two real-process tests were added to `local_cli_e2e.rs`, and they check
  things the in-process tests structurally cannot. The first starts from an
  untouched home, drives the confirmation and the delivery over a real
  pseudo-terminal, and asserts that the password arrives on `/dev/tty` while
  captured standard output stays empty, that `config`, `data`, and `cache` are
  all still empty afterwards, and that two runs of the identical command
  produce *different* passwords — which is the only end-to-end evidence that
  the operating-system CSPRNG is genuinely wired in rather than a fixed buffer
  being formatted convincingly. It also drives a narrowed policy and checks the
  alphabet of what comes back. The second proves the refusals: a confirmation
  answered `no` delivers nothing, an under-strength policy fails with the
  entropy-floor message before the terminal is touched at all, disabling every
  character class and passing a `--vault` selector are invalid, and `--copy`
  exits with the unsupported class.

  Both send the same standard-input injection every other reveal test sends, so
  a generated password can neither be influenced by nor leak through an
  attacker-controlled stdin.

- The shipped executable can now change its master passphrase:
  `vault-pm [--vault NAME] passphrase rotate`. This crate's own sources are
  unchanged; the ceremony lives in `coding_adventures_vault_pm_cli` under
  `code/specs/VLT-PM43-cli-passphrase-rotation.md`, and it is recorded here
  because it changes what this binary does.

  Its end-to-end coverage is the part worth naming. `local_cli_e2e.rs` snapshots
  every encrypted object on disk before the rotation and requires each of them
  to still be present and byte-for-byte unchanged afterwards — the direct
  measurement of §14.8's "without re-encrypting every item body" — then proves
  across process restarts that the retired passphrase is refused with exit 3,
  that the new one lists the same item, and that the audit chain carried the
  rotation across and still verifies.

- The shipped executable now survives a crash inside a mutation. This crate's
  own sources are unchanged; the repair is in
  `coding_adventures_vault_pm_cli` under
  `code/specs/VLT-PM42-cli-pending-publication-recovery.md`, and it is recorded
  here because it changes what this binary does. A `vault-pm` process killed
  mid-write used to leave a vault that every later command refused with exit 2
  `vault-pm: invalid command`; the next command that opens the vault now
  replays the exact journal with the passphrase it already collects and reports
  `vault-pm: recovered an interrupted write` on standard error. No verb, flag,
  file format, on-disk artifact, or environment variable was added.
- The crash-injection isolation is unchanged and re-verified. This crate still
  names `crash-injection` in no section, `src/main.rs` still fails to compile
  with it, and `the_shipped_executable_contains_no_crash_injection` still reads
  the produced binary and rejects either injection variable name.

- Added `the_shipped_executable_contains_no_crash_injection` to
  `tests/local_cli_e2e.rs`, and kept this crate free of any mention of
  `coding_adventures_vault_pm_cli`'s `crash-injection` feature. VLT-PM41 needs
  a binary it can kill at a chosen durable write, and the obvious way to get
  one — enabling that feature through this crate's `dev-dependencies` — is a
  trap: Cargo resolves features per package across a build graph, so
  `cargo build --release --all-targets` pulls dev-dependencies in and uplifts
  the instrumented binary to `target/release/vault-pm`, the exact path a
  packaging step copies from. The instrumented twin therefore lives in
  `code/programs/rust/vault-pm-cli-drill` as `vault-pm-drill`, and the new test
  reads the binary this crate produced — in a build that does have
  dev-dependencies resolved — and fails if either injection variable name
  appears anywhere in it.
- Added a `const` assertion in `src/main.rs` on
  `coding_adventures_vault_pm_cli::CRASH_INJECTION_COMPILED`. Naming no feature
  is necessary and *not sufficient*: cargo's `--features <dep>/<feature>`
  syntax reaches a direct dependency's features even when the root package
  declares none of its own, so
  `cargo build --release --features coding_adventures_vault_pm_cli/crash-injection`
  would otherwise still have produced an instrumented
  `target/release/vault-pm`. It is now a compile error, which needs no test to
  have been run.
- **Found by the VLT-PM41 drill, not fixed here:** an interrupted mutation
  leaves a vault this command surface cannot repair. The tree is never torn,
  the durable `PendingPublication` journal is exact, and both read-only
  diagnostics correctly report `recovery_required` — but no verb replays it, so
  every later command fails, and it fails as exit 2
  `vault-pm: invalid command`, telling a person their command is wrong about a
  vault that is intact and one journal replay from healthy. See
  `code/specs/VLT-PM41-cli-crash-fault-matrix.md` section 8 and VLT-PM00 §23
  item 10a.

- Exposed `vault-pm [--vault NAME] shell`, the foreground interactive session
  host, through the unchanged thin executable. Two real-process
  pseudo-terminal drills prove a session unlocks once for several commands,
  runs a hidden item-creation ceremony inside the session, re-authenticates
  after `lock`, refuses vault-lifecycle and reselection verbs without ending,
  ends cleanly on `Ctrl-D`, and leaks no secret to the transcript or the
  profile tree while its standard input remains an injected pipe.

- Exposed audited authored opaque-record conflict merge with a single hidden
  hexadecimal payload prompt, opaque base retention, an inherited content type,
  closed hexadecimal and CBOR-canonicality validation, and all-current-parent
  publication. Every record type this product can hold now has an authored
  merge.
- Exposed audited authored TOTP conflict merge with a hidden Base32 seed
  prompt, opaque base retention, closed seed/algorithm/digit/period validation,
  and all-current-parent publication.
- Exposed audited authored database-credential conflict merge with a hidden
  password prompt, opaque base retention, closed engine/port validation, and
  all-current-parent publication of a static (leaseless) credential.
- Exposed audited authored API-key conflict merge with a hidden token prompt,
  opaque base retention, closed scope/expiry validation, and all-current-parent
  publication.
- Exposed audited authored payment-card conflict merge with hidden PAN/CVV,
  opaque base retention, closed validation, and all-current-parent publication.
- Exposed audited authored secure-note conflict merge with hidden body input,
  opaque base retention, and all-current-parent publication.
- Exposed audited authored login conflict merge with opaque base selection,
  complete hidden form collection, durable failure ordering, and an
  all-current-parent success mutation.
- Exposed audited `conflict reveal ITEM REVISION FIELD` and extended the
  restart drill through terminal-confirmation denial plus an unconflicted
  candidate failure, empty stdout, secret exclusion, and durable audit-chain
  advancement.
- Exposed audited `search QUERY` and extended the primary restart drill through
  a URL metadata match, non-echoed query, redacted result row, audit-chain
  advancement, and closed audit-field verification.
- Extended the real-process login lifecycle through two ordered URLs, hidden
  notes creation and replacement, notes-presence redaction, separate audited
  notes reveal, history restoration, and plaintext-tree exclusion.
- Exposed audited TOTP creation with hidden canonical Base32 input,
  restart-backed metadata-only rendering, separate audited Base32 reveal,
  closed audit rows, and encoded/raw plaintext-tree exclusion in a real PTY
  drill.
- Exposed audited static database-credential creation with restart-backed
  redaction, separate password reveal, closed audit rows, and plaintext-tree
  exclusion in a real PTY drill.
- Exposed audited API-key creation and added a real PTY drill for hidden token
  input, restart-backed metadata-only rendering, separately authorized token
  reveal, closed-field audit advancement, and plaintext-tree exclusion.
- Exposed audited payment-card creation and added a second real PTY drill for
  hidden PAN/CVV input, restart-backed redaction, separate direct-terminal
  reveal, closed-field audit advancement, and full-PAN plaintext-tree
  exclusion.
- Exposed audited interactive current-secret reveal and extended the real PTY
  drill to prove direct controlling-terminal delivery with empty captured
  process stdout and restart-backed audit advancement.
- Exposed audited redacted conflict listing and choose-existing-candidate
  resolution through the thin executable.
- Exposed explicit-target `restore FILE` and moved the real-process PTY drill to
  one artifact authentication, audited import, independent target reopen, and
  completed-and-verified aggregate output with no intermediate import claim.
- Exposed `vault create NAME` and command-scoped `--vault NAME`, and moved the
  real-process restore drill onto a separately keyed named target in the same
  profile with a final restart-backed source-isolation check.
- Exposed retryable audited `restore verify FILE` and extended the real-process
  PTY drill through another independent target reopen and hidden artifact
  prompt before aggregate verification output.
- Exposed audited `import FILE` and extended the real-process PTY suite through
  an independent initialized target, hidden artifact input, and restarted
  redacted observation.
- Exposed audited encrypted `export FILE` and extended the real-process PTY
  suite through two hidden export-passphrase prompts and plaintext-tree checks.
- Exposed secure-note creation and redacted show, with real-process PTY proof
  that the body is hidden during input and absent from the storage tree.
- Extended the real-process PTY audit ceremony through an invalid item-create
  prompt and exact trace selection of its durable failed event.
- Exposed authenticated `audit list` and `audit show TRACE`, and extended the
  real-process PTY suite through trace selection plus later verification that
  both audit-history accesses became durable.
- Exposed `audit enable` and extended the real-process PTY suite through audit
  activation, an invalid edit prompt, and later verification of its event.
- Exposed reversible item delete/restore and extended the real-process PTY
  suite through tombstone observation and exact historical restoration.
- Exposed redacted revision history listing and extended the PTY suite through
  canonical newest-first history after a durable edit.
- Exposed revision-safe login edit and extended the PTY restart suite through
  replacement plus a later redacted show.
- Exposed login add and redacted list/show through the thin executable.
- Extended the PTY suite through encrypted item persistence across processes.
- Exposed authenticated `audit verify` and `doctor --unlock` through the thin
  executable.
- Extended the real-process PTY suite across restart and redirected-stdin
  injection for both authenticated commands.

## 0.1.0

- Added the `vault-pm` executable composition root.
- Added real-process pseudo-terminal initialization and restart coverage.
