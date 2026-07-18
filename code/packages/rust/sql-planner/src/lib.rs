//! # SQL Planner — Logical Query Planning for Mini-SQLite (Level 1)
//!
//! This crate is the **third stage** of the Mini-SQLite SQL pipeline:
//!
//! ```text
//! sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
//! ```
//!
//! The planner accepts a parsed SQL AST ([`GrammarASTNode`] from `sql-parser`)
//! and a [`SchemaProvider`] (which can tell us what columns a table has), then
//! produces a **[`LogicalPlan`]** tree — a pure in-memory description of *what*
//! the query does, without doing any actual data access.
//!
//! ## Why a separate planner?
//!
//! Splitting planning from execution follows the classic database textbook
//! architecture (Ramakrishnan & Gehrke, "Database Management Systems"):
//!
//! ```text
//! Parse → Plan → Optimize → Execute
//! ```
//!
//! The optimizer (next stage) works entirely on `LogicalPlan` trees: it can
//! push filters down, reorder joins, choose indexes — all without knowing about
//! physical storage.  The planner's job is only to produce a *correct* tree;
//! making it *efficient* is the optimizer's job.
//!
//! ## Plan node ordering for SELECT
//!
//! SQL's logical evaluation order (which the plan tree reflects) is:
//!
//! ```text
//! 1. Scan         — read a table
//! 2. Join         — combine multiple tables
//! 3. Filter       — apply WHERE predicate
//! 4. Aggregate    — GROUP BY + aggregate functions
//! 5. Having       — filter on aggregate results
//! 6. Distinct     — remove duplicate rows
//! 7. Sort         — ORDER BY
//! 8. Limit        — LIMIT / OFFSET
//! 9. Project      — SELECT list (OUTERMOST — applied last)
//! ```
//!
//! **Critical**: `Project` is always the outermost node (lessons.md: "SQL query
//! planner: `Project` must be the OUTERMOST (last) step in `planSelect`").
//! This is counterintuitive because SELECT appears first in the SQL text, but
//! projection happens after sorting and pagination.

use coding_adventures_sql_backend::{ColumnDef, SchemaProvider, SqlValue};
use coding_adventures_sql_parser::parse_sql;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ===========================================================================
// Expression types
// ===========================================================================

/// A SQL expression that can appear in SELECT lists, WHERE predicates,
/// ORDER BY clauses, etc.
///
/// ## Precedence hierarchy (low to high)
///
/// ```text
/// OR  →  AND  →  NOT  →  comparison  →  additive  →  multiplicative  →  unary  →  primary
/// ```
///
/// The grammar encodes this hierarchy through nested rules; the planner
/// mirrors it here as a single recursive enum.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlExpr {
    /// A literal constant value: numbers, strings, booleans, NULL.
    ///
    /// Examples: `42`, `'hello'`, `TRUE`, `NULL`
    Literal(SqlValue),

    /// A reference to a column, optionally qualified with a table name.
    ///
    /// - `name` → `Column { table: None, name: "name" }`
    /// - `u.name` → `Column { table: Some("u"), name: "name" }`
    Column {
        table: Option<String>,
        name: String,
    },

    /// A binary operator applied to two operands.
    ///
    /// Examples: `a + b`, `x = 1`, `age > 18`
    BinaryOp {
        op: BinaryOp,
        left: Box<SqlExpr>,
        right: Box<SqlExpr>,
    },

    /// A unary operator applied to one operand.
    ///
    /// Examples: `-x`, `NOT done`
    UnaryOp {
        op: UnaryOp,
        expr: Box<SqlExpr>,
    },

    /// `expr IS NULL` — tests for null value.
    IsNull(Box<SqlExpr>),

    /// `expr IS NOT NULL` — tests for non-null value.
    IsNotNull(Box<SqlExpr>),

    /// `value BETWEEN low AND high` — inclusive range test.
    ///
    /// `negated = true` for `NOT BETWEEN`.
    Between {
        value: Box<SqlExpr>,
        low: Box<SqlExpr>,
        high: Box<SqlExpr>,
        negated: bool,
    },

    /// `value LIKE pattern [ESCAPE ch]` — SQL pattern matching.
    ///
    /// `negated = true` for `NOT LIKE`. `escape` carries the optional
    /// `ESCAPE ch` operand (an expression, usually a one-character string
    /// literal); when present, that character makes a following `%`, `_`, or
    /// escape character itself a literal in the pattern.
    Like {
        value: Box<SqlExpr>,
        pattern: Box<SqlExpr>,
        negated: bool,
        escape: Option<Box<SqlExpr>>,
    },

    /// `value IN (v1, v2, ...)` — membership test.
    ///
    /// `negated = true` for `NOT IN`.
    InList {
        value: Box<SqlExpr>,
        list: Vec<SqlExpr>,
        negated: bool,
    },

    /// A scalar function call: `func(arg1, arg2, ...)` or `func(*)`.
    ///
    /// Aggregate functions (COUNT, SUM, etc.) are represented by [`SqlExpr::Aggregate`].
    FunctionCall {
        name: String,
        args: Vec<SqlExpr>,
        /// `star = true` for `func(*)` — the argument is an asterisk.
        star: bool,
    },

    /// `CAST(expr AS type)` — an explicit type conversion.
    ///
    /// Example: `CAST('12' AS INTEGER)` → the integer `12`.
    Cast {
        expr: Box<SqlExpr>,
        ty: CastType,
    },

    /// A searched `CASE WHEN cond THEN val … [ELSE val] END` expression.
    ///
    /// The `branches` are evaluated top-to-bottom; the value of the first branch
    /// whose condition is truthy (non-zero, non-NULL) is the result. If none
    /// match, the result is `else_val` if present, otherwise `NULL`. Later
    /// branches are NOT evaluated once one matches (short-circuit).
    Case {
        branches: Vec<(SqlExpr, SqlExpr)>,
        else_val: Option<Box<SqlExpr>>,
    },

    /// An aggregate function applied within GROUP BY context.
    ///
    /// Aggregates are separated from regular function calls because they
    /// have distinct semantics: they operate on groups of rows, not single rows.
    ///
    /// Examples: `COUNT(*)`, `SUM(price)`, `AVG(score) DISTINCT`
    Aggregate {
        func: AggFunc,
        /// The column or expression to aggregate (None for `COUNT(*)`).
        arg: Option<Box<SqlExpr>>,
        /// `DISTINCT` modifier (e.g., `COUNT(DISTINCT x)`).
        distinct: bool,
    },
}

/// A binary operator.
///
/// Grouped by semantic category for clarity.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison — SQL treats all these as returning boolean
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    // Logical (for AND/OR at the expression level, below NOT)
    And,
    Or,
    // String concatenation
    Concat,
    // Bitwise — operands coerced to integer, NULL-propagating
    BitAnd,
    BitOr,
    ShiftLeft,
    ShiftRight,
}

/// A unary operator.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    /// Arithmetic negation: `-x`
    Neg,
    /// Logical negation: `NOT x`
    Not,
    /// Bitwise complement: `~x` (operand coerced to integer, NULL-propagating).
    BitNot,
}

/// The target type of a `CAST(expr AS type)` conversion.
///
/// SQLite resolves any declared type name to one of five "affinities"; this
/// enum covers the three whose CAST results compare exactly or within the
/// oracle's numeric epsilon (INTEGER, REAL, TEXT). BLOB and NUMERIC are not
/// yet supported (a later increment) — see the planner's cast parser.
#[derive(Debug, Clone, PartialEq)]
pub enum CastType {
    /// `CAST(x AS INTEGER)` — truncate reals toward zero; parse a leading
    /// numeric prefix of text (`'12abc'` → 12, `'abc'` → 0).
    Integer,
    /// `CAST(x AS REAL)` — parse a leading numeric prefix of text as f64.
    Real,
    /// `CAST(x AS TEXT)` — render the value as its text representation.
    Text,
    /// `CAST(x AS NUMERIC)` — SQLite's NUMERIC affinity, and the *default*
    /// affinity for any type name that is not INTEGER/TEXT/REAL/BLOB (e.g.
    /// `NUMERIC`, `DECIMAL`, `BOOLEAN`, `DATE`). An INTEGER stays INTEGER and a
    /// REAL stays REAL (the cast is a no-op on numbers — `CAST(3.0 AS NUMERIC)`
    /// is `3.0`, not `3`). Text/blob is parsed to a number, preferring INTEGER
    /// when the value is integral and fits i64 (`'3.0'`→`3`, `'1e3'`→`1000`),
    /// otherwise REAL (`'3.5'`→`3.5`, an i64-overflowing integer→real).
    Numeric,
}

/// An aggregate function name.
///
/// These are the five standard SQL aggregate functions.
#[derive(Debug, Clone, PartialEq)]
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

// ===========================================================================
// Plan node types
// ===========================================================================

/// A logical query plan node.
///
/// A `LogicalPlan` is a *tree* where each node's `input` is its child.
/// The root of the tree is the last operation applied to the data.
///
/// For a `SELECT * FROM users WHERE age > 18 ORDER BY name LIMIT 10`:
///
/// ```text
/// Project { columns: [*], input:
///   Limit { count: 10, offset: None, input:
///     Sort { keys: [name ASC], input:
///       Filter { predicate: age > 18, input:
///         Scan { table: "users" }
///       }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalPlan {
    /// A full table scan — reads every row from a table.
    ///
    /// This is always a leaf node (no input).  The optimizer may later
    /// replace this with an `IndexScan` if a suitable index exists.
    Scan {
        table: String,
        /// Optional alias for the table (from `FROM t AS u`).
        alias: Option<String>,
    },

    /// A row filter — keeps only rows where `predicate` evaluates to true.
    ///
    /// Corresponds to the `WHERE` clause (or the `ON` clause of a JOIN).
    Filter {
        input: Box<LogicalPlan>,
        predicate: SqlExpr,
    },

    /// A column projection — selects and renames columns from its input.
    ///
    /// This node is *always* the **outermost** (root) node of a SELECT plan,
    /// because SQL's logical evaluation applies the SELECT list last.
    Project {
        input: Box<LogicalPlan>,
        columns: Vec<OutputColumn>,
    },

    /// A join between two plan inputs.
    ///
    /// The `condition` is the `ON expr` predicate.  For cross joins it is None.
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        kind: JoinKind,
        condition: Option<SqlExpr>,
    },

    /// An aggregation step: `GROUP BY` columns + aggregate computations.
    ///
    /// If `group_by` is empty this is a global aggregate (e.g. `SELECT COUNT(*)`).
    Aggregate {
        input: Box<LogicalPlan>,
        group_by: Vec<SqlExpr>,
        aggregates: Vec<AggregateItem>,
    },

    /// A `HAVING` predicate applied after grouping.
    Having {
        input: Box<LogicalPlan>,
        predicate: SqlExpr,
    },

    /// An `ORDER BY` sort on one or more keys.
    Sort {
        input: Box<LogicalPlan>,
        keys: Vec<SortKey>,
    },

    /// `LIMIT count OFFSET offset` pagination.
    ///
    /// Both fields are optional:
    /// - `count = None` means no row limit.
    /// - `offset = None` means start from the first row.
    Limit {
        input: Box<LogicalPlan>,
        count: Option<i64>,
        offset: Option<i64>,
    },

    /// `SELECT DISTINCT` — removes duplicate rows from the output.
    Distinct(Box<LogicalPlan>),

    /// `UNION [ALL]` of two plan subtrees.
    Union {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        /// `all = true` keeps duplicates (UNION ALL); `false` deduplicates.
        all: bool,
    },

    /// `INSERT INTO table [(cols)] VALUES (...)`.
    Insert {
        table: String,
        /// The explicit column list from `INSERT INTO t (a, b, c)`.
        /// If None the columns come from the table schema in order.
        columns: Option<Vec<String>>,
        source: InsertSource,
    },

    /// `UPDATE table SET col = expr, ... [WHERE ...]`.
    Update {
        table: String,
        assignments: Vec<Assignment>,
        predicate: Option<SqlExpr>,
    },

    /// `DELETE FROM table [WHERE ...]`.
    Delete {
        table: String,
        predicate: Option<SqlExpr>,
    },

    /// `CREATE TABLE [IF NOT EXISTS] table (col_def, ...)`.
    CreateTable {
        table: String,
        if_not_exists: bool,
        columns: Vec<ColumnDef>,
    },

    /// `DROP TABLE [IF EXISTS] table`.
    DropTable {
        table: String,
        if_exists: bool,
    },
}

/// A column in a `Project` node — an expression with an optional alias.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputColumn {
    pub expr: SqlExpr,
    /// The alias used in the output column name (`SELECT expr AS alias`).
    pub alias: Option<String>,
}

/// The kind of SQL join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinKind {
    /// `INNER JOIN` (or plain `JOIN`) — only matching rows.
    Inner,
    /// `LEFT [OUTER] JOIN` — all left rows, matching right rows or NULL.
    Left,
    /// `RIGHT [OUTER] JOIN` — all right rows, matching left rows or NULL.
    Right,
    /// `FULL [OUTER] JOIN` — all rows from both sides.
    Full,
    /// `CROSS JOIN` — Cartesian product.
    Cross,
}

/// A sort key in an `ORDER BY` clause.
#[derive(Debug, Clone, PartialEq)]
pub struct SortKey {
    pub expr: SqlExpr,
    /// `ascending = true` for ASC (default), `false` for DESC.
    pub ascending: bool,
    /// Explicit NULL placement from a `NULLS FIRST` / `NULLS LAST` clause.
    /// `None` = SQLite's default (NULLs sort first for ASC, last for DESC);
    /// `Some(true)` = NULLs first; `Some(false)` = NULLs last.
    pub nulls_first: Option<bool>,
    /// Collating sequence from a `COLLATE name` clause, applied to **text**
    /// values before comparison. `None` (or an explicit `COLLATE BINARY`) means
    /// the default byte-order comparison. `Some("NOCASE")` compares ASCII
    /// case-insensitively; `Some("RTRIM")` ignores trailing spaces. Non-text
    /// values are unaffected by collation. Stored uppercased.
    pub collation: Option<String>,
}

/// An aggregate item inside an `Aggregate` plan node.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateItem {
    pub func: AggFunc,
    /// The column or expression being aggregated (None for COUNT(*)).
    pub arg: Option<SqlExpr>,
    pub distinct: bool,
    /// Optional alias for this aggregate in the output (from `AS name`).
    pub alias: Option<String>,
}

/// A single `column = expr` assignment in an UPDATE statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub column: String,
    pub value: SqlExpr,
}

/// The data source for an INSERT statement.
#[derive(Debug, Clone, PartialEq)]
pub enum InsertSource {
    /// `VALUES (row1), (row2), ...`
    Values(Vec<Vec<SqlExpr>>),
}

// ===========================================================================
// Error type
// ===========================================================================

/// Errors that can occur during logical planning.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanError {
    /// A table referenced in the query is not in the schema.
    UnknownTable(String),
    /// A column referenced in the query is not in the schema.
    UnknownColumn(String),
    /// A SQL construct that the planner does not yet support.
    UnsupportedStatement(String),
    /// The SQL source text failed to parse.
    ParseError(String),
    /// A column reference is ambiguous (same name in multiple joined tables).
    AmbiguousColumn(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::UnknownTable(t) => write!(f, "unknown table: {t:?}"),
            PlanError::UnknownColumn(c) => write!(f, "unknown column: {c:?}"),
            PlanError::UnsupportedStatement(s) => write!(f, "unsupported statement: {s}"),
            PlanError::ParseError(s) => write!(f, "parse error: {s}"),
            PlanError::AmbiguousColumn(c) => write!(f, "ambiguous column: {c:?}"),
        }
    }
}

impl std::error::Error for PlanError {}

// ===========================================================================
// Public API
// ===========================================================================

/// Plan a SQL string by parsing it first, then planning the first statement.
///
/// # Errors
///
/// Returns `PlanError::ParseError` if the SQL fails to parse.
/// Returns other `PlanError` variants if the AST cannot be planned.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_sql_planner::{plan_sql, LogicalPlan};
/// use coding_adventures_sql_backend::SchemaProvider;
///
/// // ... implement SchemaProvider ...
/// ```
pub fn plan_sql(sql: &str, schema: &dyn SchemaProvider) -> Result<LogicalPlan, PlanError> {
    let ast = parse_sql(sql).map_err(PlanError::ParseError)?;
    plan(&ast, schema)
}

/// Plan a pre-parsed SQL AST.
///
/// The `ast` should be a `program` node from `parse_sql`.  The planner
/// extracts the first `statement` and plans it.
///
/// # Errors
///
/// - `PlanError::UnsupportedStatement` if there are no statements.
/// - Other variants for schema/unsupported-construct errors.
pub fn plan(ast: &GrammarASTNode, schema: &dyn SchemaProvider) -> Result<LogicalPlan, PlanError> {
    // The AST root is "program"; find the first "statement" child.
    //
    // Grammar: program = statement { ";" statement } [ ";" ]
    let statement = find_node(ast, "statement").ok_or_else(|| {
        PlanError::UnsupportedStatement("no statement found in program".to_string())
    })?;
    plan_statement(statement, schema)
}

/// Plan a standalone expression node (useful for testing expression planning).
///
/// The `node` should be any expression-level rule: `expr`, `or_expr`,
/// `and_expr`, `comparison`, `additive`, `primary`, etc.
pub fn plan_expr(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    plan_expression(node)
}

// ===========================================================================
// Statement dispatch
// ===========================================================================

