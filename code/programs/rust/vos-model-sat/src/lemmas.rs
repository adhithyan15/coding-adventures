//! # The ten lemmas as circuits
//!
//! Each lemma of `warrant/lemmas.part` is a `forall` claim over a few `Bool` and `U8` binders.
//! [`lemma`] builds the claim's body as ONE edge, `holds`, over fresh inputs, using whichever
//! [`SchedModel`] it is given: the honest [`crate::Encoder`], a [`crate::Mutant`] (the control),
//! or a deliberately wrong encoder (a negative control).
//!
//! ## The inputs, and the case numbering every cross-check shares
//!
//! ```text
//!   binders of the Warrant claim            inputs of the circuit (creation order)
//!   -------------------------------------   --------------------------------------------
//!   b0 .. b7: Bool  (the Ready set)         N inputs, thread 0 first         (if the lemma has them)
//!   t or k: U8      (a thread id / a pick)  8 inputs, least significant first (if the lemma has it)
//! ```
//!
//! A **case** is a number `c`; input `j` of the circuit is bit `j` of `c`.  At N = 8 this is the
//! evaluator's own binder order (`b0..b7`, then `t`), bit for bit, so case `c` means the same
//! thing to the evaluator and to the circuit.  The lemma's whole domain is `0 .. 2^inputs`:
//! 65,536 cases for eight lemmas, 256 for `policy_implies_safety` (no id binder) and
//! `config_n_is_the_id_range` (no Ready binders).
//!
//! ## Implications and the premise witness
//!
//! Six lemmas are, at the top level, implications `P => Q`.  [`LemmaCircuit::premise`] holds `P`
//! for them, so VOS-040b can ask the solver for a model of `P` alone (VOS30 4.6: an implication
//! whose premise is unsatisfiable is UNSAT for no reason).  `holds` is always the WHOLE claim,
//! antecedent included.  Which lemmas are implications is stated by [`LemmaId::has_premise`];
//! VOS-040b re-derives it from the parser's AST of `lemmas.part`, never from this crate.
//!
//! ## Where this deliberately generalises `contract.part`
//!
//! * The lemmas that name thread 7 or the number 8 are written once, for any N.
//! * `config_n_is_the_id_range` compares `pre_insert(empty(), t)` with `t < cfg_N()`; here
//!   `cfg_N()` is the encoder's `n`.  On the honest encoder both sides are the same circuit and
//!   the lemma folds to the constant true (a `constant-fold` collapse, VOS31 7.5); only the
//!   mutant makes it non-trivial.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv;
use bit_blaster::bv::BitVec;

use crate::model::{id_const, SchedModel, ID_WIDTH};
use crate::types::Ready;

/// The ten lemmas, in the order of `warrant/lemmas.part`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LemmaId {
    /// `closure_at_m`
    ClosureAtM,
    /// `insert_outside_pre_changes_nothing`
    InsertOutsidePreChangesNothing,
    /// `remove_outside_pre_changes_nothing`
    RemoveOutsidePreChangesNothing,
    /// `insert_then_remove_restores`
    InsertThenRemoveRestores,
    /// `top_is_the_maximum`
    TopIsTheMaximum,
    /// `safety_is_idle_or_a_member`
    SafetyIsIdleOrAMember,
    /// `safe_dispatch_only_removes_the_choice`
    SafeDispatchOnlyRemovesTheChoice,
    /// `policy_implies_safety`
    PolicyImpliesSafety,
    /// `policy_admits_only_top`
    PolicyAdmitsOnlyTop,
    /// `config_n_is_the_id_range`
    ConfigNIsTheIdRange,
}

/// A name that is not one of the ten lemmas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownLemma(pub String);

