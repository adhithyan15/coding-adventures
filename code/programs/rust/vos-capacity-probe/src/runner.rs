//! # `runner`: one child process per row, with a kill switch
//!
//! The probe runs hostile-sized instances on purpose (VOS30 section 9.5), so it never solves one
//! in its own process.  Each row is a child:
//!
//! ```text
//!   parent                                    child (own process group)
//!   ------                                    -------------------------
//!   /bin/sh -c 'ulimit -t "$1" || exit 126; shift; exec "$@"' sh N   (unix: the CPU backstop)
//!   /usr/bin/time -l  vos-capacity-probe row --spec <json> --wall 120
//!        |                                        |  stdout: {"progress":row}  (after the build)
//!        |  every 50 ms: resident set of the      |          {"row":row}       (the answer)
//!        |  whole group, via `ps`                 |
//!        |  > 2 GiB          -> kill group, oom   |
//!        |  > wall + slack   -> kill group, killed|
//!        v  exit               -> parse the `time` figure: peak RSS, never defaulted to 0
//! ```
//!
//! Killing the *group* is the point: a stage that never returns (a pathological generator, a
//! checker running to its work limit) is bounded as well as the solver loop, and `/usr/bin/time`
//! and the child die together.  `RLIMIT_AS` is not used: it does not bound resident memory on
//! macOS.
//!
//! ## Hardening (VOS-D097, VOS30 section 9.12 item 2, 3 and 5)
//!
//! * **No shell injection.**  The only shell text is the fixed literal [`ULIMIT_SCRIPT`].  The CPU
//!   limit N is a `u64` computed from the row-budget constants and formatted from the number; it,
//!   the program and every argument (the row's spec JSON included) are separate argv entries after
//!   the script, so they can never be parsed as shell syntax.
//! * **A bounded kill.**  The group kill checks the exit status of `/bin/kill`, falls back to
//!   `Child::kill`, and the reaping wait is a `try_wait` loop of at most [`REAP_LIMIT`]: a child
//!   that ignores both can never hang the parent.
//! * **Capped output.**  A child's stdout and stderr are read through `take(1 MiB)` and the rest
//!   is drained and discarded, so a child that prints without end neither fills the parent's
//!   memory nor blocks on a full pipe.

use crate::exp::{row_wall_s, wall_field, RowSpec, CONTROL_WALL};
use crate::results::{green, RSS_CAP_KB, TIMEOUT_S};
use crate::row::{Control, Knob, Row};
use coding_adventures_bounded_json::{parse, serialize, JsonNumber, JsonValue};
use std::io::Read;
// Unix only: puts the child in its own process group.  Elsewhere the group calls below are
// compiled out and `/usr/bin/time` fails to start, which `run_child` reports as an error.
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Slack beyond the solver's own timeout and the control stage before the group is killed: the
/// proof checker's own work limit (2^32 visits) bounds a check to about two minutes.
pub const KILL_SLACK: Duration = Duration::from_secs(150);

/// The wall time after which the parent kills a row's process group, for a row whose budget is
/// `wall_s` seconds: the budget, the control stage and the slack (VOS30 section 9.5 item 3).
pub fn hard_limit_for(wall_s: u64) -> Duration {
    Duration::from_secs(wall_s) + CONTROL_WALL + KILL_SLACK
}

/// The hard limit of a 120 s (non-chain) row.
pub fn hard_limit() -> Duration {
    hard_limit_for(TIMEOUT_S)
}

/// The most a parent waits for a killed child to be reaped before it gives up and moves on.
pub const REAP_LIMIT: Duration = Duration::from_secs(5);

/// The most output (bytes) read from each of a child's stdout and stderr.
pub const CHILD_OUTPUT_CAP: u64 = 1024 * 1024;

/// The CPU-seconds backstop N of a row whose budget is `wall_s`: the hard limit in whole seconds,
/// computed from the budget constants, never read from text.  CPU time never exceeds wall time
/// for a single-threaded solver, so this only ever fires for a child the wall-clock kill missed.
pub fn cpu_limit_s(wall_s: u64) -> u64 {
    hard_limit_for(wall_s).as_secs()
}

