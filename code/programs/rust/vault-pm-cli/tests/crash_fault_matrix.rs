//! VLT-PM41 — crash/fault matrix and local restore drill.
//!
//! # What this file is
//!
//! `local_cli_e2e.rs` proves that the real `vault-pm` executable does the right
//! thing when it is allowed to *finish*. This file proves what happens when it
//! is not.
//!
//! Every test here kills a real operating-system process with `SIGKILL` at a
//! deterministically chosen durable write, and then asks the *next* real
//! process what it can see and what it can repair. Nothing here calls a
//! recovery function directly; the only interface used is the one a person has
//! — an argument vector, a controlling terminal, and a directory tree.
//!
//! # The matrix
//!
//! `coding_adventures_vault_pm_crash_injection` gives every durable write two
//! landing points, "before" and "after", numbered from one within a process.
//! Because each durable write is an atomic `write → fsync → rename`, those
//! landing points are not a *sample* of where a crash can land: they are the
//! complete case analysis. An operation performing `n` durable writes has
//! exactly `2n` distinguishable crash outcomes.
//!
//! Each cell of the matrix must land in one of two acceptable classes:
//!
//! - **clean rollback** — the tree is indistinguishable from before the
//!   operation started; and
//! - **crash-resumable** — the tree carries an exact journal, the read-only
//!   diagnostics say so, and finishing the operation reaches the same end
//!   state the uninterrupted run would have reached.
//!
//! The forbidden class is **torn**: a tree that decodes to something neither
//! the old nor the new state, or that no longer opens at all.
//!
//! # The one cell that is neither
//!
//! `every_publication_landing_point_leaves_an_exact_resumable_journal`
//! documents a real defect found by this drill. A kill inside the shared
//! mutation publication path is *not torn* — the durable journal is exact and
//! `vault-pm-application`'s `recover_pending_publication` would replay it —
//! but **no CLI verb reaches that function**, so the vault stays wedged and
//! every later command reports `invalid command`. See VLT-PM41 section 8.
//! These assertions therefore pin *observed* behavior and are expected to be
//! rewritten by the slice that adds the recovery verb.

#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

const PASSPHRASE: &[u8] = b"crash matrix correct horse battery staple";
const ITEM_PASSWORD: &[u8] = b"crash matrix password stays encrypted";
const UPDATED_ITEM_PASSWORD: &[u8] = b"crash matrix updated password stays encrypted";
const LOGIN_NOTES: &[u8] = b"crash matrix notes stay encrypted 4f1c9a02";
/// The same notes with the line terminator the prompt is waiting for. Kept
/// separate from [`LOGIN_NOTES`] because the on-disk scan searches for the
/// secret itself, not for the keystrokes that entered it.
const LOGIN_NOTES_LINE: &[u8] = b"crash matrix notes stay encrypted 4f1c9a02\n";
const EXPORT_PASSPHRASE: &[u8] = b"crash matrix distinct export passphrase";
const STDIN_INJECTION: &[u8] = b"stdin injected secret\nstdin injected secret\n";

/// Cap on drill worker threads. Each worker owns a whole vault tree and pays a
/// production Argon2id derivation per unlock, so more threads than cores only
/// makes the wall clock worse.
const MAX_WORKERS: usize = 8;

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// One isolated platform home for one `vault-pm` vault.
///
/// The root is canonicalized because the local host refuses to walk symlinked
/// path components, and on macOS the system temporary directory is reached
/// through one.
struct TestHome(PathBuf);

impl TestHome {
    fn new(tag: &str) -> Self {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vault-pm-crash-matrix-{tag}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        let path = fs::canonicalize(path).unwrap();
        for child in ["home", "config", "data", "cache"] {
            fs::create_dir(path.join(child)).unwrap();
        }
        Self(path)
    }

    fn configure(&self, command: &mut Command) {
        command
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("XDG_CACHE_HOME", self.0.join("cache"));
    }

    fn ledger_path(&self) -> PathBuf {
        self.0.join("durable-steps.tsv")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A byte-for-byte copy of a vault tree, used to replay a ceremony from the
/// identical starting state once per landing point.
///
/// Restoring into the *same* absolute path matters: the client configuration
/// records the resolved object root, and the CLI refuses a vault whose
/// configured location is not the prepared one.
struct Snapshot {
    inside: PathBuf,
}

impl Snapshot {
    fn capture(home: &TestHome) -> Self {
        let inside = home.0.with_extension("snapshot");
        let _ = fs::remove_dir_all(&inside);
        copy_tree(home.path(), &inside);
        Self { inside }
    }

    fn restore(&self, home: &TestHome) {
        for child in fs::read_dir(home.path()).unwrap() {
            let child = child.unwrap();
            if child.file_type().unwrap().is_dir() {
                fs::remove_dir_all(child.path()).unwrap();
            } else {
                fs::remove_file(child.path()).unwrap();
            }
        }
        for child in fs::read_dir(&self.inside).unwrap() {
            let child = child.unwrap();
            let target = home.path().join(child.file_name());
            if child.file_type().unwrap().is_dir() {
                copy_tree(&child.path(), &target);
            } else {
                fs::copy(child.path(), target).unwrap();
            }
        }
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.inside);
    }
}

/// Copy a tree preserving permission bits.
///
/// The mode is not decoration here. The local host refuses to open a vault
/// root that is not owner-only, so a snapshot restored with default directory
/// permissions produces an integrity failure that has nothing to do with the
/// crash being drilled.
fn copy_tree(source: &Path, target: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(target).unwrap();
    let mode = fs::metadata(source).unwrap().permissions().mode();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), &destination).unwrap();
        }
    }
    fs::set_permissions(target, fs::Permissions::from_mode(mode)).unwrap();
}

