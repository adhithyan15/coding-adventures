//! Goal bindings at the gate: recompute, compare, list and sign (VER32 3.6, 9.2, PIF-020).
//!
//! # The problem
//!
//! A claim file says `goal "<claim id>" <theorem> sha256:<hex>`: "this theorem proves that
//! claim". The kernel checked the theorem's *proof*, but nothing yet says the theorem's
//! *statement* is the one a person signed. This module closes that gap, in four steps.
//!
//! 1. **Recompute (G4, G5).** For every goal record the gate takes the theorem's type from
//!    the kernel store, encodes it as the **binding CTE** ([`crate::cte`]) and hashes it.
//!    The digest written in the record is a *hint*: it is compared with the computed one and
//!    a difference is an error ([`CheckError::GoalDigestMismatch`]), never an input.
//! 2. **Compare with the goals list (9.2, R5).** The claim elaborator also emits a goals
//!    list, one line per claim: `claim-id`, one space, the **display CTE** of the goal (the
//!    form that keeps binder names), LF. The gate parses it and checks
//!    `erase(display) = binding` for the theorem's type, so the text a person reads and the
//!    term the kernel proved are the same term.
//! 3. **Compare with the sign-off (G6).** A sign-off that records goal digests must agree
//!    with the computed ones, claim by claim, in both directions.
//! 4. **Aggregate (PIF-020).** The goal-signing aggregate is the SHA-256 of the sorted lines
//!    `claim-id goal-digest base-environment-digest`, each line ended by LF, with both digests
//!    as 64 lowercase hex digits. VER32 does not fix the line terminator or the digest
//!    spelling; this is the choice made here (see the README).
//!
//! # R-GOL-4, twice
//!
//! The reader refuses a claim file that declares a constant occurring in a goal, but only
//! if it is told the goal's constants (gate-fed). The goals list supplies them before the
//! file is read ([`list_constants`]). After the read the gate repeats the check from the
//! *stored* goal terms ([`constants_of`]), so the rule holds even when no goals list was
//! given.

use crate::chain::Loaded;
use crate::control::{Role, Signoff};
use crate::cte::{goal_digest, Dterm};
use crate::digest::Digest;
use crate::error::CheckError;
use lean_kernel::{Expr, ExprKind, ExprStore, Name};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// One line of the goals list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalLine {
    pub claim_id: String,
    pub term: Dterm,
}

/// A goal the gate checked.
#[derive(Clone, Debug)]
pub struct CheckedGoal {
    pub claim_id: String,
    pub theorem: Name,
    /// The digest the gate computed from the theorem's type (G4).
    pub digest: Digest,
    /// Whether a goals list line was compared with this goal.
    pub listed: bool,
}

pub use crate::control::claim_id_ok;

/// The most goal lines a goals list may have: a hostile list cannot make the gate do unbounded work.
pub const MAX_GOAL_LINES: usize = 100_000;

/// Parse a goals list: one line per claim, `claim-id`, one space, display CTE, LF.
pub fn parse_goals_list(text: &str) -> Result<Vec<GoalLine>, CheckError> {
    let bad = |line, why| CheckError::BadGoalsList { line, why };
    let mut out: Vec<GoalLine> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    if text.is_empty() {
        return Ok(out);
    }
    let body = text
        .strip_suffix('\n')
        .ok_or_else(|| bad(1, "the last line has no LF"))?;
    for (i, line) in body.split('\n').enumerate() {
        let n = i + 1;
        // The CTE has no space (a space inside a string is written \u{20}), so the line is
        // exactly two fields joined by one space.
        let Some((id, cte)) = line.split_once(' ') else {
            return Err(bad(n, "expected `<claim id> <display CTE>`"));
        };
        if !claim_id_ok(id) {
            return Err(bad(n, "bad claim id"));
        }
        if !line
            .bytes()
            .all(|b| (0x21..=0x7e).contains(&b) || b == b' ')
            || cte.contains(' ')
        {
            return Err(bad(
                n,
                "irregular spacing or a byte outside printable ASCII",
            ));
        }
        if out.len() >= MAX_GOAL_LINES {
            return Err(bad(n, "too many goal lines"));
        }
        if !seen.insert(id.to_string()) {
            return Err(CheckError::DuplicateListedGoal(id.to_string()));
        }
        let term = Dterm::parse(cte).map_err(|error| CheckError::Cte {
            claim: id.to_string(),
            error,
        })?;
        out.push(GoalLine {
            claim_id: id.to_string(),
            term,
        });
    }
    Ok(out)
}

/// The constants occurring in the goals of a list (the reader's gate-fed `goal_constants`).
pub fn list_constants(list: &[GoalLine]) -> HashSet<Name> {
    list.iter().flat_map(|g| g.term.constants()).collect()
}

/// Every constant name that occurs in `e`, walking the DAG once with an explicit stack.
pub fn constants_of(store: &ExprStore, e: Expr) -> HashSet<Name> {
    let (mut seen, mut out): (HashSet<Expr>, HashSet<Name>) = (HashSet::new(), HashSet::new());
    let mut work = vec![e];
    while let Some(x) = work.pop() {
        if !seen.insert(x) {
            continue;
        }
        match store.kind(x) {
            ExprKind::Const(n, _) => {
                out.insert(n.clone());
            }
            ExprKind::App(f, a) => work.extend([*f, *a]),
            ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => work.extend([*t, *b]),
            ExprKind::Let(_, t, v, b) => work.extend([*t, *v, *b]),
            ExprKind::Proj(n, _, y) => {
                out.insert(n.clone());
                work.push(*y);
            }
            _ => {}
        }
    }
    out
}

