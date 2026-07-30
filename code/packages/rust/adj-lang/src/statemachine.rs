//! The `statemachine` **driver** (ADJ-STATEMACHINE RS-3c) — the deterministic,
//! total loop that *runs* the provenance-stamped machines RS-3b already parsed and
//! lowered ([`crate::lower::LoweredStateMachine`]).
//!
//! # What this is, and what it deliberately is not
//!
//! A `statemachine` is ADJ's construct for **long-horizon procedural reasoning**:
//! triage → work-up → decision, titrate-until-target, iterate-until-converged. The
//! spec's core claim (ADJ-STATEMACHINE §1–§2) is that a machine introduces **no new
//! evaluator** — a guard is an ordinary predicate/compute evaluation, an action is
//! an assertion into the `KnowledgeBase`, and a step is one forward-chaining
//! transition. This module honours that literally:
//!
//! - a **comparison guard** (`inr < 2`) is read exactly as a predicate-gated
//!   contribution reads its slot: [`KnowledgeBase::observed_numeric`] for the
//!   subject's valued slot, [`compute`] for the right-hand side, and
//!   [`EngineCmpOp::eval_values`] for the exact-first comparison — the SAME three
//!   calls `logic_engine::lr_aggregate` makes when a `contributes … from slot <op>
//!   thr` clause fires. No parallel comparison logic exists here.
//! - a **presence guard** (a bare finding atom like `done`) holds iff that fact is
//!   present/derivable, decided by the SAME SLD resolver ([`enumerate_all`]) the
//!   recall section uses — "has any proof?". The bare atom `true` is the
//!   always-holds special case (an unconditional transition).
//! - an **action** `assert <term>` adds a [`Fact::certain`] to a **working clone**
//!   of the KB, so the machine's asserts never leak into the rest of the program.
//!
//! What is genuinely new is only the *sequencing* and the *termination guarantee*.
//!
//! # Termination — total by construction (ADJ-STATEMACHINE §3–§4)
//!
//! The loop returns exactly one of four typed outcomes ([`StateMachineOutcome`]):
//! `Halted` / `StepBudgetExceeded` / `NonTerminating` / `Stuck`. It **cannot hang**:
//! the `steps >= budget` guard caps the loop at `budget + 1` iterations even if
//! cycle detection never fires, and `budget` is the modeller's declared literal
//! bound (a `u64`, but a fixed input value). A malicious or buggy machine therefore
//! degrades to a typed, grounded abstention ("stopped after N steps" / "livelock at
//! state …" / "no transition applies in state …"), never an unbounded spin or OOM.
//! `seen` never grows past `budget` distinct keys for the same reason.
//!
//! # The cycle key (ADJ-STATEMACHINE §3.1)
//!
//! `project(kb)` is keyed on `(state, the set of terms the machine has asserted so
//! far)`. Because actions only ever *add* facts to the working clone (monotone),
//! and the base KB is fixed for the whole run, the asserted-term set is a sound,
//! deterministic fingerprint of the machine-relevant configuration: revisiting an
//! identical `(state, asserted-set)` is a genuine fixed-point livelock (e.g. a
//! titrate step that re-asserts an unchanged value), short-circuited to
//! `NonTerminating`. Many runs never repeat a key and instead halt via an exit or
//! run out the budget — the cycle guard catches only the true loop.
//!
//! Every taken transition is recorded as a provenanced [`RunStep`] so `--explain`
//! (ADJ-REASON-MATH §E.8) can render the run as an ordered narrative and
//! `adj-verify` can re-execute it.

use std::collections::{BTreeSet, HashSet};

use logic_core::Term as CoreTerm;
use logic_engine::compute::Derived;
use logic_engine::{compute, enumerate_all, CmpOp as EngineCmpOp, Fact, KnowledgeBase, Provenance};

use crate::lower::{LoweredGuard, LoweredStateMachine};