impl LemmaId {
    /// All ten, in `lemmas.part` order.
    pub const ALL: [LemmaId; 10] = [
        LemmaId::ClosureAtM,
        LemmaId::InsertOutsidePreChangesNothing,
        LemmaId::RemoveOutsidePreChangesNothing,
        LemmaId::InsertThenRemoveRestores,
        LemmaId::TopIsTheMaximum,
        LemmaId::SafetyIsIdleOrAMember,
        LemmaId::SafeDispatchOnlyRemovesTheChoice,
        LemmaId::PolicyImpliesSafety,
        LemmaId::PolicyAdmitsOnlyTop,
        LemmaId::ConfigNIsTheIdRange,
    ];

    /// The claim's name in `lemmas.part`.
    pub fn name(self) -> &'static str {
        match self {
            LemmaId::ClosureAtM => "closure_at_m",
            LemmaId::InsertOutsidePreChangesNothing => "insert_outside_pre_changes_nothing",
            LemmaId::RemoveOutsidePreChangesNothing => "remove_outside_pre_changes_nothing",
            LemmaId::InsertThenRemoveRestores => "insert_then_remove_restores",
            LemmaId::TopIsTheMaximum => "top_is_the_maximum",
            LemmaId::SafetyIsIdleOrAMember => "safety_is_idle_or_a_member",
            LemmaId::SafeDispatchOnlyRemovesTheChoice => "safe_dispatch_only_removes_the_choice",
            LemmaId::PolicyImpliesSafety => "policy_implies_safety",
            LemmaId::PolicyAdmitsOnlyTop => "policy_admits_only_top",
            LemmaId::ConfigNIsTheIdRange => "config_n_is_the_id_range",
        }
    }

    /// The lemma named `name`.
    pub fn from_name(name: &str) -> Result<LemmaId, UnknownLemma> {
        LemmaId::ALL
            .into_iter()
            .find(|l| l.name() == name)
            .ok_or_else(|| UnknownLemma(name.to_string()))
    }

    /// The model function the lemma's mutant replaces: the `mutant` column of `lemmas.table`.
    pub fn mutant_model(self) -> &'static str {
        match self {
            LemmaId::ClosureAtM => "inv",
            LemmaId::InsertOutsidePreChangesNothing | LemmaId::InsertThenRemoveRestores => {
                "abs_insert"
            }
            LemmaId::RemoveOutsidePreChangesNothing => "abs_remove",
            LemmaId::TopIsTheMaximum => "top",
            LemmaId::SafetyIsIdleOrAMember | LemmaId::PolicyImpliesSafety => "valid_pick",
            LemmaId::SafeDispatchOnlyRemovesTheChoice => "after",
            LemmaId::PolicyAdmitsOnlyTop => "policy_dispatch",
            LemmaId::ConfigNIsTheIdRange => "pre_insert",
        }
    }

    /// Whether the claim binds the Ready bits `b0 .. b7`.
    pub fn has_ready_binders(self) -> bool {
        self != LemmaId::ConfigNIsTheIdRange
    }

    /// Whether the claim binds a `U8` (`t` or `k`).
    pub fn has_id_binder(self) -> bool {
        self != LemmaId::PolicyImpliesSafety
    }

    /// Whether the claim body is, at the top level, an implication `P => Q`.
    pub fn has_premise(self) -> bool {
        matches!(
            self,
            LemmaId::InsertOutsidePreChangesNothing
                | LemmaId::RemoveOutsidePreChangesNothing
                | LemmaId::InsertThenRemoveRestores
                | LemmaId::SafeDispatchOnlyRemovesTheChoice
                | LemmaId::PolicyImpliesSafety
                | LemmaId::PolicyAdmitsOnlyTop
        )
    }

    /// The number of circuit inputs at width `n`.
    pub fn input_count(self, n: usize) -> usize {
        usize::from(self.has_ready_binders()) * n + usize::from(self.has_id_binder()) * ID_WIDTH
    }
}

