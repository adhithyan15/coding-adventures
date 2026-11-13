//! The gate driver (VER08b sections 2, 6, 7, 8, 9): one test binary that is
//! both the *parent* (observer, judge, controls, evidence) and the *child*
//! (setup, arm, exercise).
//!
//! ```text
//!   cargo test --test runtime_capability          (parent, role = default)
//!     |
//!     |-- strace -f -v -- <self> --rcg-child N enforce             the real run
//!     |-- 3 rings x 18 sabotage runs                               negative controls
//!     |-- env-leak run                                             launch-time control
//!     |-- docker run ... <image> /rcg/test --rcg-child ...         container ring
//!     `-- write capability_runtime_evidence.json
//!
//!   <self> --rcg-child N enforce                  (child)
//!     phase A: prepare() -> audits -> arm()
//!     marker : write(2, "RCG-ARMED N landlock=.. abi=.. seccomp=..")
//!     phase B: exercise() -> TAP on fd 1 -> exit
//! ```

use crate::failure::FailureId;
use crate::json::Json;
use crate::observe::{Analysis, Ending, Violation};
use crate::policy::{PolicyOptions, RuntimePolicy};
use std::path::PathBuf;

/// One named check the package's test program reports.
#[derive(Debug, Clone)]
pub struct Check {
    /// TAP description.
    pub name: String,
    /// Whether it passed.
    pub ok: bool,
}

/// The armed-phase body: returns the checks it ran. Built during phase A by
/// [`GateConfig::prepare`], so everything it needs is already allocated.
pub type Exercise = Box<dyn FnOnce() -> Vec<Check>>;

/// What the package supplies.
pub struct GateConfig {
    /// `rust/ct-compare`-style label (also validated against the manifest).
    pub package: &'static str,
    /// The manifest text, `include_str!`ed so parent and child agree.
    pub manifest_json: &'static str,
    /// Where to write `capability_runtime_evidence.json` (and scratch files).
    pub evidence_dir: Option<PathBuf>,
    /// Phase A: load vectors, set the panic hook, return the armed-phase body.
    pub prepare: fn() -> Exercise,
    /// Policy options (for example coarse thread mapping).
    pub options: PolicyOptions,
    /// Which optional rings to run (packages run all of them; this crate's
    /// own synthetic scenarios skip the slow ones).
    pub rings: Rings,
}

/// Which optional parts of the gate to run. The main run and the observer
/// always run; the negative controls and the container ring are the slow,
/// package-independent parts.
#[derive(Debug, Clone, Copy)]
pub struct Rings {
    /// Run the mandatory negative controls (VER08b section 8).
    pub controls: bool,
    /// Run the Docker ring (reports not-live if Docker is unavailable).
    pub container: bool,
}

impl Rings {
    /// Everything, as a real package gate must.
    pub const ALL: Rings = Rings {
        controls: true,
        container: true,
    };
    /// Main run + observer only.
    pub const MAIN_ONLY: Rings = Rings {
        controls: false,
        container: false,
    };
}

/// How a child process ended, independent of the observer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildStatus {
    /// Normal exit with a code.
    Exited(i32),
    /// Terminated by a signal number.
    Signaled(i32),
}

/// SIGSYS on Linux.
pub const SIGSYS: i32 = 31;

/// The verdicts of both rings for one run, plus whether they agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judgement {
    /// The ARMED marker was seen.
    pub armed: bool,
    /// The process was killed by SIGSYS.
    pub killed_by_sigsys: bool,
    /// Exit code if it exited normally.
    pub exit_code: Option<i32>,
    /// Post-arm calls outside the allowed set.
    pub violations: Vec<Violation>,
    /// Environment variable names present that no capability allows.
    pub env_violations: Vec<String>,
    /// Registered failure ids, deduplicated, in order.
    pub ids: Vec<FailureId>,
    /// Enforcement verdict: no SIGSYS.
    pub enforcement_pass: bool,
    /// Observation verdict: armed, no violations, no env violations.
    pub observation_pass: bool,
    /// Enforcement and observation agree (always true when the filter was off).
    pub agreement: bool,
}

impl Judgement {
    /// True if there is no failure id at all.
    pub fn pass(&self) -> bool {
        self.ids.is_empty()
    }
}

/// Combine the observer's analysis and the process status into one
/// judgement (the agreement table of VER08b section 6). `enforced` says
/// whether the in-process filter was supposed to be installed.
pub fn judge(
    analysis: &Analysis,
    status: &ChildStatus,
    enforced: bool,
    policy: &RuntimePolicy,
) -> Judgement {
    let killed = matches!(&analysis.ending, Ending::Killed(sig) if sig == "SIGSYS")
        || (analysis.ending == Ending::Unknown && *status == ChildStatus::Signaled(SIGSYS));
    let exit_code = match (&analysis.ending, status) {
        (Ending::Exited(code), _) => Some(*code),
        (_, ChildStatus::Exited(code)) => Some(*code),
        _ => None,
    };
    let env_violations = analysis
        .exec_env
        .as_ref()
        .map(|env| crate::observe::env_violations(env, policy))
        .unwrap_or_default();
    let violations = analysis.violations.clone();

    let mut ids = Vec::new();
    let mut push = |id: FailureId| {
        if !ids.contains(&id) {
            ids.push(id);
        }
    };
    if !analysis.armed {
        match exit_code {
            Some(crate::failure::exit_code::INSTALL_FAILED) => push(FailureId::Rcg008),
            Some(crate::failure::exit_code::LANDLOCK_MISSING) => push(FailureId::Rcg012),
            Some(crate::failure::exit_code::ENV_EXPOSED) => push(FailureId::Rcg005),
            Some(crate::failure::exit_code::FD_LEAK) => push(FailureId::Rcg011),
            _ => push(FailureId::Rcg004),
        }
    }
    if !env_violations.is_empty() {
        push(FailureId::Rcg005);
    }
    match (killed, violations.is_empty()) {
        (true, false) => push(FailureId::Rcg001),
        (true, true) => push(FailureId::Rcg003),
        (false, false) => {
            push(FailureId::Rcg002);
            if enforced {
                push(FailureId::Rcg003);
            }
        }
        (false, true) => {}
    }
    if analysis.armed && !killed && violations.is_empty() && exit_code.is_some_and(|c| c != 0) {
        push(FailureId::Rcg007);
    }

    let agreement = !enforced || (killed == !violations.is_empty());
    Judgement {
        armed: analysis.armed,
        killed_by_sigsys: killed,
        exit_code,
        env_violations: env_violations.clone(),
        enforcement_pass: !killed,
        observation_pass: analysis.armed && violations.is_empty() && env_violations.is_empty(),
        violations,
        ids,
        agreement,
    }
}

/// Render a [`Violation`] list as JSON for the evidence file.
pub fn violations_json(violations: &[Violation]) -> Json {
    Json::Arr(
        violations
            .iter()
            .map(|v| {
                Json::obj([
                    ("syscall", Json::str(&v.name)),
                    ("family", Json::str(v.family)),
                    ("args", Json::str(&v.args)),
                ])
            })
            .collect(),
    )
}