// ---------------------------------------------------------------------------
// Driving one real process
// ---------------------------------------------------------------------------

/// One prompt the driver waits for and the bytes it answers with.
struct Turn<'a> {
    expect: &'a [u8],
    send: &'a [u8],
}

const fn turn<'a>(expect: &'a [u8], send: &'a [u8]) -> Turn<'a> {
    Turn { expect, send }
}

/// What one interrupted or completed `vault-pm` process left behind.
struct Outcome {
    status: ExitStatus,
    transcript: String,
}

impl Outcome {
    fn was_killed(&self) -> bool {
        self.status.signal() == Some(libc::SIGKILL)
    }

    fn assert_killed(&self, at: u64) {
        assert!(
            self.was_killed(),
            "expected SIGKILL at durable step {at}, got {:?}: {}",
            self.status,
            self.transcript
        );
    }

    fn assert_succeeded(&self, context: &str) {
        assert!(
            self.status.success(),
            "{context} failed with {:?}: {}",
            self.status,
            self.transcript
        );
    }

    fn code(&self) -> Option<i32> {
        self.status.code()
    }
}

/// How one process is instrumented.
#[derive(Clone, Copy, Default)]
struct Injection<'a> {
    /// Record the durable-step ledger to this path.
    trace: Option<&'a Path>,
    /// `SIGKILL` the process when this durable step is reached.
    crash_at: Option<u64>,
}

/// Run one real `vault-pm` process on a real pseudo-terminal.
///
/// The driver differs from `local_cli_e2e.rs` in exactly one way, and it is the
/// point of this file: it never panics when the terminal closes early. A killed
/// child *is* the expected result here, so "the prompt I was waiting for never
/// arrived" ends the script instead of failing the test.
fn drive(
    home: &TestHome,
    arguments: &[&str],
    script: &[Turn<'_>],
    injection: Injection<'_>,
) -> Outcome {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    if let Some(trace) = injection.trace {
        command.env("VAULT_PM_CRASH_TRACE", trace);
    }
    if let Some(step) = injection.crash_at {
        command.env("VAULT_PM_CRASH_AT", step.to_string());
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDOUT_FILENO, tiocsctty_request(), 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    drop(command);
    // The same negative control the end-to-end suite uses: nothing typed on
    // process standard input may ever satisfy a prompt.
    let _ = child.stdin.take().unwrap().write_all(STDIN_INJECTION);

    let mut transcript = Vec::new();
    let mut consumed = 0_usize;
    for step in script {
        match read_until_or_eof(&mut master, &mut transcript, consumed, step.expect) {
            Reply::Found => {}
            Reply::Closed => break,
            Reply::TimedOut => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "timed out waiting for {:?} from {arguments:?}: {}",
                    String::from_utf8_lossy(step.expect),
                    String::from_utf8_lossy(&transcript)
                );
            }
        }
        consumed = transcript.len();
        if master.write_all(step.send).is_err() {
            break;
        }
    }
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    Outcome {
        status,
        transcript: String::from_utf8_lossy(&transcript).into_owned(),
    }
}

/// Why the driver stopped waiting for a prompt.
enum Reply {
    /// The expected public text arrived.
    Found,
    /// The terminal closed first, which is the normal end of a killed process.
    Closed,
    /// Nothing arrived for [`READ_TIMEOUT_MS`]. Always a bug in the drill or a
    /// hang in the product, never an expected outcome.
    TimedOut,
}

/// A drill cell pays at most one production Argon2id derivation per prompt, so
/// a generous ceiling still separates "slow machine" from "wedged forever".
const READ_TIMEOUT_MS: libc::c_int = 180_000;

#[cfg(target_vendor = "apple")]
fn tiocsctty_request() -> libc::c_ulong {
    libc::TIOCSCTTY.into()
}

#[cfg(not(target_vendor = "apple"))]
fn tiocsctty_request() -> libc::c_ulong {
    libc::TIOCSCTTY
}

fn open_pty() -> (File, File) {
    let mut master = -1;
    let mut slave = -1;
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(result, 0, "openpty failed: {}", io::Error::last_os_error());
    unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) }
}