/// A lemma, as a circuit.
#[derive(Clone, Debug)]
pub struct LemmaCircuit {
    /// The inputs in creation order (the case numbering of the module docs).
    pub inputs: Vec<Edge>,
    /// `P` of an implication `P => Q`; `None` for a lemma that is not one.
    pub premise: Option<Edge>,
    /// The whole claim: true on a case iff the lemma holds there.
    pub holds: Edge,
}

/// Build `lemma` on `aig` with `model`.  Every input is fresh; the AIG may be shared by several
/// lemmas (hash-consing then shares their common sub-circuits).
pub fn lemma_circuit(id: LemmaId, model: &dyn SchedModel, aig: &mut Aig) -> LemmaCircuit {
    let n = model.n();
    let mut inputs: Vec<Edge> = Vec::new();
    let m = if id.has_ready_binders() {
        let r = Ready::input(aig, n);
        inputs.extend(r.0.bits.iter().copied());
        r
    } else {
        model.empty()
    };
    let t = if id.has_id_binder() {
        let v = BitVec::input(aig, ID_WIDTH);
        inputs.extend(v.bits.iter().copied());
        v
    } else {
        id_const(0)
    };
    let (premise, holds) = body(id, model, aig, &m, &t);
    LemmaCircuit {
        inputs,
        premise,
        holds,
    }
}

/// [`lemma_circuit`] by name.
pub fn lemma(
    name: &str,
    model: &dyn SchedModel,
    aig: &mut Aig,
) -> Result<LemmaCircuit, UnknownLemma> {
    Ok(lemma_circuit(LemmaId::from_name(name)?, model, aig))
}

/// `P => Q` as `(Some(P), implies(P, Q))`.
fn implication(aig: &mut Aig, p: Edge, q: Edge) -> (Option<Edge>, Edge) {
    (Some(p), aig.implies(p, q))
}

