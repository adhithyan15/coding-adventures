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
//!   document (parsed)   `op`s returning Impl with an Impl param  the SUCCESSOR operations
//!   evidence.bindings   faults= of each operation
//!   contract's `any` claims, by name + claim_sha256              which claims are the contract's
//!   warrant/sabotage.plan, claim_calls                           which claim attacks which fault
//!
//!   CC11 (a)  exactly one field of Impl is outside the cone of build, it is the LAST field, and
//!             it has a numeric type (the only thing `field-flip-last` can flip)
//!   CC11 (b)  every successor operation lists `field-flip-last`
//!   CC11 (c)  every successor operation has a plan entry `exhaustive/<claim> field-flip-last`
//!             on a CONTRACT-OWNED claim that calls that successor and no other one
//! ```
//!
//! ## Where the successors come from (VOS-039c security review F1)
//!
//! Not from the bindings.  `ret=` and `args=` of an `op` line are optional: omitted, the generator
//! falls back to the host type of the DOCUMENT's signature, so an operation bound as
//! `op insert_img export=.. faults=field-zero` is a state-to-state operation for the generator
//! and invisible to a rule that reads only the binding's spelling.  The successors are therefore
//! the `op`s of the generated document whose return type mentions `Impl` and that take an
//! `Impl` (or mutate one through an `inout` parameter); the bindings are consulted afterwards, for
//! the faults of each.  "Mentions" includes `Option<Impl>`, `Result<Impl, E>` and tuples: a type
//! the rule cannot classify is treated as a state (the sound direction), and the generator then
//! refuses `field-flip-last` on a non-record result, which leaves a finding.  A binding that
//! names no `op` of the document is itself a finding: the rule could not classify it.
//!
//! ## Whose claim attacks the fault (F3)
//!
//! A `field-flip-last` plan entry counts only on a claim the CONTRACT owns, i.e. a claim of the
//! document whose name and `claim_sha256` equal those of an obligation of the contract's `any`
//! tier (the same identity CD05 and CD06 use; the `tier=` attribute of a marker is never read).
//! An implementation-own claim that calls one successor and is attacked by the fault would
//! satisfy a name-blind rule while the contract's preservation claims stay unattacked.
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

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::fixed::FixedTy;
use vos_manifest::warrant::{
    analyze_document_with_budget, BuildShape, ClaimFact, OpFact, TAINT_BUDGET,
};

use crate::build_gen::PlanEntry;
use crate::check::WARRANT_STACK_BYTES;
use crate::error::NoVerdict;
use crate::fsio::esc;

/// The fault that corrupts the oracle: the lowest bit of the last numeric field of a record
/// result (VER04j 4.8).  A test reads VOS04 5.6 and asserts this constant is the one it names.
pub const ORACLE_FAULT: &str = "field-flip-last";

/// The Warrant record every implementation's state is.
const STATE_TYPE: &str = "Impl";

/// Can `field-flip-last` flip a field of this Warrant type?  The generator's own predicate
/// (`is_numeric` of `evidence.rs`: a fixed-width integer or `Nat`) over the type NAMES of a v1
/// document, reusing its `FixedTy` table rather than listing the types again.  Any other name,
/// including an alias or a composite (the empty name), is not numeric: fail closed.
pub fn is_numeric_type(name: &str) -> bool {
    name == "Nat" || FixedTy::from_name(name).is_some()
}

/// What CC11 needs, all of it already parsed.
pub struct Facts<'a> {
    /// The shape of `model build` of the generated document.
    pub build: &'a BuildShape,
    /// The operations that produce a new state from a state (see [`successor_ops`]).
    pub successors: &'a BTreeSet<String>,
    /// The type name of the last field of `Impl` (`""` for a composite), `None` without one.
    pub last_field: Option<&'a str>,
    /// The bound operations that are no `op` of the document (see [`DocFacts`]).
    pub unclassified: &'a [String],
    /// The claims of the document that the contract's `any` tier owns, by name and digest.
    pub owned: &'a BTreeSet<String>,
    /// The faults each bound operation lists (`evidence.bindings`).
    pub faults: &'a BTreeMap<String, Vec<String>>,
    /// The sabotage plan.
    pub plan: &'a [PlanEntry],
    /// For each claim, the bound operations it calls.
    pub calls: &'a [(String, Vec<String>)],
}

