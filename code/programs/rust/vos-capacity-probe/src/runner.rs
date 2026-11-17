//! # `runner`: one child process per row, with a kill switch
//!
//! The probe runs hostile-sized instances on purpose (VOS30 section 9.5), so it never solves one
//! in its own process.  Each row is a child:
//!
//! ```text
//!   parent                                    child (own process group)
//!   ------                                    -------------------------
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

use crate::exp::{RowSpec, CONTROL_WALL};
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

/// The wall time after which the parent kills a row's process group.
pub fn hard_limit() -> Duration {
    Duration::from_secs(TIMEOUT_S) + CONTROL_WALL + KILL_SLACK
}

/// A [`RowSpec`] as the JSON a child receives.
pub fn spec_to_json(s: &RowSpec) -> String {
    let o = JsonValue::Object(vec![
        ("experiment".into(), JsonValue::String(s.experiment.clone())),
        ("knob".into(), s.knob.to_json()),
        ("mode".into(), JsonValue::String(s.mode.clone())),
        (
            "seed".into(),
            JsonValue::Number(JsonNumber::Integer(s.seed as i64)),
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
        seeds: n("seeds")? as u32,
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

fn kill_group(pgid: u32) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
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

/// Run `exe row --spec .. --wall ..` under `/usr/bin/time` in its own process group, polling its
/// group's resident memory every 50 ms.
pub fn run_child(
    exe: &Path,
    spec: &RowSpec,
    wall_s: u64,
    rss_cap_kb: u64,
    hard: Duration,
) -> Result<Child, String> {
    let mut cmd = Command::new("/usr/bin/time");
    cmd.arg(time_flag())
        .arg(exe)
        .args([
            "row",
            "--spec",
            &spec_to_json(spec),
            "--wall",
            &wall_s.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    // Its own process group, so one signal to the group reaches `/usr/bin/time` and the probe.
    #[cfg(unix)]
    cmd.process_group(0);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start /usr/bin/time: {e}"))?;
    let pgid = child.id();
    let (tx_o, rx_o) = mpsc::channel::<String>();
    let (tx_e, rx_e) = mpsc::channel::<String>();
    let mut so = child.stdout.take().ok_or("no stdout")?;
    let mut se = child.stderr.take().ok_or("no stderr")?;
    let ho = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = so.read_to_string(&mut s);
        let _ = tx_o.send(s);
    });
    let he = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = se.read_to_string(&mut s);
        let _ = tx_e.send(s);
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
                kill_group(pgid);
                let _ = child.wait();
                break End::Oom;
            }
        }
        if t0.elapsed() > hard {
            kill_group(pgid);
            let _ = child.wait();
            break End::Killed;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = ho.join();
    let _ = he.join();
    Ok(Child {
        stdout: rx_o.recv().unwrap_or_default(),
        stderr: rx_e.recv().unwrap_or_default(),
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
    row.green = green(&row);
    row
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
        for _ in 0..jobs.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some(spec) = specs.get(i) else { break };
                let row = match run_child(exe, spec, TIMEOUT_S, RSS_CAP_KB, hard_limit()) {
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
                        r.note = e;
                        r.peak_rss_kb = 1;
                        r
                    }
                };
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
