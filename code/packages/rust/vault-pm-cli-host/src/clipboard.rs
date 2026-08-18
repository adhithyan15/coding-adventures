//! Platform clipboard delivery with a verified, timed clear (VLT-PM46).
//!
//! # What this module is for
//!
//! `VLT-PM00 §14.6` calls `--copy` the *preferred* way to get a secret out of
//! this product, and `VLT-PM07` has carried a `clipboard_clear_seconds`
//! configuration value — with a validator, a default, and a round-trip test —
//! since the config slice landed. Until this module existed, that value had no
//! writer and both `--copy` flags were parsed and then refused.
//!
//! # The shape of the problem, in three sentences
//!
//! 1. The clipboard is not reachable from portable Rust, so something outside
//!    this process has to touch it.
//! 2. `vault-pm` is a one-shot process, so a clear that must happen thirty
//!    seconds from now has nothing to happen *in*.
//! 3. The clipboard is a shared bus, so whatever performs that clear must not
//!    wipe the paragraph the person copied afterwards.
//!
//! This module answers those three with, respectively: a pre-installed platform
//! utility fed on **standard input** (never argv); a detached re-execution of
//! this same binary; and a **verified** clear that compares a commitment before
//! it wipes anything.
//!
//! # The one rule everything else follows from
//!
//! **The secret goes on a pipe, never in an argument.** On a shared host any
//! user can read another process's command line out of `ps` or
//! `/proc/<pid>/cmdline`. A six-digit TOTP code passed as an argument — or a
//! commitment to one, which is brute-forceable in microseconds — would be
//! handed to every other account on the machine. Every spawn in this file
//! therefore has a fixed, secret-free argument vector, and everything sensitive
//! travels on a pipe that only the parent and the child hold.

use super::CliHostError;
use coding_adventures_ct_compare::ct_eq_fixed;
use coding_adventures_sha256::sha256;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Debug, Formatter};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// Exact wire length of one [`ClipboardClearRequest`] parameter block.
pub const CLIPBOARD_CLEAR_REQUEST_BYTES: usize = 4 + 1 + 4 + 32 + 32;

/// Largest value this adapter will put on the clipboard.
///
/// Deliberately equal to the terminal secret bound: a value that could not have
/// been typed at a prompt is not a value this product produced.
pub const MAX_CLIPBOARD_VALUE_BYTES: usize = super::MAX_SECRET_BYTES;

/// Fixed leading bytes of a parameter block.
const REQUEST_MAGIC: [u8; 4] = *b"VPMC";

/// Wire version of the parameter block.
const REQUEST_VERSION: u8 = 1;

/// Inclusive bounds on the clear delay, restated from VLT-PM07 §4.
///
/// The configuration layer already validates this range. It is checked again
/// here because the value arrives over a process boundary, and a boundary that
/// trusts its input is not a boundary.
const MIN_CLEAR_SECONDS: u32 = 1;
/// Upper bound on the clear delay; see [`MIN_CLEAR_SECONDS`].
const MAX_CLEAR_SECONDS: u32 = 60 * 60;

/// How long a clipboard utility may take before it is killed.
///
/// Every wait in this module is bounded. A wedged `xclip` must not be able to
/// hold a person's terminal, and an unbounded wait is exactly how that happens.
const TOOL_WAIT: Duration = Duration::from_secs(5);

/// Polling interval for the bounded wait.
const TOOL_POLL: Duration = Duration::from_millis(10);

/// Ceiling on a clipboard read.
///
/// Four times the largest value this product ever copies. A larger clipboard
/// cannot be ours, so refusing to read it costs nothing and removes both an
/// unbounded allocation and a pipe deadlock.
const MAX_CLIPBOARD_READ_BYTES: usize = 4 * 1024;

/// Extra seconds of grace before the detached clearer's watchdog fires.
const CLEARER_WATCHDOG_GRACE_SECONDS: u32 = 30;

/// Directories a clipboard utility may be executed from.
///
/// `PATH` is never consulted. See VLT-PM46 §4.2: `PATH` is caller-controlled,
/// so resolving through it would let anyone who can prepend a directory receive
/// a live credential on the standard input of a program of their choosing.
/// `/usr/local/bin` is excluded on purpose — it is the conventional home of
/// locally-installed software and is group- or user-writable on a meaningful
/// fraction of real machines.
pub const TRUSTED_TOOL_DIRECTORIES: [&str; 2] = ["/usr/bin", "/bin"];

/// Which family of clipboard utilities this host uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardTooling {
    /// macOS `pbcopy` / `pbpaste`.
    MacOs,
    /// Wayland `wl-copy` / `wl-paste`.
    Wayland,
    /// X11 `xclip`.
    X11Clip,
    /// X11 `xsel`.
    X11Sel,
}

/// One spawnable utility: a program name and its complete fixed argument list.
///
/// The argument list is `&'static [&'static str]` rather than anything built at
/// runtime. That type is the invariant: there is no way to interpolate a secret
/// into a value of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolInvocation {
    /// Bare program name, resolved against [`TRUSTED_TOOL_DIRECTORIES`].
    pub program: &'static str,
    /// Complete fixed argument vector. Never contains caller data.
    pub arguments: &'static [&'static str],
}

impl ClipboardTooling {
    /// The invocation that writes the clipboard from standard input.
    pub const fn write(self) -> ToolInvocation {
        match self {
            Self::MacOs => ToolInvocation {
                program: "pbcopy",
                arguments: &[],
            },
            Self::Wayland => ToolInvocation {
                program: "wl-copy",
                arguments: &[],
            },
            Self::X11Clip => ToolInvocation {
                program: "xclip",
                arguments: &["-selection", "clipboard"],
            },
            Self::X11Sel => ToolInvocation {
                program: "xsel",
                arguments: &["--clipboard", "--input"],
            },
        }
    }

    /// The invocation that prints the current clipboard to standard output.
    pub const fn read(self) -> ToolInvocation {
        match self {
            Self::MacOs => ToolInvocation {
                program: "pbpaste",
                arguments: &[],
            },
            Self::Wayland => ToolInvocation {
                program: "wl-paste",
                arguments: &["--no-newline"],
            },
            Self::X11Clip => ToolInvocation {
                program: "xclip",
                arguments: &["-selection", "clipboard", "-o"],
            },
            Self::X11Sel => ToolInvocation {
                program: "xsel",
                arguments: &["--clipboard", "--output"],
            },
        }
    }

    /// The invocation that empties the clipboard.
    ///
    /// Two of the four families have a dedicated flag; the other two are
    /// cleared by writing zero bytes, which is what [`ClearInvocation::Write`]
    /// records.
    pub const fn clear(self) -> ClearInvocation {
        match self {
            Self::MacOs => ClearInvocation::Write(ToolInvocation {
                program: "pbcopy",
                arguments: &[],
            }),
            Self::Wayland => ClearInvocation::Flag(ToolInvocation {
                program: "wl-copy",
                arguments: &["--clear"],
            }),
            Self::X11Clip => ClearInvocation::Write(ToolInvocation {
                program: "xclip",
                arguments: &["-selection", "clipboard"],
            }),
            Self::X11Sel => ClearInvocation::Flag(ToolInvocation {
                program: "xsel",
                arguments: &["--clipboard", "--delete"],
            }),
        }
    }

