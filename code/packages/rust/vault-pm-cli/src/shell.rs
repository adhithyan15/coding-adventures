//! Foreground interactive shell over the existing one-shot command boundary.
//!
//! # What this module is, and what it deliberately is not
//!
//! Every other `vault-pm` command is one operating-system process: it parses an
//! argument vector, collects a passphrase from the controlling terminal,
//! unlocks, performs exactly one operation, and exits. Process exit is what
//! wipes the keys.
//!
//! This module adds a second *host* shape for the same commands — one process
//! that stays in the foreground and reads command lines — without adding a
//! single new capability. It contains no application logic. It cannot reach the
//! repository, the audit chain, or a record. Everything a shell command does,
//! it does by calling [`crate::run`], the very function the one-shot executable
//! calls.
//!
//! ```text
//!   one-shot                              interactive shell
//!   ────────                              ─────────────────
//!   process start                         process start
//!     parse argv                            resolve one bound vault
//!     prompt passphrase                     ┌─ read one command line
//!     unlock                                │    parse the same grammar
//!     ONE operation                         │    prompt passphrase (first time only)
//!     drop session                          │    unlock
//!   process exit  ← wipes everything        │    ONE operation
//!                                           │    drop session
//!                                           └─ loop
//!                                         process exit  ← wipes everything
//! ```
//!
//! # Why the shell retains an authenticator and not a session
//!
//! The obvious design — "unlock once, keep the unlocked session, run many
//! commands against it" — is not available here, and that is on purpose. A
//! [`coding_adventures_vault_pm_application::VaultAccessV1`] session pins the
//! repository heads it observed, and every access and mutation boundary in
//! VLT-PM05 *consumes* the session by value precisely so a stale pin cannot be
//! reused after the repository has moved on. Handing the same session to a
//! second command would reintroduce exactly the class of bug that rule exists
//! to prevent.
//!
//! So the shell retains the smallest thing that removes the repeated prompt:
//! the passphrase, in a wipe-on-drop buffer. Each command still performs its
//! own complete verified open, gets fresh pinned heads, and drops its session
//! synchronously when it finishes. The decrypted-vault exposure window inside a
//! shell is therefore *identical* to the one-shot window; only the
//! authenticator outlives a command.
//!
//! That trade is real and is stated plainly in `VLT-PM40-cli-interactive-shell`
//! §7: an attacker with read access to this process's memory recovers the
//! master passphrase rather than one vault's derived keys. The mitigations are
//! the wipe-on-drop buffer, an explicit `lock`, an idle bound measured when a
//! command is submitted and again when the value is used, and a fail-closed
//! wipe whenever an authentication attempt is rejected.

use crate::{
    configured_vault, decode_config, map_host, map_local_host, CliFailure, CliHost, CliOutput,
    ExitCode, HostError, USAGE,
};
use coding_adventures_vault_pm_cli_host::ControllingTerminal;
use coding_adventures_vault_pm_config::ConfigName;
use coding_adventures_vault_pm_local_host::LocalVaultPaths;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::cell::{Cell, RefCell};
use core::fmt::{self, Debug, Formatter};
use std::io::{self, Write};
use std::path::Path;

/// Maximum tokens accepted on one shell command line.
///
/// The longest command this grammar has is
/// `conflict merge database-credential ITEM BASE_REVISION` at five tokens. The
/// bound is generous enough for that and small enough that a pasted blob is
/// rejected before it reaches the parser.
const MAX_COMMAND_TOKENS: usize = 8;

/// Fixed help text for the shell's own built-in verbs.
///
/// The delegated grammar is unchanged, so the one-shot [`USAGE`] table is
/// printed verbatim beneath it rather than restated and allowed to drift.
const SHELL_BUILTINS: &str = "Shell:\n  lock    forget the retained passphrase; the next command re-authenticates\n  help    show this text\n  exit    end the session (also: quit, end of input)\n\n";

/// Terminal boundary for the foreground shell, injected so the loop is testable.
///
/// Splitting this from [`CliHost`] keeps the one-shot host trait unchanged: a
/// host that never runs a shell implements nothing new.
pub trait ShellTerminal {
    /// Read one command line, or report a clean end of input as `Ok(None)`.
    fn read_command_line(&self) -> Result<Option<Zeroizing<String>>, HostError>;

    /// Render one completed command's ordinary output.
    ///
    /// The rendering is byte-identical to what the one-shot executable writes
    /// for the same command; only the process lifetime differs.
    fn write_output(&self, output: &CliOutput) -> Result<(), HostError>;
}

