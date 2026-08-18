//! Deterministic real-process crash injection for local `vault-pm` hosts.
//!
//! # Why this package exists
//!
//! `vault-pm` claims that a power cut can land anywhere inside a write and
//! still leave a vault a person can open. VLT-PM05 section 7 states the claim
//! precisely: every mutation first writes a journal, then performs external
//! writes, then advances local state, and *any* interruption leaves either the
//! old state or a resumable journal — never a torn one.
//!
//! Until now that claim was checked *in process*. A unit test built the
//! application over an in-memory store, made the store return an error at a
//! chosen call, and then called the recovery function itself. That proves the
//! recovery *logic*. It does not prove that a real operating-system process,
//! killed by a signal it cannot catch, leaves a real directory tree the next
//! real process can open. Those are different claims, and only the second one
//! is what a user experiences when a laptop battery dies.
//!
//! This package closes that gap. It is **test-only scaffolding**, reachable
//! only through `coding_adventures_vault_pm_cli`'s non-default
//! `crash-injection` feature, which only `vault-pm-cli-drill` enables. The
//! product executable `code/programs/rust/vault-pm-cli` names that feature in
//! no manifest section, so no cargo invocation can build a `vault-pm` that
//! contains this code — and its own test suite reads the binary it produced
//! and fails if it ever does.
//!
//! # The model: a durable step is a countable event
//!
//! Everything `vault-pm` makes durable on a local machine passes through one
//! of a small number of gates:
//!
//! ```text
//!   application owner state  ─┐
//!   bootstrap generations    ─┤
//!   immutable repository     ─┼─→ storage_core::StorageBackend  (put/delete/…)
//!   audit chain objects      ─┘
//!
//!   client configuration     ───→ LocalWriterGuard create/compare-exchange
//!   portable export artifact ───→ CliHost::write_portable_export
//! ```
//!
//! Each of those is an *atomic* durable write: `storage-fs` writes to a
//! temporary file, `fsync`s it, `rename`s it into place, and best-effort
//! `fsync`s the parent directory; the configuration writer does the same. So
//! the on-disk state of a vault is completely described by *how many* of those
//! writes have happened. Crashing "somewhere in the middle of a write" is not
//! a distinct outcome — `rename(2)` does not have a middle.
//!
//! That gives us a discrete, finite, totally-ordered set of landing points:
//!
//! ```text
//!   step 1   before durable write #1
//!   step 2   after  durable write #1
//!   step 3   before durable write #2
//!   step 4   after  durable write #2
//!   …
//! ```
//!
//! An operation that performs `n` durable writes has exactly `2n` landing
//! points, and *every* possible crash of that operation is equivalent to
//! landing on one of them. Enumerating them is therefore not a sampling
//! strategy; it is a complete case analysis.
//!
//! # How a test uses it
//!
//! Two environment variables drive one process:
//!
//! | Variable | Meaning |
//! |---|---|
//! | `VAULT_PM_CRASH_TRACE` | append the step ledger to this file |
//! | `VAULT_PM_CRASH_AT` | `SIGKILL` this process when that step is reached |
//!
//! A drill runs the operation once with only `VAULT_PM_CRASH_TRACE` set, counts
//! the ledger lines to learn `2n`, and then replays the operation `2n` times,
//! once per landing point, restoring the same starting tree each time. The
//! matrix is therefore *derived from the code under test*: a new durable write
//! added to a ceremony grows the sweep automatically instead of silently
//! escaping it.
//!
//! # Why `SIGKILL` and not `abort()` or `exit()`
//!
//! `std::process::exit` runs `atexit` handlers and flushes standard output.
//! `std::process::abort` raises `SIGABRT`, which a process *can* install a
//! handler for and which asks macOS to write a crash report. Neither models a
//! power cut. `SIGKILL` cannot be caught, blocked, or handled; the kernel
//! removes the process immediately, closing its file descriptors and dropping
//! its advisory locks without running a single instruction of cleanup. That is
//! exactly the fault we claim to survive, so that is the fault we inject.
//!
//! A test can also *prove* the kill happened, because a killed child reports
//! `ExitStatus::signal() == Some(SIGKILL)` rather than an exit code. A test
//! that expected a crash and got a clean exit fails loudly instead of quietly
//! measuring nothing.
//!
//! # What the ledger may contain
//!
//! Only an ordinal, a phase, and a value from the closed [`DurableStep`]
//! vocabulary. No key, namespace, object identifier, path, item title,
//! ciphertext, or byte count is ever recorded, so no vault *content* can reach
//! the file.
//!
//! Be precise about what that does and does not hide. Each object write emits
//! one `storage.put` pair, so the ledger's *length* correlates with how much a
//! ceremony wrote — it is a shape and activity oracle, not a content oracle.
//! Two vaults running the same ceremony over the same number of objects
//! produce identical ledgers; a larger vault produces a longer one. That is
//! acceptable because the ledger only exists when a drill asks for it by name,
//! and the drill's whole purpose is to count those writes.
//!
//! The file must be an absolute path, is created owner-only, is opened with
//! `O_NOFOLLOW`, and is refused outright if it already exists as something
//! other than a regular file this user owns privately.

