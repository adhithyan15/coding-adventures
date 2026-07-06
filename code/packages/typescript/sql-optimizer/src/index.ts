/**
 * sql-optimizer — applies rewrite passes to a LogicalPlan.
 *
 * Pipeline position: sql-planner → **sql-optimizer** → sql-codegen → sql-vm
 *
 * Usage:
 *
 *   import { optimize } from "@coding-adventures/sql-optimizer";
 *   const optimized = optimize(logicalPlan);
 */

export { optimize, optimizeWithPasses, DEFAULT_PASSES, constantFolding, predicatePushdown, deadCodeElimination, limitPushdown } from "./optimizer.js";