    /// Every program name this tooling can execute.
    ///
    /// Used by selection: a family is only chosen when *all* of its programs
    /// are present, so a host with `wl-copy` but no `wl-paste` falls through to
    /// the next family instead of copying a value it could never verify.
    const fn programs(self) -> &'static [&'static str] {
        match self {
            Self::MacOs => &["pbcopy", "pbpaste"],
            Self::Wayland => &["wl-copy", "wl-paste"],
            Self::X11Clip => &["xclip"],
            Self::X11Sel => &["xsel"],
        }
    }
}

/// How a family empties the clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearInvocation {
    /// Run the utility with a dedicated clearing flag and no input.
    Flag(ToolInvocation),
    /// Run the writer with zero bytes of input.
    Write(ToolInvocation),
}

impl ClearInvocation {
    /// The underlying invocation, whichever arm this is.
    pub const fn invocation(self) -> ToolInvocation {
        match self {
            Self::Flag(invocation) | Self::Write(invocation) => invocation,
        }
    }
}

/// What kind of desktop session this process is running in.
///
/// Split out as plain data so selection is a pure function that tests can drive
/// through every row of VLT-PM46 §4.1's table without a display server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionKind {
    /// The target is macOS, whose pasteboard needs no display variables.
    pub macos: bool,
    /// `WAYLAND_DISPLAY` is set and non-empty.
    pub wayland: bool,
    /// `DISPLAY` is set and non-empty.
    pub x11: bool,
}

/// Choose the clipboard family for a session, given a program-trust test.
///
/// The order is VLT-PM46 §4.1 and each step of it is a decision:
///
/// | Session | Chosen | Why |
/// |---|---|---|
/// | macOS | `MacOs` | the pasteboard is always present; no display variable exists to consult |
/// | Wayland *and* X11 | `Wayland` | a Wayland session commonly also exports `DISPLAY` for XWayland, and the native selection is the correct one |
/// | X11 with `xclip` | `X11Clip` | `xclip` before `xsel` only because it is the more commonly installed of two equivalent tools |
/// | X11 with `xsel` only | `X11Sel` | |
/// | headless | `None` | fail closed; a headless CI runner must never silently succeed |
///
/// A family whose programs are not all present is skipped rather than chosen,
/// so `wl-copy` without `wl-paste` falls through to X11 instead of producing a
/// copy whose clear could never be verified.
pub(crate) fn select_tooling(
    session: SessionKind,
    trusted: &dyn Fn(&Path) -> bool,
) -> Option<ClipboardTooling> {
    let mut candidates: Vec<ClipboardTooling> = Vec::new();
    if session.macos {
        candidates.push(ClipboardTooling::MacOs);
    }
    if session.wayland {
        candidates.push(ClipboardTooling::Wayland);
    }
    if session.x11 {
        candidates.push(ClipboardTooling::X11Clip);
        candidates.push(ClipboardTooling::X11Sel);
    }
    candidates.into_iter().find(|tooling| {
        tooling
            .programs()
            .iter()
            .all(|program| resolve_program(program, trusted).is_some())
    })
}

/// Resolve a bare program name inside [`TRUSTED_TOOL_DIRECTORIES`], in order.
///
/// This is the whole of the `PATH` policy: there isn't one. A program that is
/// only in `/usr/local/bin`, `/opt`, a Nix profile, or anywhere on `PATH` is not
/// found, and the caller fails closed.
///
/// `trusted` decides whether a candidate path may be executed. Production
/// passes [`is_trusted_program`], which requires a root-owned regular file that
/// is not group- or world-writable — because "the directory is root-owned" is a
/// claim about a host's layout, and a claim this module can check for itself is
/// better than a claim it merely assumes. Tests pass a set-membership closure,
/// which is why the predicate is injected rather than called directly.
pub(crate) fn resolve_program(program: &str, trusted: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    TRUSTED_TOOL_DIRECTORIES
        .iter()
        .map(|directory| Path::new(directory).join(program))
        .find(|candidate| trusted(candidate))
}

/// Whether a candidate path is a program this adapter may pipe a secret into.
///
/// Three conditions, each closing a way the trusted-directory rule could be
/// true in name only:
///
/// | Condition | What it stops |
/// |---|---|
/// | `symlink_metadata`, and the result must be a regular file | a symbolic link in `/usr/bin` pointing at a user-writable path, which `Path::exists` would have followed silently |
/// | owner is `uid` 0 | an image or container where `/usr/bin` is not in fact root-owned |
/// | no group- or other-write bit | a root-owned binary that a group member may still replace |
///
/// A narrow time-of-check/time-of-use window remains between this test and the
/// `execve`. Winning it requires the ability to replace a file inside a
/// root-owned, non-world-writable directory, which is already root — so closing
/// it with `fexecve` would buy nothing this check has not already bought.
#[cfg(unix)]
pub(crate) fn is_trusted_program(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    metadata.is_file() && metadata.uid() == 0 && metadata.mode() & 0o022 == 0
}

/// No target outside Unix has an audited clipboard, so nothing is trusted.
#[cfg(not(unix))]
pub(crate) fn is_trusted_program(_path: &Path) -> bool {
    false
}

/// Read this process's real session kind from the environment.
fn native_session() -> SessionKind {
    SessionKind {
        macos: cfg!(target_os = "macos"),
        wayland: !cfg!(target_os = "macos") && non_empty_env("WAYLAND_DISPLAY"),
        x11: !cfg!(target_os = "macos") && non_empty_env("DISPLAY"),
    }
}

