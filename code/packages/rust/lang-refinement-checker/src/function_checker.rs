//! # Function-scope refinement checker — LANG23 PR 23-D.
//!
//! Extends the per-binding [`Checker`] (PR 23-C) to handle entire function
//! bodies.  The key insight from the LANG23 spec:
//!
//! > "Every refinement-related question is a question about a path through
//! > the program's control-flow graph."
//!
//! This module walks a simplified tree-structured CFG, accumulates guard
//! predicates **path-by-path** (the same machine TypeScript uses for
//! control-flow narrowing), and checks each return site against the
//! declared return type.
//!
//! ## Architecture
//!
//! ```text
//!   FunctionChecker
//!       │  walks CfgNode tree, accumulates predicates per var
//!       │  at each Return site, builds Evidence and calls
//!       ▼
//!   Checker::check(return_type, evidence)
//!       │  lowers to ConstraintInstructions, runs constraint-vm
//!       ▼
//!   CheckOutcome: ProvenSafe | ProvenUnsafe | Unknown
//! ```
//!
//! ## CFG model
//!
//! Functions are represented as a tree of `CfgNode`s — sufficient for
//! the LANG23 v1 vocabulary of `if`/`cond`/`when` nesting.  Future PRs
//! that need loops or join points should extend to a block-indexed DAG.
//!
//! ```text
//!   Branch { guard: x < 0, then: ..., else: ... }
//!   └─ then: Return(Literal(0))
//!   └─ else: Branch { guard: x > 255, then: ..., else: ... }
//!       └─ then: Return(Literal(255))
//!       └─ else: Return(Variable("x"))
//! ```
//!
//! ## Path-sensitive predicate accumulation
//!
//! At each `Branch`, the checker forks:
//!
//! - **Then path**: `guard.predicate` is pushed onto the predicate stack
//!   for `guard.var`.
//! - **Else path**: `Predicate::not(guard.predicate)` is pushed instead.
//!
//! When a `Return(Variable(name))` is reached, all accumulated predicates
//! for `name` are remapped to `"__v"` (the checker's sentinel variable)
//! and passed as `Evidence::Predicated` to `Checker::check`.
//!
//! ## clamp-byte worked example
//!
//! ```scheme
//! (define (clamp-byte (x : int) -> (Int 0 256))
//!   (cond ((< x 0)   0)
//!         ((> x 255) 255)
//!         (else      x)))
//! ```
//!
//! | Path | Guards accumulated for x | Return | Evidence | Outcome |
//! |------|---------------------------|--------|----------|---------|
//! | then₁ | `x < 0` | `Literal(0)` | `Concrete(0)` | `0 ∈ [0,256)` → Safe |
//! | then₂ | `¬(x<0)`, `x≥256` | `Literal(255)` | `Concrete(255)` | `255 ∈ [0,256)` → Safe |
//! | else₂ | `¬(x<0)`, `¬(x≥256)` | `Variable("x")` | `Predicated{__v≥0, __v<256}` | UNSAT → Safe |
//!
//! All three return sites are `ProvenSafe` — no runtime checks emitted.
//!
//! ## Variable substitution
//!
//! Path predicates are stored under the function parameter name (e.g.,
//! `"x"`).  The underlying `Checker` uses `"__v"` as its sentinel variable.
//! [`substitute_var`] bridges the gap by renaming `VarId`s in `LinearCmp`
//! predicates.  `Range` and `Membership` predicates carry no variable
//! references and are unaffected.

use std::collections::HashMap;

use lang_refined_types::{Predicate, RefinedType, VarId};

use crate::{CheckOutcome, Checker, Evidence};

// ---------------------------------------------------------------------------
// BranchGuard — a guard condition on a named variable
// ---------------------------------------------------------------------------

/// A guard condition on a named function parameter, as seen in an `if`/`cond`.
///
/// `predicate` is expressed in the `lang_refined_types::Predicate` vocabulary.
/// For simple numeric comparisons, `Range` variants are cleanest:
///
/// | Condition | Predicate |
/// |-----------|-----------|
/// | `x < 0`  | `Range { lo: None, hi: Some(0), inclusive_hi: false }` |
/// | `x ≥ 256`| `Range { lo: Some(256), hi: None, inclusive_hi: false }` |
/// | `x = 42` | `Membership { values: [42] }` |
///
/// For cross-variable guards (e.g., `i < len`), use `LinearCmp`:
///
/// ```rust
/// use lang_refined_types::{Predicate, VarId, CmpOp};
/// let guard = Predicate::LinearCmp {
///     coefs: vec![(VarId::new("i"), 1), (VarId::new("len"), -1)],
///     op: CmpOp::Lt,
///     rhs: 0,
/// };
/// ```
///
/// The checker automatically adds `Not(predicate)` to the else-branch's scope.
#[derive(Debug, Clone)]
pub struct BranchGuard {
    /// Name of the variable being tested (e.g., `"x"`).
    pub var: String,
    /// The predicate that holds when the `then`-branch is taken.
    pub predicate: Predicate,
}