/// Read until `pattern` appears at or after `start`, or the terminal closes.
///
/// Scanning from `start` rather than from the beginning is what lets one
/// script answer the same prompt twice, which the portable-export ceremony
/// needs and which the resume-aware passphrase script depends on.
fn read_until_or_eof(
    master: &mut File,
    transcript: &mut Vec<u8>,
    start: usize,
    pattern: &[u8],
) -> Reply {
    use std::os::fd::AsRawFd;
    loop {
        if transcript.len() >= start + pattern.len()
            && transcript[start..]
                .windows(pattern.len())
                .any(|value| value == pattern)
        {
            return Reply::Found;
        }
        let mut poller = libc::pollfd {
            fd: master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one initialized `pollfd` describing a file descriptor this
        // function owns for the duration of the call.
        let ready = unsafe { libc::poll(&mut poller, 1, READ_TIMEOUT_MS) };
        if ready == 0 {
            return Reply::TimedOut;
        }
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            panic!("pseudo-terminal poll failed: {error}");
        }
        // Only `POLLIN` promises a `read` that will not block. A master whose
        // slave has gone away reports hang-up or error instead, and reading it
        // blindly is how a drill turns a dead child into a stuck test.
        if poller.revents & libc::POLLIN == 0 {
            return Reply::Closed;
        }
        // One byte at a time, exactly as the end-to-end suite does. Reading in
        // blocks would let the transcript run past the prompt that just
        // matched, and the next turn — which resumes scanning from the end of
        // this match — would then wait forever for text already consumed.
        let mut byte = [0_u8; 1];
        match master.read(&mut byte) {
            Ok(0) => return Reply::Closed,
            Ok(count) => transcript.extend_from_slice(&byte[..count]),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return Reply::Closed,
            Err(error) => panic!("pseudo-terminal read failed: {error}"),
        }
    }
}

fn drain_pty(master: &mut File, transcript: &mut Vec<u8>) {
    let mut bytes = [0_u8; 4096];
    loop {
        match master.read(&mut bytes) {
            Ok(0) => return,
            Ok(count) => transcript.extend_from_slice(&bytes[..count]),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return,
            Err(error) => panic!("pseudo-terminal drain failed: {error}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Ceremonies, expressed as scripts
// ---------------------------------------------------------------------------

/// The passphrase script `init` needs, whether it is creating or resuming.
///
/// A fresh generation zero asks twice — `New vault passphrase: ` then
/// `Confirm vault passphrase: `. A resume of an interrupted one asks once, for
/// the *existing* passphrase, as `Vault passphrase: `. The drill runs `init`
/// against both shapes, often without knowing in advance which it will get, so
/// both turns match the common suffix and the second one simply finds no
/// prompt left when the vault is resuming.
fn new_vault_script() -> Vec<Turn<'static>> {
    vec![
        turn(
            b"passphrase: ",
            b"crash matrix correct horse battery staple\n",
        ),
        turn(
            b"passphrase: ",
            b"crash matrix correct horse battery staple\n",
        ),
    ]
}

fn unlock_script() -> Vec<Turn<'static>> {
    vec![turn(
        b"Vault passphrase: ",
        b"crash matrix correct horse battery staple\n",
    )]
}

fn login_script(password: &'static [u8], title: &'static [u8]) -> Vec<Turn<'static>> {
    vec![
        turn(
            b"Vault passphrase: ",
            b"crash matrix correct horse battery staple\n",
        ),
        turn(b"Title: ", title),
        turn(b"Username: ", b"ada@example.test\n"),
        turn(b"Password: ", password),
        turn(b"URL count (0-16): ", b"1\n"),
        turn(b"URL: ", b"https://example.test/login\n"),
        turn(b"Notes (optional): ", LOGIN_NOTES_LINE),
    ]
}

fn init(home: &TestHome, injection: Injection<'_>) -> Outcome {
    drive(home, &["init"], &new_vault_script(), injection)
}

fn unlocked(home: &TestHome, arguments: &[&str], injection: Injection<'_>) -> Outcome {
    drive(home, arguments, &unlock_script(), injection)
}

fn plain(home: &TestHome, arguments: &[&str]) -> Outcome {
    drive(home, arguments, &[], Injection::default())
}

// ---------------------------------------------------------------------------
// The durable-step ledger
// ---------------------------------------------------------------------------

/// One recorded landing point.
#[derive(Debug, PartialEq, Eq)]
struct LedgerEntry {
    ordinal: u64,
    phase: String,
    step: String,
}

fn read_ledger(path: &Path) -> Vec<LedgerEntry> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let ordinal = fields.next().unwrap().parse().unwrap();
            let phase = fields.next().unwrap().to_owned();
            let step = fields.next().unwrap().to_owned();
            assert!(fields.next().is_none(), "ledger line has extra fields");
            LedgerEntry {
                ordinal,
                phase,
                step,
            }
        })
        .collect()
}

