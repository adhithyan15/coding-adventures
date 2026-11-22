//! # CC09: the `bound` rows against the ceilings (VOS-039b-2b, the rest of VOS-D055)
//!
//! A contract (`contract.vos`) says how much an operation may cost; an implementation's
//! document says how much it claims to.  This module compares the two, per component.
//!
//! ## The mapping is derived, never declared
//!
//! VOS02 4.7 specifies a `realises=` option that would say which interface operation an `op`
//! of the document stands for.  It is not built, for two reasons: `warrant-grammar`'s binding
//! reader rejects an unknown `op` option (and that crate has a large fan-out), and an
//! author-declared mapping is gameable (point `insert_img` at the loosest ceiling).  So the
//! mapping is a function of the NAMES, computed here:
//!
//! ```text
//!   contract ceilings:   init  insert  remove  dispatch                (4 operations)
//!
//!   document op          ceiling it maps to            why
//!   ------------------   ---------------------------   ----------------------------------
//!   init                 init                          X itself is a ceiling operation
//!   insert_out           insert                        one suffix `_out` stripped
//!   insert_img           insert                        one suffix `_img` stripped
//!   dispatch_img         dispatch                      one suffix `_img` stripped
//!   alpha_w, cfg_n       (none)                        ghost-only: nothing to compare with
//!   insert_out_img       (none)                        ONE suffix only: `insert_out` is no ceiling
//! ```
//!
//! The suffixes are VOS02 4.4's fixed interim vocabulary ([`MAP_SUFFIXES`]); a test reads the
//! spec and asserts it.  The rule is applied to the operations of the PARSED document, which
//! `warrant-grammar` read; nothing is scanned.
//!
//! ## What fails (code `CC09`)
//!
//! * a row whose operation maps to a ceiling for the same metric, and whose bound is not a plain
//!   integer literal (an unknown value is not a small one) or is above that ceiling;
//! * for kind `production` and `any`: a mapped operation with no `steps` row or no `heap` row
//!   (an operation with no bound is unbounded);
//! * a ceiling operation with no mapped operation at all (nobody realises it).
//!
//! `depth` is compared when present; when absent it is a NOTE ("not evidenced"), because the
//! evidence has no depth observer yet (VER04h 7.3) and no scheduler declares it.  Kind
//! `reference` has no required rows.  A `bound` row of an unmapped operation is not compared
//! and is listed in a note.
//!
//! Not checked here: that a bound is the TRUE worst case (the step counter's evidence,
//! VOS-D089) and the lock side of NC9 (`check-profile`, VOS-034).

use std::collections::{BTreeMap, BTreeSet};

use vos_manifest::caps::{CONTRACT_MAX_BYTES, DOCUMENT_MAX_BYTES};
use vos_manifest::contract::{is_ident, parse_contract};
use vos_manifest::warrant::{bound_facts, BoundFacts};
use vos_manifest::Contract;

use crate::check::WARRANT_STACK_BYTES;
use crate::component::read_text;
use crate::error::NoVerdict;
use crate::fsio::{esc, read_regular_opt};
use crate::inputs::Layout;

/// The suffixes an interface operation's realisation may carry (VOS02 4.4): `X_out` is the
/// operation's result, `X_img` its image on the implementation state.  Exactly one is stripped.
pub const MAP_SUFFIXES: [&str; 2] = ["_out", "_img"];

/// The metrics a contract can ceil, in the order they are checked.
const METRICS: [&str; 3] = ["steps", "heap", "depth"];

/// What the comparison found: failures (each a `CC09` message) and notes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    /// One message per failure.
    pub findings: Vec<String>,
    /// What a pass does not establish, and what was left out.
    pub notes: Vec<String>,
}

/// The ceiling operation the document operation `op` maps to, or `None` (ghost-only).
///
/// `ceilings` is the set of operation names the contract has ceiling rows for.
pub fn ceiling_op(op: &str, ceilings: &BTreeSet<&str>) -> Option<String> {
    if ceilings.contains(op) {
        return Some(op.to_string());
    }
    for suffix in MAP_SUFFIXES {
        if let Some(stem) = op.strip_suffix(suffix) {
            // ONE suffix: the stem must itself be a ceiling operation.
            if ceilings.contains(stem) {
                return Some(stem.to_string());
            }
        }
    }
    None
}