/// Production shell terminal: commands from `/dev/tty`, output on stdout/stderr.
///
/// The split matters. Command lines and prompts use the controlling terminal,
/// so a redirected or piped standard input can never drive an unlocked session.
/// Ordinary output stays on the process's standard streams, so `vault-pm shell
/// > transcript` still captures results exactly as one-shot invocations do.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeShellTerminal;

impl ShellTerminal for NativeShellTerminal {
    fn read_command_line(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        ControllingTerminal
            .read_command_line()
            .map_err(crate::map_native_cli_host)
    }

    fn write_output(&self, output: &CliOutput) -> Result<(), HostError> {
        let mut stdout = io::stdout();
        let mut stderr = io::stderr();
        stdout
            .write_all(output.stdout().as_bytes())
            .and_then(|()| stdout.flush())
            .and_then(|()| stderr.write_all(output.stderr().as_bytes()))
            .and_then(|()| stderr.flush())
            .map_err(|_| HostError::Unavailable)
    }
}

/// One shell command line after the closed shell-level classification.
#[derive(Debug, PartialEq, Eq)]
enum ShellCommand {
    /// A blank line: reprompt without touching session state.
    Blank,
    /// Forget the retained authenticator.
    Lock,
    /// Print the built-in and delegated grammar.
    Help,
    /// End the session.
    Exit,
    /// Not available inside a session (vault lifecycle or vault reselection).
    Rejected,
    /// Everything else: run the one-shot grammar unchanged.
    Delegated(Vec<String>),
}

/// The retained authenticator and the policy that decides when to drop it.
///
/// `Debug` is closed by hand. There is no accessor that returns the retained
/// bytes to anything but the unlock path, and no path that copies them into a
/// `String`, an error, a log line, or an output buffer.
pub(crate) struct ShellSession {
    retained: RefCell<Option<Zeroizing<Vec<u8>>>>,
    retained_at_ms: Cell<u64>,
    idle_bound_ms: u64,
}

impl ShellSession {
    /// Begin a session with no retained authenticator.
    pub(crate) fn new(idle_bound_ms: u64) -> Self {
        Self {
            retained: RefCell::new(None),
            retained_at_ms: Cell::new(0),
            idle_bound_ms,
        }
    }

    /// Return the authenticator, collecting and retaining it on first use.
    ///
    /// Collection is lazy: `status` and `help` never need one, so a shell that
    /// only ever runs them holds no secret at all.
    ///
    /// The retained value is copied rather than moved out, because the callee
    /// consumes what it is given. The copy handed to the command is wiped when
    /// that command's unlock finishes; the retained original is wiped by
    /// [`Self::lock`] or by this session's drop.
    ///
    /// The idle bound is re-checked *here*, at the point of use, and not only
    /// where the loop checks it. The loop's check is correct as written — it
    /// runs after the blocking read, so it measures the time the session sat
    /// unattended — but a check placed before that read would measure nothing
    /// at all, since the process then waits at the prompt for as long as nobody
    /// types. A value fresh when the prompt appeared can be arbitrarily stale
    /// when somebody finally submits a command, and that unattended terminal is
    /// exactly what this bound defends against. Checking again here means the
    /// gap cannot reopen by rearranging the loop.
    pub(crate) fn authenticator(
        &self,
        host: &dyn CliHost,
    ) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.enforce_idle_bound(host);
        if let Some(retained) = self.retained.borrow().as_ref() {
            return Ok(Zeroizing::new(retained.as_slice().to_vec()));
        }
        let collected = host.read_existing_passphrase()?;
        let copy = Zeroizing::new(collected.as_slice().to_vec());
        // A clock failure must not silently disable the idle bound, so an
        // unreadable clock means the value is used once and never retained.
        if let Ok(now_ms) = host.now_ms() {
            self.retained_at_ms.set(now_ms);
            *self.retained.borrow_mut() = Some(collected);
        }
        Ok(copy)
    }

    /// Drop the retained authenticator now. Repeating this is harmless.
    pub(crate) fn lock(&self) {
        self.retained.borrow_mut().take();
        self.retained_at_ms.set(0);
    }

    /// Drop the authenticator when the configured idle bound has elapsed.
    ///
    /// This is a bound checked when a command is submitted and again when the
    /// authenticator is handed out, not a pre-emptive timer. A shell parked at
    /// its prompt for an hour re-authenticates on the very next command it is
    /// given; it does not re-lock while nobody is typing, and nothing wakes up
    /// to wipe the value in the meantime. The pre-emptive auto-lock a
    /// background timer would provide is Phase 1B work (VLT-PM00 §23 item 12)
    /// and this slice does not pretend to deliver it.
    ///
    /// A clock that cannot be read fails closed: the authenticator is dropped.
    /// So does a clock that has moved *backwards* since collection. The host
    /// clock is advisory wall time, not a monotonic source, so an NTP step or a
    /// manual correction can make "now" earlier than the collection instant. A
    /// saturating subtraction would report zero elapsed time for as long as the
    /// clock stayed behind, silently suspending the bound for exactly as long
    /// as the machine's clock was wrong. Treating an impossible reading as
    /// expiry costs one re-prompt and keeps this control fail-closed for the
    /// same reason an unreadable clock is.
    pub(crate) fn enforce_idle_bound(&self, host: &dyn CliHost) {
        if self.retained.borrow().is_none() {
            return;
        }
        let retained_at_ms = self.retained_at_ms.get();
        match host.now_ms() {
            Ok(now_ms)
                if now_ms >= retained_at_ms && now_ms - retained_at_ms < self.idle_bound_ms => {}
            _ => self.lock(),
        }
    }
}

