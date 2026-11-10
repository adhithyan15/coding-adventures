//! # Running tests: order, seeds, isolation and TAP (TF01 §5, §6)
//!
//! A [`Suite`] is a list of named tests. Running it:
//!
//! ```text
//!   ids ──sort by bytes──► shuffle with SplitMix64(run seed) ──► run each:
//!        Ctx { seed: sub_seed(run seed, id) }  →  body(&ctx)  →  Verdict
//!        a panic is caught and becomes Fail("panic")
//!   ──► TAP v14 on stdout; exit 0 if every test passed, else 1
//! ```
//!
//! A test binary built on this crate is a plain `fn main()` (Cargo's
//! `harness = false`) that fills a suite and hands it to [`main`].

use std::collections::HashMap;
use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::prng::{shuffled, sub_seed};
use crate::tfv::Value;
use crate::verdict::{self, Verdict};

/// The lane name printed in `# tf-lane:`.
pub const LANE: &str = "rust";

/// Exit status for a command line the harness does not understand.
pub const USAGE_EXIT: i32 = 2;

/// Exit status when the report has nowhere to go: file descriptor 1 is closed,
/// read-only, or otherwise cannot carry the TAP stream (LH-003c).
pub const STDOUT_UNUSABLE_EXIT: i32 = 1;

// ---------------------------------------------------------------------------
// Is standard output a place a report can actually go?
//
// Rust's `std::io::Stdout` treats the OS error EBADF ("bad file descriptor")
// as success: a write to a closed or read-only fd 1 silently reports `Ok`, on
// purpose, so that programs run without a stdout do not crash. For a test
// harness that is the worst possible default: nothing is printed, nothing is
// checked, the exit status is 0, and a supervisor reads that as a pass.
//
// std never tells us, so we ask the OS directly before running anything:
//
//   Unix     fcntl(1, F_GETFL) fails with EBADF when fd 1 is closed; when it
//            succeeds, the low two bits of the flags are the access mode
//            (O_RDONLY = 0, O_WRONLY = 1, O_RDWR = 2). Read-only is rejected.
//   Windows  GetStdHandle(STD_OUTPUT_HANDLE) is NULL or INVALID_HANDLE_VALUE
//            when there is no stdout; GetFileType reports an error for a
//            handle that is not usable.
//
// What this does NOT detect is written down in TF01 section 8: a stdout that
// accepts bytes and discards them (/dev/null, a pipe nobody reads yet), an fd 1
// that is closed *after* this check, and (Windows) a read-only handle.
// ---------------------------------------------------------------------------

/// Verify that this process's standard output is open and writable.
///
/// Returns `Err` describing the problem when fd 1 (Unix) or the standard
/// output handle (Windows) is closed, missing, or not open for writing.
pub fn stdout_is_usable() -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
    {
        // Naming the static here keeps the object that holds the constructor
        // table entry in the final link.
        std::hint::black_box(&early::INIT);
        // `None` (the constructor never ran) deliberately falls through to the
        // in-`main` probe below rather than failing: Miri and other
        // interpreters do not run loader constructors, and refusing there
        // would make the harness unusable under them. On a real binary the
        // entry is always linked (above) and always run, and the
        // `a_closed_stdout_exits_one_not_zero` test proves it on every
        // platform this block compiles for.
        if early::stdout_flags_at_start() == Some(-1) {
            return Err(io::Error::other(
                "closed when the process started (std re-opens a closed fd 1 as /dev/null, \
                 which would swallow the whole report)",
            ));
        }
    }
    #[cfg(unix)]
    {
        fd_is_writable(1)
    }
    #[cfg(windows)]
    {
        windows_stdout_is_usable()
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(())
    }
}

