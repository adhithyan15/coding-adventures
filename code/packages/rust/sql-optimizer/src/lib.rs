//! # SQL Optimizer — Logical Query Plan Optimizer for Mini-SQLite (Level 1)
//!
//! This crate is the **fourth stage** of the Mini-SQLite SQL pipeline:
//!
//! ```text
//! sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
//! ```
//!
//! The optimizer accepts a [`LogicalPlan`] from `sql-planner` and applies
//! five optimization passes in sequence, producing an [`OptimizedPlan`] tree.
//!
//! ## Why optimize?
//!
//! The planner produces a *correct* tree that mirrors SQL's logical evaluation
//! order. It does not try to make the tree *efficient*. The optimizer's job is
//! to make efficiency improvements that are:
//!
//! - **Semantically equivalent**: the optimized plan returns the same rows as
//!   the original plan for every possible database state.
//! - **Statically computable**: the optimizer works purely on the plan tree
//!   (no data access at this stage), so only transformations that are correct
//!   for all inputs are applied.
//!
//! ## The five passes (applied in order)
//!
//! ```text
//! 1. ConstantFolding   — evaluate constant sub-expressions at plan time
//! 2. PredicatePushdown — move Filter nodes closer to the Scan leaves
//! 3. ProjectionPruning — annotate Scans with only the columns they need
//! 4. DeadCodeElimination — replace provably-empty subtrees with EmptyResult
//! 5. LimitPushdown     — attach scan_limit hints to Scans
//! ```
//!
//! Each pass is a separate type implementing the [`Pass`] trait. Passes are
//! applied in sequence by [`optimize`] (which uses [`default_passes`]).
//! You can supply a custom pass list via [`optimize_with_passes`].
//!
//! ## Public API
//!
//! ```rust
//! use coding_adventures_sql_optimizer::{optimize, OptimizedPlan};
//! use coding_adventures_sql_planner::LogicalPlan;
//!
//! // Lift a planner output into an optimized plan:
//! let plan = LogicalPlan::Scan { table: "users".into(), alias: None };
//! let opt: OptimizedPlan = optimize(plan);
//! // → OptimizedPlan::Scan { table: "users", alias: None,
//! //                          required_columns: None, scan_limit: None }
//! ```

use coding_adventures_sql_backend::ColumnDef;
use coding_adventures_sql_planner::{
    AggregateItem, Assignment, BinaryOp, InsertSource, JoinKind, LogicalPlan, OutputColumn,
    SortKey, SqlExpr, UnaryOp,
};
use std::collections::HashSet;

// ===========================================================================
// OptimizedPlan — mirrors LogicalPlan with extra optimization annotations
// ===========================================================================

/// A fully-optimized query plan node.
///
/// `OptimizedPlan` mirrors [`LogicalPlan`] almost exactly, with two differences:
///
/// 1. `Scan` carries two optional annotation fields that optimization passes
///    populate:
///    - `required_columns`: the subset of columns the rest of the plan needs
///      (set by [`ProjectionPruning`]).
///    - `scan_limit`: how many raw rows the backend needs to read at most
///      (set by [`LimitPushdown`]).
///
/// 2. An extra `EmptyResult` variant is added to represent a provably-empty
///    subtree (set by [`DeadCodeElimination`]).
///
/// All other variants carry `OptimizedPlan` children instead of `LogicalPlan`
/// children, so the type is compositionally recursive after the initial
/// [`lift`] conversion.
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizedPlan {
    /// A full table scan with optional optimization annotations.
    ///
    /// The two new fields (`required_columns`, `scan_limit`) start as `None`
    /// (no restriction) and are populated by later passes.
    Scan {
        table: String,
        alias: Option<String>,
        /// `None` = read all columns; `Some(cols)` = only these columns needed.
        required_columns: Option<Vec<String>>,
        /// `None` = no row limit; `Some(n)` = stop after n rows.
        scan_limit: Option<i64>,
    },

    /// A row filter — keeps rows where `predicate` is true.
    Filter {
        input: Box<OptimizedPlan>,
        predicate: SqlExpr,
    },

    /// A column projection — selects and renames columns from input.
    Project {
        input: Box<OptimizedPlan>,
        columns: Vec<OutputColumn>,
    },

    /// A join of two inputs.
    Join {
        left: Box<OptimizedPlan>,
        right: Box<OptimizedPlan>,
        kind: JoinKind,
        condition: Option<SqlExpr>,
    },

    /// Aggregation: GROUP BY + aggregate functions.
    Aggregate {
        input: Box<OptimizedPlan>,
        group_by: Vec<SqlExpr>,
        aggregates: Vec<AggregateItem>,
    },

    /// HAVING predicate applied after grouping.
    Having {
        input: Box<OptimizedPlan>,
        predicate: SqlExpr,
    },

    /// ORDER BY sort.
    Sort {
        input: Box<OptimizedPlan>,
        keys: Vec<SortKey>,
    },

    /// LIMIT / OFFSET pagination.
    Limit {
        input: Box<OptimizedPlan>,
        count: Option<i64>,
        offset: Option<i64>,
    },

    /// SELECT DISTINCT deduplication.
    Distinct(Box<OptimizedPlan>),

    /// UNION [ALL].
    Union {
        left: Box<OptimizedPlan>,
        right: Box<OptimizedPlan>,
        all: bool,
    },

    /// INSERT statement — passed through unchanged.
    Insert {
        table: String,
        columns: Option<Vec<String>>,
        source: InsertSource,
    },

    /// UPDATE statement — passed through unchanged.
    Update {
        table: String,
        assignments: Vec<Assignment>,
        predicate: Option<SqlExpr>,
    },

    /// DELETE statement — passed through unchanged.
    Delete {
        table: String,
        predicate: Option<SqlExpr>,
    },

    /// CREATE TABLE statement — passed through unchanged.
    CreateTable {
        table: String,
        if_not_exists: bool,
        columns: Vec<ColumnDef>,
    },

    /// DROP TABLE statement — passed through unchanged.
    DropTable {
        table: String,
        if_exists: bool,
    },

    /// A provably-empty subtree (added by [`DeadCodeElimination`]).
    ///
    /// Any plan that can never return rows — because it has a FALSE predicate,
    /// a LIMIT 0, or an INNER JOIN with an empty input — is replaced by this
    /// variant. Downstream stages (codegen, VM) can special-case it to avoid
    /// pointless work.
    ///
    /// ## Example
    ///
    /// ```text
    /// Filter { predicate: FALSE, input: Scan("users") }
    /// → EmptyResult
    /// ```
    EmptyResult,
}

// ===========================================================================
// Pass trait
// ===========================================================================

/// A single optimization pass over an [`OptimizedPlan`] tree.
///
/// Passes are applied sequentially by [`optimize_with_passes`]. Each pass
/// sees the output of all previous passes, so later passes can assume that
/// earlier ones have already run (e.g., `DeadCodeElimination` assumes
/// `ConstantFolding` has already turned `1 = 2` into `FALSE`).
///
/// ## Implementing a Pass
///
/// The simplest implementation is a unit struct with an `apply` that walks
/// the plan tree recursively:
///
/// ```rust
/// use coding_adventures_sql_optimizer::{Pass, OptimizedPlan};
///
/// struct MyPass;
///
/// impl Pass for MyPass {
///     fn name(&self) -> &str { "MyPass" }
///     fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan { plan }
/// }
/// ```
pub trait Pass {
    /// A human-readable name for this pass (used for debugging / logging).
    fn name(&self) -> &str;

    /// Apply this pass to a plan tree, returning the (potentially rewritten) tree.
    ///
    /// The pass should be a total function — it must handle every variant of
    /// `OptimizedPlan` (at least by returning the node unchanged).
    fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan;
}

// ===========================================================================
// Public API
// ===========================================================================

/// Optimize a [`LogicalPlan`] using the default five-pass pipeline.
///
/// This is the main entry point for the optimizer. It:
/// 1. Lifts the `LogicalPlan` into an `OptimizedPlan` (assigning `None` to
///    the two new annotation fields on `Scan`).
/// 2. Applies the five passes from [`default_passes`] in order.
///
/// # Example
///
/// ```rust
/// use coding_adventures_sql_optimizer::optimize;
/// use coding_adventures_sql_planner::LogicalPlan;
///
/// let plan = LogicalPlan::Scan { table: "t".into(), alias: None };
/// let opt = optimize(plan);
/// ```
pub fn optimize(plan: LogicalPlan) -> OptimizedPlan {
    let passes = default_passes();
    let pass_refs: Vec<&dyn Pass> = passes.iter().map(|p| p.as_ref()).collect();
    optimize_with_passes(plan, &pass_refs)
}

/// Optimize a [`LogicalPlan`] using a caller-supplied list of passes.
///
/// Passes are applied in order; each receives the output of the previous.
/// An empty slice returns the plan after the initial lift (no optimization).
///
/// This function is primarily useful for testing individual passes in isolation:
///
/// ```rust
/// use coding_adventures_sql_optimizer::{optimize_with_passes, ConstantFoldingPass};
///
/// let pass = ConstantFoldingPass;
/// use coding_adventures_sql_planner::LogicalPlan;
/// let plan = LogicalPlan::Scan { table: "t".into(), alias: None };
/// let opt = optimize_with_passes(plan, &[&pass]);
/// ```
pub fn optimize_with_passes(plan: LogicalPlan, passes: &[&dyn Pass]) -> OptimizedPlan {
    let mut current = lift(plan);
    for pass in passes {
        current = pass.apply(current);
    }
    current
}

/// Return the default five-pass pipeline in the order they should be applied.
///
/// ## Pass ordering rationale
///
/// The order matters because later passes consume the results of earlier ones:
///
/// 1. **ConstantFolding** first — it simplifies predicates so that later passes
///    can recognize `FALSE`/`TRUE` predicates and `NULL IS NULL` patterns.
///    Without it, `DeadCodeElimination` could not detect `Filter(1=2)` as dead.
///
/// 2. **PredicatePushdown** second — it moves filters downward. Running after
///    constant folding means it sees simplified predicates (e.g. `FALSE` won't
///    be pushed — DCE handles those).
///
/// 3. **ProjectionPruning** third — once filters are in their final positions,
///    we know which columns each predicate needs. Pruning after pushdown gives
///    a more accurate picture of the needed column set.
///
/// 4. **DeadCodeElimination** fourth — removes nodes that produce no rows.
///    Must run after constant folding (to detect `FALSE` predicates) and after
///    pushdown (to catch `Limit(0)` pushed near the scan).
///
/// 5. **LimitPushdown** last — annotates scans with row-count hints. Running
///    last means we don't push limits into subtrees that DCE might eliminate.
pub fn default_passes() -> Vec<Box<dyn Pass>> {
    vec![
        Box::new(ConstantFoldingPass),
        Box::new(PredicatePushdownPass),
        Box::new(ProjectionPruningPass),
        Box::new(DeadCodeEliminationPass),
        Box::new(LimitPushdownPass),
    ]
}

// ===========================================================================
// Lift: LogicalPlan → OptimizedPlan
// ===========================================================================