/// Count the landing points one ceremony offers, by running it once uninjured.
fn count_landing_points(
    home: &TestHome,
    snapshot: Option<&Snapshot>,
    run: impl Fn(&TestHome, Injection<'_>) -> Outcome,
) -> u64 {
    let ledger = home.ledger_path();
    let _ = fs::remove_file(&ledger);
    let outcome = run(
        home,
        Injection {
            trace: Some(&ledger),
            crash_at: None,
        },
    );
    outcome.assert_succeeded("uninjured ceremony");
    let entries = read_ledger(&ledger);
    assert_contiguous(&entries);
    let total = entries.len() as u64;
    assert!(total > 0, "a ceremony with no durable write cannot crash");
    if let Some(snapshot) = snapshot {
        snapshot.restore(home);
    }
    let _ = fs::remove_file(&ledger);
    total
}

fn assert_contiguous(entries: &[LedgerEntry]) {
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(
            entry.ordinal,
            index as u64 + 1,
            "durable step ordinals must be dense and start at one"
        );
        let expected = if index % 2 == 0 { "before" } else { "after" };
        assert_eq!(
            entry.phase, expected,
            "durable writes must alternate before/after"
        );
    }
    for pair in entries.chunks(2) {
        if pair.len() == 2 {
            assert_eq!(
                pair[0].step, pair[1].step,
                "a before-step and its after-step name the same write"
            );
        }
    }
}