/// Dispatch to the appropriate statement planner.
///
/// Grammar: statement = query_stmt | insert_stmt | update_stmt | delete_stmt
///                     | create_table_stmt | drop_table_stmt
/// Grammar: query_stmt = [ with_clause ] ( values_stmt | select_stmt ) ...
fn plan_statement(
    node: &GrammarASTNode,
    schema: &dyn SchemaProvider,
) -> Result<LogicalPlan, PlanError> {
    // The "statement" node wraps exactly one concrete statement.
    // In the current sql.grammar, SELECT/VALUES are nested under a `query_stmt`
    // node:  statement → query_stmt → select_stmt.  DML/DDL are direct children.
    // Walk the children to find which kind it is.
    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            match child_node.rule_name.as_str() {
                // Direct children (DML/DDL).
                "select_stmt" => return plan_select(child_node, schema),
                "insert_stmt" => return plan_insert(child_node, schema),
                "update_stmt" => return plan_update(child_node),
                "delete_stmt" => return plan_delete(child_node),
                "create_table_stmt" => return plan_create_table(child_node),
                "drop_table_stmt" => return plan_drop_table(child_node),
                // The current grammar wraps SELECT/VALUES in query_stmt.
                // Recurse through the query_stmt layer transparently.
                "query_stmt" => {
                    for qchild in &child_node.children {
                        if let ASTNodeOrToken::Node(qn) = qchild {
                            if qn.rule_name == "select_stmt" {
                                return plan_select(qn, schema);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // If we reach here the statement contains no recognized sub-statement.
    // This can happen if the grammar grows a new statement type the planner
    // doesn't yet handle.
    Err(PlanError::UnsupportedStatement(format!(
        "unrecognized statement node: {:?}",
        node.rule_name
    )))
}

// ===========================================================================
// SELECT planning
// ===========================================================================

/// Plan a `select_stmt` node into a `LogicalPlan` tree.
///
/// ## Pipeline (bottom-up, innermost-first)
///
/// 1. `Scan` — from the first `table_ref`
/// 2. `Join` — for each `join_clause`
/// 3. `Filter` — if `where_clause` is present
/// 4. `Aggregate` — if `group_clause` or aggregates present
/// 5. `Having` — if `having_clause` present
/// 6. `Distinct` — if `DISTINCT` keyword present
/// 7. `Sort` — if `order_clause` present
/// 8. `Limit` — if `limit_clause` present
/// 9. `Project` — ALWAYS outermost; wraps the entire pipeline
///
/// ## Why Project is last
///
/// SQL's SELECT list is evaluated AFTER sorting and pagination:
/// `SELECT name FROM users ORDER BY name LIMIT 10` must first sort
/// ALL users by name (possibly on a column not in the output), then
/// take the first 10, THEN project down to just `name`.  If we projected
/// first we'd lose the sort key.
fn plan_select(
    stmt: &GrammarASTNode,
    schema: &dyn SchemaProvider,
) -> Result<LogicalPlan, PlanError> {
    // -----------------------------------------------------------------------
    // Step 1: Build the base from the FROM clause.
    // -----------------------------------------------------------------------
    // Grammar: select_stmt = SELECT [ DISTINCT | ALL ] select_list
    //                        [ FROM table_ref { join_clause } ]
    //                        [ where_clause ] ...
    //
    // When there is no FROM clause (e.g. `SELECT LENGTH('hello') AS n`),
    // SQLite evaluates the SELECT list exactly once against an implicit
    // single-row, no-column virtual table (the "dual" table).  We model
    // this as a `Scan` of a special `__dual__` table that the backend and
    // codegen handle as yielding one empty row.
    // Records the single base table (real name + optional alias) so ORDER BY can
    // resolve a bare column's schema-defined COLLATE. Left `None` for the dual
    // table and (below) for any query with JOINs, where column→table resolution
    // is ambiguous and out of scope for this pass.
    let mut base_table_ref: Option<(String, Option<String>)> = None;

    let mut plan: LogicalPlan = if let Some(table_ref) = find_node(stmt, "table_ref") {
        // Extract table name and optional alias from `table_ref = table_name [ "AS" NAME ]`.
        let (table_name, table_alias) = extract_table_ref(table_ref);

        // Validate the table exists in the schema.
        schema
            .column_names(&table_name)
            .map_err(|_| PlanError::UnknownTable(table_name.clone()))?;

        base_table_ref = Some((table_name.clone(), table_alias.clone()));

        LogicalPlan::Scan {
            table: table_name,
            alias: table_alias,
        }
    } else {
        // No FROM clause — use the implicit single-row dual table.
        // This is never looked up in the schema; the VM handles it specially.
        LogicalPlan::Scan {
            table: "__dual__".to_string(),
            alias: None,
        }
    };

    // -----------------------------------------------------------------------
    // Step 2: Apply any JOIN clauses.
    // -----------------------------------------------------------------------
    // Grammar: join_clause = join_type JOIN table_ref ON expr
    //
    // Each join wraps the current plan as the LEFT side and creates a new
    // Scan for the RIGHT side.  Multiple joins chain left-to-right.
    let join_clauses = find_nodes(stmt, "join_clause");
    for join_clause in join_clauses {
        plan = plan_join_clause(plan, join_clause, schema)?;
    }

    // -----------------------------------------------------------------------
    // Step 3: Apply WHERE predicate.
    // -----------------------------------------------------------------------
    if let Some(where_clause) = find_node(stmt, "where_clause") {
        // Grammar: where_clause = WHERE expr
        // The WHERE clause wraps an expr node; we skip the WHERE keyword token.
        let mut predicate = extract_clause_expr(where_clause)?;
        // Column-defined COLLATE flows into WHERE comparisons only for a single
        // base table (no JOINs), matching the ORDER BY restriction — with joins,
        // column→table resolution is ambiguous and out of scope for this pass.
        if find_nodes(stmt, "join_clause").is_empty() {
            if let Some((table, alias)) = base_table_ref.as_ref() {
                let ctx = build_collate_ctx(schema, table, alias.as_deref());
                predicate = collate_comparisons(predicate, &ctx);
            }
        }
        plan = LogicalPlan::Filter {
            input: Box::new(plan),
            predicate,
        };
    }

    // -----------------------------------------------------------------------
    // Step 4: Apply GROUP BY + aggregation.
    // -----------------------------------------------------------------------
    // Grammar: group_clause = GROUP BY column_ref { "," column_ref }
    //
    // We collect two things:
    // a) The GROUP BY key expressions.
    // b) Any aggregate function calls from the SELECT list and HAVING clause.
    let group_clause = find_node(stmt, "group_clause");
    let having_clause = find_node(stmt, "having_clause");
    let select_list = find_node(stmt, "select_list");

    // Collect aggregates from the SELECT list and HAVING clause.
    // For the SELECT list, we walk `select_item` nodes and capture the AS alias
    // so that the Aggregate plan carries the correct output column names.
    let mut aggregates: Vec<AggregateItem> = Vec::new();
    if let Some(sl) = select_list {
        collect_aggregates_with_aliases(sl, &mut aggregates);
    }
    if let Some(hc) = having_clause {
        let mut having_aggs: Vec<AggregateItem> = Vec::new();
        collect_aggregates_from_node(hc, &mut having_aggs);
        // Only add HAVING aggregates that are NOT already present in the
        // SELECT-list aggregates.  When COUNT(*) appears in both HAVING and
        // SELECT, they refer to the *same* computed value (slot 0).  Adding a
        // duplicate would allocate a new slot and emit an extra output column
        // named "agg_N" that should not be visible to the caller.
        for hagg in having_aggs {
            let already = aggregates.iter().any(|a| {
                a.func == hagg.func
                    && a.arg == hagg.arg
                    && a.distinct == hagg.distinct
            });
            if !already {
                aggregates.push(hagg);
            }
        }
    }

    // Build an Aggregate node if there's a GROUP BY or any aggregate calls.
    if group_clause.is_some() || !aggregates.is_empty() {
        let group_by = if let Some(gc) = group_clause {
            plan_group_by_exprs(gc)?
        } else {
            Vec::new()
        };
        plan = LogicalPlan::Aggregate {
            input: Box::new(plan),
            group_by,
            aggregates,
        };

        // Step 5: Apply HAVING predicate (after aggregation).
        if let Some(hc) = having_clause {
            let having_pred = extract_clause_expr(hc)?;
            plan = LogicalPlan::Having {
                input: Box::new(plan),
                predicate: having_pred,
            };
        }
    }

    // -----------------------------------------------------------------------
    // Step 6: DISTINCT (if present).
    // -----------------------------------------------------------------------
    // Grammar: select_stmt = SELECT [ DISTINCT | ALL ] ...
    // We check for the DISTINCT keyword token among the direct children of select_stmt.
    if has_token(stmt, "DISTINCT") {
        plan = LogicalPlan::Distinct(Box::new(plan));
    }

    // -----------------------------------------------------------------------
    // Projected output columns — computed HERE (before ORDER BY) so a
    // positional `ORDER BY <n>` can resolve the integer to the n-th output
    // expression. The same list is reused for the outermost Project (step 9),
    // so `plan_select_list` runs exactly once.
    // -----------------------------------------------------------------------
    let output_columns: Vec<OutputColumn> = if let Some(sl) = select_list {
        plan_select_list(sl)?
    } else {
        // Fallback: treat as SELECT * (shouldn't happen with a valid grammar).
        vec![OutputColumn {
            expr: SqlExpr::Column {
                table: None,
                name: "*".to_string(),
            },
            alias: None,
        }]
    };

    // -----------------------------------------------------------------------
    // Step 7: ORDER BY.
    // -----------------------------------------------------------------------
    if let Some(order_clause) = find_node(stmt, "order_clause") {
        // Column-defined COLLATE only propagates into ORDER BY for a single
        // base table (no JOINs) — see `base_table_ref`. With JOINs present we
        // pass `None`, so every sort key keeps whatever explicit collation it
        // carried and otherwise falls back to BINARY.
        let collate_ctx = if find_nodes(stmt, "join_clause").is_empty() {
            base_table_ref.as_ref().map(|(t, a)| (t.as_str(), a.as_deref()))
        } else {
            None
        };
        let keys = plan_order_by(order_clause, schema, collate_ctx, &output_columns)?;
        plan = LogicalPlan::Sort {
            input: Box::new(plan),
            keys,
        };
    }

    // -----------------------------------------------------------------------
    // Step 8: LIMIT / OFFSET.
    // -----------------------------------------------------------------------
    if let Some(limit_clause) = find_node(stmt, "limit_clause") {
        let (count, offset) = plan_limit(limit_clause)?;
        plan = LogicalPlan::Limit {
            input: Box::new(plan),
            count,
            offset,
        };
    }

    // -----------------------------------------------------------------------
    // Step 9: PROJECT — ALWAYS the outermost node.
    // -----------------------------------------------------------------------
    // From lessons.md: "Project must be the OUTERMOST (last) step in planSelect."
    //
    // The SELECT list tells us which columns (or expressions) to include in the
    // output.  This wrapping happens AFTER sort/limit so those operations can
    // still access columns that aren't in the final output.  The list was
    // already computed above (before ORDER BY) so positional sort keys could
    // resolve against it; reuse it here rather than re-planning.
    plan = LogicalPlan::Project {
        input: Box::new(plan),
        columns: output_columns,
    };

    Ok(plan)
}

// ---------------------------------------------------------------------------
// SELECT helpers
// ---------------------------------------------------------------------------

/// Extract the table name and optional alias from a `table_ref` node.
///
/// Grammar: `table_ref = table_name [ [ "AS" ] NAME ]` — the alias may be
/// written with or without `AS` (`FROM users AS u` and `FROM users u` are
/// equivalent in SQLite).
/// Grammar: `table_name = NAME [ "." NAME ]`
///
/// Returns `(table_name, Option<alias>)`.
fn extract_table_ref(table_ref: &GrammarASTNode) -> (String, Option<String>) {
    // Try to find a `table_name` child node first.
    // If no name token is found, we return an empty string.  The caller
    // immediately validates the table against the schema, so an empty string
    // will produce `PlanError::UnknownTable("")` — a visible, safe error.
    let table_name_node = find_node(table_ref, "table_name");
    let table_name = if let Some(tn) = table_name_node {
        first_name_token(tn).unwrap_or_default()
    } else {
        first_name_token(table_ref).unwrap_or_default()
    };

    // Explicit `AS name`: scan for the AS keyword, take the following token.
    let mut alias: Option<String> = None;
    let children = &table_ref.children;
    for (i, child) in children.iter().enumerate() {
        if is_keyword_token(child, "AS") {
            // The token after "AS" is the alias name.
            if let Some(next) = children.get(i + 1) {
                alias = Some(token_text_of(next));
            }
        }
    }

    // Implicit `name` (no AS): the table name lives in its own nested
    // `table_name` node, so any bare `Name`-type token directly under
    // `table_ref` is the alias (`FROM users u`). Guard on the `table_name`
    // node being present — in the degenerate fallback above the lone direct
    // token IS the table name and must not be doubled up as its own alias.
    if alias.is_none() && table_name_node.is_some() {
        for child in children {
            if let ASTNodeOrToken::Token(tok) = child {
                if tok.type_ == lexer::token::TokenType::Name {
                    alias = Some(tok.value.clone());
                    break;
                }
            }
        }
    }

    (table_name, alias)
}

/// Plan a `join_clause` node and wrap the current plan as the LEFT side.
///
/// Grammar: `join_clause = join_type JOIN table_ref ON expr`
/// Grammar: `join_type = CROSS | INNER | LEFT [OUTER] | RIGHT [OUTER] | FULL [OUTER]`
fn plan_join_clause(
    left: LogicalPlan,
    join_clause: &GrammarASTNode,
    schema: &dyn SchemaProvider,
) -> Result<LogicalPlan, PlanError> {
    // Determine join kind from the join_type child node.
    let kind = if let Some(jt) = find_node(join_clause, "join_type") {
        extract_join_kind(jt)
    } else {
        // Plain "JOIN" without an explicit type keyword = INNER.
        JoinKind::Inner
    };

    // Get the right-hand table.
    let table_ref = find_node(join_clause, "table_ref").ok_or_else(|| {
        PlanError::UnsupportedStatement("join_clause without table_ref".to_string())
    })?;
    let (table_name, table_alias) = extract_table_ref(table_ref);

    // Validate the joined table exists.
    schema
        .column_names(&table_name)
        .map_err(|_| PlanError::UnknownTable(table_name.clone()))?;

    let right: LogicalPlan = LogicalPlan::Scan {
        table: table_name,
        alias: table_alias,
    };

    // Extract the ON condition expression (if any — CROSS JOINs have none).
    let condition = extract_join_condition(join_clause)?;

    Ok(LogicalPlan::Join {
        left: Box::new(left),
        right: Box::new(right),
        kind,
        condition,
    })
}

/// Determine the `JoinKind` from a `join_type` node.
///
/// The node contains keyword tokens: INNER, LEFT, RIGHT, FULL, CROSS, OUTER.
fn extract_join_kind(join_type: &GrammarASTNode) -> JoinKind {
    // Collect all keyword tokens to identify the join flavor.
    let keywords: Vec<String> = join_type
        .children
        .iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Token(tok) = c {
                Some(tok.value.to_uppercase())
            } else {
                None
            }
        })
        .collect();

    // Match on the first keyword (OUTER is a modifier, not the primary kind).
    match keywords.first().map(String::as_str) {
        Some("CROSS") => JoinKind::Cross,
        Some("LEFT") => JoinKind::Left,
        Some("RIGHT") => JoinKind::Right,
        Some("FULL") => JoinKind::Full,
        _ => JoinKind::Inner, // INNER or plain JOIN
    }
}

/// Extract the `ON condition` expression from a `join_clause` node.
///
/// Scans for the "ON" keyword token, then plans the next child node as an expr.
fn extract_join_condition(join_clause: &GrammarASTNode) -> Result<Option<SqlExpr>, PlanError> {
    let children = &join_clause.children;
    for (i, child) in children.iter().enumerate() {
        if is_keyword_token(child, "ON") {
            if let Some(ASTNodeOrToken::Node(expr_node)) = children.get(i + 1) {
                return Ok(Some(plan_expression(expr_node)?));
            }
        }
    }
    Ok(None)
}

/// Plan the `group_clause` into a list of GROUP BY key expressions.
///
/// Grammar: `group_clause = GROUP BY column_ref { "," column_ref }`
fn plan_group_by_exprs(group_clause: &GrammarASTNode) -> Result<Vec<SqlExpr>, PlanError> {
    find_nodes(group_clause, "column_ref")
        .iter()
        .map(|cr| plan_column_ref(cr))
        .collect()
}

/// Plan the `order_clause` into sort keys.
///
/// Grammar: `order_clause = ORDER BY order_item { "," order_item }`
/// Grammar: `order_item = expr [ ASC | DESC ]`
fn plan_order_by(
    order_clause: &GrammarASTNode,
    schema: &dyn SchemaProvider,
    collate_ctx: Option<(&str, Option<&str>)>,
    output_columns: &[OutputColumn],
) -> Result<Vec<SortKey>, PlanError> {
    // Fetch the base table's declared collations exactly ONCE (not per sort
    // key). A bare `ORDER BY c0, c0, …` over a very wide table would otherwise
    // clone the whole schema for every key — O(keys × columns) deep copies,
    // both dimensions attacker-controlled. Building a name→collation map up
    // front keeps planning linear in the number of keys.
    let order_ctx = collate_ctx.map(|(table, alias)| {
        let collations = schema
            .table_collations(table)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, coll)| (name.to_ascii_lowercase(), coll))
            .collect::<std::collections::HashMap<String, String>>();
        OrderCollateCtx {
            table,
            alias,
            collations,
        }
    });
    find_nodes(order_clause, "order_item")
        .iter()
        .map(|item| plan_order_item(item, order_ctx.as_ref(), output_columns))
        .collect()
}

/// Resolve a **positional** `ORDER BY <n>` key: SQLite treats a *bare integer
/// literal* in ORDER BY as a 1-based reference to the n-th column of the SELECT
/// output list (`SELECT a, b FROM t ORDER BY 2` sorts by `b`).
///
/// The rule is deliberately narrow — only a lone integer literal is positional.
/// An *expression* that happens to evaluate to an integer is NOT: `ORDER BY 1+0`
/// sorts by the constant `1`, i.e. does not reorder at all. So we match strictly
/// on `Literal(Int(_))` and nothing else.
///
/// | ORDER BY term | interpreted as              |
/// |---------------|-----------------------------|
/// | `1`           | the 1st output column       |
/// | `2 DESC`      | the 2nd output column, desc |
/// | `1+0`         | constant `1` (no reorder)   |
/// | `name`        | column/alias `name`         |
///
/// Returns:
/// - `Ok(Some(expr))` — the key was a positional reference; `expr` is the
///   substituted n-th output expression.
/// - `Ok(None)` — the key is not positional (leave it as written). Also the
///   escape hatch for `SELECT *`, whose column count/identity isn't known at
///   plan time; we leave such a key unchanged rather than guess.
/// - `Err(..)` — a positional reference out of range, matching SQLite's
///   "ORDER BY term out of range" (only diagnosed for a fully-explicit list).
fn resolve_positional_key(
    expr: &SqlExpr,
    outputs: &[OutputColumn],
) -> Result<Option<SqlExpr>, PlanError> {
    // Only a bare integer literal is a positional reference.
    let SqlExpr::Literal(SqlValue::Int(n)) = expr else {
        return Ok(None);
    };
    let n = *n;

    // If the output list contains an unexpanded `*` (or `t.*`), we can't count
    // or identify columns at plan time. Leave the key unchanged — no regression
    // versus prior behavior; positional-over-star is a documented follow-up.
    let has_star = outputs.iter().any(|c| {
        matches!(&c.expr, SqlExpr::Column { name, .. } if name == "*")
    });
    if has_star {
        return Ok(None);
    }

    // Fully explicit list: range-check exactly like SQLite (1..=ncols). Compare
    // in i64 (not `n as usize`) so a huge ordinal like 2^32 can't truncate to an
    // in-range value on a 32-bit `usize` and then index out of bounds; on any
    // platform, an out-of-range `n` is rejected here before it becomes an index.
    if n < 1 || n > outputs.len() as i64 {
        return Err(PlanError::UnsupportedStatement(format!(
            "ORDER BY term out of range - should be between 1 and {}",
            outputs.len()
        )));
    }

    // Safe: `1 <= n <= outputs.len()`, so `n - 1` is a valid 0-based index that
    // fits `usize` on every platform.
    let target = &outputs[(n - 1) as usize].expr;

    // If the target is (or contains) an aggregate, we cannot substitute its
    // expression: aggregates are computed once per group, not re-evaluated per
    // row in the sort path, so routing `SUM(v)` back through the sort would
    // ignore it. Sorting by a positional aggregate is left unchanged here (a
    // known-divergence ledger entry) rather than silently mis-sorting. The
    // non-aggregate case — the overwhelming majority — resolves below.
    if expr_contains_aggregate(target) {
        return Ok(None);
    }

    // Substitute the n-th output expression. Routing the real expression through
    // the ordinary sort path means positional keys inherit all the existing
    // machinery (hidden sort-key columns, collation, NULL placement) for free.
    Ok(Some(target.clone()))
}

/// Does this expression tree contain an aggregate function anywhere?
///
/// Used to keep positional `ORDER BY <n>` from substituting an aggregate output
/// expression back into the (per-row) sort path, where it can't be recomputed.
fn expr_contains_aggregate(expr: &SqlExpr) -> bool {
    match expr {
        SqlExpr::Aggregate { .. } => true,
        SqlExpr::Literal(_) | SqlExpr::Column { .. } => false,
        SqlExpr::BinaryOp { left, right, .. } => {
            expr_contains_aggregate(left) || expr_contains_aggregate(right)
        }
        SqlExpr::UnaryOp { expr, .. }
        | SqlExpr::IsNull(expr)
        | SqlExpr::IsNotNull(expr)
        | SqlExpr::Cast { expr, .. } => expr_contains_aggregate(expr),
        SqlExpr::Between {
            value, low, high, ..
        } => {
            expr_contains_aggregate(value)
                || expr_contains_aggregate(low)
                || expr_contains_aggregate(high)
        }
        SqlExpr::Like {
            value,
            pattern,
            escape,
            ..
        } => {
            expr_contains_aggregate(value)
                || expr_contains_aggregate(pattern)
                || escape.as_deref().is_some_and(expr_contains_aggregate)
        }
        SqlExpr::InList { value, list, .. } => {
            expr_contains_aggregate(value) || list.iter().any(expr_contains_aggregate)
        }
        SqlExpr::FunctionCall { args, .. } => args.iter().any(expr_contains_aggregate),
        SqlExpr::Case { branches, else_val } => {
            branches
                .iter()
                .any(|(c, v)| expr_contains_aggregate(c) || expr_contains_aggregate(v))
                || else_val.as_deref().is_some_and(expr_contains_aggregate)
        }
    }
}

/// Single base table (name + alias) plus its precomputed `column → COLLATE`
/// map, used to resolve a bare ORDER BY column's inherited collation without
/// re-querying the schema per key.
struct OrderCollateCtx<'a> {
    table: &'a str,
    alias: Option<&'a str>,
    /// Lowercased column name → declared collation, for columns that have one.
    collations: std::collections::HashMap<String, String>,
}

/// Resolve the collation a bare-column ORDER BY key inherits from its column
/// definition. Returns `Some(name)` only when the sort expression is a plain
/// column reference (optionally qualified by the table's name or alias) whose
/// column was declared with a non-default `COLLATE`. Anything else — a computed
/// expression, an alias to an output column, a qualifier that doesn't match the
/// base table — yields `None`, and the key keeps the default BINARY ordering.
/// Build the single-base-table collation context: the table's name + optional
/// alias plus its precomputed lowercased `column → COLLATE` map. Shared by the
/// ORDER BY and WHERE-comparison collation passes so the schema is queried once.
fn build_collate_ctx<'a>(
    schema: &dyn SchemaProvider,
    table: &'a str,
    alias: Option<&'a str>,
) -> OrderCollateCtx<'a> {
    let collations = schema
        .table_collations(table)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, coll)| (name.to_ascii_lowercase(), coll))
        .collect();
    OrderCollateCtx {
        table,
        alias,
        collations,
    }
}

