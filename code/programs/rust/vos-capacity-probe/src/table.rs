//! # `table`: the generated table of VOS30 section 9.8 and its drift check
//!
//! `table` reads the committed results file and writes, for each `(experiment, knob, mode)`
//! group, one line that is the group's **worst seed's own row** (section 8.3: a median would let
//! one hard instance hide).  The worst row is chosen by a deterministic rule: a non-green row
//! before a green one, then the most conflicts of one obligation, then the longest solve, then
//! the LOWEST seed.  Every cell comes from that one row, except the two group-level cells that
//! say so: `Seeds` (counted of total, and the number of distinct instances among the counted
//! seeds) and `Max over seeds` (the largest solve time and resident set of any counted seed, so
//! a harder seed's time or memory is never hidden by the single-row rule).  Collapsed seeds are
//! excluded from every statistic and said so in the `Seeds` column.  Values from the results file
//! are escaped before they go into a cell.  The block begins with a fixed banner and the validity
//! limits of section 8.4, so
//! a table can never be read without them, and it ends with the section 8.3 decision computed
//! from the same rows.
//!
//! ```text
//!   <!-- vos30-table:begin -->
//!   PROXY: ...banner...        the validity limits        | the table |     decision
//!   <!-- vos30-table:end -->
//! ```
//!
//! [`check_spec`] is `table --check`: it regenerates the block and compares it byte for byte with
//! the spec (drift), fails if the results file is absent while the block is present (or the other
//! way round), and fails if section 8.5 states a decision while the block is absent, or states one
//! that differs from the decision computed from the rows.  No number in the block is typed.
//!
//! ## The re-plan block (VOS-D142)
//!
//! Section 8.6 of the spec carries a second, much smaller generated block between
//! [`REPLAN_BEGIN`] and [`REPLAN_END`]: the answer of [`decide_replan`] (the re-plan row, the cut
//! density K, the memory verdict and the conditions that placed the row), one line each, from
//! [`replan_lines`], which is also what `decide --replan` prints.  `table --write` splices it and
//! `table --check` compares it byte for byte, and also compares the hand-written label of 8.6
//! (`**Re-plan decision (VOS-028c): R3, compiled lane, reduced**` and the sentence after it) with
//! the computed row, K and memory verdict, so a hand-edited label fails `check`.  A spec that
//! has no section 8.6, no block or no label fails (like the 9.8 block, with a results file).

use crate::results::{
    decide, decide_replan, groups, red_reason, Group, ReplanDecision, Results, TITLES,
};
use crate::row::{Kv, Row};

/// The opening marker.
pub const BEGIN: &str = "<!-- vos30-table:begin -->";
/// The closing marker.
pub const END: &str = "<!-- vos30-table:end -->";

/// The validity limits of section 8.4, in one line each (they travel with every table).
pub const LIMITS: [&str; 10] = [
    "Hand-written code is not compiler output: no register allocation, spills, fusion or inlining; real code may be easier or harder.",
    "The divergence is synthetic: the reference differs from the code in controlled proportions (5%, 25%, 100% of the live instructions that have an alternative encoding), not as a real compiler's would.",
    "Trap entry, CSRs and `mret` are not modelled: only the ALU, `lw`/`lbu`/`sw`/`sb` and forward branches with a concrete PC.",
    "The probe bounds solver capacity per obligation shape; it does not prove any entry verifiable.",
    "One machine, one solver build: seconds are host-specific, conflicts and work are the portable part.",
    "The probe never raises the `drat-check` limits: a proof the default limits reject is a red row (the advisor's numbers raised them, so every proof there was checkable).",
    "The D instances share one symbolic base index; `k` independent symbolic indices are not measured, and the index has `log2 T` bits so no out-of-range case arises.",
    "Liveness is decided exactly only for cones of at most 4,000 ANDs; a larger cone's entropy rests on 256 random assignments (a row says so in its note).",
    "Both sides run on one stepper: a stepper bug that affects both alike is invisible to the miter (the checks are the differential against the simulator, the independent evaluator and the replay of every control).",
    "\"Five seeds\" means five distinct instances only where the Seeds column says so: a deterministic shape (A, C on an A1 base, D with a concrete index) may build the same circuit for several seeds. A group's line is its worst seed's own row, chosen by non-green, then most conflicts, then longest solve, then lowest seed, with every cell from that one row except the two group-level cells (the distinct-instance count and the maximum solve and resident memory over the seeds); a proof or check that was never produced shows `-`, unmeasured.",
];

fn fmt_ms(ms: f64) -> String {
    if ms < 1.0 {
        "<1 ms".into()
    } else if ms < 1000.0 {
        format!("{ms:.0} ms")
    } else if ms < 60_000.0 {
        format!("{:.1} s", ms / 1000.0)
    } else {
        format!("{:.1} min", ms / 60_000.0)
    }
}

fn fmt_bytes(b: u64) -> String {
    if b < 1024 {
        format!("{b} B")
    } else if b < 1024 * 1024 {
        format!("{:.1} kB", b as f64 / 1024.0)
    } else {
        format!("{:.1} MB", b as f64 / 1048576.0)
    }
}

fn fmt_count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