/// The outcome of a state-machine run — one of the four typed terminals of
/// ADJ-STATEMACHINE §4. Total by construction: [`run_state_machine`] returns
/// exactly one of these, always, and never hangs.
#[derive(Debug, Clone)]
pub enum StateMachineOutcome {
    /// An exit criterion held; `result` is the yielded value (numeric with its
    /// derivation tree, or a bare symbol for a symbolic yield like `at_target`).
    Halted { state: String, result: YieldValue },
    /// The budget ran out before any exit held — a grounded abstention
    /// ("stopped after N steps"), with the partial trace.
    StepBudgetExceeded {
        steps: u64,
        budget: u64,
        state: String,
    },
    /// A `(state, asserted-set)` configuration repeated — a livelock, caught,
    /// with the partial trace.
    NonTerminating { state: String },
    /// In `state`, no transition guard holds and no exit criterion holds — a dead
    /// end; a grounded abstention ("no transition applies in state …").
    Stuck { state: String },
}

impl StateMachineOutcome {
    /// The stable JSON/`--explain` discriminant for this outcome. Fixed
    /// identifiers a checker keys off — not `Debug`.
    pub fn type_tag(&self) -> &'static str {
        match self {
            StateMachineOutcome::Halted { .. } => "halted",
            StateMachineOutcome::StepBudgetExceeded { .. } => "step_budget_exceeded",
            StateMachineOutcome::NonTerminating { .. } => "non_terminating",
            StateMachineOutcome::Stuck { .. } => "stuck",
        }
    }
}

/// The value an `exit … yield <expr>` produced. A yield expression is evaluated
/// with the ordinary [`compute`] evaluator against the working KB (ADJ-STATEMACHINE
/// §3, "no new evaluator"):
///
/// - a **numeric** yield (`yield bmi(mass, height)`, or any slot with a valued
///   fact) carries its [`Derived`] — value, exact sidecar, and full derivation
///   tree — so the halt result is byte-traceable exactly like a `let` binding;
/// - a **symbolic** yield (`yield at_target`) is a bare finding atom with no
///   numeric binding, so the result *is* that symbol — the engine's
///   `UnknownSlot` on such an expression is the signal, not an error.
#[derive(Debug, Clone)]
pub enum YieldValue {
    /// A computed numeric result, with its derivation tree (boxed — a `Derived`
    /// is large, and the common case is the symbolic yield).
    Numeric(Box<Derived>),
    /// A bare symbolic result — the name of the yielded finding atom.
    Symbol(String),
}

impl YieldValue {
    /// A stable, human-readable rendering of the yielded value: the exact-first
    /// decimal for a numeric yield (all digits when finite, else the labeled `f64`),
    /// or the bare symbol name. Deterministic (ADJ-REASON-MATH §E.8 P4).
    pub fn render(&self) -> String {
        match self {
            YieldValue::Numeric(d) => d
                .exact
                .as_ref()
                .and_then(|e| e.to_exact_decimal_string())
                .unwrap_or_else(|| format!("{}", d.value)),
            YieldValue::Symbol(s) => s.clone(),
        }
    }
}

/// One taken transition in a run — the provenanced entry the trace records
/// (ADJ-STATEMACHINE §3, "every loop action is one provenanced entry in the
/// `ReasoningTrace`"). Carries the source state, the guard that fired, the target,
/// the terms this transition asserted (in order), and the machine's cited
/// provenance (each transition inherits the machine's envelope, RS-3b).
#[derive(Debug, Clone)]
pub struct RunStep {
    /// The state the machine was in when this transition fired.
    pub from_state: String,
    /// The rendered guard that held (`inr < 2`, `true`, `done`).
    pub guard: String,
    /// The state the transition moved to.
    pub target: String,
    /// The terms asserted by this transition's actions, in source order (their
    /// `Display` form). Empty for a transition with no `do assert …`.
    pub asserted: Vec<String>,
    /// The cited provenance of the machine (inherited by every transition, RS-3b).
    pub provenance: Provenance,
}

/// The full record of one run: the terminal [`StateMachineOutcome`] and the
/// ordered [`RunStep`]s that led to it (empty when the machine halts, is stuck, or
/// loops before taking any transition).
#[derive(Debug, Clone)]
pub struct StateMachineRun {
    pub outcome: StateMachineOutcome,
    pub steps: Vec<RunStep>,
}