/// Is `op` a binary comparison whose result depends on collation? These are the
/// operators SQLite subjects to collating-sequence resolution; arithmetic /
/// logical / bitwise operators are not.
fn is_comparison_op(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Lte | BinaryOp::Gt | BinaryOp::Gte
    )
}

/// Does this expression already carry an explicit collation, i.e. is it an
/// `__collate(_, _)` call produced by `wrap_collate`? Such an operand means a
/// `COLLATE` clause was written on the comparison, which outranks any
/// column-defined collation — so the column-collation pass must leave it alone.
fn is_collate_call(expr: &SqlExpr) -> bool {
    matches!(expr, SqlExpr::FunctionCall { name, .. } if name == "__collate")
}

/// Push column-defined `COLLATE` into the comparisons of a WHERE/HAVING
/// predicate. SQLite resolves a binary comparison's collating sequence as:
/// explicit `COLLATE` on either operand → the left operand's column collation →
/// the right operand's → BINARY. The explicit case is already lowered onto
/// `__collate` in `plan_comparison`; this pass handles the column cases.
///
/// It walks the boolean skeleton (`AND` / `OR` / `NOT`) down to each comparison
/// and, when neither operand is already `__collate`-wrapped and a column operand
/// declares a collation, wraps BOTH operands in `__collate(_, coll)` — turning
/// the collated comparison into a plain byte comparison of canonicalised values,
/// reusing the exact mechanism explicit `COLLATE` already uses. Non-comparison
/// leaves pass through unchanged. Only invoked for a single base table (no
/// JOINs), matching the ORDER BY collation restriction.
fn collate_comparisons(expr: SqlExpr, ctx: &OrderCollateCtx) -> SqlExpr {
    match expr {
        SqlExpr::BinaryOp { op, left, right } if is_comparison_op(&op) => {
            // An explicit COLLATE (already `__collate`-wrapped) outranks the
            // column's declared collation — leave the comparison untouched.
            if is_collate_call(&left) || is_collate_call(&right) {
                return SqlExpr::BinaryOp { op, left, right };
            }
            // SQLite picks the comparison's collation from the LEFT operand if it
            // is a column, otherwise the right operand if IT is a column, else
            // BINARY. Crucially, a *column* determines the collation even when
            // that collation is the default BINARY: `bin_col = nocase_col` uses
            // BINARY (the left column), it does NOT fall through to the right
            // column's NOCASE. So we resolve on the first operand that is a
            // base-table column and stop there — never OR-ing past a BINARY
            // column into the other operand. A resolved BINARY collation
            // (`None`) means byte order, so we leave the comparison bare rather
            // than wrapping it (which would also needlessly strip the column
            // reference that drives type affinity).
            let determining = if column_in_base_table(&left, ctx) {
                resolve_column_collation(&left, ctx)
            } else if column_in_base_table(&right, ctx) {
                resolve_column_collation(&right, ctx)
            } else {
                None
            };
            match determining {
                Some(name) => SqlExpr::BinaryOp {
                    op,
                    left: Box::new(wrap_collate(*left, &name)),
                    right: Box::new(wrap_collate(*right, &name)),
                },
                None => SqlExpr::BinaryOp { op, left, right },
            }
        }
        // Recurse through boolean connectives to reach nested comparisons.
        SqlExpr::BinaryOp { op, left, right } if matches!(op, BinaryOp::And | BinaryOp::Or) => {
            SqlExpr::BinaryOp {
                op,
                left: Box::new(collate_comparisons(*left, ctx)),
                right: Box::new(collate_comparisons(*right, ctx)),
            }
        }
        SqlExpr::UnaryOp {
            op: UnaryOp::Not,
            expr,
        } => SqlExpr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(collate_comparisons(*expr, ctx)),
        },
        // `value IN (list…)` — SQLite takes the operator's collating sequence
        // from the LEFT operand (`value`), exactly as for a binary comparison.
        // When `value` is a base-table column with a declared collation and it
        // is not already `__collate`-wrapped (an explicit COLLATE outranks), we
        // wrap the value AND every list element in `__collate(_, coll)`. The VM
        // then canonicalises each side before the membership test, so
        // `name IN ('APPLE')` on a NOCASE column matches `'Apple'`/`'apple'`.
        // `__collate` passes NULL/non-text through unchanged, so IN's NULL and
        // numeric semantics are preserved. `NOT IN` inherits this via `negated`.
        SqlExpr::InList {
            value,
            list,
            negated,
        } => {
            if is_collate_call(&value) {
                return SqlExpr::InList { value, list, negated };
            }
            let determining = if column_in_base_table(&value, ctx) {
                resolve_column_collation(&value, ctx)
            } else {
                None
            };
            match determining {
                Some(name) => SqlExpr::InList {
                    value: Box::new(wrap_collate(*value, &name)),
                    list: list.into_iter().map(|e| wrap_collate(e, &name)).collect(),
                    negated,
                },
                None => SqlExpr::InList { value, list, negated },
            }
        }
        other => other,
    }
}

fn resolve_column_collation(expr: &SqlExpr, ctx: &OrderCollateCtx) -> Option<String> {
    let SqlExpr::Column { table, name } = expr else {
        return None;
    };
    // A qualifier, if present, must name the base table or its alias; otherwise
    // it refers to something not in scope for single-table collation resolution.
    if let Some(qual) = table {
        let matches_table = qual.eq_ignore_ascii_case(ctx.table);
        let matches_alias = ctx.alias.is_some_and(|a| qual.eq_ignore_ascii_case(a));
        if !matches_table && !matches_alias {
            return None;
        }
    }
    ctx.collations.get(&name.to_ascii_lowercase()).cloned()
}

/// Is `expr` a reference to a column of the single base table (optionally
/// qualified by the table's name or alias)? Distinct from
/// [`resolve_column_collation`], which returns `None` for a column whose
/// collation is the default BINARY — here even a BINARY column is `true`,
/// because *being a column* is what makes an operand determine a comparison's
/// collating sequence (a left BINARY column forces byte order rather than
/// deferring to the other operand).
fn column_in_base_table(expr: &SqlExpr, ctx: &OrderCollateCtx) -> bool {
    let SqlExpr::Column { table, .. } = expr else {
        return false;
    };
    match table {
        None => true,
        Some(qual) => {
            qual.eq_ignore_ascii_case(ctx.table)
                || ctx.alias.is_some_and(|a| qual.eq_ignore_ascii_case(a))
        }
    }
}

/// Plan a single `order_item` node.
///
/// `collate_ctx` carries the single base table (name + alias + collation map)
/// when the query has exactly one table, enabling column-defined COLLATE to
/// flow into the sort key; it is `None` for multi-table or table-less queries.
fn plan_order_item(
    item: &GrammarASTNode,
    collate_ctx: Option<&OrderCollateCtx>,
    output_columns: &[OutputColumn],
) -> Result<SortKey, PlanError> {
    // The first child Node is the expression; check for ASC/DESC tokens.
    let expr_node = item
        .children
        .iter()
        .find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                Some(n)
            } else {
                None
            }
        })
        .ok_or_else(|| PlanError::UnsupportedStatement("empty order_item".to_string()))?;

    let raw_expr = plan_expression(expr_node)?;

    // A bare integer literal is a *positional* reference to the n-th output
    // column (`ORDER BY 2` → sort by the 2nd SELECT column). Resolve it to the
    // real output expression BEFORE collation inheritance below, so a positional
    // key over a `COLLATE NOCASE` column still picks up that collation. Non-
    // positional keys (and `SELECT *`) pass through unchanged.
    let expr = match resolve_positional_key(&raw_expr, output_columns)? {
        Some(resolved) => resolved,
        None => raw_expr,
    };

    // ASC is the default; DESC reverses.
    let ascending = !has_token(item, "DESC");

    // Optional `NULLS FIRST` / `NULLS LAST`. FIRST/LAST are NOT reserved
    // keywords (they are common column names), so the grammar accepts a generic
    // NAME after `NULLS` and we validate it here. Anything other than
    // FIRST/LAST is a syntax error, matching SQLite.
    let nulls_first = {
        let children = &item.children;
        let mut placement = None;
        for (i, c) in children.iter().enumerate() {
            if is_keyword_token(c, "NULLS") {
                match children.get(i + 1) {
                    Some(tok) => {
                        let w = token_text_of(tok).to_uppercase();
                        placement = Some(match w.as_str() {
                            "FIRST" => Ok(true),
                            "LAST" => Ok(false),
                            other => Err(PlanError::UnsupportedStatement(format!(
                                "expected FIRST or LAST after NULLS, got {other:?}"
                            ))),
                        });
                    }
                    None => {
                        placement = Some(Err(PlanError::UnsupportedStatement(
                            "NULLS clause missing FIRST/LAST".to_string(),
                        )))
                    }
                }
            }
        }
        placement.transpose()?
    };

    // Optional `COLLATE name` clause. `COLLATE` is followed by the collation
    // name (BINARY / NOCASE / RTRIM). We validate against the three built-in
    // sequences and store the name uppercased; `BINARY` (the default) collapses
    // to `None` so the VM takes the plain byte-order path. An unknown collation
    // is a planning error, matching SQLite's "no such collating sequence".
    let collation = {
        let children = &item.children;
        let mut coll: Option<Result<Option<String>, PlanError>> = None;
        for (i, c) in children.iter().enumerate() {
            if is_keyword_token(c, "COLLATE") {
                coll = Some(match children.get(i + 1) {
                    Some(tok) => {
                        let name = token_text_of(tok).to_uppercase();
                        match name.as_str() {
                            "BINARY" => Ok(None),
                            "NOCASE" | "RTRIM" => Ok(Some(name)),
                            other => Err(PlanError::UnsupportedStatement(format!(
                                "no such collating sequence: {other}"
                            ))),
                        }
                    }
                    None => Err(PlanError::UnsupportedStatement(
                        "COLLATE clause missing collation name".to_string(),
                    )),
                });
            }
        }
        coll.transpose()?.flatten()
    };

    // An explicit `COLLATE` on the ORDER BY item always wins — including
    // `COLLATE BINARY`, which forces byte order even when the column is declared
    // NOCASE/RTRIM. Because BINARY parses to `None` (same as "no collation"), we
    // must detect the *presence* of the clause separately: only a key with no
    // explicit COLLATE at all inherits the column's schema-defined sequence
    // (`CREATE TABLE t(x TEXT COLLATE NOCASE); ... ORDER BY x`).
    let has_explicit_collate = item
        .children
        .iter()
        .any(|c| is_keyword_token(c, "COLLATE"));
    let collation = if has_explicit_collate {
        collation
    } else {
        collate_ctx.and_then(|ctx| resolve_column_collation(&expr, ctx))
    };

    Ok(SortKey {
        expr,
        ascending,
        nulls_first,
        collation,
    })
}

/// Parse the `limit_clause` and return `(count, offset)`.
///
/// Grammar: `limit_clause = LIMIT [ "-" ] NUMBER [ OFFSET NUMBER | "," NUMBER ]`
///
/// Two tail forms, with the arguments in the OPPOSITE order:
///
/// | Written              | count | offset |
/// |----------------------|-------|--------|
/// | `LIMIT c`             | `c`   | —      |
/// | `LIMIT c OFFSET o`    | `c`   | `o`    |
/// | `LIMIT o , c`         | `c`   | `o`    |  ← MySQL shorthand, arguments swapped
///
/// So `LIMIT 1, 2` returns 2 rows starting after the first — identical to
/// `LIMIT 2 OFFSET 1`. The comma form is a MySQL-compatibility spelling that
/// SQLite also accepts; the only wrinkle is that the FIRST number is the
/// offset, not the count. We detect the comma token and swap accordingly.
///
/// SQLite semantics: `LIMIT -1` means "no limit" (return all rows). The `-`
/// sign only applies to the LIMIT count in the `OFFSET` form (`LIMIT -1
/// OFFSET n`); the comma form takes two plain non-negative numbers.
fn plan_limit(limit_clause: &GrammarASTNode) -> Result<(Option<i64>, Option<i64>), PlanError> {
    // A comma anywhere in the clause selects the MySQL `LIMIT off, count` form.
    let comma_form = limit_clause
        .children
        .iter()
        .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == ","));

    // Collect the numeric operands in written order, carrying the optional
    // leading `-` (only meaningful before the first number, i.e. the count in
    // the OFFSET form). Keywords (LIMIT/OFFSET) and the comma are structural.
    let mut nums: Vec<i64> = Vec::new();
    let mut pending_minus = false;
    for child in &limit_clause.children {
        if let ASTNodeOrToken::Token(tok) = child {
            match tok.value.as_str() {
                "LIMIT" | "OFFSET" | "," => {}
                "-" if nums.is_empty() => pending_minus = true,
                _ => {
                    if let Ok(n) = tok.value.parse::<i64>() {
                        let sign = if pending_minus && nums.is_empty() { -1 } else { 1 };
                        // `saturating_mul` rather than `*`: the current grammar
                        // never feeds an `i64::MIN`-valued token together with a
                        // leading `-`, so this can't overflow today — but this
                        // keeps it panic-free even if a future lexer change let
                        // NUMBER carry a sign.
                        nums.push(n.saturating_mul(sign));
                        pending_minus = false;
                    }
                }
            }
        }
    }

    // Map the operands onto (count, offset) per the form.
    let (count, offset) = match (comma_form, nums.as_slice()) {
        // `LIMIT off, count` — first is offset, second is count.
        (true, [off, cnt]) => (Some(*cnt), Some(*off)),
        (true, [off]) => (None, Some(*off)), // degenerate `LIMIT n,` — treat n as offset
        // `LIMIT count [OFFSET off]`.
        (false, [cnt, off]) => (Some(*cnt), Some(*off)),
        (false, [cnt]) => (Some(*cnt), None),
        _ => (None, None),
    };

    Ok((count, offset))
}

/// Plan the `select_list` into a list of `OutputColumn`s.
///
/// Grammar: `select_list = STAR | select_item { "," select_item }`
/// Grammar: `select_item = expr [ AS NAME ]`
fn plan_select_list(select_list: &GrammarASTNode) -> Result<Vec<OutputColumn>, PlanError> {
    // Check for SELECT * (STAR token).
    if has_token(select_list, "*") {
        return Ok(vec![OutputColumn {
            expr: SqlExpr::Column {
                table: None,
                name: "*".to_string(),
            },
            alias: None,
        }]);
    }

    // Plan each select_item.
    find_nodes(select_list, "select_item")
        .iter()
        .map(|item| plan_select_item(item))
        .collect()
}

/// Plan a single `select_item` node.
///
/// Grammar: `select_item = expr [ "AS" NAME ]`
fn plan_select_item(item: &GrammarASTNode) -> Result<OutputColumn, PlanError> {
    // The first child node is the expression.
    let expr_node = item
        .children
        .iter()
        .find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                Some(n)
            } else {
                None
            }
        })
        .ok_or_else(|| PlanError::UnsupportedStatement("empty select_item".to_string()))?;

    let expr = plan_expression(expr_node)?;

    // Look for an AS alias.
    let alias = extract_as_alias(item);

    Ok(OutputColumn { expr, alias })
}

/// Extract a `select_item` alias.
///
/// Grammar: `select_item = expr [ [ "AS" ] NAME ]` — the alias may be written
/// with or without the `AS` keyword (`SELECT a AS x` and `SELECT a x` are
/// equivalent in SQLite). Two AST shapes result:
///
/// * **Explicit** `expr AS name` → children `[Node(expr), Token("AS"),
///   Token(name)]`. We scan for the `AS` keyword and take the token after it.
/// * **Implicit** `expr name` → children `[Node(expr), Token(name)]` — no `AS`.
///   The expression is *always* a nested Node (never a bare token) and the only
///   bare tokens the grammar can place directly under `select_item` are the
///   optional `AS` and the alias, so a lone `Name`-type token that isn't a
///   keyword is unambiguously the implicit alias.
///
/// A bare `expr` with no alias (`SELECT a`) has no token children → `None`.
fn extract_as_alias(node: &GrammarASTNode) -> Option<String> {
    let children = &node.children;
    // Explicit `AS name`.
    for (i, child) in children.iter().enumerate() {
        if is_keyword_token(child, "AS") {
            if let Some(next) = children.get(i + 1) {
                return Some(token_text_of(next));
            }
        }
    }
    // Implicit `name` (no AS): the first bare identifier token directly under
    // the item. Restricting to Name-type tokens keeps us from mistaking any
    // stray keyword/punctuation for an alias.
    for child in children {
        if let ASTNodeOrToken::Token(tok) = child {
            if tok.type_ == lexer::token::TokenType::Name {
                return Some(tok.value.clone());
            }
        }
    }
    None
}

/// Extract the inner expression from a clause node (where_clause, having_clause).
///
/// These clauses have the structure `KEYWORD expr`, so we skip the keyword
/// token and plan the first child node.
fn extract_clause_expr(clause: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    clause
        .children
        .iter()
        .find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                Some(n)
            } else {
                None
            }
        })
        .map(plan_expression)
        .ok_or_else(|| {
            PlanError::UnsupportedStatement(format!(
                "clause without expression: {:?}",
                clause.rule_name
            ))
        })?
}

// ===========================================================================
// Aggregate collection
// ===========================================================================

/// Walk a `select_list` node and collect aggregate function calls WITH their
/// aliases from surrounding `select_item` nodes.
///
/// For `SELECT COUNT(*) AS cnt, SUM(v) AS total`, we produce:
///   `[AggregateItem { func: Count, alias: Some("cnt") }, AggregateItem { func: Sum, alias: Some("total") }]`
///
/// This lets the codegen emit the right column names without a separate rename step.
fn collect_aggregates_with_aliases(select_list: &GrammarASTNode, out: &mut Vec<AggregateItem>) {
    // Walk select_item children at the top level of select_list.
    for child in &select_list.children {
        if let ASTNodeOrToken::Node(item) = child {
            if item.rule_name == "select_item" {
                // Extract the optional alias (AS name).
                let alias = extract_as_alias(item);
                // Look for aggregate function calls inside this item.
                let before_len = out.len();
                collect_aggregates_from_node(item, out);
                // If we found new aggregates, set their alias from this item.
                if let Some(alias_str) = alias {
                    for agg in out[before_len..].iter_mut() {
                        if agg.alias.is_none() {
                            agg.alias = Some(alias_str.clone());
                        }
                    }
                }
            } else {
                // Non-select_item child (e.g., STAR): use recursive fallback.
                collect_aggregates_from_node(item, out);
            }
        }
    }
}

/// Walk a subtree and collect all aggregate function calls into `out`.
///
/// Aggregate functions (COUNT, SUM, AVG, MIN, MAX) can appear in the SELECT
/// list or HAVING clause.  We collect them here so the Aggregate plan node
/// can list all the aggregations needed.
fn collect_aggregates_from_node(node: &GrammarASTNode, out: &mut Vec<AggregateItem>) {
    if node.rule_name == "function_call" {
        // Check if this is an aggregate function.
        if let Some(agg_item) = try_plan_as_aggregate(node) {
            out.push(agg_item);
            return; // Don't recurse into function args when recognized as agg.
        }
    }
    // Recurse into all child nodes.
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            collect_aggregates_from_node(n, out);
        }
    }
}

/// Try to interpret a `function_call` node as an aggregate function.
///
/// Returns `Some(AggregateItem)` if the function name is one of the five
/// SQL aggregate functions; `None` otherwise.
fn try_plan_as_aggregate(func_call: &GrammarASTNode) -> Option<AggregateItem> {
    // The first token in a function_call is the function name.
    let name = first_name_token(func_call)?;
    let agg_func = match name.to_uppercase().as_str() {
        "COUNT" => AggFunc::Count,
        "SUM" => AggFunc::Sum,
        "AVG" => AggFunc::Avg,
        "MIN" => AggFunc::Min,
        "MAX" => AggFunc::Max,
        _ => return None,
    };

    // `MIN`/`MAX` are overloaded: with a single argument they are the aggregate
    // (min/max over a column), but with two-or-more they are the SCALAR function
    // that returns the smallest/largest of its arguments. Only the aggregate form
    // is collected here; the multi-argument form is left to `plan_function_call`.
    if matches!(agg_func, AggFunc::Min | AggFunc::Max) && call_arg_count(func_call) >= 2 {
        return None;
    }

    // Check for star argument: COUNT(*).
    let has_star = has_token(func_call, "*");
    let distinct = has_token(func_call, "DISTINCT");

    // Find the argument expression (if not a star call).
    let arg = if has_star {
        None
    } else {
        // The argument is the first expr/primary/etc. child node.
        func_call.children.iter().find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                // Skip "value_list" wrapper if present.
                if n.rule_name == "value_list" {
                    n.children.iter().find_map(|c2| {
                        if let ASTNodeOrToken::Node(n2) = c2 {
                            plan_expression(n2).ok().map(Box::new)
                        } else {
                            None
                        }
                    })
                } else {
                    plan_expression(n).ok().map(Box::new)
                }
            } else {
                None
            }
        })
    };

    Some(AggregateItem {
        func: agg_func,
        arg: arg.map(|b| *b),
        distinct,
        alias: None,
    })
}