impl Debug for ShellSession {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(if self.retained.borrow().is_some() {
            "ShellSession(<retained>)"
        } else {
            "ShellSession(<locked>)"
        })
    }
}

/// A [`CliHost`] that answers the unlock prompt from the retained session.
///
/// Every other authority — paths, entropy, clock, item prompts, hidden secret
/// collection, terminal reveal, portable artifact I/O, KDF policy — is
/// delegated unchanged to the real host. Only the one method that would have
/// re-prompted for the vault passphrase behaves differently, which is exactly
/// the difference between one-shot and shell operation.
///
/// In particular the hidden-input path is untouched: `item add login` inside a
/// shell collects its password through the same echo-disabled controlling
/// terminal ceremony it uses one-shot.
pub(crate) struct SessionHost<'session> {
    pub(crate) inner: &'session dyn CliHost,
    pub(crate) session: &'session ShellSession,
}

impl CliHost for SessionHost<'_> {
    fn read_existing_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.session.authenticator(self.inner)
    }

    fn paths(&self) -> Result<LocalVaultPaths, HostError> {
        self.inner.paths()
    }

    fn read_new_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.inner.read_new_passphrase()
    }

    fn read_login_title(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_login_title()
    }

    fn read_login_username(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_login_username()
    }

    fn read_login_url_count(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_login_url_count()
    }

    fn read_login_url(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_login_url()
    }

    fn read_login_password(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_login_password()
    }

    fn read_login_notes(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        self.inner.read_login_notes()
    }

    fn read_secure_note_title(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_secure_note_title()
    }

    fn read_secure_note_body(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_secure_note_body()
    }

    fn read_card_title(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_card_title()
    }

    fn read_card_holder(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_card_holder()
    }

    fn read_card_number(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_card_number()
    }

    fn read_card_expiry_month(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_card_expiry_month()
    }

    fn read_card_expiry_year(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_card_expiry_year()
    }

    fn read_card_cvv(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_card_cvv()
    }

    fn read_card_billing_postal_code(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        self.inner.read_card_billing_postal_code()
    }

    fn read_api_key_label(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_api_key_label()
    }

    fn read_api_key_service(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_api_key_service()
    }

    fn read_api_key_token(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_api_key_token()
    }

    fn read_api_key_scopes(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_api_key_scopes()
    }

    fn read_api_key_expiry(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_api_key_expiry()
    }

    fn read_database_label(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_database_label()
    }

    fn read_database_engine(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_database_engine()
    }

    fn read_database_host(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_database_host()
    }

    fn read_database_port(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_database_port()
    }

    fn read_database_name(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        self.inner.read_database_name()
    }

    fn read_database_username(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_database_username()
    }

    fn read_database_password(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_database_password()
    }

    fn read_totp_label(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_totp_label()
    }

    fn read_totp_issuer(&self) -> Result<Option<Zeroizing<String>>, HostError> {
        self.inner.read_totp_issuer()
    }

    fn read_totp_secret(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_totp_secret()
    }

    fn read_totp_algorithm(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_totp_algorithm()
    }

    fn read_totp_digits(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_totp_digits()
    }

    fn read_totp_period(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_totp_period()
    }

    fn read_opaque_payload(&self) -> Result<Zeroizing<String>, HostError> {
        self.inner.read_opaque_payload()
    }

    fn confirm_secret_reveal(&self) -> Result<bool, HostError> {
        self.inner.confirm_secret_reveal()
    }

    fn confirm_secret_copy(&self) -> Result<bool, HostError> {
        self.inner.confirm_secret_copy()
    }

    fn write_revealed_text(&self, value: &str) -> Result<(), HostError> {
        self.inner.write_revealed_text(value)
    }

    fn ensure_clipboard_available(&self) -> Result<(), HostError> {
        self.inner.ensure_clipboard_available()
    }

    fn copy_revealed_text(&self, value: &str, clear_after_seconds: u32) -> Result<(), HostError> {
        self.inner.copy_revealed_text(value, clear_after_seconds)
    }

    fn run_scheduled_clipboard_clear(&self) -> Result<(), HostError> {
        self.inner.run_scheduled_clipboard_clear()
    }

    fn read_export_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.inner.read_export_passphrase()
    }

    fn write_portable_export(&self, destination: &Path, artifact: &[u8]) -> Result<(), HostError> {
        self.inner.write_portable_export(destination, artifact)
    }

    fn read_portable_export(&self, source: &Path) -> Result<Vec<u8>, HostError> {
        self.inner.read_portable_export(source)
    }

    fn read_import_passphrase(&self) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.inner.read_import_passphrase()
    }

    fn read_external_import_source(&self, source: &Path) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.inner.read_external_import_source(source)
    }

    fn read_attachment_source(&self, source: &Path) -> Result<Zeroizing<Vec<u8>>, HostError> {
        self.inner.read_attachment_source(source)
    }

    fn write_attachment_export(
        &self,
        destination: &Path,
        contents: &[u8],
    ) -> Result<(), HostError> {
        self.inner.write_attachment_export(destination, contents)
    }

    fn confirm_attachment_export(&self) -> Result<bool, HostError> {
        self.inner.confirm_attachment_export()
    }

    fn fill_entropy(&self, output: &mut [u8]) -> Result<(), HostError> {
        self.inner.fill_entropy(output)
    }

    fn now_ms(&self) -> Result<u64, HostError> {
        self.inner.now_ms()
    }

    fn generation_zero_kdf(&self) -> (u32, u32, u8) {
        self.inner.generation_zero_kdf()
    }

    fn portable_export_kdf(&self) -> (u32, u32, u8) {
        self.inner.portable_export_kdf()
    }

    fn portable_open_kdf(&self) -> (u32, u32, u8) {
        self.inner.portable_open_kdf()
    }
}

