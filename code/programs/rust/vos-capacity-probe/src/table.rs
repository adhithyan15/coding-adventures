//! # `table`: the generated table of VOS30 section 9.8 and its drift check
//!
//! `table` reads the committed results file and writes, for each `(experiment, knob, mode)`
//! group, one line that uses the **maximum over seeds** (section 8.3: a median would let one hard
//! instance hide).  Collapsed seeds are excluded from every statistic and said so in the `Seeds`
//! column.  The block begins with a fixed banner and the six validity limits of section 8.4, so
//! a table can never be read without them, and it ends with the section 8.3 decision computed
//! from the same rows.
//!
//! ```text
//!   <!-- vos30-table:begin -->
//!   PROXY: ...banner...        the six validity limits        | the table |     decision
//!   <!-- vos30-table:end -->
//! ```
//!
//! [`check_spec`] is `table --check`: it regenerates the block and compares it byte for byte with
//! the spec (drift), fails if the results file is absent while the block is present (or the other
//! way round), and fails if section 8.5 states a decision while the block is absent, or states one
//! that differs from the decision computed from the rows.  No number in the block is typed.

use crate::results::{decide, groups, red_reason, Group, Results, TITLES};
use crate::row::{Kv, Row};

/// The opening marker.
pub const BEGIN: &str = "<!-- vos30-table:begin -->";
/// The closing marker.
pub const END: &str = "<!-- vos30-table:end -->";