/// Convert a [`LogicalPlan`] into an [`OptimizedPlan`], recursively.
///
/// This is a purely structural conversion — it does not perform any
/// optimization. The two new `Scan` fields (`required_columns`, `scan_limit`)
/// are initialized to `None`.
///
/// ## Why lift separately?
///
/// Separating the type conversion from the optimization lets each pass work
/// entirely within the `OptimizedPlan` type. A pass that sees a `Scan` can
/// confidently inspect and update `required_columns` / `scan_limit` without
/// worrying about whether the input is a raw `LogicalPlan` or an already-
/// partially-optimized tree.
fn lift(plan: LogicalPlan) -> OptimizedPlan {
    match plan {
        LogicalPlan::Scan { table, alias } => OptimizedPlan::Scan {
            table,
            alias,
            required_columns: None,
            scan_limit: None,
        },
        LogicalPlan::Filter { input, predicate } => OptimizedPlan::Filter {
            input: Box::new(lift(*input)),
            predicate,
        },
        LogicalPlan::Project { input, columns } => OptimizedPlan::Project {
            input: Box::new(lift(*input)),
            columns,
        },
        LogicalPlan::Join {
            left,
            right,
            kind,
            condition,
        } => OptimizedPlan::Join {
            left: Box::new(lift(*left)),
            right: Box::new(lift(*right)),
            kind,
            condition,
        },
        LogicalPlan::Aggregate {
            input,
            group_by,
            aggregates,
        } => OptimizedPlan::Aggregate {
            input: Box::new(lift(*input)),
            group_by,
            aggregates,
        },
        LogicalPlan::Having { input, predicate } => OptimizedPlan::Having {
            input: Box::new(lift(*input)),
            predicate,
        },
        LogicalPlan::Sort { input, keys } => OptimizedPlan::Sort {
            input: Box::new(lift(*input)),
            keys,
        },
        LogicalPlan::Limit {
            input,
            count,
            offset,
        } => OptimizedPlan::Limit {
            input: Box::new(lift(*input)),
            count,
            offset,
        },
        LogicalPlan::Distinct(inner) => OptimizedPlan::Distinct(Box::new(lift(*inner))),
        LogicalPlan::Union { left, right, all } => OptimizedPlan::Union {
            left: Box::new(lift(*left)),
            right: Box::new(lift(*right)),
            all,
        },
        // DML/DDL: pass through as-is (no optimization applicable).
        LogicalPlan::Insert {
            table,
            columns,
            source,
        } => OptimizedPlan::Insert {
            table,
            columns,
            source,
        },
        LogicalPlan::Update {
            table,
            assignments,
            predicate,
        } => OptimizedPlan::Update {
            table,
            assignments,
            predicate,
        },
        LogicalPlan::Delete { table, predicate } => OptimizedPlan::Delete { table, predicate },
        LogicalPlan::CreateTable {
            table,
            if_not_exists,
            columns,
        } => OptimizedPlan::CreateTable {
            table,
            if_not_exists,
            columns,
        },
        LogicalPlan::DropTable { table, if_exists } => OptimizedPlan::DropTable { table, if_exists },
    }
}

// ===========================================================================
// Pass 1: ConstantFolding
// ===========================================================================

/// Evaluate constant sub-expressions at plan time.
///
/// ## Why fold?
///
/// Every row the VM processes pays the cost of evaluating constant
/// sub-expressions like `1 + 1`. Folding them once in the optimizer saves
/// that cost N times for an N-row table. It also enables downstream passes:
/// `Filter(1 = 2)` only becomes `Filter(FALSE)` after folding, which only
/// then lets `DeadCodeElimination` remove the subtree.
///
/// ## Traversal
///
/// Bottom-up, post-order: we fold a node's children before the node itself.
/// This means `1 + (2 + 3)` folds `2 + 3 → 5` first, then `1 + 5 → 6`.
///
/// ## NULL semantics
///
/// SQL uses three-valued logic. Most operators propagate NULL:
/// - `NULL + x` → `NULL`
/// - `NULL = NULL` → `NULL`
/// - `NULL * 0` → `NULL`
///
/// Boolean operators are partial exceptions due to short-circuiting:
/// - `TRUE OR anything` → `TRUE` (even if `anything` is NULL)
/// - `FALSE AND anything` → `FALSE` (even if `anything` is NULL)
///
/// ## What we do NOT fold
///
/// - Division by zero (`x / 0`) — leave for the VM to raise with a proper
///   runtime error and source position.
/// - Expressions containing Column or Aggregate references — those aren't
///   constants (their values depend on the row being processed).
/// - FunctionCall nodes — scalar functions are backend-defined.
pub struct ConstantFoldingPass;

impl Pass for ConstantFoldingPass {
    fn name(&self) -> &str {
        "ConstantFolding"
    }

    fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan {
        fold_plan(plan)
    }
}

/// Walk an `OptimizedPlan` recursively, folding constant expressions.
fn fold_plan(plan: OptimizedPlan) -> OptimizedPlan {
    match plan {
        // Scan is a leaf — no expressions inside the node itself to fold.
        OptimizedPlan::Scan { .. } => plan,

        // EmptyResult is a leaf — nothing to fold.
        OptimizedPlan::EmptyResult => plan,

        OptimizedPlan::Filter { input, predicate } => OptimizedPlan::Filter {
            input: Box::new(fold_plan(*input)),
            predicate: fold_expr(predicate),
        },

        OptimizedPlan::Project { input, columns } => OptimizedPlan::Project {
            input: Box::new(fold_plan(*input)),
            // Fold expressions inside each projection item.
            columns: columns
                .into_iter()
                .map(|col| OutputColumn {
                    expr: fold_expr(col.expr),
                    alias: col.alias,
                })
                .collect(),
        },

        OptimizedPlan::Join {
            left,
            right,
            kind,
            condition,
        } => OptimizedPlan::Join {
            left: Box::new(fold_plan(*left)),
            right: Box::new(fold_plan(*right)),
            kind,
            condition: condition.map(fold_expr),
        },

        OptimizedPlan::Aggregate {
            input,
            group_by,
            aggregates,
        } => OptimizedPlan::Aggregate {
            input: Box::new(fold_plan(*input)),
            group_by: group_by.into_iter().map(fold_expr).collect(),
            aggregates,
        },

        OptimizedPlan::Having { input, predicate } => OptimizedPlan::Having {
            input: Box::new(fold_plan(*input)),
            predicate: fold_expr(predicate),
        },

        OptimizedPlan::Sort { input, keys } => OptimizedPlan::Sort {
            input: Box::new(fold_plan(*input)),
            keys: keys
                .into_iter()
                .map(|k| SortKey {
                    expr: fold_expr(k.expr),
                    ascending: k.ascending,
                    nulls_first: k.nulls_first,
                })
                .collect(),
        },

        OptimizedPlan::Limit {
            input,
            count,
            offset,
        } => OptimizedPlan::Limit {
            input: Box::new(fold_plan(*input)),
            count,
            offset,
        },

        OptimizedPlan::Distinct(inner) => {
            OptimizedPlan::Distinct(Box::new(fold_plan(*inner)))
        }

        OptimizedPlan::Union { left, right, all } => OptimizedPlan::Union {
            left: Box::new(fold_plan(*left)),
            right: Box::new(fold_plan(*right)),
            all,
        },

        // DML/DDL: pass through unchanged (no expression folding needed for
        // the optimizer at this level; the VM handles DML expressions itself).
        plan @ (OptimizedPlan::Insert { .. }
        | OptimizedPlan::Update { .. }
        | OptimizedPlan::Delete { .. }
        | OptimizedPlan::CreateTable { .. }
        | OptimizedPlan::DropTable { .. }) => plan,
    }
}

/// Fold constant sub-expressions within a single [`SqlExpr`].
///
/// Post-order: we fold children first, then the parent.
///
/// ## SQL value semantics
///
/// | Input                        | Output             |
/// |------------------------------|--------------------|
/// | `Literal(x)` (already const)| unchanged          |
/// | `Column { .. }`             | unchanged          |
/// | `1 + 1`                     | `Literal(Int(2))`  |
/// | `NULL IS NULL`               | `Literal(Bool(true))`|
/// | `x AND FALSE`               | `Literal(Bool(false))`|
/// | `x OR TRUE`                 | `Literal(Bool(true))`|
fn fold_expr(expr: SqlExpr) -> SqlExpr {
    use coding_adventures_sql_backend::SqlValue;
    use coding_adventures_sql_planner::SqlExpr::*;

    match expr {
        // Leaves — already fully reduced.
        Literal(_) | Column { .. } => expr,

        // Aggregate and function calls are not folded (not constant).
        Aggregate { .. } | FunctionCall { .. } => expr,

        BinaryOp { op, left, right } => {
            let left = fold_expr(*left);
            let right = fold_expr(*right);
            fold_binary(op, left, right)
        }

        UnaryOp { op, expr: inner } => {
            let inner = fold_expr(*inner);
            fold_unary(op, inner)
        }

        // CAST: fold the operand, but keep the conversion (a cast of a constant
        // is still evaluated by the VM — we don't constant-fold the coercion).
        Cast { expr: inner, ty } => Cast {
            expr: Box::new(fold_expr(*inner)),
            ty,
        },

        // CASE: fold each condition/value and the ELSE, but keep the branch
        // structure (short-circuit semantics are the VM's job, not the folder's).
        Case { branches, else_val } => Case {
            branches: branches
                .into_iter()
                .map(|(cond, val)| (fold_expr(cond), fold_expr(val)))
                .collect(),
            else_val: else_val.map(|e| Box::new(fold_expr(*e))),
        },

        IsNull(inner) => {
            let inner = fold_expr(*inner);
            match &inner {
                Literal(SqlValue::Null) => Literal(SqlValue::Bool(true)),
                Literal(_) => Literal(SqlValue::Bool(false)),
                _ => IsNull(Box::new(inner)),
            }
        }

        IsNotNull(inner) => {
            let inner = fold_expr(*inner);
            match &inner {
                Literal(SqlValue::Null) => Literal(SqlValue::Bool(false)),
                Literal(_) => Literal(SqlValue::Bool(true)),
                _ => IsNotNull(Box::new(inner)),
            }
        }

        Between {
            value,
            low,
            high,
            negated,
        } => Between {
            value: Box::new(fold_expr(*value)),
            low: Box::new(fold_expr(*low)),
            high: Box::new(fold_expr(*high)),
            negated,
        },

        Like {
            value,
            pattern,
            negated,
        } => Like {
            value: Box::new(fold_expr(*value)),
            pattern: Box::new(fold_expr(*pattern)),
            negated,
        },

        InList {
            value,
            list,
            negated,
        } => InList {
            value: Box::new(fold_expr(*value)),
            list: list.into_iter().map(fold_expr).collect(),
            negated,
        },
    }
}