/// `fcntl(fd, F_GETFL)`: the open-file flags of `fd`, or the OS error
/// (EBADF when `fd` is closed).
#[cfg(unix)]
fn get_fd_flags(fd: i32) -> io::Result<i32> {
    use std::ffi::c_int;
    extern "C" {
        // POSIX: int fcntl(int fd, int cmd, ...). Declared variadic as in C.
        fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    }
    // F_GETFL = 3 on both Linux and macOS.
    const F_GETFL: c_int = 3;
    // SAFETY: `fcntl(fd, F_GETFL)` takes no pointer arguments and only reads
    // kernel state for `fd`; an invalid `fd` makes it return -1 with EBADF,
    // which is exactly the condition being probed, never undefined behaviour.
    let flags = unsafe { fcntl(fd as c_int, F_GETFL) };
    if flags < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(flags)
    }
}

/// Is file descriptor `fd` open for writing? (Unix.)
#[cfg(unix)]
pub fn fd_is_writable(fd: i32) -> io::Result<()> {
    // O_ACCMODE = 3 and O_RDONLY = 0 on both Linux and macOS.
    const O_ACCMODE: i32 = 3;
    const O_RDONLY: i32 = 0;
    // Linux only: an O_PATH descriptor reports access mode 0 but cannot be
    // read or written at all.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const O_PATH: i32 = 0o10_000_000;

    let flags = get_fd_flags(fd)?;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if flags & O_PATH != 0 {
        return Err(io::Error::other("opened O_PATH, which cannot be written"));
    }
    if flags & O_ACCMODE == O_RDONLY {
        return Err(io::Error::other("opened read-only"));
    }
    Ok(())
}

// A closed fd 1 is invisible by the time `main` runs. Rust's runtime start-up
// (std's `sanitize_standard_fds`) opens /dev/null over any closed fd 0, 1 or
// 2 before calling main, precisely so that a stray `open()` cannot be handed
// fd 1 and have the program's output written into it. After that repair a
// closed stdout looks exactly like `> /dev/null`, which is a working,
// writable, discarding stdout.
//
// To see the original state we have to look *before* std's start-up code.
// Every mainstream loader has a table of constructor functions that it runs
// before `main` is entered, and std's repair happens inside `main`, so a
// one-function entry in that table samples fd 1 first. It does nothing but
// read the flags and store them in an atomic.
//
//   Platform         Constructor table                 Run by
//   ---------------  --------------------------------  ----------------------
//   Linux, Android   ELF section `.init_array`         ld.so / the libc start
//   macOS            Mach-O `__DATA,__mod_init_func`   dyld
//
// The BSDs and illumos also use `.init_array`, and iOS has `__mod_init_func`,
// but no CI job runs them, so they are left out rather than claimed untested.
// On those, a closed fd 1 at start-up is a documented known limit (TF01
// section 8.3).
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
mod early {
    use std::sync::atomic::{AtomicI32, Ordering};

    const UNSET: i32 = i32::MIN;
    static STDOUT_FLAGS: AtomicI32 = AtomicI32::new(UNSET);

    extern "C" fn sample_stdout_before_main() {
        // -1 records "closed" (fcntl failed).
        STDOUT_FLAGS.store(super::get_fd_flags(1).unwrap_or(-1), Ordering::Relaxed);
    }

    // The constructor table entry. `#[used]` stops the compiler discarding it.
    // On ELF, `.init_array` is a KEEP section in the linker script, so
    // --gc-sections keeps it too; on Mach-O, `ld64` never dead-strips
    // `__mod_init_func`, whose section type (S_MOD_INIT_FUNC_POINTERS) dyld
    // reads as "call each pointer before main". Its address is referenced
    // from `stdout_is_usable` so the object file holding it is always linked in.
    #[used]
    #[cfg_attr(
        any(target_os = "linux", target_os = "android"),
        link_section = ".init_array"
    )]
    #[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
    pub static INIT: extern "C" fn() = sample_stdout_before_main;

    /// The flags of fd 1 sampled before `main`; `Some(-1)` means it was
    /// closed, `None` means the sample never ran (e.g. a static analysis
    /// harness, or a loader that skips its constructor table).
    pub fn stdout_flags_at_start() -> Option<i32> {
        match STDOUT_FLAGS.load(Ordering::Relaxed) {
            UNSET => None,
            flags => Some(flags),
        }
    }
}