fn non_empty_env(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

/// Anything that can hold one clipboard value.
///
/// The real implementation spawns platform utilities; tests substitute an
/// in-memory double. This is the same real-versus-test split `CliHost` uses one
/// layer up, and it is what makes the interesting logic — "clear only if the
/// value is still ours" — testable on a headless runner.
pub trait ClipboardBackend {
    /// Replace the clipboard with `value`.
    fn write(&self, value: &str) -> Result<(), CliHostError>;

    /// Return the current clipboard contents.
    fn read(&self) -> Result<Zeroizing<Vec<u8>>, CliHostError>;

    /// Empty the clipboard.
    fn clear(&self) -> Result<(), CliHostError>;
}

/// The real clipboard, reached through pre-installed platform utilities.
#[derive(Clone, Debug)]
pub struct PlatformClipboard {
    tooling: ClipboardTooling,
    write: PathBuf,
    read: PathBuf,
    clear: PathBuf,
}

impl PlatformClipboard {
    /// Detect a usable clipboard, or fail closed.
    ///
    /// Detection spawns nothing and reads no clipboard: it inspects two
    /// environment variables and probes for files. That is why callers can run
    /// it *before* prompting a person for their master passphrase (VLT-PM46
    /// §3.2) without spending anything.
    pub fn detect() -> Result<Self, CliHostError> {
        Self::detect_for(native_session(), &is_trusted_program)
    }

    /// Detection against an injected session and program-trust test.
    ///
    /// Crate-private, and that is a security boundary rather than tidiness: a
    /// caller who could pass `&|_| true` would skip the ownership, mode, and
    /// symlink checks. The directory allowlist inside [`resolve_program`] would
    /// still contain the damage, but there is no reason to leave a second lock
    /// on the same door openable from outside.
    pub(crate) fn detect_for(
        session: SessionKind,
        trusted: &dyn Fn(&Path) -> bool,
    ) -> Result<Self, CliHostError> {
        let tooling = select_tooling(session, trusted).ok_or(CliHostError::ClipboardUnavailable)?;
        let resolve = |invocation: ToolInvocation| {
            resolve_program(invocation.program, trusted).ok_or(CliHostError::ClipboardUnavailable)
        };
        Ok(Self {
            tooling,
            write: resolve(tooling.write())?,
            read: resolve(tooling.read())?,
            clear: resolve(tooling.clear().invocation())?,
        })
    }

    /// Which family was selected.
    pub const fn tooling(&self) -> ClipboardTooling {
        self.tooling
    }
}

impl ClipboardBackend for PlatformClipboard {
    fn write(&self, value: &str) -> Result<(), CliHostError> {
        validate_clipboard_value(value)?;
        run_tool_with_input(
            &self.write,
            self.tooling.write().arguments,
            value.as_bytes(),
        )
        .map_err(|_| CliHostError::ClipboardWriteFailed)
    }

    fn read(&self) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
        run_tool_capturing(&self.read, self.tooling.read().arguments)
    }

    fn clear(&self) -> Result<(), CliHostError> {
        let clear = self.tooling.clear();
        run_tool_with_input(&self.clear, clear.invocation().arguments, &[])
            .map_err(|_| CliHostError::ClipboardWriteFailed)
    }
}

/// Reject anything this adapter refuses to put on a clipboard.
///
/// The contract is non-empty printable ASCII with no space and no control
/// character, at most [`MAX_CLIPBOARD_VALUE_BYTES`]. Both callers satisfy it by
/// construction — a generated password draws from an ASCII alphabet and a TOTP
/// code is decimal digits.
///
/// The restriction is not timidity about Unicode. It is what lets the
/// round-trip in [`clear_if_unchanged`] be a byte comparison: leading and
/// trailing whitespace, newlines, and multi-byte sequences are exactly the
/// things clipboard utilities disagree about, and a disagreement there turns
/// into "the clear silently never fires", which is the failure this whole
/// module exists to prevent.
pub fn validate_clipboard_value(value: &str) -> Result<(), CliHostError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_CLIPBOARD_VALUE_BYTES
        || !bytes.iter().all(|byte| (0x21..=0x7e).contains(byte))
    {
        return Err(CliHostError::ClipboardValueUnsupported);
    }
    Ok(())
}

/// `SHA-256(salt || value)`: the commitment the detached clearer compares.
///
/// The salt is fresh per copy. It does not make a six-digit code
/// unguessable — nothing can — but it does stop two commitments to the same
/// password from being equal, so a leaked digest is not an oracle for "is this
/// credential still in use".
pub fn clipboard_commitment(salt: &[u8; 32], value: &[u8]) -> [u8; 32] {
    let mut preimage = Zeroizing::new(Vec::with_capacity(salt.len() + value.len()));
    preimage.extend_from_slice(salt);
    preimage.extend_from_slice(value);
    sha256(&preimage)
}

/// Clear the clipboard **only** if it still holds the committed value.
///
/// This is VLT-PM46 §5.1, and it is the difference between a security feature
/// and a data-loss bug. Thirty seconds is long enough to paste a password and
/// then copy a paragraph of your own; an unconditional timed clear eats that
/// paragraph and is impossible to attribute to the password manager that did
/// it.
///
/// | Clipboard at the deadline | Result |
/// |---|---|
/// | exactly the copied value | cleared, `Ok(true)` |
/// | something copied afterwards | untouched, `Ok(false)` |
/// | already empty | untouched, `Ok(false)` |
/// | unreadable | untouched, `Err(..)` |
///
/// One trailing newline is trimmed before hashing because the read tools do not
/// agree on whether they append one.
pub fn clear_if_unchanged(
    backend: &dyn ClipboardBackend,
    salt: &[u8; 32],
    digest: &[u8; 32],
) -> Result<bool, CliHostError> {
    let current = backend.read()?;
    let trimmed = trim_one_trailing_newline(&current);
    if !ct_eq_fixed(&clipboard_commitment(salt, trimmed), digest) {
        return Ok(false);
    }
    backend.clear()?;
    Ok(true)
}

fn trim_one_trailing_newline(value: &[u8]) -> &[u8] {
    match value.split_last() {
        Some((b'\n', head)) => head,
        _ => value,
    }
}

/// The complete parameter block handed to a detached clearer.
///
/// It carries a delay, a salt, and a commitment — and never the copied value.
/// See VLT-PM46 §4.3 for the honest accounting of what an attacker who reads
/// this out of the child's memory gains, and why it is nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClipboardClearRequest {
    delay_seconds: u32,
    salt: [u8; 32],
    digest: [u8; 32],
}

impl Debug for ClipboardClearRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClipboardClearRequest(<redacted>)")
    }
}

impl ClipboardClearRequest {
    /// Build a request, rejecting a delay outside VLT-PM07's own range.
    pub fn new(delay_seconds: u32, salt: [u8; 32], digest: [u8; 32]) -> Result<Self, CliHostError> {
        if !(MIN_CLEAR_SECONDS..=MAX_CLEAR_SECONDS).contains(&delay_seconds) {
            return Err(CliHostError::InvalidClipboardClearRequest);
        }
        Ok(Self {
            delay_seconds,
            salt,
            digest,
        })
    }

    /// The configured delay in seconds.
    pub const fn delay_seconds(&self) -> u32 {
        self.delay_seconds
    }

    /// Encode the fixed-length wire form.
    pub fn to_bytes(&self) -> [u8; CLIPBOARD_CLEAR_REQUEST_BYTES] {
        let mut encoded = [0_u8; CLIPBOARD_CLEAR_REQUEST_BYTES];
        encoded[0..4].copy_from_slice(&REQUEST_MAGIC);
        encoded[4] = REQUEST_VERSION;
        encoded[5..9].copy_from_slice(&self.delay_seconds.to_be_bytes());
        encoded[9..41].copy_from_slice(&self.salt);
        encoded[41..73].copy_from_slice(&self.digest);
        encoded
    }

    /// Decode the fixed-length wire form, rejecting everything else.
    ///
    /// Exact length, exact magic, exact version, in-range delay. There is no
    /// tolerant arm: this block arrives over a process boundary and the only
    /// producer that should ever write one is this same binary.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CliHostError> {
        if bytes.len() != CLIPBOARD_CLEAR_REQUEST_BYTES
            || bytes[0..4] != REQUEST_MAGIC
            || bytes[4] != REQUEST_VERSION
        {
            return Err(CliHostError::InvalidClipboardClearRequest);
        }
        let mut delay = [0_u8; 4];
        delay.copy_from_slice(&bytes[5..9]);
        let mut salt = [0_u8; 32];
        salt.copy_from_slice(&bytes[9..41]);
        let mut digest = [0_u8; 32];
        digest.copy_from_slice(&bytes[41..73]);
        Self::new(u32::from_be_bytes(delay), salt, digest)
    }
}