#![deny(missing_docs)]

use std::fmt::{self, Debug, Display, Formatter};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use storage_core::{
    Revision, StorageBackend, StorageError, StorageLease, StorageListOptions, StoragePage,
    StoragePutInput, StorageRecord, StorageRecordSummary, StorageStat, StorageSummaryPage,
};

/// Environment variable naming the durable step at which the process must die.
///
/// The value is a decimal step ordinal counting from one. A value that is not
/// a positive decimal integer is a hard error: a typo must not silently turn a
/// crash drill into an ordinary successful run.
pub const CRASH_AT_VARIABLE: &str = "VAULT_PM_CRASH_AT";

/// Environment variable naming the file that receives the durable step ledger.
///
/// The file is appended to, created owner-only, and never read by this crate.
pub const CRASH_TRACE_VARIABLE: &str = "VAULT_PM_CRASH_TRACE";

/// One durable side effect a local `vault-pm` host can perform.
///
/// The vocabulary is closed on purpose. A ledger line is built only from these
/// names, so no caller can smuggle vault content into the trace file by
/// choosing an interesting label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DurableStep {
    /// Backend-owned root creation. Idempotent, but still a real write.
    StorageInitialize,
    /// One record body written through the backend.
    StoragePut,
    /// One record removed through the backend.
    StorageDelete,
    /// One advisory backend lease acquisition attempt.
    StorageLease,
    /// First creation of the client configuration file.
    ConfigCreate,
    /// Compare-and-exchange replacement of the client configuration file.
    ConfigReplace,
    /// Creation of one encrypted portable export artifact.
    ExportArtifact,
}

impl DurableStep {
    /// Return the stable ledger name of this step.
    pub const fn label(self) -> &'static str {
        match self {
            Self::StorageInitialize => "storage.initialize",
            Self::StoragePut => "storage.put",
            Self::StorageDelete => "storage.delete",
            Self::StorageLease => "storage.lease",
            Self::ConfigCreate => "config.create",
            Self::ConfigReplace => "config.replace",
            Self::ExportArtifact => "export.artifact",
        }
    }
}

impl Display for DurableStep {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Which side of a durable write a landing point sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// The write has been fully prepared but nothing has reached the disk.
    Before,
    /// The write has returned; its bytes are durable or its error is known.
    After,
}