/// Fold a binary expression where both children have already been folded.
///
/// ## Short-circuit rules (SQL three-valued logic)
///
/// AND truth table:
/// ```text
/// AND   | TRUE  | FALSE | NULL
/// TRUE  | TRUE  | FALSE | NULL
/// FALSE | FALSE | FALSE | FALSE   ← FALSE dominates
/// NULL  | NULL  | FALSE | NULL
/// ```
///
/// OR truth table:
/// ```text
/// OR    | TRUE  | FALSE | NULL
/// TRUE  | TRUE  | TRUE  | TRUE    ← TRUE dominates
/// FALSE | TRUE  | FALSE | NULL
/// NULL  | TRUE  | NULL  | NULL
/// ```
fn fold_binary(op: BinaryOp, left: SqlExpr, right: SqlExpr) -> SqlExpr {
    use coding_adventures_sql_backend::SqlValue;
    use coding_adventures_sql_planner::SqlExpr::Literal;

    // --- Boolean short-circuit rules ---
    // These fire even when only ONE side is a literal.

    if op == BinaryOp::And {
        // FALSE AND anything → FALSE
        if matches!(&left, Literal(SqlValue::Bool(false))) {
            return Literal(SqlValue::Bool(false));
        }
        if matches!(&right, Literal(SqlValue::Bool(false))) {
            return Literal(SqlValue::Bool(false));
        }
        // TRUE AND x → x
        if matches!(&left, Literal(SqlValue::Bool(true))) {
            return right;
        }
        if matches!(&right, Literal(SqlValue::Bool(true))) {
            return left;
        }
        // NULL AND NULL → NULL
        if matches!(&left, Literal(SqlValue::Null))
            && matches!(&right, Literal(SqlValue::Null))
        {
            return Literal(SqlValue::Null);
        }
    }

    if op == BinaryOp::Or {
        // TRUE OR anything → TRUE
        if matches!(&left, Literal(SqlValue::Bool(true))) {
            return Literal(SqlValue::Bool(true));
        }
        if matches!(&right, Literal(SqlValue::Bool(true))) {
            return Literal(SqlValue::Bool(true));
        }
        // FALSE OR x → x
        if matches!(&left, Literal(SqlValue::Bool(false))) {
            return right;
        }
        if matches!(&right, Literal(SqlValue::Bool(false))) {
            return left;
        }
        // NULL OR NULL → NULL
        if matches!(&left, Literal(SqlValue::Null))
            && matches!(&right, Literal(SqlValue::Null))
        {
            return Literal(SqlValue::Null);
        }
    }

    // --- Full constant folding (both sides must be literals) ---
    let (lv, rv) = match (&left, &right) {
        (Literal(l), Literal(r)) => (l.clone(), r.clone()),
        _ => {
            return SqlExpr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
        }
    };

    // NULL propagation for non-boolean ops.
    if (lv == SqlValue::Null || rv == SqlValue::Null)
        && op != BinaryOp::And
        && op != BinaryOp::Or
    {
        return Literal(SqlValue::Null);
    }

    // Evaluate the operation.
    let result = apply_binary_op(&op, &lv, &rv);
    match result {
        Some(val) => Literal(val),
        None => SqlExpr::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

/// Apply a binary operator to two concrete SQL values.
///
/// Returns `None` if the operation cannot be evaluated at compile time
/// (e.g., division by zero — we leave those for the VM).
///
/// ## Arithmetic on SQL integer types
///
/// SQL integers are 64-bit signed. We use `i64::checked_*` operations to
/// avoid silent overflow and fall through to `None` on overflow, which
/// preserves the expression for the VM to evaluate with proper error handling.
fn apply_binary_op(op: &BinaryOp, lv: &coding_adventures_sql_backend::SqlValue, rv: &coding_adventures_sql_backend::SqlValue) -> Option<coding_adventures_sql_backend::SqlValue> {
    use coding_adventures_sql_backend::SqlValue;

    match (op, lv, rv) {
        // Arithmetic — integer × integer
        (BinaryOp::Add, SqlValue::Int(a), SqlValue::Int(b)) => {
            Some(SqlValue::Int(a.checked_add(*b)?))
        }
        (BinaryOp::Sub, SqlValue::Int(a), SqlValue::Int(b)) => {
            Some(SqlValue::Int(a.checked_sub(*b)?))
        }
        (BinaryOp::Mul, SqlValue::Int(a), SqlValue::Int(b)) => {
            Some(SqlValue::Int(a.checked_mul(*b)?))
        }
        (BinaryOp::Div, SqlValue::Int(a), SqlValue::Int(b)) => {
            // Division by zero → leave for VM (returns NULL in SQLite).
            if *b == 0 {
                return None;
            }
            Some(SqlValue::Int(a.checked_div(*b)?))
        }
        (BinaryOp::Mod, SqlValue::Int(a), SqlValue::Int(b)) => {
            if *b == 0 {
                return None;
            }
            Some(SqlValue::Int(a.checked_rem(*b)?))
        }

        // Arithmetic — float × float
        (BinaryOp::Add, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Float(a + b))
        }
        (BinaryOp::Sub, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Float(a - b))
        }
        (BinaryOp::Mul, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Float(a * b))
        }
        (BinaryOp::Div, SqlValue::Float(a), SqlValue::Float(b)) => {
            if *b == 0.0 {
                return None;
            }
            Some(SqlValue::Float(a / b))
        }

        // Arithmetic — int × float (promote int to float)
        (BinaryOp::Add, SqlValue::Int(a), SqlValue::Float(b)) => {
            Some(SqlValue::Float(*a as f64 + b))
        }
        (BinaryOp::Add, SqlValue::Float(a), SqlValue::Int(b)) => {
            Some(SqlValue::Float(a + *b as f64))
        }
        (BinaryOp::Sub, SqlValue::Int(a), SqlValue::Float(b)) => {
            Some(SqlValue::Float(*a as f64 - b))
        }
        (BinaryOp::Sub, SqlValue::Float(a), SqlValue::Int(b)) => {
            Some(SqlValue::Float(a - *b as f64))
        }
        (BinaryOp::Mul, SqlValue::Int(a), SqlValue::Float(b)) => {
            Some(SqlValue::Float(*a as f64 * b))
        }
        (BinaryOp::Mul, SqlValue::Float(a), SqlValue::Int(b)) => {
            Some(SqlValue::Float(a * *b as f64))
        }

        // String concatenation
        (BinaryOp::Concat, SqlValue::Text(a), SqlValue::Text(b)) => {
            Some(SqlValue::Text(format!("{}{}", a, b)))
        }

        // Equality comparisons — work for all types.
        (BinaryOp::Eq, a, b) => Some(SqlValue::Bool(a == b)),
        (BinaryOp::Neq, a, b) => Some(SqlValue::Bool(a != b)),

        // Ordered comparisons — only for types with a natural ordering.
        (BinaryOp::Lt, SqlValue::Int(a), SqlValue::Int(b)) => Some(SqlValue::Bool(a < b)),
        (BinaryOp::Lte, SqlValue::Int(a), SqlValue::Int(b)) => Some(SqlValue::Bool(a <= b)),
        (BinaryOp::Gt, SqlValue::Int(a), SqlValue::Int(b)) => Some(SqlValue::Bool(a > b)),
        (BinaryOp::Gte, SqlValue::Int(a), SqlValue::Int(b)) => Some(SqlValue::Bool(a >= b)),
        (BinaryOp::Lt, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Bool(a < b))
        }
        (BinaryOp::Lte, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Bool(a <= b))
        }
        (BinaryOp::Gt, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Bool(a > b))
        }
        (BinaryOp::Gte, SqlValue::Float(a), SqlValue::Float(b)) => {
            Some(SqlValue::Bool(a >= b))
        }
        (BinaryOp::Lt, SqlValue::Text(a), SqlValue::Text(b)) => {
            Some(SqlValue::Bool(a < b))
        }
        (BinaryOp::Lte, SqlValue::Text(a), SqlValue::Text(b)) => {
            Some(SqlValue::Bool(a <= b))
        }
        (BinaryOp::Gt, SqlValue::Text(a), SqlValue::Text(b)) => {
            Some(SqlValue::Bool(a > b))
        }
        (BinaryOp::Gte, SqlValue::Text(a), SqlValue::Text(b)) => {
            Some(SqlValue::Bool(a >= b))
        }

        // AND/OR with literal booleans (both sides are already literals here).
        (BinaryOp::And, SqlValue::Bool(a), SqlValue::Bool(b)) => {
            Some(SqlValue::Bool(*a && *b))
        }
        (BinaryOp::Or, SqlValue::Bool(a), SqlValue::Bool(b)) => {
            Some(SqlValue::Bool(*a || *b))
        }

        // Everything else — leave for the VM.
        _ => None,
    }
}

/// Fold a unary expression where the operand has already been folded.
fn fold_unary(op: UnaryOp, operand: SqlExpr) -> SqlExpr {
    use coding_adventures_sql_backend::SqlValue;
    use coding_adventures_sql_planner::SqlExpr::Literal;

    match (&op, &operand) {
        // Use checked_neg to avoid a panic when n == i64::MIN.
        // -i64::MIN overflows in two's complement; leave it for the VM to report.
        (UnaryOp::Neg, Literal(SqlValue::Int(n))) => match n.checked_neg() {
            Some(v) => Literal(SqlValue::Int(v)),
            None => SqlExpr::UnaryOp {
                op,
                expr: Box::new(operand),
            },
        },
        (UnaryOp::Neg, Literal(SqlValue::Float(f))) => Literal(SqlValue::Float(-f)),
        (UnaryOp::Neg, Literal(SqlValue::Null)) => Literal(SqlValue::Null),
        (UnaryOp::Not, Literal(SqlValue::Bool(b))) => Literal(SqlValue::Bool(!b)),
        (UnaryOp::Not, Literal(SqlValue::Null)) => Literal(SqlValue::Null),
        _ => SqlExpr::UnaryOp {
            op,
            expr: Box::new(operand),
        },
    }
}

// ===========================================================================
// Pass 2: PredicatePushdown
// ===========================================================================

/// Move Filter nodes closer to the Scan leaves.
///
/// ## Why push predicates down?
///
/// A filter that eliminates 99 % of rows is best applied *before* expensive
/// operations — sorts that touch every row, joins that multiply row counts,
/// projections that evaluate expressions per row. The goal: "cheap first,
/// expensive later."
///
/// ## What we push through
///
/// - **Sort**: always safe — sorting doesn't change which rows exist, only
///   their order.
/// - **Project**: safe when the predicate only references columns that exist
///   *below* the projection (i.e., input columns, not computed aliases).
///   In this v1 we conservatively push only when we cannot detect a computed
///   column reference (treating all columns as pushable).
///
/// ## What we do NOT push through
///
/// - **Limit**: `LIMIT 5 + Filter` yields different rows than `Filter + LIMIT 5`.
/// - **Aggregate**: the filter above an aggregate is a HAVING clause; it
///   references aggregate outputs that don't exist below the Aggregate.
/// - **Join**: complex outer-join semantics make this unsafe in v1; we push
///   only through Sort.
pub struct PredicatePushdownPass;

impl Pass for PredicatePushdownPass {
    fn name(&self) -> &str {
        "PredicatePushdown"
    }

    fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan {
        push_predicate(plan)
    }
}

