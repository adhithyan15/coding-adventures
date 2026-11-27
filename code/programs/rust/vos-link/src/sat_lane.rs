//! # CK11: the model-SAT lane's files (VOS-040c, VOS31 10.2)
//!
//! A contract crate whose `vos.toml` says `sat = "model"` opts in to the host model-SAT lane
//! (`vos-model-sat`, VOS31).  The lane certifies the LEMMAS of the contract's model; it says
//! nothing about any implementation.  `vos-link` cannot call the lane (the Cargo edge runs the
//! other way: `vos-model-sat` depends on `vos-link`), so this module checks **files only**, and
//! the BUILD line the generator writes does the solving:
//!
//! ```text
//!   vos.toml  sat = "model" ─┐
//!   BUILD, BUILD_windows ────┼─► CK11 (this module): the lane line is there, the table is well
//!   warrant/lemmas.sat ──────┤        formed, names every lemma at every width, is current
//!   warrant/lemmas.part ─────┘        against the two parts, and at least one obligation RAN
//!
//!   BUILD line ──► vos-model-sat check .   re-derives every row and compares byte for byte
//! ```
//!
//! **None ran = fail** (VOS04 5.2 lines 6 to 10): a lane that is silently skipped reports green,
//! so an empty table, a lemma with no row, or a table of only trivial rows is a finding.
//!
//! What CK11 does NOT decide is whether a row is true: that is the lane's job, and the BUILD
//! line runs it.  The two checks cover each other: if the line is absent CK11 says so; if the
//! table is hand-edited the lane's byte comparison says so.

use vos_manifest::digest::sha256_hex;
use vos_shape_check::manifest::SatLane;

use crate::build_gen::{MARKER, SAT_LINE};
use crate::contract_check::{ContractInputs, SAT_TABLE_FILE};
use crate::error::Finding;
use crate::fsio::esc;

/// The width ladder of VOS31 section 4.  The lane's own constant is `vos_model_sat::LADDER`; a
/// test of `vos-model-sat` asserts the two are equal.
pub const SAT_LADDER: [u64; 4] = [8, 16, 32, 64];

/// The label token of a trivial row (VOS31 section 11, `_` for spaces): printed, never counted.
pub const SAT_TRIVIAL_LABEL: &str = "trivial_(inv_is_true)";

/// What CK11 learned from a table.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SatTableFacts {
    /// Rows that are not trivial: the obligations that ran (VOS31 7.5, "counted").
    pub counted: usize,
    /// Rows in all.
    pub rows: usize,
}