#[cfg(windows)]
fn windows_stdout_is_usable() -> io::Result<()> {
    use std::ffi::c_void;
    extern "system" {
        fn GetStdHandle(std_handle: u32) -> *mut c_void;
        fn GetFileType(handle: *mut c_void) -> u32;
        fn GetLastError() -> u32;
    }
    // (DWORD)-11, the documented STD_OUTPUT_HANDLE.
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const FILE_TYPE_UNKNOWN: u32 = 0;
    const NO_ERROR: u32 = 0;

    // SAFETY: GetStdHandle only reads the process's standard handle table.
    let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    // NULL: the process has no standard output (e.g. a detached GUI process).
    // INVALID_HANDLE_VALUE (-1): the lookup itself failed.
    if handle.is_null() || handle as isize == -1 {
        return Err(io::Error::other("no standard output handle"));
    }
    // SAFETY: GetFileType only inspects `handle`, which GetStdHandle returned
    // and is not NULL or INVALID_HANDLE_VALUE; GetLastError reads thread state.
    let (kind, error) = unsafe { (GetFileType(handle), GetLastError()) };
    if kind == FILE_TYPE_UNKNOWN && error != NO_ERROR {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    Ok(())
}

/// How to run a suite, from the command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// The run seed: it fixes the order and every test's sub-seed.
    pub seed: u64,
    /// `--sabotage always-pass`: every verdict from the core becomes `Pass`
    /// (TF01 §6). Corpus cases that expect a failure then disagree with the
    /// core, so a working harness reports them as `not ok`. The BUILD checks
    /// that from outside, proving the harness can report failure at all.
    pub sabotage: bool,
}

/// Read the command line (without the program name).
///
/// ```text
///   []                              → seed 0, no sabotage
///   ["--seed", "42"]                → seed 42
///   ["--sabotage", "always-pass"]   → sabotage on
///   ["--nocapture"]                 → ignored (Cargo forwards it to every test binary)
///   anything else                   → Err, and the binary exits 2
/// ```
pub fn parse_args<I, S>(args: I) -> Result<Options, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "--seed" => {
                let value = args.next().ok_or("--seed needs a value")?;
                let value = value.as_ref();
                // Digits only: `str::parse` would also accept a leading `+`.
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(format!("--seed needs a decimal u64, got {value:?}"));
                }
                options.seed = value
                    .parse()
                    .map_err(|_| format!("--seed {value:?} does not fit in a u64"))?;
            }
            "--sabotage" => match args.next() {
                Some(mode) if mode.as_ref() == "always-pass" => options.sabotage = true,
                _ => return Err("--sabotage needs the mode always-pass".into()),
            },
            "--nocapture" => {}
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(options)
}

/// What a test body is given: its own seed, and the verdict core.
///
/// Tests reach the core through `Ctx` rather than calling
/// [`verdict`](crate::verdict) directly, so that sabotage mode can stand
/// between the two.
pub struct Ctx {
    seed: u64,
    sabotage: bool,
}

impl Ctx {
    /// This test's sub-seed, for any randomness the test needs.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// [`verdict::deep_equal`], or `Pass` under sabotage.
    pub fn deep_equal(&self, actual: &Value, expected: &Value) -> Verdict {
        if self.sabotage {
            return Verdict::Pass;
        }
        verdict::deep_equal(actual, expected)
    }

    /// [`verdict::error_is`], or `Pass` under sabotage.
    pub fn error_is(&self, raised: Option<&str>, expected_id: &str) -> Verdict {
        if self.sabotage {
            return Verdict::Pass;
        }
        verdict::error_is(raised, expected_id)
    }
}

type Body = Box<dyn Fn(&Ctx) -> Verdict>;

/// A set of uniquely named tests.
#[derive(Default)]
pub struct Suite {
    tests: Vec<(String, Body)>,
    /// Registration mistakes. They are reported when the suite runs, before
    /// any test does, so one bad id cannot hide behind a passing run.
    problems: Vec<String>,
}

/// One test's verdict as the runner reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub id: String,
    pub verdict: Verdict,
}

impl Outcome {
    /// True when the runner reported `ok` for this test.
    pub fn passed(&self) -> bool {
        matches!(self.verdict, Verdict::Pass)
    }
}