/// Read exactly one parameter block from this process's standard input.
///
/// VLT-PM08's rule that secret *input* never comes from stdin is untouched: no
/// passphrase, record secret, or copied value is read here. What arrives is the
/// 73-byte block from [`ClipboardClearRequest::to_bytes`], written by the
/// parent into an anonymous pipe — and it arrives that way *because* argv is
/// world-readable through `ps`.
pub fn read_clear_request_from_stdin() -> Result<ClipboardClearRequest, CliHostError> {
    // One byte of headroom, so reading the over-long case cannot reallocate
    // and leave a copy of salt-and-digest behind (see `run_tool_capturing`).
    let mut block = Zeroizing::new(Vec::with_capacity(CLIPBOARD_CLEAR_REQUEST_BYTES + 1));
    std::io::stdin()
        .take(u64::try_from(CLIPBOARD_CLEAR_REQUEST_BYTES).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut block)
        .map_err(|_| CliHostError::InvalidClipboardClearRequest)?;
    ClipboardClearRequest::from_bytes(&block)
}

/// Put one secret on the clipboard and arm its verified clear.
///
/// The order matters and is the contract:
///
/// 1. validate the value, before anything is spawned;
/// 2. detect a clipboard, so a headless host fails without touching anything;
/// 3. draw a fresh salt and commit to the value;
/// 4. write the clipboard;
/// 5. spawn the detached clearer.
///
/// If step 5 fails after step 4 succeeded, the clipboard is **cleared
/// immediately** and the failure is reported. A copy whose clear was never
/// scheduled leaves a secret in the clipboard forever while the person believes
/// a timeout is running, which is worse than a command that plainly failed.
pub fn copy_and_schedule_clear(
    value: &str,
    clear_after_seconds: u32,
    clearer_arguments: &[&str],
) -> Result<(), CliHostError> {
    validate_clipboard_value(value)?;
    let clipboard = PlatformClipboard::detect()?;
    // Wipe-on-drop even in this process. The salt alone is not secret, but the
    // pair (salt, digest) is secret-equivalent for a low-entropy value, and
    // this process has no reason to keep either after the child has them.
    let mut salt = Zeroizing::new([0_u8; 32]);
    super::OsEntropy.fill(salt.as_mut_slice())?;
    let request = ClipboardClearRequest::new(
        clear_after_seconds,
        *salt,
        clipboard_commitment(&salt, value.as_bytes()),
    )?;
    let program =
        std::env::current_exe().map_err(|_| CliHostError::ClipboardClearScheduleFailed)?;
    clipboard.write(value)?;
    if let Err(error) = spawn_detached_clearer(&program, clearer_arguments, &request) {
        // Best effort by necessity: if this also fails there is nothing further
        // this process can do, and the original scheduling failure is the one
        // worth reporting.
        let _ = clipboard.clear();
        return Err(error);
    }
    Ok(())
}

/// Sleep for the requested delay, then perform the verified clear.
///
/// Runs in the detached child. Before anything else it arms a kernel watchdog
/// (`alarm`), so a wedged clipboard utility cannot leave this process resident:
/// it is killed unconditionally at `delay + 30` seconds whatever happens.
pub fn run_scheduled_clear(request: &ClipboardClearRequest) -> Result<(), CliHostError> {
    arm_watchdog(request.delay_seconds + CLEARER_WATCHDOG_GRACE_SECONDS);
    std::thread::sleep(Duration::from_secs(u64::from(request.delay_seconds)));
    let clipboard = PlatformClipboard::detect()?;
    clear_if_unchanged(&clipboard, &request.salt, &request.digest).map(|_| ())
}

#[cfg(unix)]
fn arm_watchdog(seconds: u32) {
    // SAFETY: both calls take integers and return integers. They touch no
    // memory this process owns and cannot fail in a way that matters here.
    //
    // The disposition is reset first, and that is not belt and braces.
    // `execve` resets *handled* signals to default but preserves *ignored*
    // ones, so a `vault-pm` launched from a parent that had `SIGALRM` set to
    // `SIG_IGN` would inherit an ignored alarm and this watchdog would be a
    // silent no-op — removing the only unconditional bound on this process.
    unsafe {
        libc::signal(libc::SIGALRM, libc::SIG_DFL);
        libc::alarm(seconds);
    }
}

#[cfg(not(unix))]
fn arm_watchdog(_seconds: u32) {}

/// Spawn the detached clearer and hand it its parameter block on a pipe.
///
/// `arguments` is the fixed verb this binary answers to; it never contains
/// caller data, and a test asserts that.
///
/// Three things make the child genuinely outlive its parent:
///
/// - **A second `fork`.** The process this call creates immediately forks again
///   and the intermediate exits, so the grandchild is orphaned to `init`.
///   Without it the long-lived `vault-pm shell` (VLT-PM40) would accumulate one
///   zombie per copy, because nothing ever waits for the clearer.
/// - **`setsid`.** The grandchild leaves the terminal's session, so closing the
///   window does not `SIGHUP` the pending clear away.
/// - **`/dev/null` for output.** The clearer can never write to the terminal it
///   was launched from.
pub fn spawn_detached_clearer(
    program: &Path,
    arguments: &[&str],
    request: &ClipboardClearRequest,
) -> Result<(), CliHostError> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(not(unix))]
    {
        // No non-Unix target has an audited clipboard — `PlatformClipboard`
        // refuses to detect one — so this arm is unreachable through the
        // product. It is written as a refusal rather than left to fall through
        // because the fall-through would be worse than a compile error: with no
        // second `fork` below, the direct child *is* the clearer, and the
        // `wait` at the end of this function would block the foreground process
        // for the whole configured timeout, up to an hour, with the secret
        // already on the clipboard.
        let _ = &mut command;
        return Err(CliHostError::ClipboardClearScheduleFailed);
    }
    #[cfg(unix)]
    {
        // SAFETY: the closure runs in the freshly forked child, between `fork`
        // and `execvp`. It is non-capturing, allocates nothing, and calls only
        // `fork`, `setsid`, and `_exit`, all of which POSIX lists as
        // async-signal-safe.
        //
        // The precondition worth stating plainly: **this process must be
        // single-threaded when it spawns.** glibc's `fork` is not strictly
        // async-signal-safe in practice — it runs `pthread_atfork` handlers
        // under an internal lock — so calling it from the post-fork child of a
        // *multithreaded* parent can deadlock on a lock another thread held.
        // `vault-pm` is a single-threaded one-shot CLI, which is why this is
        // sound; a future thread pool in the composition root would not be.
        unsafe {
            command.pre_exec(|| {
                match libc::fork() {
                    -1 => return Err(std::io::Error::last_os_error()),
                    0 => {}
                    _ => libc::_exit(0),
                }
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let mut child = command
        .spawn()
        .map_err(|_| CliHostError::ClipboardClearScheduleFailed)?;
    let block = Zeroizing::new(request.to_bytes());
    let written = child.stdin.take().ok_or(()).and_then(|mut pipe| {
        pipe.write_all(&*block)
            .and_then(|()| pipe.flush())
            .map_err(|_| ())
    });
    // The direct child is the short-lived intermediate; reaping it is
    // immediate and leaves the grandchild orphaned as intended. The wait is
    // still bounded rather than open ended: an intermediate that somehow does
    // not exit must not be able to hold a person's terminal, and every other
    // wait in this module is bounded for the same reason.
    let _ = wait_bounded(&mut child);
    written.map_err(|()| CliHostError::ClipboardClearScheduleFailed)
}

/// Run a utility, feed it `input` on standard input, and wait — bounded.
///
/// Standard output and standard error go to `/dev/null`. That is not tidiness:
/// `xclip`, `xsel`, and `wl-copy` fork and stay resident to serve the selection
/// (VLT-PM46 §7.2), so a captured pipe would never reach end of file and the
/// wait would hang forever.
fn run_tool_with_input(program: &Path, arguments: &[&str], input: &[u8]) -> Result<(), ()> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let written = child.stdin.take().ok_or(()).and_then(|mut pipe| {
        pipe.write_all(input)
            .and_then(|()| pipe.flush())
            .map_err(|_| ())
    });
    let exited_cleanly = wait_bounded(&mut child);
    written?;
    if exited_cleanly {
        Ok(())
    } else {
        Err(())
    }
}

/// Run a utility and capture a bounded amount of its standard output.
fn run_tool_capturing(
    program: &Path,
    arguments: &[&str],
) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| CliHostError::ClipboardReadFailed)?;
    // Pre-sized past the ceiling on purpose: `Zeroize for Vec<u8>` scrubs the
    // capacity it currently owns, so a growth reallocation would abandon a
    // plaintext copy of the clipboard in freed heap. Never growing means never
    // abandoning one.
    let mut captured = Zeroizing::new(Vec::with_capacity(MAX_CLIPBOARD_READ_BYTES + 512));
    if capture_bounded(&mut child, &mut captured).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CliHostError::ClipboardReadFailed);
    }
    let exited_cleanly = wait_bounded(&mut child);
    if !exited_cleanly || captured.len() > MAX_CLIPBOARD_READ_BYTES {
        return Err(CliHostError::ClipboardReadFailed);
    }
    Ok(captured)
}