// ===========================================================================
// DML statement planners
// ===========================================================================

/// Plan an `insert_stmt` node.
///
/// Grammar: `insert_stmt = INSERT INTO NAME [ "(" NAME { "," NAME } ")" ]
///                          VALUES row_value { "," row_value }`
/// Grammar: `row_value = "(" expr { "," expr } ")"`
///
/// When the INSERT has no explicit column list (positional VALUES), we resolve
/// the columns from the schema so downstream codegen always has named columns.
fn plan_insert(stmt: &GrammarASTNode, schema: &dyn SchemaProvider) -> Result<LogicalPlan, PlanError> {
    // The table name is the first NAME token after INSERT INTO.
    let table = extract_insert_table_name(stmt)?;

    // The optional column list: `(col1, col2, ...)`.
    let explicit_columns = extract_insert_columns(stmt);

    // Resolve the column list. If the INSERT specifies columns explicitly, use
    // them directly. Otherwise resolve from the schema so the codegen always has
    // named columns to emit (positional INSERT).
    let columns: Option<Vec<String>> = if explicit_columns.is_some() {
        explicit_columns
    } else {
        // Attempt to resolve column names from schema. If the table is not yet
        // in the schema (e.g. table created in the same batch), fall back to None
        // and let the backend handle it positionally.
        schema
            .column_names(&table)
            .ok()
    };

    // The VALUES rows.
    let rows = find_nodes(stmt, "row_value")
        .iter()
        .map(|rv| plan_row_value(rv))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LogicalPlan::Insert {
        table,
        columns,
        source: InsertSource::Values(rows),
    })
}

/// Extract the target table name from an INSERT statement.
///
/// Grammar positions: INSERT INTO <name> ...
/// We look for the first NAME token that follows the "INTO" keyword.
fn extract_insert_table_name(stmt: &GrammarASTNode) -> Result<String, PlanError> {
    let children = &stmt.children;
    for (i, child) in children.iter().enumerate() {
        if is_keyword_token(child, "INTO") {
            if let Some(next) = children.get(i + 1) {
                let name = token_text_of(next);
                if !name.is_empty() {
                    return Ok(name);
                }
            }
        }
    }
    Err(PlanError::UnsupportedStatement(
        "INSERT without table name".to_string(),
    ))
}

/// Extract the optional column list from an INSERT statement.
///
/// Returns `Some(vec![...])` if `(col1, col2, ...)` is present,
/// or `None` if INSERT uses implicit column ordering.
///
/// Detection: look for `(` before VALUES — all NAME tokens inside
/// the parentheses (before any VALUES keyword) form the column list.
fn extract_insert_columns(stmt: &GrammarASTNode) -> Option<Vec<String>> {
    let children = &stmt.children;
    let mut in_col_list = false;
    let mut cols: Vec<String> = Vec::new();
    let mut found_open = false;

    for child in children {
        match child {
            ASTNodeOrToken::Token(tok) => {
                let upper = tok.value.to_uppercase();
                if upper == "VALUES" {
                    // Reached VALUES; stop.
                    break;
                }
                if upper == "(" && !found_open {
                    // This is the opening paren of the column list
                    // (not a VALUES row — that comes later).
                    in_col_list = true;
                    found_open = true;
                } else if upper == ")" && in_col_list {
                    in_col_list = false;
                } else if in_col_list && upper != "," {
                    // A NAME inside the column list.
                    cols.push(tok.value.clone());
                }
            }
            ASTNodeOrToken::Node(_) => {
                // Node children in insert_stmt are row_value nodes (after VALUES).
                // If we see a node before VALUES we've reached the values section.
                break;
            }
        }
    }

    if cols.is_empty() {
        None
    } else {
        Some(cols)
    }
}

/// Plan a `row_value` node into a list of expressions.
///
/// Grammar: `row_value = "(" expr { "," expr } ")"`
fn plan_row_value(row_value: &GrammarASTNode) -> Result<Vec<SqlExpr>, PlanError> {
    row_value
        .children
        .iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                Some(plan_expression(n))
            } else {
                None
            }
        })
        .collect()
}

/// Plan an `update_stmt` node.
///
/// Grammar: `update_stmt = UPDATE NAME SET assignment { "," assignment } [ where_clause ]`
/// Grammar: `assignment = NAME "=" expr`
fn plan_update(stmt: &GrammarASTNode) -> Result<LogicalPlan, PlanError> {
    // Table name: the NAME token immediately after UPDATE.
    let table = extract_first_name_token(stmt)?;

    // Collect assignments.
    let assignments = find_nodes(stmt, "assignment")
        .iter()
        .map(|a| plan_assignment(a))
        .collect::<Result<Vec<_>, _>>()?;

    // Optional WHERE clause.
    let predicate = if let Some(wc) = find_node(stmt, "where_clause") {
        Some(extract_clause_expr(wc)?)
    } else {
        None
    };

    Ok(LogicalPlan::Update {
        table,
        assignments,
        predicate,
    })
}

/// Plan a single `assignment` node.
///
/// Grammar: `assignment = NAME "=" expr`
fn plan_assignment(node: &GrammarASTNode) -> Result<Assignment, PlanError> {
    // First token is the column name.
    let column = first_name_token(node)
        .ok_or_else(|| PlanError::UnsupportedStatement("assignment without column name".to_string()))?;

    // First child node is the expression.
    let expr_node = node
        .children
        .iter()
        .find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                Some(n)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            PlanError::UnsupportedStatement("assignment without expression".to_string())
        })?;

    let value = plan_expression(expr_node)?;
    Ok(Assignment { column, value })
}

/// Plan a `delete_stmt` node.
///
/// Grammar: `delete_stmt = DELETE FROM NAME [ where_clause ]`
fn plan_delete(stmt: &GrammarASTNode) -> Result<LogicalPlan, PlanError> {
    // Table name: the NAME token (after FROM keyword).
    let table = extract_name_after_keyword(stmt, "FROM")?;

    // Optional WHERE clause.
    let predicate = if let Some(wc) = find_node(stmt, "where_clause") {
        Some(extract_clause_expr(wc)?)
    } else {
        None
    };

    Ok(LogicalPlan::Delete { table, predicate })
}

/// Plan a `create_table_stmt` node.
///
/// Grammar: `create_table_stmt = CREATE TABLE [ IF NOT EXISTS ] NAME "(" col_def { "," col_def } ")"`
/// Grammar: `col_def = NAME NAME col_constraint*`
/// Grammar: `col_constraint = NOT NULL | NULL | PRIMARY KEY | UNIQUE | DEFAULT primary`
fn plan_create_table(stmt: &GrammarASTNode) -> Result<LogicalPlan, PlanError> {
    // Check for IF NOT EXISTS.
    let if_not_exists = has_token(stmt, "EXISTS");

    // Table name: the NAME token after TABLE (and after EXISTS if present).
    let table = extract_create_table_name(stmt)?;

    // Collect column definitions.
    let columns = find_nodes(stmt, "col_def")
        .iter()
        .map(|cd| plan_col_def(cd))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LogicalPlan::CreateTable {
        table,
        if_not_exists,
        columns,
    })
}

/// Extract the table name from `CREATE TABLE [IF NOT EXISTS] <name>`.
fn extract_create_table_name(stmt: &GrammarASTNode) -> Result<String, PlanError> {
    // Collect NAME tokens, skipping IF, NOT, EXISTS keywords.
    let skip_keywords = ["CREATE", "TABLE", "IF", "NOT", "EXISTS"];
    for child in &stmt.children {
        if let ASTNodeOrToken::Token(tok) = child {
            let upper = tok.value.to_uppercase();
            if !skip_keywords.contains(&upper.as_str()) && upper != "(" && upper != ")" && upper != "," {
                // This must be the table name (a non-keyword NAME token).
                return Ok(tok.value.clone());
            }
        }
    }
    Err(PlanError::UnsupportedStatement(
        "CREATE TABLE without table name".to_string(),
    ))
}

/// Plan a `col_def` node into a `ColumnDef`.
///
/// Grammar: `col_def = NAME NAME col_constraint*`
///
/// The first NAME is the column name, the second is the type name.
fn plan_col_def(col_def: &GrammarASTNode) -> Result<ColumnDef, PlanError> {
    // Collect the two leading NAME tokens (column name + type).
    let names: Vec<String> = col_def
        .children
        .iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Token(tok) = c {
                let upper = tok.value.to_uppercase();
                // Filter out constraint keywords.
                if !["NOT", "NULL", "PRIMARY", "KEY", "UNIQUE", "DEFAULT", "(", ")", ","]
                    .contains(&upper.as_str())
                {
                    Some(tok.value.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let col_name = names
        .first()
        .cloned()
        .ok_or_else(|| PlanError::UnsupportedStatement("col_def missing column name".to_string()))?;
    let type_name = names.get(1).cloned().unwrap_or_else(|| "TEXT".to_string());

    let mut col = ColumnDef::new(col_name, type_name);

    // Apply constraints from col_constraint children.
    for child in &col_def.children {
        if let ASTNodeOrToken::Node(constraint) = child {
            if constraint.rule_name == "col_constraint" {
                apply_col_constraint(&mut col, constraint)?;
            }
        }
    }

    Ok(col)
}

/// Apply a `col_constraint` to a mutable `ColumnDef`.
///
/// Grammar: `col_constraint = ( "NOT" "NULL" … ) | "NULL" | ( "PRIMARY" "KEY" … )
///                          | ( "UNIQUE" … ) | ( "DEFAULT" primary )
///                          | ( "CHECK" "(" expr ")" ) | ( "COLLATE" NAME )
///                          | ( "REFERENCES" NAME … ) ;`
///
/// Returns an error only for an unrecognised `COLLATE` sequence — SQLite
/// rejects `CREATE TABLE t(x COLLATE BOGUS)` at prepare time with "no such
/// collating sequence", so we surface that here rather than silently storing a
/// collation the VM cannot honour.
fn apply_col_constraint(
    col: &mut ColumnDef,
    constraint: &GrammarASTNode,
) -> Result<(), PlanError> {
    // Check which constraint keyword tokens are present.
    if has_token(constraint, "PRIMARY") {
        col.primary_key = true;
        col.not_null = true; // PRIMARY KEY implies NOT NULL.
    }
    if has_token(constraint, "UNIQUE") {
        col.unique = true;
    }
    if has_token(constraint, "NOT") && has_token(constraint, "NULL") {
        col.not_null = true;
    }
    // DEFAULT handling: if there's a "DEFAULT" keyword, look for the value.
    if has_token(constraint, "DEFAULT") {
        if let Some(primary) = find_node(constraint, "primary") {
            if let Ok(SqlExpr::Literal(sql_val)) = plan_primary(primary) {
                col.default_value = sql_val;
                col.has_default = true;
            }
        }
    }
    // COLLATE handling: the collation name is the token immediately after the
    // COLLATE keyword. `BINARY` is the default and collapses to `None` (see
    // `ColumnDef::collation`); `NOCASE`/`RTRIM` are stored uppercased; anything
    // else is a "no such collating sequence" error, matching SQLite. Only the
    // three built-in sequences exist in mini-sqlite (no user-registered ones).
    if has_token(constraint, "COLLATE") {
        let name = token_after_keyword(constraint, "COLLATE").ok_or_else(|| {
            PlanError::UnsupportedStatement("COLLATE clause missing collation name".to_string())
        })?;
        let upper = name.to_uppercase();
        match upper.as_str() {
            "BINARY" => col.collation = None,
            "NOCASE" | "RTRIM" => col.collation = Some(upper),
            other => {
                return Err(PlanError::UnsupportedStatement(format!(
                    "no such collating sequence: {other}"
                )))
            }
        }
    }
    Ok(())
}

/// Return the text of the token immediately following the first occurrence of
/// keyword `kw` among a node's direct token children. Used to read the operand
/// of a single-keyword clause such as `COLLATE NOCASE`.
fn token_after_keyword(node: &GrammarASTNode, kw: &str) -> Option<String> {
    let toks: Vec<&str> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
            _ => None,
        })
        .collect();
    let pos = toks.iter().position(|t| t.eq_ignore_ascii_case(kw))?;
    toks.get(pos + 1).map(|s| s.to_string())
}

/// Plan a `drop_table_stmt` node.
///
/// Grammar: `drop_table_stmt = DROP TABLE [ IF EXISTS ] NAME`
fn plan_drop_table(stmt: &GrammarASTNode) -> Result<LogicalPlan, PlanError> {
    // IF EXISTS check: look for the EXISTS keyword.
    let if_exists = has_token(stmt, "EXISTS");

    // Table name: the last NAME token (after TABLE and optional IF EXISTS).
    let table = extract_drop_table_name(stmt)?;

    Ok(LogicalPlan::DropTable { table, if_exists })
}

/// Extract the table name from `DROP TABLE [IF EXISTS] <name>`.
fn extract_drop_table_name(stmt: &GrammarASTNode) -> Result<String, PlanError> {
    let skip = ["DROP", "TABLE", "IF", "NOT", "EXISTS"];
    // We want the LAST non-keyword name (the table name comes after IF EXISTS).
    let mut last_name: Option<String> = None;
    for child in &stmt.children {
        if let ASTNodeOrToken::Token(tok) = child {
            let upper = tok.value.to_uppercase();
            if !skip.contains(&upper.as_str()) {
                last_name = Some(tok.value.clone());
            }
        }
    }
    last_name.ok_or_else(|| {
        PlanError::UnsupportedStatement("DROP TABLE without table name".to_string())
    })
}

// ===========================================================================
// Expression planner
// ===========================================================================

/// Maximum allowed expression nesting depth.
///
/// SQL expressions are recursive (parenthesized sub-expressions, NOT NOT NOT ...),
/// so a deeply-nested adversarial input could overflow the call stack.  We cap
/// recursion at this depth and return an error rather than crash.
///
/// 512 levels is far beyond any reasonable SQL expression; the SQL grammar's
/// longest chain is expr→or→and→not→comparison→additive→multiplicative→unary→primary
/// (9 levels per expression atom), so 512 accommodates ~55 levels of explicit
/// parenthesization.
const MAX_EXPR_DEPTH: usize = 512;

// Thread-local recursion depth counter for expression planning.
//
// Using a thread-local avoids threading a `depth` parameter through every
// function signature while still providing stack-overflow protection.
// The counter is reset to 0 at the start of each top-level `plan_expression`
// call (which is called from all clause planners).
//
// Safety: `std::cell::Cell` is not `Sync`, so this is safe for single-threaded
// and multi-threaded use alike — each thread has its own copy.
use std::cell::Cell;
thread_local! {
    static EXPR_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Plan any expression-level AST node into a [`SqlExpr`].
///
/// Protects against stack overflow from adversarially deep expression nesting
/// (e.g., `((((... 10,000 levels ... 1 ...))))`) by tracking recursion depth
/// via a thread-local counter.  Returns [`PlanError::UnsupportedStatement`]
/// when the depth limit is exceeded.
///
/// The grammar's expression hierarchy (from lowest to highest precedence):
///
/// ```text
/// expr           → or_expr
/// or_expr        → and_expr { OR and_expr }
/// and_expr       → not_expr { AND not_expr }
/// not_expr       → NOT not_expr | comparison
/// comparison     → additive { (= | != | < | > | <= | >=
///                              | IS [NOT] NULL | [NOT] LIKE
///                              | [NOT] BETWEEN | [NOT] IN) additive }
/// additive       → multiplicative { (+ | - | ||) multiplicative }
/// multiplicative → unary { (* | / | %) unary }
/// unary          → - unary | + unary | primary
/// primary        → NUMBER | STRING | NULL | TRUE | FALSE
///                | column_ref | function_call | "(" expr ")"
/// ```
///
/// Each rule either:
/// 1. Has a single child → pass through to the child.
/// 2. Has multiple operands separated by operators → build a `BinaryOp` chain.
fn plan_expression(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    // Increment depth counter; decrement on exit (RAII via a guard struct).
    let depth = EXPR_DEPTH.with(|c| {
        let d = c.get();
        c.set(d + 1);
        d
    });
    // Ensure the counter is decremented even on early return.
    struct DepthGuard;
    impl Drop for DepthGuard {
        fn drop(&mut self) {
            EXPR_DEPTH.with(|c| c.set(c.get().saturating_sub(1)));
        }
    }
    let _guard = DepthGuard;

    if depth >= MAX_EXPR_DEPTH {
        return Err(PlanError::UnsupportedStatement(
            "expression nesting depth limit exceeded (max 512 levels)".to_string(),
        ));
    }

    match node.rule_name.as_str() {
        // Passthrough rules — they just delegate to their child.
        "expr" => plan_expr_rule(node),
        "or_expr" => plan_or_expr(node),
        "and_expr" => plan_and_expr(node),
        "not_expr" => plan_not_expr(node),
        "comparison" => plan_comparison(node),
        "bitwise" => plan_bitwise(node),
        "additive" => plan_additive(node),
        "multiplicative" => plan_multiplicative(node),
        "unary" => plan_unary(node),
        "primary" => plan_primary(node),
        // Column reference and function call are reached from `primary`.
        "column_ref" => plan_column_ref(node),
        "function_call" => plan_function_call(node),
        "value_list" => {
            // Shouldn't appear at top-level, but handle gracefully.
            Err(PlanError::UnsupportedStatement(
                "unexpected value_list in expression context".to_string(),
            ))
        }
        other => {
            // Unknown rule — try to pass through to the first child node.
            // This makes the planner forward-compatible with grammar extensions.
            for child in &node.children {
                if let ASTNodeOrToken::Node(n) = child {
                    return plan_expression(n);
                }
            }
            Err(PlanError::UnsupportedStatement(format!(
                "unsupported expression rule: {other:?}"
            )))
        }
    }
}

/// Plan an `expr` node (handles the grammar rule named "expr").
///
/// Grammar: `expr = or_expr`
///
/// This is a passthrough — expr always delegates to or_expr.
/// Named `plan_expr_rule` to avoid collision with the public `plan_expr` API.
fn plan_expr_rule(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    // Find the or_expr child.
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            return plan_expression(n);
        }
    }
    Err(PlanError::UnsupportedStatement(
        "empty expr node".to_string(),
    ))
}

/// Plan an `or_expr` node.
///
/// Grammar: `or_expr = and_expr { OR and_expr }`
///
/// Multiple `and_expr` operands joined by OR produce a left-associative tree:
/// `a OR b OR c` becomes `BinaryOp(Or, BinaryOp(Or, a, b), c)`.
fn plan_or_expr(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    plan_left_assoc_binary(node, |tok| {
        if tok.to_uppercase() == "OR" {
            Some(BinaryOp::Or)
        } else {
            None
        }
    })
}

/// Plan an `and_expr` node.
///
/// Grammar: `and_expr = not_expr { AND not_expr }`
fn plan_and_expr(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    plan_left_assoc_binary(node, |tok| {
        if tok.to_uppercase() == "AND" {
            Some(BinaryOp::And)
        } else {
            None
        }
    })
}

/// Plan a `not_expr` node.
///
/// Grammar: `not_expr = NOT not_expr | comparison`
fn plan_not_expr(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    if has_token(node, "NOT") {
        // `NOT expr` — find the inner not_expr or comparison node.
        let inner = node
            .children
            .iter()
            .find_map(|c| {
                if let ASTNodeOrToken::Node(n) = c {
                    Some(n)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                PlanError::UnsupportedStatement("NOT without expression".to_string())
            })?;
        Ok(SqlExpr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(plan_expression(inner)?),
        })
    } else {
        // No NOT keyword — delegate to the single child node.
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                return plan_expression(n);
            }
        }
        Err(PlanError::UnsupportedStatement(
            "empty not_expr".to_string(),
        ))
    }
}