/// Compare the document's `bound` rows with the contract's ceilings.  Pure.
///
/// `require_rows` is true for kinds `production` and `any` (a mapped operation must carry a
/// `steps` and a `heap` row); a `reference` crate's rows are optional and compared when present.
pub fn judge(require_rows: bool, contract: &Contract, facts: &BoundFacts) -> Verdict {
    let mut v = Verdict::default();
    let ceil_ops: BTreeSet<&str> = contract.ceilings.iter().map(|c| c.op.as_str()).collect();
    let ceiling_of = |op: &str, metric: &str| {
        contract
            .ceilings
            .iter()
            .find(|c| c.op == op && c.metric.name() == metric)
            .map(|c| c.value)
    };

    // Every document operation, mapped or ghost-only.
    let mut mapped: BTreeMap<&str, String> = BTreeMap::new();
    let mut ghost: BTreeSet<&str> = BTreeSet::new();
    for op in &facts.ops {
        match ceiling_op(op, &ceil_ops) {
            Some(c) => {
                mapped.insert(op.as_str(), c);
            }
            None => {
                ghost.insert(op.as_str());
            }
        }
    }

    // (c) a ceiling operation nobody realises.
    let realised: BTreeSet<&str> = mapped.values().map(String::as_str).collect();
    for c in &ceil_ops {
        if !realised.contains(c) {
            v.findings.push(format!(
                "bounds: the contract has ceilings for operation {} but no operation of the document maps to it (an operation {}, or {} with one suffix {} stripped, is required)",
                esc(c),
                esc(c),
                esc(c),
                MAP_SUFFIXES.join(" or ")
            ));
        }
    }

    // (a) every row of a mapped operation, against the ceiling of its metric.
    let mut ghost_rows: BTreeSet<&str> = BTreeSet::new();
    for row in &facts.rows {
        let Some(c) = ceiling_op(&row.op, &ceil_ops) else {
            ghost_rows.insert(row.op.as_str());
            continue;
        };
        let Some(ceiling) = ceiling_of(&c, &row.metric) else {
            continue; // a metric the contract does not ceil (`output`, ..): nothing to compare
        };
        match row.value {
            None => v.findings.push(format!(
                "bounds: bound {} row {} <= is not an integer literal, so it cannot be compared with the ceiling {ceiling} of {} (a bound that is not a number is not a bound)",
                esc(&row.op),
                esc(&row.metric),
                esc(&c)
            )),
            Some(n) if n > ceiling => v.findings.push(format!(
                "bounds: bound {} row {} <= {n} is above the ceiling {ceiling} of the contract operation {}",
                esc(&row.op),
                esc(&row.metric),
                esc(&c)
            )),
            Some(_) => {}
        }
    }

    // (b) `steps` and `heap` are required of a mapped operation; `depth` is a note.
    let mut no_depth: Vec<&str> = Vec::new();
    for (op, c) in &mapped {
        for metric in METRICS {
            let has = facts.rows.iter().any(|r| r.op == *op && r.metric == metric);
            if has {
                continue;
            }
            if metric == "depth" {
                if ceiling_of(c, "depth").is_some() {
                    no_depth.push(op);
                }
            } else if require_rows {
                v.findings.push(format!(
                    "bounds: operation {} (ceiling operation {}) has no {metric} row in a bound {} declaration: an operation without a bound is unbounded",
                    esc(op),
                    esc(c),
                    esc(op)
                ));
            }
        }
    }

    if !no_depth.is_empty() {
        v.notes.push(format!(
            "bounds: `depth` is not evidenced for {} (no `depth` row; VER04h 7.3 has no depth observer yet, so none is required; a `depth` row that is present is compared)",
            list(&no_depth)
        ));
    }
    if !ghost.is_empty() || !ghost_rows.is_empty() {
        let all: BTreeSet<&str> = ghost.union(&ghost_rows).copied().collect();
        let all: Vec<&str> = all.into_iter().collect();
        v.notes.push(format!(
            "bounds: {} map to no contract ceiling (ghost-only operations); their `bound` rows, if any, are not compared",
            list(&all)
        ));
    }
    v
}

fn list(names: &[&str]) -> String {
    names.iter().map(|n| esc(n)).collect::<Vec<_>>().join(", ")
}

/// What reading the inputs gave: a ready comparison, or findings that stop it.
pub enum Inputs {
    /// The contract and the document's bound facts.
    Ready(Box<Contract>, Box<BoundFacts>),
    /// A rule that could not be applied to readable bytes: a `CC09` message each.
    Stop(Vec<String>),
}

/// Read `contract.vos` of the contract package `contract` and the generated document of the
/// crate at `layout`, and parse the document's operations and bounds.
pub fn load(layout: &Layout, contract: Option<&str>) -> Result<Inputs, NoVerdict> {
    let Some(name) = contract.filter(|n| is_ident(n)) else {
        return Ok(Inputs::Stop(vec![
            "bounds: vos.toml names no valid `contract` package, so there are no ceilings to compare with".to_string(),
        ]));
    };
    let path = layout.package_dir(name).join("contract.vos");
    let Some(bytes) = read_regular_opt(&path, CONTRACT_MAX_BYTES)? else {
        return Ok(Inputs::Stop(vec![format!(
            "bounds: contract package {} has no contract.vos",
            esc(name)
        )]));
    };
    let parsed = match parse_contract(&bytes) {
        Ok(c) => c,
        Err(e) if e.is_cap() => return Err(NoVerdict::new(e.to_string())),
        Err(e) => {
            return Ok(Inputs::Stop(vec![format!(
                "bounds: contract.vos of {}: {}",
                esc(name),
                esc(&e.to_string())
            )]))
        }
    };
    let doc = layout.specs_dir().join(layout.doc_name());
    let Some(text) = read_text(&doc, DOCUMENT_MAX_BYTES)? else {
        return Ok(Inputs::Stop(vec![format!(
            "bounds: {} is missing, so there are no bound rows to compare",
            layout.doc_display()
        )]));
    };
    Ok(match parse_facts(text)? {
        Ok(f) => Inputs::Ready(Box::new(parsed), Box::new(f)),
        Err(why) => Inputs::Stop(vec![format!(
            "bounds: {}: {}",
            layout.doc_display(),
            esc(&why)
        )]),
    })
}

/// The Warrant parser recurses; run it on a thread with the stack the other commands use.
fn parse_facts(text: String) -> Result<Result<BoundFacts, String>, NoVerdict> {
    let h = std::thread::Builder::new()
        .stack_size(WARRANT_STACK_BYTES)
        .spawn(move || bound_facts(&text).map_err(|e| e.to_string()))
        .map_err(|e| NoVerdict::new(format!("cannot start the Warrant thread: {e}")))?;
    h.join()
        .map_err(|_| NoVerdict::new("the Warrant thread panicked"))
}
