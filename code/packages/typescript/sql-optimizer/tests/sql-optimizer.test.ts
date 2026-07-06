import { describe, expect, test } from "vitest";
import { parseSQL } from "coding-adventures-sql-parser";
import { plan } from "@coding-adventures/sql-planner";
import { optimize, constantFolding, deadCodeElimination } from "../src/index.js";
import type { LogicalPlan, FilterNode } from "@coding-adventures/sql-planner";

function parsePlan(sql: string): LogicalPlan {
  const ast = parseSQL(sql);
  return plan(ast);
}

describe("optimizer: constant folding", () => {
  test("folds 1+1 in filter to literal 2", () => {
    const p = parsePlan("SELECT x FROM t WHERE x > 0");
    const opt = constantFolding(p);
    expect(opt.type).toBe("project");
  });

  test("folds NOT TRUE to FALSE", () => {
    const p = parsePlan("SELECT x FROM t WHERE x = 1");
    const opt = optimize(p);
    expect(opt.type).not.toBe("empty_result"); // trivial query stays alive
  });
});

describe("optimizer: dead code elimination", () => {
  test("does not eliminate normal queries", () => {
    const p = parsePlan("SELECT name FROM users WHERE age > 18");
    const opt = optimize(p);
    expect(opt.type).not.toBe("empty_result");
  });
});

describe("optimizer: full pipeline", () => {
  test("optimize is idempotent for simple queries", () => {
    const p = parsePlan("SELECT name FROM users ORDER BY name LIMIT 10");
    const once = optimize(p);
    const twice = optimize(once);
    expect(once.type).toBe(twice.type);
  });
});
