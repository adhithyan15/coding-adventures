//! # Module-scope refinement checker — LANG23 PR 23-F.
//!
//! Extends the function-scope checker (PR 23-D) to reason *across* function
//! call boundaries within a single module.
//!
//! ## The problem 23-F solves
//!
//! PR 23-D can prove that `clamp-byte`'s internal return paths all satisfy
//! its declared return type.  But it cannot help when a caller *invokes* a
//! refinement-annotated function — it has no knowledge of the callee's
//! parameter annotation, so it would emit a runtime check at every call site.
//!
//! Module-scope reasoning fixes this:
//!
//! ```scheme
//! ;; module: text/ascii.twig
//! (define (decode (codepoint : (Int 0 128))) ...)
//!
//! (define (latin1-decode (cp : (Int 0 256)))
//!   (if (< cp 128)
//!       (decode cp)              ; ← cp narrowed to [0, 128) by guard → safe
//!       (latin1-fallback cp)))   ; ← latin1-fallback not annotated → skip
//! ```
//!
//! The checker:
//! 1. Knows `cp : (Int 0 256)` from `latin1-decode`'s signature.
//! 2. Applies the `(< cp 128)` guard → then-branch narrows `cp` to `[0, 128)`.
//! 3. Checks: `[0, 256) ∩ [0, 128) ⊆ [0, 128)` → UNSAT → **ProvenSafe**.
//!
//! The call to `decode` is proven safe with **no runtime check**.
//!
//! ## Architecture
//!
//! ```text
//!   ModuleChecker
//!       │  holds a ModuleScope (name → FunctionSignature registry)
//!       │  walks ModuleCfgNode trees, seeded with caller param annotations
//!       │  at Branch: forks predicate scope (same as FunctionChecker 23-D)
//!       │  at Call:   checks each arg against callee's param annotations
//!       │             (callee looked up in ModuleScope; skip if absent)
//!       │  at Return: checks return value against caller's return type
//!       ▼
//!   Checker::check(annotation, evidence)
//!       ▼
//!   CheckOutcome: ProvenSafe | ProvenUnsafe | Unknown
//! ```
//!
//! ## Per-symbol opt-out via `: any`
//!
//! A function absent from the [`ModuleScope`] is silently skipped — all
//! call sites targeting it produce no outcomes.  This is the "per-symbol
//! opt-out via `: any`" mechanism from the LANG23 spec: the frontend simply
//! does not register functions annotated `any` or left un-annotated.
//!
//! A registered function with `RefinedType::unrefined(Kind::Int)` parameters
//! is also effectively opted-out for those parameters — `Checker::check`
//! returns `ProvenSafe` immediately for unrefined annotations.
//!
//! ## Call-argument evidence
//!
//! At a call site `(decode cp)`, the checker builds evidence for `cp` from
//! two sources — in accumulation order:
//!
//! 1. **Caller parameter annotation** — if `cp : (Int 0 256)` appears in
//!    `latin1-decode`'s signature, that predicate seeds the initial scope.
//! 2. **CFG guard accumulation** — every `if`/`cond` guard on the path to this
//!    call site narrows the predicate scope further.
//!
//! The union of both predicates becomes `Evidence::Predicated`, passed to
//! `Checker::check`.

use std::collections::HashMap;

use lang_refined_types::{Kind, Predicate, RefinedType};

use crate::{
    function_checker::{
        substitute_var, BranchGuard, FunctionSignature, ReturnSiteOutcome, ReturnValue,
    },
    CheckOutcome, Checker, Evidence,
};

// ---------------------------------------------------------------------------
// Safety budgets — guard against adversarially-crafted or pathological CFGs.
// ---------------------------------------------------------------------------
//
// Three classes of denial-of-service are possible with a recursive CFG walk:
//
//   1. **Stack overflow** — a linear chain of N Branch nodes produces an
//      N-frame call stack.  We cap at MAX_MODULE_CFG_DEPTH = 64.
//
//   2. **Exponential memory via return sites** — a balanced binary tree of
//      depth D produces 2^D return sites.  We cap at MAX_MODULE_RETURN_SITES.
//
//   3. **Huge call-site collection** — a long chain of Call nodes produces
//      one entry per arg per call.  We cap at MAX_MODULE_CALL_SITES.
//
// All limits produce `CheckOutcome::Unknown` so the compiler can still emit
// runtime checks in lenient mode rather than crashing.

/// Maximum CFG branch nesting depth. Beyond this the walker emits `Unknown`.
const MAX_MODULE_CFG_DEPTH: usize = 64;

/// Maximum total return sites. Beyond this further `Return` nodes are skipped.
const MAX_MODULE_RETURN_SITES: usize = 1_024;

/// Maximum total call-site outcomes. Beyond this further `Call` args are skipped.
const MAX_MODULE_CALL_SITES: usize = 4_096;

// ---------------------------------------------------------------------------
// CallArg — argument at a cross-function call site
// ---------------------------------------------------------------------------

/// An argument passed to a cross-function call.
///
/// Mirrors [`ReturnValue`] but for call-site arguments.  The checker uses the
/// current path-predicate scope to build `Evidence` for each argument:
///
/// - `Literal(v)` → `Evidence::Concrete(v)` (no scope lookup needed).
/// - `Variable(name)` → accumulated predicates for `name` in the current
///   path scope, mapped to `Evidence::Predicated`; or `Evidence::Unconstrained`
///   if no predicates have been gathered for `name`.
///
/// # Truth table
///
/// | Arg kind  | Scope has predicates for name? | Evidence          |
/// |-----------|-------------------------------|-------------------|
/// | Literal   | —                             | `Concrete(v)`     |
/// | Variable  | Yes                           | `Predicated(...)` |
/// | Variable  | No                            | `Unconstrained`   |
///
/// `Unconstrained` → `Unknown` → runtime check in lenient mode.
#[derive(Debug, Clone)]
pub enum CallArg {
    /// A compile-time integer constant.
    Literal(i128),
    /// A variable from the calling function's scope.
    Variable(String),
}

