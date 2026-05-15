//! # `lang-refinement-protocol` — LANG54
//!
//! Generic protocol for wiring the refinement-type solver into any language's
//! type checker.  Before LANG54, the 150-line glue between a type checker and
//! `lang-refinement-checker` had to be copy-pasted and adapted for every new
//! language.  LANG54 extracts that glue into two generic free functions backed
//! by a three-method trait.
//!
//! ## What Language X gets by implementing `RefinementBridge`
//!
//! | Feature | Code in Language X |
//! |---|---|
//! | Call-site proof obligations | `check_call_site_refinements(…)` |
//! | Flow-sensitive narrowing from `if` guards | `compute_if_narrowing(…)` |
//! | All solver infrastructure | provided by `lang-refinement-checker` |
//!
//! ## Architecture
//!
//! ```text
//!  ┌────────────────────────────────────────────────────────────┐
//!  │ Language X type checker                                    │
//!  │                                                            │
//!  │  impl RefinementBridge for LangXBridge {                  │
//!  │      type Expr = LangXExpr;                               │
//!  │      type Kind = LangXKind;                               │
//!  │      fn evidence_for(…)  → Evidence   // 5–10 lines       │
//!  │      fn narrowing_facts(…) → Vec<…>  // 10–30 lines       │
//!  │      fn narrow_kind(…)   → LangXKind // 5–10 lines        │
//!  │  }                                                         │
//!  │                                                            │
//!  │  infer_apply: check_call_site_refinements(&bridge, …)     │
//!  │  infer_if:    compute_if_narrowing(&bridge, …)            │
//!  └───────────────────────────┬────────────────────────────────┘
//!                              │ uses
//!  ┌───────────────────────────▼────────────────────────────────┐
//!  │ lang-refinement-protocol (this crate)                      │
//!  │   RefinementBridge trait                                   │
//!  │   check_call_site_refinements()  generic free fn           │
//!  │   compute_if_narrowing()         generic free fn           │
//!  └───────────────────────────┬────────────────────────────────┘
//!                              │ uses
//!  ┌───────────────────────────▼────────────────────────────────┐
//!  │ lang-refinement-checker                                    │
//!  │   Checker::check(annotation, evidence) → CheckOutcome     │
//!  └───────────────────────────┬────────────────────────────────┘
//!                              │ uses
//!  ┌───────────────────────────▼────────────────────────────────┐
//!  │ constraint-vm → constraint-engine → SAT / LIA tactics      │
//!  └────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Minimal example — implementing the bridge for a new language
//!
//! ```rust,ignore
//! use lang_refinement_protocol::{
//!     RefinementBridge, RefinementDiagnostic, RefinementMode,
//!     NarrowedBindings, Evidence, Predicate, RefinedType,
//!     check_call_site_refinements, compute_if_narrowing,
//! };
//!
//! // Language X's types.
//! enum LangXExpr { IntLit(i64), Var(String), Other }
//! #[derive(Clone)] enum LangXKind { Int, RefinedInt(Predicate), Bool }
//!
//! struct LangXBridge;
//!
//! impl RefinementBridge for LangXBridge {
//!     type Expr = LangXExpr;
//!     type Kind = LangXKind;
//!
//!     fn evidence_for(&self, expr: &LangXExpr, _kind: Option<&LangXKind>) -> Evidence {
//!         match expr {
//!             LangXExpr::IntLit(n) => Evidence::Concrete(*n as i128),
//!             _ => Evidence::Unconstrained,
//!         }
//!     }
//!
//!     fn narrowing_facts(&self, _guard: &LangXExpr) -> Vec<(String, Predicate)> {
//!         vec![]   // implement guard analysis here
//!     }
//!
//!     fn narrow_kind(&self, base: &LangXKind, pred: Predicate) -> LangXKind {
//!         match base {
//!             LangXKind::Int => LangXKind::RefinedInt(pred),
//!             LangXKind::RefinedInt(existing) => {
//!                 LangXKind::RefinedInt(Predicate::and(vec![existing.clone(), pred]))
//!             }
//!             other => other.clone(),
//!         }
//!     }
//! }
//! ```
//!
//! ## Quick start — call-site checking
//!
//! ```rust,ignore
//! // In your type checker's function-application handler:
//! let diags = check_call_site_refinements(
//!     &LangXBridge,
//!     callee_name,      // &str
//!     call_site_line,   // usize
//!     call_site_column, // usize
//!     &arg_exprs,       // &[LangXExpr]
//!     &arg_kinds,       // &[LangXKind]
//!     param_refinements, // &[Option<RefinedType>]
//!     RefinementMode::Strict,
//! );
//! for d in diags {
//!     your_error_list.push((d.message, d.line, d.column));
//! }
//! ```
//!
//! ## Quick start — flow-sensitive narrowing
//!
//! ```rust,ignore
//! // In your type checker's if-expression handler:
//! let narrowed = compute_if_narrowing(
//!     &LangXBridge,
//!     &guard_expr,
//!     |var| your_scope.lookup(var).cloned(),
//! );
//! // Apply true_branch bindings, infer then-body, restore.
//! // Apply false_branch bindings, infer else-body, restore.
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// ---------------------------------------------------------------------------
// Re-exports — Language X only needs this one dep in Cargo.toml
// ---------------------------------------------------------------------------