/// A knob as `key=value` words.
pub fn knob_text(r: &Row) -> String {
    r.knob
        .0
        .iter()
        .map(|(k, v)| match v {
            Kv::I(i) => format!("{k}={i}"),
            Kv::F(f) if k == "divergence" => format!("div={}%", (f * 100.0).round() as i64),
            Kv::F(f) => format!("{k}={f}"),
            Kv::S(s) => format!("{k}={s}"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The row that stands for a group: a non-green row before a green one, then the most conflicts
/// of one obligation, then the longest solve.  Every cell of the group's line comes from THIS
/// row, so conflicts, DRAT size and the checker's verdict are always from one seed (a table
/// that took each column's maximum separately printed a conflict count of one seed beside the
/// proof size and verdict of another).
pub fn worst_row<'a>(basis: &[&'a Row]) -> &'a Row {
    // Highest (non-green, conflicts, solve time); a tie goes to the LOWEST seed, so the choice
    // never depends on the order of the file.
    let key = |r: &Row| {
        (
            !r.green,
            r.worst_step_conflicts.unwrap_or(r.conflicts),
            (r.worst_step_solve_ms * 1000.0) as u64,
        )
    };
    let mut best = basis[0];
    for r in &basis[1..] {
        let (kr, kb) = (key(r), key(best));
        if kr > kb || (kr == kb && r.seed < best.seed) {
            best = r;
        }
    }
    best
}

/// Make a value from the results file safe for one table cell: a pipe would end the cell, a
/// newline would end the row, and a generated-region marker would end the block.
pub fn cell(text: &str) -> String {
    text.replace('|', "\\|")
        .replace(['\n', '\r'], " ")
        .replace("<!--", "&lt;!--")
}

fn verdict(g: &Group<'_>) -> String {
    let first = g.first();
    if g.green() {
        if first.mode == "enum" {
            return match first.experiment.as_str() {
                "E1" => {
                    let slowest = g
                        .counted()
                        .iter()
                        .filter_map(|r| r.metric("cases_per_s"))
                        .fold(f64::INFINITY, f64::min);
                    format!("pass ({} cases/s, slowest)", fmt_count(slowest as u64))
                }
                _ => "pass".into(),
            };
        }
        return "GREEN".into();
    }
    let counted = g.counted();
    if counted.is_empty() {
        return "collapsed".into();
    }
    // The worst counted row names the verdict.
    let worst = worst_row(&counted);
    match worst.result.as_str() {
        "unknown" => format!(
            "unknown ({})",
            if worst.cap == "wall" {
                "timeout"
            } else {
                "conflict cap"
            }
        ),
        "killed" | "oom" | "crashed" => worst.result.clone(),
        "refused" if worst.control.ok() => {
            let code = worst.note.split(':').next().unwrap_or("refused");
            format!("refused ({code})")
        }
        _ => format!("red ({})", red_reason(worst)),
    }
}

fn line(g: &Group<'_>) -> String {
    let first = g.first();
    let counted = g.counted();
    let basis: Vec<&Row> = if counted.is_empty() {
        g.rows.clone()
    } else {
        counted.clone()
    };
    let enum_row = first.mode == "enum";
    let dash = || "-".to_string();
    let w = worst_row(&basis);
    let distinct = g
        .rows
        .iter()
        .filter(|r| basis.iter().any(|b| std::ptr::eq(*b, **r)))
        .map(|r| r.instance_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let chain = first.steps > 1;
    let conflicts = if enum_row {
        dash()
    } else if chain {
        format!(
            "{} / {}",
            fmt_count(w.conflicts),
            fmt_count(w.worst_step_conflicts.unwrap_or(0))
        )
    } else {
        fmt_count(w.conflicts)
    };
    let solve = if chain {
        format!(
            "{} ({} worst)",
            fmt_ms(w.solve_ms),
            fmt_ms(w.worst_step_solve_ms)
        )
    } else {
        fmt_ms(w.solve_ms)
    };
    // A proof that was never produced or checked is "unmeasured", not a zero or a pass.
    let drat = w.drat_bytes;
    let work = w.check_work;
    let limits = match w.pass_default_limits {
        Some(false) => "REJECT".to_string(),
        Some(true) => "pass".to_string(),
        None => dash(),
    };
    let control = if w.control.ok() {
        "replayed".to_string()
    } else {
        w.control.result.clone()
    };
    let max_solve = basis.iter().map(|r| r.solve_ms).fold(0.0, f64::max);
    let max_rss = basis.iter().map(|r| r.peak_rss_kb).max().unwrap_or(0);
    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {}/{} ({} distinct) | {} | {} | {} | {} | {} | {} MB | {} | {} / {} MB | {} |",
        cell(&first.experiment),
        cell(&knob_text(first)),
        cell(&first.mode),
        if enum_row {
            dash()
        } else {
            fmt_count(w.ands)
        },
        if enum_row {
            dash()
        } else {
            fmt_count(w.vars)
        },
        if enum_row {
            dash()
        } else {
            fmt_count(w.clauses)
        },
        if enum_row {
            dash()
        } else {
            format!(
                "{}/{}",
                fmt_count(w.entropy_bits),
                fmt_count(w.declared_bits)
            )
        },
        counted.len(),
        g.rows.len(),
        distinct,
        conflicts,
        solve,
        drat.map(fmt_bytes).unwrap_or_else(dash),
        work.map(fmt_count).unwrap_or_else(dash),
        limits,
        (w.peak_rss_kb as f64 / 1024.0).round() as u64,
        cell(&control),
        fmt_ms(max_solve),
        (max_rss as f64 / 1024.0).round() as u64,
        cell(&verdict(g)),
    )
}

/// Experiment F: constants derived from the solver rows (never typed).
pub fn derived_constants(rows: &[Row]) -> Vec<String> {
    let solved: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            r.result == "unsat" && r.mode == "monolithic" && r.conflicts >= 1000 && r.solve_ms > 0.0
        })
        .collect();
    if solved.is_empty() {
        return vec!["no solved row of at least 1,000 conflicts".to_string()];
    }
    let range = |f: &dyn Fn(&Row) -> Option<f64>| -> Option<(f64, f64)> {
        let v: Vec<f64> = solved.iter().filter_map(|r| f(r)).collect();
        if v.is_empty() {
            None
        } else {
            Some((
                v.iter().cloned().fold(f64::INFINITY, f64::min),
                v.iter().cloned().fold(0.0, f64::max),
            ))
        }
    };
    let mut out = Vec::new();
    if let Some((lo, hi)) = range(&|r| Some(r.conflicts as f64 / (r.solve_ms / 1000.0))) {
        out.push(format!(
            "Solver throughput: {} to {} conflicts per second.",
            fmt_count(lo as u64),
            fmt_count(hi as u64)
        ));
    }
    if let Some((lo, hi)) = range(&|r| r.drat_bytes.map(|b| b as f64 / r.conflicts as f64)) {
        out.push(format!("DRAT size: {lo:.0} to {hi:.0} bytes per conflict."));
    }
    if let Some((lo, hi)) = range(&|r| r.check_ms.map(|c| c / r.solve_ms)) {
        out.push(format!(
            "`drat-check` time: {lo:.1}x to {hi:.1}x the solve time."
        ));
    }
    if let Some((lo, hi)) = range(&|r| r.check_work.map(|w| w as f64 / r.conflicts as f64)) {
        out.push(format!(
            "Checker work: {} to {} literal visits per conflict.",
            fmt_count(lo as u64),
            fmt_count(hi as u64)
        ));
    }
    let accepted = solved
        .iter()
        .filter(|r| r.pass_default_limits == Some(true))
        .map(|r| r.conflicts)
        .max();
    let rejected = solved
        .iter()
        .filter(|r| r.pass_default_limits == Some(false) && r.note.contains("default work limit"))
        .map(|r| r.conflicts)
        .min();
    match (accepted, rejected) {
        (Some(a), Some(b)) => out.push(format!("Default `drat-check` work limit (2^32): the largest proof accepted has {} conflicts, the smallest rejected for work has {}.", fmt_count(a), fmt_count(b))),
        (Some(a), None) => out.push(format!("Default `drat-check` work limit (2^32): the largest proof accepted has {} conflicts; none was rejected for work.", fmt_count(a))),
        (None, _) => out.push("Default `drat-check` limits: no proof of at least 1,000 conflicts was accepted.".to_string()),
    }
    let big: Vec<&&Row> = solved
        .iter()
        .filter(|r| r.conflicts >= 100_000 && r.peak_rss_kb > 0)
        .collect();
    if !big.is_empty() {
        let per: Vec<f64> = big
            .iter()
            .map(|r| r.peak_rss_kb as f64 / 1048576.0 / (r.conflicts as f64 / 1e6))
            .collect();
        let lo = per.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = per.iter().cloned().fold(0.0, f64::max);
        out.push(format!("Resident memory: {lo:.1} to {hi:.1} GB per million conflicts (rows of at least 100,000 conflicts)."));
    }
    out
}