/// Run one lowered `statemachine` against `base_kb`, returning its typed outcome
/// and provenanced step trace. Deterministic and **total** — see the module docs
/// for the termination argument. `base_kb` is never mutated: the machine reasons
/// over a working clone so its `assert`s cannot leak into the rest of the program.
pub fn run_state_machine(sm: &LoweredStateMachine, base_kb: &KnowledgeBase) -> StateMachineRun {
    // The working clone — the machine's asserts land here, never in the caller's KB.
    let mut kb = base_kb.clone();
    let mut state = sm.initial.clone();
    let mut steps: u64 = 0;
    // Visited `(state, sorted asserted-term-set)` keys — the cycle guard (§3.1).
    let mut seen: HashSet<(String, Vec<String>)> = HashSet::new();
    // The set of terms asserted so far, sorted+deduped for a deterministic key.
    let mut asserted: BTreeSet<String> = BTreeSet::new();
    let mut trace: Vec<RunStep> = Vec::new();

    loop {
        // (1) Exit first: the FIRST exit (source order) whose guard holds halts the
        //     run, yielding its evaluated expression.
        if let Some(exit) = sm.exits.iter().find(|e| guard_holds(&e.guard, &kb)) {
            let result = eval_yield(&exit.yield_expr, &kb);
            return StateMachineRun {
                outcome: StateMachineOutcome::Halted {
                    state: state.clone(),
                    result,
                },
                steps: trace,
            };
        }

        // (2) Budget: the hard termination guarantee. At most `budget + 1`
        //     iterations even if cycle detection never fires.
        if steps >= sm.budget {
            return StateMachineRun {
                outcome: StateMachineOutcome::StepBudgetExceeded {
                    steps,
                    budget: sm.budget,
                    state: state.clone(),
                },
                steps: trace,
            };
        }

        // (3) Cycle key: (state, machine-relevant configuration). Because asserts
        //     are monotone and the base KB is fixed, the asserted-set is a sound
        //     deterministic fingerprint; a repeat is a livelock.
        let key = (state.clone(), asserted.iter().cloned().collect::<Vec<_>>());
        if seen.contains(&key) {
            return StateMachineRun {
                outcome: StateMachineOutcome::NonTerminating {
                    state: state.clone(),
                },
                steps: trace,
            };
        }
        seen.insert(key);

        // (4) Select the FIRST transition of the current state, in source order,
        //     whose guard holds (first-guard-wins, §3). No such transition — and no
        //     exit held above — is a dead end.
        //
        //     A missing state is impossible post-lowering (the initial name and
        //     every target are validated against the declared states); we treat it
        //     defensively as `Stuck` rather than panic.
        let Some(cur) = sm.states.iter().find(|s| s.name == state) else {
            return StateMachineRun {
                outcome: StateMachineOutcome::Stuck {
                    state: state.clone(),
                },
                steps: trace,
            };
        };
        let Some(tr) = cur.transitions.iter().find(|t| guard_holds(&t.guard, &kb)) else {
            return StateMachineRun {
                outcome: StateMachineOutcome::Stuck {
                    state: state.clone(),
                },
                steps: trace,
            };
        };

        // (5) Fire: apply each `assert` action to the working KB, record the step,
        //     move to the target, and consume one budget unit.
        let mut asserted_now: Vec<String> = Vec::new();
        for a in &tr.actions {
            let crate::lower::LoweredAction::Assert(term) = a;
            let rendered = format!("{term}");
            asserted.insert(rendered.clone());
            asserted_now.push(rendered);
            kb.add_fact(Fact::certain(term.clone()));
        }
        trace.push(RunStep {
            from_state: state.clone(),
            guard: render_guard(&tr.guard),
            target: tr.target.clone(),
            asserted: asserted_now,
            provenance: sm.provenance.clone(),
        });
        state = tr.target.clone();
        steps += 1;
    }
}