/// Run `body(landing_point)` for every point in `1..=total`, sharded across
/// worker threads.
///
/// Each cell owns a whole vault tree and pays production Argon2id, so the
/// sweep is embarrassingly parallel and badly serial.
fn sweep(total: u64, body: impl Fn(u64) + Sync) {
    let workers = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(MAX_WORKERS)
        .min(total as usize)
        .max(1);
    let body = &body;
    std::thread::scope(|scope| {
        for worker in 0..workers {
            scope.spawn(move || {
                let mut point = worker as u64 + 1;
                while point <= total {
                    body(point);
                    point += workers as u64;
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Reading the tree the way a person would
// ---------------------------------------------------------------------------

fn status(home: &TestHome) -> String {
    let outcome = plain(home, &["status"]);
    outcome
        .transcript
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| line.starts_with("Status: "))
        .unwrap_or_else(|| panic!("no status line: {}", outcome.transcript))
        .trim_start_matches("Status: ")
        .to_owned()
}

fn doctor(home: &TestHome) -> (String, Option<i32>) {
    let outcome = plain(home, &["doctor"]);
    let line = outcome
        .transcript
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| line.starts_with("Doctor: "))
        .unwrap_or_else(|| panic!("no doctor line: {}", outcome.transcript))
        .trim_start_matches("Doctor: ")
        .to_owned();
    (line, outcome.code())
}

fn extract_item_id(transcript: &str) -> String {
    let marker = "Item added: ";
    let start = transcript.find(marker).expect("item-add marker") + marker.len();
    transcript[start..]
        .lines()
        .next()
        .expect("item-add ID")
        .trim_end_matches('\r')
        .to_owned()
}

/// Every byte string a crashed process must never have left readable on disk.
const FORBIDDEN: [&[u8]; 6] = [
    PASSPHRASE,
    ITEM_PASSWORD,
    UPDATED_ITEM_PASSWORD,
    LOGIN_NOTES,
    EXPORT_PASSPHRASE,
    b"stdin injected secret",
];

/// Assert the entire tree still contains none of the drill's secret material.
///
/// This runs once per matrix cell, so it walks the tree once and tests every
/// pattern per file rather than once per pattern.
fn assert_tree_excludes_secrets(root: &Path) {
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            assert_tree_excludes_secrets(&path);
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        for forbidden in FORBIDDEN {
            assert!(
                !bytes
                    .windows(forbidden.len())
                    .any(|value| value == forbidden),
                "{} leaked {}",
                path.display(),
                String::from_utf8_lossy(forbidden)
            );
        }
    }
}

/// Build one vault holding one login, and return it plus its item identifier.
fn initialized_vault(tag: &str) -> (TestHome, String) {
    let home = TestHome::new(tag);
    init(&home, Injection::default()).assert_succeeded("init");
    let added = drive(
        &home,
        &["item", "add", "login"],
        &login_script(
            b"crash matrix password stays encrypted\n",
            b"Example account\n",
        ),
        Injection::default(),
    );
    added.assert_succeeded("item add login");
    let item = extract_item_id(&added.transcript);
    (home, item)
}

// ---------------------------------------------------------------------------
// 1. The mechanism itself
// ---------------------------------------------------------------------------

#[test]
fn the_released_binary_shape_is_the_only_thing_the_drill_changes() {
    // A ledger is produced only when the drill asks for one. Without the
    // variables the executable behaves exactly as the end-to-end suite
    // observes it, which is what makes every other assertion in this file a
    // statement about the real product rather than about instrumentation.
    let home = TestHome::new("mechanism");
    let ledger = home.ledger_path();
    init(&home, Injection::default()).assert_succeeded("uninstrumented init");
    assert!(
        !ledger.exists(),
        "an uninstrumented process must not write a ledger"
    );
    assert_eq!(status(&home), "locked");
}

#[test]
fn the_ledger_names_only_ordinals_phases_and_a_closed_step_vocabulary() {
    let home = TestHome::new("vocabulary");
    let ledger = home.ledger_path();
    init(
        &home,
        Injection {
            trace: Some(&ledger),
            crash_at: None,
        },
    )
    .assert_succeeded("instrumented init");

    let entries = read_ledger(&ledger);
    assert_contiguous(&entries);
    let vocabulary: BTreeSet<&str> = entries.iter().map(|entry| entry.step.as_str()).collect();
    for name in &vocabulary {
        assert!(
            matches!(
                *name,
                "storage.initialize"
                    | "storage.put"
                    | "storage.delete"
                    | "storage.lease"
                    | "config.create"
                    | "config.replace"
                    | "export.artifact"
            ),
            "unexpected durable step name {name}"
        );
    }

    // Generation zero must install its retry journal *before* the random
    // locator becomes discoverable through the configuration file. That
    // ordering is the whole reason a crash before configuration leaves only
    // unreachable opaque data.
    let first_state_put = entries
        .iter()
        .position(|entry| entry.step == "storage.put")
        .expect("generation zero writes application state");
    let config_create = entries
        .iter()
        .position(|entry| entry.step == "config.create")
        .expect("generation zero creates a configuration");
    assert!(
        first_state_put < config_create,
        "the prepared-init journal must precede configuration publication"
    );

    // The ledger is metadata about *shape*, never about content.
    let raw = fs::read(&ledger).unwrap();
    for forbidden in [
        PASSPHRASE,
        ITEM_PASSWORD,
        b"stdin injected secret".as_slice(),
    ] {
        assert!(!raw.windows(forbidden.len()).any(|value| value == forbidden));
    }
}

// ---------------------------------------------------------------------------
// 2. Generation zero — every landing point
// ---------------------------------------------------------------------------

#[test]
fn every_generation_zero_landing_point_is_clean_or_resumable() {
    let probe = TestHome::new("gen0-probe");
    let total = count_landing_points(&probe, None, init);
    drop(probe);

    sweep(total, |point| {
        let home = TestHome::new(&format!("gen0-{point}"));
        let crashed = init(
            &home,
            Injection {
                trace: None,
                crash_at: Some(point),
            },
        );
        crashed.assert_killed(point);

        // A crashed generation zero is never unreadable: the two read-only
        // diagnostics agree on which acceptable class this cell fell into.
        let observed = status(&home);
        let (report, code) = doctor(&home);
        match observed.as_str() {
            // Nothing durable, or a durable journal. `status` distinguishes
            // "there is no vault here" from "there is a half-built one";
            // `doctor` collapses both into the one instruction a person can
            // act on. Rerunning `init` is the repair for both: a fresh
            // initialization when nothing was published, an exact journal
            // resume when something was.
            "uninitialized" | "initializing" => {
                assert_eq!(report, "initialization_required", "landing point {point}");
                assert_eq!(code, Some(2), "landing point {point}");
                init(&home, Injection::default()).assert_succeeded(&format!("resume at {point}"));
                assert_eq!(status(&home), "locked", "landing point {point}");
            }
            // The very last landing point: the owner state reached `Active`
            // durably and only the success line on the terminal was lost. The
            // ceremony is over; there is nothing left to resume.
            "locked" => {
                assert_eq!(report, "authentication_required", "landing point {point}");
                assert_eq!(code, Some(3), "landing point {point}");
            }
            other => panic!("landing point {point} left status {other}"),
        }

        // The recovered vault is a *complete* vault, not merely one that
        // decodes. `doctor --unlock` re-derives every identity, re-verifies
        // the bootstrap signature and pins, and runs the whole audit-chain
        // walk, so one authenticated pass covers what `audit verify` would
        // repeat at the cost of another Argon2id derivation.
        let healthy = unlocked(&home, &["doctor", "--unlock"], Injection::default());
        healthy.assert_succeeded(&format!("doctor after resume at {point}"));
        assert!(
            healthy.transcript.contains("Doctor: healthy"),
            "landing point {point}: {}",
            healthy.transcript
        );
        assert_tree_excludes_secrets(home.path());
    });
}

#[test]
fn a_generation_zero_kill_never_leaves_a_readable_secret_or_a_held_lock() {
    let home = TestHome::new("gen0-residue");
    let probe = TestHome::new("gen0-residue-probe");
    let total = count_landing_points(&probe, None, init);
    drop(probe);

    // The last landing point of the ceremony is the most interesting residue
    // case: everything is durable except the final owner-state advance.
    init(
        &home,
        Injection {
            trace: None,
            crash_at: Some(total - 1),
        },
    )
    .assert_killed(total - 1);

    // The writer lock is advisory and process-scoped, so the kernel released
    // it when it removed the process. If it had not, this next command would
    // report a concurrent writer instead of a status.
    assert_eq!(status(&home), "initializing");
    assert_tree_excludes_secrets(home.path());
}

// ---------------------------------------------------------------------------
// 3. The shared publication path — every landing point
// ---------------------------------------------------------------------------

#[test]
fn every_publication_landing_point_leaves_an_exact_resumable_journal() {
    // `audit verify` is the smallest command that drives the shared mutation
    // publication state machine: with an audit epoch present it publishes an
    // audit-only commit before releasing its report. Every item ceremony,
    // every authored merge, delete, restore, export, and import reaches the
    // disk through this same `publish_mutation`, so sweeping it sweeps the
    // write-ahead machinery for all of them.
    let home = TestHome::new("publish");
    init(&home, Injection::default()).assert_succeeded("init");
    let snapshot = Snapshot::capture(&home);
    let total = count_landing_points(&home, Some(&snapshot), |home, injection| {
        unlocked(home, &["audit", "verify"], injection)
    });

    let mut wedged = 0_u64;
    let mut clean = 0_u64;
    for point in 1..=total {
        snapshot.restore(&home);
        let crashed = unlocked(
            &home,
            &["audit", "verify"],
            Injection {
                trace: None,
                crash_at: Some(point),
            },
        );
        crashed.assert_killed(point);

        let observed = status(&home);
        let (report, code) = doctor(&home);
        match observed.as_str() {
            // Clean rollback: the write-ahead record had not landed yet, so
            // the tree is exactly the tree the ceremony started from.
            "locked" => {
                clean += 1;
                assert_eq!(report, "authentication_required", "landing point {point}");
                assert_eq!(code, Some(3), "landing point {point}");
                unlocked(&home, &["audit", "verify"], Injection::default())
                    .assert_succeeded(&format!("retry after clean rollback at {point}"));
            }
            // Crash-resumable: an exact journal is durable and both read-only
            // diagnostics say so in the vocabulary VLT-PM05 defines.
            "recovery_required" => {
                wedged += 1;
                assert_eq!(report, "recovery_required", "landing point {point}");
                assert_eq!(code, Some(5), "landing point {point}");

                // VLT-PM41 section 8. The journal is exact and replayable, but
                // no CLI verb replays it, so every later command fails closed.
                // This is a *defect*, pinned here so the slice that adds a
                // recovery verb must come back and rewrite these three lines.
                let blocked = unlocked(&home, &["item", "list"], Injection::default());
                assert_eq!(blocked.code(), Some(2), "landing point {point}");
                assert!(
                    blocked.transcript.contains("vault-pm: invalid command"),
                    "landing point {point}: {}",
                    blocked.transcript
                );
            }
            other => panic!("landing point {point} left status {other}"),
        }
        // Whatever class the cell fell into, the tree is never torn: it always
        // decodes, and it never holds plaintext.
        assert_tree_excludes_secrets(home.path());
    }

    assert!(clean > 0, "some landing point must roll back cleanly");
    assert!(wedged > 0, "some landing point must leave a journal");
    assert_eq!(clean + wedged, total);
}

// ---------------------------------------------------------------------------
// 4. One worked example per ceremony family
// ---------------------------------------------------------------------------

/// The three landing points every mutating ceremony must survive.
///
/// Sweeping one ceremony exhaustively (test 3) proves the publication state
/// machine. What each *other* ceremony still has to prove is that it reaches
/// that machine and that its own preparation phase — prompts, entropy,
/// encoding — writes nothing durable it could tear. Three points are enough
/// for that: the first landing point of all, the write-ahead installation, and
/// the release.
fn assert_ceremony_survives_its_characteristic_kills(
    tag: &str,
    home: &TestHome,
    snapshot: &Snapshot,
    run: impl Fn(&TestHome, Injection<'_>) -> Outcome,
) {
    let total = count_landing_points(home, Some(snapshot), &run);
    for point in [1, total / 2, total] {
        snapshot.restore(home);
        run(
            home,
            Injection {
                trace: None,
                crash_at: Some(point),
            },
        )
        .assert_killed(point);
        let observed = status(home);
        assert!(
            matches!(observed.as_str(), "locked" | "recovery_required"),
            "{tag} landing point {point} left status {observed}"
        );
        let (report, code) = doctor(home);
        assert!(
            matches!(
                (report.as_str(), code),
                ("authentication_required", Some(3)) | ("recovery_required", Some(5))
            ),
            "{tag} landing point {point} reported {report}/{code:?}"
        );
        assert_tree_excludes_secrets(home.path());
    }
    snapshot.restore(home);
}

#[test]
fn an_interrupted_item_create_is_clean_or_resumable() {
    let home = TestHome::new("create");
    init(&home, Injection::default()).assert_succeeded("init");
    let snapshot = Snapshot::capture(&home);
    assert_ceremony_survives_its_characteristic_kills(
        "item add",
        &home,
        &snapshot,
        |home, injection| {
            drive(
                home,
                &["item", "add", "login"],
                &login_script(
                    b"crash matrix password stays encrypted\n",
                    b"Example account\n",
                ),
                injection,
            )
        },
    );
    // After the last restore the vault is the empty one we snapshotted, so a
    // fresh create still works end to end.
    let added = drive(
        &home,
        &["item", "add", "login"],
        &login_script(
            b"crash matrix password stays encrypted\n",
            b"Example account\n",
        ),
        Injection::default(),
    );
    added.assert_succeeded("item add after drill");
    assert!(added.transcript.contains("Item added: "));
}

#[test]
fn an_interrupted_item_edit_is_clean_or_resumable() {
    let (home, item) = initialized_vault("edit");
    let snapshot = Snapshot::capture(&home);
    assert_ceremony_survives_its_characteristic_kills(
        "item edit",
        &home,
        &snapshot,
        |home, injection| {
            drive(
                home,
                &["item", "edit", &item],
                &login_script(
                    b"crash matrix updated password stays encrypted\n",
                    b"Updated account\n",
                ),
                injection,
            )
        },
    );
    let shown = unlocked(&home, &["item", "show", &item], Injection::default());
    shown.assert_succeeded("item show after drill");
    assert!(
        shown.transcript.contains("Title: \"Example account\""),
        "an interrupted edit must not have taken effect: {}",
        shown.transcript
    );
}

#[test]
fn an_interrupted_item_delete_is_clean_or_resumable() {
    let (home, item) = initialized_vault("delete");
    let snapshot = Snapshot::capture(&home);
    assert_ceremony_survives_its_characteristic_kills(
        "item delete",
        &home,
        &snapshot,
        |home, injection| unlocked(home, &["item", "delete", &item], injection),
    );
    let listed = unlocked(&home, &["item", "list"], Injection::default());
    listed.assert_succeeded("item list after drill");
    assert!(
        listed.transcript.contains(&item),
        "an interrupted delete must not have taken effect: {}",
        listed.transcript
    );
}

#[test]
fn an_interrupted_history_restore_is_clean_or_resumable() {
    let (home, item) = initialized_vault("restore");
    // Give the item a second revision so a restore has something to select.
    drive(
        &home,
        &["item", "edit", &item],
        &login_script(
            b"crash matrix updated password stays encrypted\n",
            b"Updated account\n",
        ),
        Injection::default(),
    )
    .assert_succeeded("item edit");
    let history = unlocked(&home, &["history", "list", &item], Injection::default());
    history.assert_succeeded("history list");
    let revision = history
        .transcript
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| line.contains("\tlive\t") && line.ends_with("\"Example account\""))
        .and_then(|line| line.split('\t').next())
        .expect("a historical live revision")
        .to_owned();

    let snapshot = Snapshot::capture(&home);
    assert_ceremony_survives_its_characteristic_kills(
        "history restore",
        &home,
        &snapshot,
        |home, injection| unlocked(home, &["history", "restore", &item, &revision], injection),
    );
    let shown = unlocked(&home, &["item", "show", &item], Injection::default());
    shown.assert_succeeded("item show after drill");
    assert!(shown.transcript.contains("Title: \"Updated account\""));
}

#[test]
fn an_interrupted_conflict_merge_is_clean_or_resumable() {
    // No unresolved conflict exists in a single-device vault, so this ceremony
    // fails closed before publishing. That is precisely the case worth
    // drilling: a *refused* mutation must still write nothing durable it
    // could tear, and — with auditing on — the durable record of the refusal
    // is itself a publication that can be interrupted.
    let (home, item) = initialized_vault("merge");
    let history = unlocked(&home, &["history", "list", &item], Injection::default());
    history.assert_succeeded("history list");
    let revision = history
        .transcript
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| line.contains("\tlive\t"))
        .and_then(|line| line.split('\t').next())
        .expect("a live revision")
        .to_owned();

    let snapshot = Snapshot::capture(&home);
    let total = count_landing_points_of_failure(&home, &snapshot, |home, injection| {
        unlocked(home, &["conflict", "list", &item], injection)
    });
    for point in [1, total] {
        snapshot.restore(&home);
        unlocked(
            &home,
            &["conflict", "list", &item],
            Injection {
                trace: None,
                crash_at: Some(point),
            },
        )
        .assert_killed(point);
        let observed = status(&home);
        assert!(
            matches!(observed.as_str(), "locked" | "recovery_required"),
            "conflict landing point {point} left status {observed}"
        );
        assert_tree_excludes_secrets(home.path());
    }
    snapshot.restore(&home);

    // The refusal is unchanged after the drill, and it still names the closed
    // failure — exit class 5, "conflict requiring resolution" — rather than
    // leaking the candidate it declined to merge.
    let refused = unlocked(
        &home,
        &["conflict", "merge", "login", &item, &revision],
        Injection::default(),
    );
    assert_eq!(refused.code(), Some(5), "{}", refused.transcript);
    assert!(refused
        .transcript
        .contains("vault-pm: recovery or conflict required"));
    assert_tree_excludes_secrets(home.path());
}

#[test]
fn an_interrupted_portable_export_never_publishes_a_partial_artifact() {
    let (home, _item) = initialized_vault("export");
    let destination = home.path().join("drill-export.vpm");
    let snapshot = Snapshot::capture(&home);
    let run = |home: &TestHome, injection: Injection<'_>| {
        let _ = fs::remove_file(&destination);
        drive(
            home,
            &["export", destination.to_str().unwrap()],
            &[
                turn(
                    b"Vault passphrase: ",
                    b"crash matrix correct horse battery staple\n",
                ),
                turn(
                    b"Export passphrase: ",
                    b"crash matrix distinct export passphrase\n",
                ),
                turn(
                    b"Confirm export passphrase: ",
                    b"crash matrix distinct export passphrase\n",
                ),
            ],
            injection,
        )
    };
    let total = count_landing_points(&home, Some(&snapshot), run);
    let _ = fs::remove_file(&destination);

    for point in 1..=total {
        snapshot.restore(&home);
        let _ = fs::remove_file(&destination);
        run(
            &home,
            Injection {
                trace: None,
                crash_at: Some(point),
            },
        )
        .assert_killed(point);

        // The artifact is written last and exactly once. A kill anywhere
        // before its "after" step must leave no artifact at all rather than a
        // truncated one a person might mistake for a backup.
        if point < total {
            assert!(
                !destination.exists(),
                "landing point {point} published an artifact before its write completed"
            );
        }
        assert_tree_excludes_secrets(home.path());
    }
    snapshot.restore(&home);
    let _ = fs::remove_file(&destination);

    // Export still works after the whole sweep, and its artifact holds no
    // readable secret.
    run(&home, Injection::default()).assert_succeeded("export after drill");
    assert!(destination.exists());
    let artifact = fs::read(&destination).unwrap();
    for forbidden in [ITEM_PASSWORD, LOGIN_NOTES, PASSPHRASE, EXPORT_PASSPHRASE] {
        assert!(!artifact
            .windows(forbidden.len())
            .any(|value| value == forbidden));
    }
}

/// Landing-point count for a ceremony whose *command* fails closed.
///
/// A refused command still performs durable audit writes, so it still has a
/// matrix row; it just cannot be counted with `count_landing_points`, which
/// insists the uninjured run succeed.
fn count_landing_points_of_failure(
    home: &TestHome,
    snapshot: &Snapshot,
    run: impl Fn(&TestHome, Injection<'_>) -> Outcome,
) -> u64 {
    let ledger = home.ledger_path();
    let _ = fs::remove_file(&ledger);
    run(
        home,
        Injection {
            trace: Some(&ledger),
            crash_at: None,
        },
    );
    let entries = read_ledger(&ledger);
    assert_contiguous(&entries);
    let total = entries.len() as u64;
    assert!(total > 0, "a ceremony with no durable write cannot crash");
    snapshot.restore(home);
    let _ = fs::remove_file(&ledger);
    total
}

// ---------------------------------------------------------------------------
// 5. The local restore drill
// ---------------------------------------------------------------------------

#[test]
fn the_read_only_diagnostics_describe_every_stage_of_an_interrupted_vault() {
    // A person whose machine died wants three questions answered without
    // typing a passphrase: is there a vault here, is it usable, and if not
    // what kind of not-usable is it. `status` and `doctor` answer all three
    // from durable state alone, at every stage.
    let home = TestHome::new("drill");

    // Stage 0: nothing at all.
    assert_eq!(status(&home), "uninitialized");
    assert_eq!(
        doctor(&home),
        ("initialization_required".to_owned(), Some(2))
    );

    // Stage 1: an interrupted generation zero.
    let probe = TestHome::new("drill-probe");
    let total = count_landing_points(&probe, None, init);
    drop(probe);
    init(
        &home,
        Injection {
            trace: None,
            crash_at: Some(total - 1),
        },
    )
    .assert_killed(total - 1);
    assert_eq!(status(&home), "initializing");
    assert_eq!(
        doctor(&home),
        ("initialization_required".to_owned(), Some(2))
    );

    // Stage 2: the resume completes and the vault becomes ordinary.
    init(&home, Injection::default()).assert_succeeded("resume");
    assert_eq!(status(&home), "locked");
    assert_eq!(
        doctor(&home),
        ("authentication_required".to_owned(), Some(3))
    );
    unlocked(&home, &["doctor", "--unlock"], Injection::default())
        .assert_succeeded("authenticated doctor");

    // Stage 3: an interrupted mutation.
    let snapshot = Snapshot::capture(&home);
    let mutation_total = count_landing_points(&home, Some(&snapshot), |home, injection| {
        unlocked(home, &["audit", "verify"], injection)
    });
    unlocked(
        &home,
        &["audit", "verify"],
        Injection {
            trace: None,
            crash_at: Some(mutation_total - 1),
        },
    )
    .assert_killed(mutation_total - 1);
    assert_eq!(status(&home), "recovery_required");
    assert_eq!(doctor(&home), ("recovery_required".to_owned(), Some(5)));

    // Stage 4: restoring the pre-mutation tree makes it ordinary again, which
    // is the property a person's own file-level backup depends on.
    snapshot.restore(&home);
    assert_eq!(status(&home), "locked");
    unlocked(&home, &["audit", "verify"], Injection::default())
        .assert_succeeded("audit verify after restore");
    assert_tree_excludes_secrets(home.path());
}