/// Recursively apply predicate pushdown to a plan tree.
fn push_predicate(plan: OptimizedPlan) -> OptimizedPlan {
    match plan {
        OptimizedPlan::Filter { input, predicate } => {
            // First recurse into the input, then try to push the predicate.
            let input = push_predicate(*input);
            push_filter_through(input, predicate)
        }

        OptimizedPlan::Project { input, columns } => OptimizedPlan::Project {
            input: Box::new(push_predicate(*input)),
            columns,
        },

        OptimizedPlan::Join {
            left,
            right,
            kind,
            condition,
        } => OptimizedPlan::Join {
            left: Box::new(push_predicate(*left)),
            right: Box::new(push_predicate(*right)),
            kind,
            condition,
        },

        OptimizedPlan::Aggregate {
            input,
            group_by,
            aggregates,
        } => OptimizedPlan::Aggregate {
            input: Box::new(push_predicate(*input)),
            group_by,
            aggregates,
        },

        OptimizedPlan::Having { input, predicate } => OptimizedPlan::Having {
            input: Box::new(push_predicate(*input)),
            predicate,
        },

        OptimizedPlan::Sort { input, keys } => OptimizedPlan::Sort {
            input: Box::new(push_predicate(*input)),
            keys,
        },

        OptimizedPlan::Limit {
            input,
            count,
            offset,
        } => OptimizedPlan::Limit {
            input: Box::new(push_predicate(*input)),
            count,
            offset,
        },

        OptimizedPlan::Distinct(inner) => {
            OptimizedPlan::Distinct(Box::new(push_predicate(*inner)))
        }

        OptimizedPlan::Union { left, right, all } => OptimizedPlan::Union {
            left: Box::new(push_predicate(*left)),
            right: Box::new(push_predicate(*right)),
            all,
        },

        other => other,
    }
}

/// Try to push a filter predicate through the given (already-pushed) input plan.
///
/// Rules:
/// - Through **Sort**: always safe — insert Filter below Sort.
/// - Through **Project**: push below Project (conservative: push always in v1).
/// - Through **Distinct**: always safe — DISTINCT doesn't change which rows exist
///   (only removes duplicates), so filtering before or after yields the same rows.
/// - Everything else: leave the Filter on top of the input.
fn push_filter_through(input: OptimizedPlan, predicate: SqlExpr) -> OptimizedPlan {
    match input {
        // Through Sort: Filter(Sort(x, keys), pred) → Sort(Filter(x, pred), keys)
        // Sort doesn't change which rows exist, only their order.
        OptimizedPlan::Sort { input: sort_input, keys } => OptimizedPlan::Sort {
            input: Box::new(OptimizedPlan::Filter {
                input: sort_input,
                predicate,
            }),
            keys,
        },

        // Through Project: Filter(Project(x, cols), pred) → Project(Filter(x, pred), cols)
        // This is safe in v1 (we don't yet resolve computed column aliases).
        OptimizedPlan::Project { input: proj_input, columns } => OptimizedPlan::Project {
            input: Box::new(OptimizedPlan::Filter {
                input: proj_input,
                predicate,
            }),
            columns,
        },

        // Through Distinct: Filter(Distinct(x), pred) → Distinct(Filter(x, pred))
        // Filtering before dedup yields the same unique rows.
        OptimizedPlan::Distinct(distinct_input) => {
            OptimizedPlan::Distinct(Box::new(OptimizedPlan::Filter {
                input: distinct_input,
                predicate,
            }))
        }

        // All other nodes: leave Filter on top.
        // This covers Scan, Aggregate, Having, Limit, Join, Union, etc.
        other => OptimizedPlan::Filter {
            input: Box::new(other),
            predicate,
        },
    }
}

// ===========================================================================
// Pass 3: ProjectionPruning
// ===========================================================================

/// Annotate Scans with only the columns the rest of the query needs.
///
/// ## Why prune?
///
/// Backends that support columnar reads can skip unneeded columns entirely,
/// avoiding unnecessary I/O and memory. A query like `SELECT id FROM users`
/// against a 50-column table needs only 1 column, not 50.
///
/// ## Approach
///
/// Top-down traversal carrying a "requirement set" — the set of column names
/// the parent plan needs. At each `Scan`, we write the required columns
/// (filtered by the scan's table alias) into `required_columns`.
///
/// A wildcard column (`name: "*"`) in a `Project` signals "all columns needed,"
/// so we set `required_columns = None`.
pub struct ProjectionPruningPass;

impl Pass for ProjectionPruningPass {
    fn name(&self) -> &str {
        "ProjectionPruning"
    }

    fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan {
        // Start with no requirement (None = all columns needed).
        prune_plan(plan, None)
    }
}

/// Walk the plan top-down, threading required-column information downward.
///
/// `required`: `None` means "all columns are needed" (no pruning possible).
///             `Some(set)` means "only these column names are needed."
fn prune_plan(plan: OptimizedPlan, required: Option<&HashSet<String>>) -> OptimizedPlan {
    match plan {
        OptimizedPlan::Scan {
            table,
            alias,
            scan_limit,
            ..
        } => {
            // If we have a requirement set, intersect it with what the scan can provide.
            // `required_columns = None` means "no constraint" = all columns.
            let required_columns = required.map(|req| {
                let mut cols: Vec<String> = req.iter().cloned().collect();
                cols.sort(); // Deterministic output for tests.
                cols
            });
            OptimizedPlan::Scan {
                table,
                alias,
                required_columns,
                scan_limit,
            }
        }

        OptimizedPlan::EmptyResult => OptimizedPlan::EmptyResult,

        OptimizedPlan::Project { input, columns } => {
            // The Project defines what its parent can see.
            // We compute what columns our child (the input) needs from the
            // expressions in our column list.
            let child_req = columns_required_by_output(&columns);
            // If any column is `*`, we need everything from the child.
            let child_req_opt: Option<HashSet<String>> = if columns
                .iter()
                .any(|c| matches!(&c.expr, SqlExpr::Column { name, .. } if name == "*"))
            {
                None // wildcard — can't prune
            } else {
                Some(child_req)
            };
            OptimizedPlan::Project {
                input: Box::new(prune_plan(*input, child_req_opt.as_ref())),
                columns,
            }
        }

        OptimizedPlan::Filter { input, predicate } => {
            // The filter needs whatever its parent needs PLUS the columns in
            // its predicate.
            let mut req = required.cloned().unwrap_or_default();
            collect_columns_in_expr(&predicate, &mut req);
            OptimizedPlan::Filter {
                input: Box::new(prune_plan(*input, Some(&req))),
                predicate,
            }
        }

        OptimizedPlan::Aggregate {
            input,
            group_by,
            aggregates,
        } => {
            // Aggregate needs the GROUP BY columns and the aggregate arguments.
            let mut req: HashSet<String> = HashSet::new();
            for e in &group_by {
                collect_columns_in_expr(e, &mut req);
            }
            for agg in &aggregates {
                if let Some(arg) = &agg.arg {
                    collect_columns_in_expr(arg, &mut req);
                }
            }
            let req_opt = Some(req);
            OptimizedPlan::Aggregate {
                input: Box::new(prune_plan(*input, req_opt.as_ref())),
                group_by,
                aggregates,
            }
        }

        OptimizedPlan::Having { input, predicate } => {
            let mut req = required.cloned().unwrap_or_default();
            collect_columns_in_expr(&predicate, &mut req);
            OptimizedPlan::Having {
                input: Box::new(prune_plan(*input, Some(&req))),
                predicate,
            }
        }

        OptimizedPlan::Sort { input, keys } => {
            let mut req = required.cloned().unwrap_or_default();
            for k in &keys {
                collect_columns_in_expr(&k.expr, &mut req);
            }
            OptimizedPlan::Sort {
                input: Box::new(prune_plan(*input, Some(&req))),
                keys,
            }
        }

        OptimizedPlan::Limit {
            input,
            count,
            offset,
        } => OptimizedPlan::Limit {
            input: Box::new(prune_plan(*input, required)),
            count,
            offset,
        },

        OptimizedPlan::Distinct(inner) => {
            OptimizedPlan::Distinct(Box::new(prune_plan(*inner, required)))
        }

        OptimizedPlan::Join {
            left,
            right,
            kind,
            condition,
        } => {
            let mut req = required.cloned().unwrap_or_default();
            if let Some(cond) = &condition {
                collect_columns_in_expr(cond, &mut req);
            }
            let req_opt = Some(req);
            OptimizedPlan::Join {
                left: Box::new(prune_plan(*left, req_opt.as_ref())),
                right: Box::new(prune_plan(*right, req_opt.as_ref())),
                kind,
                condition,
            }
        }

        OptimizedPlan::Union { left, right, all } => OptimizedPlan::Union {
            left: Box::new(prune_plan(*left, required)),
            right: Box::new(prune_plan(*right, required)),
            all,
        },

        // DML/DDL: pass through.
        other => other,
    }
}

/// Collect column names referenced in a list of output columns.
///
/// This is used by the `Project` handler to determine what its input must provide.
fn columns_required_by_output(columns: &[OutputColumn]) -> HashSet<String> {
    let mut out = HashSet::new();
    for col in columns {
        collect_columns_in_expr(&col.expr, &mut out);
    }
    out
}

/// Walk a [`SqlExpr`] and collect all referenced column names into `out`.
///
/// We collect the bare `name` field of each `Column` variant (ignoring the
/// optional `table` qualifier for now). The backend receives column names
/// without table prefixes.
fn collect_columns_in_expr(expr: &SqlExpr, out: &mut HashSet<String>) {
    match expr {
        SqlExpr::Column { name, .. } => {
            if name != "*" {
                out.insert(name.clone());
            }
        }
        SqlExpr::BinaryOp { left, right, .. } => {
            collect_columns_in_expr(left, out);
            collect_columns_in_expr(right, out);
        }
        SqlExpr::UnaryOp { expr, .. } => collect_columns_in_expr(expr, out),
        SqlExpr::Cast { expr, .. } => collect_columns_in_expr(expr, out),
        SqlExpr::Case { branches, else_val } => {
            for (cond, val) in branches {
                collect_columns_in_expr(cond, out);
                collect_columns_in_expr(val, out);
            }
            if let Some(e) = else_val {
                collect_columns_in_expr(e, out);
            }
        }
        SqlExpr::IsNull(inner) | SqlExpr::IsNotNull(inner) => {
            collect_columns_in_expr(inner, out)
        }
        SqlExpr::Between {
            value, low, high, ..
        } => {
            collect_columns_in_expr(value, out);
            collect_columns_in_expr(low, out);
            collect_columns_in_expr(high, out);
        }
        SqlExpr::Like { value, pattern, .. } => {
            collect_columns_in_expr(value, out);
            collect_columns_in_expr(pattern, out);
        }
        SqlExpr::InList { value, list, .. } => {
            collect_columns_in_expr(value, out);
            for item in list {
                collect_columns_in_expr(item, out);
            }
        }
        SqlExpr::FunctionCall { args, .. } => {
            for arg in args {
                collect_columns_in_expr(arg, out);
            }
        }
        SqlExpr::Aggregate { arg, .. } => {
            if let Some(arg) = arg {
                collect_columns_in_expr(arg, out);
            }
        }
        SqlExpr::Literal(_) => {}
    }
}

// ===========================================================================
// Pass 4: DeadCodeElimination
// ===========================================================================