/// Plan a `comparison` node.
///
/// Grammar: `comparison = additive [ optional_part ]`
///
/// Where `optional_part` is one of:
/// - `cmp_op additive`            — simple comparison (`=`, `!=`, `<`, `>`, `<=`, `>=`)
/// - `BETWEEN additive AND additive`
/// - `NOT BETWEEN additive AND additive`
/// - `IN ( value_list )`
/// - `NOT IN ( value_list )`
/// - `LIKE additive`
/// - `NOT LIKE additive`
/// - `IS NULL`
/// - `IS NOT NULL`
///
/// ## Key grammar detail
///
/// The comparison operators (`=`, `!=`, `<`, etc.) are NOT direct token children
/// of the `comparison` node.  They are inside a `cmp_op` child NODE (a rule
/// reference).  This is because the grammar defines:
///
/// ```text
/// cmp_op = "=" | NOT_EQUALS | "<" | ">" | "<=" | ">="
/// ```
///
/// So we must find the `cmp_op` node, then extract its first token.
fn plan_comparison(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    let children = &node.children;

    // Separate into child nodes (in order) and direct keyword tokens.
    //
    // Note: `cmp_op` operators arrive as a child Node, not direct tokens.
    // Direct keyword tokens in `comparison` are: IS, NOT, NULL, BETWEEN, AND,
    // IN, LIKE (and punctuation: `(`, `)`).
    let mut child_nodes_ordered: Vec<&GrammarASTNode> = Vec::new();
    let mut direct_tok_uppers: Vec<String> = Vec::new();

    for child in children {
        match child {
            ASTNodeOrToken::Node(n) => child_nodes_ordered.push(n),
            ASTNodeOrToken::Token(t) => direct_tok_uppers.push(t.value.to_uppercase()),
        }
    }

    // The first child node is always the left-hand `additive`.
    let left_node = match child_nodes_ordered.first() {
        Some(n) => n,
        None => return Err(PlanError::UnsupportedStatement("empty comparison".to_string())),
    };
    let left = plan_expression(left_node)?;

    // If there's only the left operand and no keywords/operators, it's a passthrough.
    if child_nodes_ordered.len() == 1 && direct_tok_uppers.is_empty() {
        return Ok(left);
    }

    // IS [NOT] NULL   and   IS [NOT] <expr>  (null-safe (in)equality)
    // Grammar: `IS NULL` / `IS NOT NULL` produce only keyword tokens (no right
    // operand node), whereas `IS <expr>` / `IS NOT <expr>` produce a second
    // expression node — that is how we tell the two apart.
    if direct_tok_uppers.contains(&"IS".to_string()) {
        let has_not = direct_tok_uppers.contains(&"NOT".to_string());
        if let Some(right_node) = child_nodes_ordered.get(1) {
            let right = plan_expression(right_node)?;
            // Two spellings share the null-safe compare `plan_is_distinct`:
            //   `x IS [NOT] y`                 → negated = has_not
            //   `x IS [NOT] DISTINCT FROM y`   → negated = !has_not
            // because DISTINCT *inverts* the sense: `IS NOT DISTINCT FROM` is the
            // null-safe equality (like `IS`), and `IS DISTINCT FROM` its negation
            // (like `IS NOT`).
            let has_distinct = direct_tok_uppers.contains(&"DISTINCT".to_string());
            let negated = if has_distinct { !has_not } else { has_not };
            return Ok(plan_is_distinct(left, right, negated));
        }
        return Ok(if has_not {
            SqlExpr::IsNotNull(Box::new(left))
        } else {
            SqlExpr::IsNull(Box::new(left))
        });
    }

    // BETWEEN / NOT BETWEEN
    // Grammar: `[NOT] BETWEEN additive AND additive`
    // The `additive` operands are the 2nd and 3rd child nodes (after the first).
    if direct_tok_uppers.contains(&"BETWEEN".to_string()) {
        let negated = direct_tok_uppers.contains(&"NOT".to_string());
        // child_nodes_ordered[0] = left, [1] = low, [2] = high
        let low_node = child_nodes_ordered
            .get(1)
            .ok_or_else(|| PlanError::UnsupportedStatement("BETWEEN missing low".to_string()))?;
        let high_node = child_nodes_ordered
            .get(2)
            .ok_or_else(|| PlanError::UnsupportedStatement("BETWEEN missing high".to_string()))?;
        return Ok(SqlExpr::Between {
            value: Box::new(left),
            low: Box::new(plan_expression(low_node)?),
            high: Box::new(plan_expression(high_node)?),
            negated,
        });
    }

    // LIKE / NOT LIKE  [ESCAPE ch]
    // Grammar: `[NOT] LIKE additive [ESCAPE additive]`
    // Child nodes: [left, pattern] or [left, pattern, escape]; the `ESCAPE`
    // keyword shows up as a direct token.
    if direct_tok_uppers.contains(&"LIKE".to_string()) {
        let negated = direct_tok_uppers.contains(&"NOT".to_string());
        let pattern_node = child_nodes_ordered
            .get(1)
            .ok_or_else(|| PlanError::UnsupportedStatement("LIKE missing pattern".to_string()))?;
        let escape = if direct_tok_uppers.contains(&"ESCAPE".to_string()) {
            let escape_node = child_nodes_ordered.get(2).ok_or_else(|| {
                PlanError::UnsupportedStatement("LIKE ESCAPE missing character".to_string())
            })?;
            Some(Box::new(plan_expression(escape_node)?))
        } else {
            None
        };
        return Ok(SqlExpr::Like {
            value: Box::new(left),
            pattern: Box::new(plan_expression(pattern_node)?),
            negated,
            escape,
        });
    }

    // GLOB / NOT GLOB
    // Grammar: `[NOT] GLOB additive`
    //
    // SQLite defines the operator `X GLOB Y` as the function `glob(Y, X)` —
    // note the argument order: the PATTERN is the first argument. Rather than
    // add a dedicated `Glob` expr node + codegen + VM opcode, we lower the
    // operator onto the `glob` builtin the VM already implements, exactly as
    // SQLite does. `NOT GLOB` wraps the call in a logical NOT.
    if direct_tok_uppers.contains(&"GLOB".to_string()) {
        let negated = direct_tok_uppers.contains(&"NOT".to_string());
        let pattern_node = child_nodes_ordered
            .get(1)
            .ok_or_else(|| PlanError::UnsupportedStatement("GLOB missing pattern".to_string()))?;
        let pattern = plan_expression(pattern_node)?;
        let call = SqlExpr::FunctionCall {
            name: "glob".to_string(),
            args: vec![pattern, left], // glob(pattern, value)
            star: false,
        };
        return Ok(if negated {
            SqlExpr::UnaryOp { op: UnaryOp::Not, expr: Box::new(call) }
        } else {
            call
        });
    }

    // IN / NOT IN
    // Grammar: `[NOT] IN ( value_list )`
    // The value_list is a child node.
    if direct_tok_uppers.contains(&"IN".to_string()) {
        let negated = direct_tok_uppers.contains(&"NOT".to_string());
        // value_list is a direct child node.
        let list_items = if let Some(vl) = find_node(node, "value_list") {
            vl.children
                .iter()
                .filter_map(|c| {
                    if let ASTNodeOrToken::Node(n) = c {
                        plan_expression(n).ok()
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        return Ok(SqlExpr::InList {
            value: Box::new(left),
            list: list_items,
            negated,
        });
    }

    // Standard comparison operator via `cmp_op` child node.
    //
    // Grammar: `comparison = additive [ cmp_op additive | ... ]`
    // `cmp_op` is a rule reference → produces a GrammarASTNode with rule_name "cmp_op".
    // Inside `cmp_op` there is one token: `=`, `!=` (NOT_EQUALS token), `<`, `>`, `<=`, `>=`.
    //
    // child_nodes_ordered: [left_additive, cmp_op_node, right_additive]
    if let Some(cmp_op_node) = child_nodes_ordered.get(1).filter(|n| n.rule_name == "cmp_op") {
        // Extract the operator token from inside cmp_op.
        let op_str = cmp_op_node
            .children
            .iter()
            .find_map(|c| {
                if let ASTNodeOrToken::Token(t) = c {
                    Some(t.value.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let op = match op_str.as_str() {
            "=" => BinaryOp::Eq,
            "!=" | "<>" => BinaryOp::Neq,
            "<=" => BinaryOp::Lte,
            ">=" => BinaryOp::Gte,
            "<" => BinaryOp::Lt,
            ">" => BinaryOp::Gt,
            _ => return Err(PlanError::UnsupportedStatement(
                format!("unknown comparison operator: {op_str:?}"),
            )),
        };

        let right_node = child_nodes_ordered
            .get(2)
            .ok_or_else(|| PlanError::UnsupportedStatement("comparison missing right operand".to_string()))?;
        let right = plan_expression(right_node)?;

        // Optional `COLLATE name` on the comparison (grammar allows it after the
        // right operand). NOCASE (ASCII case-fold) and RTRIM (trim trailing
        // spaces) are *canonicalising* transforms, so `x <op> y COLLATE C` is
        // exactly `canon_C(x) <op> canon_C(y)` under the default byte comparison
        // — including the non-text and NULL cases (`5 = '5' COLLATE NOCASE` is 0
        // because 5 stays 5 while '5' stays '5'; a NULL operand stays NULL). We
        // therefore lower the collation onto the internal `__collate` builtin
        // wrapping BOTH operands, reusing the VM's existing collation helper —
        // no new comparison opcode, mirroring the `GLOB → glob()` lowering.
        // `COLLATE BINARY` wraps too — an identity transform in the VM, but it
        // marks the comparison as explicitly collated so a column-defined
        // collation can't later override it.
        let (left, right) = match collate_name_after(&direct_tok_uppers)? {
            Some(name) => (wrap_collate(left, &name), wrap_collate(right, &name)),
            None => (left, right),
        };

        return Ok(SqlExpr::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        });
    }

    // Passthrough — single operand, no recognised operator.
    Ok(left)
}

/// Find the collation name in a comparison's direct tokens, if it carries a
/// `COLLATE name` clause. Returns `None` only when no `COLLATE` clause is
/// present; `Some(NAME)` for `NOCASE`/`RTRIM`/`BINARY` (BINARY is the identity
/// transform but is still reported so an explicit clause overrides an operand's
/// column-defined collation); and an error for an unknown collation, matching
/// SQLite's "no such collating sequence".
fn collate_name_after(direct_tok_uppers: &[String]) -> Result<Option<String>, PlanError> {
    let Some(pos) = direct_tok_uppers.iter().position(|t| t == "COLLATE") else {
        return Ok(None);
    };
    let name = direct_tok_uppers.get(pos + 1).ok_or_else(|| {
        PlanError::UnsupportedStatement("COLLATE clause missing collation name".to_string())
    })?;
    match name.as_str() {
        // `COLLATE BINARY` is the default byte order, so at first glance it needs
        // no wrapping. But it must still be *recorded* as an explicit collation:
        // it forces byte order even when an operand's column is declared NOCASE,
        // and the column-collation pass below keys off "is either operand already
        // `__collate`-wrapped?" to decide whether an explicit collation is
        // present. Wrapping in the identity `__collate(_, 'BINARY')` (the VM
        // treats an unknown/BINARY collation as a no-op transform) both preserves
        // byte semantics and marks the comparison as explicitly collated.
        "BINARY" => Ok(Some(name.clone())),
        "NOCASE" | "RTRIM" => Ok(Some(name.clone())),
        other => Err(PlanError::UnsupportedStatement(format!(
            "no such collating sequence: {other}"
        ))),
    }
}

/// Wrap an expression in the internal `__collate(value, 'NAME')` builtin, which
/// canonicalises a text value for the given collation (and passes non-text and
/// NULL through unchanged) so a following byte comparison honours the collation.
fn wrap_collate(expr: SqlExpr, name: &str) -> SqlExpr {
    SqlExpr::FunctionCall {
        name: "__collate".to_string(),
        args: vec![expr, SqlExpr::Literal(SqlValue::Text(name.to_string()))],
        star: false,
    }
}

/// Plan an `additive` node.
///
/// Grammar: `additive = multiplicative { (+ | - | ||) multiplicative }`
fn plan_additive(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    plan_left_assoc_binary(node, |tok| match tok {
        "+" => Some(BinaryOp::Add),
        "-" => Some(BinaryOp::Sub),
        "||" => Some(BinaryOp::Concat),
        _ => None,
    })
}

/// Plan a `bitwise` node.
///
/// Grammar: `bitwise = additive { ("&" | "|" | "<<" | ">>") additive }`
///
/// All four operators share one precedence level and are left-associative, so
/// `5 | 3 & 2` parses as `(5 | 3) & 2`. The VM coerces both operands to integer
/// and propagates NULL; see `apply_binary`/`apply_shift` in sql-vm.
fn plan_bitwise(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    plan_left_assoc_binary(node, |tok| match tok {
        "&" => Some(BinaryOp::BitAnd),
        "|" => Some(BinaryOp::BitOr),
        "<<" => Some(BinaryOp::ShiftLeft),
        ">>" => Some(BinaryOp::ShiftRight),
        _ => None,
    })
}

/// Plan a `multiplicative` node.
///
/// Grammar: `multiplicative = unary { (* | / | %) unary }`
fn plan_multiplicative(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    plan_left_assoc_binary(node, |tok| match tok {
        "*" => Some(BinaryOp::Mul),
        "/" => Some(BinaryOp::Div),
        "%" => Some(BinaryOp::Mod),
        _ => None,
    })
}

/// Plan a `unary` node.
///
/// Grammar: `unary = "-" unary | "+" unary | primary`
fn plan_unary(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    // `-x` (arithmetic negation) and `~x` (bitwise complement) both wrap the
    // inner operand; `+x` is a no-op handled by the fall-through below. `~`
    // coerces its operand to integer and propagates NULL (see the VM).
    let unary_op = if has_token(node, "-") {
        Some(UnaryOp::Neg)
    } else if has_token(node, "~") {
        Some(UnaryOp::BitNot)
    } else {
        None
    };
    if let Some(op) = unary_op {
        let inner = node
            .children
            .iter()
            .find_map(|c| {
                if let ASTNodeOrToken::Node(n) = c {
                    Some(n)
                } else {
                    None
                }
            })
            .ok_or_else(|| PlanError::UnsupportedStatement("unary operator without operand".to_string()))?;
        return Ok(SqlExpr::UnaryOp {
            op,
            expr: Box::new(plan_expression(inner)?),
        });
    }
    // `+x` is a no-op; just delegate to the child.
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            return plan_expression(n);
        }
    }
    Err(PlanError::UnsupportedStatement(
        "empty unary node".to_string(),
    ))
}

/// Plan a `primary` node — the leaf of the expression hierarchy.
///
/// Grammar:
/// ```text
/// primary = NUMBER | STRING | NULL | TRUE | FALSE
///         | column_ref | function_call | "(" expr ")"
/// ```
///
/// Truth table for literal recognition:
///
/// | Token value          | Type         | SqlValue              |
/// |----------------------|--------------|-----------------------|
/// | `123`                | NUMBER       | `Int(123)`            |
/// | `3.14`               | NUMBER       | `Float(3.14)`         |
/// | `'hello'`            | STRING       | `Text("hello")`       |
/// | `NULL`               | KEYWORD      | `Null`                |
/// | `TRUE`               | KEYWORD      | `Bool(true)`          |
/// | `FALSE`              | KEYWORD      | `Bool(false)`         |
fn plan_primary(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    // `CAST(expr AS type)` — recognised by a leading `CAST` keyword token.
    // Handled before the generic loop so the `CAST` token isn't mistaken for a
    // bare column name.
    if let Some(ASTNodeOrToken::Token(t)) = node.children.first() {
        if t.value.eq_ignore_ascii_case("CAST") {
            return plan_cast(node);
        }
        if t.value.eq_ignore_ascii_case("CASE") {
            return plan_case(node);
        }
    }

    // Check child nodes first (column_ref, function_call, or parenthesized expr).
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) => match n.rule_name.as_str() {
                "column_ref" => return plan_column_ref(n),
                "function_call" => return plan_function_call(n),
                "expr" => return plan_expression(n),
                // A `( SELECT … )` scalar subquery parses to a nested `select_stmt`
                // node in a primary. Parsing is wired but evaluation is not yet:
                // reject it with a clear error rather than mis-planning it as an
                // expression. Wiring `SqlExpr::ScalarSubquery` + the VM sub-plan
                // eval is the follow-up.
                "select_stmt" => {
                    return Err(PlanError::UnsupportedStatement(
                        "scalar subqueries are not yet supported".to_string(),
                    ))
                }
                "or_expr" | "and_expr" | "not_expr" | "comparison" | "bitwise"
                | "additive" | "multiplicative" | "unary" | "primary" => return plan_expression(n),
                _ => return plan_expression(n),
            },
            ASTNodeOrToken::Token(tok) => {
                let upper = tok.value.to_uppercase();
                return match upper.as_str() {
                    "NULL" => Ok(SqlExpr::Literal(SqlValue::Null)),
                    "TRUE" => Ok(SqlExpr::Literal(SqlValue::Bool(true))),
                    "FALSE" => Ok(SqlExpr::Literal(SqlValue::Bool(false))),
                    "(" | ")" | "," => {
                        // Punctuation — skip (shouldn't be first unless malformed).
                        continue;
                    }
                    _ => {
                        // NUMBER or STRING literal, or a bare NAME.
                        plan_primary_token(tok)
                    }
                };
            }
        }
    }
    Err(PlanError::UnsupportedStatement(
        "empty primary node".to_string(),
    ))
}

/// Plan a `CAST(expr AS type)` primary node into [`SqlExpr::Cast`].
///
/// Grammar: `primary = "CAST" "(" expr "AS" NAME ")"` (among other choices).
/// The node's children are the `CAST` / `(` / `AS` / `)` tokens, the inner
/// expression node, and the type-name token.
///
/// The declared type name is mapped to a [`CastType`] using SQLite's
/// substring affinity rule (so synonyms like `INT`, `VARCHAR`, `FLOAT` resolve
/// correctly): a name containing `INT` → INTEGER; `CHAR`/`CLOB`/`TEXT` → TEXT;
/// `REAL`/`FLOA`/`DOUB` → REAL. `BLOB` and `NUMERIC`/other names are not yet
/// supported and yield an `UnsupportedStatement` error (a later increment).
fn plan_cast(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    // The inner expression is the sole nested Node child.
    let expr_node = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            _ => None,
        })
        .ok_or_else(|| PlanError::UnsupportedStatement("CAST missing expression".to_string()))?;
    let inner = plan_expression(expr_node)?;

    // The type name is the token following `AS`.
    let type_name = {
        let children = &node.children;
        let mut found = None;
        for (i, c) in children.iter().enumerate() {
            if is_keyword_token(c, "AS") {
                if let Some(ASTNodeOrToken::Token(t)) = children.get(i + 1) {
                    found = Some(t.value.to_uppercase());
                }
            }
        }
        found.ok_or_else(|| PlanError::UnsupportedStatement("CAST missing type name".to_string()))?
    };

    // SQLite's type-name → affinity rules (https://sqlite.org/datatype3.html
    // §3.1), applied in order: INT → INTEGER; CHAR/CLOB/TEXT → TEXT; BLOB (or an
    // empty name) → BLOB; REAL/FLOA/DOUB → REAL; anything else → NUMERIC. NUMERIC
    // is therefore the default for `NUMERIC`, `DECIMAL`, `BOOLEAN`, `DATE`, … .
    // BLOB casts are not yet implemented, so we reject them explicitly rather
    // than silently mis-routing them into the NUMERIC fallback.
    let ty = if type_name.contains("INT") {
        CastType::Integer
    } else if type_name.contains("CHAR") || type_name.contains("CLOB") || type_name.contains("TEXT") {
        CastType::Text
    } else if type_name.contains("BLOB") {
        return Err(PlanError::UnsupportedStatement(
            "CAST to BLOB is not yet supported".to_string(),
        ));
    } else if type_name.contains("REAL") || type_name.contains("FLOA") || type_name.contains("DOUB") {
        CastType::Real
    } else {
        CastType::Numeric
    };

    Ok(SqlExpr::Cast {
        expr: Box::new(inner),
        ty,
    })
}

