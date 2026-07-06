/**
 * Types for the SQL query planner.
 *
 * The planner converts a raw AST from sql-parser into a typed LogicalPlan
 * tree, which represents a relational-algebra computation. Downstream stages
 * (sql-optimizer, sql-codegen) consume this tree.
 *
 * Expression grammar (what the planner can produce):
 *
 *   Expr = Literal(value)           -- constant SQL value
 *        | Column(table?, name)     -- column reference
 *        | Star                     -- SELECT *
 *        | Binary(op, left, right)  -- arithmetic / comparison / boolean
 *        | Unary(op, operand)       -- NOT, unary minus
 *        | Func(name, args)         -- scalar function call
 *        | Aggregate(func, arg?)    -- COUNT(*), SUM(x), ...
 *        | Between(expr, lo, hi)    -- expr BETWEEN lo AND hi
 *        | InList(expr, list)       -- expr IN (v1, v2, ...)
 *        | Like(expr, pattern)      -- expr LIKE '%foo%'
 *        | IsNull(expr)             -- expr IS NULL
 *        | Coalesce(args)           -- COALESCE(a, b, ...)
 *
 * Plan tree (bottom-to-top for a SELECT query):
 *
 *   ScanNode      -- open a table cursor
 *     FilterNode  -- apply WHERE predicate
 *     AggregateNode -- GROUP BY + aggregate accumulators
 *     HavingNode  -- apply HAVING predicate
 *     SortNode    -- ORDER BY
 *     ProjectNode -- SELECT list
 *     DistinctNode -- DISTINCT
 *     LimitNode   -- LIMIT / OFFSET
 */

/** SQL runtime value type, matching the backend's SqlValue. */
export type SqlValue = null | boolean | number | string;

// ---------------------------------------------------------------------------
// Expression types
// ---------------------------------------------------------------------------

export type Expr =
  | { kind: "literal"; value: SqlValue }
  | { kind: "column"; table: string | null; name: string }
  | { kind: "star" }
  | { kind: "binary"; op: string; left: Expr; right: Expr }
  | { kind: "unary"; op: string; operand: Expr }
  | { kind: "func"; name: string; args: Expr[] }
  | { kind: "aggregate"; func: string; arg: Expr | null; distinct: boolean }
  | { kind: "between"; expr: Expr; low: Expr; high: Expr; negated: boolean }
  | { kind: "in_list"; expr: Expr; list: Expr[]; negated: boolean }
  | { kind: "like"; expr: Expr; pattern: Expr; negated: boolean }
  | { kind: "is_null"; expr: Expr; negated: boolean }
  | { kind: "coalesce"; args: Expr[] };

/** An item in the SELECT list: an expression plus an optional alias. */
export interface ProjectItem {
  expr: Expr;
  alias: string | null;
}

/** A sort key: an expression plus sort direction. */
export interface SortKey {
  expr: Expr;
  ascending: boolean;
  /** NULLs sort last by default (SQL standard NULLS LAST for ASC). */
  nullsLast: boolean;
}

/** Aggregate function specification used inside AggregateNode. */
export interface AggregateSpec {
  /** Lowercase function name: "count", "sum", "avg", "min", "max". */
  func: string;
  /** NULL for COUNT(*). */
  arg: Expr | null;
  distinct: boolean;
  /** Internal alias used by subsequent plan nodes to reference this aggregate. */
  alias: string;
}

/** Column definition in a CREATE TABLE statement. */
export interface ColumnDefinition {
  name: string;
  /** Raw data-type string from SQL (TEXT, INTEGER, REAL, BLOB, etc.). */
  dataType: string;
  notNull: boolean;
  primaryKey: boolean;
  unique: boolean;
  defaultValue: Expr | null;
}

export type JoinType = "inner" | "left" | "right" | "full" | "cross";

// ---------------------------------------------------------------------------
// Logical plan node types
// ---------------------------------------------------------------------------

export type LogicalPlan =
  | ScanNode
  | FilterNode
  | ProjectNode
  | AggregateNode
  | HavingNode
  | SortNode
  | LimitNode
  | DistinctNode
  | JoinNode
  | InsertNode
  | UpdateNode
  | DeleteNode
  | CreateTableNode
  | DropTableNode
  | EmptyResultNode;

/** Open a full table scan. */
export interface ScanNode {
  type: "scan";
  table: string;
  /** Qualifier used to resolve unqualified column references. */
  alias: string;
}

/** Apply a boolean predicate to each row, keeping rows where it is TRUE. */
export interface FilterNode {
  type: "filter";
  input: LogicalPlan;
  predicate: Expr;
}

/**
 * Project a set of expressions to form the output columns.
 *
 * Items with `alias` starting with `__sort_` are hidden sentinel columns
 * emitted so that ORDER BY can reference non-projected columns; they are
 * stripped from the final result by the VM's `buildResult` step.
 */
export interface ProjectNode {
  type: "project";
  input: LogicalPlan;
  items: ProjectItem[];
}

/**
 * Partition rows by `keys` and compute `aggregates` for each partition.
 *
 * An empty `keys` array means the entire input is treated as a single group
 * (implicit aggregation: `SELECT COUNT(*) FROM t`).
 */
export interface AggregateNode {
  type: "aggregate";
  input: LogicalPlan;
  keys: Expr[];
  aggregates: AggregateSpec[];
}

/** Filter groups produced by AggregateNode using a predicate on aggregates. */
export interface HavingNode {
  type: "having";
  input: LogicalPlan;
  predicate: Expr;
}

/** Sort the result by `keys`. Applied after projection so aliases work. */
export interface SortNode {
  type: "sort";
  input: LogicalPlan;
  keys: SortKey[];
}

/** Limit and/or skip rows. */
export interface LimitNode {
  type: "limit";
  input: LogicalPlan;
  count: number;
  offset: number;
}

/** Remove duplicate rows from the output. */
export interface DistinctNode {
  type: "distinct";
  input: LogicalPlan;
}

/** Combine two input streams via a join. */
export interface JoinNode {
  type: "join";
  left: LogicalPlan;
  right: LogicalPlan;
  joinType: JoinType;
  condition: Expr | null;
}

/** INSERT INTO table [(cols)] VALUES rows... */
export interface InsertNode {
  type: "insert";
  table: string;
  columns: string[] | null;
  rows: Expr[][];
}

/** UPDATE table SET col=expr [WHERE pred] */
export interface UpdateNode {
  type: "update";
  table: string;
  assignments: { column: string; value: Expr }[];
  predicate: Expr | null;
}

/** DELETE FROM table [WHERE pred] */
export interface DeleteNode {
  type: "delete";
  table: string;
  predicate: Expr | null;
}

/** CREATE TABLE [IF NOT EXISTS] table (col_defs) */
export interface CreateTableNode {
  type: "create_table";
  table: string;
  columns: ColumnDefinition[];
  ifNotExists: boolean;
}

/** DROP TABLE [IF EXISTS] table */
export interface DropTableNode {
  type: "drop_table";
  table: string;
  ifExists: boolean;
}

/**
 * Sentinel produced by the optimizer's dead-code-elimination pass when a
 * plan node is provably empty (e.g. `WHERE 1=0`).
 */
export interface EmptyResultNode {
  type: "empty_result";
}