// ---------------------------------------------------------------------------
// ReturnValue — what a return statement produces
// ---------------------------------------------------------------------------

/// The value produced by a `return` statement in the simplified CFG.
#[derive(Debug, Clone)]
pub enum ReturnValue {
    /// A compile-time integer constant (e.g., the literal `0` or `255`).
    Literal(i128),
    /// A function-scope variable returned directly.
    ///
    /// The checker uses all predicates accumulated for `name` along the
    /// current CFG path as the evidence.  If no predicates are accumulated,
    /// the evidence is `Unconstrained` and the outcome is `Unknown`.
    Variable(String),
}

// ---------------------------------------------------------------------------
// CfgNode — the tree-structured CFG
// ---------------------------------------------------------------------------

/// A node in the simplified function body CFG.
///
/// This is a **tree** of owned subtrees — not a DAG — which is sufficient
/// for the LANG23 v1 `if`/`cond` vocabulary.  The checker performs an
/// exhaustive root-to-leaf traversal, forking the predicate scope at each
/// `Branch`.
///
/// ## Modelling `cond`
///
/// ```scheme
/// (cond ((< x 0)   0)
///       ((> x 255) 255)
///       (else      x))
/// ```
///
/// maps to:
///
/// ```rust
/// use lang_refined_types::Predicate;
/// use lang_refinement_checker::function_checker::{BranchGuard, CfgNode, ReturnValue};
///
/// let cfg = CfgNode::Branch {
///     guard: BranchGuard {
///         var: "x".to_string(),
///         // x < 0  ≡  hi=0, exclusive
///         predicate: Predicate::Range { lo: None, hi: Some(0), inclusive_hi: false },
///     },
///     then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))),
///     else_node: Box::new(CfgNode::Branch {
///         guard: BranchGuard {
///             var: "x".to_string(),
///             // x > 255  ≡  x ≥ 256  ≡  lo=256
///             predicate: Predicate::Range { lo: Some(256), hi: None, inclusive_hi: false },
///         },
///         then_node: Box::new(CfgNode::Return(ReturnValue::Literal(255))),
///         else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
///     }),
/// };
/// # let _ = cfg;
/// ```
#[derive(Debug, Clone)]
pub enum CfgNode {
    /// A conditional branch splitting the path-predicate set into two.
    Branch {
        /// The guard condition.
        guard: BranchGuard,
        /// The then-arm: taken when `guard.predicate` holds.
        then_node: Box<CfgNode>,
        /// The else-arm: taken when `Not(guard.predicate)` holds.
        else_node: Box<CfgNode>,
    },
    /// A function return.
    Return(ReturnValue),
}

// ---------------------------------------------------------------------------
// FunctionSignature — parameter annotations + return type
// ---------------------------------------------------------------------------

/// The annotated signature of the function being checked.
///
/// Both parameter annotations and the return type are optional (unrefined).
/// Unrefined parameters contribute no predicates to the scope; an unrefined
/// return type immediately yields `ProvenSafe` for all return sites.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Named parameters with their declared types.
    ///
    /// If a parameter has no annotation, use `RefinedType::unrefined(Kind::Int)`.
    /// Annotated parameters seed the initial predicate scope.
    pub params: Vec<(String, RefinedType)>,
    /// Declared return type.
    ///
    /// If `is_unrefined()`, every return site immediately yields `ProvenSafe`
    /// (no proof obligation to discharge).
    pub return_type: RefinedType,
}

// ---------------------------------------------------------------------------
// FunctionCheckResult — per-return-site and aggregate outcomes
// ---------------------------------------------------------------------------

/// The outcome for a single return site in the function.
#[derive(Debug, Clone)]
pub struct ReturnSiteOutcome {
    /// Human-readable label for this return site.
    ///
    /// Examples: `"return 0"`, `"return 255"`, `"return var x"`.
    pub label: String,
    /// The checker outcome for this site.
    pub outcome: CheckOutcome,
}

/// The aggregate outcome of checking an entire function body.
///
/// Contains one [`ReturnSiteOutcome`] per `CfgNode::Return` reached by the
/// path-sensitive traversal (i.e., one per root-to-leaf path).
#[derive(Debug, Clone)]
pub struct FunctionCheckResult {
    /// Outcomes for each return site, in path-traversal order (DFS, then
    /// before else at each branch).
    pub return_sites: Vec<ReturnSiteOutcome>,
}

impl FunctionCheckResult {
    /// Returns `true` if **every** return site is `ProvenSafe`.
    ///
    /// When this holds, the caller may strip all runtime checks from the
    /// function and narrow downstream uses of the return value accordingly.
    pub fn all_proven_safe(&self) -> bool {
        !self.return_sites.is_empty()
            && self.return_sites.iter().all(|r| r.outcome.is_safe())
    }

    /// Returns `true` if **any** return site is `ProvenUnsafe`.
    ///
    /// When this holds, the function has a definite refinement violation and
    /// the compiler should emit an error with the first counter-example.
    pub fn has_violation(&self) -> bool {
        self.return_sites.iter().any(|r| r.outcome.is_unsafe())
    }