/// Lower `a IS b` (and `a IS NOT b`) — SQLite's **null-safe** (in)equality —
/// onto a `CASE`, reusing existing nodes so no new codegen or VM opcode is
/// needed.
///
/// Semantics: `a IS b` is 1 when both operands are NULL, 0 when exactly one is
/// NULL, and `a = b` otherwise (in that last case neither is NULL, so `a = b` is
/// a clean boolean, never NULL). This differs from `a = b`, which yields NULL
/// whenever either side is NULL. `a IS NOT b` is the logical negation.
///
/// Expressed as:
/// ```text
/// CASE WHEN a IS NULL AND b IS NULL THEN 1
///      WHEN a IS NULL OR  b IS NULL THEN 0
///      ELSE a = b END
/// ```
/// Both operands are pure, so re-evaluating them across the CASE branches is
/// sound. (The tempting `(a = b) OR (a IS NULL AND b IS NULL)` is WRONG — it
/// yields NULL, not 0, for `1 IS NULL`.)
fn plan_is_distinct(left: SqlExpr, right: SqlExpr, negated: bool) -> SqlExpr {
    let both_null = SqlExpr::BinaryOp {
        op: BinaryOp::And,
        left: Box::new(SqlExpr::IsNull(Box::new(left.clone()))),
        right: Box::new(SqlExpr::IsNull(Box::new(right.clone()))),
    };
    let either_null = SqlExpr::BinaryOp {
        op: BinaryOp::Or,
        left: Box::new(SqlExpr::IsNull(Box::new(left.clone()))),
        right: Box::new(SqlExpr::IsNull(Box::new(right.clone()))),
    };
    let eq = SqlExpr::BinaryOp {
        op: BinaryOp::Eq,
        left: Box::new(left),
        right: Box::new(right),
    };
    let case = SqlExpr::Case {
        branches: vec![
            (both_null, SqlExpr::Literal(SqlValue::Int(1))),
            (either_null, SqlExpr::Literal(SqlValue::Int(0))),
        ],
        else_val: Some(Box::new(eq)),
    };
    if negated {
        SqlExpr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(case),
        }
    } else {
        case
    }
}

/// Plan a searched `CASE WHEN … THEN … [ELSE …] END` primary node into
/// [`SqlExpr::Case`].
///
/// Grammar: `CASE ("WHEN" expr "THEN" expr)+ ["ELSE" expr] "END"`. The node's
/// children are the `CASE`/`WHEN`/`THEN`/`ELSE`/`END` keyword tokens interleaved
/// with the condition and value expression nodes. We walk them in order: each
/// keyword tags the expression node that follows it, so `WHEN`→condition,
/// `THEN`→pairs with the last condition into a branch, `ELSE`→the fallback.
fn plan_case(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    let mut branches: Vec<(SqlExpr, SqlExpr)> = Vec::new();
    let mut else_val: Option<Box<SqlExpr>> = None;
    let mut pending_cond: Option<SqlExpr> = None;
    // What the next expression node is filling (set by the preceding keyword).
    let mut slot: Option<&'static str> = None;
    // The *simple* form carries an operand expr between `CASE` and the first
    // `WHEN` — the one expression node that appears while no WHEN/THEN/ELSE
    // keyword is active. `None` = the searched form (`CASE WHEN cond …`).
    let mut operand: Option<SqlExpr> = None;

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                slot = match t.value.to_uppercase().as_str() {
                    "WHEN" => Some("when"),
                    "THEN" => Some("then"),
                    "ELSE" => Some("else"),
                    _ => None, // CASE / END and any punctuation
                };
            }
            ASTNodeOrToken::Node(n) => {
                let expr = plan_expression(n)?;
                match slot {
                    Some("when") => pending_cond = Some(expr),
                    Some("then") => {
                        let value = pending_cond.take().ok_or_else(|| {
                            PlanError::UnsupportedStatement("CASE THEN without WHEN".to_string())
                        })?;
                        // Simple form: the WHEN expression is a *value* compared
                        // to the operand for equality (`operand = value`); the
                        // searched form uses the WHEN expression as the condition
                        // verbatim. Cloning the operand per branch is exact
                        // because mini-sqlite expressions are pure (SQLite
                        // evaluates the operand once, but a pure operand yields
                        // the same result each time). A NULL operand makes every
                        // `operand = value` NULL — never true — so a NULL operand
                        // falls through to ELSE, matching SQLite.
                        let cond = match &operand {
                            Some(op) => SqlExpr::BinaryOp {
                                op: BinaryOp::Eq,
                                left: Box::new(op.clone()),
                                right: Box::new(value),
                            },
                            None => value,
                        };
                        branches.push((cond, expr));
                    }
                    Some("else") => else_val = Some(Box::new(expr)),
                    // A node with no active slot, before any WHEN, is the simple
                    // form's operand.
                    _ => operand = Some(expr),
                }
                slot = None;
            }
        }
    }

    if branches.is_empty() {
        return Err(PlanError::UnsupportedStatement(
            "CASE requires at least one WHEN … THEN … branch".to_string(),
        ));
    }
    Ok(SqlExpr::Case { branches, else_val })
}

/// Plan a single token from a primary node into a literal or column reference.
///
/// ## Token classification
///
/// The SQL lexer (sql-lexer crate) strips quotes from string literals, so
/// `'hello'` arrives with `tok.value = "hello"` and `tok.type_ = TokenType::String`.
/// We use `tok.type_` to distinguish strings from column name references.
///
/// | `tok.type_`       | Meaning                    | SqlExpr                      |
/// |-------------------|----------------------------|------------------------------|
/// | `String`          | SQL string literal         | `Literal(Text(value))`       |
/// | `Number`          | Integer or float literal   | `Literal(Int)` or `Float`    |
/// | `Name`/`Keyword`  | Column name reference      | `Column { name: value }`     |
fn plan_primary_token(tok: &Token) -> Result<SqlExpr, PlanError> {
    // STRING literal: the lexer strips the surrounding quotes and sets
    // type_ = TokenType::String, but leaves the inner content raw. SQL escapes a
    // literal single quote by doubling it (`'it''s'` is the four-character string
    // `it's`), so collapse each `''` back to one `'` here.
    if tok.type_ == TokenType::String {
        return Ok(SqlExpr::Literal(SqlValue::Text(tok.value.replace("''", "'"))));
    }

    let val = &tok.value;

    // Blob literal `x'48656C6C6F'` / `X'…'`. The lexer's `BLOB_HEX` rule aliases
    // it to `BLOB`; because `TokenType` has no `Blob` variant, the lexer records
    // the name in `type_name` and falls `type_` back to `Name`. Decode the hex
    // body into raw bytes: SQLite reads `x'414243'` as the three bytes
    // `41 42 43`. An odd number of hex digits is a tokenizer error in SQLite
    // (`x'012'` → "unrecognized token"); we surface it as a plan error rather
    // than silently truncating. The empty blob `x''` is the zero-byte blob.
    if tok.type_name.as_deref() == Some("BLOB") || tok.type_name.as_deref() == Some("BLOB_HEX") {
        return decode_blob_literal(val).map(|bytes| SqlExpr::Literal(SqlValue::Blob(bytes)));
    }

    // NUMBER literal: try integer first, then float.
    if let Ok(i) = val.parse::<i64>() {
        return Ok(SqlExpr::Literal(SqlValue::Int(i)));
    }
    if let Ok(f) = val.parse::<f64>() {
        return Ok(SqlExpr::Literal(SqlValue::Float(f)));
    }

    // Bare NAME token in a primary context = unqualified column reference.
    Ok(SqlExpr::Column {
        table: None,
        name: val.clone(),
    })
}

/// Decode the body of a blob literal `x'…'` / `X'…'` into raw bytes.
///
/// The `raw` argument is the token text as the lexer captured it. The
/// `BLOB_HEX` rule (`[xX]'[0-9A-Fa-f]*'`) keeps the surrounding `x'` / `'`, so
/// we strip them here; we also tolerate a body that has already been unquoted,
/// so the function is robust to either shape.
///
/// SQLite semantics:
///
/// | Literal        | Bytes            | Notes                                  |
/// |----------------|------------------|----------------------------------------|
/// | `x'414243'`    | `41 42 43`       | two hex digits per byte, case-insens.  |
/// | `X'FF00'`      | `FF 00`          | leading `X` is equivalent to `x`       |
/// | `x''`          | *(empty)*        | the zero-byte blob                     |
/// | `x'012'`       | *error*          | odd digit count is a tokenizer error   |
///
/// The lexer's regex already guarantees the body is all hex digits, so the only
/// failure this needs to guard is an odd digit count; the non-hex check is kept
/// as defence in depth.
fn decode_blob_literal(raw: &str) -> Result<Vec<u8>, PlanError> {
    // Strip a leading `x'` / `X'` and the trailing `'` when present.
    let body = raw
        .strip_prefix("x'")
        .or_else(|| raw.strip_prefix("X'"))
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw);

    if body.len() % 2 != 0 {
        return Err(PlanError::UnsupportedStatement(format!(
            "blob literal has an odd number of hex digits: x'{body}'"
        )));
    }

    let hex = body.as_bytes();
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut i = 0;
    while i < hex.len() {
        // Decode a nibble pair straight from the byte slice. Slicing the *bytes*
        // (never the `&str`) sidesteps any UTF-8 char-boundary panic if a
        // non-ASCII body ever reaches here; `from_utf8` then rejects it as a
        // non-hex digit instead. `i` steps by two and `body.len()` is even, so
        // `i + 2 <= hex.len()` always holds — the slice cannot go out of bounds.
        let pair = std::str::from_utf8(&hex[i..i + 2])
            .ok()
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .ok_or_else(|| {
                PlanError::UnsupportedStatement(format!(
                    "blob literal has a non-hex digit: x'{body}'"
                ))
            })?;
        bytes.push(pair);
        i += 2;
    }
    Ok(bytes)
}

/// Plan a `column_ref` node.
///
/// Grammar: `column_ref = NAME [ "." NAME ]`
fn plan_column_ref(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    // Collect all NAME tokens in the column_ref.
    let names: Vec<String> = node
        .children
        .iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Token(tok) = c {
                if tok.value != "." {
                    Some(tok.value.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    match names.len() {
        0 => Err(PlanError::UnsupportedStatement(
            "empty column_ref".to_string(),
        )),
        1 => Ok(SqlExpr::Column {
            table: None,
            name: names[0].clone(),
        }),
        _ => Ok(SqlExpr::Column {
            table: Some(names[0].clone()),
            name: names[1].clone(),
        }),
    }
}

/// Plan a `function_call` node.
///
/// Grammar: `function_call = NAME "(" ( STAR | [ value_list ] ) ")"`
///
/// Aggregate functions (COUNT/SUM/AVG/MIN/MAX) produce `SqlExpr::Aggregate`.
/// All other function calls produce `SqlExpr::FunctionCall`.
fn plan_function_call(node: &GrammarASTNode) -> Result<SqlExpr, PlanError> {
    let name = first_name_token(node)
        .ok_or_else(|| PlanError::UnsupportedStatement("function_call without name".to_string()))?;

    // Check if this is an aggregate function.
    let agg_func = match name.to_uppercase().as_str() {
        "COUNT" => Some(AggFunc::Count),
        "SUM" => Some(AggFunc::Sum),
        "AVG" => Some(AggFunc::Avg),
        "MIN" => Some(AggFunc::Min),
        "MAX" => Some(AggFunc::Max),
        _ => None,
    };
    // `MIN(a, b, …)` / `MAX(a, b, …)` with two-or-more arguments are the SCALAR
    // largest/smallest functions, not the aggregate; fall through to the plain
    // `FunctionCall` path so the VM's `call_builtin` handles them.
    let agg_func = match agg_func {
        Some(AggFunc::Min | AggFunc::Max) if call_arg_count(node) >= 2 => None,
        other => other,
    };

    let has_star = has_token(node, "*");
    let distinct = has_token(node, "DISTINCT");

    if let Some(func) = agg_func {
        // Aggregate function.
        let arg = if has_star {
            None
        } else {
            // Find the argument expression.
            let arg_expr = node.children.iter().find_map(|c| {
                if let ASTNodeOrToken::Node(n) = c {
                    if n.rule_name == "value_list" {
                        n.children.iter().find_map(|c2| {
                            if let ASTNodeOrToken::Node(n2) = c2 {
                                plan_expression(n2).ok()
                            } else {
                                None
                            }
                        })
                    } else {
                        plan_expression(n).ok()
                    }
                } else {
                    None
                }
            });
            arg_expr
        };
        return Ok(SqlExpr::Aggregate {
            func,
            arg: arg.map(Box::new),
            distinct,
        });
    }

    // Regular (non-aggregate) function call.
    if has_star {
        return Ok(SqlExpr::FunctionCall {
            name,
            args: Vec::new(),
            star: true,
        });
    }

    // Collect arguments from value_list.
    let args: Vec<SqlExpr> = if let Some(vl) = find_node(node, "value_list") {
        vl.children
            .iter()
            .filter_map(|c| {
                if let ASTNodeOrToken::Node(n) = c {
                    plan_expression(n).ok()
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(SqlExpr::FunctionCall {
        name,
        args,
        star: false,
    })
}

// ===========================================================================
// Generic binary expression helper
// ===========================================================================

/// Plan a left-associative binary operator chain.
///
/// This generic helper handles rules like `additive = multiplicative { op multiplicative }`
/// by walking the children and folding them left-to-right:
///
/// `a + b + c` → `BinaryOp(Add, BinaryOp(Add, a, b), c)`
///
/// The `op_matcher` function maps a token text string to a `BinaryOp`,
/// returning `None` for tokens that are not operators.
fn plan_left_assoc_binary<F>(node: &GrammarASTNode, op_matcher: F) -> Result<SqlExpr, PlanError>
where
    F: Fn(&str) -> Option<BinaryOp>,
{
    // Separate children into operands (nodes) and operators (tokens).
    let mut operands: Vec<&GrammarASTNode> = Vec::new();
    let mut operators: Vec<BinaryOp> = Vec::new();

    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) => operands.push(n),
            ASTNodeOrToken::Token(tok) => {
                if let Some(op) = op_matcher(&tok.value) {
                    operators.push(op);
                }
            }
        }
    }

    if operands.is_empty() {
        return Err(PlanError::UnsupportedStatement(format!(
            "empty binary expression: {:?}",
            node.rule_name
        )));
    }

    if operands.len() == 1 {
        // No operators — passthrough to the single child.
        return plan_expression(operands[0]);
    }

    // Build left-associative tree.
    let mut result = plan_expression(operands[0])?;
    for (op, operand) in operators.into_iter().zip(operands[1..].iter()) {
        let right = plan_expression(operand)?;
        result = SqlExpr::BinaryOp {
            op,
            left: Box::new(result),
            right: Box::new(right),
        };
    }

    Ok(result)
}

// ===========================================================================
// AST traversal helpers
// ===========================================================================

/// Find the first child node with a specific `rule_name`.
/// Count the top-level arguments of a `function_call` node — the number of
/// expression children inside its `value_list`. Used to distinguish the
/// single-argument aggregate `MIN`/`MAX` from the two-or-more-argument scalar
/// forms. A call with no `value_list` (`COUNT(*)` or no args) counts as 0.
fn call_arg_count(node: &GrammarASTNode) -> usize {
    find_node(node, "value_list")
        .map(|vl| {
            vl.children
                .iter()
                .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
                .count()
        })
        .unwrap_or(0)
}

fn find_node<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == rule {
                return Some(n);
            }
        }
    }
    None
}

/// Collect all child nodes with a specific `rule_name`.
fn find_nodes<'a>(node: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == rule {
                    Some(n)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Check whether a node has a direct `Token` child whose text (case-insensitively)
/// matches `text`.
fn has_token(node: &GrammarASTNode, text: &str) -> bool {
    let upper = text.to_uppercase();
    node.children.iter().any(|c| {
        if let ASTNodeOrToken::Token(tok) = c {
            tok.value.to_uppercase() == upper
        } else {
            false
        }
    })
}

/// Get the text of a `ASTNodeOrToken` — returns the token text for tokens,
/// or empty string for nodes.
fn token_text_of(child: &ASTNodeOrToken) -> String {
    match child {
        ASTNodeOrToken::Token(tok) => tok.value.clone(),
        ASTNodeOrToken::Node(_) => String::new(),
    }
}

/// Return true if the child is a Token with the given keyword text.
fn is_keyword_token(child: &ASTNodeOrToken, keyword: &str) -> bool {
    if let ASTNodeOrToken::Token(tok) = child {
        tok.value.to_uppercase() == keyword.to_uppercase()
    } else {
        false
    }
}

/// Get the text of the first `Token` in a node (the name/keyword token).
fn first_name_token(node: &GrammarASTNode) -> Option<String> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(tok) = child {
            let val = &tok.value;
            // Skip punctuation tokens.
            if val != "(" && val != ")" && val != "," && val != "." && val != "*" {
                return Some(val.clone());
            }
        }
    }
    None
}

/// Extract the first name-like token value from a node.
///
/// Used for extracting table names from UPDATE/DELETE statements.
fn extract_first_name_token(node: &GrammarASTNode) -> Result<String, PlanError> {
    // Skip leading keywords like UPDATE, DELETE, FROM, etc.
    let skip = ["UPDATE", "DELETE", "FROM", "SET", "WHERE"];
    for child in &node.children {
        if let ASTNodeOrToken::Token(tok) = child {
            let upper = tok.value.to_uppercase();
            if !skip.contains(&upper.as_str()) && tok.value != "," {
                return Ok(tok.value.clone());
            }
        }
    }
    Err(PlanError::UnsupportedStatement(
        "statement without table name".to_string(),
    ))
}