/// The generated block, markers included, ending with a newline.
pub fn render_block(res: &Results) -> String {
    let mut s = String::new();
    s.push_str(BEGIN);
    s.push('\n');
    let banner = match &res.meta.regeneration {
        None => "Full tier not independently regenerated.".to_string(),
        // A signed file names BOTH anchors (VOS-028c3c-3b-1b, VOS-D141): the recording run and the
        // verify run that reproduced it at the reviewer's checkout.  The recording sentence is
        // unchanged and the verify sentence follows it, so a reader sees where the rows were
        // measured and where they were reproduced, which may be different commits.  Revisions
        // are shortened to 12 hex digits and the verify file's digest to 16: enough to find the
        // run and to compare by eye, while the banner in the results file holds the full values.
        Some(g) => format!(
            "Full tier regenerated by {} on {}. Recording: run {} attempt {} at revision {}. Verified: run {} attempt {} at revision {}, verify file SHA-256 {}.",
            g.reviewer,
            g.host,
            g.run_id,
            g.run_attempt,
            g.head_sha.chars().take(12).collect::<String>(),
            g.verified.run_id,
            g.verified.run_attempt,
            g.verified.head_sha.chars().take(12).collect::<String>(),
            g.verified.verify_sha256.chars().take(16).collect::<String>()
        ),
    };
    let refreshed: Vec<String> = res
        .meta
        .merges
        .iter()
        .map(|g| format!("{} rows refreshed at rev {}", g.rows.len(), g.rev))
        .collect();
    let banner = if refreshed.is_empty() {
        banner
    } else {
        format!(
            "{banner} Provenance (`tool`, `host`, `revisions`) is the original run's; {}.",
            refreshed.join("; ")
        )
    };
    s.push_str(&format!(
        "PROXY: synthetic hand-written code with synthetic divergence, not compiler\noutput. These numbers bound solver capacity per obligation shape; they do not\nprove any entry verifiable. Seeds >= 5; each line is its worst seed's own row (ties: lowest seed), with the maximum solve time and resident memory over the seeds in the last column but one. {banner}\n\n"
    ));
    s.push_str("Validity limits (VOS30 section 8.4), printed with every number:\n\n");
    for (i, l) in LIMITS.iter().enumerate() {
        s.push_str(&format!("{}. {l}\n", i + 1));
    }
    s.push_str(&format!(
        "\nRun: `{}` {}, `{}`, {}; the `check` re-run reproduces the quick-tier rows.\n\n",
        res.meta.name, res.meta.version, res.meta.rustc, res.meta.os
    ));
    s.push_str("| Experiment | Knob | Mode | ANDs | Vars | Clauses | Entropy bits | Seeds (distinct instances) | Conflicts (total / worst) | Solve | DRAT | Check work | Default limits | RSS | Control | Max over seeds (solve / RSS) | Verdict |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for g in groups(&res.rows) {
        s.push_str(&line(&g));
        s.push('\n');
    }
    s.push_str("\nDerived constants (experiment F), from the counted monolithic UNSAT rows of at least 1,000 conflicts:\n\n");
    for l in derived_constants(&res.rows) {
        s.push_str(&format!("* {l}\n"));
    }
    let d = decide(&res.rows);
    s.push_str(&format!(
        "\nSection 8.3 evaluated on these rows: **row {}, {}**.\n",
        d.row, d.title
    ));
    for b in &d.because {
        s.push_str(&format!("\n* {b}"));
    }
    s.push('\n');
    s.push_str(END);
    s.push('\n');
    s
}