/// The one piece of shell text the probe ever runs: set the CPU limit from argument 1, drop it,
/// and `exec` the rest.  Everything variable is an argument, not part of this string.  If the
/// limit cannot be set (a shell that refuses it) the script exits 126 instead of running the row
/// unbounded (finding L3 of the VOS-028c2 review): a backstop that silently does not apply is
/// worse than none.
pub const ULIMIT_SCRIPT: &str = "ulimit -t \"$1\" || exit 126; shift; exec \"$@\"";

/// A [`RowSpec`] as the JSON a child receives.
pub fn spec_to_json(s: &RowSpec) -> String {
    let o = JsonValue::Object(vec![
        ("experiment".into(), JsonValue::String(s.experiment.clone())),
        ("knob".into(), s.knob.to_json()),
        ("mode".into(), JsonValue::String(s.mode.clone())),
        (
            "seed".into(),
            JsonValue::Number(JsonNumber::Integer(
                i64::try_from(s.seed).unwrap_or(i64::MAX),
            )),
        ),
        (
            "seeds".into(),
            JsonValue::Number(JsonNumber::Integer(s.seeds as i64)),
        ),
        ("tier".into(), JsonValue::String(s.tier.clone())),
    ]);
    serialize(&o).unwrap_or_default()
}

/// Read a [`RowSpec`] back.
pub fn spec_from_json(text: &str) -> Result<RowSpec, String> {
    let v = parse(text).map_err(|e| format!("spec is not JSON: {e}"))?;
    let JsonValue::Object(p) = &v else {
        return Err("spec is not an object".into());
    };
    let get = |k: &str| {
        p.iter()
            .find(|(n, _)| n == k)
            .map(|(_, x)| x)
            .ok_or_else(|| format!("spec has no {k}"))
    };
    let s = |k: &str| -> Result<String, String> {
        match get(k)? {
            JsonValue::String(x) => Ok(x.clone()),
            _ => Err(format!("{k} is not a string")),
        }
    };
    let n = |k: &str| -> Result<u64, String> {
        match get(k)? {
            JsonValue::Number(JsonNumber::Integer(i)) if *i >= 0 => Ok(*i as u64),
            _ => Err(format!("{k} is not a non-negative integer")),
        }
    };
    Ok(RowSpec {
        experiment: s("experiment")?,
        knob: Knob::from_json(get("knob")?)?,
        mode: s("mode")?,
        seed: n("seed")?,
        seeds: u32::try_from(n("seeds")?)
            .map_err(|_| "seeds does not fit in 32 bits".to_string())?,
        tier: s("tier")?,
    })
}

/// The progress and answer lines a child prints.
pub fn progress_line(r: &Row) -> String {
    serialize(&JsonValue::Object(vec![("progress".into(), r.to_json())])).unwrap_or_default()
}

/// The answer line.
pub fn row_line(r: &Row) -> String {
    serialize(&JsonValue::Object(vec![("row".into(), r.to_json())])).unwrap_or_default()
}

/// What a child's stdout held.
#[derive(Default)]
pub struct ChildOut {
    /// The last progress row.
    pub progress: Option<Row>,
    /// The final row.
    pub row: Option<Row>,
}

/// Parse a child's stdout (lines of `{"progress":..}` and `{"row":..}`).
pub fn parse_child_out(text: &str) -> ChildOut {
    let mut out = ChildOut::default();
    for line in text.lines() {
        let Ok(JsonValue::Object(p)) = parse(line) else {
            continue;
        };
        for (k, v) in &p {
            if let Ok(r) = Row::from_json(v) {
                match k.as_str() {
                    "progress" => out.progress = Some(r),
                    "row" => out.row = Some(r),
                    _ => {}
                }
            }
        }
    }
    out
}

/// The peak resident set in KiB from the text `/usr/bin/time` printed: macOS `-l` prints bytes
/// ("  12345  maximum resident set size"), GNU `-v` prints KiB ("Maximum resident set size
/// (kbytes): 12345").  `None` if the figure cannot be found: the caller fails the row.
pub fn parse_peak_rss_kb(stderr: &str) -> Option<u64> {
    for line in stderr.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_suffix("maximum resident set size") {
            return rest.trim().parse::<u64>().ok().map(|b| b.div_ceil(1024));
        }
        if let Some(rest) = l.strip_prefix("Maximum resident set size (kbytes):") {
            return rest.trim().parse::<u64>().ok();
        }
    }
    None
}