/// Run the gate. Returns the process exit code (0 = pass, or not-live in
/// non-strict mode; 1 = fail). The caller should `std::process::exit` with it.
///
/// When invoked with `--rcg-child ...` this function runs the child role and
/// never returns.
pub fn run_package_gate(config: &GateConfig) -> i32 {
    imp::run(config)
}

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(not(target_os = "linux"))]
use other as imp;

// ---------------------------------------------------------------------------
// Non-Linux: honest not-live reporting (RCG010). Never a silent pass.
// ---------------------------------------------------------------------------
#[cfg(not(target_os = "linux"))]
mod other {
    use super::*;

    pub fn run(config: &GateConfig) -> i32 {
        let reason = format!(
            "{}: runtime capability gate is Linux-only (seccomp/Landlock/strace); \
             this platform has no live ring",
            FailureId::Rcg010
        );
        println!("NOT-LIVE {} ({reason})", config.package);
        if let Some(dir) = &config.evidence_dir {
            let _ = std::fs::create_dir_all(dir);
            let evidence = Json::obj([
                ("schema", Json::str("capability_runtime_evidence.v1")),
                ("package", Json::str(config.package)),
                ("live", Json::Bool(false)),
                ("not_live_reason", Json::str(&reason)),
                ("overall", Json::str("not-live")),
            ]);
            let _ = std::fs::write(
                dir.join("capability_runtime_evidence.json"),
                evidence.pretty(),
            );
        }
        if std::env::var_os("RCG_STRICT").is_some() {
            1
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Linux: the live gate.
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use crate::arm::{arm, landlock_abi, ArmError, ArmReport};
    use crate::container::{docker_args, parse_ldd, seccomp_profile_json};
    use crate::failure::exit_code;
    use crate::observe::analyze;
    use crate::policy::BASELINE;
    use crate::sabotage::{self, CONTROLS};
    use std::io::Write as _;
    use std::os::unix::process::{CommandExt as _, ExitStatusExt};
    use std::path::Path;
    use std::process::{Command, Stdio};

    /// Variables Docker injects into every container; scrubbed in phase A.
    const CONTAINER_INJECTED_ENV: &[&str] = &["PATH", "HOSTNAME", "HOME"];

    /// Variables that the *toolchain runtime inside the child* sets for
    /// itself after `exec` (the LLVM coverage runtime does this in an
    /// instrumented build). The launcher never passed them, and the parent's
    /// independent `execve` check would still flag a leaked copy, so the
    /// in-child audit ignores exactly these names.
    const SELF_SET_ENV: &[&str] = &["__LLVM_PROFILE_RT_INIT_ONCE"];

    pub fn run(config: &GateConfig) -> i32 {
        let args: Vec<String> = std::env::args().collect();
        if let Some(position) = args.iter().position(|a| a == "--rcg-child") {
            child_main(config, &args[position + 1..]);
        }
        parent_main(config)
    }

    // ----- child ---------------------------------------------------------

    fn raw_write(fd: i32, text: &str) {
        // SAFETY: writes `text.len()` bytes from a live &str to an fd.
        unsafe {
            libc::write(fd, text.as_ptr().cast(), text.len());
        }
    }

    fn child_main(config: &GateConfig, args: &[String]) -> ! {
        let nonce = args.first().cloned().unwrap_or_default();
        let enforce = args.get(1).map(String::as_str) == Some("enforce");
        let container = args.iter().any(|a| a == "--rcg-container");
        let sabotage_name = args
            .iter()
            .position(|a| a == "--rcg-sabotage")
            .and_then(|i| args.get(i + 1))
            .cloned();

        let policy = match RuntimePolicy::from_manifest_json(config.manifest_json, config.options) {
            Ok(policy) => policy,
            Err(refusal) => {
                eprintln!("{refusal}");
                std::process::exit(exit_code::INSTALL_FAILED);
            }
        };
        // SAFETY: setrlimit with a valid struct; no core dump on SIGSYS.
        unsafe {
            let zero = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &zero);
        }
        if container {
            for name in CONTAINER_INJECTED_ENV {
                if !policy.env_name_allowed(name) {
                    std::env::remove_var(name);
                }
            }
        }

        // ---- phase A ----
        let exercise = (config.prepare)();
        let _ = std::io::stdout().flush();
        let stray_env: Vec<String> = std::env::vars_os()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .filter(|name| !policy.env_name_allowed(name) && !SELF_SET_ENV.contains(&name.as_str()))
            .collect();
        if !stray_env.is_empty() {
            eprintln!(
                "{}: unexpected environment: {stray_env:?}",
                FailureId::Rcg005
            );
            std::process::exit(exit_code::ENV_EXPOSED);
        }
        let stray_fds = unexpected_fds();
        if !stray_fds.is_empty() {
            eprintln!("{}: unexpected fds: {stray_fds:?}", FailureId::Rcg011);
            std::process::exit(exit_code::FD_LEAK);
        }
        let report = if enforce {
            match arm(&policy) {
                Ok(report) => report,
                Err(ArmError::LandlockRequired) => {
                    eprintln!("{}: Landlock required", FailureId::Rcg012);
                    std::process::exit(exit_code::LANDLOCK_MISSING);
                }
                Err(error) => {
                    eprintln!("{}: {error}", FailureId::Rcg008);
                    std::process::exit(exit_code::INSTALL_FAILED);
                }
            }
        } else {
            ArmReport {
                landlock_abi: landlock_abi(),
                landlock_enforced: false,
                seccomp_enforced: false,
            }
        };

        // ---- the boundary ----
        raw_write(
            2,
            &format!(
                "RCG-ARMED {nonce} landlock={} abi={} seccomp={}\n",
                u8::from(report.landlock_enforced),
                report.landlock_abi,
                u8::from(report.seccomp_enforced)
            ),
        );

        // ---- phase B ----
        if let Some(name) = sabotage_name {
            let known = sabotage::attempt(&name, &nonce);
            println!("SABOTAGE-SURVIVED {name} known={known}");
            let _ = std::io::stdout().flush();
            fast_exit(0);
        }
        let checks = exercise();
        let mut failed = 0;
        for (index, check) in checks.iter().enumerate() {
            if check.ok {
                println!("ok {} - {}", index + 1, check.name);
            } else {
                failed += 1;
                println!("not ok {} - {}", index + 1, check.name);
            }
        }
        println!("1..{}", checks.len());
        let _ = std::io::stdout().flush();
        fast_exit(if failed == 0 {
            0
        } else {
            exit_code::CHECK_FAILED
        });
    }

    /// Leave via `_exit(2)`: straight to `exit_group`, skipping `std`'s
    /// runtime cleanup (which would `sigaltstack`/`munmap` its handler stack)
    /// and libc's atexit machinery. Stdout is flushed by the caller first.
    /// This keeps the baseline to what the *library* needs.
    fn fast_exit(code: i32) -> ! {
        // SAFETY: `_exit` takes an integer and never returns.
        unsafe { libc::_exit(code) }
    }

    /// File descriptors other than 0-2 that are still open.
    fn unexpected_fds() -> Vec<String> {
        let mut found = Vec::new();
        let names: Vec<String> = match std::fs::read_dir("/proc/self/fd") {
            Ok(dir) => dir
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => return found,
        };
        for name in names {
            let Ok(fd) = name.parse::<i32>() else {
                continue;
            };
            if fd <= 2 {
                continue;
            }
            // The read_dir handle is closed by now, so its link no longer
            // resolves; anything that still resolves is a real descriptor.
            if let Ok(target) = std::fs::read_link(format!("/proc/self/fd/{name}")) {
                found.push(format!("{fd}->{}", target.display()));
            }
        }
        found
    }

    // ----- parent --------------------------------------------------------

    struct RunOutcome {
        status: ChildStatus,
        stdout: String,
        stderr: String,
        trace: Option<String>,
    }

    fn which(program: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    }

    fn status_of(status: std::process::ExitStatus) -> ChildStatus {
        match status.signal() {
            Some(signal) => ChildStatus::Signaled(signal),
            None => ChildStatus::Exited(status.code().unwrap_or(-1)),
        }
    }

    /// Why an observed run's trace cannot be trusted, if it cannot.
    ///
    /// An absent, unreadable or empty trace is not "no violations"; it is an
    /// observer that saw nothing, and counting it as a clean observation
    /// would fail open. Callers turn `Some` into an `RCG013` failure.
    fn trace_problem(outcome: &RunOutcome) -> Option<&'static str> {
        match outcome.trace.as_deref() {
            None => Some("trace file absent or unreadable"),
            Some(trace) if trace.trim().is_empty() => Some("trace file empty"),
            Some(_) => None,
        }
    }