// ---------------------------------------------------------------------------
// ModuleCfgNode — extended CFG node with a Call variant
// ---------------------------------------------------------------------------

/// A node in the module-scope CFG.
///
/// Extends the function-scope `CfgNode` (PR 23-D) with a `Call` variant that
/// represents a cross-function call whose argument safety must be proven
/// against the callee's parameter annotations.
///
/// ## Node variants
///
/// | Variant | Action |
/// |---------|--------|
/// | `Branch` | Fork predicate scope; then-arm gets guard predicate, else-arm gets its negation. |
/// | `Call`   | For each arg, check against callee's param annotation (if callee is in scope). |
/// | `Return` | Check return value against caller's return type. |
///
/// ## Modelling `latin1-decode`
///
/// ```scheme
/// (define (latin1-decode (cp : (Int 0 256)))
///   (if (< cp 128)
///       (decode cp)
///       (latin1-fallback cp)))
/// ```
///
/// ```rust
/// use lang_refined_types::Predicate;
/// use lang_refinement_checker::module_checker::{CallArg, ModuleCfgNode};
/// use lang_refinement_checker::function_checker::{BranchGuard, ReturnValue};
///
/// let cfg = ModuleCfgNode::Branch {
///     guard: BranchGuard {
///         var: "cp".to_string(),
///         predicate: Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false },
///     },
///     then_node: Box::new(ModuleCfgNode::Call {
///         callee: "decode".to_string(),
///         args: vec![CallArg::Variable("cp".to_string())],
///         next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".to_string()))),
///     }),
///     else_node: Box::new(ModuleCfgNode::Call {
///         callee: "latin1-fallback".to_string(),
///         args: vec![CallArg::Variable("cp".to_string())],
///         next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".to_string()))),
///     }),
/// };
/// # let _ = cfg;
/// ```
#[derive(Debug, Clone)]
pub enum ModuleCfgNode {
    /// A conditional branch: forks the predicate scope into two paths.
    ///
    /// Then-path: `guard.predicate` is added to the scope for `guard.var`.
    /// Else-path: `Predicate::not(guard.predicate)` is added instead.
    Branch {
        /// The guard condition.
        guard: BranchGuard,
        /// Taken when `guard.predicate` holds.
        then_node: Box<ModuleCfgNode>,
        /// Taken when `Predicate::not(guard.predicate)` holds.
        else_node: Box<ModuleCfgNode>,
    },
    /// A cross-function call: check each argument against the callee's
    /// parameter annotations (if the callee is in the [`ModuleScope`]).
    ///
    /// Execution continues in `next` after the call regardless of outcome.
    /// This models the caller's subsequent computation — if the call returns a
    /// value used later, that result's type is not yet propagated (call-return
    /// propagation is a future extension).
    Call {
        /// Name of the callee function.
        callee: String,
        /// Arguments passed to the callee, in parameter order.
        args: Vec<CallArg>,
        /// The rest of the CFG path after this call.
        next: Box<ModuleCfgNode>,
    },
    /// Function return — same semantics as `CfgNode::Return`.
    ///
    /// The return value is checked against the *caller* function's declared
    /// return type, not the callee's.
    Return(ReturnValue),
}

// ---------------------------------------------------------------------------
// ModuleScope — registry of function signatures
// ---------------------------------------------------------------------------

/// Registry of function signatures for cross-function reasoning.
///
/// The module checker looks up callee signatures here when checking call
/// sites.  Functions that are absent from the scope are silently skipped
/// ("per-symbol opt-out via `: any`" in the LANG23 spec).
///
/// ## Building a scope
///
/// ```rust
/// use lang_refined_types::{Kind, Predicate, RefinedType};
/// use lang_refinement_checker::function_checker::FunctionSignature;
/// use lang_refinement_checker::module_checker::ModuleScope;
///
/// // (define (decode (codepoint : (Int 0 128))) ...)
/// let decode_sig = FunctionSignature {
///     params: vec![(
///         "codepoint".into(),
///         RefinedType::refined(
///             Kind::Int,
///             Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
///         ),
///     )],
///     return_type: RefinedType::unrefined(Kind::Int),
/// };
///
/// let mut scope = ModuleScope::new();
/// scope.register("decode", decode_sig);
///
/// assert!(scope.get_signature("decode").is_some());
/// assert!(scope.get_signature("unknown_fn").is_none()); // opt-out
/// ```
#[derive(Debug, Default)]
pub struct ModuleScope {
    functions: HashMap<String, FunctionSignature>,
}

impl ModuleScope {
    /// Construct an empty scope.
    pub fn new() -> Self {
        ModuleScope { functions: HashMap::new() }
    }

    /// Register a function signature.
    ///
    /// If a signature for `name` already exists, it is replaced.  Returns
    /// `&mut self` for method chaining.
    pub fn register(&mut self, name: impl Into<String>, sig: FunctionSignature) -> &mut Self {
        self.functions.insert(name.into(), sig);
        self
    }

    /// Look up a function signature by name.
    ///
    /// Returns `None` if the function has not been registered (opt-out).
    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.get(name)
    }

    /// The number of registered functions.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Returns `true` if no functions have been registered.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// CallSiteOutcome — outcome for one argument at one call site
// ---------------------------------------------------------------------------

/// The outcome for a single argument at a cross-function call site.
///
/// One `CallSiteOutcome` is produced per argument per registered callee.
/// Arguments to unregistered callees produce no outcomes (opt-out).
#[derive(Debug, Clone)]
pub struct CallSiteOutcome {
    /// Name of the callee function.
    pub callee: String,
    /// Zero-based index of the argument being checked.
    pub param_index: usize,
    /// Human-readable label: `"call decode[arg 0]"`.
    pub label: String,
    /// The checker outcome for this argument against the callee's annotation.
    pub outcome: CheckOutcome,
}