/// Replace provably-empty subtrees with [`OptimizedPlan::EmptyResult`].
///
/// ## Driven by what ConstantFolding produced
///
/// After constant folding:
/// - `Filter(scan, Literal(Bool(false)))` — the filter cannot pass any rows.
/// - `Filter(scan, Literal(Null))` — NULL in WHERE is treated as FALSE.
/// - `Limit(_, Some(0), _)` — LIMIT 0 returns no rows.
///
/// ## EmptyResult propagation
///
/// Once we produce an `EmptyResult`, we propagate it upward:
///
/// | Outer node                     | Result                      |
/// |--------------------------------|-----------------------------|
/// | `Filter(EmptyResult, _)`       | `EmptyResult`               |
/// | `Project(EmptyResult, _)`      | `Project(EmptyResult, _)` * |
/// | `Sort(EmptyResult, _)`         | `EmptyResult`               |
/// | `Limit(EmptyResult, _, _)`     | `EmptyResult`               |
/// | `Distinct(EmptyResult)`        | `EmptyResult`               |
/// | `Join(EmptyResult, _, INNER)`  | `EmptyResult`               |
/// | `Join(_, EmptyResult, INNER)`  | `EmptyResult`               |
///
/// \* `Project` is intentionally **not** propagated through: the Project's
/// column list encodes the SELECT output schema.  Preserving it lets the
/// codegen emit `DefineColumns` so that `QueryResult.columns` is populated
/// even when no rows are produced (e.g. `SELECT x FROM t LIMIT 0`).
///
/// Outer joins are NOT propagated in v1 — a LEFT JOIN with an empty right
/// side still produces left-side rows with NULL-filled right columns.
///
/// `Aggregate(EmptyResult, ..)` is also NOT propagated — `SELECT COUNT(*)`
/// from an empty table must still produce one row (the value 0).
pub struct DeadCodeEliminationPass;

impl Pass for DeadCodeEliminationPass {
    fn name(&self) -> &str {
        "DeadCodeElimination"
    }

    fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan {
        eliminate_dead_code(plan)
    }
}

/// Recursively eliminate dead code from a plan tree.
fn eliminate_dead_code(plan: OptimizedPlan) -> OptimizedPlan {
    use coding_adventures_sql_backend::SqlValue;
    use coding_adventures_sql_planner::SqlExpr::Literal;

    match plan {
        // Leaves — nothing to eliminate.
        OptimizedPlan::Scan { .. } | OptimizedPlan::EmptyResult => plan,

        OptimizedPlan::Filter { input, predicate } => {
            let input = eliminate_dead_code(*input);

            // If the input is already empty, no rows to filter.
            if matches!(input, OptimizedPlan::EmptyResult) {
                return OptimizedPlan::EmptyResult;
            }

            // If the predicate folds to FALSE or NULL, no rows pass.
            match &predicate {
                Literal(SqlValue::Bool(false)) => OptimizedPlan::EmptyResult,
                Literal(SqlValue::Null) => OptimizedPlan::EmptyResult,
                // TRUE → the filter is a no-op; remove it.
                Literal(SqlValue::Bool(true)) => input,
                _ => OptimizedPlan::Filter {
                    input: Box::new(input),
                    predicate,
                },
            }
        }

        OptimizedPlan::Project { input, columns } => {
            let input = eliminate_dead_code(*input);
            // We do NOT propagate EmptyResult through Project because the
            // Project's column list encodes the SELECT output schema.  If we
            // collapsed Project(EmptyResult) → EmptyResult the codegen would
            // lose the column names and the QueryResult would have an empty
            // `columns` vec even though the SQL clearly selected named columns
            // (e.g. `SELECT x FROM t LIMIT 0` should return columns=["x"],
            // rows=[]).  Keeping Project(EmptyResult) lets compile_project emit
            // a DefineColumns instruction that records the schema without
            // producing any rows.
            OptimizedPlan::Project {
                input: Box::new(input),
                columns,
            }
        }

        OptimizedPlan::Sort { input, keys } => {
            let input = eliminate_dead_code(*input);
            if matches!(input, OptimizedPlan::EmptyResult) {
                OptimizedPlan::EmptyResult
            } else {
                OptimizedPlan::Sort {
                    input: Box::new(input),
                    keys,
                }
            }
        }

        OptimizedPlan::Limit {
            input,
            count,
            offset,
        } => {
            let input = eliminate_dead_code(*input);
            // LIMIT 0 → empty regardless of input.
            if count == Some(0) {
                return OptimizedPlan::EmptyResult;
            }
            if matches!(input, OptimizedPlan::EmptyResult) {
                OptimizedPlan::EmptyResult
            } else {
                OptimizedPlan::Limit {
                    input: Box::new(input),
                    count,
                    offset,
                }
            }
        }

        OptimizedPlan::Distinct(inner) => {
            let inner = eliminate_dead_code(*inner);
            if matches!(inner, OptimizedPlan::EmptyResult) {
                OptimizedPlan::EmptyResult
            } else {
                OptimizedPlan::Distinct(Box::new(inner))
            }
        }

        OptimizedPlan::Having { input, predicate } => {
            let input = eliminate_dead_code(*input);
            if matches!(input, OptimizedPlan::EmptyResult) {
                OptimizedPlan::EmptyResult
            } else {
                OptimizedPlan::Having {
                    input: Box::new(input),
                    predicate,
                }
            }
        }

        OptimizedPlan::Aggregate {
            input,
            group_by,
            aggregates,
        } => {
            // Do NOT eliminate Aggregate on EmptyResult input.
            // `SELECT COUNT(*) FROM empty_table` must still return one row (0).
            OptimizedPlan::Aggregate {
                input: Box::new(eliminate_dead_code(*input)),
                group_by,
                aggregates,
            }
        }

        OptimizedPlan::Join {
            left,
            right,
            kind,
            condition,
        } => {
            let left = eliminate_dead_code(*left);
            let right = eliminate_dead_code(*right);
            // INNER and CROSS joins with an empty side are empty.
            if (kind == JoinKind::Inner || kind == JoinKind::Cross)
                && (matches!(left, OptimizedPlan::EmptyResult)
                    || matches!(right, OptimizedPlan::EmptyResult))
            {
                return OptimizedPlan::EmptyResult;
            }
            OptimizedPlan::Join {
                left: Box::new(left),
                right: Box::new(right),
                kind,
                condition,
            }
        }

        OptimizedPlan::Union { left, right, all } => {
            let left = eliminate_dead_code(*left);
            let right = eliminate_dead_code(*right);
            // Union with one empty side → just the other side.
            match (
                matches!(left, OptimizedPlan::EmptyResult),
                matches!(right, OptimizedPlan::EmptyResult),
            ) {
                (true, true) => OptimizedPlan::EmptyResult,
                (true, false) => right,
                (false, true) => left,
                (false, false) => OptimizedPlan::Union {
                    left: Box::new(left),
                    right: Box::new(right),
                    all,
                },
            }
        }

        // DML/DDL: pass through unchanged.
        other => other,
    }
}

// ===========================================================================
// Pass 5: LimitPushdown
// ===========================================================================

/// Attach `scan_limit` hints to Scan nodes when a Limit sits above them.
///
/// ## A hint, not a guarantee
///
/// The VM always enforces the real `Limit` at the correct level. These hints
/// tell a backend "you only need to return at most N rows" so it can stop
/// reading early. When a `Filter` sits between the `Limit` and the `Scan`,
/// the backend may need to read more raw rows before N pass the filter —
/// but the real `Limit` above the Filter is still the arbiter.
///
/// ## What we push through
///
/// - `Limit(Scan, n, offset)` → set `scan.scan_limit = Some(n + offset.unwrap_or(0))`
///   (the scan needs to supply at least `n + offset` rows for the Limit to work).
/// - `Limit(Sort(Scan), n, offset)` → same, because Sort needs all rows before
///   Limit can pick the top N.
///
/// ## What we do NOT push through
///
/// - `Filter` between Limit and Scan: the filter may reduce rows, so the scan
///   may need to read more than `n + offset` rows. We do NOT set `scan_limit`
///   in that case (conservative).
/// - `Aggregate`: aggregates consume all input rows.
/// - `Join`: joins may duplicate or drop rows.
/// - `Distinct`: dedup needs all rows first.
pub struct LimitPushdownPass;

impl Pass for LimitPushdownPass {
    fn name(&self) -> &str {
        "LimitPushdown"
    }

    fn apply(&self, plan: OptimizedPlan) -> OptimizedPlan {
        push_limit(plan)
    }
}

/// Recursively apply limit pushdown to a plan tree.
fn push_limit(plan: OptimizedPlan) -> OptimizedPlan {
    match plan {
        OptimizedPlan::Limit {
            input,
            count,
            offset,
        } => {
            // Recurse into the input first.
            let input = push_limit(*input);
            // Compute the total rows the scan needs: count + offset.
            // `offset` defaults to 0 if absent.
            let scan_need = count.map(|n| n + offset.unwrap_or(0));
            // Attach hint if we have a non-zero count.
            let new_input = if let Some(need) = scan_need {
                if need > 0 {
                    attach_scan_limit(input, need)
                } else {
                    input
                }
            } else {
                input
            };
            OptimizedPlan::Limit {
                input: Box::new(new_input),
                count,
                offset,
            }
        }

        // Recurse into all other structural nodes.
        OptimizedPlan::Filter { input, predicate } => OptimizedPlan::Filter {
            input: Box::new(push_limit(*input)),
            predicate,
        },

        OptimizedPlan::Project { input, columns } => OptimizedPlan::Project {
            input: Box::new(push_limit(*input)),
            columns,
        },

        OptimizedPlan::Sort { input, keys } => OptimizedPlan::Sort {
            input: Box::new(push_limit(*input)),
            keys,
        },

        OptimizedPlan::Aggregate {
            input,
            group_by,
            aggregates,
        } => OptimizedPlan::Aggregate {
            input: Box::new(push_limit(*input)),
            group_by,
            aggregates,
        },

        OptimizedPlan::Having { input, predicate } => OptimizedPlan::Having {
            input: Box::new(push_limit(*input)),
            predicate,
        },

        OptimizedPlan::Distinct(inner) => {
            OptimizedPlan::Distinct(Box::new(push_limit(*inner)))
        }

        OptimizedPlan::Join {
            left,
            right,
            kind,
            condition,
        } => OptimizedPlan::Join {
            left: Box::new(push_limit(*left)),
            right: Box::new(push_limit(*right)),
            kind,
            condition,
        },

        OptimizedPlan::Union { left, right, all } => OptimizedPlan::Union {
            left: Box::new(push_limit(*left)),
            right: Box::new(push_limit(*right)),
            all,
        },

        other => other,
    }
}