/// The six validity limits of section 8.4, in one line each (they travel with every table).
pub const LIMITS: [&str; 6] = [
    "Hand-written code is not compiler output: no register allocation, spills, fusion or inlining; real code may be easier or harder.",
    "The divergence is synthetic: the reference differs from the code in controlled proportions (5%, 25%, 100% of the live instructions that have an alternative encoding), not as a real compiler's would.",
    "Trap entry, CSRs and `mret` are not modelled: only the ALU, `lw`/`lbu`/`sw`/`sb` and forward branches with a concrete PC.",
    "The probe bounds solver capacity per obligation shape; it does not prove any entry verifiable.",
    "One machine, one solver build: seconds are host-specific, conflicts and work are the portable part.",
    "The probe never raises the `drat-check` limits: a proof the default limits reject is a red row (the advisor's numbers raised them, so every proof there was checkable).",
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
    let worst = counted
        .iter()
        .find(|r| !r.green)
        .copied()
        .unwrap_or(counted[0]);
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
    let maxu = |f: fn(&Row) -> u64| basis.iter().map(|r| f(r)).max().unwrap_or(0);
    let ent = basis.iter().min_by(|a, b| {
        let fa = a.entropy_bits as f64 / a.declared_bits.max(1) as f64;
        let fb = b.entropy_bits as f64 / b.declared_bits.max(1) as f64;
        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let chain = first.steps > 1;
    let conflicts = if enum_row {
        dash()
    } else if chain {
        format!(
            "{} / {}",
            fmt_count(maxu(|r| r.conflicts)),
            fmt_count(
                basis
                    .iter()
                    .filter_map(|r| r.worst_step_conflicts)
                    .max()
                    .unwrap_or(0)
            )
        )
    } else {
        fmt_count(maxu(|r| r.conflicts))
    };
    let solve_max = basis.iter().map(|r| r.solve_ms).fold(0.0, f64::max);
    let worst_max = basis
        .iter()
        .map(|r| r.worst_step_solve_ms)
        .fold(0.0, f64::max);
    let solve = if chain {
        format!("{} ({} worst)", fmt_ms(solve_max), fmt_ms(worst_max))
    } else {
        fmt_ms(solve_max)
    };
    let drat = basis.iter().filter_map(|r| r.drat_bytes).max();
    let work = basis.iter().filter_map(|r| r.check_work).max();
    let limits = if basis.iter().any(|r| r.pass_default_limits == Some(false)) {
        "REJECT".to_string()
    } else if basis.iter().any(|r| r.pass_default_limits == Some(true)) {
        "pass".to_string()
    } else {
        dash()
    };
    let control = match basis.iter().find(|r| !r.control.ok()) {
        None => "replayed".to_string(),
        Some(r) => r.control.result.clone(),
    };
    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {}/{} | {} | {} | {} | {} | {} | {} MB | {} | {} |",
        first.experiment,
        knob_text(first),
        first.mode,
        if enum_row {
            dash()
        } else {
            fmt_count(maxu(|r| r.ands))
        },
        if enum_row {
            dash()
        } else {
            fmt_count(maxu(|r| r.vars))
        },
        if enum_row {
            dash()
        } else {
            fmt_count(maxu(|r| r.clauses))
        },
        if enum_row {
            dash()
        } else {
            ent.map(|r| {
                format!(
                    "{}/{}",
                    fmt_count(r.entropy_bits),
                    fmt_count(r.declared_bits)
                )
            })
            .unwrap_or_else(dash)
        },
        counted.len(),
        g.rows.len(),
        conflicts,
        solve,
        drat.map(fmt_bytes).unwrap_or_else(dash),
        work.map(fmt_count).unwrap_or_else(dash),
        limits,
        (maxu(|r| r.peak_rss_kb) as f64 / 1024.0).round() as u64,
        control,
        verdict(g),
    )
}

/// The generated block, markers included, ending with a newline.
pub fn render_block(res: &Results) -> String {
    let mut s = String::new();
    s.push_str(BEGIN);
    s.push('\n');
    let banner = match &res.meta.regeneration {
        None => "Full tier not independently regenerated.".to_string(),
        Some((who, host)) => format!("Full tier regenerated by {who} on {host}."),
    };
    s.push_str(&format!(
        "PROXY: synthetic hand-written code with synthetic divergence, not compiler\noutput. These numbers bound solver capacity per obligation shape; they do not\nprove any entry verifiable. Seeds >= 5, maximum shown. {banner}\n\n"
    ));
    s.push_str("Validity limits (VOS30 section 8.4), printed with every number:\n\n");
    for (i, l) in LIMITS.iter().enumerate() {
        s.push_str(&format!("{}. {l}\n", i + 1));
    }
    s.push_str(&format!(
        "\nRun: `{}` {}, `{}`, {}; the `check` re-run reproduces the quick-tier rows.\n\n",
        res.meta.name, res.meta.version, res.meta.rustc, res.meta.os
    ));
    s.push_str("| Experiment | Knob | Mode | ANDs | Vars | Clauses | Entropy bits | Seeds | Conflicts (total / worst) | Solve | DRAT | Check work | Default limits | RSS | Control | Verdict |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for g in groups(&res.rows) {
        s.push_str(&line(&g));
        s.push('\n');
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
            },
            rows,
        }
    }

    #[test]
    fn the_block_starts_with_the_banner_and_the_six_limits_and_uses_the_maximum() {
        let b = render_block(&sample());
        assert!(b.starts_with(BEGIN));
        assert!(b.contains("PROXY: synthetic hand-written code with synthetic divergence"));
        assert!(b.contains("Full tier not independently regenerated."));
        for i in 1..=6 {
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
    fn a_regenerated_banner_names_the_reviewer_and_the_host() {
        let mut r = sample();
        r.meta.regeneration = Some(("A. Reviewer".into(), "host-1".into()));
        assert!(render_block(&r).contains("Full tier regenerated by A. Reviewer on host-1."));
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
        assert!(check_spec(&spec_with(Some(&block), None), Some(&r)).is_empty());
        let drifted = block.replace("GREEN", "green");
        let v = check_spec(&spec_with(Some(&drifted), None), Some(&r));
        assert!(v.iter().any(|m| m.contains("drift")), "{v:?}");
    }

    #[test]
    fn check_fails_when_one_side_is_missing() {
        let r = sample();
        let block = render_block(&r);
        assert!(check_spec(&spec_with(Some(&block), None), None)
            .iter()
            .any(|m| m.contains("absent")));
        assert!(check_spec(&spec_with(None, None), Some(&r))
            .iter()
            .any(|m| m.contains("no generated block")));
        assert!(check_spec(&spec_with(None, None), None).is_empty());
        let empty = format!("{BEGIN}\n{END}\n");
        let v = check_spec(&spec_with(Some(&empty), Some(TITLES[0])), None);
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
            check_spec(&ok, Some(&r)).is_empty(),
            "{:?}",
            check_spec(&ok, Some(&r))
        );
        let wrong = spec_with(Some(&block), Some(&format!("{} (row 3)", TITLES[2])));
        let v = check_spec(&wrong, Some(&r));
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