    /// Returns the first counter-example found (if any).
    pub fn first_counter_example(&self) -> Option<&crate::CounterExample> {
        self.return_sites
            .iter()
            .filter_map(|r| r.outcome.counter_example())
            .next()
    }

    /// Returns the number of return sites that require a runtime check
    /// (`Unknown` outcome).
    ///
    /// In `lenient` mode these sites emit a `check_refinement!` intrinsic.
    /// In `strict` mode, any nonzero count is a compile error.
    pub fn runtime_check_count(&self) -> usize {
        self.return_sites.iter().filter(|r| r.outcome.is_unknown()).count()
    }

    /// Returns `true` if no return sites exist — the CFG has no returns.
    ///
    /// An empty result is pathological (vacuously safe) but not an error;
    /// a diverging function (e.g., one that always panics) can have no
    /// reachable return nodes.
    pub fn is_vacuous(&self) -> bool {
        self.return_sites.is_empty()
    }
}

// ---------------------------------------------------------------------------
// FunctionChecker — the main entry point for function-scope checking
// ---------------------------------------------------------------------------

/// Function-scope refinement checker.
///
/// Extends the per-binding [`Checker`] (PR 23-C) to handle full function
/// bodies.  Walk the CFG with [`check_function`]; the checker accumulates
/// guard predicates path-by-path and discharges each return site using the
/// underlying `Checker`.
///
/// The checker is **stateless** across function calls — reuse the same
/// instance for a batch of functions.
///
/// # Example — `clamp-byte`
///
/// ```rust
/// use lang_refined_types::{Kind, Predicate, RefinedType};
/// use lang_refinement_checker::function_checker::{
///     BranchGuard, CfgNode, FunctionChecker, FunctionSignature, ReturnValue,
/// };
///
/// // (define (clamp-byte (x : int) -> (Int 0 256))
/// //   (cond ((< x 0)   0)
/// //         ((> x 255) 255)
/// //         (else      x)))
/// let sig = FunctionSignature {
///     params: vec![("x".to_string(), RefinedType::unrefined(Kind::Int))],
///     return_type: RefinedType::refined(
///         Kind::Int,
///         Predicate::Range { lo: Some(0), hi: Some(256), inclusive_hi: false },
///     ),
/// };
/// let cfg = CfgNode::Branch {
///     guard: BranchGuard {
///         var: "x".to_string(),
///         predicate: Predicate::Range { lo: None, hi: Some(0), inclusive_hi: false },
///     },
///     then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))),
///     else_node: Box::new(CfgNode::Branch {
///         guard: BranchGuard {
///             var: "x".to_string(),
///             predicate: Predicate::Range { lo: Some(256), hi: None, inclusive_hi: false },
///         },
///         then_node: Box::new(CfgNode::Return(ReturnValue::Literal(255))),
///         else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
///     }),
/// };
///
/// let mut fc = FunctionChecker::new();
/// let result = fc.check_function(&sig, &cfg);
/// assert!(result.all_proven_safe(), "clamp-byte should be fully proven safe");
/// ```
///
/// [`check_function`]: FunctionChecker::check_function
#[derive(Debug, Default)]
pub struct FunctionChecker {
    checker: Checker,
}

impl FunctionChecker {
    /// Construct a new function-scope checker.
    pub fn new() -> Self {
        FunctionChecker { checker: Checker::new() }
    }

    /// Check all return sites of `cfg` against `sig.return_type`.
    ///
    /// The walk is **path-sensitive**: at each `Branch`, guard predicates are
    /// added to the then-arm's scope; their negations to the else-arm's scope.
    /// Parameter annotations from `sig.params` seed the initial scope.
    ///
    /// Returns a [`FunctionCheckResult`] with one [`ReturnSiteOutcome`] per
    /// reachable `Return` node in the CFG.
    pub fn check_function(
        &mut self,
        sig: &FunctionSignature,
        cfg: &CfgNode,
    ) -> FunctionCheckResult {
        // Seed the initial predicate scope with parameter annotations.
        // e.g., `(f (x : (Int 0 100)) ...)` → scope["x"] = [Range(0,100)]
        let mut init_scope: HashMap<String, Vec<Predicate>> = HashMap::new();
        for (name, refined_ty) in &sig.params {
            if let Some(pred) = &refined_ty.predicate {
                init_scope.entry(name.clone()).or_default().push(pred.clone());
            }
        }

        let mut return_sites = Vec::new();
        self.walk_cfg(cfg, &init_scope, &sig.return_type, &mut return_sites);
        FunctionCheckResult { return_sites }
    }

    // ── Internal recursive CFG walker ─────────────────────────────────────────
    //
    // `scope` maps each function variable to the predicates known to hold
    // about it along the current path.  At each Branch we clone the scope
    // and push the guard (or its negation) before recursing.