/// The tally of one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub passed: usize,
    pub failed: usize,
    /// True when registration or a runtime protocol error stopped the run.
    pub bailed: bool,
}

impl Summary {
    /// 0 when every test passed, else 1.
    pub fn exit_code(&self) -> i32 {
        if self.bailed || self.failed > 0 {
            1
        } else {
            0
        }
    }
}

impl Suite {
    pub fn new() -> Self {
        Suite::default()
    }

    /// Register a test. Ids match `[A-Za-z0-9._/-]+` and must be unique;
    /// a bad or repeated id makes the whole run bail out.
    pub fn add<F>(&mut self, id: impl Into<String>, body: F)
    where
        F: Fn(&Ctx) -> Verdict + 'static,
    {
        let id = id.into();
        let well_formed = !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-'));
        if !well_formed {
            self.problems.push(format!("invalid test id {id:?}"));
        } else if self.tests.iter().any(|(seen, _)| *seen == id) {
            self.problems.push(format!("duplicate test id {id:?}"));
        } else {
            self.tests.push((id, Box::new(body)));
        }
    }

    /// Run tests in shuffled order, writing TAP v14 to `out`.
    /// A malformed callback verdict bails out before it can escape as invalid TAP.
    pub fn run(&self, options: &Options, out: &mut dyn Write) -> io::Result<Summary> {
        self.run_recorded(options, out).map(|(summary, _)| summary)
    }

    /// [`Suite::run`], but also return every test's verdict in execution
    /// order. Evidence manifests (VER04h) are built from these outcomes, so a
    /// manifest records what the runner actually reported rather than what the
    /// test body believed.
    pub fn run_recorded(
        &self,
        options: &Options,
        out: &mut dyn Write,
    ) -> io::Result<(Summary, Vec<Outcome>)> {
        let mut outcomes = Vec::new();
        writeln!(out, "TAP version 14")?;
        writeln!(out, "# tf-lane: {LANE}")?;
        writeln!(out, "# tf-seed: {}", options.seed)?;
        let mut summary = Summary {
            passed: 0,
            failed: 0,
            bailed: false,
        };
        if !self.problems.is_empty() {
            // Ids are printed with {:?}, so no newline can escape this line.
            writeln!(out, "Bail out! {}", self.problems.join("; "))?;
            summary.bailed = true;
            return Ok((summary, outcomes));
        }
        let bodies: HashMap<&str, &Body> = self
            .tests
            .iter()
            .map(|(id, body)| (id.as_str(), body))
            .collect();
        let ids: Vec<&str> = self.tests.iter().map(|(id, _)| id.as_str()).collect();
        let order = shuffled(options.seed, &ids);
        // The plan is written with an explicit write_all + flush, and its
        // result is propagated, so a stdout that rejects the report (ENOSPC,
        // EPIPE, ...) turns the whole run into a failure at the first line a
        // supervisor relies on. Flushing here also puts the plan on the wire
        // before any test runs, so a test that later calls process::exit(0)
        // still leaves "1..N" behind for a supervisor to compare against.
        out.write_all(format!("1..{}\n", order.len()).as_bytes())?;
        out.flush()?;
        for (index, id) in order.iter().enumerate() {
            let body = bodies[id.as_str()];
            let ctx = Ctx {
                seed: sub_seed(options.seed, id),
                sabotage: options.sabotage,
            };
            // A panic is this test's failure, not the run's end. The panic's
            // own message goes to stderr through the default hook; TAP gets
            // only a fixed text, because a panic message may quote inputs.
            let verdict = catch_unwind(AssertUnwindSafe(|| body(&ctx)))
                .unwrap_or_else(|_| Verdict::fail("panic", "the test panicked"));
            let number = index + 1;
            outcomes.push(Outcome {
                id: id.clone(),
                verdict: verdict.clone(),
            });
            match verdict {
                Verdict::Pass => {
                    summary.passed += 1;
                    writeln!(out, "ok {number} - {id}")?;
                }
                Verdict::Fail {
                    failure_id,
                    message,
                } => {
                    // Validate the observable boundary, not just the public
                    // convenience constructor: callbacks can build Fail directly.
                    // A fixed bail-out reveals neither the bad ID nor its message.
                    if !verdict::is_registered_failure_id(failure_id) {
                        writeln!(out, "Bail out! invalid failure id")?;
                        summary.bailed = true;
                        return Ok((summary, outcomes));
                    }
                    summary.failed += 1;
                    writeln!(out, "not ok {number} - {id}")?;
                    writeln!(out, "  ---")?;
                    writeln!(out, "  failure_id: {}", yaml_quote(failure_id))?;
                    writeln!(out, "  message: {}", yaml_quote(&message))?;
                    writeln!(out, "  ...")?;
                }
            }
        }
        Ok((summary, outcomes))
    }
}