fn is_decimal(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Escape and cut a text for a finding.
fn show(s: &str) -> String {
    esc(&s.chars().take(120).collect::<String>())
}

/// Judge a committed `warrant/lemmas.sat` **as text, by position**, against the claims of
/// `lemmas.part` and the digests of the two parts.  Returns the reasons it is wrong (empty when
/// it is fine) and what it holds.
///
/// ```text
///   vos-sat 1
///   contract-part <sha256>        equals the sha256 of contract.part as it is now
///   lemmas-part <sha256>          equals the sha256 of lemmas.part as it is now
///   route-budget conflicts <n> check-work <n>
///   lemma <name> n <N> label <l> and <k> vars <v> clauses <c> cnf <sha> verdict unsat
///         collapsed <r> control sat premise <p>     ONE row per claim of lemmas.part, in claim
///                                                    order, per width of the ladder, ascending
///   trivial <name>                zero or more, each naming a claim
///   end
/// ```
pub fn judge_sat_table(
    table: &str,
    claims: &[String],
    contract_sha: &str,
    lemmas_sha: &str,
) -> (Vec<String>, SatTableFacts) {
    let mut bad: Vec<String> = Vec::new();
    let mut facts = SatTableFacts::default();
    if !table.ends_with('\n') {
        bad.push("the table does not end with a newline".to_string());
    }
    let lines: Vec<&str> = table.lines().collect();
    let header = [
        ("vos-sat 1".to_string(), "vos-sat 1"),
        (format!("contract-part {contract_sha}"), "contract-part"),
        (format!("lemmas-part {lemmas_sha}"), "lemmas-part"),
    ];
    for (i, (want, key)) in header.iter().enumerate() {
        match lines.get(i) {
            Some(l) if l == want => {}
            Some(l) if i > 0 && l.split(' ').next() == Some(*key) => bad.push(format!(
                "line {}: `{}` is not the digest of the part as it is now (regenerate with `vos-model-sat gen .`)",
                i + 1,
                show(l)
            )),
            other => bad.push(format!(
                "line {}: expected `{key}`, found `{}`",
                i + 1,
                show(other.copied().unwrap_or("(end of file)"))
            )),
        }
    }
    let budget_ok = lines.get(3).is_some_and(|l| {
        let w: Vec<&str> = l.split(' ').collect();
        w.len() == 5
            && w[0] == "route-budget"
            && w[1] == "conflicts"
            && w[3] == "check-work"
            && is_decimal(w[2])
            && is_decimal(w[4])
    });
    if !budget_ok {
        bad.push(format!(
            "line 4: expected `route-budget conflicts <n> check-work <n>`, found `{}`",
            show(lines.get(3).copied().unwrap_or("(end of file)"))
        ));
    }
    // The rows, in claim order and ascending width.
    let mut at = 4;
    for claim in claims {
        for n in SAT_LADDER {
            // A slot that does not hold a `lemma` row (the table ended early, or `end` or a
            // `trivial` line came first) is one missing row; the line is left for the tail.
            let Some(line) = lines.get(at).filter(|l| l.starts_with("lemma ")) else {
                bad.push(format!(
                    "no row for lemma `{claim}` at N = {n} (the table ends early)"
                ));
                continue;
            };
            at += 1;
            let w: Vec<&str> = line.split(' ').collect();
            let shape = w.len() == 22
                && w[0] == "lemma"
                && w[2] == "n"
                && w[4] == "label"
                && w[6] == "and"
                && w[8] == "vars"
                && w[10] == "clauses"
                && w[12] == "cnf"
                && w[14] == "verdict"
                && w[16] == "collapsed"
                && w[18] == "control"
                && w[20] == "premise";
            if !shape {
                bad.push(format!(
                    "line {at}: expected the row of lemma `{claim}` at N = {n}, found `{}`",
                    show(line)
                ));
                continue;
            }
            facts.rows += 1;
            if w[1] != claim || w[3] != n.to_string() {
                bad.push(format!(
                    "line {at}: expected the row of lemma `{claim}` at N = {n}, found lemma `{}` at N = {}",
                    show(w[1]),
                    show(w[3])
                ));
            }
            let sha_ok = w[13].len() == 64
                && w[13]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if !(is_decimal(w[7]) && is_decimal(w[9]) && is_decimal(w[11]) && sha_ok) {
                bad.push(format!(
                    "line {at}: the counts or the CNF digest of lemma `{claim}` at N = {n} are malformed"
                ));
            }
            if w[15] != "unsat" || w[19] != "sat" {
                bad.push(format!(
                    "line {at}: lemma `{claim}` at N = {n} must say `verdict unsat` and `control sat`"
                ));
            }
            if !["no", "constant-fold", "propagation-only"].contains(&w[17]) {
                bad.push(format!(
                    "line {at}: lemma `{claim}` at N = {n}: collapsed must be no, constant-fold or propagation-only"
                ));
            }
            if !["witnessed", "none"].contains(&w[21]) {
                bad.push(format!(
                    "line {at}: lemma `{claim}` at N = {n}: premise must be witnessed or none"
                ));
            }
            if w[5] != SAT_TRIVIAL_LABEL {
                facts.counted += 1;
            }
        }
    }
    // `trivial <claim>` lines, then `end`, and nothing after it.
    let mut ended = false;
    for (i, line) in lines.iter().enumerate().skip(at) {
        if ended {
            bad.push(format!("line {}: text after `end`", i + 1));
            break;
        }
        if *line == "end" {
            ended = true;
        } else if line
            .strip_prefix("trivial ")
            .is_some_and(|c| claims.iter().any(|x| x == c))
        {
            // a trivial lemma: printed, never counted
        } else {
            bad.push(format!(
                "line {}: unexpected `{}` (a row beyond the claims of lemmas.part, or not `trivial <lemma>` or `end`)",
                i + 1,
                show(line)
            ));
        }
    }
    if !ended {
        bad.push("the table has no closing `end` line".to_string());
    }
    if facts.counted == 0 {
        bad.push(
            "zero counted obligations: none ran is a failure (VOS04 5.2 lines 6 to 10)".to_string(),
        );
    }
    (bad, facts)
}

/// The part of a BUILD file the generator owns: everything before the marker line.
fn generated_region(text: &str) -> &str {
    text.split(MARKER).next().unwrap_or("")
}

/// CK11, the whole rule, over what `load_inputs` read.
///
/// | `vos.toml` | BUILD and BUILD_windows | `warrant/lemmas.sat` | result |
/// |---|---|---|---|
/// | `sat = "model"` | the lane line in the generated region | present, well formed, covers every claim, at least one counted row | clean |
/// | `sat = "model"` | the line missing in either file | | CK11 |
/// | `sat = "model"` | | absent, malformed, partial, stale or empty | CK11 |
/// | no key | no lane line | absent | clean (opt-in, VOS31 10.3) |
/// | no key | the lane line | | CK11: an unrequested SAT line |
/// | no key | | present | CK11: an unrequested SAT table |
///
/// The last two rows are the decision this PR makes where VOS31 10.3 was silent: a contract
/// without the key "ships no encoder and prints no lemma SAT line", so a stray table or BUILD
/// line is a finding, not an extra.  A SAT claim nobody opted into is exactly the over-claim
/// VOS31 section 14 warns about.  The BUILD bytes are also compared whole by CC03 of
/// `check-component`; CK11 is the rule that NAMES the missing or stray lane line.
pub fn check_sat_lane(inp: &ContractInputs, claims: &[String]) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut f = |m: String| out.push(Finding::new("CK11", m));
    match inp.sat {
        Some(SatLane::Model) => {
            for (name, text) in &inp.builds {
                let Some(text) = text else {
                    continue; // CC02 reports a missing BUILD file
                };
                if !generated_region(text).lines().any(|l| l == SAT_LINE) {
                    f(format!(
                        "{name}: vos.toml says `sat = \"model\"` but the generated region has no `{SAT_LINE}` line (run `vos-link gen-build`)"
                    ));
                }
            }
            match &inp.sat_table {
                None => f(format!(
                    "{SAT_TABLE_FILE}: not found, and vos.toml says `sat = \"model\"` (run `vos-model-sat gen .`)"
                )),
                Some(t) => {
                    let (bad, _) = judge_sat_table(
                        t,
                        claims,
                        &sha256_hex(inp.contract_part.text().as_bytes()),
                        &sha256_hex(inp.lemmas_part.text().as_bytes()),
                    );
                    for b in bad {
                        f(format!("{SAT_TABLE_FILE}: {b}"));
                    }
                }
            }
        }
        None => {
            for (name, text) in &inp.builds {
                let stray = text.as_deref().is_some_and(|t| {
                    generated_region(t)
                        .lines()
                        .any(|l| !l.trim_start().starts_with('#') && l.contains("vos-model-sat"))
                });
                if stray {
                    f(format!(
                        "{name}: runs the model-SAT lane but vos.toml has no `sat = \"model\"` (the lane is opt-in per contract, VOS31 10.3)"
                    ));
                }
            }
            if inp.sat_table.is_some() {
                f(format!(
                    "{SAT_TABLE_FILE}: exists but vos.toml has no `sat = \"model\"` (the lane is opt-in per contract, VOS31 10.3)"
                ));
            }
        }
    }
    out
}