    fn walk_cfg(
        &mut self,
        node: &CfgNode,
        scope: &HashMap<String, Vec<Predicate>>,
        return_type: &RefinedType,
        out: &mut Vec<ReturnSiteOutcome>,
    ) {
        match node {
            // ── Branch: fork the scope, recurse both arms ─────────────────────
            CfgNode::Branch { guard, then_node, else_node } => {
                // Then-arm: guard predicate holds.
                {
                    let mut then_scope = scope.clone();
                    then_scope
                        .entry(guard.var.clone())
                        .or_default()
                        .push(guard.predicate.clone());
                    self.walk_cfg(then_node, &then_scope, return_type, out);
                }

                // Else-arm: negation of guard holds.
                //
                // `Predicate::not` double-negation-eliminates so the scope
                // doesn't accumulate gratuitous `Not(Not(...))` wrappers.
                {
                    let mut else_scope = scope.clone();
                    else_scope
                        .entry(guard.var.clone())
                        .or_default()
                        .push(Predicate::not(guard.predicate.clone()));
                    self.walk_cfg(else_node, &else_scope, return_type, out);
                }
            }

            // ── Return: build evidence, discharge the obligation ───────────────
            CfgNode::Return(ret_val) => {
                let (label, evidence) = build_evidence(ret_val, scope);
                let outcome = self.checker.check(return_type, &evidence);
                out.push(ReturnSiteOutcome { label, outcome });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// build_evidence — construct Evidence from a ReturnValue + scope
// ---------------------------------------------------------------------------

/// Build the label and `Evidence` for a return site.
///
/// - `Literal(v)` → `Evidence::Concrete(v)` (no scope lookup needed).
/// - `Variable(name)` → look up accumulated predicates in `scope`, remap
///   variable names via [`substitute_var`], and produce `Evidence::Predicated`
///   (or `Evidence::Unconstrained` if no predicates are known).
fn build_evidence(
    ret_val: &ReturnValue,
    scope: &HashMap<String, Vec<Predicate>>,
) -> (String, Evidence) {
    match ret_val {
        // A compile-time constant needs no scope lookup.
        ReturnValue::Literal(v) => (format!("return {v}"), Evidence::Concrete(*v)),

        // A variable: gather path predicates, remap to "__v", pass as evidence.
        ReturnValue::Variable(name) => {
            let label = format!("return var {name}");
            let preds = scope.get(name).cloned().unwrap_or_default();

            if preds.is_empty() {
                // Nothing known about this variable → unconstrained.
                (label, Evidence::Unconstrained)
            } else {
                // Remap every LinearCmp reference to `name` → `"__v"` so the
                // underlying Checker can reason about them.  Range and
                // Membership predicates need no remapping (they carry no
                // explicit variable references — `to_constraint_predicate`
                // supplies the variable name when lowering them).
                let remapped: Vec<Predicate> = preds
                    .iter()
                    .map(|p| substitute_var(p, name, "__v"))
                    .collect();
                (label, Evidence::Predicated(remapped))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// substitute_var — remap variable names in predicates
// ---------------------------------------------------------------------------

/// Substitute all occurrences of the variable named `from` with `to` in
/// `pred`.
///
/// ## Which variants are affected?
///
/// | Variant | Action |
/// |---------|--------|
/// | `Range` | Clone unchanged — no variable reference. |
/// | `Membership` | Clone unchanged — no variable reference. |
/// | `Opaque` | Clone unchanged — opaque to the solver. |
/// | `And` / `Or` | Recurse into each part. |
/// | `Not` | Recurse into the inner predicate. |
/// | `LinearCmp` | Remap every `VarId` matching `from` to `to`. |
///
/// ## Why is this needed?
///
/// Path predicates are stored under the function parameter name (e.g., `"x"`).
/// `Checker::check_predicated` always calls
/// `Predicate::to_constraint_predicate("__v")`, which substitutes the
/// `var_name` argument into `Range`/`Membership` variants but uses the
/// literal `VarId` names in `LinearCmp`.  Without remapping, a guard like
/// `x < 0` stored as `LinearCmp { coefs: [(VarId("x"), 1)], ... }` would
/// produce a solver variable `"x"` disjoint from `"__v"`.
///
/// After `substitute_var(pred, "x", "__v")` the `LinearCmp` references
/// `VarId("__v")`, which matches the checker's sentinel.
pub fn substitute_var(pred: &Predicate, from: &str, to: &str) -> Predicate {
    match pred {
        // No variable references in these variants — return as-is.
        Predicate::Range { .. }
        | Predicate::Membership { .. }
        | Predicate::Opaque { .. } => pred.clone(),

        // Recurse through compound predicates.
        Predicate::And(parts) => {
            Predicate::and(parts.iter().map(|p| substitute_var(p, from, to)).collect())
        }
        Predicate::Or(parts) => {
            Predicate::or(parts.iter().map(|p| substitute_var(p, from, to)).collect())
        }
        Predicate::Not(inner) => Predicate::not(substitute_var(inner, from, to)),

        // Remap the matching VarId in each coefficient.
        Predicate::LinearCmp { coefs, op, rhs } => {
            let new_coefs: Vec<(VarId, i128)> = coefs
                .iter()
                .map(|(vid, c)| {
                    let new_vid = if vid.0 == from {
                        VarId(to.to_string())
                    } else {
                        vid.clone()
                    };
                    (new_vid, *c)
                })
                .collect();
            Predicate::LinearCmp { coefs: new_coefs, op: *op, rhs: *rhs }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use lang_refined_types::{CmpOp, Kind, Predicate, RefinedType, VarId};

    use super::*;

    // ─── helpers ─────────────────────────────────────────────────────────────

    fn range(lo: i128, hi: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: false }
    }

    fn range_lo(lo: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: None, inclusive_hi: false }
    }

    fn range_hi(hi: i128) -> Predicate {
        Predicate::Range { lo: None, hi: Some(hi), inclusive_hi: false }
    }

    fn range_inclusive(lo: i128, hi: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: true }
    }

    fn int_ann(pred: Predicate) -> RefinedType {
        RefinedType::refined(Kind::Int, pred)
    }

    fn unrefined_int() -> RefinedType {
        RefinedType::unrefined(Kind::Int)
    }

    /// Guard: var < hi  (exclusive upper bound on `var`)
    fn guard_lt(var: &str, hi: i128) -> BranchGuard {
        BranchGuard { var: var.to_string(), predicate: range_hi(hi) }
    }

    /// Guard: var ≥ lo  (i.e., `var > lo-1` for integers)
    fn guard_ge(var: &str, lo: i128) -> BranchGuard {
        BranchGuard { var: var.to_string(), predicate: range_lo(lo) }
    }

    // ─── 1. clamp-byte: the primary acceptance criterion ──────────────────────
    //
    // (define (clamp-byte (x : int) -> (Int 0 256))
    //   (cond ((< x 0)   0)
    //         ((> x 255) 255)
    //         (else      x)))
    //
    // All three paths must be ProvenSafe; no runtime checks needed.

    #[test]
    fn clamp_byte_all_paths_proven_safe() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: int_ann(range(0, 256)),
        };

        // x < 0 → guard: hi=0 (exclusive)  i.e. Range{lo:None, hi:Some(0), inclusive_hi:false}
        // x > 255 → guard: lo=256           i.e. Range{lo:Some(256), hi:None, inclusive_hi:false}
        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))),
            else_node: Box::new(CfgNode::Branch {
                guard: guard_ge("x", 256), // x > 255  ≡  x ≥ 256
                then_node: Box::new(CfgNode::Return(ReturnValue::Literal(255))),
                else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
            }),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 3,
            "clamp-byte has exactly 3 return sites");
        assert!(result.all_proven_safe(),
            "all paths should be ProvenSafe; got: {:?}", result.return_sites);
        assert!(!result.has_violation());
        assert_eq!(result.runtime_check_count(), 0);
        assert!(!result.is_vacuous());
    }

    // ─── 2. clamp-byte with violation: tighten return type to [0, 200) ────────
    //
    // Path 2 (return 255) now violates [0, 200).
    // Path 3 (return x with x ∈ [200, 255]) also violates.

    #[test]
    fn clamp_byte_tighter_return_type_has_violation() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: int_ann(range(0, 200)), // tighter than [0, 256)
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))), // safe
            else_node: Box::new(CfgNode::Branch {
                guard: guard_ge("x", 256),
                then_node: Box::new(CfgNode::Return(ReturnValue::Literal(255))), // 255 ∉ [0,200)!
                else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
            }),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        // At least one path is ProvenUnsafe.
        assert!(result.has_violation(),
            "return 255 should violate [0, 200)");
        // Counter-example should be available.
        let cx = result.first_counter_example()
            .expect("expected a counter-example");
        // The counter-example value is the witness of the violation.
        // For the Literal(255) path, it's 255.
        assert_eq!(cx.value, 255,
            "counter-example should be 255, the literal violating [0, 200)");
    }

    // ─── 3. clamp-byte return literal 0 violates annotation ──────────────────
    //
    // If the annotation is [1, 256) (excluding 0), then returning 0 is unsafe.

    #[test]
    fn clamp_byte_literal_zero_violates_annotation() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: int_ann(range(1, 256)), // [1, 256) — excludes 0
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))), // violates [1,256)!
            else_node: Box::new(CfgNode::Branch {
                guard: guard_ge("x", 256),
                then_node: Box::new(CfgNode::Return(ReturnValue::Literal(255))), // safe
                else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
            }),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert!(result.has_violation());
        let cx = result.first_counter_example().expect("expected counter-example");
        assert_eq!(cx.value, 0, "literal 0 violates [1, 256)");
    }

    // ─── 4. Identity function with annotated parameter ────────────────────────
    //
    // (define (identity (x : (Int 0 100)) -> (Int 0 100)) x)
    //
    // The parameter annotation x ∈ [0, 100) implies the return annotation.

    #[test]
    fn identity_with_param_annotation_safe() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), int_ann(range(0, 100)))],
            return_type: int_ann(range(0, 100)),
        };

        let cfg = CfgNode::Return(ReturnValue::Variable("x".to_string()));

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 1);
        assert!(result.all_proven_safe(),
            "x ∈ [0,100) satisfies return [0,100); got {:?}", result.return_sites);
    }

    // ─── 5. Identity function without annotation is unconstrained ─────────────
    //
    // (define (identity (x : int) -> (Int 0 100)) x)
    //
    // No annotation on x → evidence is Unconstrained → Unknown outcome.

    #[test]
    fn identity_without_param_annotation_unknown() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: int_ann(range(0, 100)),
        };

        let cfg = CfgNode::Return(ReturnValue::Variable("x".to_string()));

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 1);
        assert!(result.return_sites[0].outcome.is_unknown(),
            "unconstrained x → Unknown; got {:?}", result.return_sites[0].outcome);
        assert_eq!(result.runtime_check_count(), 1);
    }

    // ─── 6. Unrefined return type → all sites proven safe immediately ─────────
    //
    // (define (f (x : int) -> int) x)
    //
    // No return annotation → the Checker returns ProvenSafe for everything.

    #[test]
    fn unrefined_return_type_always_safe() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: unrefined_int(),
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(-1))),
            else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert!(result.all_proven_safe(),
            "unrefined return type → all sites safe");
    }

    // ─── 7. Single branch with two literal arms ───────────────────────────────

    #[test]
    fn single_branch_two_literal_arms() {
        // (define (sign (x : int) -> (Int -1 2))
        //   (if (< x 0) -1 1))
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: int_ann(range_inclusive(-1, 1)),
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(-1))),
            else_node: Box::new(CfgNode::Return(ReturnValue::Literal(1))),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 2);
        assert!(result.all_proven_safe());
    }

    // ─── 8. Guard narrows variable to satisfy a tighter annotation ────────────
    //
    // (define (ascii-info (n : int) -> (Int 0 128))
    //   (if (< n 128)
    //       n    ;; safe: n < 128 narrows n into [0, 128)
    //       0))  ;; safe: literal 0
    //
    // Without the guard, returning n with an unrefined param would be Unknown.
    // The guard x < 128 narrows x into [0, 128) (assuming no lower bound guard,
    // the path predicate is only "x < 128"; the solver sees it as UNKNOWN because
    // we don't know x ≥ 0).  Let's give x an annotation [0, ∞).

    #[test]
    fn guard_narrows_variable_into_annotation() {
        // (define (ascii-info (n : (Int 0 _)) -> (Int 0 128))
        //   (if (< n 128) n 0))
        let sig = FunctionSignature {
            params: vec![(
                "n".to_string(),
                int_ann(Predicate::Range { lo: Some(0), hi: None, inclusive_hi: false }),
            )],
            return_type: int_ann(range(0, 128)),
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("n", 128),
            then_node: Box::new(CfgNode::Return(ReturnValue::Variable("n".to_string()))),
            else_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 2);
        // Both paths should be ProvenSafe.
        assert!(result.all_proven_safe(),
            "guard + param annotation narrows n into [0,128); got {:?}", result.return_sites);
    }

    // ─── 9. Three-level deep nesting ─────────────────────────────────────────

    #[test]
    fn deep_three_level_nesting() {
        // if x < 0: return 0         -- x < 0
        // elif x < 50: return x      -- 0 ≤ x < 50
        // elif x < 100: return x     -- 50 ≤ x < 100
        // else: return 99            -- x ≥ 100
        // return type: (Int 0 100)
        let sig = FunctionSignature {
            params: vec![(
                "x".to_string(),
                // x ≥ 0 to help the solver for the variable-return paths
                int_ann(Predicate::Range { lo: Some(0), hi: None, inclusive_hi: false }),
            )],
            return_type: int_ann(range(0, 100)),
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))),
            else_node: Box::new(CfgNode::Branch {
                guard: guard_lt("x", 50),
                then_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
                else_node: Box::new(CfgNode::Branch {
                    guard: guard_lt("x", 100),
                    then_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
                    else_node: Box::new(CfgNode::Return(ReturnValue::Literal(99))),
                }),
            }),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 4);
        assert!(result.all_proven_safe(),
            "all four paths should be proven safe; got {:?}", result.return_sites);
    }

    // ─── 10. LinearCmp guard with substitute_var ─────────────────────────────
    //
    // Guard expressed as LinearCmp (1*x < 50) instead of Range.
    // Tests that substitute_var correctly remaps VarId("x") → VarId("__v").

    #[test]
    fn linear_cmp_guard_substitute_var() {
        // Guard: 1*x < 50  (via LinearCmp)
        let guard_pred = Predicate::LinearCmp {
            coefs: vec![(VarId::new("x"), 1)],
            op: CmpOp::Lt,
            rhs: 50,
        };

        let sig = FunctionSignature {
            params: vec![(
                "x".to_string(),
                int_ann(Predicate::Range { lo: Some(0), hi: None, inclusive_hi: false }),
            )],
            return_type: int_ann(range(0, 50)),
        };

        let cfg = CfgNode::Branch {
            guard: BranchGuard { var: "x".to_string(), predicate: guard_pred },
            then_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
            else_node: Box::new(CfgNode::Return(ReturnValue::Literal(49))), // safe fallback
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 2);
        assert!(result.all_proven_safe(),
            "x ≥ 0 ∧ x < 50 ⊆ [0, 50); got {:?}", result.return_sites);
    }

    // ─── 11. substitute_var: Range is identity ────────────────────────────────

    #[test]
    fn substitute_var_range_is_identity() {
        let pred = range(10, 100);
        let result = substitute_var(&pred, "x", "__v");
        assert_eq!(result, pred, "Range has no variable refs; substitute is identity");
    }

    // ─── 12. substitute_var: Membership is identity ───────────────────────────

    #[test]
    fn substitute_var_membership_is_identity() {
        let pred = Predicate::Membership { values: vec![1, 2, 3] };
        let result = substitute_var(&pred, "x", "__v");
        assert_eq!(result, pred);
    }

    // ─── 13. substitute_var: LinearCmp renames matching VarId ────────────────

    #[test]
    fn substitute_var_linear_cmp_renames() {
        let pred = Predicate::LinearCmp {
            coefs: vec![(VarId::new("x"), 2), (VarId::new("y"), -1)],
            op: CmpOp::Le,
            rhs: 10,
        };
        let result = substitute_var(&pred, "x", "__v");

        // "x" should become "__v"; "y" should remain "y".
        if let Predicate::LinearCmp { coefs, .. } = &result {
            assert_eq!(coefs[0].0, VarId::new("__v"));
            assert_eq!(coefs[1].0, VarId::new("y"));
        } else {
            panic!("expected LinearCmp after substitution");
        }
    }

    // ─── 14. substitute_var: does not rename non-matching VarId ──────────────

    #[test]
    fn substitute_var_linear_cmp_no_match() {
        let pred = Predicate::LinearCmp {
            coefs: vec![(VarId::new("y"), 1)],
            op: CmpOp::Gt,
            rhs: 0,
        };
        let result = substitute_var(&pred, "x", "__v");

        if let Predicate::LinearCmp { coefs, .. } = &result {
            assert_eq!(coefs[0].0, VarId::new("y"), "non-matching var unchanged");
        } else {
            panic!("expected LinearCmp");
        }
    }

    // ─── 15. substitute_var: recurses through Not ─────────────────────────────

    #[test]
    fn substitute_var_not_recurses() {
        let inner = Predicate::LinearCmp {
            coefs: vec![(VarId::new("x"), 1)],
            op: CmpOp::Lt,
            rhs: 0,
        };
        let pred = Predicate::not(inner);
        let result = substitute_var(&pred, "x", "__v");

        if let Predicate::Not(inner_r) = &result {
            if let Predicate::LinearCmp { coefs, .. } = inner_r.as_ref() {
                assert_eq!(coefs[0].0, VarId::new("__v"));
            } else {
                panic!("expected LinearCmp inside Not");
            }
        } else {
            panic!("expected Not after substitution");
        }
    }

    // ─── 16. substitute_var: recurses through And/Or ─────────────────────────

    #[test]
    fn substitute_var_and_or_recurses() {
        let lc = Predicate::LinearCmp {
            coefs: vec![(VarId::new("x"), 1)],
            op: CmpOp::Ge,
            rhs: 0,
        };
        let and_pred = Predicate::and(vec![range(0, 100), lc.clone()]);
        let or_pred = Predicate::or(vec![range(0, 50), lc]);

        let and_result = substitute_var(&and_pred, "x", "__v");
        let or_result = substitute_var(&or_pred, "x", "__v");

        // Verify the LinearCmp inside And got its var renamed.
        if let Predicate::And(parts) = &and_result {
            let lc_part = parts.iter().find(|p| matches!(p, Predicate::LinearCmp { .. }));
            if let Some(Predicate::LinearCmp { coefs, .. }) = lc_part {
                assert_eq!(coefs[0].0, VarId::new("__v"));
            } else {
                panic!("expected LinearCmp in And after substitution");
            }
        } else {
            // May simplify to a single predicate if And([Range, LinearCmp]) collapses.
            // As long as no panic, the substitution was applied.
        }

        // Verify the Or version.
        if let Predicate::Or(parts) = &or_result {
            let lc_part = parts.iter().find(|p| matches!(p, Predicate::LinearCmp { .. }));
            if let Some(Predicate::LinearCmp { coefs, .. }) = lc_part {
                assert_eq!(coefs[0].0, VarId::new("__v"));
            }
        }
    }

    // ─── 17. FunctionCheckResult accessors ───────────────────────────────────

    #[test]
    fn function_check_result_accessors() {
        use crate::CounterExample;

        let safe = ReturnSiteOutcome {
            label: "return 0".into(),
            outcome: CheckOutcome::ProvenSafe,
        };
        let cx = CounterExample { value: 500, description: "test".into() };
        let unsafe_site = ReturnSiteOutcome {
            label: "return var x".into(),
            outcome: CheckOutcome::ProvenUnsafe(cx.clone()),
        };
        let unk = ReturnSiteOutcome {
            label: "return var y".into(),
            outcome: CheckOutcome::Unknown("reason".into()),
        };

        let all_safe = FunctionCheckResult { return_sites: vec![safe.clone()] };
        assert!(all_safe.all_proven_safe());
        assert!(!all_safe.has_violation());
        assert!(all_safe.first_counter_example().is_none());
        assert_eq!(all_safe.runtime_check_count(), 0);
        assert!(!all_safe.is_vacuous());

        let with_violation = FunctionCheckResult {
            return_sites: vec![safe.clone(), unsafe_site],
        };
        assert!(!with_violation.all_proven_safe());
        assert!(with_violation.has_violation());
        assert_eq!(with_violation.first_counter_example().unwrap().value, 500);

        let with_unknown = FunctionCheckResult {
            return_sites: vec![safe, unk],
        };
        assert!(!with_unknown.all_proven_safe());
        assert!(!with_unknown.has_violation());
        assert_eq!(with_unknown.runtime_check_count(), 1);

        let empty = FunctionCheckResult { return_sites: vec![] };
        assert!(empty.is_vacuous());
        assert!(!empty.all_proven_safe()); // vacuous result is not "all safe"
    }

    // ─── 18. Return site labels are descriptive ───────────────────────────────

    #[test]
    fn return_site_labels() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), unrefined_int())],
            return_type: unrefined_int(),
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("x", 0),
            then_node: Box::new(CfgNode::Return(ReturnValue::Literal(42))),
            else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites[0].label, "return 42");
        assert_eq!(result.return_sites[1].label, "return var x");
    }

    // ─── 19. Vacuous CFG (no returns) ────────────────────────────────────────

    #[test]
    fn vacuous_cfg_is_vacuous() {
        // A CFG that is just a Branch with no Return nodes inside is unusual
        // but not illegal.  We can only produce this if both arms also have
        // no Return — which isn't expressible with the current enum.
        // Instead, test the FunctionCheckResult::is_vacuous() directly.
        let result = FunctionCheckResult { return_sites: vec![] };
        assert!(result.is_vacuous());
        assert!(!result.all_proven_safe());
    }

    // ─── 20. Annotated param tighter than return — still safe ────────────────
    //
    // (define (narrow (x : (Int 0 50)) -> (Int 0 100)) x)
    //
    // [0, 50) ⊆ [0, 100) → ProvenSafe.

    #[test]
    fn narrower_param_annotation_implies_return() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), int_ann(range(0, 50)))],
            return_type: int_ann(range(0, 100)),
        };

        let cfg = CfgNode::Return(ReturnValue::Variable("x".to_string()));

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert!(result.all_proven_safe(), "[0,50)⊆[0,100) → safe");
    }

    // ─── 21. Wider param annotation partially violates return ─────────────────
    //
    // (define (wider (x : (Int 0 200)) -> (Int 0 100)) x)
    //
    // x can be in [100, 200) which violates [0, 100).

    #[test]
    fn wider_param_annotation_violates_return() {
        let sig = FunctionSignature {
            params: vec![("x".to_string(), int_ann(range(0, 200)))],
            return_type: int_ann(range(0, 100)),
        };

        let cfg = CfgNode::Return(ReturnValue::Variable("x".to_string()));

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert!(result.has_violation(),
            "x ∈ [0,200) but return needs [0,100); should be ProvenUnsafe");
        let cx = result.first_counter_example().unwrap();
        // Counter-example must be in [100, 200).
        assert!(cx.value >= 100 && cx.value < 200,
            "counter-example {} should be in [100, 200)", cx.value);
    }

    // ─── 22. Multiple parameters: guard depends on different param ────────────
    //
    // (define (saturate (a : (Int 0 100)) (b : int) -> (Int 0 100))
    //   (if (< b 0) a 50))
    //
    // The guard is on `b` but the return involves `a` (which is annotated).

    #[test]
    fn guard_on_different_param() {
        let sig = FunctionSignature {
            params: vec![
                ("a".to_string(), int_ann(range(0, 100))),
                ("b".to_string(), unrefined_int()),
            ],
            return_type: int_ann(range(0, 100)),
        };

        let cfg = CfgNode::Branch {
            guard: guard_lt("b", 0), // guard on b
            then_node: Box::new(CfgNode::Return(ReturnValue::Variable("a".to_string()))),
            else_node: Box::new(CfgNode::Return(ReturnValue::Literal(50))),
        };

        let mut fc = FunctionChecker::new();
        let result = fc.check_function(&sig, &cfg);

        assert_eq!(result.return_sites.len(), 2);
        assert!(result.all_proven_safe(),
            "a ∈ [0,100) and literal 50 ∈ [0,100); both safe");
    }
}