/// Run one foreground interactive session until it ends.
///
/// The returned [`CliOutput`] is the *process* result, not a command result:
/// per-command output has already been rendered through `terminal` by the time
/// this returns. A session that ends through `exit`, `quit`, or end of input
/// succeeds even if individual commands inside it failed, which is the ordinary
/// behaviour of an interactive program. A command's own exit class reaches the
/// user as the same fixed stderr line the one-shot process would have printed.
pub(crate) fn run_shell(
    host: &dyn CliHost,
    terminal: &dyn ShellTerminal,
    selected_vault: Option<ConfigName>,
) -> Result<CliOutput, CliFailure> {
    let bound = bind_session_vault(host, selected_vault)?;
    let session = ShellSession::new(bound.idle_bound_ms);
    let session_host = SessionHost {
        inner: host,
        session: &session,
    };
    loop {
        // The bound is enforced *after* the blocking read, when the command is
        // actually submitted, so the elapsed time measured is the time the
        // session sat unattended rather than the time before the prompt was
        // printed. `ShellSession::authenticator` checks it again at the point
        // of use, so reordering this loop cannot silently reopen the gap.
        let Some(line) = terminal.read_command_line().map_err(map_host)? else {
            break;
        };
        session.enforce_idle_bound(host);
        let output = match classify(&line) {
            ShellCommand::Blank => continue,
            ShellCommand::Exit => break,
            ShellCommand::Lock => {
                session.lock();
                CliOutput::success("Locked.\n")
            }
            ShellCommand::Help => CliOutput::success(format!("{SHELL_BUILTINS}{USAGE}")),
            ShellCommand::Rejected => CliOutput::failure(CliFailure::InvalidCommand),
            ShellCommand::Delegated(tokens) => dispatch(&session_host, &session, &bound, tokens),
        };
        terminal.write_output(&output).map_err(map_host)?;
    }
    Ok(CliOutput::success(""))
}