/// The opening marker of the re-plan block in section 8.6.
pub const REPLAN_BEGIN: &str = "<!-- vos30-replan:begin -->";
/// The closing marker of the re-plan block.
pub const REPLAN_END: &str = "<!-- vos30-replan:end -->";
/// The start of the hand-written label of 8.6 that the re-plan block backs.
const REPLAN_LABEL: &str = "**Re-plan decision (VOS-028c):";

/// The lines that state a re-plan decision: `decide --replan` prints them as they are and the
/// block of 8.6 renders each as a bullet.  Deterministic in the rows alone.
pub fn replan_lines(d: &ReplanDecision) -> Vec<String> {
    let mut v = vec![
        format!("row R{}: {}", d.row, d.title),
        format!("cut density K = {}", d.cut_density_k),
        format!("memory: {}", d.memory_verdict),
    ];
    for b in &d.because {
        v.push(format!("because: {b}"));
    }
    v
}

/// The re-plan block of 8.6 for `res`, markers and trailing newline included.
pub fn render_replan_block(res: &Results) -> String {
    let mut s = String::new();
    s.push_str(REPLAN_BEGIN);
    s.push('\n');
    s.push_str("Generated by `table --write` from the results file (`decide --replan` prints the same lines). Do not edit: `table --check` compares this block and the label above it with the rows.\n\n");
    for l in replan_lines(&decide_replan(&res.rows)) {
        s.push_str(&format!("* {l}\n"));
    }
    s.push_str(REPLAN_END);
    s.push('\n');
    s
}

