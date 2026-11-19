//! # CC11: every `-any` has evidence that its operations keep the oracle (VOS-039c, VOS-D080)
//!
//! An `-any` scheduler is the model plus an **oracle register**: a field of its state that the
//! abstraction `alpha` drops and that only the evidence harness may write (VOS01 3.2).  The
//! safety claims of the contract quantify over `build(M)`, whose oracle is a constant, and `alpha`
//! forgets the oracle, so by themselves they cannot tell an `insert` that overwrites the oracle
//! from one that leaves it alone.  The contract therefore carries claims for it (VOS10 9.3,
//! `warrant/obligations-any.part`: `insert_oracle_*`, `remove_oracle_*`, `alpha_build_with_oracle`,
//! and `dispatch_sound`'s `.choice == c`), and CD07 requires every `any` obligation by digest.
//!
//! A claim is only as good as the fault that can fail it.  The record faults of the evidence
//! generator (`field-zero`, `field-flip`) rewrite the FIRST numeric field of a result, which for
//! `Impl(mask, choice)` is the bitmap, so until `field-flip-last` existed nothing could corrupt
//! the oracle and a claim that an operation keeps it had no negative control that could ever
//! catch a regression.  This module is the generic half of the answer: it derives, from the
//! parsed document and the bindings, what an `-any` crate must attack, and holds the crate's
//! bindings and sabotage plan to it.  Nothing is declared by the author and no claim text is
//! linted.
//!
//! ```text
//!   document (parsed)   model build(m) = Impl(e0, .., en)        which field is outside the cone
//!   evidence.bindings   ops with ret=Impl and an Impl argument   the SUCCESSOR operations
//!   evidence.bindings   faults= of each operation
//!   warrant/sabotage.plan, claim_calls                           which claim attacks which fault
//!
//!   CC11 (a)  exactly one field of Impl is outside the cone of build, and it is the LAST field
//!   CC11 (b)  every successor operation lists `field-flip-last`
//!   CC11 (c)  every successor operation has a plan entry `exhaustive/<claim> field-flip-last`
//!             on a claim that calls that successor and no other one
//! ```
//!
//! Whether the claim really FAILS under the fault is not decided here: the generated BUILD runs
//! the plan through `warrant_negctl.py`, and CC08 requires every in-harness control to pass.  A
//! contract whose `any` block lacks the preservation claims cannot satisfy (c), because no claim
//! of its document will call a successor alone and fail on a flipped oracle; the rule needs no
//! change to the `contract.vos` format.
//!
//! ## What the rule does not do
//!
//! * "The oracle is the last field" deliberately rejects a two-oracle `-any` (the extension is a
//!   named-field fault, adjacent to VOS-D101).
//! * It does not read the implementation's source: a lexical "never reads the oracle" rule is
//!   neither sufficient (a mutant that writes `choice: 0` never reads it) nor cheap (the honest
//!   `..s` and `return s` do read the field lexically), so the claims evidence the behaviour on
//!   the domain of VOS10 9.3 instead.

use std::collections::{BTreeMap, BTreeSet};

use coding_adventures_warrant_grammar::binding::{parse_evidence_bindings, HostSpec, HostTy};
use vos_manifest::warrant::{analyze_document_with_budget, BuildShape, TAINT_BUDGET};

use crate::build_gen::PlanEntry;
use crate::check::WARRANT_STACK_BYTES;
use crate::error::NoVerdict;
use crate::fsio::esc;

/// The fault that corrupts the oracle: the lowest bit of the last numeric field of a record
/// result (VER04j 4.8).  A test reads VOS04 5.6 and asserts this constant is the one it names.
pub const ORACLE_FAULT: &str = "field-flip-last";

/// The Warrant record every implementation's state is.
const STATE_TYPE: &str = "Impl";

/// What CC11 needs, all of it already parsed.
pub struct Facts<'a> {
    /// The shape of `model build` of the generated document.
    pub build: &'a BuildShape,
    /// The operations that produce a new state from a state (see [`successor_ops`]).
    pub successors: &'a BTreeSet<String>,
    /// The faults each bound operation lists (`evidence.bindings`).
    pub faults: &'a BTreeMap<String, Vec<String>>,
    /// The sabotage plan.
    pub plan: &'a [PlanEntry],
    /// For each claim, the bound operations it calls.
    pub calls: &'a [(String, Vec<String>)],
}

/// The operations that produce a state from a state: bound with `ret=Impl` and an `Impl`
/// among `args=`.  `insert_img`, `remove_img` and `dispatch_img` for the scheduler.  `init`
/// (no argument), the projections (`_out`, `alpha_w`) and `cfg_n` return something else.
pub fn successor_ops(hosts: &HostSpec) -> BTreeSet<String> {
    let is_state = |t: &HostTy| *t == HostTy::Named(STATE_TYPE.to_string());
    hosts
        .ops
        .iter()
        .filter(|(_, h)| {
            h.ret.as_ref().is_some_and(is_state)
                && h.args.as_ref().is_some_and(|a| a.iter().any(is_state))
        })
        .map(|(op, _)| op.clone())
        .collect()
}