/// The `/usr/bin/time` flag for this OS.
pub fn time_flag() -> &'static str {
    if cfg!(target_os = "macos") {
        "-l"
    } else {
        "-v"
    }
}

/// The resident set (KiB) of every process in process group `pgid`, summed, via `ps`.
fn group_rss_kb(pgid: u32) -> Option<u64> {
    let out = Command::new("ps")
        .args(["-A", "-o", "pgid=,rss="])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut total = 0u64;
    let mut seen = false;
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if let (Some(g), Some(r)) = (it.next(), it.next()) {
            if g.parse::<u32>().ok() == Some(pgid) {
                total += r.parse::<u64>().unwrap_or(0);
                seen = true;
            }
        }
    }
    seen.then_some(total)
}

/// Signal the whole process group; whether `/bin/kill` ran and reported success.
fn kill_group(pgid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A kill-on-drop guard on a child's process group: if the parent unwinds (a panic) or returns
/// early while the child is still running, dropping the guard kills the group instead of
/// orphaning it.  Disarmed once the child has been reaped.
pub struct GroupGuard {
    pgid: u32,
    armed: bool,
}

impl GroupGuard {
    /// Guard process group `pgid`.
    pub fn new(pgid: u32) -> GroupGuard {
        GroupGuard { pgid, armed: true }
    }
    /// The child is gone: nothing left to kill.
    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        if self.armed {
            kill_group(self.pgid);
        }
    }
}

fn kill_and_reap(child: &mut std::process::Child, pgid: u32) {
    let _ = terminate(
        child,
        &mut || kill_group(pgid),
        REAP_LIMIT,
        Duration::from_millis(10),
    );
}

/// How a child ended, from the parent's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// It exited by itself.
    Exited(Option<i32>),
    /// The resident-set cap tripped.
    Oom,
    /// The wall-clock kill tripped.
    Killed,
}

/// The raw result of one child.
pub struct Child {
    /// Stdout text.
    pub stdout: String,
    /// Stderr text (`/usr/bin/time`'s figures live here).
    pub stderr: String,
    /// How it ended.
    pub end: End,
    /// The largest resident set the parent polled.
    pub polled_kb: u64,
}

/// The command that runs one row: on unix `/bin/sh -c ULIMIT_SCRIPT sh N /usr/bin/time FLAG EXE
/// row --spec JSON --wall W`, with the spec JSON and every other variable part a separate argv
/// entry.  Elsewhere there is no `ulimit`, and the command is `/usr/bin/time` alone (which fails
/// to start, reported by [`run_child`]).
pub fn launch_command(exe: &Path, spec: &RowSpec, wall_s: u64) -> Command {
    launch_command_sub(exe, "row", spec, wall_s)
}

/// [`launch_command`] for the child subcommand `sub` (`row`, or `structure` for `check`'s
/// structural rebuild): the same limits, the same argv discipline.
pub fn launch_command_sub(exe: &Path, sub: &str, spec: &RowSpec, wall_s: u64) -> Command {
    #[cfg(unix)]
    let mut cmd = {
        let mut c = Command::new("/bin/sh");
        c.arg("-c")
            .arg(ULIMIT_SCRIPT)
            .arg("sh")
            .arg(cpu_limit_s(wall_s).to_string())
            .arg("/usr/bin/time");
        c
    };
    #[cfg(not(unix))]
    let mut cmd = Command::new("/usr/bin/time");
    cmd.arg(time_flag());
    cmd.arg(exe).args([
        sub,
        "--spec",
        &spec_to_json(spec),
        "--wall",
        &wall_s.to_string(),
    ]);
    cmd
}

/// Read at most `cap` bytes from `r` as text (lossy), then drain and discard the rest so the
/// writer never blocks on a full pipe.
pub fn read_capped(mut r: impl Read, cap: u64) -> String {
    let mut buf = Vec::new();
    let _ = r.by_ref().take(cap).read_to_end(&mut buf);
    let _ = std::io::copy(&mut r, &mut std::io::sink());
    String::from_utf8_lossy(&buf).into_owned()
}