/// Does this guard hold in `kb`? The whole of the "no new evaluator" claim
/// (ADJ-STATEMACHINE §3) lives in this function: a comparison guard is the
/// predicate-gated-contribution comparison, a presence guard is an SLD "any proof?"
/// check, and the bare atom `true` is the always-holds special case.
fn guard_holds(g: &LoweredGuard, kb: &KnowledgeBase) -> bool {
    match &g.comparison {
        // Comparison guard `subject <op> rhs`: read the subject's valued slot and
        // the (computed) rhs, then compare exact-first — mirroring exactly how
        // `logic_engine::lr_aggregate` evaluates a `contributes … from slot <op>
        // thr` predicate clause.
        Some((op, rhs)) => {
            let slot = subject_slot(&g.subject);
            let Some((observed, observed_exact)) = kb.observed_numeric(&slot) else {
                // No value for the slot → the comparison cannot hold (it is not an
                // error; the fact simply is not there yet).
                return false;
            };
            let Ok(r) = compute("__sm_guard_rhs", rhs, kb) else {
                // A malformed rhs is a non-firing guard, never a panic.
                return false;
            };
            eval_cmp(*op, observed, r.value, observed_exact, r.exact)
        }
        // Presence guard: the bare atom `true` always holds (an unconditional
        // transition); any other atom holds iff it is present/derivable in the KB.
        None => {
            if is_true_atom(&g.subject) {
                return true;
            }
            let dag = enumerate_all(&g.subject, kb);
            !dag.proofs.is_empty()
        }
    }
}

/// Evaluate `lhs <op> rhs` exactly the way the engine's predicate clauses do:
/// exact-rational comparison when both sidecars are present, `f64` otherwise.
fn eval_cmp(
    op: EngineCmpOp,
    lhs: f64,
    rhs: f64,
    lhs_exact: Option<logic_engine::compute::ExactRational>,
    rhs_exact: Option<logic_engine::compute::ExactRational>,
) -> bool {
    op.eval_values(lhs, rhs, lhs_exact, rhs_exact)
}

/// The valued-slot name a comparison guard's subject resolves against — the atom
/// name for a bare finding (`inr`), or the functor for a compound subject.
fn subject_slot(subject: &CoreTerm) -> String {
    match subject {
        CoreTerm::Atom(s) => s.clone(),
        CoreTerm::Compound { functor, .. } => functor.clone(),
        other => format!("{other}"),
    }
}

/// Whether a presence-guard subject is the always-holds sentinel atom `true`.
fn is_true_atom(subject: &CoreTerm) -> bool {
    matches!(subject, CoreTerm::Atom(s) if s == "true")
}

/// Evaluate an exit's yield expression against the (final) working KB. A numeric
/// result carries its derivation tree; a bare symbolic atom (whose slot has no
/// numeric binding) is reported as a symbol — the engine's `UnknownSlot` on such
/// an expression is the signal that the yield is symbolic, not a failure.
fn eval_yield(expr: &logic_engine::ComputeExpr, kb: &KnowledgeBase) -> YieldValue {
    match compute("__sm_yield", expr, kb) {
        Ok(d) => YieldValue::Numeric(Box::new(d)),
        Err(_) => YieldValue::Symbol(expr_symbol(expr)),
    }
}

/// The symbolic rendering of a yield expression that did not evaluate to a number
/// — the slot name for a bare `Ref` (the common `yield at_target` case), the
/// literal for a `Lit`, and a generic placeholder otherwise (an expression that
/// genuinely failed to compute for another reason still yields a clean, non-panic
/// symbol).
fn expr_symbol(expr: &logic_engine::ComputeExpr) -> String {
    use logic_engine::ComputeExpr as E;
    match expr {
        E::Ref(slot) => slot.clone(),
        E::Lit(x) => format!("{x}"),
        _ => "<expr>".to_string(),
    }
}

/// Render a guard for the trace/`--explain` narrative: `subject <op> rhs` for a
/// comparison guard, or the bare subject for a presence guard.
fn render_guard(g: &LoweredGuard) -> String {
    match &g.comparison {
        Some((op, rhs)) => format!("{} {} {}", g.subject, op.symbol(), render_expr(rhs)),
        None => format!("{}", g.subject),
    }
}

/// A compact, deterministic rendering of a guard's rhs expression — the literal or
/// slot in the common cases, a generic placeholder for a compound rhs (whose full
/// structure the JSON derivation channel carries elsewhere).
fn render_expr(expr: &logic_engine::ComputeExpr) -> String {
    use logic_engine::ComputeExpr as E;
    match expr {
        E::Ref(slot) => slot.clone(),
        E::Lit(x) => format!("{x}"),
        E::Agg(_, slot) => slot.clone(),
        _ => "<expr>".to_string(),
    }
}