impl Phase {
    /// Return the stable ledger name of this phase.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

impl Display for Phase {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// The process-wide crash policy, read once from the environment.
#[derive(Debug, Default)]
struct Policy {
    crash_at: Option<u64>,
    trace: Option<PathBuf>,
}

fn policy() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(|| Policy {
        crash_at: std::env::var(CRASH_AT_VARIABLE).ok().map(|raw| {
            raw.parse::<u64>()
                .ok()
                .filter(|value| *value > 0)
                .unwrap_or_else(|| {
                    panic!("{CRASH_AT_VARIABLE} must be a positive decimal step ordinal")
                })
        }),
        trace: std::env::var_os(CRASH_TRACE_VARIABLE).map(PathBuf::from),
    })
}

/// The next unallocated durable-step ordinal in this process.
///
/// Local `vault-pm` commands drive their write path from one thread, so the
/// ordinal sequence is reproducible run to run. The counter is atomic anyway,
/// so a future concurrent host degrades to "ordinals are still unique" rather
/// than to undefined behavior.
static NEXT_STEP: AtomicU64 = AtomicU64::new(1);

/// Record one landing point, terminating the process when it is the chosen one.
///
/// Returns the ordinal that was consumed, which is useful in this crate's own
/// tests and harmless everywhere else.
pub fn record(step: DurableStep, phase: Phase) -> u64 {
    let ordinal = NEXT_STEP.fetch_add(1, Ordering::SeqCst);
    let policy = policy();
    if let Some(path) = policy.trace.as_ref() {
        append_ledger_line(path, ordinal, step, phase);
    }
    if policy.crash_at == Some(ordinal) {
        crash_now();
    }
    ordinal
}

/// Run one durable write between its two landing points.
///
/// Pairing the two [`record`] calls in one combinator is what makes the
/// "before" and "after" ordinals of a write reliably adjacent, so a sweep can
/// read the parity of an ordinal as "did this write happen".
pub fn around<T>(step: DurableStep, action: impl FnOnce() -> T) -> T {
    record(step, Phase::Before);
    let value = action();
    record(step, Phase::After);
    value
}

/// Render one ledger line.
///
/// Split out from the file write so the exact wire format the drill parses is
/// unit-testable without an environment variable or a real crash.
pub fn ledger_line(ordinal: u64, step: DurableStep, phase: Phase) -> String {
    format!("{ordinal}\t{}\t{}\n", phase.label(), step.label())
}

/// Append one ledger line, refusing any path that is not plainly a file this
/// caller owns.
///
/// The path comes from the environment, so it is treated as untrusted even
/// though only a process that already controls this one's environment can set
/// it. Three rules:
///
/// - the path must be absolute, so a working directory cannot redirect it;
/// - `O_NOFOLLOW` refuses a symlink at the final component, so the ledger can
///   never be appended through a link to somewhere else; and
/// - an existing file must already be owner-only, because `mode(0o600)`
///   applies to creation and would otherwise silently keep a world-readable
///   file world-readable.
fn append_ledger_line(path: &Path, ordinal: u64, step: DurableStep, phase: Phase) {
    let line = ledger_line(ordinal, step, phase);
    let refuse =
        |reason: &str| -> ! { panic!("crash trace {} is unusable: {reason}", path.display()) };
    if !path.is_absolute() {
        refuse("the path must be absolute");
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    // A ledger that cannot be written is a broken drill, not a degraded one:
    // a silent failure here would make a sweep believe an operation performs
    // fewer durable writes than it does, and the missing landing points would
    // never be tested.
    let Ok(mut file) = options.open(path) else {
        refuse("it could not be opened as an owned regular file");
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;
        let Ok(metadata) = file.metadata() else {
            refuse("its metadata could not be read");
        };
        if !metadata.is_file() {
            refuse("it is not a regular file");
        }
        if metadata.permissions().mode() & 0o077 != 0 {
            refuse("it is readable or writable by somebody else");
        }
        // A hard link to a file this user does not own would defeat the mode
        // check above, so confirm ownership as well.
        if metadata.uid() != unsafe_current_uid() {
            refuse("it is owned by another user");
        }
    }
    if file
        .write_all(line.as_bytes())
        .and_then(|()| file.flush())
        .is_err()
    {
        refuse("it could not be appended to");
    }
}

#[cfg(unix)]
fn unsafe_current_uid() -> u32 {
    // SAFETY: `getuid` takes no arguments, touches no memory the compiler
    // knows about, and cannot fail.
    unsafe { libc::getuid() }
}

/// Remove this process immediately, the way a power cut would.
fn crash_now() -> ! {
    #[cfg(unix)]
    {
        // SAFETY: `kill` with the caller's own process id and `SIGKILL` has no
        // preconditions and touches no memory the compiler knows about. The
        // signal is neither catchable nor blockable, so on success control
        // never returns.
        let sent = unsafe { libc::kill(libc::getpid(), libc::SIGKILL) };
        if sent != 0 {
            // A sandbox that filters `kill(2)` must not turn a crash drill
            // into a process that spins forever: fall back to the strongest
            // termination still available. `abort` runs no destructor and
            // flushes nothing, so the only fidelity lost is that the drill's
            // parent sees `SIGABRT` instead of `SIGKILL` — a loud, visible
            // difference rather than a hang.
            std::process::abort();
        }
        // The kernel has already scheduled this process for death. Park rather
        // than spin, so the few instructions before it lands cost nothing; the
        // loop exists only so the function's `!` return type is honest.
        loop {
            std::thread::park();
        }
    }
    #[cfg(not(unix))]
    {
        std::process::abort()
    }
}

/// A [`StorageBackend`] that turns every durable backend write into two
/// landing points.
///
/// Reads are passed through untouched and are not counted: a crash during a
/// read changes nothing on disk, so it collapses into the "before" landing
/// point of the next write.
pub struct CrashInjectingStorageBackend<B> {
    inner: B,
}

impl<B> Debug for CrashInjectingStorageBackend<B> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CrashInjectingStorageBackend")
            .finish_non_exhaustive()
    }
}

