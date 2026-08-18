# Changelog

## Unreleased

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
  An observation it cannot make degrades to silence, never to a false claim.
  Standard output and every exit class are unchanged, and the notice is
  attached to a failing command too, because a repair is worth saying even when
  the verb that triggered it went on to report `not found`.
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