/// Drain a child's standard output under both a byte ceiling and a deadline.
///
/// A plain `read_to_end` would be bounded in *bytes* and unbounded in *time*,
/// and the difference matters: on X11 and Wayland the clipboard is served on
/// demand by whichever process owns the selection, so a reader can sit forever
/// waiting for an owner that never answers. `wait_bounded` would not help,
/// because it does not run until the read returns. The consequence would be a
/// verified clear that silently never fires — the exact outcome this module
/// exists to prevent — reachable by any process on the same display.
///
/// So the pipe is put in non-blocking mode and polled against the same
/// [`TOOL_WAIT`] deadline everything else in this module uses.
#[cfg(unix)]
fn capture_bounded(child: &mut Child, captured: &mut Vec<u8>) -> Result<(), ()> {
    use std::os::fd::AsRawFd;
    let mut pipe = child.stdout.take().ok_or(())?;
    // SAFETY: `fcntl` is called on a descriptor this process owns for the
    // duration of the borrow, and only reads and sets its status flags.
    let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(());
    }
    // SAFETY: as above.
    if unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(());
    }
    let deadline = Instant::now() + TOOL_WAIT;
    // Wipe-on-drop: this buffer holds clipboard bytes, which may be the very
    // secret this process is about to decide whether to clear.
    let mut chunk = Zeroizing::new(vec![0_u8; 512]);
    loop {
        match pipe.read(chunk.as_mut_slice()) {
            Ok(0) => return Ok(()),
            Ok(read) => {
                captured.extend_from_slice(&chunk[..read]);
                // Past the ceiling the value cannot be ours, so there is
                // nothing to learn from reading further.
                if captured.len() > MAX_CLIPBOARD_READ_BYTES {
                    return Ok(());
                }
                continue;
            }
            // `Interrupted` sits beside `WouldBlock` deliberately. Treating a
            // signal-interrupted read as a hard failure would make the
            // verified clear give up and leave the secret on the clipboard —
            // failing *open* on the one thing this module exists to close.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(()),
        }
        if Instant::now() >= deadline {
            return Err(());
        }
        std::thread::sleep(TOOL_POLL);
    }
}

/// No non-Unix target reaches this: `PlatformClipboard` never detects one.
#[cfg(not(unix))]
fn capture_bounded(child: &mut Child, captured: &mut Vec<u8>) -> Result<(), ()> {
    let pipe = child.stdout.take().ok_or(())?;
    pipe.take(u64::try_from(MAX_CLIPBOARD_READ_BYTES).unwrap_or(u64::MAX) + 1)
        .read_to_end(captured)
        .map(|_| ())
        .map_err(|_| ())
}