/// The one vault a session is bound to, and the policy read from its config.
///
/// The VLT-PM07 codec rejects `auto_lock_seconds = 0`, so `idle_bound_ms` is
/// always at least one second. There is no "zero means never" reading of the
/// configuration for this host to get wrong.
struct BoundVault {
    name: ConfigName,
    idle_bound_ms: u64,
}

/// Resolve and freeze the session's vault before any command runs.
///
/// Binding at start is a security decision, not a convenience. The retained
/// authenticator belongs to one vault; if a later command could name a
/// different vault, the shell would silently present a passphrase collected in
/// one context to a target chosen in another. The name is resolved once —
/// explicitly given, or the configured default as it stood at session start —
/// and every delegated command carries it as an explicit selector.
///
/// The writer lock taken to read configuration is released before the loop
/// begins. A shell must not hold the cross-process writer while it waits at a
/// prompt; each command acquires and releases it exactly as a one-shot process
/// does, so other processes can still work between commands.
fn bind_session_vault(
    host: &dyn CliHost,
    selected_vault: Option<ConfigName>,
) -> Result<BoundVault, CliFailure> {
    let paths = host.paths().map_err(map_host)?;
    let prepared = paths.prepare().map_err(map_local_host)?;
    let writer = prepared.try_acquire_writer().map_err(map_local_host)?;
    let exact_config = writer
        .load_config()
        .map_err(map_local_host)?
        .ok_or(CliFailure::InvalidCommand)?;
    let config = decode_config(&exact_config)?;
    let name = selected_vault.unwrap_or_else(|| config.default_vault().clone());
    let vault = configured_vault(prepared.paths(), &config, Some(&name))?;
    Ok(BoundVault {
        name,
        idle_bound_ms: u64::from(vault.auto_lock_seconds()).saturating_mul(1_000),
    })
}

/// Run one delegated command through the unchanged one-shot entry point.
///
/// A `Locked` class means the retained authenticator was rejected — a wrong
/// passphrase, or a vault whose state moved beneath it. Keeping it would turn
/// one mistyped passphrase into a session that can never succeed again, so the
/// session fails closed and the next command re-authenticates.
///
/// The refusal check is repeated here rather than trusted from [`classify`].
/// [`crate::run`] resolves a shell to the *real* controlling terminal, so a
/// `shell` verb that ever reached this point would open a nested session over
/// the same terminal, inheriting this session's authenticator and recursing
/// without bound. That is unreachable today; the guard makes it unreachable by
/// construction rather than by one classifier arm staying correct.
fn dispatch(
    session_host: &SessionHost<'_>,
    session: &ShellSession,
    bound: &BoundVault,
    tokens: Vec<String>,
) -> CliOutput {
    if tokens.first().is_some_and(|token| is_refused(token)) {
        return CliOutput::failure(CliFailure::InvalidCommand);
    }
    // Nearly every delegated command carries the session's bound vault as an
    // explicit selector, so a command can never be aimed somewhere the person
    // did not authenticate against. `password generate` is the exception, and
    // for the opposite reason: VLT-PM44 §2.2 refuses the selector because the
    // command opens no vault, so prefixing it would turn a usable verb into an
    // invalid one. See `crate::takes_no_vault_selector`.
    let vault_free = tokens
        .first()
        .is_some_and(|token| crate::takes_no_vault_selector(token));
    let mut arguments = Vec::with_capacity(tokens.len() + 2);
    if !vault_free {
        arguments.push("--vault".to_owned());
        arguments.push(bound.name.as_str().to_owned());
    }
    arguments.extend(tokens);
    let output = crate::run(arguments, session_host);
    if output.exit_code() == ExitCode::Locked {
        session.lock();
    }
    output
}

/// Classify one raw command line against the closed shell grammar.
fn classify(line: &str) -> ShellCommand {
    let Ok(tokens) = tokenize(line) else {
        return ShellCommand::Rejected;
    };
    match tokens.first().map(String::as_str) {
        None => ShellCommand::Blank,
        Some("exit" | "quit") if tokens.len() == 1 => ShellCommand::Exit,
        Some("lock") if tokens.len() == 1 => ShellCommand::Lock,
        Some("help" | "--help" | "-h") if tokens.len() == 1 => ShellCommand::Help,
        Some(verb) if is_refused(verb) => ShellCommand::Rejected,
        Some(_) => ShellCommand::Delegated(tokens),
    }
}

