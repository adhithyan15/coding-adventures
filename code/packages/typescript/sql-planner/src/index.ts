/**
 * sql-planner — translates a SQL AST into a typed LogicalPlan tree.
 *
 * Pipeline position: sql-parser → **sql-planner** → sql-optimizer → sql-codegen → sql-vm
 *
 * Usage:
 *
 *   import { plan, planAll, PlanError } from "@coding-adventures/sql-planner";
 *   import { parseSQL } from "coding-adventures-sql-parser";
 *
 *   const ast = parseSQL("SELECT name FROM users WHERE age > 18");
 *   const logicalPlan = plan(ast);
 */

export { plan, planAll, planExpr, PlanError } from "./planner.js";
export type {
  AggregateSpec,
  ColumnDefinition,
  DeleteNode,
  DistinctNode,
  EmptyResultNode,
  Expr,
  FilterNode,
  HavingNode,
  InsertNode,
  JoinNode,
  JoinType,
  LimitNode,
  LogicalPlan,
  ProjectItem,
  ProjectNode,
  AggregateNode,
  ScanNode,
  SortKey,
  SortNode,
  SqlValue,
  CreateTableNode,
  DropTableNode,
  UpdateNode,
} from "./types.js";