/// What [`terminate`] needs of a child.
pub trait Reap {
    /// Has it exited?  (`Ok(true)` once it has.)
    fn exited(&mut self) -> std::io::Result<bool>;
    /// Kill it directly (the fallback when the group kill failed).
    fn kill_it(&mut self);
}

impl Reap for std::process::Child {
    fn exited(&mut self) -> std::io::Result<bool> {
        self.try_wait().map(|s| s.is_some())
    }
    fn kill_it(&mut self) {
        let _ = self.kill();
    }
}

/// Kill a child and wait for it for at most `limit`.  `group_kill` returns whether `/bin/kill`
/// reported success; it is tried twice, and if both fail the child is killed directly.  Returns whether the child was reaped:
/// `false` means it ignored the kill, and the caller moves on rather than hang.
pub fn terminate(
    c: &mut dyn Reap,
    group_kill: &mut dyn FnMut() -> bool,
    limit: Duration,
    poll: Duration,
) -> bool {
    // `/bin/kill` can fail transiently (a signal race, a busy process table): retry it once
    // before the direct kill, which reaches only the child and not its group (finding L3).
    if !group_kill() && !group_kill() {
        c.kill_it();
    }
    let t0 = Instant::now();
    loop {
        if matches!(c.exited(), Ok(true)) {
            return true;
        }
        if t0.elapsed() >= limit {
            return false;
        }
        std::thread::sleep(poll);
    }
}

/// Run `exe row --spec .. --wall ..` under `/usr/bin/time` (behind the `ulimit` wrapper) in its
/// own process group, polling its group's resident memory every 50 ms.
pub fn run_child(
    exe: &Path,
    spec: &RowSpec,
    wall_s: u64,
    rss_cap_kb: u64,
    hard: Duration,
) -> Result<Child, String> {
    run_child_sub(exe, "row", spec, wall_s, rss_cap_kb, hard, READER_WAIT)
}

/// How long the parent waits for a child's output readers after the child has ended (round 2,
/// finding 7).  A reader finishes when the pipe closes; a process that outlived the kill (an
/// orphan of a failed group kill) holds the pipe open, and an unbounded `join` would then hold
/// the parent for as long as the orphan lives, defeating the wall bound.
pub const READER_WAIT: Duration = Duration::from_secs(5);