pub use lang_refined_types::{Kind as RefKind, Predicate, RefinedType};
pub use lang_refinement_checker::{
    check_all, CheckOutcome, Checker, CounterExample, Evidence, Obligation,
};

// ---------------------------------------------------------------------------
// RefinementMode — how Unknown outcomes are treated
// ---------------------------------------------------------------------------

/// Whether `Unknown` proof obligations produce diagnostics.
///
/// The solver returns `Unknown` when it cannot determine at compile time
/// whether an annotation holds (e.g., the argument comes from user input).
/// The configured mode decides whether that counts as an error.
///
/// ```text
/// Outcome        Lenient       Strict
/// ──────────────────────────────────
/// ProvenSafe     silent        silent
/// ProvenUnsafe   error         error
/// Unknown        silent        error
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefinementMode {
    /// `Unknown` outcomes are silent; only `ProvenUnsafe` is an error.
    ///
    /// Appropriate for early development where not every call site can be
    /// statically proven — the compiler warns but does not reject.
    Lenient,
    /// Both `Unknown` and `ProvenUnsafe` outcomes are errors.
    ///
    /// Appropriate for production builds and fully-annotated modules where
    /// every refinement obligation must be statically discharged.
    Strict,
}

// ---------------------------------------------------------------------------
// RefinementDiagnostic — an error/warning from refinement checking
// ---------------------------------------------------------------------------

/// A diagnostic produced by the refinement checker at a specific source location.
///
/// Carries enough information for the caller to synthesise a `TypeErrorDiagnostic`
/// or equivalent error struct in its own type system.
///
/// # Diagnostic messages
///
/// - `ProvenUnsafe`: `"refinement error: argument N to `f` violates annotation: …"`
/// - `Unknown` + `Strict`: `"refinement error: argument N to `f` cannot be proven to satisfy annotation (strict mode)"`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinementDiagnostic {
    /// Human-readable error message.  Suitable for display to the user.
    pub message: String,
    /// Source line of the call site (1-indexed).
    pub line: usize,
    /// Source column of the call site (1-indexed).
    pub column: usize,
}

// ---------------------------------------------------------------------------
// NarrowedBindings — output of compute_if_narrowing
// ---------------------------------------------------------------------------

/// The narrowed variable bindings for the two branches of an `if` expression.
///
/// After calling [`compute_if_narrowing`], the caller applies these bindings
/// to its scope before inferring each branch, then restores the scope after.
///
/// # Usage pattern
///
/// ```rust,ignore
/// let narrowed = compute_if_narrowing(&bridge, guard, |v| scope.lookup(v).cloned());
///
/// // True branch
/// scope.push_frame();
/// for (var, kind) in narrowed.true_branch { scope.bind(&var, kind); }
/// let then_kind = infer_expr(then_branch, …);
/// scope.pop_frame();
///
/// // False branch
/// scope.push_frame();
/// for (var, kind) in narrowed.false_branch { scope.bind(&var, kind); }
/// let else_kind = infer_expr(else_branch, …);
/// scope.pop_frame();
/// ```
#[derive(Debug, Clone)]
pub struct NarrowedBindings<K> {
    /// `(variable_name, narrowed_kind)` bindings for the true (then) branch.
    ///
    /// Each variable's kind is narrowed by the guard predicate — e.g., a
    /// variable `x : Int` narrows to `x : RefinedInt(x < 128)` inside the
    /// true branch of `(if (< x 128) …)`.
    pub true_branch: Vec<(String, K)>,