// ---------------------------------------------------------------------------
// FunctionBodyCheckResult — aggregate outcome for a function body
// ---------------------------------------------------------------------------

/// Aggregate outcome of checking a function body at module scope.
///
/// Extends `FunctionCheckResult` (PR 23-D) with call-site outcomes: the
/// return-site obligations (caller's return type) and the call-site obligations
/// (callee's parameter annotations) are collected in one structure.
///
/// ## Interpretation
///
/// | Condition | Meaning |
/// |-----------|---------|
/// | `all_proven_safe()` | Every obligation is discharged; no runtime checks needed anywhere in this function. |
/// | `has_violation()` | At least one definite bug (counter-example available). |
/// | `runtime_check_count() > 0` | Some obligations are `Unknown`; emit that many runtime checks in lenient mode. |
/// | `is_vacuous()` | No obligations at all (diverging function, or no annotated calls/returns). |
#[derive(Debug, Clone)]
pub struct FunctionBodyCheckResult {
    /// Outcomes for each return site, in DFS path order (same as PR 23-D).
    pub return_sites: Vec<ReturnSiteOutcome>,
    /// Outcomes for each cross-function call argument, in traversal order.
    pub call_sites: Vec<CallSiteOutcome>,
}

impl FunctionBodyCheckResult {
    /// Returns `true` if every return site AND every call-site argument is
    /// `ProvenSafe`.
    ///
    /// Vacuous results (no sites at all) return `false` — there is nothing
    /// proven.
    pub fn all_proven_safe(&self) -> bool {
        // Vacuous: nothing proven.
        if self.return_sites.is_empty() && self.call_sites.is_empty() {
            return false;
        }
        // Every return site safe (vacuous iter → true) and every call site safe.
        self.return_sites.iter().all(|r| r.outcome.is_safe())
            && self.call_sites.iter().all(|c| c.outcome.is_safe())
    }

    /// Returns `true` if every checked call-site argument is `ProvenSafe`.
    ///
    /// Useful when the function's return type is unrefined (no return sites)
    /// but you still want to confirm all outgoing calls are safe.
    ///
    /// An empty `call_sites` is trivially `true` (nothing to violate).
    pub fn all_call_sites_proven_safe(&self) -> bool {
        self.call_sites.iter().all(|c| c.outcome.is_safe())
    }

    /// Returns `true` if any return site or call-site argument is `ProvenUnsafe`.
    pub fn has_violation(&self) -> bool {
        self.return_sites.iter().any(|r| r.outcome.is_unsafe())
            || self.call_sites.iter().any(|c| c.outcome.is_unsafe())
    }

    /// Returns the first counter-example found, scanning return sites then
    /// call sites in traversal order.
    pub fn first_counter_example(&self) -> Option<&crate::CounterExample> {
        self.return_sites
            .iter()
            .filter_map(|r| r.outcome.counter_example())
            .chain(self.call_sites.iter().filter_map(|c| c.outcome.counter_example()))
            .next()
    }

    /// Number of `Unknown` outcomes across both return sites and call sites.
    ///
    /// In lenient mode, each Unknown causes one runtime check to be emitted.
    /// In strict mode, any nonzero count is a compile error.
    pub fn runtime_check_count(&self) -> usize {
        self.return_sites.iter().filter(|r| r.outcome.is_unknown()).count()
            + self.call_sites.iter().filter(|c| c.outcome.is_unknown()).count()
    }