/// Wait for a child for at most [`TOOL_WAIT`], then kill it.
///
/// Returns whether the child exited successfully on its own. A killed child is
/// a failure, which is the whole point: an unbounded wait on a clipboard
/// utility is how a password manager ends up holding someone's terminal.
fn wait_bounded(child: &mut Child) -> bool {
    let deadline = Instant::now() + TOOL_WAIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {}
            Err(_) => return false,
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        std::thread::sleep(TOOL_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Test double: a clipboard that is a `Mutex<Option<Vec<u8>>>`.
    struct FakeClipboard {
        value: Mutex<Option<Vec<u8>>>,
        readable: bool,
        clears: Mutex<usize>,
    }

    impl FakeClipboard {
        fn holding(value: &[u8]) -> Self {
            Self {
                value: Mutex::new(Some(value.to_vec())),
                readable: true,
                clears: Mutex::new(0),
            }
        }

        fn unreadable() -> Self {
            Self {
                value: Mutex::new(None),
                readable: false,
                clears: Mutex::new(0),
            }
        }

        fn clears(&self) -> usize {
            *self.clears.lock().unwrap()
        }
    }

    impl ClipboardBackend for FakeClipboard {
        fn write(&self, value: &str) -> Result<(), CliHostError> {
            *self.value.lock().unwrap() = Some(value.as_bytes().to_vec());
            Ok(())
        }

        fn read(&self) -> Result<Zeroizing<Vec<u8>>, CliHostError> {
            if !self.readable {
                return Err(CliHostError::ClipboardReadFailed);
            }
            Ok(Zeroizing::new(
                self.value.lock().unwrap().clone().unwrap_or_default(),
            ))
        }

        fn clear(&self) -> Result<(), CliHostError> {
            *self.clears.lock().unwrap() += 1;
            *self.value.lock().unwrap() = None;
            Ok(())
        }
    }

    fn present(paths: &'static [&'static str]) -> impl Fn(&Path) -> bool {
        move |candidate: &Path| paths.iter().any(|path| Path::new(path) == candidate)
    }

    const ALL_UNIX_TOOLS: &[&str] = &[
        "/usr/bin/pbcopy",
        "/usr/bin/pbpaste",
        "/usr/bin/wl-copy",
        "/usr/bin/wl-paste",
        "/usr/bin/xclip",
        "/usr/bin/xsel",
    ];

    #[test]
    fn tooling_selection_follows_the_specified_table() {
        let everything = |candidate: &Path| {
            ALL_UNIX_TOOLS
                .iter()
                .any(|path| Path::new(path) == candidate)
        };
        let macos = SessionKind {
            macos: true,
            wayland: false,
            x11: false,
        };
        let wayland_and_x11 = SessionKind {
            macos: false,
            wayland: true,
            x11: true,
        };
        let x11 = SessionKind {
            macos: false,
            wayland: false,
            x11: true,
        };
        let headless = SessionKind {
            macos: false,
            wayland: false,
            x11: false,
        };
        assert_eq!(
            select_tooling(macos, &everything),
            Some(ClipboardTooling::MacOs)
        );
        // Wayland wins even though DISPLAY is also exported, which is the
        // ordinary XWayland arrangement.
        assert_eq!(
            select_tooling(wayland_and_x11, &everything),
            Some(ClipboardTooling::Wayland)
        );
        assert_eq!(
            select_tooling(x11, &everything),
            Some(ClipboardTooling::X11Clip)
        );
        assert_eq!(
            select_tooling(x11, &present(&["/usr/bin/xsel"])),
            Some(ClipboardTooling::X11Sel)
        );
        assert_eq!(select_tooling(headless, &everything), None);
        assert_eq!(select_tooling(x11, &|_| false), None);
    }

    #[test]
    fn a_family_missing_one_of_its_programs_is_skipped() {
        // wl-copy without wl-paste cannot verify its own clear, so the session
        // falls through to X11 rather than copying something it could never
        // take back.
        let session = SessionKind {
            macos: false,
            wayland: true,
            x11: true,
        };
        assert_eq!(
            select_tooling(session, &present(&["/usr/bin/wl-copy", "/usr/bin/xclip"])),
            Some(ClipboardTooling::X11Clip)
        );
        // And with no X11 fallback available it fails closed.
        let wayland_only = SessionKind {
            macos: false,
            wayland: true,
            x11: false,
        };
        assert_eq!(
            select_tooling(wayland_only, &present(&["/usr/bin/wl-copy"])),
            None
        );
    }

    #[test]
    fn programs_are_never_resolved_from_path_or_usr_local_bin() {
        let elsewhere = present(&[
            "/usr/local/bin/xclip",
            "/opt/homebrew/bin/xclip",
            "/home/attacker/bin/xclip",
        ]);
        assert_eq!(resolve_program("xclip", &elsewhere), None);
        let session = SessionKind {
            macos: false,
            wayland: false,
            x11: true,
        };
        assert_eq!(select_tooling(session, &elsewhere), None);
        assert_eq!(
            resolve_program("xclip", &present(&["/usr/bin/xclip"])),
            Some(PathBuf::from("/usr/bin/xclip"))
        );
        // /usr/bin is probed before /bin.
        assert_eq!(
            resolve_program("xclip", &present(&["/usr/bin/xclip", "/bin/xclip"])),
            Some(PathBuf::from("/usr/bin/xclip"))
        );
        assert_eq!(
            resolve_program("xclip", &present(&["/bin/xclip"])),
            Some(PathBuf::from("/bin/xclip"))
        );
    }

    #[test]
    fn detection_resolves_every_program_of_the_selected_family() {
        let session = SessionKind {
            macos: true,
            wayland: false,
            x11: false,
        };
        let clipboard = PlatformClipboard::detect_for(
            session,
            &present(&["/usr/bin/pbcopy", "/usr/bin/pbpaste"]),
        )
        .unwrap();
        assert_eq!(clipboard.tooling(), ClipboardTooling::MacOs);
        assert_eq!(
            PlatformClipboard::detect_for(session, &|_| false).unwrap_err(),
            CliHostError::ClipboardUnavailable
        );
    }

    #[test]
    fn no_invocation_in_the_table_can_carry_caller_data() {
        // The gate is structural: every argument vector is a `&'static` slice
        // of `&'static str`, so there is nowhere for a secret to be
        // interpolated. This test states the resulting table so a future edit
        // that reaches for a `String` has to change a visible expectation.
        for tooling in [
            ClipboardTooling::MacOs,
            ClipboardTooling::Wayland,
            ClipboardTooling::X11Clip,
            ClipboardTooling::X11Sel,
        ] {
            for invocation in [
                tooling.write(),
                tooling.read(),
                tooling.clear().invocation(),
            ] {
                assert!(!invocation.program.is_empty());
                // Every argument is drawn from this closed set. Nothing here
                // is derived from a value, a policy, a vault, or a person.
                assert!(invocation.arguments.iter().all(|argument| [
                    "-selection",
                    "clipboard",
                    "-o",
                    "--clipboard",
                    "--input",
                    "--output",
                    "--delete",
                    "--clear",
                    "--no-newline",
                ]
                .contains(argument)));
            }
        }
        assert_eq!(ClipboardTooling::MacOs.write().arguments, &[] as &[&str]);
        assert_eq!(
            ClipboardTooling::X11Clip.read().arguments,
            &["-selection", "clipboard", "-o"]
        );
        assert_eq!(
            ClipboardTooling::Wayland.clear(),
            ClearInvocation::Flag(ToolInvocation {
                program: "wl-copy",
                arguments: &["--clear"],
            })
        );
        assert_eq!(
            ClipboardTooling::MacOs.clear(),
            ClearInvocation::Write(ToolInvocation {
                program: "pbcopy",
                arguments: &[],
            })
        );
    }

    #[test]
    fn value_contract_rejects_everything_that_would_not_round_trip() {
        assert!(validate_clipboard_value("Xk4$mQ2!vB9pLw7z").is_ok());
        assert!(validate_clipboard_value("042311").is_ok());
        for rejected in ["", "has space", "trailing\n", "tab\there", "café", "\u{7f}"] {
            assert_eq!(
                validate_clipboard_value(rejected).unwrap_err(),
                CliHostError::ClipboardValueUnsupported,
                "{rejected:?} must be refused"
            );
        }
        let longest = "a".repeat(MAX_CLIPBOARD_VALUE_BYTES);
        assert!(validate_clipboard_value(&longest).is_ok());
        assert_eq!(
            validate_clipboard_value(&format!("{longest}a")).unwrap_err(),
            CliHostError::ClipboardValueUnsupported
        );
    }

    #[test]
    fn commitment_depends_on_the_salt() {
        let first = clipboard_commitment(&[1_u8; 32], b"042311");
        let second = clipboard_commitment(&[2_u8; 32], b"042311");
        assert_ne!(first, second);
        assert_eq!(first, clipboard_commitment(&[1_u8; 32], b"042311"));
        assert_ne!(first, clipboard_commitment(&[1_u8; 32], b"042312"));
    }

    #[test]
    fn verified_clear_only_fires_on_a_match() {
        let salt = [7_u8; 32];
        let digest = clipboard_commitment(&salt, b"Xk4$mQ2!vB9pLw7z");

        let ours = FakeClipboard::holding(b"Xk4$mQ2!vB9pLw7z");
        assert!(clear_if_unchanged(&ours, &salt, &digest).unwrap());
        assert_eq!(ours.clears(), 1);
        assert!(ours.value.lock().unwrap().is_none());

        // The paragraph the person copied thirty seconds later survives.
        let theirs = FakeClipboard::holding(b"a shopping list");
        assert!(!clear_if_unchanged(&theirs, &salt, &digest).unwrap());
        assert_eq!(theirs.clears(), 0);
        assert_eq!(
            theirs.value.lock().unwrap().clone().unwrap(),
            b"a shopping list".to_vec()
        );

        // An already-empty clipboard is a mismatch, not a match on "".
        let empty = FakeClipboard::holding(b"");
        assert!(!clear_if_unchanged(&empty, &salt, &digest).unwrap());
        assert_eq!(empty.clears(), 0);

        // A tool that appends one newline still matches.
        let newline = FakeClipboard::holding(b"Xk4$mQ2!vB9pLw7z\n");
        assert!(clear_if_unchanged(&newline, &salt, &digest).unwrap());
        assert_eq!(newline.clears(), 1);

        // Two newlines are not ours.
        let two = FakeClipboard::holding(b"Xk4$mQ2!vB9pLw7z\n\n");
        assert!(!clear_if_unchanged(&two, &salt, &digest).unwrap());

        // An unreadable clipboard clears nothing and reports why.
        let unreadable = FakeClipboard::unreadable();
        assert_eq!(
            clear_if_unchanged(&unreadable, &salt, &digest).unwrap_err(),
            CliHostError::ClipboardReadFailed
        );
        assert_eq!(unreadable.clears(), 0);
    }

    #[test]
    fn a_second_copy_does_not_disarm_the_first_clear() {
        // VLT-PM46 §5.3: two pending clears, no coordination between them.
        let first_salt = [1_u8; 32];
        let second_salt = [2_u8; 32];
        let first_digest = clipboard_commitment(&first_salt, b"first");
        let second_digest = clipboard_commitment(&second_salt, b"second");
        let clipboard = FakeClipboard::holding(b"second");
        assert!(!clear_if_unchanged(&clipboard, &first_salt, &first_digest).unwrap());
        assert!(clear_if_unchanged(&clipboard, &second_salt, &second_digest).unwrap());
        assert_eq!(clipboard.clears(), 1);
    }

    #[test]
    fn a_backend_write_puts_the_value_where_a_read_finds_it() {
        let clipboard = FakeClipboard::holding(b"");
        clipboard.write("042311").unwrap();
        assert_eq!(&*clipboard.read().unwrap(), b"042311");
        clipboard.clear().unwrap();
        assert_eq!(&*clipboard.read().unwrap(), b"");
    }

    #[test]
    fn request_round_trips_and_refuses_every_malformed_block() {
        let request = ClipboardClearRequest::new(30, [3_u8; 32], [4_u8; 32]).unwrap();
        assert_eq!(request.delay_seconds(), 30);
        let encoded = request.to_bytes();
        assert_eq!(encoded.len(), CLIPBOARD_CLEAR_REQUEST_BYTES);
        assert_eq!(
            ClipboardClearRequest::from_bytes(&encoded).unwrap(),
            request
        );
        assert_eq!(format!("{request:?}"), "ClipboardClearRequest(<redacted>)");

        // Neither the salt nor the digest appears anywhere but its own field.
        assert_eq!(&encoded[0..4], b"VPMC");
        assert_eq!(encoded[4], 1);
        assert_eq!(&encoded[5..9], &30_u32.to_be_bytes());

        let mut wrong_magic = encoded;
        wrong_magic[0] = b'X';
        let mut wrong_version = encoded;
        wrong_version[4] = 2;
        let mut zero_delay = encoded;
        zero_delay[5..9].copy_from_slice(&0_u32.to_be_bytes());
        let mut over_range = encoded;
        over_range[5..9].copy_from_slice(&3_601_u32.to_be_bytes());
        for malformed in [wrong_magic, wrong_version, zero_delay, over_range] {
            assert_eq!(
                ClipboardClearRequest::from_bytes(&malformed).unwrap_err(),
                CliHostError::InvalidClipboardClearRequest
            );
        }
        assert_eq!(
            ClipboardClearRequest::from_bytes(&[]).unwrap_err(),
            CliHostError::InvalidClipboardClearRequest
        );
        assert_eq!(
            ClipboardClearRequest::from_bytes(&encoded[..72]).unwrap_err(),
            CliHostError::InvalidClipboardClearRequest
        );
        let mut too_long = encoded.to_vec();
        too_long.push(0);
        assert_eq!(
            ClipboardClearRequest::from_bytes(&too_long).unwrap_err(),
            CliHostError::InvalidClipboardClearRequest
        );
        assert_eq!(
            ClipboardClearRequest::new(0, [0_u8; 32], [0_u8; 32]).unwrap_err(),
            CliHostError::InvalidClipboardClearRequest
        );
        assert_eq!(
            ClipboardClearRequest::new(3_601, [0_u8; 32], [0_u8; 32]).unwrap_err(),
            CliHostError::InvalidClipboardClearRequest
        );
        assert!(ClipboardClearRequest::new(3_600, [0_u8; 32], [0_u8; 32]).is_ok());
    }

    /// A shell-safe scratch path.
    ///
    /// Shell-safe matters: these tests hand the path to `/bin/sh -c`, and a
    /// `ThreadId(3)` rendering would put unquoted parentheses in a command.
    #[cfg(unix)]
    fn scratch_path(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        std::env::temp_dir().join(format!(
            "vault-pm-clipboard-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[cfg(unix)]
    #[test]
    fn a_tool_receives_its_input_on_stdin_and_nothing_in_argv() {
        // `/bin/sh -c 'cat > FILE'` stands in for a clipboard writer: the
        // argument vector is fixed and secret-free, and everything the tool
        // ends up holding arrived on its standard input.
        let destination = scratch_path("stdin");
        let _ = std::fs::remove_file(&destination);
        let script = format!("cat > {}", destination.display());
        run_tool_with_input(Path::new("/bin/sh"), &["-c", &script], b"Xk4$mQ2!vB9pLw7z").unwrap();
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"Xk4$mQ2!vB9pLw7z".to_vec()
        );
        assert!(!script.contains("Xk4$mQ2!vB9pLw7z"));
        std::fs::remove_file(&destination).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_or_missing_tool_is_a_failure_not_a_silent_success() {
        assert!(run_tool_with_input(Path::new("/bin/sh"), &["-c", "exit 3"], b"x").is_err());
        assert!(
            run_tool_with_input(Path::new("/nonexistent/vault-pm-clipboard-tool"), &[], b"x")
                .is_err()
        );
        assert!(
            run_tool_capturing(Path::new("/nonexistent/vault-pm-clipboard-tool"), &[]).is_err()
        );
        assert!(matches!(
            run_tool_capturing(Path::new("/bin/sh"), &["-c", "exit 1"]),
            Err(CliHostError::ClipboardReadFailed)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn a_capturing_tool_returns_bounded_output() {
        assert_eq!(
            &*run_tool_capturing(Path::new("/bin/sh"), &["-c", "printf 042311"]).unwrap(),
            b"042311"
        );
        // Output past the ceiling is refused rather than allocated.
        // Written to run identically under dash and bash: no printf format
        // reuse and no expanded argument list.
        let oversize = format!(
            "dd if=/dev/zero bs=1 count={} 2>/dev/null | tr '\\0' 'a'",
            MAX_CLIPBOARD_READ_BYTES + 64
        );
        assert!(matches!(
            run_tool_capturing(Path::new("/bin/sh"), &["-c", &oversize]),
            Err(CliHostError::ClipboardReadFailed)
        ));
    }

    /// The production trust predicate checks the file, not just the directory.
    ///
    /// "It is in `/usr/bin`, and `/usr/bin` is root-owned" is a claim about a
    /// host's layout. This proves the module checks the claim rather than
    /// assuming it — in particular that a symbolic link planted in a trusted
    /// directory, which `Path::exists` would have followed without comment, is
    /// refused.
    #[cfg(unix)]
    #[test]
    fn program_trust_requires_a_root_owned_regular_file() {
        use std::os::unix::fs::symlink;

        // A real root-owned binary in a trusted directory is accepted.
        //
        // The probe is a list rather than one path, and `/bin/sh` in
        // particular is not on it: it is a regular file on macOS but a symlink
        // to `dash` on Debian and Ubuntu, and this predicate refuses symlinks
        // by design. Every candidate that *is* a regular file must be
        // accepted, and at least one must exist, so the test proves the
        // positive case without asserting anything about a given host's
        // layout.
        let regular: Vec<&str> = [
            "/bin/cat",
            "/usr/bin/cat",
            "/bin/ls",
            "/usr/bin/ls",
            "/usr/bin/env",
        ]
        .into_iter()
        .filter(|candidate| {
            std::fs::symlink_metadata(candidate).is_ok_and(|metadata| metadata.is_file())
        })
        .collect();
        assert!(
            !regular.is_empty(),
            "no standard utility resolved to a regular file on this host"
        );
        for candidate in regular {
            assert!(
                is_trusted_program(Path::new(candidate)),
                "{candidate} is a root-owned regular file and must be accepted"
            );
        }

        // A directory is not a program, and an absent path is not one either.
        assert!(!is_trusted_program(Path::new("/usr/bin")));
        assert!(!is_trusted_program(Path::new(
            "/usr/bin/vault-pm-no-such-clipboard-tool"
        )));

        // A file this test owns is not root's, whatever it is called.
        let planted = scratch_path("planted");
        let _ = std::fs::remove_file(&planted);
        std::fs::write(&planted, b"#!/bin/sh\nexfiltrate\n").unwrap();
        assert!(!is_trusted_program(&planted));

        // And a symbolic link is refused even when it points at something that
        // would itself pass, because the link is what an attacker can replace.
        let link = scratch_path("link");
        let _ = std::fs::remove_file(&link);
        symlink("/bin/sh", &link).unwrap();
        assert!(!is_trusted_program(&link));

        std::fs::remove_file(&planted).unwrap();
        std::fs::remove_file(&link).unwrap();
    }

    /// A reader that writes something and then stalls is killed, not awaited.
    ///
    /// This is the case a byte ceiling alone does not cover: on X11 and Wayland
    /// the selection is served by whichever process owns it, so a reader can
    /// wait forever on an owner that never answers. Reading to end of file
    /// would be bounded in bytes and unbounded in time, and the consequence
    /// would be a verified clear that silently never fires.
    #[cfg(unix)]
    #[test]
    fn a_reader_that_stalls_below_the_ceiling_is_killed_on_the_deadline() {
        let started = Instant::now();
        assert!(matches!(
            run_tool_capturing(Path::new("/bin/sh"), &["-c", "printf 042311; sleep 60"]),
            Err(CliHostError::ClipboardReadFailed)
        ));
        assert!(
            started.elapsed() < TOOL_WAIT + Duration::from_secs(10),
            "the read must be bounded in time, not only in bytes"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_wedged_tool_is_killed_rather_than_waited_on_forever() {
        // The bound is five seconds; `sleep 30` proves the wait is not open
        // ended. The child is killed and the call reports failure.
        let started = Instant::now();
        assert!(run_tool_with_input(Path::new("/bin/sh"), &["-c", "sleep 30"], b"x").is_err());
        assert!(started.elapsed() < TOOL_WAIT + Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[test]
    fn the_detached_clearer_gets_its_block_on_stdin_and_outlives_its_parent() {
        let destination = scratch_path("detached");
        let _ = std::fs::remove_file(&destination);
        let script = format!("cat > {}", destination.display());
        let request = ClipboardClearRequest::new(30, [9_u8; 32], [8_u8; 32]).unwrap();
        spawn_detached_clearer(Path::new("/bin/sh"), &["-c", &script], &request).unwrap();
        // Nothing waits for the grandchild, so poll for its output.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(written) = std::fs::read(&destination) {
                if written.len() == CLIPBOARD_CLEAR_REQUEST_BYTES {
                    assert_eq!(written, request.to_bytes().to_vec());
                    break;
                }
            }
            assert!(Instant::now() < deadline, "detached clearer never ran");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::remove_file(&destination).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn scheduling_against_a_missing_program_fails_closed() {
        let request = ClipboardClearRequest::new(30, [0_u8; 32], [0_u8; 32]).unwrap();
        assert_eq!(
            spawn_detached_clearer(
                Path::new("/nonexistent/vault-pm-clipboard-clearer"),
                &["clipboard", "clear"],
                &request
            )
            .unwrap_err(),
            CliHostError::ClipboardClearScheduleFailed
        );
    }

    /// The real clipboard, only when a person opts in.
    ///
    /// This test clobbers whatever is on the clipboard, so it never runs on its
    /// own. `VAULT_PM_CLIPBOARD_E2E=1 cargo test -p
    /// coding_adventures_vault_pm_cli_host` opts in on a machine with a real
    /// session; CI is headless and would skip it anyway.
    #[test]
    fn real_platform_clipboard_round_trip_when_opted_in() {
        if std::env::var_os("VAULT_PM_CLIPBOARD_E2E").is_none() {
            return;
        }
        let clipboard = PlatformClipboard::detect().expect("a real clipboard session");
        clipboard.write("Xk4$mQ2!vB9pLw7z").unwrap();
        let salt = [5_u8; 32];
        let digest = clipboard_commitment(&salt, b"Xk4$mQ2!vB9pLw7z");
        assert!(clear_if_unchanged(&clipboard, &salt, &digest).unwrap());
        assert!(!clear_if_unchanged(&clipboard, &salt, &digest).unwrap());
    }

    #[test]
    fn detection_on_a_headless_host_is_unavailable_rather_than_a_silent_success() {
        // The exact CI situation: a Linux runner with neither display
        // variable set. Nothing is spawned and nothing succeeds quietly.
        let headless = SessionKind {
            macos: false,
            wayland: false,
            x11: false,
        };
        assert_eq!(
            PlatformClipboard::detect_for(headless, &|_| true).unwrap_err(),
            CliHostError::ClipboardUnavailable
        );
    }
}