/// The claim body of `id`, transcribed from `warrant/lemmas.part`.  `m` is `ready_of(b0..b7)`,
/// `t` the `U8` binder (`t` or `k`).
fn body(
    id: LemmaId,
    model: &dyn SchedModel,
    aig: &mut Aig,
    m: &Ready,
    t: &BitVec,
) -> (Option<Edge>, Edge) {
    match id {
        // inv(init_m()) and (inv(m) => (inv(insert.1) and inv(remove.1) and inv(after(m, top(m)))))
        LemmaId::ClosureAtM => {
            let init = model.init_m();
            let inv_init = model.inv(aig, &init);
            let inv_m = model.inv(aig, m);
            let ins = model.abs_insert(aig, m, t);
            let rem = model.abs_remove(aig, m, t);
            let best = model.top(aig, m);
            let next = model.after(aig, m, &best);
            let a = model.inv(aig, &ins.ready);
            let b = model.inv(aig, &rem.ready);
            let c = model.inv(aig, &next);
            let conclusion = aig.and_all(&[a, b, c]);
            let step = aig.implies(inv_m, conclusion);
            (None, aig.and(inv_init, step))
        }
        // not pre_insert(m, t) => (abs_insert(m, t).0 == OutOfRange and abs_insert(m, t).1 == m)
        LemmaId::InsertOutsidePreChangesNothing => {
            let pre = model.pre_insert(aig, m, t);
            let ins = model.abs_insert(aig, m, t);
            let same = model.ready_eq(aig, &ins.ready, m);
            let q = aig.and(ins.upd.out_of_range, same);
            implication(aig, !pre, q)
        }
        LemmaId::RemoveOutsidePreChangesNothing => {
            let pre = model.pre_remove(aig, m, t);
            let rem = model.abs_remove(aig, m, t);
            let same = model.ready_eq(aig, &rem.ready, m);
            let q = aig.and(rem.upd.out_of_range, same);
            implication(aig, !pre, q)
        }
        // (t < 8 and not mem(m, t)) => (abs_insert(m, t).0 == Changed
        //                               and abs_remove(abs_insert(m, t).1, t).1 == m)
        LemmaId::InsertThenRemoveRestores => {
            let in_range = bv::ult(aig, t, &id_const(model.n()));
            let member = model.mem(aig, m, t);
            let p = aig.and(in_range, !member);
            let ins = model.abs_insert(aig, m, t);
            let back = model.abs_remove(aig, &ins.ready, t);
            let restored = model.ready_eq(aig, &back.ready, m);
            let q = aig.and(ins.upd.changed, restored);
            implication(aig, p, q)
        }
        // (mem(m, t) => (match top(m) with Idle -> false | Run(h) -> t <= h))
        // and (match top(m) with Idle -> not mem(m, t) | Run(h) -> mem(m, h))
        LemmaId::TopIsTheMaximum => {
            let best = model.top(aig, m);
            let member_t = model.mem(aig, m, t);
            let t_le_h = bv::ule(aig, t, &best.id);
            let first_match = aig.mux(best.idle, Edge::constant(false), t_le_h);
            let first = aig.implies(member_t, first_match);
            let member_h = model.mem(aig, m, &best.id);
            let second = aig.mux(best.idle, !member_t, member_h);
            (None, aig.and(first, second))
        }
        // safe_dispatch(m, pick_of(k), after(m, pick_of(k))) == (k == 255 or (k < 8 and mem(m, k)))
        LemmaId::SafetyIsIdleOrAMember => {
            let o = model.pick_of(aig, t);
            let next = model.after(aig, m, &o);
            let lhs = model.safe_dispatch(aig, m, &o, &next);
            let is_idle = bv::eq(aig, t, &BitVec::constant(ID_WIDTH, 255));
            let in_range = bv::ult(aig, t, &id_const(model.n()));
            let member = model.mem(aig, m, t);
            let run_ok = aig.and(in_range, member);
            let rhs = aig.or(is_idle, run_ok);
            (None, aig.xnor(lhs, rhs))
        }
        // valid_pick(m, pick_of(k)) => (subset(after(..), m) and (match pick_of(k) with
        //    Idle -> after(..) == m
        //  | Run(h) -> after(..) == put(m, h, false) and not mem(after(..), h)))
        LemmaId::SafeDispatchOnlyRemovesTheChoice => {
            let o = model.pick_of(aig, t);
            let p = model.valid_pick(aig, m, &o);
            let next = model.after(aig, m, &o);
            let sub = model.subset(aig, &next, m);
            let idle_case = model.ready_eq(aig, &next, m);
            let removed = model.put(aig, m, &o.id, Edge::constant(false));
            let is_removed = model.ready_eq(aig, &next, &removed);
            let still = model.mem(aig, &next, &o.id);
            let run_case = aig.and(is_removed, !still);
            let matched = aig.mux(o.idle, idle_case, run_case);
            let q = aig.and(sub, matched);
            implication(aig, p, q)
        }
        // policy_dispatch(m, top(m), after(m, top(m))) => safe_dispatch(m, top(m), after(m, top(m)))
        LemmaId::PolicyImpliesSafety => {
            let best = model.top(aig, m);
            let next = model.after(aig, m, &best);
            let p = model.policy_dispatch(aig, m, &best, &next);
            let q = model.safe_dispatch(aig, m, &best, &next);
            implication(aig, p, q)
        }
        // policy_dispatch(m, pick_of(k), after(m, pick_of(k))) => pick_of(k) == top(m)
        LemmaId::PolicyAdmitsOnlyTop => {
            let o = model.pick_of(aig, t);
            let next = model.after(aig, m, &o);
            let p = model.policy_dispatch(aig, m, &o, &next);
            let best = model.top(aig, m);
            let q = model.pick_eq(aig, &o, &best);
            implication(aig, p, q)
        }
        // pre_insert(empty(), t) == (t < cfg_N())
        LemmaId::ConfigNIsTheIdRange => {
            let lhs = model.pre_insert(aig, m, t);
            let rhs = bv::ult(aig, t, &id_const(model.n()));
            (None, aig.xnor(lhs, rhs))
        }
    }
}