    fn no_trace_failure(what: &str, why: &str) -> String {
        format!(
            "{}: observer produced no trace ({what}: {why})",
            FailureId::Rcg013
        )
    }

    struct Runner<'a> {
        exe: PathBuf,
        strace: Option<PathBuf>,
        scratch: &'a Path,
    }

    impl Runner<'_> {
        /// Run the child with an exactly specified environment.
        fn run(
            &self,
            nonce: &str,
            enforce: bool,
            sabotage: Option<&str>,
            env: &[(&str, &str)],
            observe: bool,
        ) -> std::io::Result<RunOutcome> {
            let mode = if enforce { "enforce" } else { "observe-only" };
            let mut child_args: Vec<String> = vec!["--rcg-child".into(), nonce.into(), mode.into()];
            if let Some(name) = sabotage {
                child_args.push("--rcg-sabotage".into());
                child_args.push(name.into());
            }
            let trace_path = self.scratch.join(format!("trace-{nonce}.txt"));
            let mut command = match (&self.strace, observe) {
                (Some(strace), true) => {
                    let mut c = Command::new(strace);
                    c.args(["-f", "-v", "-q", "-s", "4096", "-o"])
                        .arg(&trace_path)
                        .arg("--")
                        .arg(&self.exe);
                    c
                }
                _ => Command::new(&self.exe),
            };
            command
                .args(&child_args)
                .env_clear()
                .envs(env.iter().copied())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            // SAFETY: the closure runs between fork and exec and only calls
            // async-signal-safe functions (`close_range`/`fcntl`/`getrlimit`).
            unsafe {
                command.pre_exec(|| {
                    close_inherited_fds();
                    Ok(())
                });
            }
            let output = command.output()?;
            let trace = std::fs::read_to_string(&trace_path).ok();
            let _ = std::fs::remove_file(&trace_path);
            Ok(RunOutcome {
                status: status_of(output.status),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                trace,
            })
        }
    }

    /// Mark every descriptor above stderr close-on-exec in the freshly
    /// forked launcher child, so that `exec` drops them.
    ///
    /// `std::process::Command` only closes descriptors that are already
    /// `O_CLOEXEC`. A CI runner (or a build tool above it) routinely leaks
    /// plain pipes into every process it starts, and without this step they
    /// reach the gate's child, where the RCG011 audit rightly refuses them
    /// ("no inherited descriptor may be ambient authority"). The audit is
    /// the guarantee; this makes the *launcher* hand over exactly 0/1/2 so
    /// the audit measures the launch, not the runner's hygiene. `Command`'s
    /// own stdio plumbing is already dup'd onto 0-2 by this point.
    ///
    /// Why set CLOEXEC instead of closing: between fork and exec, `std`
    /// keeps a private CLOEXEC pipe open to report an `exec` failure back
    /// to the parent. Closing it would turn "exec failed" into an ordinary
    /// exit status; marking descriptors CLOEXEC leaves that pipe working
    /// and still guarantees nothing above 2 survives into the new image.
    fn close_inherited_fds() {
        // Linux 5.11+: one call marks the whole range CLOEXEC.
        const CLOSE_RANGE_CLOEXEC: u32 = 1 << 2;
        // SAFETY: raw syscalls with integer arguments; `getrlimit` writes
        // into a live local struct.
        unsafe {
            if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, CLOSE_RANGE_CLOEXEC) == 0 {
                return;
            }
            let mut limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            let top = if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) == 0 {
                limit.rlim_cur.min(1 << 20) as i32
            } else {
                4096
            };
            for fd in 3..top {
                // EBADF for descriptors that are not open is expected.
                libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
            }
        }
    }

    /// A working directory name that belongs to THIS gate process alone.
    ///
    /// `evidence_dir` is shared: Cargo's `CARGO_TARGET_TMPDIR`, which every
    /// Rust adopter passes, is one directory per workspace, and the build
    /// tool runs independent packages in parallel. A fixed name let the
    /// first gate to finish delete the directory a slower gate was still
    /// writing `strace -o` traces into (PR #19314, `rust/gf256`, CI only), so
    /// the name carries the process id.
    ///
    /// The process id alone is *predictable*, and when `evidence_dir` is
    /// `None` the directory sits under shared `/tmp`, where another local
    /// user could plant a symlink at `rcg-scratch-<pid>` and have strace
    /// write through it. So the name also carries 64 random bits from the
    /// same source as the run nonces, and the directory is only ever made by
    /// [`create_private_dir`], which refuses anything already there.
    /// Only `capability_runtime_evidence.json` stays at a fixed path.
    fn private_dir(config: &GateConfig, leaf: &str) -> PathBuf {
        config
            .evidence_dir
            .clone()
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("{leaf}-{}-{}", std::process::id(), random_hex()))
    }

    /// Create `path` as a brand-new directory, mode 0700, or fail.
    ///
    /// ```text
    /// create_dir_all(path)   follows a symlink at `path`  -> writes land
    ///                        wherever the attacker pointed it
    /// mkdir(path, 0700)      EEXIST for a symlink, a file or a directory;
    ///                        the final component is never followed
    /// ```
    ///
    /// Only the *parent* is created recursively (it is the adopter's shared
    /// evidence directory, or `/tmp`, never a name we invented). The leaf is
    /// created non-recursively and exclusively, so an existing path of any
    /// kind is refused rather than reused, and the 0700 mode keeps other
    /// users out of the traces even for the instant before anything is
    /// written. A refusal is the caller's NOT-LIVE, never a fallback.
    fn create_private_dir(path: &Path) -> std::io::Result<()> {
        use std::os::unix::fs::DirBuilderExt as _;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::DirBuilder::new().mode(0o700).create(path)
    }

    /// A private directory that deletes itself.
    ///
    /// The gate has many early returns after the directory exists (`NOT-LIVE`
    /// because strace is missing or cannot trace, a child that will not
    /// spawn, ...), and a panic unwinds through all of them. Cleaning up by
    /// hand at the end only covered the happy path; `Drop` covers every
    /// return and every unwind. A SIGKILL cannot be caught by anything, so a
    /// killed gate can still leave its (private, unpredictable) directory.
    struct ScratchDir {
        path: PathBuf,
    }

    impl ScratchDir {
        fn new(config: &GateConfig, leaf: &str) -> std::io::Result<ScratchDir> {
            let path = private_dir(config, leaf);
            create_private_dir(&path)?;
            Ok(ScratchDir { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            // `remove_dir_all` does not follow a symlink at the top path.
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[cfg(test)]
    thread_local! {
        /// Test-only: make the OS randomness source fail on this thread.
        static RANDOM_FAILS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    fn random_hex() -> String {
        let mut bytes = [0u8; 8];
        // SAFETY: getrandom into a live 8-byte buffer.
        let got = unsafe { libc::getrandom(bytes.as_mut_ptr().cast(), bytes.len(), 0) };
        if got != bytes.len() as isize {
            // getrandom essentially cannot fail on Linux >= 3.17; if it does,
            // fall back to clock and address bits rather than all zeros.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let here = &bytes as *const _ as u64;
            bytes = (nanos ^ here.rotate_left(32)).to_le_bytes();
        }
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn random_nonce(tag: &str) -> String {
        format!("{tag}-{}", random_hex())
    }

    fn command_line(program: &str, args: &[&str]) -> Option<String> {
        let output = Command::new(program).args(args).output().ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::to_string)
    }

    fn kernel_info() -> String {
        // SAFETY: uname fills a zeroed utsname struct.
        let mut u: libc::utsname = unsafe { std::mem::zeroed() };
        // SAFETY: valid pointer to utsname.
        unsafe { libc::uname(&mut u) };
        let field = |raw: &[libc::c_char]| {
            let bytes: Vec<u8> = raw
                .iter()
                .take_while(|c| **c != 0)
                .map(|c| *c as u8)
                .collect();
            String::from_utf8_lossy(&bytes).into_owned()
        };
        format!(
            "{} {} {}",
            field(&u.sysname),
            field(&u.release),
            field(&u.machine)
        )
    }

    fn strict() -> bool {
        std::env::var_os("RCG_STRICT").is_some()
    }

    fn not_live(config: &GateConfig, reason: &str, id: FailureId) -> i32 {
        let message = format!("{id}: {reason}");
        println!("NOT-LIVE {} ({message})", config.package);
        write_evidence(
            config,
            &Json::obj([
                ("schema", Json::str("capability_runtime_evidence.v1")),
                ("package", Json::str(config.package)),
                ("gate_version", Json::str("rcg-1")),
                ("live", Json::Bool(false)),
                ("not_live_reason", Json::str(&message)),
                ("overall", Json::str("not-live")),
            ]),
        );
        if strict() {
            1
        } else {
            0
        }
    }

    fn write_evidence(config: &GateConfig, evidence: &Json) {
        if let Some(dir) = &config.evidence_dir {
            let _ = std::fs::create_dir_all(dir);
            let path = dir.join("capability_runtime_evidence.json");
            match std::fs::write(&path, evidence.pretty()) {
                Ok(()) => println!("evidence: {}", path.display()),
                Err(error) => println!("evidence write failed: {error}"),
            }
        }
    }

    fn parent_main(config: &GateConfig) -> i32 {
        let policy = match RuntimePolicy::from_manifest_json(config.manifest_json, config.options) {
            Ok(policy) => policy,
            Err(refusal) => {
                println!("REFUSED {} {refusal}", config.package);
                write_evidence(
                    config,
                    &Json::obj([
                        ("schema", Json::str("capability_runtime_evidence.v1")),
                        ("package", Json::str(config.package)),
                        ("live", Json::Bool(true)),
                        ("refusal", Json::str(refusal.to_string())),
                        ("overall", Json::str("fail")),
                    ]),
                );
                return 1;
            }
        };
        // Removed by Drop on every return path (and on unwind).
        let scratch_guard = match ScratchDir::new(config, "rcg-scratch") {
            Ok(guard) => guard,
            Err(error) => {
                return not_live(
                    config,
                    &format!("cannot create private scratch directory: {error}"),
                    FailureId::Rcg013,
                )
            }
        };
        let scratch = scratch_guard.path().to_path_buf();
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(error) => {
                return not_live(
                    config,
                    &format!("no current_exe: {error}"),
                    FailureId::Rcg013,
                )
            }
        };

        // The observer must work before any verdict is trusted.
        let Some(strace) = which("strace") else {
            return not_live(config, "strace not installed", FailureId::Rcg013);
        };
        let probe = Command::new(&strace)
            .args(["-f", "-o", "/dev/null", "--", "/bin/true"])
            .env_clear()
            .status();
        if !probe.map(|s| s.success()).unwrap_or(false) {
            return not_live(
                config,
                "strace cannot trace here (ptrace blocked?)",
                FailureId::Rcg013,
            );
        }

        let runner = Runner {
            exe: exe.clone(),
            strace: Some(strace.clone()),
            scratch: &scratch,
        };
        let mut failures: Vec<String> = Vec::new();

        // ---- the real run: both rings ----
        let nonce = random_nonce("main");
        let main_json;
        let main_pass;
        let observed;
        match runner.run(&nonce, true, None, &[], true) {
            Ok(outcome) => {
                let trace = outcome.trace.clone().unwrap_or_default();
                let analysis = analyze(&trace, &nonce, &policy);
                let judgement = judge(&analysis, &outcome.status, true, &policy);
                let tap_ok = outcome
                    .stdout
                    .lines()
                    .filter(|l| l.starts_with("ok "))
                    .count();
                let tap_bad = outcome
                    .stdout
                    .lines()
                    .filter(|l| l.starts_with("not ok"))
                    .count();
                println!(
                    "main run: armed={} checks_ok={tap_ok} checks_failed={tap_bad} ids={:?} exit={:?}",
                    judgement.armed,
                    judgement.ids.iter().map(|i| i.as_str()).collect::<Vec<_>>(),
                    judgement.exit_code
                );
                for v in &judgement.violations {
                    println!("  VIOLATION [{}] {} {}", v.family, v.name, v.args);
                }
                if !judgement.pass() {
                    println!("--- child stderr ---\n{}", outcome.stderr);
                    failures.push(format!(
                        "main run failed: {:?}",
                        judgement.ids.iter().map(|i| i.as_str()).collect::<Vec<_>>()
                    ));
                }
                if tap_ok == 0 {
                    failures.push("no checks ran".to_string());
                }
                let trace_missing = trace_problem(&outcome);
                if let Some(why) = trace_missing {
                    println!("  OBSERVER BROKEN: {why}");
                    failures.push(no_trace_failure("main run", why));
                }
                observed = analysis.post_arm.clone();
                main_pass = judgement.pass() && tap_ok > 0 && trace_missing.is_none();
                let (enforcement_kind, observation_kind) = (
                    if judgement.enforcement_pass {
                        "pass"
                    } else {
                        "fail"
                    },
                    if judgement.observation_pass && trace_missing.is_none() {
                        "pass"
                    } else {
                        "fail"
                    },
                );
                let detail = analysis.marker_detail.clone();
                main_json = Json::obj([
                    (
                        "enforcement",
                        Json::obj([
                            ("verdict", Json::str(enforcement_kind)),
                            ("marker", Json::str(&detail)),
                            ("seccomp_enforced", Json::Bool(detail.contains("seccomp=1"))),
                            (
                                "landlock_enforced",
                                Json::Bool(detail.contains("landlock=1")),
                            ),
                            ("child_status", Json::str(format!("{:?}", outcome.status))),
                            ("checks_ok", Json::Num(tap_ok as u64)),
                            ("checks_failed", Json::Num(tap_bad as u64)),
                            (
                                "failure_ids",
                                Json::strings(judgement.ids.iter().map(|i| i.as_str())),
                            ),
                        ]),
                    ),
                    (
                        "observation",
                        Json::obj([
                            ("verdict", Json::str(observation_kind)),
                            ("observer", Json::str("strace -f -v")),
                            ("armed_marker_seen", Json::Bool(judgement.armed)),
                            ("pre_arm_syscalls", Json::Num(analysis.pre_arm_calls)),
                            (
                                "observed_syscalls",
                                Json::Obj(
                                    analysis
                                        .post_arm
                                        .iter()
                                        .map(|(k, v)| (k.clone(), Json::Num(*v)))
                                        .collect(),
                                ),
                            ),
                            (
                                "unobserved_baseline",
                                Json::strings(
                                    BASELINE
                                        .iter()
                                        .map(|b| b.name)
                                        .filter(|n| !analysis.post_arm.contains_key(*n)),
                                ),
                            ),
                            ("violations", violations_json(&judgement.violations)),
                            (
                                "exec_env",
                                Json::strings(analysis.exec_env.clone().unwrap_or_default()),
                            ),
                        ]),
                    ),
                    ("agreement", Json::Bool(judgement.agreement)),
                ]);
            }
            Err(error) => {
                return not_live(
                    config,
                    &format!("cannot spawn child: {error}"),
                    FailureId::Rcg013,
                )
            }
        }

        // ---- negative controls ----
        let mut control_json = Vec::new();
        let mut known_gaps = Vec::new();
        for control in CONTROLS.iter().filter(|_| config.rings.controls) {
            let result = run_control(&runner, &policy, control);
            for (run, why) in &result.trace_problems {
                failures.push(no_trace_failure(
                    &format!("control {}, {run} run", control.name),
                    why,
                ));
            }
            if !result.all_detected() {
                failures.push(format!(
                    "{}: negative control {} NOT detected",
                    FailureId::Rcg009,
                    control.name
                ));
            }
            println!(
                "control {:<14} full(enf={} obs={}) observe-only={} enforce-only={}",
                control.name,
                result.full_enforced,
                result.full_observed,
                result.observe_only,
                result.enforce_only
            );
            control_json.push(result.to_json(control.name, control.expected));
        }
        // env-leak: a launcher that leaks a variable the manifest does not name.
        if config.rings.controls {
            let nonce = random_nonce("env");
            let leak = runner.run(&nonce, true, None, &[("RCG_CANARY", "leaked")], true);
            let (enforced, observed_env, ids) = match leak {
                Ok(outcome) => {
                    if let Some(why) = trace_problem(&outcome) {
                        failures.push(no_trace_failure("control env-leak", why));
                    }
                    let analysis =
                        analyze(&outcome.trace.clone().unwrap_or_default(), &nonce, &policy);
                    let env_hits = analysis
                        .exec_env
                        .as_ref()
                        .map(|e| crate::observe::env_violations(e, &policy))
                        .unwrap_or_default();
                    (
                        outcome.status == ChildStatus::Exited(exit_code::ENV_EXPOSED),
                        env_hits.contains(&"RCG_CANARY".to_string()),
                        vec![FailureId::Rcg005],
                    )
                }
                Err(_) => (false, false, vec![]),
            };
            println!(
                "control env-leak     in-child audit={enforced} execve-envp check={observed_env}"
            );
            if !(enforced && observed_env) {
                failures.push(format!(
                    "{}: negative control env-leak NOT detected",
                    FailureId::Rcg009
                ));
            }
            control_json.push(Json::obj([
                ("name", Json::str("env-leak")),
                ("expected_syscall", Json::Null),
                (
                    "in_child_audit",
                    Json::obj([
                        ("detected", Json::Bool(enforced)),
                        (
                            "failure_id",
                            Json::str(ids.first().map_or("", |i| i.as_str())),
                        ),
                    ]),
                ),
                (
                    "execve_observer",
                    Json::obj([
                        ("detected", Json::Bool(observed_env)),
                        ("failure_id", Json::str("RCG005")),
                    ]),
                ),
            ]));
        }
        // known gap: vDSO time is invisible to both rings.
        if config.rings.controls {
            let nonce = random_nonce("gap");
            let detected = match runner.run(&nonce, true, Some(sabotage::TIME_VDSO), &[], true) {
                Ok(outcome) => {
                    if let Some(why) = trace_problem(&outcome) {
                        failures.push(no_trace_failure("known gap time-vdso", why));
                    }
                    let analysis =
                        analyze(&outcome.trace.clone().unwrap_or_default(), &nonce, &policy);
                    !analysis.violations.is_empty()
                        || outcome.status == ChildStatus::Signaled(SIGSYS)
                }
                Err(_) => false,
            };
            println!(
                "known gap time-vdso  detected={detected} (expected false: vDSO makes no syscall)"
            );
            known_gaps.push(Json::obj([
                ("name", Json::str("time-vdso")),
                ("detected", Json::Bool(detected)),
                ("note", Json::str("Instant::now() is served by the vDSO without a syscall; neither seccomp nor strace can see it")),
            ]));
        }
        drop(scratch_guard); // free the traces before the (slow) container ring

        // ---- container ring ----
        let container = if config.rings.container {
            container_ring(config, &policy, &exe)
        } else {
            ContainerResult {
                verdict: ContainerVerdict::NotLive,
                detail: "skipped by configuration".to_string(),
                invocation: Vec::new(),
                control_detected: None,
            }
        };
        let container_json = container.to_json();
        match container.verdict {
            ContainerVerdict::Pass => {}
            ContainerVerdict::NotLive => {
                println!("container ring: NOT-LIVE ({})", container.detail);
                if std::env::var_os("RCG_REQUIRE_DOCKER").is_some() {
                    failures.push(format!(
                        "{}: docker required but unavailable",
                        FailureId::Rcg014
                    ));
                }
            }
            ContainerVerdict::Fail => {
                failures.push(format!("{}: {}", FailureId::Rcg014, container.detail))
            }
        }

        let overall = if failures.is_empty() { "pass" } else { "fail" };
        let evidence = Json::Obj(
            [
                (
                    "schema".to_string(),
                    Json::str("capability_runtime_evidence.v1"),
                ),
                ("package".to_string(), Json::str(&policy.package)),
                ("gate_version".to_string(), Json::str("rcg-1")),
                (
                    "manifest_sha256".to_string(),
                    Json::str(&policy.manifest_sha256),
                ),
                ("plan_sha256".to_string(), Json::str(&policy.plan_sha256)),
                (
                    "baseline_version".to_string(),
                    Json::str(policy.baseline_version),
                ),
                (
                    "baseline".to_string(),
                    Json::Arr(
                        BASELINE
                            .iter()
                            .map(|b| {
                                Json::obj([
                                    ("syscall", Json::str(b.name)),
                                    ("why", Json::str(b.why)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "declared_capabilities".to_string(),
                    Json::strings(policy.declared.iter().cloned()),
                ),
                ("live".to_string(), Json::Bool(true)),
                ("not_live_reason".to_string(), Json::Null),
            ]
            .into_iter()
            .chain(match main_json {
                Json::Obj(pairs) => pairs,
                _ => Vec::new(),
            })
            .chain([
                ("negative_controls".to_string(), Json::Arr(control_json)),
                ("known_gaps".to_string(), Json::Arr(known_gaps)),
                ("container".to_string(), container_json),
                (
                    "environment".to_string(),
                    Json::obj([
                        ("kernel", Json::str(kernel_info())),
                        ("arch", Json::str(std::env::consts::ARCH)),
                        (
                            "runner",
                            Json::str(if std::env::var_os("GITHUB_ACTIONS").is_some() {
                                "github-actions"
                            } else {
                                "local"
                            }),
                        ),
                        (
                            "strace",
                            Json::str(
                                command_line(&strace.to_string_lossy(), &["-V"])
                                    .unwrap_or_default(),
                            ),
                        ),
                    ]),
                ),
                ("failures".to_string(), Json::strings(failures.clone())),
                ("overall".to_string(), Json::str(overall)),
            ])
            .collect(),
        );
        write_evidence(config, &evidence);
        let _ = (main_pass, observed);
        if failures.is_empty() {
            println!("RUNTIME CAPABILITY GATE: PASS ({})", config.package);
            0
        } else {
            println!("RUNTIME CAPABILITY GATE: FAIL ({})", config.package);
            for failure in &failures {
                println!("  {failure}");
            }
            1
        }
    }

    // ----- controls -------------------------------------------------------

    struct ControlResult {
        full_enforced: bool,
        full_observed: bool,
        observe_only: bool,
        enforce_only: bool,
        full_observed_name: String,
        /// Observed runs whose trace was absent or empty (`RCG013`).
        trace_problems: Vec<(&'static str, &'static str)>,
    }

    impl ControlResult {
        fn all_detected(&self) -> bool {
            self.full_enforced && self.full_observed && self.observe_only && self.enforce_only
        }

        fn to_json(&self, name: &str, expected: &[&str]) -> Json {
            Json::obj([
                ("name", Json::str(name)),
                ("expected_syscall", Json::strings(expected.iter().copied())),
                (
                    "full",
                    Json::obj([
                        ("detected_by_enforcement", Json::Bool(self.full_enforced)),
                        ("detected_by_observation", Json::Bool(self.full_observed)),
                        ("failure_id", Json::str("RCG001")),
                        ("observed", Json::str(&self.full_observed_name)),
                    ]),
                ),
                (
                    "observe_only",
                    Json::obj([
                        ("detected", Json::Bool(self.observe_only)),
                        ("failure_id", Json::str("RCG002")),
                    ]),
                ),
                (
                    "enforce_only",
                    Json::obj([
                        ("detected", Json::Bool(self.enforce_only)),
                        ("failure_id", Json::str("RCG001")),
                    ]),
                ),
            ])
        }
    }

    fn observed_expected(analysis: &Analysis, expected: &[&str]) -> Option<String> {
        analysis
            .violations
            .iter()
            .find(|v| expected.contains(&v.name.as_str()))
            .map(|v| v.name.clone())
    }

    fn run_control(
        runner: &Runner<'_>,
        policy: &RuntimePolicy,
        control: &sabotage::Control,
    ) -> ControlResult {
        let mut result = ControlResult {
            full_enforced: false,
            full_observed: false,
            observe_only: false,
            enforce_only: false,
            full_observed_name: String::new(),
            trace_problems: Vec::new(),
        };
        // Full: filter on + strace.
        let nonce = random_nonce("full");
        if let Ok(outcome) = runner.run(&nonce, true, Some(control.name), &[], true) {
            if let Some(why) = trace_problem(&outcome) {
                result.trace_problems.push(("full", why));
            }
            let analysis = analyze(&outcome.trace.clone().unwrap_or_default(), &nonce, policy);
            let judgement = judge(&analysis, &outcome.status, true, policy);
            result.full_enforced = judgement.killed_by_sigsys
                && !outcome.stdout.contains("SABOTAGE-SURVIVED")
                && judgement.ids.contains(&FailureId::Rcg001);
            if let Some(name) = observed_expected(&analysis, control.expected) {
                result.full_observed = true;
                result.full_observed_name = name;
            }
        }
        // Observe-only: filter off, strace on. The attempt must go through
        // (the process survives) AND the observer must still flag it.
        let nonce = random_nonce("obs");
        if let Ok(outcome) = runner.run(&nonce, false, Some(control.name), &[], true) {
            if let Some(why) = trace_problem(&outcome) {
                result.trace_problems.push(("observe-only", why));
            }
            let analysis = analyze(&outcome.trace.clone().unwrap_or_default(), &nonce, policy);
            let judgement = judge(&analysis, &outcome.status, false, policy);
            result.observe_only = observed_expected(&analysis, control.expected).is_some()
                && judgement.ids.contains(&FailureId::Rcg002)
                && !judgement.killed_by_sigsys;
        }
        // Enforce-only: filter on, no strace.
        let nonce = random_nonce("enf");
        if let Ok(outcome) = runner.run(&nonce, true, Some(control.name), &[], false) {
            result.enforce_only = outcome.status == ChildStatus::Signaled(SIGSYS)
                && !outcome.stdout.contains("SABOTAGE-SURVIVED");
        }
        let _ = std::fs::remove_file(format!("/tmp/rcg-sabotage-{nonce}"));
        result
    }

    // ----- container ring --------------------------------------------------

    #[derive(PartialEq, Eq)]
    enum ContainerVerdict {
        Pass,
        Fail,
        NotLive,
    }

    struct ContainerResult {
        verdict: ContainerVerdict,
        detail: String,
        invocation: Vec<String>,
        control_detected: Option<bool>,
    }

    impl ContainerResult {
        fn to_json(&self) -> Json {
            Json::obj([
                (
                    "live",
                    Json::Bool(self.verdict != ContainerVerdict::NotLive),
                ),
                (
                    "verdict",
                    Json::str(match self.verdict {
                        ContainerVerdict::Pass => "pass",
                        ContainerVerdict::Fail => "fail",
                        ContainerVerdict::NotLive => "not-live",
                    }),
                ),
                ("detail", Json::str(&self.detail)),
                ("invocation", Json::strings(self.invocation.iter().cloned())),
                (
                    "negative_control_io_uring_killed_by_container_filter",
                    match self.control_detected {
                        Some(b) => Json::Bool(b),
                        None => Json::Null,
                    },
                ),
            ])
        }
    }

    fn docker_ok(docker: &Path, args: &[&str]) -> Result<std::process::Output, String> {
        Command::new(docker)
            .args(args)
            .output()
            .map_err(|e| format!("docker {args:?}: {e}"))
    }

    fn container_ring(config: &GateConfig, policy: &RuntimePolicy, exe: &Path) -> ContainerResult {
        let not_live = |detail: String| ContainerResult {
            verdict: ContainerVerdict::NotLive,
            detail,
            invocation: Vec::new(),
            control_detected: None,
        };
        let Some(docker) = which("docker") else {
            return not_live("docker binary not found".into());
        };
        match docker_ok(&docker, &["info", "--format", "{{.ServerVersion}}"]) {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                return not_live(format!(
                    "docker daemon unreachable: {}",
                    String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .next()
                        .unwrap_or("")
                ))
            }
            Err(error) => return not_live(error),
        }
        let work_guard = match ScratchDir::new(config, "rcg-container") {
            Ok(guard) => guard,
            Err(error) => return not_live(format!("cannot create container work dir: {error}")),
        };
        let work = work_guard.path().to_path_buf();
        let rootfs = work.join("rootfs");
        if let Err(error) = std::fs::create_dir_all(rootfs.join("rcg")) {
            return not_live(format!("cannot create {}: {error}", work.display()));
        }
        // Rootfs: the test binary and its ldd closure, nothing else.
        let result = (|| -> Result<(), String> {
            std::fs::copy(exe, rootfs.join("rcg/test")).map_err(|e| format!("copy exe: {e}"))?;
            let ldd = Command::new("ldd")
                .arg(exe)
                .output()
                .map_err(|e| format!("ldd: {e}"))?;
            for lib in parse_ldd(&String::from_utf8_lossy(&ldd.stdout)) {
                let dest = rootfs.join(lib.trim_start_matches('/'));
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
                }
                std::fs::copy(&lib, &dest).map_err(|e| format!("copy {lib}: {e}"))?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            return not_live(error);
        }
        let tar_path = work.join("rootfs.tar");
        let tar = Command::new("tar")
            .arg("-C")
            .arg(&rootfs)
            .arg("-cf")
            .arg(&tar_path)
            .arg(".")
            .status();
        if !tar.map(|s| s.success()).unwrap_or(false) {
            return not_live("tar failed".into());
        }
        let nonce = random_nonce("c");
        let image = format!("rcg-{nonce}:1");
        match docker_ok(&docker, &["import", &tar_path.to_string_lossy(), &image]) {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                return not_live(format!(
                    "docker import failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ))
            }
            Err(error) => return not_live(error),
        }
        let profile_path = work.join("seccomp-profile.json");
        let _ = std::fs::write(&profile_path, seccomp_profile_json(policy));

        let run_in_container = |extra: &[&str],
                                tag: &str|
         -> (Vec<String>, Option<std::process::Output>) {
            let mut command = vec!["/rcg/test".to_string(), "--rcg-child".into(), nonce.clone()];
            command.extend(extra.iter().map(ToString::to_string));
            command.push("--rcg-container".into());
            let args = docker_args(
                policy,
                &profile_path.to_string_lossy(),
                &image,
                &format!("rcg-{tag}-{nonce}"),
                &command,
            );
            let output = Command::new(&docker).args(&args).env_clear().output().ok();
            (args, output)
        };

        // Real run inside the container: in-process arming still happens.
        let (invocation, output) = run_in_container(&["enforce"], "main");
        let mut detail = String::new();
        let mut verdict = ContainerVerdict::Fail;
        if let Some(out) = &output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let ok_count = stdout.lines().filter(|l| l.starts_with("ok ")).count();
            if out.status.success() && ok_count > 0 && stderr.contains("RCG-ARMED") {
                verdict = ContainerVerdict::Pass;
                detail = format!("container ran {ok_count} checks armed; exit 0");
            } else {
                detail = format!(
                    "container run failed: status {:?}, ok={ok_count}, stderr: {}",
                    out.status.code(),
                    stderr.lines().take(3).collect::<Vec<_>>().join(" | ")
                );
            }
        } else {
            detail.push_str("docker run could not be spawned");
        }
        // Container-ring negative control: without the in-process filter,
        // an io_uring_setup() attempt must be killed by the *container's* profile.
        let (_, control) = run_in_container(&["observe-only", "--rcg-sabotage", "io-uring"], "ctl");
        let control_detected = control.as_ref().map(|out| {
            !String::from_utf8_lossy(&out.stdout).contains("SABOTAGE-SURVIVED")
                && out.status.code() == Some(128 + SIGSYS)
        });
        if control_detected != Some(true) && verdict == ContainerVerdict::Pass {
            verdict = ContainerVerdict::Fail;
            detail = format!(
                "container negative control not detected (status {:?})",
                control.as_ref().and_then(|o| o.status.code())
            );
        }
        let _ = docker_ok(&docker, &["rmi", "-f", &image]);
        drop(work_guard);
        ContainerResult {
            verdict,
            detail,
            invocation: std::iter::once("docker".to_string())
                .chain(invocation)
                .collect(),
            control_detected,
        }
    }

    // In-process tests of the parent-side and pre-arm helpers. The armed child
    // body (`child_main`) cannot report coverage (it ends in `_exit` inside a
    // seccomp sandbox); its behaviour is asserted end to end by the
    // `selftest_*` targets instead.
    #[cfg(test)]
    mod tests {
        use super::*;

        fn pure_config(dir: PathBuf, manifest: &'static str) -> GateConfig {
            GateConfig {
                package: "rust/harness-unit",
                manifest_json: manifest,
                evidence_dir: Some(dir),
                prepare: || Box::new(Vec::new),
                options: PolicyOptions::default(),
                rings: Rings::MAIN_ONLY,
            }
        }

        fn scratch(name: &str) -> PathBuf {
            let dir = std::env::temp_dir()
                .join(format!("rcg-harness-unit-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            dir
        }

        #[test]
        fn launcher_closes_leaked_non_cloexec_descriptors_before_exec() {
            // Plain pipe: no O_CLOEXEC, exactly what a CI runner leaks.
            let mut pipe = [0i32; 2];
            // SAFETY: pipe2 with flags 0 into a live two-element array; the
            // descriptor is then moved high (F_DUPFD, no CLOEXEC) so `ls`'s
            // own directory handle cannot reuse its number.
            let leaked = unsafe {
                assert_eq!(libc::pipe2(pipe.as_mut_ptr(), 0), 0);
                let high = libc::fcntl(pipe[0], libc::F_DUPFD, 200);
                libc::close(pipe[0]);
                libc::close(pipe[1]);
                high
            };
            assert!(leaked >= 200);
            let mut command = Command::new("ls");
            command
                .arg("/proc/self/fd")
                .env_clear()
                .env("PATH", "/usr/bin:/bin");
            // SAFETY: the closure only calls async-signal-safe functions.
            unsafe {
                command.pre_exec(|| {
                    close_inherited_fds();
                    Ok(())
                });
            }
            let output = command.output().expect("spawn ls");
            // SAFETY: closing the descriptor created above.
            unsafe {
                libc::close(leaked);
            }
            let listing = String::from_utf8_lossy(&output.stdout);
            let open: Vec<i32> = listing
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .filter(|fd| *fd > 2)
                .collect();
            assert!(!open.contains(&leaked), "{listing}");
        }

        #[test]
        fn unexpected_fds_reports_an_open_descriptor_above_stderr() {
            let file = std::fs::File::open("/proc/self/exe").unwrap();
            let found = unexpected_fds();
            assert!(!found.is_empty(), "{found:?}");
            drop(file);
        }

        #[test]
        fn not_live_writes_honest_evidence_and_only_fails_when_strict() {
            let dir = scratch("notlive");
            let config = pure_config(dir.clone(), "{}");
            let code = not_live(&config, "unit test reason", FailureId::Rcg013);
            assert_eq!(code, i32::from(strict()));
            let evidence =
                std::fs::read_to_string(dir.join("capability_runtime_evidence.json")).unwrap();
            assert!(evidence.contains("\"overall\": \"not-live\""), "{evidence}");
            assert!(evidence.contains("unit test reason"));
            assert!(evidence.contains("\"live\": false"));
            let _ = std::fs::remove_dir_all(dir);
        }

        #[test]
        fn evidence_write_failure_is_reported_not_fatal() {
            let blocker = scratch("blocker");
            std::fs::write(&blocker, b"a file, not a directory").unwrap();
            let config = pure_config(blocker.join("sub"), "{}");
            write_evidence(&config, &Json::obj([("k", Json::str("v"))]));
            let _ = std::fs::remove_file(blocker);
        }

        #[test]
        fn a_manifest_the_planner_rejects_is_refused_with_evidence() {
            let dir = scratch("refused");
            let config = pure_config(dir.clone(), "{ not json");
            assert_eq!(run(&config), 1);
            let evidence =
                std::fs::read_to_string(dir.join("capability_runtime_evidence.json")).unwrap();
            assert!(evidence.contains("\"overall\": \"fail\""), "{evidence}");
            assert!(evidence.contains("RCG006"), "{evidence}");
            let _ = std::fs::remove_dir_all(dir);
        }

        // ---- private working directories (symlink safety, cleanup) ----

        #[test]
        fn private_dir_refuses_a_preplanted_symlink_and_never_writes_through_it() {
            let base = scratch("symlink");
            std::fs::create_dir_all(&base).unwrap();
            let victim = base.join("victim");
            std::fs::create_dir_all(&victim).unwrap();
            let link = base.join("rcg-scratch-planted");
            std::os::unix::fs::symlink(&victim, &link).unwrap();
            assert!(create_private_dir(&link).is_err(), "followed a symlink");
            std::fs::write(victim.join("sentinel"), b"x").unwrap();
            assert!(std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink());
            let entries: Vec<_> = std::fs::read_dir(&victim).unwrap().collect();
            assert_eq!(entries.len(), 1, "something was created through the link");
            let _ = std::fs::remove_dir_all(base);
        }

        #[test]
        fn private_dir_refuses_an_existing_file_and_an_existing_directory() {
            let base = scratch("existing");
            std::fs::create_dir_all(&base).unwrap();
            let file = base.join("a-file");
            std::fs::write(&file, b"keep").unwrap();
            assert!(create_private_dir(&file).is_err());
            assert_eq!(std::fs::read(&file).unwrap(), b"keep");
            let dir = base.join("a-dir");
            std::fs::create_dir(&dir).unwrap();
            assert!(create_private_dir(&dir).is_err(), "reused a directory");
            let _ = std::fs::remove_dir_all(base);
        }

        #[test]
        fn private_dir_is_mode_0700() {
            use std::os::unix::fs::PermissionsExt;
            let base = scratch("mode");
            let path = base.join("fresh");
            create_private_dir(&path).unwrap();
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o700, "mode {mode:o}");
            let _ = std::fs::remove_dir_all(base);
        }

        #[test]
        fn private_dir_names_are_unpredictable_but_carry_the_pid() {
            let config = pure_config(scratch("names"), "{}");
            let a = private_dir(&config, "rcg-scratch");
            let b = private_dir(&config, "rcg-scratch");
            assert_ne!(a, b, "the name must not be predictable from the pid alone");
            let pid = std::process::id().to_string();
            assert!(a.file_name().unwrap().to_string_lossy().contains(&pid));
        }

        #[test]
        fn scratch_guard_removes_the_directory_on_drop_and_on_panic() {
            let config = pure_config(scratch("guard"), "{}");
            let guard = ScratchDir::new(&config, "rcg-scratch").unwrap();
            let path = guard.path().to_path_buf();
            std::fs::write(path.join("trace-x.txt"), b"x").unwrap();
            drop(guard);
            assert!(!path.exists(), "drop left {}", path.display());

            let guard = ScratchDir::new(&config, "rcg-scratch").unwrap();
            let path = guard.path().to_path_buf();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _guard = guard;
                panic!("simulated panic in the gate");
            }));
            assert!(result.is_err());
            assert!(!path.exists(), "unwinding left {}", path.display());
            let _ = std::fs::remove_dir_all(config.evidence_dir.unwrap());
        }

        #[test]
        fn no_os_randomness_fails_closed_with_not_live_rcg013() {
            const PURE: &str = r#"{"version":1,"package":"rust/harness-unit","capabilities":[],"justification":"unit"}"#;
            let dir = scratch("norandom");
            let config = pure_config(dir.clone(), PURE);
            RANDOM_FAILS.with(|f| f.set(true));
            let created = ScratchDir::new(&config, "rcg-scratch");
            let code = run(&config);
            RANDOM_FAILS.with(|f| f.set(false));
            let err = created.err().expect("weak name accepted").to_string();
            assert!(err.contains("no OS randomness"), "{err}");
            assert_eq!(code, i32::from(strict()));
            let evidence =
                std::fs::read_to_string(dir.join("capability_runtime_evidence.json")).unwrap();
            assert!(evidence.contains("\"overall\": \"not-live\""), "{evidence}");
            assert!(evidence.contains("RCG013"), "{evidence}");
            assert!(evidence.contains("no OS randomness"), "{evidence}");
            let left: Vec<_> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("rcg-scratch"))
                .collect();
            assert!(left.is_empty(), "left {left:?}");
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