/// Extract the NAME token that follows a specific keyword.
fn extract_name_after_keyword(node: &GrammarASTNode, keyword: &str) -> Result<String, PlanError> {
    let children = &node.children;
    let keyword_upper = keyword.to_uppercase();
    for (i, child) in children.iter().enumerate() {
        if let ASTNodeOrToken::Token(tok) = child {
            if tok.value.to_uppercase() == keyword_upper {
                // Next token is the name.
                if let Some(next) = children.get(i + 1) {
                    let name = token_text_of(next);
                    if !name.is_empty() {
                        return Ok(name);
                    }
                }
            }
        }
    }
    Err(PlanError::UnsupportedStatement(format!(
        "no name after keyword {:?}",
        keyword
    )))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_sql_backend::BackendError;
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // Mock schema for tests
    // -----------------------------------------------------------------------

    /// A simple in-memory schema provider for unit tests.
    ///
    /// This lets us test the planner's schema-validation logic without
    /// setting up a real database backend.
    ///
    /// # Usage
    ///
    /// ```rust
    /// let schema = MockSchema::with_tables(&[
    ///     ("users", &["id", "name", "age"]),
    ///     ("orders", &["id", "user_id", "total"]),
    /// ]);
    /// ```
    struct MockSchema {
        tables: HashMap<String, Vec<String>>,
    }

    impl MockSchema {
        /// Create a MockSchema from a list of `(table_name, [column_names])` pairs.
        fn with_tables(tables: &[(&str, &[&str])]) -> Self {
            Self {
                tables: tables
                    .iter()
                    .map(|(t, cols)| {
                        (
                            t.to_string(),
                            cols.iter().map(|c| c.to_string()).collect(),
                        )
                    })
                    .collect(),
            }
        }
    }

    impl SchemaProvider for MockSchema {
        fn column_names(&self, table: &str) -> Result<Vec<String>, BackendError> {
            self.tables
                .get(table)
                .cloned()
                .ok_or_else(|| BackendError::TableNotFound {
                    table: table.to_string(),
                })
        }
    }

    /// Build a test schema with standard tables.
    fn test_schema() -> MockSchema {
        MockSchema::with_tables(&[
            ("users", &["id", "name", "age", "city", "status"]),
            ("orders", &["id", "user_id", "total", "status"]),
            ("products", &["id", "name", "price"]),
            ("employees", &["id", "dept", "salary"]),
            ("t", &["id", "x", "y", "name", "a", "b", "c"]),
            ("a", &["id", "val"]),
            ("b", &["id", "val"]),
        ])
    }

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    fn plan_ok(sql: &str) -> LogicalPlan {
        let schema = test_schema();
        plan_sql(sql, &schema).unwrap_or_else(|e| panic!("plan_sql({sql:?}) failed: {e}"))
    }

    fn plan_err(sql: &str) -> PlanError {
        let schema = test_schema();
        plan_sql(sql, &schema).expect_err(&format!("Expected error for {sql:?}"))
    }

    // -----------------------------------------------------------------------
    // C1: SELECT * FROM table
    // -----------------------------------------------------------------------

    /// The simplest possible plan: project all columns from a single scan.
    ///
    /// ```text
    /// Project { * }
    ///   Scan { users }
    /// ```
    #[test]
    fn test_select_star() {
        let plan = plan_ok("SELECT * FROM users");
        assert!(
            matches!(plan, LogicalPlan::Project { .. }),
            "Root must be Project, got: {plan:?}"
        );
        if let LogicalPlan::Project { input, columns } = &plan {
            assert!(
                matches!(columns[0].expr, SqlExpr::Column { name: ref n, .. } if n == "*"),
                "Expected * projection"
            );
            assert!(matches!(**input, LogicalPlan::Scan { .. }));
        }
    }

    // -----------------------------------------------------------------------
    // C2: SELECT columns FROM table WHERE predicate
    // -----------------------------------------------------------------------

    /// Filter pushes beneath projection.
    ///
    /// ```text
    /// Project { id, name }
    ///   Filter { age > 18 }
    ///     Scan { users }
    /// ```
    #[test]
    fn test_select_where() {
        let plan = plan_ok("SELECT id, name FROM users WHERE age > 18");
        if let LogicalPlan::Project { input, .. } = &plan {
            assert!(matches!(**input, LogicalPlan::Filter { .. }), "Expected Filter below Project");
            if let LogicalPlan::Filter { input: scan, predicate } = input.as_ref() {
                assert!(matches!(**scan, LogicalPlan::Scan { .. }));
                assert!(
                    matches!(predicate, SqlExpr::BinaryOp { op: BinaryOp::Gt, .. }),
                    "Expected GT comparison"
                );
            }
        } else {
            panic!("Expected Project root, got: {plan:?}");
        }
    }

    // -----------------------------------------------------------------------
    // C3: SELECT aggregate FROM table GROUP BY column
    // -----------------------------------------------------------------------

    /// Aggregation with GROUP BY.
    ///
    /// ```text
    /// Project { status, COUNT(*) }
    ///   Aggregate { group_by: [status], COUNT(*) }
    ///     Scan { orders }
    /// ```
    #[test]
    fn test_select_group_by_count() {
        let plan = plan_ok("SELECT status, COUNT(*) FROM orders GROUP BY status");
        if let LogicalPlan::Project { input, .. } = &plan {
            assert!(matches!(**input, LogicalPlan::Aggregate { .. }), "Expected Aggregate below Project");
        } else {
            panic!("Expected Project root, got: {plan:?}");
        }
    }

    // -----------------------------------------------------------------------
    // C4: JOIN
    // -----------------------------------------------------------------------

    /// Inner join produces: Project(Join(Scan, Scan)).
    ///
    /// ```text
    /// Project { * }
    ///   Join { INNER, a.id = b.id }
    ///     Scan { a }
    ///     Scan { b }
    /// ```
    #[test]
    fn test_select_inner_join() {
        let plan = plan_ok("SELECT * FROM a INNER JOIN b ON a.id = b.id");
        if let LogicalPlan::Project { input, .. } = &plan {
            assert!(matches!(**input, LogicalPlan::Join { kind: JoinKind::Inner, .. }), "Expected inner Join");
            if let LogicalPlan::Join { left, right, condition, .. } = input.as_ref() {
                assert!(matches!(**left, LogicalPlan::Scan { .. }));
                assert!(matches!(**right, LogicalPlan::Scan { .. }));
                assert!(condition.is_some(), "Expected ON condition");
            }
        } else {
            panic!("Expected Project root");
        }
    }

    // -----------------------------------------------------------------------
    // C5: ORDER BY
    // -----------------------------------------------------------------------

    /// Sort is below Project (outermost).
    ///
    /// ```text
    /// Project { * }
    ///   Sort { name DESC }
    ///     Scan { users }
    /// ```
    #[test]
    fn test_select_order_by_desc() {
        let plan = plan_ok("SELECT * FROM users ORDER BY name DESC");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Sort { keys, .. } = input.as_ref() {
                assert_eq!(keys.len(), 1);
                assert!(!keys[0].ascending, "Expected DESC");
            } else {
                panic!("Expected Sort below Project");
            }
        } else {
            panic!("Expected Project root");
        }
    }

    // -----------------------------------------------------------------------
    // C6: LIMIT / OFFSET
    // -----------------------------------------------------------------------

    /// Limit is below Project.
    ///
    /// ```text
    /// Project { * }
    ///   Limit { count: 10, offset: 5 }
    ///     Sort { name ASC }
    ///       Scan { t }
    /// ```
    #[test]
    fn test_select_limit_offset() {
        let plan = plan_ok("SELECT * FROM t ORDER BY name LIMIT 10 OFFSET 5");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Limit { count, offset, .. } = input.as_ref() {
                assert_eq!(*count, Some(10));
                assert_eq!(*offset, Some(5));
            } else {
                panic!("Expected Limit below Project, got: {input:?}");
            }
        } else {
            panic!("Expected Project root");
        }
    }

    /// MySQL shorthand `LIMIT off, count` swaps the arguments: `LIMIT 5, 10`
    /// means `LIMIT 10 OFFSET 5` (offset 5, count 10) — the reverse of the
    /// `OFFSET` form. Both spellings must yield the SAME Limit plan.
    #[test]
    fn test_select_limit_comma_form() {
        let plan = plan_ok("SELECT * FROM t ORDER BY name LIMIT 5, 10");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Limit { count, offset, .. } = input.as_ref() {
                assert_eq!(*count, Some(10), "comma form: second number is the count");
                assert_eq!(*offset, Some(5), "comma form: first number is the offset");
            } else {
                panic!("Expected Limit below Project, got: {input:?}");
            }
        } else {
            panic!("Expected Project root");
        }
    }

    // -----------------------------------------------------------------------
    // C7: DISTINCT
    // -----------------------------------------------------------------------

    /// DISTINCT wraps Project's input before sort/limit.
    ///
    /// ```text
    /// Project { city }
    ///   Distinct
    ///     Scan { users }
    /// ```
    #[test]
    fn test_select_distinct() {
        let plan = plan_ok("SELECT DISTINCT city FROM users");
        if let LogicalPlan::Project { input, .. } = &plan {
            assert!(matches!(**input, LogicalPlan::Distinct(_)), "Expected Distinct below Project");
        } else {
            panic!("Expected Project root");
        }
    }

    // -----------------------------------------------------------------------
    // C8: INSERT
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_with_columns() {
        let plan = plan_ok("INSERT INTO t (a, b) VALUES (1, 'x')");
        if let LogicalPlan::Insert { table, columns, source: InsertSource::Values(rows) } = &plan {
            assert_eq!(table, "t");
            let expected: Vec<String> = vec!["a".to_string(), "b".to_string()];
            assert_eq!(columns.as_deref(), Some(expected.as_slice()));
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].len(), 2);
        } else {
            panic!("Expected Insert, got: {plan:?}");
        }
    }

    #[test]
    fn test_insert_without_columns() {
        // When there is no explicit column list, the planner resolves column
        // names from the schema so the codegen always has named columns.
        let plan = plan_ok("INSERT INTO t VALUES (1, 2, 3)");
        if let LogicalPlan::Insert { columns, .. } = &plan {
            // Table "t" is in the test schema with columns [id, x, y, name, a, b, c].
            // The planner resolves the first 3 positional columns.
            assert!(
                columns.is_some(),
                "Expected resolved columns from schema, got None"
            );
        } else {
            panic!("Expected Insert, got: {plan:?}");
        }
    }

    #[test]
    fn test_insert_multiple_rows() {
        let plan = plan_ok("INSERT INTO t VALUES (1), (2), (3)");
        if let LogicalPlan::Insert { source: InsertSource::Values(rows), .. } = &plan {
            assert_eq!(rows.len(), 3, "Expected 3 rows");
        } else {
            panic!("Expected Insert with 3 rows");
        }
    }

    // -----------------------------------------------------------------------
    // C9: UPDATE
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_with_where() {
        let plan = plan_ok("UPDATE t SET x = 1 WHERE id = 5");
        if let LogicalPlan::Update { table, assignments, predicate } = &plan {
            assert_eq!(table, "t");
            assert_eq!(assignments.len(), 1);
            assert_eq!(assignments[0].column, "x");
            assert!(predicate.is_some());
        } else {
            panic!("Expected Update, got: {plan:?}");
        }
    }

    #[test]
    fn test_update_multiple_assignments() {
        let plan = plan_ok("UPDATE t SET x = 1, y = 2");
        if let LogicalPlan::Update { assignments, predicate, .. } = &plan {
            assert_eq!(assignments.len(), 2);
            assert!(predicate.is_none());
        } else {
            panic!("Expected Update");
        }
    }

    // -----------------------------------------------------------------------
    // C10: DELETE
    // -----------------------------------------------------------------------

    #[test]
    fn test_delete_with_where() {
        let plan = plan_ok("DELETE FROM t WHERE id = 99");
        if let LogicalPlan::Delete { table, predicate } = &plan {
            assert_eq!(table, "t");
            assert!(predicate.is_some());
        } else {
            panic!("Expected Delete, got: {plan:?}");
        }
    }

    #[test]
    fn test_delete_without_where() {
        let plan = plan_ok("DELETE FROM users");
        if let LogicalPlan::Delete { predicate, .. } = &plan {
            assert!(predicate.is_none());
        } else {
            panic!("Expected Delete");
        }
    }

    // -----------------------------------------------------------------------
    // C11: CREATE TABLE
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_table() {
        let plan = plan_ok("CREATE TABLE t (id INTEGER, name TEXT)");
        if let LogicalPlan::CreateTable { table, if_not_exists, columns } = &plan {
            assert_eq!(table, "t");
            assert!(!if_not_exists);
            assert_eq!(columns.len(), 2);
            assert_eq!(columns[0].name, "id");
            assert_eq!(columns[1].name, "name");
        } else {
            panic!("Expected CreateTable, got: {plan:?}");
        }
    }

    #[test]
    fn test_create_table_if_not_exists() {
        let plan = plan_ok("CREATE TABLE IF NOT EXISTS t (id INTEGER)");
        if let LogicalPlan::CreateTable { if_not_exists, .. } = &plan {
            assert!(*if_not_exists, "Expected IF NOT EXISTS = true");
        } else {
            panic!("Expected CreateTable");
        }
    }

    #[test]
    fn test_create_table_with_constraints() {
        let plan = plan_ok("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)");
        if let LogicalPlan::CreateTable { columns, .. } = &plan {
            assert!(columns[0].primary_key, "id should be PRIMARY KEY");
            assert!(columns[0].not_null, "PRIMARY KEY implies NOT NULL");
            assert!(columns[1].not_null, "name should be NOT NULL");
        } else {
            panic!("Expected CreateTable");
        }
    }

    #[test]
    fn test_create_table_stores_column_collation() {
        // NOCASE / RTRIM are stored uppercased; an explicit BINARY collapses to
        // None (the default), so `has-collation` is unambiguous downstream.
        let plan = plan_ok(
            "CREATE TABLE t2 (a TEXT COLLATE NOCASE, b TEXT COLLATE RTRIM, \
             c TEXT COLLATE BINARY, d TEXT)",
        );
        if let LogicalPlan::CreateTable { columns, .. } = &plan {
            assert_eq!(columns[0].collation.as_deref(), Some("NOCASE"));
            assert_eq!(columns[1].collation.as_deref(), Some("RTRIM"));
            assert_eq!(columns[2].collation, None, "explicit BINARY → None");
            assert_eq!(columns[3].collation, None, "no COLLATE → None");
        } else {
            panic!("Expected CreateTable");
        }
    }

    #[test]
    fn test_create_table_unknown_collation_errors() {
        // Matches SQLite's prepare-time "no such collating sequence" rejection.
        let err = plan_err("CREATE TABLE t2 (x TEXT COLLATE BOGUS)");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("no such collating sequence"),
            "unexpected error: {msg}"
        );
    }

    /// A schema whose single table `tc` declares `name` with a `NOCASE`
    /// collation, used to prove a bare ORDER BY key inherits it.
    struct CollSchema;
    impl SchemaProvider for CollSchema {
        fn column_names(&self, table: &str) -> Result<Vec<String>, BackendError> {
            if table.eq_ignore_ascii_case("tc") {
                Ok(vec!["id".to_string(), "name".to_string()])
            } else {
                Err(BackendError::TableNotFound {
                    table: table.to_string(),
                })
            }
        }
        fn table_collations(&self, table: &str) -> Result<Vec<(String, String)>, BackendError> {
            if table.eq_ignore_ascii_case("tc") {
                Ok(vec![("name".to_string(), "NOCASE".to_string())])
            } else {
                Ok(Vec::new())
            }
        }
    }

    /// Pull the first Sort node's keys out of a planned SELECT (Sort sits below
    /// the outermost Project).
    fn sort_keys(plan: &LogicalPlan) -> Vec<SortKey> {
        fn walk(p: &LogicalPlan) -> Option<Vec<SortKey>> {
            match p {
                LogicalPlan::Sort { keys, .. } => Some(keys.clone()),
                LogicalPlan::Project { input, .. }
                | LogicalPlan::Filter { input, .. }
                | LogicalPlan::Distinct(input)
                | LogicalPlan::Limit { input, .. } => walk(input),
                _ => None,
            }
        }
        walk(plan).unwrap_or_default()
    }

    #[test]
    fn test_order_by_inherits_column_collation() {
        // Bare `ORDER BY name` inherits the column's declared NOCASE sequence…
        let plan = plan_sql("SELECT name FROM tc ORDER BY name", &CollSchema).unwrap();
        assert_eq!(sort_keys(&plan)[0].collation.as_deref(), Some("NOCASE"));

        // …a qualified reference (`tc.name`) resolves the same way…
        let plan = plan_sql("SELECT name FROM tc ORDER BY tc.name", &CollSchema).unwrap();
        assert_eq!(sort_keys(&plan)[0].collation.as_deref(), Some("NOCASE"));

        // …and through a table alias (`u.name`).
        let plan = plan_sql("SELECT u.name FROM tc AS u ORDER BY u.name", &CollSchema).unwrap();
        assert_eq!(sort_keys(&plan)[0].collation.as_deref(), Some("NOCASE"));
    }

    #[test]
    fn test_explicit_collate_overrides_column_collation() {
        // An explicit COLLATE BINARY on the key wins over the column's NOCASE.
        let plan =
            plan_sql("SELECT name FROM tc ORDER BY name COLLATE BINARY", &CollSchema).unwrap();
        assert_eq!(sort_keys(&plan)[0].collation, None, "BINARY → no collation");
    }

    #[test]
    fn test_order_by_column_without_collation_stays_binary() {
        // `id` has no declared collation, so its sort key stays BINARY (None).
        let plan = plan_sql("SELECT id FROM tc ORDER BY id", &CollSchema).unwrap();
        assert_eq!(sort_keys(&plan)[0].collation, None);
    }

    /// Pull the WHERE `Filter` predicate out of a plan built against a given
    /// schema (Filter sits under the outermost Project).
    fn filter_predicate(plan: &LogicalPlan) -> SqlExpr {
        fn walk(p: &LogicalPlan) -> Option<SqlExpr> {
            match p {
                LogicalPlan::Filter { predicate, .. } => Some(predicate.clone()),
                LogicalPlan::Project { input, .. }
                | LogicalPlan::Sort { input, .. }
                | LogicalPlan::Distinct(input)
                | LogicalPlan::Limit { input, .. } => walk(input),
                _ => None,
            }
        }
        walk(plan).expect("expected a Filter in the plan")
    }

    /// If `expr` is a `__collate(inner, 'NAME')` call, return `NAME`.
    fn collate_wrapper(expr: &SqlExpr) -> Option<String> {
        if let SqlExpr::FunctionCall { name, args, .. } = expr {
            if name == "__collate" {
                if let Some(SqlExpr::Literal(SqlValue::Text(coll))) = args.get(1) {
                    return Some(coll.clone());
                }
            }
        }
        None
    }

    #[test]
    fn test_where_comparison_inherits_column_collation() {
        // A bare `WHERE name = 'x'` on a NOCASE column wraps BOTH operands in
        // `__collate(_, 'NOCASE')`, so the byte comparison folds case.
        let plan = plan_sql("SELECT id FROM tc WHERE name = 'apple'", &CollSchema).unwrap();
        let SqlExpr::BinaryOp { op, left, right } = filter_predicate(&plan) else {
            panic!("expected a comparison predicate");
        };
        assert_eq!(op, BinaryOp::Eq);
        assert_eq!(collate_wrapper(&left).as_deref(), Some("NOCASE"));
        assert_eq!(collate_wrapper(&right).as_deref(), Some("NOCASE"));
    }

    #[test]
    fn test_where_explicit_collate_binary_overrides_column() {
        // An explicit `COLLATE BINARY` outranks the column's NOCASE: the operands
        // are wrapped in the identity `__collate(_, 'BINARY')`, NOT NOCASE, so the
        // comparison is byte-exact.
        let plan =
            plan_sql("SELECT id FROM tc WHERE name = 'apple' COLLATE BINARY", &CollSchema).unwrap();
        let SqlExpr::BinaryOp { left, right, .. } = filter_predicate(&plan) else {
            panic!("expected a comparison predicate");
        };
        assert_eq!(collate_wrapper(&left).as_deref(), Some("BINARY"));
        assert_eq!(collate_wrapper(&right).as_deref(), Some("BINARY"));
    }

    #[test]
    fn test_where_binary_left_column_does_not_inherit_right_collation() {
        // `id = name`: the LEFT operand `id` is a column with the default BINARY
        // collation, so it determines the comparison — the pass must NOT fall
        // through to the right NOCASE column `name`. Result: no `__collate` wrap.
        let plan = plan_sql("SELECT id FROM tc WHERE id = name", &CollSchema).unwrap();
        let SqlExpr::BinaryOp { left, right, .. } = filter_predicate(&plan) else {
            panic!("expected a comparison predicate");
        };
        assert_eq!(collate_wrapper(&left), None, "BINARY left column stays bare");
        assert_eq!(collate_wrapper(&right), None);

        // The mirror `name = id` is driven by the NOCASE left column, so BOTH
        // operands are wrapped in NOCASE.
        let plan = plan_sql("SELECT id FROM tc WHERE name = id", &CollSchema).unwrap();
        let SqlExpr::BinaryOp { left, right, .. } = filter_predicate(&plan) else {
            panic!("expected a comparison predicate");
        };
        assert_eq!(collate_wrapper(&left).as_deref(), Some("NOCASE"));
        assert_eq!(collate_wrapper(&right).as_deref(), Some("NOCASE"));
    }

    #[test]
    fn test_where_plain_column_comparison_stays_binary() {
        // `id` has no declared collation, so its comparison is left bare — no
        // `__collate` wrapping, plain byte comparison.
        let plan = plan_sql("SELECT id FROM tc WHERE id = 5", &CollSchema).unwrap();
        let SqlExpr::BinaryOp { left, right, .. } = filter_predicate(&plan) else {
            panic!("expected a comparison predicate");
        };
        assert_eq!(collate_wrapper(&left), None);
        assert_eq!(collate_wrapper(&right), None);
    }

    #[test]
    fn test_where_column_collation_flows_through_and_or() {
        // The pass recurses through boolean connectives: each comparison under an
        // `OR` independently picks up the column's NOCASE collation.
        let plan = plan_sql(
            "SELECT id FROM tc WHERE name = 'a' OR name = 'b'",
            &CollSchema,
        )
        .unwrap();
        let SqlExpr::BinaryOp { op, left, right } = filter_predicate(&plan) else {
            panic!("expected a boolean predicate");
        };
        assert_eq!(op, BinaryOp::Or);
        for side in [&left, &right] {
            let SqlExpr::BinaryOp {
                left: l,
                right: r,
                ..
            } = side.as_ref()
            else {
                panic!("expected a comparison on each side of OR");
            };
            assert_eq!(collate_wrapper(l).as_deref(), Some("NOCASE"));
            assert_eq!(collate_wrapper(r).as_deref(), Some("NOCASE"));
        }
    }

    // -----------------------------------------------------------------------
    // C12: DROP TABLE
    // -----------------------------------------------------------------------

    #[test]
    fn test_drop_table() {
        let plan = plan_ok("DROP TABLE t");
        if let LogicalPlan::DropTable { table, if_exists } = &plan {
            assert_eq!(table, "t");
            assert!(!if_exists);
        } else {
            panic!("Expected DropTable, got: {plan:?}");
        }
    }

    #[test]
    fn test_drop_table_if_exists() {
        let plan = plan_ok("DROP TABLE IF EXISTS t");
        if let LogicalPlan::DropTable { if_exists, .. } = &plan {
            assert!(*if_exists, "Expected IF EXISTS = true");
        } else {
            panic!("Expected DropTable");
        }
    }

    // -----------------------------------------------------------------------
    // Expression tests
    // -----------------------------------------------------------------------

    /// Helper: plan a WHERE expression from a simple SELECT.
    fn plan_where_expr(sql: &str) -> SqlExpr {
        let plan = plan_ok(sql);
        // Navigate: Project -> Filter -> predicate
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Filter { predicate, .. } = input.as_ref() {
                return predicate.clone();
            }
        }
        panic!("Expected Project(Filter(...)) for {sql:?}");
    }

    // -----------------------------------------------------------------------
    // E1: Literal expressions
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_integer_literal() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x = 42");
        if let SqlExpr::BinaryOp { op: BinaryOp::Eq, right, .. } = expr {
            assert_eq!(*right, SqlExpr::Literal(SqlValue::Int(42)));
        } else {
            panic!("Expected EQ comparison");
        }
    }

    #[test]
    fn test_expr_string_literal() {
        let expr = plan_where_expr("SELECT * FROM t WHERE name = 'hello'");
        if let SqlExpr::BinaryOp { right, .. } = expr {
            assert_eq!(*right, SqlExpr::Literal(SqlValue::Text("hello".to_string())));
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_expr_null_literal() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x IS NULL");
        assert!(matches!(expr, SqlExpr::IsNull(_)));
    }

    #[test]
    fn test_expr_blob_literal() {
        // `x'414243'` → the three bytes `A B C`; upper-case `X'…'` is equivalent.
        let expr = plan_where_expr("SELECT * FROM t WHERE b = x'414243'");
        if let SqlExpr::BinaryOp { right, .. } = expr {
            assert_eq!(*right, SqlExpr::Literal(SqlValue::Blob(vec![0x41, 0x42, 0x43])));
        } else {
            panic!("Expected blob literal");
        }
        let upper = plan_where_expr("SELECT * FROM t WHERE b = X'FF00'");
        if let SqlExpr::BinaryOp { right, .. } = upper {
            assert_eq!(*right, SqlExpr::Literal(SqlValue::Blob(vec![0xFF, 0x00])));
        } else {
            panic!("Expected upper-case blob literal");
        }
    }

    #[test]
    fn test_decode_blob_literal() {
        assert_eq!(decode_blob_literal("x'414243'").unwrap(), vec![0x41, 0x42, 0x43]);
        assert_eq!(decode_blob_literal("X'FF00'").unwrap(), vec![0xFF, 0x00]);
        // Empty blob.
        assert_eq!(decode_blob_literal("x''").unwrap(), Vec::<u8>::new());
        // Already-unquoted body is tolerated.
        assert_eq!(decode_blob_literal("0a0B").unwrap(), vec![0x0A, 0x0B]);
        // Odd digit count is rejected (matches SQLite's tokenizer error).
        assert!(decode_blob_literal("x'012'").is_err());
    }

    #[test]
    fn test_expr_true_false_literal() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x = TRUE");
        if let SqlExpr::BinaryOp { right, .. } = expr {
            assert_eq!(*right, SqlExpr::Literal(SqlValue::Bool(true)));
        } else {
            panic!("Expected TRUE literal");
        }
    }

    // -----------------------------------------------------------------------
    // E2: Column references
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_column_ref_unqualified() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x = 1");
        if let SqlExpr::BinaryOp { left, .. } = expr {
            assert!(
                matches!(*left, SqlExpr::Column { table: None, ref name } if name == "x"),
                "Expected unqualified column ref"
            );
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_expr_column_ref_qualified() {
        let plan = plan_ok("SELECT * FROM a INNER JOIN b ON a.id = b.id");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Join { condition: Some(cond), .. } = input.as_ref() {
                if let SqlExpr::BinaryOp { left, .. } = cond {
                    assert!(
                        matches!(*left.as_ref(), SqlExpr::Column { table: Some(ref t), .. } if t == "a"),
                        "Expected qualified column a.id"
                    );
                } else {
                    panic!("Expected BinaryOp in JOIN condition");
                }
            } else {
                panic!("Expected Join with condition");
            }
        }
    }

    // -----------------------------------------------------------------------
    // E3: Binary operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_arithmetic_add() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x + 1 > 5");
        if let SqlExpr::BinaryOp { op: BinaryOp::Gt, left, .. } = expr {
            assert!(matches!(*left, SqlExpr::BinaryOp { op: BinaryOp::Add, .. }));
        } else {
            panic!("Expected GT with ADD inside");
        }
    }

    #[test]
    fn test_expr_arithmetic_multiply() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x * 2 = 10");
        if let SqlExpr::BinaryOp { op: BinaryOp::Eq, left, .. } = expr {
            assert!(matches!(*left, SqlExpr::BinaryOp { op: BinaryOp::Mul, .. }));
        } else {
            panic!("Expected EQ with MUL inside");
        }
    }

    #[test]
    fn test_expr_comparison_neq() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x != 0");
        assert!(matches!(expr, SqlExpr::BinaryOp { op: BinaryOp::Neq, .. }));
    }

    #[test]
    fn test_expr_comparison_lte() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x <= 100");
        assert!(matches!(expr, SqlExpr::BinaryOp { op: BinaryOp::Lte, .. }));
    }

    // -----------------------------------------------------------------------
    // E4: Logical operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_and_or() {
        let expr = plan_where_expr("SELECT * FROM t WHERE a = 1 AND b = 2 OR c = 3");
        // OR is lower precedence, so: (a=1 AND b=2) OR c=3
        assert!(matches!(expr, SqlExpr::BinaryOp { op: BinaryOp::Or, .. }));
    }

    // -----------------------------------------------------------------------
    // E5: Unary operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_unary_neg() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x = -1");
        if let SqlExpr::BinaryOp { right, .. } = expr {
            // -1: could be UnaryOp::Neg(1) or Int(-1) depending on grammar.
            // Accept either form.
            let ok = matches!(*right, SqlExpr::UnaryOp { op: UnaryOp::Neg, .. })
                || matches!(*right, SqlExpr::Literal(SqlValue::Int(-1)));
            assert!(ok, "Expected negation, got: {right:?}");
        }
    }

    // -----------------------------------------------------------------------
    // E6: IS NULL / IS NOT NULL
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_is_null() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x IS NULL");
        assert!(matches!(expr, SqlExpr::IsNull(_)), "Expected IsNull");
    }

    #[test]
    fn test_expr_is_not_null() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x IS NOT NULL");
        assert!(matches!(expr, SqlExpr::IsNotNull(_)), "Expected IsNotNull");
    }

    // -----------------------------------------------------------------------
    // E7: BETWEEN
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_between() {
        let expr = plan_where_expr("SELECT * FROM t WHERE x BETWEEN 1 AND 10");
        if let SqlExpr::Between { negated, low, high, .. } = expr {
            assert!(!negated);
            assert!(matches!(*low, SqlExpr::Literal(SqlValue::Int(1))));
            assert!(matches!(*high, SqlExpr::Literal(SqlValue::Int(10))));
        } else {
            panic!("Expected Between");
        }
    }

    // -----------------------------------------------------------------------
    // E8: LIKE
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_like() {
        let expr = plan_where_expr("SELECT * FROM t WHERE name LIKE 'A%'");
        assert!(matches!(expr, SqlExpr::Like { negated: false, .. }), "Expected Like");
    }

    // -----------------------------------------------------------------------
    // E9: IN list
    // -----------------------------------------------------------------------

    #[test]
    fn test_expr_in_list() {
        let expr = plan_where_expr("SELECT * FROM t WHERE id IN (1, 2, 3)");
        if let SqlExpr::InList { list, negated, .. } = expr {
            assert!(!negated);
            assert_eq!(list.len(), 3);
        } else {
            panic!("Expected InList");
        }
    }

    // -----------------------------------------------------------------------
    // E10: Aggregate functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_agg_count_star() {
        let plan = plan_ok("SELECT COUNT(*) FROM users");
        if let LogicalPlan::Project { input, .. } = &plan {
            // Should have an Aggregate node.
            assert!(matches!(**input, LogicalPlan::Aggregate { .. }));
        }
    }

    #[test]
    fn test_agg_sum() {
        let plan = plan_ok("SELECT SUM(total) FROM orders GROUP BY status");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Aggregate { aggregates, .. } = input.as_ref() {
                assert!(
                    aggregates.iter().any(|a| matches!(a.func, AggFunc::Sum)),
                    "Expected SUM aggregate"
                );
            } else {
                panic!("Expected Aggregate below Project");
            }
        }
    }

    #[test]
    fn test_agg_avg() {
        let plan = plan_ok("SELECT AVG(salary) FROM employees GROUP BY dept");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Aggregate { aggregates, .. } = input.as_ref() {
                assert!(aggregates.iter().any(|a| matches!(a.func, AggFunc::Avg)));
            }
        }
    }

    #[test]
    fn test_agg_min_max() {
        let plan = plan_ok("SELECT MIN(salary), MAX(salary) FROM employees GROUP BY dept");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Aggregate { aggregates, .. } = input.as_ref() {
                assert!(aggregates.iter().any(|a| matches!(a.func, AggFunc::Min)));
                assert!(aggregates.iter().any(|a| matches!(a.func, AggFunc::Max)));
            }
        }
    }

    // -----------------------------------------------------------------------
    // E11: Function calls
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_call_scalar() {
        // A non-aggregate function in the select list.
        let plan = plan_ok("SELECT COUNT(*) FROM users");
        assert!(matches!(plan, LogicalPlan::Project { .. }));
    }

    // -----------------------------------------------------------------------
    // Plan stacking tests (verify exact nesting order)
    // -----------------------------------------------------------------------

    /// Verify the full pipeline for a complex query:
    ///
    /// ```text
    /// Project
    ///   Limit
    ///     Sort
    ///       Distinct
    ///         Scan
    /// ```
    #[test]
    fn test_plan_stacking_distinct_sort_limit() {
        let plan = plan_ok("SELECT DISTINCT city FROM users ORDER BY city LIMIT 5");
        // Project (outermost)
        let project_input = if let LogicalPlan::Project { input, .. } = &plan {
            input
        } else {
            panic!("Expected Project root");
        };
        // Limit
        let limit_input = if let LogicalPlan::Limit { input, count, .. } = project_input.as_ref() {
            assert_eq!(*count, Some(5));
            input
        } else {
            panic!("Expected Limit below Project");
        };
        // Sort
        let sort_input = if let LogicalPlan::Sort { input, .. } = limit_input.as_ref() {
            input
        } else {
            panic!("Expected Sort below Limit");
        };
        // Distinct
        let distinct_input = if let LogicalPlan::Distinct(input) = sort_input.as_ref() {
            input
        } else {
            panic!("Expected Distinct below Sort");
        };
        // Scan
        assert!(matches!(**distinct_input, LogicalPlan::Scan { .. }));
    }

    /// Full WHERE + GROUP BY + HAVING + ORDER + LIMIT pipeline.
    ///
    /// ```text
    /// Project
    ///   Limit
    ///     Sort
    ///       Having
    ///         Aggregate
    ///           Filter
    ///             Scan
    /// ```
    #[test]
    fn test_plan_full_pipeline() {
        let plan = plan_ok(
            "SELECT status, COUNT(*) FROM orders WHERE total > 0 \
             GROUP BY status HAVING COUNT(*) > 5 \
             ORDER BY status LIMIT 10",
        );
        let project_input = if let LogicalPlan::Project { input, .. } = &plan {
            input
        } else {
            panic!("Expected Project root");
        };
        let limit_input = if let LogicalPlan::Limit { input, .. } = project_input.as_ref() {
            input
        } else {
            panic!("Expected Limit below Project");
        };
        let sort_input = if let LogicalPlan::Sort { input, .. } = limit_input.as_ref() {
            input
        } else {
            panic!("Expected Sort below Limit");
        };
        let having_input = if let LogicalPlan::Having { input, .. } = sort_input.as_ref() {
            input
        } else {
            panic!("Expected Having below Sort");
        };
        let agg_input = if let LogicalPlan::Aggregate { input, .. } = having_input.as_ref() {
            input
        } else {
            panic!("Expected Aggregate below Having");
        };
        assert!(matches!(**agg_input, LogicalPlan::Filter { .. }), "Expected Filter below Aggregate");
    }

    // -----------------------------------------------------------------------
    // JOIN variants
    // -----------------------------------------------------------------------

    #[test]
    fn test_left_join() {
        let plan = plan_ok("SELECT * FROM a LEFT OUTER JOIN b ON a.id = b.id");
        if let LogicalPlan::Project { input, .. } = &plan {
            assert!(matches!(**input, LogicalPlan::Join { kind: JoinKind::Left, .. }));
        }
    }

    #[test]
    fn test_cross_join() {
        let plan = plan_ok("SELECT * FROM a CROSS JOIN b ON a.id = b.id");
        // Note: CROSS JOIN also requires a join type keyword; CROSS is valid.
        if let LogicalPlan::Project { input, .. } = &plan {
            assert!(matches!(**input, LogicalPlan::Join { kind: JoinKind::Cross, .. }));
        }
    }

    // -----------------------------------------------------------------------
    // Table alias
    // -----------------------------------------------------------------------

    #[test]
    fn test_table_alias() {
        let plan = plan_ok("SELECT * FROM users AS u");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Scan { alias, .. } = input.as_ref() {
                assert_eq!(alias.as_deref(), Some("u"));
            } else {
                panic!("Expected Scan with alias");
            }
        }
    }

    // -----------------------------------------------------------------------
    // SELECT with aliases
    // -----------------------------------------------------------------------

    #[test]
    fn test_select_item_alias() {
        let plan = plan_ok("SELECT age AS years FROM users");
        if let LogicalPlan::Project { columns, .. } = &plan {
            assert_eq!(columns[0].alias.as_deref(), Some("years"));
        } else {
            panic!("Expected Project");
        }
    }

    // -----------------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_unknown_table() {
        let err = plan_err("SELECT * FROM nonexistent_table");
        assert!(
            matches!(err, PlanError::UnknownTable(ref t) if t == "nonexistent_table"),
            "Expected UnknownTable error, got: {err:?}"
        );
    }

    #[test]
    fn test_error_parse_error() {
        let err = plan_err("SELECT FROM");
        assert!(matches!(err, PlanError::ParseError(_)), "Expected ParseError, got: {err:?}");
    }

    #[test]
    fn test_error_unknown_table_in_join() {
        let err = plan_err("SELECT * FROM users INNER JOIN no_such_table ON users.id = no_such_table.id");
        assert!(matches!(err, PlanError::UnknownTable(_)));
    }

    // -----------------------------------------------------------------------
    // plan_expr public API test
    // -----------------------------------------------------------------------

    #[test]
    fn test_plan_expr_public_api() {
        use coding_adventures_sql_parser::parse_sql;
        use parser::grammar_parser::ASTNodeOrToken;

        let ast = parse_sql("SELECT x FROM t WHERE x = 1").unwrap();
        // Navigate to the where_clause → expr.
        fn find_rule<'a>(node: &'a parser::grammar_parser::GrammarASTNode, rule: &str) -> Option<&'a parser::grammar_parser::GrammarASTNode> {
            if node.rule_name == rule { return Some(node); }
            for c in &node.children {
                if let ASTNodeOrToken::Node(n) = c {
                    if let Some(found) = find_rule(n, rule) { return Some(found); }
                }
            }
            None
        }
        if let Some(expr_node) = find_rule(&ast, "expr") {
            let result = plan_expr(expr_node);
            assert!(result.is_ok(), "plan_expr should succeed");
        }
    }

    // -----------------------------------------------------------------------
    // plan_sql convenience API
    // -----------------------------------------------------------------------

    #[test]
    fn test_plan_sql_convenience() {
        let schema = test_schema();
        let result = plan_sql("SELECT * FROM users", &schema);
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // PlanError Display
    // -----------------------------------------------------------------------

    #[test]
    fn test_plan_error_display() {
        let e = PlanError::UnknownTable("foo".to_string());
        assert!(e.to_string().contains("foo"));

        let e2 = PlanError::ParseError("bad sql".to_string());
        assert!(e2.to_string().contains("parse"));

        let e3 = PlanError::UnsupportedStatement("subquery".to_string());
        assert!(e3.to_string().contains("subquery"));
    }

    // -----------------------------------------------------------------------
    // ORDER BY multi-key
    // -----------------------------------------------------------------------

    #[test]
    fn test_order_by_multi_key() {
        let plan = plan_ok("SELECT * FROM users ORDER BY name ASC, age DESC");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Sort { keys, .. } = input.as_ref() {
                assert_eq!(keys.len(), 2);
                assert!(keys[0].ascending);
                assert!(!keys[1].ascending);
            }
        }
    }

    // -----------------------------------------------------------------------
    // LIMIT without OFFSET
    // -----------------------------------------------------------------------

    #[test]
    fn test_limit_without_offset() {
        let plan = plan_ok("SELECT * FROM users LIMIT 20");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Limit { count, offset, .. } = input.as_ref() {
                assert_eq!(*count, Some(20));
                assert_eq!(*offset, None);
            }
        }
    }

    // -----------------------------------------------------------------------
    // String literal test
    // -----------------------------------------------------------------------

    /// Tests that single-quoted string literals are correctly unwrapped.
    /// Note: the lexer does not support SQL's '' escape sequence for embedded quotes,
    /// so we test with a simple string without embedded quotes.
    #[test]
    fn test_string_literal_unwrapping() {
        let expr = plan_where_expr("SELECT * FROM t WHERE name = 'hello'");
        if let SqlExpr::BinaryOp { right, .. } = expr {
            assert_eq!(
                *right,
                SqlExpr::Literal(SqlValue::Text("hello".to_string()))
            );
        } else {
            panic!("Expected BinaryOp");
        }
    }

    // -----------------------------------------------------------------------
    // Multiple ASC/DESC keys
    // -----------------------------------------------------------------------

    #[test]
    fn test_sort_asc_default() {
        let plan = plan_ok("SELECT * FROM users ORDER BY name");
        if let LogicalPlan::Project { input, .. } = &plan {
            if let LogicalPlan::Sort { keys, .. } = input.as_ref() {
                assert!(keys[0].ascending, "Default ORDER BY should be ASC");
            }
        }
    }

    // -----------------------------------------------------------------------
    // ORDER BY positional (ordinal) column references
    // -----------------------------------------------------------------------

    /// Pull the sort keys out of a planned `SELECT … ORDER BY …`.
    fn sort_keys_of(sql: &str) -> Vec<SortKey> {
        match plan_ok(sql) {
            LogicalPlan::Project { input, .. } => match *input {
                LogicalPlan::Sort { keys, .. } => keys,
                other => panic!("expected Sort under Project, got {other:?}"),
            },
            other => panic!("expected Project at root, got {other:?}"),
        }
    }

    #[test]
    fn test_order_by_ordinal_resolves_to_nth_column() {
        // `ORDER BY 2` sorts by the 2nd output column (`b`), not the constant 2.
        let keys = sort_keys_of("SELECT a, b, c FROM t ORDER BY 2");
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys[0].expr,
            SqlExpr::Column {
                table: None,
                name: "b".to_string()
            }
        );
        assert!(keys[0].ascending);
    }

    #[test]
    fn test_order_by_ordinal_carries_direction() {
        // Direction and multi-key tie-breaks apply to the resolved columns.
        let keys = sort_keys_of("SELECT a, b, c FROM t ORDER BY 3 DESC, 1");
        assert_eq!(keys.len(), 2);
        assert_eq!(
            keys[0].expr,
            SqlExpr::Column {
                table: None,
                name: "c".to_string()
            }
        );
        assert!(!keys[0].ascending);
        assert_eq!(
            keys[1].expr,
            SqlExpr::Column {
                table: None,
                name: "a".to_string()
            }
        );
        assert!(keys[1].ascending);
    }

    #[test]
    fn test_order_by_integer_expression_is_not_positional() {
        // Only a BARE integer literal is positional. `1+0` is an expression that
        // happens to equal 1, so it sorts by the constant — it must stay a
        // BinaryOp, NOT be rewritten to the first output column.
        let keys = sort_keys_of("SELECT a, b FROM t ORDER BY 1+0");
        assert_eq!(keys.len(), 1);
        assert!(
            matches!(keys[0].expr, SqlExpr::BinaryOp { op: BinaryOp::Add, .. }),
            "1+0 must remain an expression, got {:?}",
            keys[0].expr
        );
    }

    #[test]
    fn test_order_by_ordinal_out_of_range_errors() {
        // `SELECT a` has one output column; `ORDER BY 2` (and `ORDER BY 0`) are
        // out of range, matching SQLite's prepare-time error.
        let err = plan_err("SELECT a FROM t ORDER BY 2");
        assert!(
            format!("{err}").contains("out of range"),
            "expected out-of-range error, got {err}"
        );
        let err0 = plan_err("SELECT a FROM t ORDER BY 0");
        assert!(format!("{err0}").contains("out of range"));
    }

    #[test]
    fn test_order_by_ordinal_over_star_is_unchanged() {
        // With `SELECT *` the output column count/identity isn't known at plan
        // time, so a positional key is left as the literal (no guess, no error).
        let keys = sort_keys_of("SELECT * FROM t ORDER BY 1");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].expr, SqlExpr::Literal(SqlValue::Int(1)));
    }
}