    /// `(variable_name, narrowed_kind)` bindings for the false (else) branch.
    ///
    /// Each variable's kind is narrowed by the **negation** of the guard
    /// predicate — e.g., `x` narrows to `RefinedInt(NOT(x < 128))` inside
    /// the false branch of `(if (< x 128) …)`.
    pub false_branch: Vec<(String, K)>,
}

// ---------------------------------------------------------------------------
// RefinementBridge — the three-method trait Language X implements
// ---------------------------------------------------------------------------

/// The bridge between a language's type checker and the generic refinement logic.
///
/// Implement this trait once for a language to get
/// [`check_call_site_refinements`] and [`compute_if_narrowing`] for free.
///
/// ## Type parameters
///
/// - `Expr` — the language's AST expression type.  Passed at call sites (for
///   evidence classification) and at guard positions (for narrowing facts).
/// - `Kind` — the language's kind/type representation returned by the type
///   checker.  Must implement `Clone` so narrowed bindings can be stored.
///
/// ## Implementing the three methods
///
/// ### `evidence_for`
///
/// Classify a call-site argument as `Evidence` for the refinement solver.
/// The three cases a typical implementation handles:
///
/// | Argument | Evidence |
/// |----------|---------|
/// | Integer literal `n` | `Concrete(n)` |
/// | Variable `x` with refined integer kind | `Predicated([predicate])` |
/// | Anything else | `Unconstrained` |
///
/// ### `narrowing_facts`
///
/// Analyse a guard expression and return `(variable, predicate)` pairs that
/// hold when the guard is true.  A conservative implementation returns `[]`
/// for guards it cannot analyse — the worst case is missed optimisation, not
/// unsoundness.
///
/// Typical patterns to handle:
/// - `(< x k)` → `[("x", Range { hi: Some(k) })]`
/// - `(>= x k)` → `[("x", Range { lo: Some(k) })]`
/// - `(= x k)` → `[("x", Range { lo: k, hi: k, inclusive_hi: true })]`
/// - `(and c1 c2)` → merge facts from `c1` and `c2`
/// - `(not c)` → negate facts from `c`
///
/// ### `narrow_kind`
///
/// Merge a base kind with a narrowing predicate, returning the narrowed kind.
/// Typical rules:
/// - `Int + p` → `RefinedInt(p)`
/// - `RefinedInt(q) + p` → `RefinedInt(and([q, p]))`
/// - Any non-numeric kind → return unchanged
pub trait RefinementBridge {
    /// The language's AST expression type at call sites and guard positions.
    type Expr;

    /// The language's kind/type representation (result of type inference).
    ///
    /// Must implement `Clone` so narrowed bindings can be collected into
    /// [`NarrowedBindings`].
    type Kind: Clone;

    /// Classify a call-site argument expression as `Evidence` for the solver.
    ///
    /// # Parameters
    ///
    /// - `expr` — the argument AST node from the call site.
    /// - `inferred_kind` — the kind the type checker already computed for
    ///   this expression.  Pass `None` if you have not pre-computed it;
    ///   the bridge may fall back to `Unconstrained` in that case.
    ///   Passing the pre-computed kind avoids a second traversal.
    fn evidence_for(&self, expr: &Self::Expr, inferred_kind: Option<&Self::Kind>) -> Evidence;

    /// Extract narrowing facts from a guard expression.
    ///
    /// Returns `(variable_name, predicate)` pairs meaning "if this guard
    /// expression evaluates to true, the named variable satisfies this
    /// predicate."
    ///
    /// Return an empty `Vec` for guards that cannot be analysed — conservative
    /// behaviour (no narrowing applied) is always safe.
    fn narrowing_facts(&self, guard: &Self::Expr) -> Vec<(String, Predicate)>;