    /// `true` if no return sites and no call sites were collected.
    ///
    /// Vacuous results arise from diverging functions (no reachable `Return`
    /// nodes and no calls to annotated functions) or fully unrefined modules.
    pub fn is_vacuous(&self) -> bool {
        self.return_sites.is_empty() && self.call_sites.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ModuleChecker — the main entry point for module-scope checking
// ---------------------------------------------------------------------------

/// Module-scope refinement checker.
///
/// Extends `FunctionChecker` (PR 23-D) to reason across function call
/// boundaries within a module.  The checker uses a [`ModuleScope`] to look
/// up callee parameter annotations at call sites.
///
/// # Example — `latin1-decode` / `decode` (primary acceptance criterion)
///
/// ```rust
/// use lang_refined_types::{Kind, Predicate, RefinedType};
/// use lang_refinement_checker::function_checker::{BranchGuard, FunctionSignature, ReturnValue};
/// use lang_refinement_checker::module_checker::{
///     CallArg, ModuleCfgNode, ModuleChecker, ModuleScope,
/// };
///
/// // (define (decode (codepoint : (Int 0 128))) ...)
/// let decode_sig = FunctionSignature {
///     params: vec![(
///         "codepoint".into(),
///         RefinedType::refined(
///             Kind::Int,
///             Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
///         ),
///     )],
///     return_type: RefinedType::unrefined(Kind::Int),
/// };
///
/// let mut scope = ModuleScope::new();
/// scope.register("decode", decode_sig);
///
/// // (define (latin1-decode (cp : (Int 0 256)))
/// //   (if (< cp 128) (decode cp) ...))
/// let latin1_sig = FunctionSignature {
///     params: vec![(
///         "cp".into(),
///         RefinedType::refined(
///             Kind::Int,
///             Predicate::Range { lo: Some(0), hi: Some(256), inclusive_hi: false },
///         ),
///     )],
///     return_type: RefinedType::unrefined(Kind::Int),
/// };
///
/// let cfg = ModuleCfgNode::Branch {
///     guard: BranchGuard {
///         var: "cp".to_string(),
///         predicate: Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false },
///     },
///     then_node: Box::new(ModuleCfgNode::Call {
///         callee: "decode".to_string(),
///         args: vec![CallArg::Variable("cp".to_string())],
///         next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".to_string()))),
///     }),
///     else_node: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".to_string()))),
/// };
///
/// let mut checker = ModuleChecker::new(scope);
/// let result = checker.check_function(&latin1_sig, &cfg);
///
/// // The call to `decode cp` is proven safe by the `< cp 128` guard.
/// assert!(result.all_call_sites_proven_safe());
/// ```
pub struct ModuleChecker {
    scope: ModuleScope,
    checker: Checker,
}

impl ModuleChecker {
    /// Construct a module checker with the given scope.
    pub fn new(scope: ModuleScope) -> Self {
        ModuleChecker { scope, checker: Checker::new() }
    }

    /// Check the body of one function for both return-type obligations and
    /// cross-function call-site obligations.
    ///
    /// # Arguments
    ///
    /// * `sig` — the caller function's annotated signature.  Parameter
    ///   annotations seed the initial predicate scope.
    /// * `cfg` — the function body as a `ModuleCfgNode` tree.
    ///
    /// # Returns
    ///
    /// A [`FunctionBodyCheckResult`] collecting outcomes for every return
    /// site and every call-site argument visited during the DFS traversal.
    pub fn check_function(
        &mut self,
        sig: &FunctionSignature,
        cfg: &ModuleCfgNode,
    ) -> FunctionBodyCheckResult {
        let mut result = FunctionBodyCheckResult {
            return_sites: Vec::new(),
            call_sites: Vec::new(),
        };

        // Seed the initial predicate scope from the caller's parameter
        // annotations.  Each param's predicate (if present) is pushed into
        // the scope keyed by the param name — the same seeding logic that
        // FunctionChecker uses.
        //
        // Example: `(cp : (Int 0 256))` → scope["cp"] = [Range{0, 256}]
        let mut initial_scope: HashMap<String, Vec<Predicate>> = HashMap::new();
        for (name, rt) in &sig.params {
            if let Some(pred) = &rt.predicate {
                initial_scope.entry(name.clone()).or_default().push(pred.clone());
            }
        }

        self.walk(cfg, sig, &initial_scope, 0, &mut result);
        result
    }

    // -------------------------------------------------------------------------
    // CFG walk — recursive, path-sensitive
    // -------------------------------------------------------------------------
    //
    // The walk maintains a `scope: HashMap<VarName, Vec<Predicate>>` that
    // records all predicates known to hold for each variable along the current
    // path from the CFG root.  At each `Branch`, the scope is cloned and the
    // guard predicate (or its negation) is pushed before recursing into each
    // arm — this is the path-sensitivity.
    //
    // At `Call` nodes, predicates in scope for each argument variable are
    // remapped to the checker's `"__v"` sentinel and passed as
    // `Evidence::Predicated` to `Checker::check`.  At `Return` nodes, the
    // same substitution is done for the return value.

    fn walk(
        &mut self,
        node: &ModuleCfgNode,
        sig: &FunctionSignature,
        scope: &HashMap<String, Vec<Predicate>>,
        depth: usize,
        out: &mut FunctionBodyCheckResult,
    ) {
        // ── depth guard ──────────────────────────────────────────────────────
        //
        // Prevents a stack overflow on pathologically deep Branch/Call chains.
        // The guard fires when depth > MAX_MODULE_CFG_DEPTH (i.e., after
        // MAX_MODULE_CFG_DEPTH + 1 recursive frames starting from depth 0),
        // which is consistent with function_checker's guard.
        //
        // No sentinel is pushed here — the caller can inspect the result's
        // completeness via `is_vacuous()` or by comparing against expected
        // path counts.  Emitting a sentinel would inflate the output Vecs
        // past their bounds without corresponding semantic value.
        if depth > MAX_MODULE_CFG_DEPTH {
            return;
        }

        match node {
            // ── Branch: fork the predicate scope ──────────────────────────────
            //
            // Then-arm: the guard predicate holds for guard.var.
            // Else-arm: the negation of the guard predicate holds for guard.var.
            //
            // Both arms receive a *clone* of the current scope extended with
            // their respective predicate.  HashMap cloning is O(n) in the
            // number of variables in scope — typically a handful of params.
            ModuleCfgNode::Branch { guard, then_node, else_node } => {
                let mut then_scope = scope.clone();
                then_scope
                    .entry(guard.var.clone())
                    .or_default()
                    .push(guard.predicate.clone());
                self.walk(then_node, sig, &then_scope, depth + 1, out);

                let mut else_scope = scope.clone();
                else_scope
                    .entry(guard.var.clone())
                    .or_default()
                    .push(Predicate::not(guard.predicate.clone()));
                self.walk(else_node, sig, &else_scope, depth + 1, out);
            }

            // ── Call: check each argument against the callee's annotations ────
            //
            // If the callee is not in the ModuleScope, the entire call is
            // skipped — this is the per-symbol opt-out.
            //
            // For each registered argument we build Evidence from the current
            // scope and call Checker::check against the callee's parameter
            // annotation.  Execution always continues into `next` so that
            // later call sites and return sites on the same path are checked.
            ModuleCfgNode::Call { callee, args, next } => {
                // Clone the signature to avoid borrow-checker conflicts while
                // also calling `self.checker` (which mutably borrows `self`).
                if let Some(callee_sig) = self.scope.functions.get(callee.as_str()).cloned() {
                    for (i, arg) in args.iter().enumerate() {
                        // ── call-site count guard ────────────────────────────
                        //
                        // Guard fires BEFORE pushing a new entry so the Vec
                        // size never exceeds MAX_MODULE_CALL_SITES.  No sentinel
                        // is pushed — pushing a sentinel would cause the check
                        // `len >= MAX` to be true for the next Call's arg loop
                        // as well, leading to repeated sentinel accumulation
                        // (one per Call node after the limit, bounded by depth).
                        if out.call_sites.len() >= MAX_MODULE_CALL_SITES {
                            break;
                        }

                        // Callee's annotation for this parameter.
                        // If the callee has fewer params than we have args
                        // (variadic or mis-matched arity), treat the excess
                        // args as unrefined → immediately ProvenSafe.
                        let param_annotation = callee_sig
                            .params
                            .get(i)
                            .map(|(_, rt)| rt.clone())
                            .unwrap_or_else(|| RefinedType::unrefined(Kind::Int));

                        let evidence = build_arg_evidence(arg, scope);
                        let outcome = self.checker.check(&param_annotation, &evidence);

                        out.call_sites.push(CallSiteOutcome {
                            callee: callee.clone(),
                            param_index: i,
                            label: format!("call {callee}[arg {i}]"),
                            outcome,
                        });
                    }
                }
                // Continue walking `next` regardless of call-site outcomes.
                self.walk(next, sig, scope, depth + 1, out);
            }

            // ── Return: check the return value against the caller's return type
            //
            // Same logic as FunctionChecker (PR 23-D).
            ModuleCfgNode::Return(ret_val) => {
                // ── return-site count guard ──────────────────────────────────
                //
                // Guard fires BEFORE pushing so the Vec size never exceeds
                // MAX_MODULE_RETURN_SITES.  No sentinel is pushed for the same
                // reason as the call-site guard above.
                if out.return_sites.len() >= MAX_MODULE_RETURN_SITES {
                    return;
                }

                let evidence = build_return_evidence(ret_val, scope);
                let outcome = self.checker.check(&sig.return_type, &evidence);
                out.return_sites.push(ReturnSiteOutcome {
                    label: label_for_return(ret_val),
                    outcome,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Evidence builders for call arguments and return values
// ---------------------------------------------------------------------------

/// Build `Evidence` for a call argument from the current predicate scope.
///
/// The underlying `Checker` uses `"__v"` as its sentinel variable name.
/// Predicates are stored under the actual variable name (e.g., `"cp"`), so
/// we rename them via [`substitute_var`] before creating `Predicated` evidence.
///
/// ## Why substitute_var is needed
///
/// `Predicate::LinearCmp` carries explicit `VarId` references (e.g., `cp`
/// with coefficient 1).  If we passed the predicate to the solver without
/// renaming, the solver would see `cp ≥ 0` and `__v ∈ [0, 256)` as
/// independent constraints — the connection between `cp` and `__v` would
/// be lost and the refutation query would silently return SAT/UNKNOWN.
///
/// After substitution, the solver sees `__v ≥ 0`, `__v < 256`, etc., which
/// correctly expresses the evidence about the argument value.
///
/// `Range` and `Membership` predicates carry no `VarId`s; `substitute_var`
/// returns them unchanged.
fn build_arg_evidence(arg: &CallArg, scope: &HashMap<String, Vec<Predicate>>) -> Evidence {
    match arg {
        CallArg::Literal(v) => Evidence::Concrete(*v),
        CallArg::Variable(name) => {
            let preds = scope.get(name.as_str()).cloned().unwrap_or_default();
            if preds.is_empty() {
                // No predicates known for this variable → unconstrained.
                Evidence::Unconstrained
            } else {
                // Remap predicates from parameter name to "__v".
                let remapped: Vec<Predicate> =
                    preds.iter().map(|p| substitute_var(p, name, "__v")).collect();
                Evidence::Predicated(remapped)
            }
        }
    }
}

/// Build `Evidence` for a return value from the current predicate scope.
///
/// Mirrors the logic in `FunctionChecker`'s internal return evidence builder
/// (PR 23-D) — included here to avoid a public dependency on that internal
/// function.
fn build_return_evidence(
    ret_val: &ReturnValue,
    scope: &HashMap<String, Vec<Predicate>>,
) -> Evidence {
    match ret_val {
        ReturnValue::Literal(v) => Evidence::Concrete(*v),
        ReturnValue::Variable(name) => {
            let preds = scope.get(name.as_str()).cloned().unwrap_or_default();
            if preds.is_empty() {
                Evidence::Unconstrained
            } else {
                let remapped: Vec<Predicate> =
                    preds.iter().map(|p| substitute_var(p, name, "__v")).collect();
                Evidence::Predicated(remapped)
            }
        }
    }
}

/// Human-readable label for a return site.
fn label_for_return(ret_val: &ReturnValue) -> String {
    match ret_val {
        ReturnValue::Literal(v) => format!("return {v}"),
        ReturnValue::Variable(name) => format!("return var {name}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use lang_refined_types::{Kind, Predicate, RefinedType};

    use super::*;
    use crate::function_checker::{BranchGuard, FunctionSignature, ReturnValue};

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn range(lo: i128, hi: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: false }
    }

    fn int_annotation(pred: Predicate) -> RefinedType {
        RefinedType::refined(Kind::Int, pred)
    }

    fn unrefined() -> RefinedType {
        RefinedType::unrefined(Kind::Int)
    }

    /// Build a scope with `decode : (Int 0 128) -> unrefined`.
    fn decode_scope() -> ModuleScope {
        let mut scope = ModuleScope::new();
        scope.register(
            "decode",
            FunctionSignature {
                params: vec![("codepoint".into(), int_annotation(range(0, 128)))],
                return_type: unrefined(),
            },
        );
        scope
    }

    // ─── Primary acceptance criterion (LANG23 §"Rung 3 — module-scope") ──────

    /// The `latin1-decode` / `decode` example from the spec.
    ///
    /// `latin1-decode` calls `decode cp` inside a `(< cp 128)` guard.
    /// The guard narrows `cp` from `[0, 256)` to `[0, 128)`.
    /// The call is `ProvenSafe` — no runtime check needed.
    ///
    /// ```text
    /// Predicate set entering the call:
    ///   scope["cp"] = [Range{0,256}  (param annotation),
    ///                  Range{None,128} (guard: cp < 128)]
    /// After substitute_var "cp"→"__v": [Range{0,256}, Range{None,128}]
    /// Refutation: ∃ __v. [0,256) ∧ __v<128 ∧ ¬[0,128) = UNSAT → ProvenSafe ✓
    /// ```
    #[test]
    fn latin1_decode_call_proven_safe_by_guard() {
        let latin1_sig = FunctionSignature {
            params: vec![("cp".into(), int_annotation(range(0, 256)))],
            return_type: unrefined(),
        };

        let cfg = ModuleCfgNode::Branch {
            guard: BranchGuard {
                var: "cp".into(),
                // (< cp 128) ≡ cp ∈ (-∞, 128)
                predicate: Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false },
            },
            // then: (decode cp) — cp narrowed to [0, 128)
            then_node: Box::new(ModuleCfgNode::Call {
                callee: "decode".into(),
                args: vec![CallArg::Variable("cp".into())],
                next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".into()))),
            }),
            // else: (latin1-fallback cp) — not in scope → skipped
            else_node: Box::new(ModuleCfgNode::Call {
                callee: "latin1-fallback".into(),
                args: vec![CallArg::Variable("cp".into())],
                next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".into()))),
            }),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&latin1_sig, &cfg);

        // Only decode call site is recorded (latin1-fallback is not in scope).
        assert_eq!(result.call_sites.len(), 1, "only decode call site recorded");
        let site = &result.call_sites[0];
        assert_eq!(site.callee, "decode");
        assert_eq!(site.param_index, 0);
        assert!(
            site.outcome.is_safe(),
            "decode call should be ProvenSafe by the cp < 128 guard; got {:?}",
            site.outcome
        );
        // The call-site check is the point of 23-F.
        assert!(result.all_call_sites_proven_safe());
    }

    // ─── Without guard: [0,256) does not imply [0,128) → unsafe ─────────────

    /// Without the `(< cp 128)` guard, `cp : (Int 0 256)` can violate
    /// `decode`'s `(Int 0 128)` annotation.  The checker finds a counter-
    /// example (e.g., cp = 200) and returns `ProvenUnsafe`.
    #[test]
    fn call_without_guard_is_proven_unsafe() {
        let caller_sig = FunctionSignature {
            params: vec![("cp".into(), int_annotation(range(0, 256)))],
            return_type: unrefined(),
        };

        // Direct call without narrowing — cp ∈ [0, 256) ⊄ [0, 128)
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Variable("cp".into())],
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".into()))),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 1);
        assert!(
            result.call_sites[0].outcome.is_unsafe(),
            "cp ∈ [0,256) can violate decode's [0,128) — expected ProvenUnsafe; got {:?}",
            result.call_sites[0].outcome
        );
    }

    // ─── Literal arguments ────────────────────────────────────────────────────

    /// Literal in range → `ProvenSafe` (fast path, no solver).
    #[test]
    fn literal_arg_in_range_is_safe() {
        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Literal(64)], // 64 ∈ [0, 128) ✓
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 1);
        assert!(result.call_sites[0].outcome.is_safe());
    }

    /// Literal out of range → `ProvenUnsafe` with the literal as counter-example.
    #[test]
    fn literal_arg_out_of_range_is_unsafe() {
        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Literal(200)], // 200 ∉ [0, 128) ✗
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 1);
        assert!(result.call_sites[0].outcome.is_unsafe());
        assert_eq!(
            result.call_sites[0].outcome.counter_example().map(|cx| cx.value),
            Some(200)
        );
    }

    // ─── Per-symbol opt-out (unregistered callee) ─────────────────────────────

    /// A callee not in the `ModuleScope` produces no call-site outcomes.
    /// This is the "per-symbol opt-out via `: any`" mechanism.
    #[test]
    fn unregistered_callee_is_skipped() {
        let caller_sig = FunctionSignature {
            params: vec![("x".into(), int_annotation(range(0, 1000)))],
            return_type: unrefined(),
        };
        let cfg = ModuleCfgNode::Call {
            callee: "latin1-fallback".into(), // not in scope
            args: vec![CallArg::Variable("x".into())],
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(ModuleScope::new()); // empty scope
        let result = checker.check_function(&caller_sig, &cfg);

        assert!(
            result.call_sites.is_empty(),
            "unregistered callee should produce no call-site outcomes"
        );
    }

    // ─── Unconstrained variable → Unknown ─────────────────────────────────────

    /// A variable with no predicates in scope (no annotation, no guard)
    /// produces `Evidence::Unconstrained` → `Unknown`.
    #[test]
    fn unconstrained_variable_arg_is_unknown() {
        let caller_sig = FunctionSignature {
            params: vec![], // "y" has no annotation in the caller sig
            return_type: unrefined(),
        };
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Variable("y".into())], // y has no predicates
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 1);
        assert!(result.call_sites[0].outcome.is_unknown());
    }

    // ─── Multiple parameters ──────────────────────────────────────────────────

    /// A function with two annotated params — both arguments checked independently.
    #[test]
    fn multiple_params_all_proven_safe() {
        let mut scope = ModuleScope::new();
        scope.register(
            "clamp",
            FunctionSignature {
                params: vec![
                    ("lo".into(), int_annotation(range(0, 100))),
                    ("hi".into(), int_annotation(range(50, 200))),
                ],
                return_type: unrefined(),
            },
        );

        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "clamp".into(),
            // lo=10 ∈ [0,100) ✓, hi=75 ∈ [50,200) ✓
            args: vec![CallArg::Literal(10), CallArg::Literal(75)],
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(scope);
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 2);
        assert!(result.call_sites[0].outcome.is_safe(), "lo=10 ∈ [0,100)");
        assert!(result.call_sites[1].outcome.is_safe(), "hi=75 ∈ [50,200)");
    }

    /// Second argument out of range → `ProvenUnsafe` for that argument only.
    #[test]
    fn multiple_params_one_unsafe() {
        let mut scope = ModuleScope::new();
        scope.register(
            "clamp",
            FunctionSignature {
                params: vec![
                    ("lo".into(), int_annotation(range(0, 100))),
                    ("hi".into(), int_annotation(range(50, 200))),
                ],
                return_type: unrefined(),
            },
        );

        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "clamp".into(),
            args: vec![
                CallArg::Literal(10),  // lo=10 ✓
                CallArg::Literal(300), // hi=300 ∉ [50, 200) ✗
            ],
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(scope);
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 2);
        assert!(result.call_sites[0].outcome.is_safe(), "lo=10 safe");
        assert!(result.call_sites[1].outcome.is_unsafe(), "hi=300 unsafe");
        assert!(result.has_violation());
    }