/// Run `suite` with this process's arguments and return the exit status.
///
/// ```ignore
/// fn main() {
///     let mut suite = Suite::new();
///     suite.add("math/two-plus-two", |ctx| {
///         ctx.deep_equal(&Value::from_u64(2 + 2), &Value::from_u64(4))
///     });
///     std::process::exit(run::main(&suite));
/// }
/// ```
pub fn main(suite: &Suite) -> i32 {
    main_recorded(suite, |_| ())
}

/// [`main`], handing every test's verdict to `finish` once the run is over
/// and before the exit status is returned. `finish` is where a harness writes
/// its evidence manifest (see [`crate::manifest`]). It is not called when the
/// command line is rejected or stdout closes early, because no complete run
/// exists to record then.
pub fn main_recorded(suite: &Suite, finish: impl FnOnce(&[Outcome])) -> i32 {
    let mut args = Vec::new();
    for arg in std::env::args_os().skip(1) {
        match arg.into_string() {
            Ok(arg) => args.push(arg),
            Err(_) => {
                eprintln!("test-bootstrap: arguments must be UTF-8");
                return USAGE_EXIT;
            }
        }
    }
    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(problem) => {
            eprintln!("test-bootstrap: {problem}");
            eprintln!("usage: <test binary> [--seed <u64>] [--sabotage always-pass]");
            return USAGE_EXIT;
        }
    };
    // Before anything runs: if fd 1 is closed or read-only, std would swallow
    // every write and the run would exit 0 having reported nothing.
    if let Err(problem) = stdout_is_usable() {
        eprintln!("test-bootstrap: standard output is not writable ({problem}); refusing to run");
        return STDOUT_UNUSABLE_EXIT;
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match suite
        .run_recorded(&options, &mut out)
        .and_then(|s| out.flush().map(|()| s))
    {
        Ok((summary, outcomes)) => {
            finish(&outcomes);
            summary.exit_code()
        }
        // stdout closed early (say, `| grep -q`): the run cannot be reported,
        // so it cannot count as a pass.
        Err(_) => 1,
    }
}