/// Is the `len` bytes at `at` the whole of their line, apart from surrounding whitespace?
fn alone_on_line(spec: &str, at: usize, len: usize) -> bool {
    let start = spec[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = spec[at + len..]
        .find('\n')
        .map_or(spec.len(), |i| at + len + i);
    spec[start..at].trim().is_empty() && spec[at + len..end].trim().is_empty()
}

/// The re-plan block of `spec` (markers included) and its offset: `Ok(None)` when neither marker
/// is there, `Err` when the pair is broken (one marker, the end first, or a second pair).
pub fn extract_replan_block(spec: &str) -> Result<Option<(usize, &str)>, String> {
    let (a, e) = (spec.find(REPLAN_BEGIN), spec.find(REPLAN_END));
    match (a, e) {
        (None, None) => Ok(None),
        (Some(a), Some(e))
            if a < e
                && spec.matches(REPLAN_BEGIN).count() == 1
                && spec.matches(REPLAN_END).count() == 1 =>
        {
            if !alone_on_line(spec, a, REPLAN_BEGIN.len())
                || !alone_on_line(spec, e, REPLAN_END.len())
            {
                return Err("a replan marker shares its line with other text".to_string());
            }
            let end = e + REPLAN_END.len();
            let end = if spec[end..].starts_with('\n') {
                end + 1
            } else {
                end
            };
            Ok(Some((a, &spec[a..end])))
        }
        _ => Err("the replan markers are not exactly one begin followed by one end".to_string()),
    }
}

/// Replace the re-plan block of `spec` by `block`; `None` when the spec has no marker pair.
pub fn splice_replan(spec: &str, block: &str) -> Option<String> {
    let (_, old) = extract_replan_block(spec).ok()??;
    Some(spec.replacen(old, block, 1))
}

/// The byte range of section 8.6 (its subsections included) in `spec`.
fn section_86(spec: &str) -> Option<(usize, usize)> {
    // The heading is a line of its own: prose that mentions "### 8.6 " is not the section.
    let a = if spec.starts_with("### 8.6 ") {
        0
    } else {
        spec.find("\n### 8.6 ")? + 1
    };
    let rest = &spec[a + 8..];
    let end = ["\n## ", "\n### "]
        .iter()
        .filter_map(|h| rest.find(h))
        .min()
        .map(|i| a + 8 + i)
        .unwrap_or(spec.len());
    Some((a, end))
}

/// Every reason the hand-written label of 8.6 disagrees with `d`.  The label is the bold text
/// after [`REPLAN_LABEL`] (`R3, compiled lane, reduced`), and the paragraph it opens must say the
/// cut density and which memory verdict applies.
fn check_replan_label(section: &str, d: &ReplanDecision) -> Vec<String> {
    let mut v = Vec::new();
    let n = section.matches(REPLAN_LABEL).count();
    if n != 1 {
        return vec![format!(
            "section 8.6 must carry the re-plan label exactly once, and carries it {n} times"
        )];
    }
    let at = section.find(REPLAN_LABEL).unwrap_or(0);
    let after = &section[at + REPLAN_LABEL.len()..];
    let para = &after[..after.find("\n\n").unwrap_or(after.len())];
    let label = para.split("**").next().unwrap_or("").trim();
    let mut title = d.title.chars();
    let lowered: String = title
        .next()
        .map(|c| c.to_lowercase().chain(title).collect())
        .unwrap_or_default();
    let want = format!("R{}, {lowered}", d.row);
    if label != want {
        v.push(format!(
            "section 8.6 states the re-plan label '{label}' but the results give '{want}'"
        ));
    }
    let k = format!("cut density K = {}", d.cut_density_k);
    // `K = 2` must not be satisfied by `K = 25`: the text after it is not a digit.
    let k_ok = para
        .match_indices(&k)
        .any(|(i, m)| !para[i + m.len()..].starts_with(|c: char| c.is_ascii_digit()));
    if !k_ok {
        v.push(format!(
            "section 8.6's re-plan paragraph does not say '{k}'"
        ));
    }
    let (word, other) = if d.memory_shipped_t {
        ("at their shipped T", "T at most 16")
    } else {
        ("T at most 16", "at their shipped T")
    };
    if !para.contains(word) || para.contains(other) {
        v.push(format!(
            "section 8.6's re-plan paragraph does not state the memory verdict '{word}'"
        ));
    }
    v
}

/// The re-plan half of `table --check`.
fn check_replan(spec: &str, results: Option<&Results>) -> Vec<String> {
    let mut v = Vec::new();
    let block = match extract_replan_block(spec) {
        Ok(b) => b,
        Err(m) => return vec![m],
    };
    // Fail closed, like the 9.8 block: with a results file, a missing section, block or label is
    // a finding, so deleting or renaming what is checked cannot make `check` pass.
    let Some(r) = results else {
        if block.is_some() {
            v.push("the results file is absent while the spec has a re-plan block".to_string());
        }
        return v;
    };
    let section = section_86(spec);
    if section.is_none() {
        v.push("the spec has no section 8.6 (`### 8.6 `) to hold the re-plan decision".to_string());
    }
    match block {
        None => v.push(
            "section 8.6 has no generated re-plan block between the replan markers".to_string(),
        ),
        Some((at, b)) => {
            if !section.is_some_and(|(a, e)| at >= a && at < e) {
                v.push("the re-plan block is not inside section 8.6".to_string());
            }
            if b != render_replan_block(r) {
                v.push("the re-plan block in the spec differs from the one regenerated from the results file (drift)".to_string());
            }
        }
    }
    if let Some((a, e)) = section {
        v.extend(check_replan_label(&spec[a..e], &decide_replan(&r.rows)));
    }
    v
}

/// The block between the markers of `spec` (markers included), if both are there.
pub fn extract_block(spec: &str) -> Option<&str> {
    let a = spec.find(BEGIN)?;
    let e = spec.find(END)?;
    if e < a {
        return None;
    }
    let end = e + END.len();
    let end = if spec[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    Some(&spec[a..end])
}

/// Replace the block of `spec` by `block` (the markers must be there).
pub fn splice(spec: &str, block: &str) -> Option<String> {
    let old = extract_block(spec)?;
    Some(spec.replacen(old, block, 1))
}

/// The text of section 8.5 of `spec`.
fn section_85(spec: &str) -> Option<&str> {
    let a = spec.find("### 8.5 ")?;
    let rest = &spec[a..];
    let end = rest[8..].find("\n## ").map(|i| i + 8).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// The row (1 to 3) that section 8.5 states as the decision, if it states one.
pub fn stated_decision(spec: &str) -> Option<u8> {
    let s = section_85(spec)?;
    let at = s.find("**Decision (VOS-028b):**")?;
    let after = &s[at..];
    TITLES
        .iter()
        .position(|t| after.contains(t))
        .map(|i| i as u8 + 1)
}

/// `table --check`: every reason the spec and the results file disagree.
pub fn check_spec(spec: &str, results: Option<&Results>) -> Vec<String> {
    // A CRLF copy of the spec (a Windows checkout) is the same document: compare on `\n` text.
    let spec = spec.replace("\r\n", "\n");
    let mut v = check_table_and_decision(&spec, results);
    v.extend(check_replan(&spec, results));
    v
}

/// The 9.8 block and the 8.5 decision half of [`check_spec`].
fn check_table_and_decision(spec: &str, results: Option<&Results>) -> Vec<String> {
    let mut v = Vec::new();
    let block = extract_block(spec);
    let present = block.is_some_and(|b| b.len() > BEGIN.len() + END.len() + 2);
    let stated = stated_decision(spec);
    match (present, results) {
        (true, None) => {
            v.push("the results file is absent while the spec has a generated block".to_string())
        }
        (false, Some(_)) => v.push(
            "the results file exists but the spec has no generated block between the markers"
                .to_string(),
        ),
        (true, Some(r)) => {
            if block != Some(render_block(r).as_str()) {
                v.push("the generated block in the spec differs from the table regenerated from the results file (drift)".to_string());
            }
        }
        (false, None) => {}
    }
    if let Some(row) = stated {
        if !present {
            v.push(format!("section 8.5 states a decision (row {row}) while the generated block is absent or empty"));
        }
        if let Some(r) = results {
            let computed = decide(&r.rows).row;
            if computed != row {
                v.push(format!(
                    "section 8.5 states row {row} but the results give row {computed}"
                ));
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::Meta;
    use crate::row::{Control, Knob};

    fn row(exp: &str, knob: Knob, mode: &str, seed: u64, result: &str, green: bool) -> Row {
        let mut r = Row::new(exp, knob, mode, seed, 5);
        r.result = result.into();
        r.green = green;
        r.peak_rss_kb = 2048;
        r.ands = 100;
        r.vars = 120;
        r.clauses = 300;
        r.entropy_bits = 8;
        r.declared_bits = 8;
        r.conflicts = 10 + seed;
        r.solve_ms = 1.5;
        r.worst_step_solve_ms = 1.5;
        r.drat_bytes = Some(2048);
        r.check_work = Some(12345);
        r.pass_default_limits = Some(true);
        r.control = Control {
            result: "sat".into(),
            model_checks_cnf: true,
            replayed: true,
        };
        r
    }

    fn sample() -> Results {
        let mut rows = Vec::new();
        for seed in 1..=5 {
            rows.push(row(
                "A1",
                Knob::of(&[("n", Kv::I(8))]),
                "monolithic",
                seed,
                "unsat",
                true,
            ));
        }
        for seed in 1..=5 {
            let mut r = row(
                "B2",
                Knob::of(&[("length", Kv::I(25)), ("divergence", Kv::F(0.25))]),
                "monolithic",
                seed,
                "unknown",
                false,
            );
            r.cap = "wall".into();
            rows.push(r);
        }
        Results {
            meta: Meta {
                name: "vos-capacity-probe".into(),
                version: "0.1.0".into(),
                binary_sha256: "00".into(),
                dirty_tree: false,
                triple: "t".into(),
                rustc: "rustc 1".into(),
                os: "os".into(),
                revisions: vec![],
                regeneration: None,
                merges: vec![],
                recording: None,
                trees: vec![],
            },
            rows,
        }
    }

    #[test]
    fn the_block_starts_with_the_banner_and_the_limits_and_uses_the_maximum() {
        let b = render_block(&sample());
        assert!(b.starts_with(BEGIN));
        assert!(b.contains("PROXY: synthetic hand-written code with synthetic divergence"));
        assert!(b.contains("Full tier not independently regenerated."));
        for i in 1..=10 {
            assert!(b.contains(&format!("\n{i}. ")), "limit {i}");
        }
        // The A1 line takes the largest conflict count over seeds (10 + 5).
        let a1 = b.lines().find(|l| l.starts_with("| A1 ")).unwrap();
        assert!(a1.contains("| 15 |"), "{a1}");
        assert!(
            a1.contains("GREEN") && a1.contains("replayed") && a1.contains("5/5"),
            "{a1}"
        );
        let b2 = b.lines().find(|l| l.starts_with("| B2 ")).unwrap();
        assert!(b2.contains("unknown (timeout)"), "{b2}");
        assert!(b2.contains("div=25%"), "{b2}");
        assert!(b.ends_with(&format!("{END}\n")));
    }

    #[test]
    fn derived_constants_come_from_the_rows() {
        let mut r = sample();
        // Two solved monolithic rows: one accepted by the default limits, one rejected for work.
        for (i, row) in r
            .rows
            .iter_mut()
            .filter(|x| x.experiment == "A1")
            .take(2)
            .enumerate()
        {
            row.conflicts = if i == 0 { 50_000 } else { 400_000 };
            row.solve_ms = 1000.0;
            row.drat_bytes = Some(row.conflicts * 200);
            row.check_ms = Some(2000.0);
            row.check_work = Some(row.conflicts * 10_000);
            row.pass_default_limits = Some(i == 0);
            row.note = if i == 0 {
                String::new()
            } else {
                "default work limit 2^32 exceeded at step 9".into()
            };
            row.peak_rss_kb = 1_048_576;
        }
        let lines = derived_constants(&r.rows).join("\n");
        assert!(
            lines.contains("50,000 to 400,000 conflicts per second"),
            "{lines}"
        );
        assert!(lines.contains("200 to 200 bytes per conflict"), "{lines}");
        assert!(lines.contains("2.0x to 2.0x the solve time"), "{lines}");
        assert!(lines.contains("the largest proof accepted has 50,000 conflicts, the smallest rejected for work has 400,000"), "{lines}");
        assert!(lines.contains("GB per million conflicts"), "{lines}");
        assert!(render_block(&r).contains("Derived constants (experiment F)"));
        // No solved row: said so, not omitted.
        assert!(derived_constants(&[]).join(" ").contains("no solved row"));
        // Accepted only: no rejection to name.
        let mut only = sample();
        only.rows[0].conflicts = 5000;
        only.rows[0].solve_ms = 10.0;
        assert!(derived_constants(&only.rows)
            .join(" ")
            .contains("none was rejected for work"));
    }

    fn d3(seed: u64, result: &str, green: bool, conflicts: u64, worst: u64) -> Row {
        let mut r = row(
            "D3",
            Knob::of(&[("t", Kv::I(8)), ("k", Kv::I(16))]),
            "split",
            seed,
            result,
            green,
        );
        r.steps = 8;
        r.conflicts = conflicts;
        r.worst_step_conflicts = Some(worst);
        r.instance_sha256 = format!("{}", seed % 2);
        r
    }

    #[test]
    fn a_groups_line_takes_every_cell_from_the_worst_row_and_not_the_last_one() {
        // The worst row (seed 2: most conflicts, no proof) is neither first nor last; a green
        // row and a smaller red row with a proof surround it.
        let mut a = d3(1, "unsat", false, 1_000, 500);
        a.drat_bytes = Some(111_111);
        a.pass_default_limits = Some(true);
        a.entropy_bits = 7;
        let mut b = d3(2, "unknown", false, 2_000, 900);
        b.cap = "wall".into();
        b.drat_bytes = None;
        b.check_work = None;
        b.pass_default_limits = None;
        b.entropy_bits = 99;
        b.declared_bits = 100;
        b.control = Control::none("no SAT control");
        b.solve_ms = 5_000.0;
        let mut c = d3(3, "unsat", true, 10, 10);
        c.solve_ms = 90_000.0;
        c.peak_rss_kb = 9 * 1024;
        let rows = vec![a, b, c];
        let l = line(&groups(&rows)[0]);
        assert!(l.contains("2,000 / 900"), "{l}");
        assert!(!l.contains("108.5 kB") && !l.contains("pass"), "{l}");
        assert!(l.contains("99/100"), "entropy from the same row: {l}");
        assert!(
            l.contains("no SAT control"),
            "control from the same row: {l}"
        );
        assert!(l.contains("(2 distinct)") && l.contains("3/3"), "{l}");
        assert!(l.contains("unknown (timeout)"), "{l}");
        // The hidden harder time and memory of another seed are in the group-level cell.
        assert!(l.contains("1.5 min / 9 MB"), "{l}");
    }

    #[test]
    fn ties_go_to_the_lowest_seed_whatever_the_file_order() {
        let mk = |seed: u64| {
            let mut r = d3(seed, "unsat", false, 100, 100);
            r.entropy_bits = seed;
            r.declared_bits = 10;
            r
        };
        for order in [vec![3, 1, 2], vec![2, 3, 1], vec![1, 2, 3]] {
            let rows: Vec<Row> = order.into_iter().map(mk).collect();
            let basis: Vec<&Row> = rows.iter().collect();
            assert_eq!(worst_row(&basis).seed, 1);
        }
    }

    #[test]
    fn collapsed_rows_do_not_count_as_distinct_instances() {
        let mut a = d3(1, "unsat", true, 5, 5);
        a.instance_sha256 = "x".into();
        let mut b = d3(2, "collapsed", false, 0, 0);
        b.instance_sha256 = "y".into();
        let rows = vec![a, b];
        assert!(line(&groups(&rows)[0]).contains("1/2 (1 distinct)"));
    }

    #[test]
    fn values_from_the_file_cannot_break_a_row_or_the_block() {
        assert_eq!(cell("a|b\nc"), "a\\|b c");
        assert!(!cell("<!-- vos30-table:end -->").contains("<!--"));
        let mut r = sample();
        r.rows[0].knob = Knob::of(&[("n", Kv::S("x|y\n<!-- vos30-table:end -->".into()))]);
        let block = render_block(&r);
        assert_eq!(block.matches(END).count(), 1);
        let line = block
            .lines()
            .find(|l| l.contains("x\\|y"))
            .expect("escaped line");
        assert!(!line.contains('\n'));
    }

    #[test]
    fn a_regenerated_banner_names_the_reviewer_and_the_host() {
        let mut r = sample();
        r.meta.regeneration = Some(crate::results::Regeneration {
            reviewer: "A. Reviewer".into(),
            host: "host-1".into(),
            run_id: "123".into(),
            run_attempt: "2".into(),
            head_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            unsigned_sha256: "ab".repeat(32),
            verified: crate::results::Verified {
                run_id: "456".into(),
                run_attempt: "1".into(),
                head_sha: "fedcba9876543210fedcba9876543210fedcba98".into(),
                verify_sha256: "cd".repeat(32),
                trees: Vec::new(),
            },
        });
        let b = render_block(&r);
        assert!(b.contains("Full tier regenerated by A. Reviewer on host-1. Recording: run 123 attempt 2 at revision 0123456789ab. Verified: run 456 attempt 1 at revision fedcba987654, verify file SHA-256 cdcdcdcdcdcdcdcd."));
        assert!(!b.contains("not independently regenerated"));
    }

    #[test]
    fn collapsed_seeds_are_excluded_and_said_so() {
        let mut r = sample();
        r.rows[0].result = "collapsed".into();
        r.rows[0].green = false;
        let b = render_block(&r);
        let a1 = b.lines().find(|l| l.starts_with("| A1 ")).unwrap();
        assert!(a1.contains("4/5"), "{a1}");
        assert!(
            a1.contains("GREEN"),
            "the four counted seeds are green: {a1}"
        );
        let mut all = sample();
        for row in all.rows.iter_mut().filter(|r| r.experiment == "A1") {
            row.result = "collapsed".into();
            row.green = false;
        }
        let a1 = render_block(&all)
            .lines()
            .find(|l| l.starts_with("| A1 "))
            .unwrap()
            .to_string();
        assert!(a1.contains("0/5") && a1.ends_with("collapsed |"), "{a1}");
    }

    fn spec_with(block: Option<&str>, decision: Option<&str>) -> String {
        let mut s = String::from("### 8.4 Limits\n\ntext\n\n### 8.5 Current decision state\n\n");
        if let Some(d) = decision {
            s.push_str(&format!("**Decision (VOS-028b):** {d}\n"));
        }
        s.push_str("\n---\n\n## 9. The probe\n\n");
        if let Some(b) = block {
            s.push_str(b);
        } else {
            s.push_str("(no table yet)\n");
        }
        s.push_str("\n## 10. Next\n");
        s
    }

    #[test]
    fn check_passes_when_the_block_is_current_and_fails_on_drift() {
        let r = sample();
        let block = render_block(&r);
        assert!(check_table_and_decision(&spec_with(Some(&block), None), Some(&r)).is_empty());
        let drifted = block.replace("GREEN", "green");
        let v = check_table_and_decision(&spec_with(Some(&drifted), None), Some(&r));
        assert!(v.iter().any(|m| m.contains("drift")), "{v:?}");
    }

    #[test]
    fn check_fails_when_one_side_is_missing() {
        let r = sample();
        let block = render_block(&r);
        assert!(
            check_table_and_decision(&spec_with(Some(&block), None), None)
                .iter()
                .any(|m| m.contains("absent"))
        );
        assert!(check_table_and_decision(&spec_with(None, None), Some(&r))
            .iter()
            .any(|m| m.contains("no generated block")));
        assert!(check_table_and_decision(&spec_with(None, None), None).is_empty());
        let empty = format!("{BEGIN}\n{END}\n");
        let v = check_table_and_decision(&spec_with(Some(&empty), Some(TITLES[0])), None);
        assert!(v.iter().any(|m| m.contains("absent or empty")), "{v:?}");
    }

    #[test]
    fn a_stated_decision_must_match_the_computed_one() {
        let r = sample();
        let block = render_block(&r);
        let computed = decide(&r.rows);
        assert_eq!(
            computed.row, 1,
            "the sample lacks the required shapes: unclassified is red"
        );
        let ok = spec_with(Some(&block), Some(&format!("{} (row 1)", TITLES[0])));
        assert!(
            check_table_and_decision(&ok, Some(&r)).is_empty(),
            "{:?}",
            check_table_and_decision(&ok, Some(&r))
        );
        let wrong = spec_with(Some(&block), Some(&format!("{} (row 3)", TITLES[2])));
        let v = check_table_and_decision(&wrong, Some(&r));
        assert!(v.iter().any(|m| m.contains("states row 3")), "{v:?}");
        assert_eq!(stated_decision(&ok), Some(1));
        assert_eq!(stated_decision(&spec_with(None, None)), None);
    }

    #[test]
    fn splice_replaces_only_the_block() {
        let r = sample();
        let old = spec_with(Some(&format!("{BEGIN}\nold\n{END}\n")), None);
        let new = splice(&old, &render_block(&r)).unwrap();
        assert!(new.contains("PROXY:") && !new.contains("\nold\n"));
        assert!(new.starts_with("### 8.4") && new.ends_with("## 10. Next\n"));
        assert!(splice("no markers", "x").is_none());
        assert!(extract_block(&format!("{END}\n{BEGIN}\n")).is_none());
    }

    #[test]
    fn number_formatting_is_stable() {
        assert_eq!(fmt_count(1234567), "1,234,567");
        assert_eq!(fmt_count(12), "12");
        assert_eq!(fmt_ms(0.2), "<1 ms");
        assert_eq!(fmt_ms(250.0), "250 ms");
        assert_eq!(fmt_ms(2500.0), "2.5 s");
        assert_eq!(fmt_ms(120_000.0), "2.0 min");
        assert_eq!(fmt_bytes(10), "10 B");
        assert_eq!(fmt_bytes(2048), "2.0 kB");
        assert_eq!(fmt_bytes(3 * 1048576), "3.0 MB");
    }
}