/// [`run_child`] for the subcommand `sub`, with an explicit reader deadline.  When a reader does
/// not finish inside `reader_wait` the threads are detached (they end when the orphan closes the
/// pipe), the output is treated as lost and the row ends [`End::Killed`]: unverified, never a
/// partial answer.
pub fn run_child_sub(
    exe: &Path,
    sub: &str,
    spec: &RowSpec,
    wall_s: u64,
    rss_cap_kb: u64,
    hard: Duration,
    reader_wait: Duration,
) -> Result<Child, String> {
    let mut cmd = launch_command_sub(exe, sub, spec, wall_s);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    // Its own process group, so one signal to the group reaches `/usr/bin/time` and the probe.
    #[cfg(unix)]
    cmd.process_group(0);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start /usr/bin/time: {e}"))?;
    let pgid = child.id();
    let mut guard = GroupGuard::new(pgid);
    let (tx_o, rx_o) = mpsc::channel::<String>();
    let (tx_e, rx_e) = mpsc::channel::<String>();
    let so = child.stdout.take().ok_or("no stdout")?;
    let se = child.stderr.take().ok_or("no stderr")?;
    let ho = std::thread::spawn(move || {
        let _ = tx_o.send(read_capped(so, CHILD_OUTPUT_CAP));
    });
    let he = std::thread::spawn(move || {
        let _ = tx_e.send(read_capped(se, CHILD_OUTPUT_CAP));
    });
    let t0 = Instant::now();
    let mut polled = 0u64;
    let end = loop {
        match child.try_wait() {
            Ok(Some(st)) => break End::Exited(st.code()),
            Ok(None) => {}
            Err(e) => return Err(format!("wait failed: {e}")),
        }
        if let Some(kb) = group_rss_kb(pgid) {
            polled = polled.max(kb);
            if kb > rss_cap_kb {
                kill_and_reap(&mut child, pgid);
                break End::Oom;
            }
        }
        if t0.elapsed() > hard {
            kill_and_reap(&mut child, pgid);
            break End::Killed;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    guard.disarm();
    // Wait for the readers with ONE shared deadline, never `join` (finding 7).  On a timeout the
    // handles are dropped, which detaches the threads.
    let until = Instant::now() + reader_wait;
    let left = || until.saturating_duration_since(Instant::now());
    let stdout = rx_o.recv_timeout(left());
    let stderr = rx_e.recv_timeout(left());
    let (stdout, stderr, end) = match (stdout, stderr) {
        (Ok(o), Ok(e)) => {
            let _ = ho.join();
            let _ = he.join();
            (o, e, end)
        }
        (o, e) => (
            o.unwrap_or_default(),
            e.unwrap_or_default(),
            // The output is incomplete: the row is unverified, whatever the exit status said.
            End::Killed,
        ),
    };
    Ok(Child {
        stdout,
        stderr,
        end,
        polled_kb: polled,
    })
}

/// Turn a finished child into a row: fill the peak RSS, the limits that tripped, and `green`.
pub fn finish_row(spec: &RowSpec, ch: &Child) -> Row {
    let out = parse_child_out(&ch.stdout);
    let rss = parse_peak_rss_kb(&ch.stderr);
    let blank = |why: &str| {
        let mut r = out.progress.clone().unwrap_or_else(|| {
            let mut r = Row::new(
                &spec.experiment,
                spec.knob.clone(),
                &spec.mode,
                spec.seed,
                spec.seeds,
            );
            r.instance_sha256 = "unavailable".into();
            r.control_instance_sha256 = "unavailable".into();
            r
        });
        r.tier = spec.tier.clone();
        r.note = why.to_string();
        r
    };
    let mut row = match ch.end {
        End::Oom => {
            let mut r = blank(&format!(
                "the resident set passed {} KiB; the process group was killed",
                RSS_CAP_KB
            ));
            r.result = "oom".into();
            r.cap = "rss".into();
            r.control = Control::none("oom");
            r.peak_rss_kb = ch.polled_kb.max(RSS_CAP_KB);
            r
        }
        End::Killed => {
            let mut r = blank("the wall-clock kill ended a stage that did not return");
            r.result = "killed".into();
            r.cap = "wall".into();
            r.control = Control::none("killed");
            r.peak_rss_kb = ch.polled_kb.max(1);
            r
        }
        End::Exited(code) => match out.row {
            Some(mut r) => {
                match rss {
                    Some(kb) => r.peak_rss_kb = kb.max(1),
                    None => {
                        r.result = "crashed".into();
                        r.note = format!(
                            "peak RSS could not be parsed from /usr/bin/time ({})",
                            r.note
                        );
                        r.peak_rss_kb = ch.polled_kb.max(1);
                    }
                }
                r
            }
            None => {
                let tail: String = ch
                    .stderr
                    .lines()
                    .rev()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" | ");
                let mut r = blank(&format!(
                    "the child died without an answer (exit status {code:?}): {tail}"
                ));
                r.result = "crashed".into();
                r.control = Control::none("crashed");
                r.peak_rss_kb = rss.unwrap_or(ch.polled_kb).max(1);
                r
            }
        },
    };
    row.tier = spec.tier.clone();
    row.wall_s = wall_field(&spec.experiment);
    row.green = green(&row);
    row
}

/// The most children a run starts at once.
pub const MAX_JOBS: usize = 64;

/// `jobs` clamped to `1..=MAX_JOBS` (VOS-D097 item 5).
pub fn clamp_jobs(jobs: usize) -> usize {
    jobs.clamp(1, MAX_JOBS)
}

/// Run one row as a child under its own budget (VOS-D095) and turn the outcome into a row.
pub fn run_one(exe: &Path, spec: &RowSpec) -> Row {
    run_one_bounded(exe, spec, None)
}

/// [`run_one`] with a tighter hard limit: the child is killed after `min(hard_limit_for(budget),
/// bound)`.  The spot check passes what is left of its own budget (finding M5), so a child that
/// would run for minutes cannot hold `check` for minutes.  A child killed by the bound ends as a
/// `killed` row, exactly as one killed by its own hard limit.
pub fn run_one_bounded(exe: &Path, spec: &RowSpec, bound: Option<Duration>) -> Row {
    run_sub_bounded(exe, "row", spec, bound)
}

/// [`run_one_bounded`] for the child subcommand `sub` (`structure`: the structural rebuild of
/// `check`, which builds an instance and never solves it).
pub fn run_sub_bounded(exe: &Path, sub: &str, spec: &RowSpec, bound: Option<Duration>) -> Row {
    let wall_s = row_wall_s(&spec.experiment);
    let hard = bound.map_or(hard_limit_for(wall_s), |b| b.min(hard_limit_for(wall_s)));
    match run_child_sub(exe, sub, spec, wall_s, RSS_CAP_KB, hard, READER_WAIT) {
        Ok(ch) => finish_row(spec, &ch),
        Err(e) => {
            let mut r = Row::new(
                &spec.experiment,
                spec.knob.clone(),
                &spec.mode,
                spec.seed,
                spec.seeds,
            );
            r.tier = spec.tier.clone();
            r.wall_s = wall_field(&spec.experiment);
            r.note = e;
            r.peak_rss_kb = 1;
            r
        }
    }
}

/// Run a list of rows with `jobs` children at a time, calling `on_row` as each finishes (in
/// completion order) and returning the rows in the order of `specs`.
pub fn run_rows(
    exe: &Path,
    specs: &[RowSpec],
    jobs: usize,
    on_row: &(dyn Fn(usize, &Row) + Sync),
) -> Vec<Row> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: std::sync::Mutex<Vec<Option<Row>>> = std::sync::Mutex::new(vec![None; specs.len()]);
    std::thread::scope(|s| {
        for _ in 0..clamp_jobs(jobs) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some(spec) = specs.get(i) else { break };
                let row = run_one(exe, spec);
                on_row(i, &row);
                slots.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(row);
            });
        }
    });
    slots
        .into_inner()
        .unwrap_or_else(|e| e.into_inner())
        .into_iter()
        .map(|r| r.expect("every spec ran"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::Kv;

    fn spec() -> RowSpec {
        RowSpec {
            experiment: "A1".into(),
            knob: Knob::of(&[("n", Kv::I(8))]),
            mode: "monolithic".into(),
            seed: 2,
            seeds: 5,
            tier: "quick".into(),
        }
    }

    #[test]
    fn a_spec_round_trips_and_a_bad_one_is_an_error() {
        let s = spec();
        assert_eq!(spec_from_json(&spec_to_json(&s)).unwrap(), s);
        assert!(spec_from_json("[]").is_err());
        assert!(spec_from_json("{}").is_err());
        assert!(spec_from_json("nope").is_err());
        assert!(spec_from_json(&spec_to_json(&s).replace("\"seed\":2", "\"seed\":-2")).is_err());
    }

    #[test]
    fn the_peak_rss_is_parsed_from_both_time_formats_and_never_defaulted() {
        let mac = "        0.00 real         0.00 user         0.00 sys\n            1572864  maximum resident set size\n                 0  average shared memory size\n";
        assert_eq!(parse_peak_rss_kb(mac), Some(1536));
        let gnu = "\tCommand being timed: \"x\"\n\tMaximum resident set size (kbytes): 4321\n";
        assert_eq!(parse_peak_rss_kb(gnu), Some(4321));
        assert_eq!(parse_peak_rss_kb("nothing useful"), None);
        assert_eq!(parse_peak_rss_kb("   abc  maximum resident set size"), None);
        assert_eq!(parse_peak_rss_kb(""), None);
    }

    #[test]
    fn child_output_lines_are_read_and_junk_is_ignored() {
        let mut r = Row::new("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 1, 5);
        r.result = "unsat".into();
        let text = format!(
            "garbage\n{}\n{}\n{{\"other\":1}}\n",
            progress_line(&r),
            row_line(&r)
        );
        let o = parse_child_out(&text);
        assert_eq!(o.progress.unwrap().result, "unsat");
        assert_eq!(o.row.unwrap().experiment, "A1");
        assert!(parse_child_out("").row.is_none());
    }

    fn child(stdout: &str, stderr: &str, end: End) -> Child {
        Child {
            stdout: stdout.into(),
            stderr: stderr.into(),
            end,
            polled_kb: 700,
        }
    }

    #[test]
    fn a_normal_exit_takes_its_peak_rss_from_time_and_computes_green() {
        let mut r = Row::new("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 2, 5);
        r.result = "unsat".into();
        r.drat_bytes = Some(10);
        r.pass_default_limits = Some(true);
        r.control = Control {
            result: "sat".into(),
            model_checks_cnf: true,
            replayed: true,
        };
        let out = child(
            &row_line(&r),
            "  2048000  maximum resident set size\n",
            End::Exited(Some(0)),
        );
        let row = finish_row(&spec(), &out);
        assert_eq!(row.peak_rss_kb, 2000);
        assert!(row.green);
        // No parsable figure: the row fails and says why; it is never recorded as 0.
        let bad = finish_row(
            &spec(),
            &child(&row_line(&r), "no figures", End::Exited(Some(0))),
        );
        assert_eq!(bad.result, "crashed");
        assert!(bad.note.contains("peak RSS could not be parsed"));
        assert!(bad.peak_rss_kb > 0);
        assert!(!bad.green);
    }

    #[test]
    fn oom_killed_and_crashed_children_become_red_rows() {
        let oom = finish_row(&spec(), &child("", "", End::Oom));
        assert_eq!((oom.result.as_str(), oom.cap.as_str()), ("oom", "rss"));
        assert!(oom.peak_rss_kb >= RSS_CAP_KB && !oom.green);
        let killed = finish_row(&spec(), &child("", "", End::Killed));
        assert_eq!(
            (killed.result.as_str(), killed.cap.as_str()),
            ("killed", "wall")
        );
        let crashed = finish_row(
            &spec(),
            &child("", "thread panicked\n", End::Exited(Some(101))),
        );
        assert_eq!(crashed.result, "crashed");
        assert!(crashed.note.contains("101"));
        // A progress line survives the kill: the digest of the instance is kept.
        let mut p = Row::new("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 2, 5);
        p.instance_sha256 = "ab".repeat(32);
        let k = finish_row(&spec(), &child(&progress_line(&p), "", End::Killed));
        assert_eq!(k.instance_sha256, "ab".repeat(32));
    }

    /// A child that ignores every kill: it never reports exit.
    struct Stubborn {
        kills: u32,
    }
    impl Reap for Stubborn {
        fn exited(&mut self) -> std::io::Result<bool> {
            Ok(false)
        }
        fn kill_it(&mut self) {
            self.kills += 1;
        }
    }

    /// A child that exits as soon as it is killed.
    struct Obedient {
        dead: bool,
    }
    impl Reap for Obedient {
        fn exited(&mut self) -> std::io::Result<bool> {
            Ok(self.dead)
        }
        fn kill_it(&mut self) {
            self.dead = true;
        }
    }

    /// A child whose wait fails.
    struct Broken;
    impl Reap for Broken {
        fn exited(&mut self) -> std::io::Result<bool> {
            Err(std::io::Error::other("wait failed"))
        }
        fn kill_it(&mut self) {}
    }

    #[test]
    fn a_stub_child_that_ignores_the_kill_cannot_hang_the_bounded_wait() {
        let mut c = Stubborn { kills: 0 };
        let t0 = Instant::now();
        // The group kill reports failure: the fallback kills the child directly, once.
        let reaped = terminate(
            &mut c,
            &mut || false,
            Duration::from_millis(80),
            Duration::from_millis(5),
        );
        assert!(!reaped, "it ignored the kill: not reaped, but we returned");
        assert_eq!(c.kills, 1, "the fallback ran");
        assert!(t0.elapsed() >= Duration::from_millis(80));
        assert!(
            t0.elapsed() < Duration::from_secs(5),
            "bounded by the limit, not forever"
        );
        // The group kill succeeded: no fallback, and a still-running child is still not waited on
        // past the limit.
        let mut c = Stubborn { kills: 0 };
        assert!(!terminate(
            &mut c,
            &mut || true,
            Duration::from_millis(30),
            Duration::from_millis(5)
        ));
        assert_eq!(c.kills, 0);
        // A wait that errors is not a reap either.
        assert!(!terminate(
            &mut Broken,
            &mut || true,
            Duration::from_millis(30),
            Duration::from_millis(5)
        ));
        // An obedient child is reaped at once.
        let mut o = Obedient { dead: false };
        assert!(terminate(
            &mut o,
            &mut || false,
            REAP_LIMIT,
            Duration::from_millis(5)
        ));
        assert!(o.dead);
    }

    /// Finding L3: a failed group kill is retried once before the direct kill.
    #[test]
    fn the_group_kill_is_retried_once_before_the_direct_kill_fallback() {
        let mut calls = 0;
        let mut c = Obedient { dead: false };
        // Fails once, succeeds on the retry: the direct kill never runs (the stub only dies
        // from it, so it is still alive and the wait gives up at the limit).
        assert!(!terminate(
            &mut c,
            &mut || {
                calls += 1;
                calls == 2
            },
            Duration::from_millis(30),
            Duration::from_millis(5)
        ));
        assert_eq!(calls, 2);
        assert!(!c.dead, "no direct kill after a successful retry");
        // Fails twice: exactly two attempts, then the direct kill.
        let mut calls = 0;
        let mut c = Obedient { dead: false };
        assert!(terminate(
            &mut c,
            &mut || {
                calls += 1;
                false
            },
            REAP_LIMIT,
            Duration::from_millis(5)
        ));
        assert_eq!(calls, 2);
        assert!(c.dead);
    }

    #[cfg(unix)]
    #[test]
    fn dropping_an_armed_guard_kills_the_process_group_and_a_disarmed_one_does_not() {
        use std::os::unix::process::CommandExt;
        let mut ch = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .unwrap();
        drop(GroupGuard::new(ch.id()));
        assert!(terminate(
            &mut ch,
            &mut || true,
            REAP_LIMIT,
            Duration::from_millis(5)
        ));
        let mut ch = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .unwrap();
        let mut g = GroupGuard::new(ch.id());
        g.disarm();
        drop(g);
        std::thread::sleep(Duration::from_millis(100));
        assert!(matches!(ch.try_wait(), Ok(None)), "still running");
        let _ = ch.kill();
        let _ = ch.wait();
    }

    #[test]
    fn the_real_child_impl_reaps_a_killed_process() {
        let mut ch = Command::new("sleep").arg("30").spawn().unwrap();
        assert!(terminate(
            &mut ch,
            &mut || false,
            REAP_LIMIT,
            Duration::from_millis(5)
        ));
    }

    #[test]
    fn output_is_read_through_a_cap_and_the_rest_is_drained() {
        let data = vec![b'x'; 5000];
        let s = read_capped(std::io::Cursor::new(data), 1000);
        assert_eq!(s.len(), 1000);
        assert_eq!(
            read_capped(std::io::Cursor::new(b"abc".to_vec()), 1000),
            "abc"
        );
        // Invalid UTF-8 is replaced, never an error that loses the whole output.
        assert!(read_capped(std::io::Cursor::new(vec![0xff, b'a']), 10).ends_with('a'));
        assert_eq!(CHILD_OUTPUT_CAP, 1024 * 1024);
    }

    #[test]
    fn jobs_are_clamped_and_a_chain_row_records_its_budget() {
        assert_eq!(
            (clamp_jobs(0), clamp_jobs(7), clamp_jobs(9999)),
            (1, 7, MAX_JOBS)
        );
        let b3 = RowSpec {
            experiment: "B3".into(),
            knob: Knob::of(&[("length", Kv::I(25))]),
            mode: "certified".into(),
            seed: 1,
            seeds: 5,
            tier: "full".into(),
        };
        for end in [End::Killed, End::Oom, End::Exited(Some(1))] {
            assert_eq!(finish_row(&b3, &child("", "", end)).wall_s, Some(600));
            assert_eq!(finish_row(&spec(), &child("", "", end)).wall_s, None);
        }
        assert_eq!(cpu_limit_s(600), 780);
        assert_eq!(cpu_limit_s(120), 300);
    }

    #[test]
    fn the_hard_limit_covers_the_solver_the_control_and_the_slack() {
        assert_eq!(
            hard_limit(),
            Duration::from_secs(TIMEOUT_S) + CONTROL_WALL + KILL_SLACK
        );
        assert_eq!(
            time_flag(),
            if cfg!(target_os = "macos") {
                "-l"
            } else {
                "-v"
            }
        );
    }
}