/// The operations that produce a state from a state, from the document's own signatures: the
/// return type mentions `Impl` and a parameter does too (`insert_img`, `remove_img` and
/// `dispatch_img` for the scheduler), or an `inout` parameter mutates an `Impl`.  `init` (no
/// state argument), the projections (`_out`, `alpha_w`) and `cfg_n` are not.
pub fn successor_ops(ops: &[OpFact]) -> BTreeSet<String> {
    ops.iter()
        .filter(|o| (o.returns_state && o.state_params > 0) || o.inout_state)
        .map(|o| o.name.clone())
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
            // The oracle must also be a number: `field-flip-last` flips the lowest bit of the
            // last NUMERIC field, so a `Bool` or `Bytes` oracle would leave it unreachable (the
            // generator would refuse the fault, or reach a different field).
            if !f.last_field.is_some_and(is_numeric_type) {
                out.push(format!(
                    "oracle: the last field of `Impl` has type {}, which `field-flip-last` cannot flip; the oracle must be a fixed-width integer or `Nat`",
                    f.last_field
                        .filter(|t| !t.is_empty())
                        .map_or("(not a plain named type)".to_string(), esc)
                ));
            }
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

    // A bound operation that is no `op` of the document cannot be classified.
    for op in f.unclassified {
        out.push(format!(
            "oracle: the bindings bind {}, which is no `op` of the generated document, so whether it produces a state from a state cannot be decided (fails closed)",
            esc(op)
        ));
    }

    // A crate with no successor operation has nothing that could keep or lose an oracle; an
    // `-any` whose bindings hide its successors would otherwise pass (b) and (c) by having none.
    if f.successors.is_empty() {
        out.push(format!(
            "oracle: no `op` of the document returns `{STATE_TYPE}` from an `{STATE_TYPE}` argument, so no operation is known to keep the oracle (the document must declare the state-to-state operations, `insert_img` and its kind)"
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
        // (c) A plan entry on a contract-owned claim that calls this successor and no other
        // one: a claim calling two would be failed by a flip in either, and so prove neither;
        // a claim of the implementation's own is not the contract's preservation claim.
        let attacked = f.plan.iter().any(|e| {
            e.tier == "exhaustive"
                && e.fault == ORACLE_FAULT
                && f.owned.contains(&e.name)
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
                "oracle: no `exhaustive/<claim> {ORACLE_FAULT}` entry of the sabotage plan names a claim of the contract's `any` tier that calls {} and no other state-producing operation, so a regression that overwrites the oracle is not shown to fail any claim",
                esc(op)
            ));
        }
    }
    out
}

/// What is read from the generated document and the bindings.
#[derive(Debug)]
pub struct DocFacts {
    /// The shape of `model build`.
    pub build: BuildShape,
    /// The successor operations (see [`successor_ops`]).
    pub successors: BTreeSet<String>,
    /// The type name of the last field of `Impl`.
    pub last_field: Option<String>,
    /// The bound operations that are no `op` of the document.
    pub unclassified: Vec<String>,
    /// Every claim of the document with its digest.
    pub claims: Vec<ClaimFact>,
}

/// The inputs read from a crate: the parsed document's facts and the bindings' operation names.
/// `Err(message)` is a refusal by the tools on admitted bytes (a CC11 finding).
pub fn load(bindings: &str, document: String) -> Result<Result<DocFacts, String>, NoVerdict> {
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
        Ok(a) => {
            let unclassified = hosts
                .ops
                .keys()
                .filter(|b| !a.ops.iter().any(|o| o.name == **b))
                .cloned()
                .collect();
            Ok(DocFacts {
                build: a.build,
                successors: successor_ops(&a.ops),
                last_field: a.impl_last_field_type,
                unclassified,
                claims: a.claims,
            })
        }
        Err(e) => Err(format!("the document: {}", esc(&e.to_string()))),
    })
}