/// The goal-signing aggregate of PIF-020: SHA-256 of the sorted lines
/// `claim-id goal-digest base-environment-digest`, each ended by LF.
pub fn aggregate(goals: &[CheckedGoal], base_environment: &Digest) -> Digest {
    let lines: BTreeSet<String> = goals
        .iter()
        .map(|g| {
            format!(
                "{} {} {}\n",
                g.claim_id,
                g.digest.hex(),
                base_environment.hex()
            )
        })
        .collect();
    Digest::of(lines.into_iter().collect::<String>().as_bytes())
}

/// Check every goal of a loaded chain (steps 1 to 3 of the module documentation).
pub fn check_goals(
    l: &Loaded,
    list: Option<&[GoalLine]>,
    signoff: Option<&Signoff>,
) -> Result<Vec<CheckedGoal>, CheckError> {
    // Models and preludes hold definitions only: a goal record in a base file is refused.
    if let Some(index) = l
        .files
        .iter()
        .position(|f| f.role != Role::Claim && !f.goals.is_empty())
    {
        return Err(CheckError::GoalInBaseFile { index });
    }
    let claim_file = l.files.iter().find(|f| f.role == Role::Claim);
    let declared: HashSet<&Name> = claim_file
        .map(|f| f.decls.iter().map(|d| &d.name).collect())
        .unwrap_or_default();
    let mut out: Vec<CheckedGoal> = Vec::new();
    // Index the list once: lookups by claim id are O(1), not a scan per claim.
    let by_id: HashMap<&str, &GoalLine> = list
        .map(|l| l.iter().map(|x| (x.claim_id.as_str(), x)).collect())
        .unwrap_or_default();
    for g in claim_file.map(|f| f.goals.as_slice()).unwrap_or_default() {
        let cte_err = |error| CheckError::Cte {
            claim: g.claim_id.clone(),
            error,
        };
        // The theorem's type, from the store: the only source the gate trusts for the digest.
        let ty = l
            .env
            .get(&g.theorem)
            .map(|d| d.ty)
            .ok_or_else(|| CheckError::Cte {
                claim: g.claim_id.clone(),
                error: crate::cte::CteError::Unsupported("theorem missing"),
            })?;
        let digest = goal_digest(&l.env.store, ty).map_err(cte_err)?;
        // G4: the record's digest is a hint; a different one is an error.
        if digest.hex() != g.digest {
            return Err(CheckError::GoalDigestMismatch {
                claim: g.claim_id.clone(),
                recorded: g.digest.clone(),
                computed: digest.hex(),
            });
        }
        // R-GOL-4 from the stored goal term.
        if let Some(c) = constants_of(&l.env.store, ty)
            .into_iter()
            .find(|c| declared.contains(c))
        {
            return Err(CheckError::GoalConstantRedeclared(crate::report::dotted(
                &c,
            )));
        }
        let mut listed = false;
        if list.is_some() {
            let line = by_id
                .get(g.claim_id.as_str())
                .ok_or_else(|| CheckError::GoalNotInList(g.claim_id.clone()))?;
            let erased = line.term.binding().map_err(cte_err)?;
            let binding = crate::cte::binding_cte(&l.env.store, ty).map_err(cte_err)?;
            if erased != binding {
                return Err(CheckError::DisplayBindingMismatch {
                    claim: g.claim_id.clone(),
                });
            }
            listed = true;
        }
        out.push(CheckedGoal {
            claim_id: g.claim_id.clone(),
            theorem: g.theorem.clone(),
            digest,
            listed,
        });
    }
    if let Some(list) = list {
        let have: HashSet<&str> = out.iter().map(|g| g.claim_id.as_str()).collect();
        if let Some(extra) = list.iter().find(|x| !have.contains(x.claim_id.as_str())) {
            return Err(CheckError::ListedGoalMissing(extra.claim_id.clone()));
        }
    }
    if let Some(s) = signoff {
        check_signed(&out, s, &l.base_environment)?;
    }
    Ok(out)
}

/// G6: the sign-off's goals (when it records any) agree with the computed ones in both
/// directions, and so does its aggregate (PIF-020) when it records one.
fn check_signed(goals: &[CheckedGoal], s: &Signoff, base: &Digest) -> Result<(), CheckError> {
    if !s.goals.is_empty() {
        let signed: BTreeMap<&str, &Digest> =
            s.goals.iter().map(|(c, d)| (c.as_str(), d)).collect();
        for g in goals {
            match signed.get(g.claim_id.as_str()) {
                None => return Err(CheckError::GoalNotSigned(g.claim_id.clone())),
                Some(d) if **d != g.digest => {
                    return Err(CheckError::SignedGoalMismatch {
                        claim: g.claim_id.clone(),
                        signed: d.hex(),
                        computed: g.digest.hex(),
                    })
                }
                Some(_) => {}
            }
        }
        let have: HashSet<&str> = goals.iter().map(|g| g.claim_id.as_str()).collect();
        if let Some((c, _)) = s.goals.iter().find(|(c, _)| !have.contains(c.as_str())) {
            return Err(CheckError::SignedGoalMissing(c.clone()));
        }
    }
    if let Some(expected) = s.goals_sha256 {
        let actual = aggregate(goals, base);
        if expected != actual {
            return Err(CheckError::AggregateMismatch {
                expected: expected.hex(),
                actual: actual.hex(),
            });
        }
    }
    Ok(())
}