/// Verbs a session refuses to delegate, checked by both classification and
/// dispatch.
///
/// `init` and `vault create` build a *different* vault than the one this
/// session is bound to. A leading `--vault` would aim the retained
/// authenticator somewhere the user never authenticated against. A nested
/// `shell` would open a second session over the same terminal with its own
/// retained authenticator.
///
/// `passphrase` is refused for a sharper reason, and VLT-PM43 §3.1 states it: a
/// session's whole premise is that the authenticator it collected once still
/// opens the vault, and a successful rotation is precisely the event that makes
/// that false. Allowing it would leave two bad options — keep using a
/// passphrase that no longer works, turning every later command into an
/// authentication failure the person cannot explain, or silently adopt the new
/// one, which is a retained secret this session never prompted for and cannot
/// re-confirm. A rotation belongs to a one-shot invocation, which collects and
/// discards its own secrets.
///
/// `clipboard` is refused because `clipboard clear` (VLT-PM46 §2.1) is not a
/// command for a person at all: it is the detached process `vault-pm`
/// re-executes itself as, and it takes its parameters from a pipe its parent
/// wrote. A session's standard input is not that pipe — it is the person's
/// terminal, or a redirect — so the verb could only ever read the wrong thing
/// and then fail. `--copy` inside a session works exactly as it does outside
/// one; it spawns its own clearer.
///
/// `agent` (VLT-PM48) is refused wholesale rather than verb by verb. Most of
/// its subcommands would be harmless here — `agent unlock`, for one, would
/// simply reuse the session's already-retained authenticator instead of
/// prompting again — but `agent run-foreground` is the long-lived accept loop
/// `agent start` re-executes this binary as, and running it inline would
/// block the session's own command prompt forever, the same category of
/// mistake a nested `shell` would be. One rule covering the whole noun is
/// easier to state and to keep correct than a rule that allows some of its
/// verbs and not others.
pub(crate) fn is_refused(verb: &str) -> bool {
    matches!(
        verb,
        "init" | "vault" | "shell" | "passphrase" | "clipboard" | "agent" | "--vault"
    )
}

/// Split one command line into the closed shell token grammar.
///
/// The rules are deliberately minimal — this is a selector language, not a
/// programming language:
///
/// | Input | Tokens |
/// |---|---|
/// | `item show ABC` | `item`, `show`, `ABC` |
/// | `search "two words"` | `search`, `two words` |
/// | `item   list` | `item`, `list` |
/// | `search "unterminated` | rejected |
/// | more than eight tokens | rejected |
///
/// Double quotes group one token containing spaces, which the operating-system
/// shell would otherwise have done for a one-shot invocation. There are no
/// escapes, no single quotes, no nesting, and no variable expansion, so no line
/// can mean anything other than what it visibly says.
pub(crate) fn tokenize(line: &str) -> Result<Vec<String>, CliFailure> {
    let mut tokens = Vec::new();
    let mut rest = line;
    loop {
        rest = rest.trim_start_matches([' ', '\t']);
        if rest.is_empty() {
            return Ok(tokens);
        }
        if tokens.len() == MAX_COMMAND_TOKENS {
            zeroize_tokens(&mut tokens);
            return Err(CliFailure::InvalidCommand);
        }
        let (token, remainder) = if let Some(quoted) = rest.strip_prefix('"') {
            match quoted.split_once('"') {
                Some((token, remainder)) => (token, remainder),
                None => {
                    zeroize_tokens(&mut tokens);
                    return Err(CliFailure::InvalidCommand);
                }
            }
        } else {
            match rest.find([' ', '\t']) {
                Some(end) => (&rest[..end], &rest[end..]),
                None => (rest, ""),
            }
        };
        // A closing quote must end the token, so `"a"b` cannot smuggle a second
        // fragment into one selector.
        if !remainder.is_empty() && !remainder.starts_with([' ', '\t']) {
            zeroize_tokens(&mut tokens);
            return Err(CliFailure::InvalidCommand);
        }
        tokens.push(token.to_owned());
        rest = remainder;
    }
}

/// Wipe partially built tokens on a rejected line.
///
/// A rejected line can still hold a search query, and a search query is treated
/// as secret-bearing everywhere else in this crate.
fn zeroize_tokens(tokens: &mut Vec<String>) {
    for token in tokens.iter_mut() {
        token.zeroize();
    }
    tokens.clear();
}