/// Thread a `scan_limit` hint through safe passthrough nodes until it reaches a `Scan`.
///
/// We push through Sort (needs rows before trimming) but NOT through Filter
/// (filter may discard rows, so backend needs to read more than `limit`).
fn attach_scan_limit(plan: OptimizedPlan, limit: i64) -> OptimizedPlan {
    match plan {
        OptimizedPlan::Scan {
            table,
            alias,
            required_columns,
            scan_limit: existing,
        } => {
            // If there's already a tighter hint, preserve it.
            let new_limit = match existing {
                Some(prev) => prev.min(limit),
                None => limit,
            };
            OptimizedPlan::Scan {
                table,
                alias,
                required_columns,
                scan_limit: Some(new_limit),
            }
        }

        // Pass through Sort: a Sort above a Limit still needs at most `limit`
        // raw rows from the scan (the sort reads all of them to find the top N).
        OptimizedPlan::Sort { input, keys } => OptimizedPlan::Sort {
            input: Box::new(attach_scan_limit(*input, limit)),
            keys,
        },

        // Do NOT push through Filter — filter may discard rows.
        // Fall back to the regular push_limit walk.
        other => push_limit(other),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_sql_backend::{ColumnDef, SqlValue};
    use coding_adventures_sql_planner::{
        AggFunc, AggregateItem, Assignment, BinaryOp, InsertSource, JoinKind, LogicalPlan,
        OutputColumn, SortKey, SqlExpr, UnaryOp,
    };

    // --- Helpers ---

    /// Build a simple Scan plan.
    fn scan(table: &str) -> LogicalPlan {
        LogicalPlan::Scan {
            table: table.to_string(),
            alias: None,
        }
    }

    /// Build a Scan with an alias.
    fn scan_alias(table: &str, alias: &str) -> LogicalPlan {
        LogicalPlan::Scan {
            table: table.to_string(),
            alias: Some(alias.to_string()),
        }
    }

    /// Build an OptimizedPlan Scan (as if after lift).
    fn opt_scan(table: &str) -> OptimizedPlan {
        OptimizedPlan::Scan {
            table: table.to_string(),
            alias: None,
            required_columns: None,
            scan_limit: None,
        }
    }

    /// Integer literal expression.
    fn lit_int(n: i64) -> SqlExpr {
        SqlExpr::Literal(SqlValue::Int(n))
    }

    /// Boolean literal expression.
    fn lit_bool(b: bool) -> SqlExpr {
        SqlExpr::Literal(SqlValue::Bool(b))
    }

    /// NULL literal expression.
    fn lit_null() -> SqlExpr {
        SqlExpr::Literal(SqlValue::Null)
    }

    /// Column reference expression.
    fn col(name: &str) -> SqlExpr {
        SqlExpr::Column {
            table: None,
            name: name.to_string(),
        }
    }

    /// Binary operation expression.
    fn bin(op: BinaryOp, left: SqlExpr, right: SqlExpr) -> SqlExpr {
        SqlExpr::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Filter plan wrapping a logical plan.
    fn filter(inner: LogicalPlan, pred: SqlExpr) -> LogicalPlan {
        LogicalPlan::Filter {
            input: Box::new(inner),
            predicate: pred,
        }
    }

    /// Sort plan wrapping a logical plan.
    fn sort(inner: LogicalPlan, col_name: &str) -> LogicalPlan {
        LogicalPlan::Sort {
            input: Box::new(inner),
            keys: vec![SortKey {
                expr: col(col_name),
                ascending: true,
                nulls_first: None,
            }],
        }
    }

    /// Limit plan wrapping a logical plan.
    fn limit(inner: LogicalPlan, count: i64, offset: Option<i64>) -> LogicalPlan {
        LogicalPlan::Limit {
            input: Box::new(inner),
            count: Some(count),
            offset,
        }
    }

    // =======================================================================
    // Lift tests
    // =======================================================================

    #[test]
    fn test_lift_scan_initializes_annotations() {
        // A bare Scan should lift with both annotations as None.
        let plan = scan("users");
        let opt = optimize(plan);
        assert_eq!(
            opt,
            OptimizedPlan::Scan {
                table: "users".to_string(),
                alias: None,
                required_columns: None,
                scan_limit: None,
            }
        );
    }

    #[test]
    fn test_lift_scan_with_alias() {
        let plan = scan_alias("users", "u");
        let opt = optimize(plan);
        assert!(matches!(
            opt,
            OptimizedPlan::Scan {
                alias: Some(ref a),
                ..
            } if a == "u"
        ));
    }

    #[test]
    fn test_lift_dml_insert_passthrough() {
        // DML plans should pass through the optimizer unchanged.
        let plan = LogicalPlan::Insert {
            table: "t".to_string(),
            columns: None,
            source: InsertSource::Values(vec![vec![lit_int(42)]]),
        };
        let opt = optimize(plan);
        assert!(matches!(opt, OptimizedPlan::Insert { .. }));
    }

    #[test]
    fn test_lift_ddl_create_table() {
        let plan = LogicalPlan::CreateTable {
            table: "t".to_string(),
            if_not_exists: false,
            columns: vec![ColumnDef::new("id", "INTEGER")],
        };
        let opt = optimize(plan);
        assert!(matches!(opt, OptimizedPlan::CreateTable { .. }));
    }

    #[test]
    fn test_lift_ddl_drop_table() {
        let plan = LogicalPlan::DropTable {
            table: "t".to_string(),
            if_exists: true,
        };
        let opt = optimize(plan);
        assert!(matches!(opt, OptimizedPlan::DropTable { if_exists: true, .. }));
    }

    #[test]
    fn test_lift_update() {
        let plan = LogicalPlan::Update {
            table: "t".to_string(),
            assignments: vec![Assignment {
                column: "x".to_string(),
                value: lit_int(1),
            }],
            predicate: None,
        };
        let opt = optimize(plan);
        assert!(matches!(opt, OptimizedPlan::Update { .. }));
    }

    #[test]
    fn test_lift_delete() {
        let plan = LogicalPlan::Delete {
            table: "t".to_string(),
            predicate: Some(lit_bool(true)),
        };
        let opt = optimize(plan);
        assert!(matches!(opt, OptimizedPlan::Delete { .. }));
    }

    // =======================================================================
    // Pass 1: ConstantFolding tests
    // =======================================================================

    fn fold(expr: SqlExpr) -> SqlExpr {
        fold_expr(expr)
    }

    #[test]
    fn test_cf_int_add() {
        // 1 + 1 → Int(2)
        let e = bin(BinaryOp::Add, lit_int(1), lit_int(1));
        assert_eq!(fold(e), lit_int(2));
    }

    #[test]
    fn test_cf_int_mul() {
        // 3 * 4 → Int(12)
        let e = bin(BinaryOp::Mul, lit_int(3), lit_int(4));
        assert_eq!(fold(e), lit_int(12));
    }

    #[test]
    fn test_cf_int_sub() {
        // 10 - 3 → Int(7)
        let e = bin(BinaryOp::Sub, lit_int(10), lit_int(3));
        assert_eq!(fold(e), lit_int(7));
    }

    #[test]
    fn test_cf_int_div() {
        // 10 / 2 → Int(5)
        let e = bin(BinaryOp::Div, lit_int(10), lit_int(2));
        assert_eq!(fold(e), lit_int(5));
    }

    #[test]
    fn test_cf_div_by_zero_left_alone() {
        // 5 / 0 — leave for VM
        let e = bin(BinaryOp::Div, lit_int(5), lit_int(0));
        // Result should NOT be a literal (left as BinaryOp).
        assert!(matches!(fold(e), SqlExpr::BinaryOp { .. }));
    }

    #[test]
    fn test_cf_and_false_short_circuits() {
        // x AND FALSE → FALSE
        let e = bin(BinaryOp::And, col("x"), lit_bool(false));
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_cf_false_and_x_short_circuits() {
        // FALSE AND x → FALSE
        let e = bin(BinaryOp::And, lit_bool(false), col("x"));
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_cf_or_true_short_circuits() {
        // x OR TRUE → TRUE
        let e = bin(BinaryOp::Or, col("x"), lit_bool(true));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_true_or_x_short_circuits() {
        // TRUE OR x → TRUE
        let e = bin(BinaryOp::Or, lit_bool(true), col("x"));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_true_and_x_identity() {
        // TRUE AND x → x
        let e = bin(BinaryOp::And, lit_bool(true), col("x"));
        assert_eq!(fold(e), col("x"));
    }

    #[test]
    fn test_cf_x_and_true_identity() {
        // x AND TRUE → x
        let e = bin(BinaryOp::And, col("x"), lit_bool(true));
        assert_eq!(fold(e), col("x"));
    }

    #[test]
    fn test_cf_false_or_x_identity() {
        // FALSE OR x → x
        let e = bin(BinaryOp::Or, lit_bool(false), col("x"));
        assert_eq!(fold(e), col("x"));
    }

    #[test]
    fn test_cf_null_is_null() {
        // NULL IS NULL → TRUE
        let e = SqlExpr::IsNull(Box::new(lit_null()));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_int_is_not_null() {
        // 42 IS NOT NULL → TRUE
        let e = SqlExpr::IsNotNull(Box::new(lit_int(42)));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_null_is_not_null() {
        // NULL IS NOT NULL → FALSE
        let e = SqlExpr::IsNotNull(Box::new(lit_null()));
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_cf_literal_is_null_false() {
        // 1 IS NULL → FALSE
        let e = SqlExpr::IsNull(Box::new(lit_int(1)));
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_cf_null_propagation_in_add() {
        // NULL + 1 → NULL
        let e = bin(BinaryOp::Add, lit_null(), lit_int(1));
        assert_eq!(fold(e), lit_null());
    }

    #[test]
    fn test_cf_neg_int() {
        // -5 (unary negation of literal 5) → Int(-5)
        let e = SqlExpr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(lit_int(5)),
        };
        assert_eq!(fold(e), lit_int(-5));
    }

    #[test]
    fn test_cf_neg_i64_min_left_unfolded() {
        // -(i64::MIN) overflows in two's complement — must NOT panic.
        // The expression is left unfolded so the VM can handle it at runtime.
        let e = SqlExpr::UnaryOp {
            op: UnaryOp::Neg,
            expr: Box::new(lit_int(i64::MIN)),
        };
        // Result should be UnaryOp (not a Literal), because checked_neg returns None.
        assert!(matches!(fold(e), SqlExpr::UnaryOp { .. }));
    }

    #[test]
    fn test_cf_not_true() {
        // NOT TRUE → FALSE
        let e = SqlExpr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(lit_bool(true)),
        };
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_cf_not_false() {
        // NOT FALSE → TRUE
        let e = SqlExpr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(lit_bool(false)),
        };
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_column_stays_unfolded() {
        // Column references are not folded.
        let e = col("name");
        assert_eq!(fold(e.clone()), e);
    }

    #[test]
    fn test_cf_equality_true() {
        // 1 = 1 → TRUE
        let e = bin(BinaryOp::Eq, lit_int(1), lit_int(1));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_equality_false() {
        // 1 = 2 → FALSE
        let e = bin(BinaryOp::Eq, lit_int(1), lit_int(2));
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_cf_string_concat() {
        // 'hello' || ' world' → 'hello world'
        let e = bin(
            BinaryOp::Concat,
            SqlExpr::Literal(SqlValue::Text("hello".to_string())),
            SqlExpr::Literal(SqlValue::Text(" world".to_string())),
        );
        assert_eq!(
            fold(e),
            SqlExpr::Literal(SqlValue::Text("hello world".to_string()))
        );
    }

    #[test]
    fn test_cf_nested_constant_expression() {
        // (1 + 2) * (3 + 4) → 21
        let e = bin(
            BinaryOp::Mul,
            bin(BinaryOp::Add, lit_int(1), lit_int(2)),
            bin(BinaryOp::Add, lit_int(3), lit_int(4)),
        );
        assert_eq!(fold(e), lit_int(21));
    }

    #[test]
    fn test_cf_plan_folds_filter_predicate() {
        // Filter(scan, 1 + 1 = 2) → Filter(scan, TRUE)
        let pred = bin(BinaryOp::Eq, bin(BinaryOp::Add, lit_int(1), lit_int(1)), lit_int(2));
        let plan = filter(scan("t"), pred);
        let opt = optimize_with_passes(plan, &[&ConstantFoldingPass]);
        assert!(matches!(
            opt,
            OptimizedPlan::Filter {
                predicate: SqlExpr::Literal(SqlValue::Bool(true)),
                ..
            }
        ));
    }

    // =======================================================================
    // Pass 2: PredicatePushdown tests
    // =======================================================================

    #[test]
    fn test_ppd_filter_stays_on_scan() {
        // Filter(Scan, pred) — cannot push further; stays as-is.
        let pred = bin(BinaryOp::Gt, col("age"), lit_int(18));
        let plan = filter(scan("users"), pred.clone());
        let opt = optimize_with_passes(plan, &[&PredicatePushdownPass]);
        // Should be: Filter { input: Scan, predicate: pred }
        assert!(matches!(&opt, OptimizedPlan::Filter { .. }));
    }

    #[test]
    fn test_ppd_filter_through_sort() {
        // Filter(Sort(scan, key), pred) → Sort(Filter(scan, pred), key)
        let pred = bin(BinaryOp::Gt, col("age"), lit_int(18));
        let plan = filter(sort(scan("users"), "name"), pred.clone());
        let opt = optimize_with_passes(plan, &[&PredicatePushdownPass]);
        // The outer node should now be Sort.
        assert!(matches!(&opt, OptimizedPlan::Sort { .. }));
        // And inside Sort there should be a Filter.
        if let OptimizedPlan::Sort { input, .. } = &opt {
            assert!(matches!(input.as_ref(), OptimizedPlan::Filter { .. }));
        }
    }

    #[test]
    fn test_ppd_filter_not_through_limit() {
        // Filter(Limit(scan, 10), pred) — CANNOT push through Limit.
        // The Filter should remain on top of Limit.
        let pred = lit_bool(true);
        let plan = filter(limit(scan("t"), 10, None), pred);
        let _opt = optimize_with_passes(plan, &[&PredicatePushdownPass]);
        // Outer node is still Filter (limit blocks pushdown).
        // But after DCE TRUE predicates get dropped — test with a column predicate.
        let pred2 = bin(BinaryOp::Gt, col("id"), lit_int(5));
        let plan2 = filter(limit(scan("t"), 10, None), pred2);
        let opt2 = optimize_with_passes(plan2, &[&PredicatePushdownPass]);
        // The outer node should be Filter (Limit blocks pushdown).
        assert!(matches!(&opt2, OptimizedPlan::Filter { .. }));
        if let OptimizedPlan::Filter { input, .. } = &opt2 {
            assert!(matches!(input.as_ref(), OptimizedPlan::Limit { .. }));
        }
    }

    #[test]
    fn test_ppd_filter_through_project() {
        // Filter(Project(scan, cols), pred) → Project(Filter(scan, pred), cols)
        let pred = bin(BinaryOp::Gt, col("id"), lit_int(0));
        let plan = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::Project {
                input: Box::new(scan("t")),
                columns: vec![OutputColumn {
                    expr: col("id"),
                    alias: None,
                }],
            }),
            predicate: pred,
        };
        let opt = optimize_with_passes(plan, &[&PredicatePushdownPass]);
        assert!(matches!(&opt, OptimizedPlan::Project { .. }));
    }

    #[test]
    fn test_ppd_filter_through_distinct() {
        // Filter(Distinct(Scan), pred) → Distinct(Filter(Scan, pred))
        let pred = bin(BinaryOp::Eq, col("id"), lit_int(1));
        let plan = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::Distinct(Box::new(scan("t")))),
            predicate: pred,
        };
        let opt = optimize_with_passes(plan, &[&PredicatePushdownPass]);
        assert!(matches!(&opt, OptimizedPlan::Distinct(_)));
        if let OptimizedPlan::Distinct(inner) = &opt {
            assert!(matches!(inner.as_ref(), OptimizedPlan::Filter { .. }));
        }
    }

    // =======================================================================
    // Pass 3: ProjectionPruning tests
    // =======================================================================

    #[test]
    fn test_proj_prune_single_column() {
        // Project(Scan, [col("id")]) → Scan with required_columns = Some(["id"])
        let plan = LogicalPlan::Project {
            input: Box::new(scan("users")),
            columns: vec![OutputColumn {
                expr: col("id"),
                alias: None,
            }],
        };
        let opt = optimize_with_passes(plan, &[&ProjectionPruningPass]);
        if let OptimizedPlan::Project { input, .. } = opt {
            if let OptimizedPlan::Scan {
                required_columns: Some(cols),
                ..
            } = *input
            {
                assert!(cols.contains(&"id".to_string()));
                assert_eq!(cols.len(), 1);
            } else {
                panic!("expected Scan with required_columns");
            }
        } else {
            panic!("expected Project");
        }
    }

    #[test]
    fn test_proj_prune_wildcard_keeps_none() {
        // Project(Scan, [col("*")]) → Scan with required_columns = None (all columns)
        let plan = LogicalPlan::Project {
            input: Box::new(scan("users")),
            columns: vec![OutputColumn {
                expr: SqlExpr::Column {
                    table: None,
                    name: "*".to_string(),
                },
                alias: None,
            }],
        };
        let opt = optimize_with_passes(plan, &[&ProjectionPruningPass]);
        if let OptimizedPlan::Project { input, .. } = opt {
            assert!(matches!(
                *input,
                OptimizedPlan::Scan {
                    required_columns: None,
                    ..
                }
            ));
        } else {
            panic!("expected Project");
        }
    }

    #[test]
    fn test_proj_prune_multiple_columns() {
        // Project(Scan, [col("id"), col("name")]) → required_columns = ["id", "name"]
        let plan = LogicalPlan::Project {
            input: Box::new(scan("users")),
            columns: vec![
                OutputColumn {
                    expr: col("id"),
                    alias: None,
                },
                OutputColumn {
                    expr: col("name"),
                    alias: None,
                },
            ],
        };
        let opt = optimize_with_passes(plan, &[&ProjectionPruningPass]);
        if let OptimizedPlan::Project { input, .. } = opt {
            if let OptimizedPlan::Scan {
                required_columns: Some(cols),
                ..
            } = *input
            {
                let mut sorted = cols.clone();
                sorted.sort();
                assert!(sorted.contains(&"id".to_string()));
                assert!(sorted.contains(&"name".to_string()));
            } else {
                panic!("expected Scan with required_columns");
            }
        } else {
            panic!("expected Project");
        }
    }

    #[test]
    fn test_proj_prune_bare_scan_no_annotation() {
        // Bare Scan with no Project above → no required_columns annotation.
        let plan = scan("t");
        let opt = optimize_with_passes(plan, &[&ProjectionPruningPass]);
        assert!(matches!(
            opt,
            OptimizedPlan::Scan {
                required_columns: None,
                ..
            }
        ));
    }

    // =======================================================================
    // Pass 4: DeadCodeElimination tests
    // =======================================================================

    #[test]
    fn test_dce_filter_false_becomes_empty() {
        // Filter(scan, FALSE) → EmptyResult
        let plan = filter(scan("t"), lit_bool(false));
        // We need ConstantFolding first (FALSE is already a literal here).
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_filter_null_becomes_empty() {
        // Filter(scan, NULL) → EmptyResult (NULL in WHERE is FALSE)
        let plan = filter(scan("t"), lit_null());
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_filter_true_drops_filter() {
        // Filter(scan, TRUE) → scan (filter is a no-op)
        let plan = filter(scan("t"), lit_bool(true));
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert!(matches!(opt, OptimizedPlan::Scan { .. }));
    }

    #[test]
    fn test_dce_limit_zero_becomes_empty() {
        // Limit(scan, 0) → EmptyResult
        let plan = limit(scan("t"), 0, None);
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_project_on_empty_keeps_project() {
        // Project(EmptyResult) is intentionally NOT collapsed to EmptyResult.
        // We preserve the Project wrapper so that the codegen can emit a
        // DefineColumns instruction, giving QueryResult the correct column
        // schema even when no rows are produced (e.g. SELECT x … LIMIT 0).
        let plan = LogicalPlan::Project {
            input: Box::new(filter(scan("t"), lit_bool(false))),
            columns: vec![OutputColumn {
                expr: col("id"),
                alias: None,
            }],
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        // The outer node should still be Project with an EmptyResult inner.
        assert!(matches!(opt, OptimizedPlan::Project { .. }));
        if let OptimizedPlan::Project { input, .. } = opt {
            assert_eq!(*input, OptimizedPlan::EmptyResult);
        }
    }

    #[test]
    fn test_dce_filter_on_empty_becomes_empty() {
        // Filter(EmptyResult, _) → EmptyResult
        let plan = filter(filter(scan("t"), lit_bool(false)), col("x"));
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_inner_join_empty_left() {
        // Join(EmptyResult, Scan, INNER) → EmptyResult
        let plan = LogicalPlan::Join {
            left: Box::new(filter(scan("t"), lit_bool(false))),
            right: Box::new(scan("u")),
            kind: JoinKind::Inner,
            condition: None,
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_inner_join_empty_right() {
        // Join(Scan, EmptyResult, INNER) → EmptyResult
        let plan = LogicalPlan::Join {
            left: Box::new(scan("t")),
            right: Box::new(filter(scan("u"), lit_bool(false))),
            kind: JoinKind::Inner,
            condition: None,
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_left_join_empty_right_not_eliminated() {
        // Join(Scan, EmptyResult, LEFT) — NOT empty (left rows still appear with NULLs).
        let plan = LogicalPlan::Join {
            left: Box::new(scan("t")),
            right: Box::new(filter(scan("u"), lit_bool(false))),
            kind: JoinKind::Left,
            condition: None,
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        // Should remain a Join (not EmptyResult).
        assert!(matches!(&opt, OptimizedPlan::Join { .. }));
    }

    #[test]
    fn test_dce_empty_propagates_through_sort() {
        // Sort(EmptyResult) → EmptyResult
        let plan = LogicalPlan::Sort {
            input: Box::new(filter(scan("t"), lit_bool(false))),
            keys: vec![SortKey {
                expr: col("id"),
                ascending: true,
                nulls_first: None,
            }],
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_aggregate_not_eliminated_on_empty() {
        // Aggregate(EmptyResult) — NOT eliminated (COUNT(*) returns 0, not nothing).
        let plan = LogicalPlan::Aggregate {
            input: Box::new(filter(scan("t"), lit_bool(false))),
            group_by: vec![],
            aggregates: vec![AggregateItem {
                func: AggFunc::Count,
                arg: None,
                distinct: false,
                alias: Some("cnt".to_string()),
            }],
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert!(matches!(&opt, OptimizedPlan::Aggregate { .. }));
    }

    // =======================================================================
    // Pass 5: LimitPushdown tests
    // =======================================================================

    #[test]
    fn test_lpd_limit_scan_sets_scan_limit() {
        // Limit(Scan, 10, None) → Scan with scan_limit = Some(10)
        let plan = limit(scan("t"), 10, None);
        let opt = optimize_with_passes(plan, &[&LimitPushdownPass]);
        if let OptimizedPlan::Limit { input, .. } = opt {
            assert!(matches!(
                *input,
                OptimizedPlan::Scan {
                    scan_limit: Some(10),
                    ..
                }
            ));
        } else {
            panic!("expected Limit");
        }
    }

    #[test]
    fn test_lpd_limit_with_offset_adds_both() {
        // Limit(Scan, 10, Some(5)) → scan_limit = Some(15)
        let plan = limit(scan("t"), 10, Some(5));
        let opt = optimize_with_passes(plan, &[&LimitPushdownPass]);
        if let OptimizedPlan::Limit { input, .. } = opt {
            assert!(matches!(
                *input,
                OptimizedPlan::Scan {
                    scan_limit: Some(15),
                    ..
                }
            ));
        } else {
            panic!("expected Limit");
        }
    }

    #[test]
    fn test_lpd_limit_through_sort_to_scan() {
        // Limit(Sort(Scan, key), 10, None) → Sort(Scan { scan_limit: Some(10) })
        let plan = limit(sort(scan("t"), "name"), 10, None);
        let opt = optimize_with_passes(plan, &[&LimitPushdownPass]);
        if let OptimizedPlan::Limit { input, .. } = opt {
            if let OptimizedPlan::Sort { input: sort_inner, .. } = *input {
                assert!(matches!(
                    *sort_inner,
                    OptimizedPlan::Scan {
                        scan_limit: Some(10),
                        ..
                    }
                ));
            } else {
                panic!("expected Sort inside Limit");
            }
        } else {
            panic!("expected Limit");
        }
    }

    #[test]
    fn test_lpd_limit_not_pushed_through_filter() {
        // Limit(Filter(Scan, pred), 10, None)
        // Filter blocks pushdown — scan_limit should NOT be set.
        let pred = bin(BinaryOp::Gt, col("age"), lit_int(18));
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Filter {
                input: Box::new(scan("t")),
                predicate: pred,
            }),
            count: Some(10),
            offset: None,
        };
        let opt = optimize_with_passes(plan, &[&LimitPushdownPass]);
        if let OptimizedPlan::Limit { input, .. } = opt {
            if let OptimizedPlan::Filter { input: inner, .. } = *input {
                // The Scan inside Filter should NOT have a scan_limit.
                assert!(matches!(
                    *inner,
                    OptimizedPlan::Scan {
                        scan_limit: None,
                        ..
                    }
                ));
            } else {
                panic!("expected Filter inside Limit");
            }
        } else {
            panic!("expected Limit");
        }
    }

    #[test]
    fn test_lpd_preserves_tighter_existing_hint() {
        // Two nested Limits: the inner tighter one wins.
        // Limit(Limit(Scan, 3, None), 10, None) — inner limit of 3 is tighter.
        let plan = LogicalPlan::Limit {
            input: Box::new(limit(scan("t"), 3, None)),
            count: Some(10),
            offset: None,
        };
        let opt = optimize_with_passes(plan, &[&LimitPushdownPass]);
        // The scan_limit should be min(3, 10) = 3.
        fn find_scan_limit(plan: &OptimizedPlan) -> Option<i64> {
            match plan {
                OptimizedPlan::Scan { scan_limit, .. } => *scan_limit,
                OptimizedPlan::Limit { input, .. } => find_scan_limit(input),
                _ => None,
            }
        }
        let sl = find_scan_limit(&opt);
        assert_eq!(sl, Some(3));
    }

    // =======================================================================
    // Full pipeline tests (all five passes)
    // =======================================================================

    #[test]
    fn test_full_pipeline_filter_false_then_empty() {
        // Full optimize: Filter(Scan, 1 = 2) should become EmptyResult.
        // ConstantFolding turns 1=2 → FALSE, then DCE turns Filter(_, FALSE) → EmptyResult.
        let plan = filter(scan("t"), bin(BinaryOp::Eq, lit_int(1), lit_int(2)));
        let opt = optimize(plan);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_full_pipeline_limit_zero_empty() {
        // Limit(Scan, 0) → EmptyResult via DCE.
        let plan = limit(scan("t"), 0, None);
        let opt = optimize(plan);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_full_pipeline_sort_filter_pushdown_and_limit() {
        // Select * FROM t WHERE age > 18 ORDER BY name LIMIT 10
        // Plan: Limit(Sort(Filter(Scan, age > 18), name), 10)
        // After optimization:
        //   - PredicatePushdown: Filter stays below Sort (already there).
        //   - LimitPushdown: scan_limit = Some(10) set on Scan.
        let pred = bin(BinaryOp::Gt, col("age"), lit_int(18));
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Sort {
                input: Box::new(LogicalPlan::Filter {
                    input: Box::new(scan("t")),
                    predicate: pred,
                }),
                keys: vec![SortKey {
                    expr: col("name"),
                    ascending: true,
                    nulls_first: None,
                }],
            }),
            count: Some(10),
            offset: None,
        };
        let opt = optimize(plan);
        // Outer is still Limit.
        assert!(matches!(&opt, OptimizedPlan::Limit { count: Some(10), .. }));
    }

    #[test]
    fn test_full_pipeline_constant_fold_then_dce() {
        // Filter(Scan, TRUE AND FALSE) → EmptyResult.
        // ConstantFolding: TRUE AND FALSE → FALSE.
        // DCE: Filter(_, FALSE) → EmptyResult.
        let plan = filter(
            scan("t"),
            bin(BinaryOp::And, lit_bool(true), lit_bool(false)),
        );
        let opt = optimize(plan);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_full_pipeline_projection_pruning_with_filter() {
        // SELECT id FROM t WHERE age > 18
        // Plan: Project(Filter(Scan, age > 18), [col("id")])
        // Projection pruning: Scan should have required_columns containing "id" and "age".
        let plan = LogicalPlan::Project {
            input: Box::new(LogicalPlan::Filter {
                input: Box::new(scan("t")),
                predicate: bin(BinaryOp::Gt, col("age"), lit_int(18)),
            }),
            columns: vec![OutputColumn {
                expr: col("id"),
                alias: None,
            }],
        };
        let opt = optimize(plan);
        // The optimizer may push the filter, but the scan should have required_columns.
        fn find_scan(plan: &OptimizedPlan) -> Option<&OptimizedPlan> {
            match plan {
                s @ OptimizedPlan::Scan { .. } => Some(s),
                OptimizedPlan::Filter { input, .. }
                | OptimizedPlan::Project { input, .. }
                | OptimizedPlan::Sort { input, .. }
                | OptimizedPlan::Limit { input, .. } => find_scan(input),
                _ => None,
            }
        }
        let scan = find_scan(&opt).expect("no Scan found");
        if let OptimizedPlan::Scan {
            required_columns: Some(cols),
            ..
        } = scan
        {
            assert!(cols.contains(&"id".to_string()));
        }
        // (age may or may not be in required_columns depending on push order)
    }

    #[test]
    fn test_optimize_with_no_passes() {
        // optimize_with_passes with empty slice = just lift.
        let plan = scan("t");
        let opt = optimize_with_passes(plan, &[]);
        assert_eq!(opt, opt_scan("t"));
    }

    #[test]
    fn test_default_passes_returns_five() {
        let passes = default_passes();
        assert_eq!(passes.len(), 5);
        assert_eq!(passes[0].name(), "ConstantFolding");
        assert_eq!(passes[1].name(), "PredicatePushdown");
        assert_eq!(passes[2].name(), "ProjectionPruning");
        assert_eq!(passes[3].name(), "DeadCodeElimination");
        assert_eq!(passes[4].name(), "LimitPushdown");
    }

    #[test]
    fn test_distinct_plan_lifted() {
        let plan = LogicalPlan::Distinct(Box::new(scan("t")));
        let opt = optimize(plan);
        assert!(matches!(&opt, OptimizedPlan::Distinct(_)));
    }

    #[test]
    fn test_union_plan_lifted() {
        let plan = LogicalPlan::Union {
            left: Box::new(scan("a")),
            right: Box::new(scan("b")),
            all: true,
        };
        let opt = optimize(plan);
        assert!(matches!(&opt, OptimizedPlan::Union { all: true, .. }));
    }

    #[test]
    fn test_aggregate_plan_lifted() {
        let plan = LogicalPlan::Aggregate {
            input: Box::new(scan("t")),
            group_by: vec![col("dept")],
            aggregates: vec![AggregateItem {
                func: AggFunc::Sum,
                arg: Some(col("salary")),
                distinct: false,
                alias: Some("total".to_string()),
            }],
        };
        let opt = optimize(plan);
        assert!(matches!(&opt, OptimizedPlan::Aggregate { .. }));
    }

    #[test]
    fn test_having_plan_lifted() {
        let plan = LogicalPlan::Having {
            input: Box::new(scan("t")),
            predicate: bin(BinaryOp::Gt, col("total"), lit_int(1000)),
        };
        let opt = optimize(plan);
        assert!(matches!(&opt, OptimizedPlan::Having { .. }));
    }

    #[test]
    fn test_join_inner_lifted() {
        let plan = LogicalPlan::Join {
            left: Box::new(scan("a")),
            right: Box::new(scan("b")),
            kind: JoinKind::Inner,
            condition: Some(bin(BinaryOp::Eq, col("a.id"), col("b.id"))),
        };
        let opt = optimize(plan);
        assert!(matches!(
            &opt,
            OptimizedPlan::Join {
                kind: JoinKind::Inner,
                ..
            }
        ));
    }

    #[test]
    fn test_cf_int_mod() {
        // 10 % 3 → Int(1)
        let e = bin(BinaryOp::Mod, lit_int(10), lit_int(3));
        assert_eq!(fold(e), lit_int(1));
    }

    #[test]
    fn test_cf_lt_comparison() {
        // 3 < 5 → TRUE
        let e = bin(BinaryOp::Lt, lit_int(3), lit_int(5));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_gt_comparison() {
        // 5 > 3 → TRUE
        let e = bin(BinaryOp::Gt, lit_int(5), lit_int(3));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_gte_equal() {
        // 5 >= 5 → TRUE
        let e = bin(BinaryOp::Gte, lit_int(5), lit_int(5));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_neq() {
        // 1 != 2 → TRUE
        let e = bin(BinaryOp::Neq, lit_int(1), lit_int(2));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_and_true_true() {
        // TRUE AND TRUE → TRUE
        let e = bin(BinaryOp::And, lit_bool(true), lit_bool(true));
        assert_eq!(fold(e), lit_bool(true));
    }

    #[test]
    fn test_cf_or_false_false() {
        // FALSE OR FALSE → FALSE
        let e = bin(BinaryOp::Or, lit_bool(false), lit_bool(false));
        assert_eq!(fold(e), lit_bool(false));
    }

    #[test]
    fn test_dce_distinct_on_empty() {
        // Distinct(Filter(Scan, FALSE)) → EmptyResult
        let plan = LogicalPlan::Distinct(Box::new(filter(scan("t"), lit_bool(false))));
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_dce_having_on_empty() {
        // Having(Filter(Scan, FALSE), pred) → EmptyResult
        let plan = LogicalPlan::Having {
            input: Box::new(filter(scan("t"), lit_bool(false))),
            predicate: lit_bool(true),
        };
        let opt = optimize_with_passes(plan, &[&DeadCodeEliminationPass]);
        assert_eq!(opt, OptimizedPlan::EmptyResult);
    }

    #[test]
    fn test_ppd_no_filter_no_change() {
        // A plan without any Filter should be unchanged by PredicatePushdown.
        let plan = sort(scan("t"), "name");
        let opt = optimize_with_passes(plan, &[&PredicatePushdownPass]);
        assert!(matches!(&opt, OptimizedPlan::Sort { .. }));
    }

    #[test]
    fn test_lpd_offset_only_no_pushdown() {
        // Limit(Scan, None, Some(5)) — no count, no pushdown.
        let plan = LogicalPlan::Limit {
            input: Box::new(scan("t")),
            count: None,
            offset: Some(5),
        };
        let opt = optimize_with_passes(plan, &[&LimitPushdownPass]);
        if let OptimizedPlan::Limit { input, .. } = opt {
            assert!(matches!(
                *input,
                OptimizedPlan::Scan {
                    scan_limit: None,
                    ..
                }
            ));
        }
    }
}