/// A YAML double-quoted scalar. Quotes, backslashes and every control or
/// line-breaking character are escaped, so a message can never end the
/// YAML block early or start a new TAP line.
fn yaml_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}') => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(suite: &Suite, options: &Options) -> (String, Summary) {
        let mut buffer = Vec::new();
        let summary = suite.run(options, &mut buffer).unwrap();
        (String::from_utf8(buffer).unwrap(), summary)
    }

    fn tally(suite: &Suite) -> Summary {
        output(suite, &Options::default()).1
    }

    #[test]
    fn run_recorded_returns_each_verdict_by_id() {
        let mut suite = Suite::new();
        suite.add("good", |_| Verdict::Pass);
        suite.add("bad", |ctx| ctx.error_is(None, "E"));
        let mut buffer = Vec::new();
        let (summary, outcomes) = suite
            .run_recorded(&Options::default(), &mut buffer)
            .unwrap();
        assert_eq!((summary.passed, summary.failed), (1, 1));
        assert_eq!(outcomes.len(), 2);
        let good = outcomes.iter().find(|o| o.id == "good").unwrap();
        let bad = outcomes.iter().find(|o| o.id == "bad").unwrap();
        assert!(good.passed());
        assert!(!bad.passed());
        // A bailed run (duplicate id) records nothing and still reports.
        let mut dup = Suite::new();
        dup.add("x", |_| Verdict::Pass);
        dup.add("x", |_| Verdict::Pass);
        let (summary, outcomes) = dup.run_recorded(&Options::default(), &mut Vec::new()).unwrap();
        assert!(summary.bailed);
        assert!(outcomes.is_empty());
    }

    #[test]
    fn arguments() {
        let none: [&str; 0] = [];
        assert_eq!(parse_args(none), Ok(Options::default()));
        assert_eq!(
            parse_args([
                "--seed",
                "18446744073709551615",
                "--nocapture",
                "--sabotage",
                "always-pass"
            ]),
            Ok(Options {
                seed: u64::MAX,
                sabotage: true
            })
        );
        for bad in [
            &["--seed"][..],
            &["--seed", "+1"],
            &["--seed", ""],
            &["--seed", "18446744073709551616"],
            &["--sabotage"],
            &["--sabotage", "always-fail"],
            &["--exact"],
        ] {
            assert!(parse_args(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn tap_shape_for_a_pass_and_a_fail() {
        let mut suite = Suite::new();
        suite.add("a", |ctx| {
            ctx.deep_equal(&Value::Bool(true), &Value::Bool(true))
        });
        suite.add("b", |ctx| ctx.error_is(None, "E"));
        let (text, summary) = output(&suite, &Options::default());
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            &lines[..4],
            ["TAP version 14", "# tf-lane: rust", "# tf-seed: 0", "1..2"]
        );
        let fail_at = lines
            .iter()
            .position(|l| *l == "not ok 1 - b" || *l == "not ok 2 - b");
        let fail_at = fail_at.expect("b must fail");
        assert_eq!(lines[fail_at + 1], "  ---");
        assert_eq!(lines[fail_at + 2], "  failure_id: \"no-error\"");
        assert_eq!(
            lines[fail_at + 3],
            "  message: \"expected error \\\"E\\\", none raised\""
        );
        assert_eq!(lines[fail_at + 4], "  ...");
        assert_eq!(
            summary,
            Summary {
                passed: 1,
                failed: 1,
                bailed: false
            }
        );
        assert_eq!(summary.exit_code(), 1);
    }

    #[test]
    fn sabotage_turns_core_failures_into_passes() {
        let mut suite = Suite::new();
        suite.add("x", |ctx| ctx.deep_equal(&Value::Null, &Value::Bool(true)));
        suite.add("y", |ctx| ctx.error_is(Some("A"), "B"));
        let sabotaged = Options {
            seed: 0,
            sabotage: true,
        };
        assert_eq!(output(&suite, &sabotaged).1.exit_code(), 0);
        assert_eq!(tally(&suite).failed, 2);
    }

    #[test]
    fn a_panic_fails_one_test_and_the_run_goes_on() {
        let mut suite = Suite::new();
        suite.add("boom", |_| panic!("secret input 1234"));
        suite.add("fine", |_| Verdict::Pass);
        let (text, summary) = output(&suite, &Options::default());
        assert_eq!(
            summary,
            Summary {
                passed: 1,
                failed: 1,
                bailed: false
            }
        );
        assert!(text.contains("failure_id: \"panic\""));
        assert!(!text.contains("1234"), "a panic message must not reach TAP");
    }

    #[test]
    fn bad_and_duplicate_ids_bail_out_before_running() {
        let mut suite = Suite::new();
        suite.add("ok", |_| panic!("must not run"));
        suite.add("ok", |_| Verdict::Pass);
        suite.add("has space\nok 1 - forged", |_| Verdict::Pass);
        suite.add("", |_| Verdict::Pass);
        let (text, summary) = output(&suite, &Options::default());
        assert!(summary.bailed);
        assert_eq!(summary.exit_code(), 1);
        let bail: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with("Bail out! "))
            .collect();
        assert_eq!(bail.len(), 1);
        assert!(!text.lines().any(|l| l.starts_with("ok ")));
    }

    #[test]
    fn same_seed_same_order_and_seeds_reach_tests() {
        let mut suite = Suite::new();
        for i in 0..20 {
            let id = format!("t{i:02}");
            let expected = sub_seed(7, &id);
            suite.add(id, move |ctx| {
                ctx.deep_equal(&Value::from_u64(ctx.seed()), &Value::from_u64(expected))
            });
        }
        let seven = Options {
            seed: 7,
            sabotage: false,
        };
        let (first, summary) = output(&suite, &seven);
        assert_eq!(summary.failed, 0);
        assert_eq!(first, output(&suite, &seven).0);
        let other = Options {
            seed: 8,
            sabotage: false,
        };
        assert_ne!(
            first.lines().nth(4),
            output(&suite, &other).0.lines().nth(4)
        );
    }

    #[test]
    fn empty_suite_passes_with_an_empty_plan() {
        let (text, summary) = output(&Suite::new(), &Options::default());
        assert!(text.ends_with("1..0\n"));
        assert_eq!(summary.exit_code(), 0);
    }

    #[test]
    fn registered_failure_ids_remain_reported_failures() {
        for failure_id in [
            "not-deep-equal",
            "no-error",
            "wrong-error",
            "panic",
            "corpus-mismatch",
            "corpus-malformed",
        ] {
            let mut suite = Suite::new();
            suite.add("registered", move |_| {
                Verdict::fail(failure_id, "ordinary detail")
            });
            let (text, summary) = output(&suite, &Options::default());
            assert_eq!(
                summary,
                Summary {
                    passed: 0,
                    failed: 1,
                    bailed: false
                }
            );
            assert_eq!(summary.exit_code(), 1);
            assert!(text.contains(&format!("failure_id: {}", yaml_quote(failure_id))));
            assert!(!text.contains("Bail out!"));
        }
    }

    #[test]
    fn invalid_constructor_and_direct_failure_ids_bail_without_leaking() {
        for failure_id in [
            "",
            "adapter-baseline",
            "adapter-mutation",
            "PANIC",
            "panic ",
            "PRIVATE_ID\nok 999 - forged",
            "pаnic",
        ] {
            for direct in [false, true] {
                let mut suite = Suite::new();
                suite.add("invalid", move |_| {
                    if direct {
                        Verdict::Fail {
                            failure_id,
                            message: "PRIVATE_MESSAGE\nok 888 - forged".into(),
                        }
                    } else {
                        Verdict::fail(failure_id, "PRIVATE_MESSAGE\nok 888 - forged")
                    }
                });
                let (text, summary) = output(&suite, &Options::default());
                assert_eq!(
                    summary,
                    Summary {
                        passed: 0,
                        failed: 0,
                        bailed: true
                    }
                );
                assert_eq!(summary.exit_code(), 1);
                assert_eq!(text.lines().last(), Some("Bail out! invalid failure id"));
                assert!(!text.contains("failure_id:"));
                assert!(!text.contains("not ok "));
                assert!(!text.contains("PRIVATE_MESSAGE"));
                assert!(!text.contains("forged"));
                if !failure_id.is_empty() {
                    assert!(!text.contains(failure_id));
                }
            }
        }
    }

    #[test]
    fn invalid_failure_stops_before_running_remaining_callbacks() {
        use std::cell::Cell;
        use std::rc::Rc;
        let calls = Rc::new(Cell::new(0));
        let mut suite = Suite::new();
        for id in ["a", "b", "c"] {
            let calls = calls.clone();
            suite.add(id, move |_| {
                calls.set(calls.get() + 1);
                Verdict::fail("unknown", "hidden")
            });
        }
        let (text, summary) = output(&suite, &Options::default());
        assert_eq!(calls.get(), 1);
        assert!(summary.bailed);
        assert_eq!(summary.passed + summary.failed, 0);
        assert_eq!(text.matches("Bail out!").count(), 1);
    }

    #[test]
    fn a_runtime_bail_preserves_a_valid_prefix_count() {
        // Choose roles after shuffling: this tests prefix accounting, not the
        // independently pinned shuffle algorithm or registration order.
        let ids = shuffled(0, &["a", "b"]);
        let mut suite = Suite::new();
        suite.add(ids[0].clone(), |_| Verdict::Pass);
        suite.add(ids[1].clone(), |_| Verdict::fail("unknown", "hidden"));
        let (text, summary) = output(&suite, &Options::default());
        assert_eq!(
            summary,
            Summary {
                passed: 1,
                failed: 0,
                bailed: true
            }
        );
        assert!(text.contains(&format!("ok 1 - {}\n", ids[0])));
        assert!(!text.contains("not ok"));
        assert_eq!(summary.exit_code(), 1);
    }

    #[test]
    fn an_invalid_id_bailout_propagates_a_writer_error() {
        struct Budget(usize);
        impl Write for Budget {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.0 == 0 {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
                }
                let count = self.0.min(bytes.len());
                self.0 -= count;
                Ok(count)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut suite = Suite::new();
        suite.add("invalid", |_| Verdict::fail("unknown", "hidden"));
        let (text, _) = output(&suite, &Options::default());
        let bytes_before_bail = text
            .find("Bail out!")
            .expect("runtime protocol error must bail");
        let result = suite.run(&Options::default(), &mut Budget(bytes_before_bail));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn yaml_quoting_cannot_break_out() {
        assert_eq!(yaml_quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(yaml_quote("x\ny\tz\r"), "\"x\\ny\\tz\\r\"");
        assert_eq!(
            yaml_quote("\u{0}\u{7f}\u{2028}"),
            "\"\\u0000\\u007f\\u2028\""
        );
        assert_eq!(yaml_quote("héllo"), "\"héllo\"");
    }

    #[cfg(unix)]
    #[test]
    fn fd_probe_accepts_writable_and_rejects_closed_or_read_only() {
        use std::fs::{File, OpenOptions};
        use std::os::fd::AsRawFd;

        let write_only = OpenOptions::new().write(true).open("/dev/null").unwrap();
        assert!(fd_is_writable(write_only.as_raw_fd()).is_ok());

        let read_write = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/null")
            .unwrap();
        assert!(fd_is_writable(read_write.as_raw_fd()).is_ok());

        let read_only = File::open("/dev/null").unwrap();
        let err = fd_is_writable(read_only.as_raw_fd()).unwrap_err();
        assert!(err.to_string().contains("read-only"), "{err}");

        // An fd number far beyond anything this process has open: EBADF.
        let err = fd_is_writable(987_654).unwrap_err();
        assert_eq!(err.raw_os_error(), Some(9), "EBADF expected, got {err}");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn fd_probe_rejects_an_o_path_descriptor() {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        const O_PATH: i32 = 0o10_000_000;
        let path_only = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_PATH)
            .open("/dev/null")
            .unwrap();
        let err = fd_is_writable(path_only.as_raw_fd()).unwrap_err();
        assert!(err.to_string().contains("O_PATH"), "{err}");
    }

    #[test]
    fn the_test_runners_own_stdout_is_usable() {
        // libtest captures output by swapping Rust-level handles, but fd 1 is
        // still the real descriptor the runner was given.
        assert!(stdout_is_usable().is_ok());
    }

    #[test]
    fn the_plan_write_error_propagates() {
        // A writer that accepts the header lines but rejects the plan line
        // (and everything after): the run must be an Err, not a quiet Ok.
        struct RejectsPlan;
        impl Write for RejectsPlan {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if bytes.starts_with(b"1..") {
                    Err(io::Error::new(io::ErrorKind::StorageFull, "no space"))
                } else {
                    Ok(bytes.len())
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut suite = Suite::new();
        suite.add("t", |_| Verdict::Pass);
        assert!(suite.run(&Options::default(), &mut RejectsPlan).is_err());
    }

    #[test]
    fn a_plan_that_cannot_be_flushed_is_an_error() {
        struct FlushFails;
        impl Write for FlushFails {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone"))
            }
        }
        assert!(Suite::new()
            .run(&Options::default(), &mut FlushFails)
            .is_err());
    }

    #[test]
    fn a_closed_stdout_is_not_a_pass() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(Suite::new().run(&Options::default(), &mut Closed).is_err());
    }
}