impl<B> CrashInjectingStorageBackend<B> {
    /// Wrap one backend.
    pub const fn new(inner: B) -> Self {
        Self { inner }
    }

    /// Borrow the wrapped backend.
    pub const fn inner(&self) -> &B {
        &self.inner
    }

    /// Consume the wrapper and return the wrapped backend.
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: StorageBackend> StorageBackend for CrashInjectingStorageBackend<B> {
    fn initialize(&self) -> Result<(), StorageError> {
        around(DurableStep::StorageInitialize, || self.inner.initialize())
    }

    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
        self.inner.get(namespace, key)
    }

    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
        around(DurableStep::StoragePut, || self.inner.put(input))
    }

    fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<&Revision>,
    ) -> Result<(), StorageError> {
        around(DurableStep::StorageDelete, || {
            self.inner.delete(namespace, key, if_revision)
        })
    }

    fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StoragePage, StorageError> {
        self.inner.list(namespace, options)
    }

    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
        self.inner.stat(namespace, key)
    }

    fn get_summary(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<StorageRecordSummary>, StorageError> {
        self.inner.get_summary(namespace, key)
    }

    fn list_summaries(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StorageSummaryPage, StorageError> {
        self.inner.list_summaries(namespace, options)
    }

    fn acquire_lease(&self, name: &str, ttl_ms: u64) -> Result<Option<StorageLease>, StorageError> {
        around(DurableStep::StorageLease, || {
            self.inner.acquire_lease(name, ttl_ms)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_json_value::JsonValue;
    use std::sync::Mutex;
    use storage_core::InMemoryStorageBackend;

    /// The step counter is process-global, so tests that read it must not
    /// interleave. Everything else in this module is order-independent.
    static COUNTER: Mutex<()> = Mutex::new(());

    fn put_input(namespace: &str, key: &str, body: &[u8]) -> StoragePutInput {
        StoragePutInput::new(
            namespace,
            key,
            "application/octet-stream",
            JsonValue::Object(Vec::new()),
            body.to_vec(),
        )
        .expect("valid put input")
    }

    /// A scratch path no other test, run, or recycled process id can collide
    /// with.
    ///
    /// Three of the tests below assert a *refusal*, so they end by panicking
    /// and never clean up after themselves. Process ids are recycled, so a
    /// name built from the pid alone would eventually meet a leftover file
    /// from a previous run — and since two of those leftovers are deliberately
    /// hostile (a dangling symlink, a world-readable file), meeting one would
    /// flip an unrelated test's result. A monotonic counter makes each name
    /// unique within a run, and the leading `remove_file` clears anything a
    /// recycled pid left behind.
    fn scratch_path(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "vault-pm-crash-injection-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn step_labels_are_stable_and_distinct() {
        let steps = [
            DurableStep::StorageInitialize,
            DurableStep::StoragePut,
            DurableStep::StorageDelete,
            DurableStep::StorageLease,
            DurableStep::ConfigCreate,
            DurableStep::ConfigReplace,
            DurableStep::ExportArtifact,
        ];
        let mut labels: Vec<&str> = steps.iter().map(|step| step.label()).collect();
        let total = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), total, "every step needs a distinct name");
        assert_eq!(DurableStep::StoragePut.to_string(), "storage.put");
        assert_eq!(DurableStep::StorageInitialize.label(), "storage.initialize");
        assert_eq!(DurableStep::StorageDelete.label(), "storage.delete");
        assert_eq!(DurableStep::StorageLease.label(), "storage.lease");
        assert_eq!(DurableStep::ConfigCreate.label(), "config.create");
        assert_eq!(DurableStep::ConfigReplace.label(), "config.replace");
        assert_eq!(DurableStep::ExportArtifact.label(), "export.artifact");
        assert_eq!(Phase::Before.to_string(), "before");
        assert_eq!(Phase::After.to_string(), "after");
        assert!(Phase::Before < Phase::After);
    }

    #[test]
    fn a_ledger_line_names_only_an_ordinal_a_phase_and_a_step() {
        assert_eq!(
            ledger_line(7, DurableStep::StoragePut, Phase::Before),
            "7\tbefore\tstorage.put\n"
        );
        assert_eq!(
            ledger_line(8, DurableStep::StoragePut, Phase::After),
            "8\tafter\tstorage.put\n"
        );
    }

    #[test]
    fn the_ledger_is_appended_to_and_stays_owner_only() {
        let path = scratch_path("ledger");
        append_ledger_line(&path, 1, DurableStep::ConfigCreate, Phase::Before);
        append_ledger_line(&path, 2, DurableStep::ConfigCreate, Phase::After);
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            contents,
            "1\tbefore\tconfig.create\n2\tafter\tconfig.create\n"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    #[should_panic(expected = "the path must be absolute")]
    fn a_relative_ledger_path_is_refused() {
        append_ledger_line(
            Path::new("relative-ledger.tsv"),
            1,
            DurableStep::StoragePut,
            Phase::Before,
        );
    }

    #[cfg(unix)]
    #[test]
    #[should_panic(expected = "could not be opened")]
    fn a_symlinked_ledger_path_is_refused() {
        // Without `O_NOFOLLOW` this would append through the link and quietly
        // write into whatever the link names.
        let target = scratch_path("symlink-target");
        let link = scratch_path("symlink-ledger");
        std::fs::write(&target, b"").unwrap();
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&target, &link).unwrap();
        append_ledger_line(&link, 1, DurableStep::StoragePut, Phase::Before);
    }

    #[cfg(unix)]
    #[test]
    #[should_panic(expected = "readable or writable by somebody else")]
    fn a_world_readable_ledger_is_refused() {
        // `mode(0o600)` only applies at creation, so an existing loose file
        // has to be rejected explicitly rather than silently kept loose.
        use std::os::unix::fs::PermissionsExt;
        let path = scratch_path("loose-ledger");
        std::fs::write(&path, b"").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        append_ledger_line(&path, 1, DurableStep::StoragePut, Phase::Before);
    }

    #[test]
    fn ordinals_are_unique_and_monotonic() {
        let _guard = COUNTER.lock().unwrap();
        let first = record(DurableStep::ConfigCreate, Phase::Before);
        let second = record(DurableStep::ConfigCreate, Phase::After);
        assert_eq!(second, first + 1);
    }

    #[test]
    fn around_brackets_its_action_with_two_ordinals() {
        let _guard = COUNTER.lock().unwrap();
        let before = NEXT_STEP.load(Ordering::SeqCst);
        let inside = around(DurableStep::ExportArtifact, || {
            NEXT_STEP.load(Ordering::SeqCst)
        });
        let after = NEXT_STEP.load(Ordering::SeqCst);
        assert_eq!(inside, before + 1, "the action runs after the before-step");
        assert_eq!(after, before + 2, "an after-step follows the action");
    }

    #[test]
    fn writes_are_wrapped_and_reads_pass_through() {
        let _guard = COUNTER.lock().unwrap();
        let backend = CrashInjectingStorageBackend::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();

        let before = NEXT_STEP.load(Ordering::SeqCst);
        backend.get("ns", "missing").unwrap();
        backend.list("ns", StorageListOptions::default()).unwrap();
        backend
            .list_summaries("ns", StorageListOptions::default())
            .unwrap();
        backend.stat("ns", "missing").unwrap();
        backend.get_summary("ns", "missing").unwrap();
        assert_eq!(
            NEXT_STEP.load(Ordering::SeqCst),
            before,
            "reads must not consume landing points"
        );

        backend.put(put_input("ns", "key", b"body")).unwrap();
        assert_eq!(NEXT_STEP.load(Ordering::SeqCst), before + 2);
        assert_eq!(
            backend.get("ns", "key").unwrap().unwrap().body,
            b"body".to_vec()
        );

        backend.acquire_lease("writer", 1_000).unwrap();
        assert_eq!(NEXT_STEP.load(Ordering::SeqCst), before + 4);

        backend.delete("ns", "key", None).unwrap();
        assert_eq!(NEXT_STEP.load(Ordering::SeqCst), before + 6);
        assert!(backend.get("ns", "key").unwrap().is_none());
    }

    #[test]
    fn wrapper_exposes_the_backend_it_wraps() {
        let backend = CrashInjectingStorageBackend::new(InMemoryStorageBackend::new());
        backend.inner().initialize().unwrap();
        assert!(format!("{backend:?}").contains("CrashInjectingStorageBackend"));
        let recovered = backend.into_inner();
        recovered.initialize().unwrap();
    }

    #[test]
    fn an_unset_policy_neither_traces_nor_crashes() {
        // The package's own test process sets neither variable, so this is the
        // production-shaped path: recording is a counter bump and nothing more.
        let policy = policy();
        assert!(policy.crash_at.is_none());
        assert!(policy.trace.is_none());
        for _ in 0..8 {
            record(DurableStep::StorageInitialize, Phase::After);
        }
    }
}