    /// Narrow a variable's kind by adding a guard predicate.
    ///
    /// Called once for each `(variable, predicate)` pair returned by
    /// [`narrowing_facts`]:
    ///
    /// - For the **true branch**: called with the predicate as-is.
    /// - For the **false branch**: called with `Predicate::not(predicate)`.
    ///
    /// A typical implementation for a kind system with `Int` and
    /// `RefinedInt(Predicate)`:
    ///
    /// ```rust,ignore
    /// fn narrow_kind(&self, base: &MyKind, pred: Predicate) -> MyKind {
    ///     match base {
    ///         MyKind::Int => MyKind::RefinedInt(pred),
    ///         MyKind::RefinedInt(existing) => {
    ///             MyKind::RefinedInt(Predicate::and(vec![existing.clone(), pred]))
    ///         }
    ///         other => other.clone(),   // non-numeric: no narrowing
    ///     }
    /// }
    /// ```
    fn narrow_kind(&self, base: &Self::Kind, pred: Predicate) -> Self::Kind;
}

// ---------------------------------------------------------------------------
// check_call_site_refinements — generic call-site proof obligation checker
// ---------------------------------------------------------------------------

/// Check refinement annotations at a function call site.
///
/// This is the primary entry point for integrating the refinement solver into
/// a type checker's function-application handler.
///
/// ## Algorithm
///
/// For each `(arg_expr, arg_kind, maybe_refinement)` triple:
///
/// 1. If `maybe_refinement` is `None`, skip this argument (no annotation).
/// 2. Call `bridge.evidence_for(arg_expr, Some(arg_kind))` to classify
///    the argument as `Concrete`, `Predicated`, or `Unconstrained`.
/// 3. Run `Checker::check(refinement, evidence)`.
/// 4. Map the three outcomes to diagnostics based on `mode`:
///    - `ProvenSafe` → silent.
///    - `ProvenUnsafe(cx)` → `RefinementDiagnostic` always.
///    - `Unknown` + `Lenient` → silent.
///    - `Unknown` + `Strict` → `RefinementDiagnostic`.
///
/// ## Parameters
///
/// - `bridge` — the language-specific `RefinementBridge` implementation.
/// - `callee_name` — name of the function being called (used in error messages).
/// - `call_site_line` / `call_site_column` — source location of the call
///   (used in `RefinementDiagnostic`).
/// - `arg_exprs` — the argument AST nodes from the call site.
/// - `arg_kinds` — the inferred kinds for each argument (may be shorter than
///   `arg_exprs`; missing entries are treated as `None` → `Unconstrained`).
/// - `param_refinements` — the registered `RefinedType` per parameter (indexed
///   by parameter position).  `None` at position `i` means no annotation.
/// - `mode` — `Lenient` or `Strict` (controls handling of `Unknown`).
///
/// ## Returns
///
/// A `Vec<RefinementDiagnostic>` — empty if no violations are found.
///
/// # Example — wiring into `infer_apply`
///
/// ```rust,ignore
/// let diags = check_call_site_refinements(
///     &MyBridge,
///     callee_name,
///     app.line,
///     app.column,
///     &app.args,
///     &arg_kinds,
///     param_refinements,
///     mode.into(),
/// );
/// for d in diags {
///     errors.push(TypeErrorDiagnostic { message: d.message, line: d.line, column: d.column });
/// }
/// ```
pub fn check_call_site_refinements<B>(
    bridge: &B,
    callee_name: &str,
    call_site_line: usize,
    call_site_column: usize,
    arg_exprs: &[B::Expr],
    arg_kinds: &[B::Kind],
    param_refinements: &[Option<RefinedType>],
    mode: RefinementMode,
) -> Vec<RefinementDiagnostic>
where
    B: RefinementBridge,
{
    let mut checker = Checker::new();
    let mut diagnostics = Vec::new();

    // Walk over argument positions up to the length of param_refinements.
    // Extra arguments beyond param_refinements (shouldn't happen after arity
    // checking, but be defensive) are silently ignored.
    let n = param_refinements.len().min(arg_exprs.len());

    for i in 0..n {
        // Skip positions with no annotation — no obligation to discharge.
        let Some(rt) = &param_refinements[i] else {
            continue;
        };

        let arg_expr = &arg_exprs[i];
        let arg_kind = arg_kinds.get(i);

        // Classify the argument as Evidence.
        // `arg_kind` is passed so the bridge can check for `RefinedInt(p)` etc.
        // without doing a second scope lookup.
        let evidence = bridge.evidence_for(arg_expr, arg_kind);

        // Run the proof obligation.
        match checker.check(rt, &evidence) {
            CheckOutcome::ProvenSafe => {
                // Annotation holds for all values consistent with evidence.
                // Silent — this is the happy path.
            }

            CheckOutcome::ProvenUnsafe(cx) => {
                // Solver found a concrete counter-example violating the annotation.
                // This is a definite compile-time bug — error in all modes.
                diagnostics.push(RefinementDiagnostic {
                    message: format!(
                        "refinement error: argument {} to `{}` violates annotation: {}",
                        i, callee_name, cx.description
                    ),
                    line: call_site_line,
                    column: call_site_column,
                });
            }

            CheckOutcome::Unknown(_) if mode == RefinementMode::Strict => {
                // Solver could not determine the outcome and we are in strict
                // mode — treat as an error.
                diagnostics.push(RefinementDiagnostic {
                    message: format!(
                        "refinement error: argument {} to `{}` cannot be proven \
                         to satisfy annotation (strict mode)",
                        i, callee_name
                    ),
                    line: call_site_line,
                    column: call_site_column,
                });
            }

            CheckOutcome::Unknown(_) => {
                // Lenient mode: solver gave up, emit runtime check (caller's
                // responsibility), proceed silently at compile time.
            }
        }
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// compute_if_narrowing — generic flow-sensitive narrowing for if-expressions
// ---------------------------------------------------------------------------

/// Compute narrowed variable bindings for both branches of an `if` expression.
///
/// Call this in your type checker's `if`-expression handler to get the
/// narrowed kind for each variable in the true and false branches.
///
/// ## Algorithm
///
/// 1. Call `bridge.narrowing_facts(guard)` to extract `(variable, predicate)`
///    pairs implied by the guard being true.
/// 2. For each pair, call `scope_lookup(variable)` to find the variable's
///    current kind.  Variables not in scope are skipped.
/// 3. For the **true branch**: narrow each variable's kind via
///    `bridge.narrow_kind(current_kind, predicate)`.
/// 4. For the **false branch**: narrow via
///    `bridge.narrow_kind(current_kind, Predicate::not(predicate))`.
///
/// ## Parameters
///
/// - `bridge` — the language-specific `RefinementBridge` implementation.
/// - `guard` — the condition expression of the `if`.
/// - `scope_lookup` — a closure that looks up a variable's current kind in
///   the type checker's scope.  Return `None` for variables not in scope.
///
/// ## Returns
///
/// [`NarrowedBindings`] containing the `(variable_name, narrowed_kind)` pairs
/// for each branch.  Apply these to your scope *after* pushing a frame and
/// *before* inferring the branch body.  Restore by popping the frame.
///
/// ## Conservative behaviour
///
/// If `bridge.narrowing_facts(guard)` returns `[]` (the guard implies nothing
/// useful), both branches in the returned `NarrowedBindings` will be empty
/// and no narrowing is applied.  This is always safe: the branches are
/// checked with unnarrowed kinds.
///
/// # Example
///
/// ```rust,ignore
/// // In your if-expression handler:
/// let narrowed = compute_if_narrowing(
///     &MyBridge,
///     &if_expr.cond,
///     |var| scope.lookup(var).cloned().or_else(|| globals.get(var).cloned()),
/// );
///
/// // True branch
/// scope.push_frame();
/// for (var, kind) in narrowed.true_branch {
///     scope.bind(&var, kind);
/// }
/// let then_kind = infer_expr(&if_expr.then_branch, …);
/// scope.pop_frame();
///
/// // False branch
/// scope.push_frame();
/// for (var, kind) in narrowed.false_branch {
///     scope.bind(&var, kind);
/// }
/// let else_kind = infer_expr(&if_expr.else_branch, …);
/// scope.pop_frame();
/// ```
pub fn compute_if_narrowing<B, F>(
    bridge: &B,
    guard: &B::Expr,
    scope_lookup: F,
) -> NarrowedBindings<B::Kind>
where
    B: RefinementBridge,
    F: Fn(&str) -> Option<B::Kind>,
{
    let facts = bridge.narrowing_facts(guard);

    let mut true_branch = Vec::new();
    let mut false_branch = Vec::new();

    for (var, pred) in facts {
        // Only narrow variables that are currently in scope.
        // Variables not found are silently skipped — conservative is safe.
        let Some(base_kind) = scope_lookup(&var) else {
            continue;
        };

        // True branch: narrow by the guard predicate as-is.
        let narrowed_true = bridge.narrow_kind(&base_kind, pred.clone());
        true_branch.push((var.clone(), narrowed_true));

        // False branch: narrow by the logical negation of the guard predicate.
        // For example, a guard `(< x 128)` means `x < 128` is false in the
        // else branch, so we add `NOT(x < 128)` ≡ `x ≥ 128`.
        let negated_pred = Predicate::not(pred);
        let narrowed_false = bridge.narrow_kind(&base_kind, negated_pred);
        false_branch.push((var, narrowed_false));
    }

    NarrowedBindings { true_branch, false_branch }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lang_refined_types::{Kind as RefKind, Predicate, RefinedType};

    // ─── Mock bridge ─────────────────────────────────────────────────────────
    //
    // A minimal, self-contained bridge used only in tests.
    // MockExpr and MockKind are simple enough to cover all Evidence paths.

    /// Minimal expression type for tests.
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum MockExpr {
        /// An integer literal.
        IntLit(i64),
        /// A variable reference.
        Var(String),
        /// A simple `(< var literal)` guard.
        LtGuard { var: String, bound: i64 },
        /// A `(and guard1 guard2)` conjunction.
        AndGuard(Box<MockExpr>, Box<MockExpr>),
        /// Any other expression — yields no narrowing facts and Unconstrained evidence.
        Other,
    }

    /// Minimal kind type for tests.
    #[derive(Debug, Clone, PartialEq)]
    enum MockKind {
        Int,
        /// Integer with a refinement predicate.
        RefinedInt(Predicate),
        Bool,
    }

    /// The mock bridge:
    /// - `IntLit(n)` → `Concrete(n)`
    /// - `Var(_)` with `RefinedInt(p)` kind → `Predicated([p])`
    /// - anything else → `Unconstrained`
    /// - `LtGuard { var, bound }` → one narrowing fact: `var ∈ (-∞, bound)`
    /// - `AndGuard(l, r)` → merged facts from both children
    struct MockBridge;

    impl RefinementBridge for MockBridge {
        type Expr = MockExpr;
        type Kind = MockKind;

        fn evidence_for(&self, expr: &MockExpr, inferred_kind: Option<&MockKind>) -> Evidence {
            match expr {
                MockExpr::IntLit(n) => Evidence::Concrete(*n as i128),
                MockExpr::Var(_) => match inferred_kind {
                    Some(MockKind::RefinedInt(p)) => Evidence::Predicated(vec![p.clone()]),
                    _ => Evidence::Unconstrained,
                },
                _ => Evidence::Unconstrained,
            }
        }

        fn narrowing_facts(&self, guard: &MockExpr) -> Vec<(String, Predicate)> {
            match guard {
                MockExpr::LtGuard { var, bound } => {
                    // Guard `var < bound` → predicate: Range { hi: Some(bound), exclusive }
                    vec![(
                        var.clone(),
                        Predicate::Range {
                            lo: None,
                            hi: Some(*bound as i128),
                            inclusive_hi: false,
                        },
                    )]
                }
                MockExpr::AndGuard(l, r) => {
                    // Merge facts from both children.
                    let mut facts = self.narrowing_facts(l);
                    facts.extend(self.narrowing_facts(r));
                    facts
                }
                _ => vec![],
            }
        }

        fn narrow_kind(&self, base: &MockKind, pred: Predicate) -> MockKind {
            match base {
                MockKind::Int => MockKind::RefinedInt(pred),
                MockKind::RefinedInt(existing) => {
                    MockKind::RefinedInt(Predicate::and(vec![existing.clone(), pred]))
                }
                // Bool cannot be narrowed by integer predicates.
                other => other.clone(),
            }
        }
    }

    // ─── Helper — build a [0, 128) range annotation ──────────────────────────

    fn ascii_annotation() -> RefinedType {
        RefinedType::refined(
            RefKind::Int,
            Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
        )
    }

    // ─── check_call_site_refinements — concrete evidence ─────────────────────

    #[test]
    fn check_call_site_concrete_in_range_no_diagnostic() {
        // Literal 42 satisfies [0, 128).
        let diags = check_call_site_refinements(
            &MockBridge,
            "ascii-info",
            10,
            5,
            &[MockExpr::IntLit(42)],
            &[MockKind::Int],
            &[Some(ascii_annotation())],
            RefinementMode::Strict,
        );
        assert!(diags.is_empty(), "42 is in [0,128); expected no diagnostic");
    }

    #[test]
    fn check_call_site_concrete_out_of_range_is_diagnostic() {
        // Literal 200 violates [0, 128).
        let diags = check_call_site_refinements(
            &MockBridge,
            "ascii-info",
            10,
            5,
            &[MockExpr::IntLit(200)],
            &[MockKind::Int],
            &[Some(ascii_annotation())],
            RefinementMode::Strict,
        );
        assert_eq!(diags.len(), 1, "200 is outside [0,128); expected 1 diagnostic");
        assert!(diags[0].message.contains("ascii-info"));
        assert!(diags[0].message.contains("violates"));
        assert_eq!(diags[0].line, 10);
        assert_eq!(diags[0].column, 5);
    }

    // ─── check_call_site_refinements — unconstrained evidence ────────────────

    #[test]
    fn check_call_site_unconstrained_lenient_is_silent() {
        // Unknown source in lenient mode → no diagnostic.
        let diags = check_call_site_refinements(
            &MockBridge,
            "ascii-info",
            1, 1,
            &[MockExpr::Var("x".into())],
            &[MockKind::Int],   // Int (unrefined) → Unconstrained
            &[Some(ascii_annotation())],
            RefinementMode::Lenient,
        );
        assert!(diags.is_empty(), "unconstrained in lenient mode should be silent");
    }

    #[test]
    fn check_call_site_unconstrained_strict_is_diagnostic() {
        // Unknown source in strict mode → diagnostic.
        let diags = check_call_site_refinements(
            &MockBridge,
            "ascii-info",
            1, 1,
            &[MockExpr::Var("x".into())],
            &[MockKind::Int],   // Int → Unconstrained
            &[Some(ascii_annotation())],
            RefinementMode::Strict,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("strict mode"));
    }

    // ─── check_call_site_refinements — no annotation registered ─────────────

    #[test]
    fn check_call_site_no_annotation_is_silent() {
        // param_refinements[0] = None → no obligation, no diagnostic.
        let diags = check_call_site_refinements(
            &MockBridge,
            "f",
            1, 1,
            &[MockExpr::IntLit(200)],
            &[MockKind::Int],
            &[None],
            RefinementMode::Strict,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn check_call_site_empty_param_refinements_is_silent() {
        // No registered refinements for any param.
        let diags = check_call_site_refinements(
            &MockBridge,
            "g",
            1, 1,
            &[MockExpr::IntLit(999)],
            &[MockKind::Int],
            &[],  // no param_refinements at all
            RefinementMode::Strict,
        );
        assert!(diags.is_empty());
    }

    // ─── check_call_site_refinements — predicated evidence ──────────────────

    #[test]
    fn check_call_site_predicated_proven_safe_is_silent() {
        // Variable x has RefinedInt(x ∈ [0, 50)) → safe for [0, 128) annotation.
        let narrow_pred = Predicate::Range { lo: Some(0), hi: Some(50), inclusive_hi: false };
        let diags = check_call_site_refinements(
            &MockBridge,
            "ascii-info",
            5, 10,
            &[MockExpr::Var("x".into())],
            &[MockKind::RefinedInt(narrow_pred)],
            &[Some(ascii_annotation())],
            RefinementMode::Strict,
        );
        assert!(diags.is_empty(), "[0,50) ⊆ [0,128); expected no diagnostic");
    }

    #[test]
    fn check_call_site_predicated_proven_unsafe_is_diagnostic() {
        // Variable y has RefinedInt(y ∈ [100, 200)) — overlaps OUTSIDE [0,128).
        let wide_pred = Predicate::Range { lo: Some(100), hi: Some(200), inclusive_hi: false };
        let diags = check_call_site_refinements(
            &MockBridge,
            "ascii-info",
            7, 3,
            &[MockExpr::Var("y".into())],
            &[MockKind::RefinedInt(wide_pred)],
            &[Some(ascii_annotation())],
            RefinementMode::Lenient,
        );
        assert_eq!(diags.len(), 1, "y ∈ [100,200) can violate [0,128)");
    }

    // ─── compute_if_narrowing ─────────────────────────────────────────────────

    #[test]
    fn compute_if_narrowing_extracts_true_and_false_branches() {
        // Guard: `x < 128`.  Variable x is Int in scope.
        let guard = MockExpr::LtGuard { var: "x".into(), bound: 128 };
        let scope = |var: &str| {
            if var == "x" { Some(MockKind::Int) } else { None }
        };

        let nb = compute_if_narrowing(&MockBridge, &guard, scope);

        assert_eq!(nb.true_branch.len(), 1);
        assert_eq!(nb.true_branch[0].0, "x");
        // True branch: x narrowed to RefinedInt(x < 128).
        assert!(matches!(nb.true_branch[0].1, MockKind::RefinedInt(_)));

        assert_eq!(nb.false_branch.len(), 1);
        assert_eq!(nb.false_branch[0].0, "x");
        // False branch: x narrowed to RefinedInt(NOT(x < 128)).
        assert!(matches!(nb.false_branch[0].1, MockKind::RefinedInt(_)));
    }

    #[test]
    fn compute_if_narrowing_no_facts_yields_empty() {
        // Other expression → no narrowing facts.
        let guard = MockExpr::Other;
        let nb = compute_if_narrowing(&MockBridge, &guard, |_| Some(MockKind::Int));
        assert!(nb.true_branch.is_empty());
        assert!(nb.false_branch.is_empty());
    }

    #[test]
    fn compute_if_narrowing_variable_not_in_scope_is_skipped() {
        // Guard references `x`, but scope has no `x` → nothing narrowed.
        let guard = MockExpr::LtGuard { var: "x".into(), bound: 128 };
        let nb = compute_if_narrowing(&MockBridge, &guard, |_| None);
        assert!(nb.true_branch.is_empty());
        assert!(nb.false_branch.is_empty());
    }

    #[test]
    fn narrowed_bindings_false_branch_is_negated_predicate() {
        // Narrow `x : Int` with guard `x < 10`.
        // True branch: x ∈ (-∞, 10) [exclusive].
        // False branch: x ∈ NOT(-∞, 10) — some negated predicate.
        let guard = MockExpr::LtGuard { var: "x".into(), bound: 10 };
        let nb = compute_if_narrowing(&MockBridge, &guard, |var| {
            if var == "x" { Some(MockKind::Int) } else { None }
        });

        // True: RefinedInt with Range predicate.
        if let MockKind::RefinedInt(Predicate::Range { hi: Some(10), inclusive_hi: false, .. }) =
            &nb.true_branch[0].1
        {
            // ok
        } else {
            panic!("expected true branch to carry Range(hi=10) predicate; got {:?}", nb.true_branch[0].1);
        }

        // False: RefinedInt with Not(Range).
        assert!(matches!(&nb.false_branch[0].1, MockKind::RefinedInt(Predicate::Not(_))),
            "false branch should carry Not(…) predicate; got {:?}", nb.false_branch[0].1);
    }

    #[test]
    fn compute_if_narrowing_and_guard_produces_two_facts() {
        // Guard: (and (x < 128) (x < 64)) → two overlapping facts for x.
        // Both have the same variable, so two entries in true_branch.
        let guard = MockExpr::AndGuard(
            Box::new(MockExpr::LtGuard { var: "x".into(), bound: 128 }),
            Box::new(MockExpr::LtGuard { var: "x".into(), bound: 64 }),
        );
        let nb = compute_if_narrowing(&MockBridge, &guard, |var| {
            if var == "x" { Some(MockKind::Int) } else { None }
        });
        // Two facts → two entries (we don't merge same-variable facts in the protocol).
        assert_eq!(nb.true_branch.len(), 2);
        assert_eq!(nb.false_branch.len(), 2);
    }

    #[test]
    fn narrow_kind_bool_is_unchanged() {
        // Non-numeric kinds should pass through unchanged.
        let bridge = MockBridge;
        let pred = Predicate::Range { lo: Some(0), hi: Some(1), inclusive_hi: true };
        let narrowed = bridge.narrow_kind(&MockKind::Bool, pred);
        assert_eq!(narrowed, MockKind::Bool);
    }
}