/// CC11, pure: one message per failure.
pub fn judge(f: &Facts<'_>) -> Vec<String> {
    let mut out = Vec::new();

    // (a) The oracle is the one field outside the cone of `build`, and it is the last field.
    // Anything the walk cannot decide fails closed, as CD07 does for the same shape.
    match f.build {
        BuildShape::Constructor { in_cone } => {
            let outside: Vec<usize> = in_cone
                .iter()
                .enumerate()
                .filter(|(_, c)| !**c)
                .map(|(i, _)| i)
                .collect();
            let last = in_cone.len().checked_sub(1);
            if outside.len() != 1 || Some(outside[0]) != last {
                out.push(format!(
                    "oracle: the field(s) of `Impl` outside the cone of `build` are {}; `field-flip-last` can corrupt only the LAST field, so the oracle must be exactly the last field and the only one (a two-oracle `-any` needs a named-field fault, VOS-D101)",
                    if outside.is_empty() {
                        "none".to_string()
                    } else {
                        outside
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ));
            }
        }
        BuildShape::Missing => out.push(
            "oracle: the `build` model is missing, so the oracle field cannot be found".to_string(),
        ),
        BuildShape::NotConstructor => out.push(
            "oracle: the `build` model is not an `Impl(..)` constructor application, so the oracle field cannot be found (fails closed)".to_string(),
        ),
        BuildShape::BudgetExhausted | BuildShape::WideModel => out.push(
            "oracle: the cone walk could not decide the fields of `build`, so the oracle field cannot be found (fails closed)".to_string(),
        ),
    }

    // A crate with no successor operation has nothing that could keep or lose an oracle; an
    // `-any` whose bindings hide its successors would otherwise pass (b) and (c) by having none.
    if f.successors.is_empty() {
        out.push(format!(
            "oracle: no bound operation has `ret={STATE_TYPE}` and an `{STATE_TYPE}` argument, so no operation is known to keep the oracle (the bindings must bind the state-to-state operations, `insert_img` and its kind)"
        ));
    }

    for op in f.successors {
        // (b) The fault is bound, so the harness can attack the operation.
        let listed = f
            .faults
            .get(op)
            .is_some_and(|fs| fs.iter().any(|x| x == ORACLE_FAULT));
        if !listed {
            out.push(format!(
                "oracle: operation {} produces a state from a state but does not list `{ORACLE_FAULT}` in `faults=`, so nothing can corrupt the oracle it must keep",
                esc(op)
            ));
        }
        // (c) A plan entry on a claim that calls this successor and no other one: a claim
        // calling two would be failed by a flip in either, and so prove neither.
        let attacked = f.plan.iter().any(|e| {
            e.tier == "exhaustive"
                && e.fault == ORACLE_FAULT
                && f.calls.iter().any(|(claim, called)| {
                    *claim == e.name
                        && called.iter().any(|c| c == op)
                        && !called
                            .iter()
                            .any(|c| c != op && f.successors.contains(c.as_str()))
                })
        });
        if !attacked {
            out.push(format!(
                "oracle: no `exhaustive/<claim> {ORACLE_FAULT}` entry of the sabotage plan names a claim that calls {} and no other state-producing operation, so a regression that overwrites the oracle is not shown to fail any claim",
                esc(op)
            ));
        }
    }
    out
}

/// The inputs read from a crate: the parsed document's `build` shape and the bindings' host
/// signatures.  `Err(message)` is a refusal by the tools on admitted bytes (a CC11 finding).
pub fn load(
    bindings: &str,
    document: String,
) -> Result<Result<(BuildShape, BTreeSet<String>), String>, NoVerdict> {
    let hosts = match parse_evidence_bindings(bindings) {
        Ok(b) => b.hosts,
        Err(e) => return Ok(Err(format!("evidence.bindings: {}", esc(&e)))),
    };
    let h = std::thread::Builder::new()
        .stack_size(WARRANT_STACK_BYTES)
        .spawn(move || analyze_document_with_budget(&document, TAINT_BUDGET))
        .map_err(|e| NoVerdict::new(format!("cannot start the Warrant thread: {e}")))?;
    let analysis = h
        .join()
        .map_err(|_| NoVerdict::new("the Warrant thread panicked"))?;
    Ok(match analysis {
        Ok(a) => Ok((a.build, successor_ops(&hosts))),
        Err(e) => Err(format!("the document: {}", esc(&e.to_string()))),
    })
}