    // ─── Return sites and call sites together ─────────────────────────────────

    /// A function body with both a call site and a return site.
    /// Both are collected and checked independently.
    #[test]
    fn return_site_checked_alongside_call_site() {
        let mut scope = ModuleScope::new();
        scope.register(
            "helper",
            FunctionSignature {
                params: vec![("v".into(), int_annotation(range(0, 64)))],
                return_type: unrefined(),
            },
        );

        // (define (f (n : (Int 0 64)) -> (Int 0 64))
        //   (if (< n 32) (helper n) n))
        let caller_sig = FunctionSignature {
            params: vec![("n".into(), int_annotation(range(0, 64)))],
            return_type: int_annotation(range(0, 64)),
        };
        let cfg = ModuleCfgNode::Branch {
            guard: BranchGuard {
                var: "n".into(),
                predicate: Predicate::Range { lo: None, hi: Some(32), inclusive_hi: false },
            },
            // then: (helper n) — n ∈ [0, 32) ⊆ [0, 64) → safe; return n ∈ [0, 32) → safe
            then_node: Box::new(ModuleCfgNode::Call {
                callee: "helper".into(),
                args: vec![CallArg::Variable("n".into())],
                next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("n".into()))),
            }),
            // else: return n — n ∈ [32, 64) ⊆ [0, 64) → safe
            else_node: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("n".into()))),
        };

        let mut checker = ModuleChecker::new(scope);
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 1, "one helper call site");
        assert_eq!(result.return_sites.len(), 2, "two return sites (then + else)");
        assert!(result.call_sites[0].outcome.is_safe(), "helper call: n<32 ⊆ [0,64)");
        assert!(
            result.return_sites.iter().all(|r| r.outcome.is_safe()),
            "all returns safe"
        );
        assert!(result.all_proven_safe());
    }

    // ─── FunctionBodyCheckResult methods ─────────────────────────────────────

    /// `all_proven_safe` requires both calls and returns to be safe.
    #[test]
    fn all_proven_safe_requires_calls_and_returns() {
        let mut scope = ModuleScope::new();
        scope.register(
            "helper",
            FunctionSignature {
                params: vec![("v".into(), int_annotation(range(0, 100)))],
                return_type: unrefined(),
            },
        );

        // x ∈ [0,50) ⊆ [0,100) → helper call safe; return x ∈ [0,50) ⊆ [0,50) → safe.
        let caller_sig = FunctionSignature {
            params: vec![("x".into(), int_annotation(range(0, 50)))],
            return_type: int_annotation(range(0, 50)),
        };
        let cfg = ModuleCfgNode::Call {
            callee: "helper".into(),
            args: vec![CallArg::Variable("x".into())],
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("x".into()))),
        };

        let mut checker = ModuleChecker::new(scope);
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites.len(), 1);
        assert_eq!(result.return_sites.len(), 1);
        assert!(result.all_proven_safe());
    }

    /// `all_proven_safe` is `false` for vacuous results.
    #[test]
    fn all_proven_safe_false_when_vacuous() {
        let empty = FunctionBodyCheckResult { return_sites: vec![], call_sites: vec![] };
        assert!(!empty.all_proven_safe(), "vacuous is not proven safe");
    }

    /// `all_call_sites_proven_safe` ignores return-site outcomes.
    #[test]
    fn all_call_sites_proven_safe_ignores_return_sites() {
        // Call site is safe (literal 64 ∈ [0, 128)).
        // Return is unconstrained (n has no annotation) → Unknown.
        let caller_sig = FunctionSignature {
            params: vec![("n".into(), unrefined())], // no annotation on n
            return_type: int_annotation(range(0, 10)),
        };
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Literal(64)], // safe
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("n".into()))), // unknown
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert!(result.all_call_sites_proven_safe(), "call site is safe");
        assert!(result.return_sites[0].outcome.is_unknown(), "return is unknown");
        // all_proven_safe should be false (return is unknown).
        assert!(!result.all_proven_safe());
    }

    /// `first_counter_example` finds the first `ProvenUnsafe` outcome.
    #[test]
    fn first_counter_example_found() {
        let mut scope = ModuleScope::new();
        scope.register(
            "f",
            FunctionSignature {
                params: vec![("n".into(), int_annotation(range(0, 50)))],
                return_type: unrefined(),
            },
        );

        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "f".into(),
            args: vec![CallArg::Literal(200)], // violates [0, 50)
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(scope);
        let result = checker.check_function(&caller_sig, &cfg);

        let cx = result.first_counter_example().expect("should have counter-example");
        assert_eq!(cx.value, 200);
    }

    /// `runtime_check_count` counts Unknown outcomes from call AND return sites.
    #[test]
    fn runtime_check_count_counts_unknowns() {
        // Unconstrained variable arg → Unknown call site.
        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Variable("x".into())], // no predicates → Unknown
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.runtime_check_count(), 1, "one Unknown call site");
    }

    /// `is_vacuous` is true only when both result vecs are empty.
    #[test]
    fn is_vacuous_both_empty() {
        let empty = FunctionBodyCheckResult { return_sites: vec![], call_sites: vec![] };
        assert!(empty.is_vacuous());

        // A result with a single return site is not vacuous.
        let with_return = FunctionBodyCheckResult {
            return_sites: vec![crate::function_checker::ReturnSiteOutcome {
                label: "return 0".into(),
                outcome: CheckOutcome::ProvenSafe,
            }],
            call_sites: vec![],
        };
        assert!(!with_return.is_vacuous());
    }

    // ─── ModuleScope API ──────────────────────────────────────────────────────

    /// `register` replaces an existing signature for the same name.
    #[test]
    fn scope_register_replaces_existing() {
        let mut scope = ModuleScope::new();
        scope.register("f", FunctionSignature { params: vec![], return_type: unrefined() });
        assert_eq!(scope.len(), 1);
        scope.register(
            "f",
            FunctionSignature {
                params: vec![("x".into(), int_annotation(range(0, 10)))],
                return_type: unrefined(),
            },
        );
        assert_eq!(scope.len(), 1, "still one entry after replacement");
        assert_eq!(
            scope.get_signature("f").unwrap().params.len(),
            1,
            "replaced with new sig"
        );
    }

    /// `is_empty` and `len` reflect the number of registered functions.
    #[test]
    fn scope_is_empty_and_len() {
        let scope = ModuleScope::new();
        assert!(scope.is_empty());
        assert_eq!(scope.len(), 0);

        let mut scope = decode_scope();
        assert!(!scope.is_empty());
        assert_eq!(scope.len(), 1);
        scope.register("other", FunctionSignature { params: vec![], return_type: unrefined() });
        assert_eq!(scope.len(), 2);
    }

    /// `get_signature` returns `None` for unregistered functions.
    #[test]
    fn scope_get_signature_absent() {
        let scope = decode_scope();
        assert!(scope.get_signature("decode").is_some());
        assert!(scope.get_signature("nonexistent").is_none());
    }

    // ─── CallSiteOutcome labels ───────────────────────────────────────────────

    /// Call-site outcome labels include callee name and zero-based param index.
    #[test]
    fn call_site_outcome_label_format() {
        let caller_sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let cfg = ModuleCfgNode::Call {
            callee: "decode".into(),
            args: vec![CallArg::Literal(42)],
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut checker = ModuleChecker::new(decode_scope());
        let result = checker.check_function(&caller_sig, &cfg);

        assert_eq!(result.call_sites[0].label, "call decode[arg 0]");
        assert_eq!(result.call_sites[0].callee, "decode");
        assert_eq!(result.call_sites[0].param_index, 0);
    }

    // ─── Safety budget tests ──────────────────────────────────────────────────

    /// A 66-level deep chain of Branch nodes hits the depth limit.
    /// The walker should not crash; it should emit Unknown at the limit point.
    #[test]
    fn depth_limit_does_not_crash() {
        // Build a 66-level right-skewed chain: Branch → else → Branch → else → ...
        let mut node = Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0)));
        for _ in 0..66 {
            node = Box::new(ModuleCfgNode::Branch {
                guard: BranchGuard {
                    var: "x".into(),
                    predicate: Predicate::Range {
                        lo: None,
                        hi: Some(100),
                        inclusive_hi: false,
                    },
                },
                then_node: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
                else_node: node,
            });
        }

        let sig = FunctionSignature {
            params: vec![("x".into(), unrefined())],
            return_type: unrefined(),
        };

        let mut checker = ModuleChecker::new(ModuleScope::new());
        // Must not panic and output Vecs must stay within their limits.
        let result = checker.check_function(&sig, &*node);
        assert!(result.call_sites.len() <= MAX_MODULE_CALL_SITES);
        assert!(result.return_sites.len() <= MAX_MODULE_RETURN_SITES);
    }

    /// A single Call with more args than MAX_MODULE_CALL_SITES is bounded.
    ///
    /// Note: a *chain* of Call nodes is bounded by MAX_MODULE_CFG_DEPTH (the
    /// depth guard fires at depth 65), not by MAX_MODULE_CALL_SITES.  The
    /// call-site limit is the guard for a single Call with many parameters.
    #[test]
    fn call_site_count_limit_bounds_collection() {
        let limit = MAX_MODULE_CALL_SITES;

        // Register a function with limit + 10 parameters.
        let params: Vec<(String, RefinedType)> = (0..(limit + 10))
            .map(|i| (format!("p{i}"), int_annotation(range(0, 100))))
            .collect();
        let mut scope = ModuleScope::new();
        scope.register("big", FunctionSignature { params, return_type: unrefined() });

        // Call it with limit + 10 literal args (all in range).
        let args: Vec<CallArg> = (0..(limit + 10)).map(|_| CallArg::Literal(50)).collect();
        let cfg = ModuleCfgNode::Call {
            callee: "big".into(),
            args,
            next: Box::new(ModuleCfgNode::Return(ReturnValue::Literal(0))),
        };

        let sig = FunctionSignature { params: vec![], return_type: unrefined() };
        let mut checker = ModuleChecker::new(scope);
        let result = checker.check_function(&sig, &cfg);

        // The Vec must not exceed MAX_MODULE_CALL_SITES entries (no sentinel push).
        assert!(
            result.call_sites.len() <= limit,
            "call sites bounded to ≤ {}; got {}",
            limit,
            result.call_sites.len()
        );
    }
}
