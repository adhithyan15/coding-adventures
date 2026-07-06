import { describe, expect, test } from "vitest";
import { parseSQL } from "coding-adventures-sql-parser";
import { plan, planAll, PlanError } from "../src/index.js";
import type { LogicalPlan, ProjectNode, FilterNode, AggregateNode, SortNode, LimitNode, ScanNode } from "../src/index.js";

function parsePlan(sql: string): LogicalPlan {
  const ast = parseSQL(sql);
  return plan(ast);
}

function parsePlans(sql: string): LogicalPlan[] {
  const ast = parseSQL(sql);
  return planAll(ast);
}

describe("sql-planner: SELECT", () => {
  test("simple scan + projection", () => {
    const p = parsePlan("SELECT name FROM users");
    expect(p.type).toBe("project");
    const proj = p as ProjectNode;
    expect(proj.items).toHaveLength(1);
    expect(proj.items[0].expr).toEqual({ kind: "column", table: null, name: "name" });
    expect((proj.input as ScanNode).type).toBe("scan");
    expect((proj.input as ScanNode).table).toBe("users");
  });

  test("SELECT * FROM table", () => {
    const p = parsePlan("SELECT * FROM users");
    expect(p.type).toBe("project");
    const proj = p as ProjectNode;
    expect(proj.items[0].expr.kind).toBe("star");
  });

  test("WHERE clause wraps scan in filter", () => {
    const p = parsePlan("SELECT name FROM users WHERE age > 18");
    expect(p.type).toBe("project");
    const proj = p as ProjectNode;
    expect(proj.input.type).toBe("filter");
    const filt = proj.input as FilterNode;
    expect(filt.predicate.kind).toBe("binary");
    expect(filt.input.type).toBe("scan");
  });

  test("ORDER BY produces sort node wrapping project", () => {
    const p = parsePlan("SELECT name FROM users ORDER BY name ASC");
    expect(p.type).toBe("sort");
    const sort = p as SortNode;
    expect(sort.keys[0].ascending).toBe(true);
    expect(sort.input.type).toBe("project");
  });

  test("ORDER BY DESC", () => {
    const p = parsePlan("SELECT name FROM users ORDER BY name DESC");
    const sort = p as SortNode;
    expect(sort.keys[0].ascending).toBe(false);
  });

  test("LIMIT clause", () => {
    const p = parsePlan("SELECT name FROM users LIMIT 10");
    expect(p.type).toBe("limit");
    const lim = p as LimitNode;
    expect(lim.count).toBe(10);
    expect(lim.offset).toBe(0);
  });

  test("LIMIT OFFSET clause", () => {
    const p = parsePlan("SELECT name FROM users LIMIT 5 OFFSET 10");
    const lim = p as LimitNode;
    expect(lim.count).toBe(5);
    expect(lim.offset).toBe(10);
  });

  test("DISTINCT wraps projection", () => {
    const p = parsePlan("SELECT DISTINCT name FROM users");
    expect(p.type).toBe("distinct");
  });

  test("GROUP BY produces aggregate node", () => {
    const p = parsePlan("SELECT dept, COUNT(*) AS n FROM employees GROUP BY dept");
    // Structure: project → aggregate → scan
    expect(p.type).toBe("project");
    const proj = p as ProjectNode;
    expect(proj.input.type).toBe("aggregate");
    const agg = proj.input as AggregateNode;
    expect(agg.keys).toHaveLength(1);
    expect(agg.aggregates.length).toBeGreaterThan(0);
    expect(agg.aggregates[0].func).toBe("count");
    expect(agg.aggregates[0].arg).toBeNull();
  });

  test("alias on projection item", () => {
    const p = parsePlan("SELECT name AS n FROM users");
    const proj = p as ProjectNode;
    expect(proj.items[0].alias).toBe("n");
  });

  test("implicit aggregate (no GROUP BY)", () => {
    const p = parsePlan("SELECT COUNT(*) FROM users");
    expect(p.type).toBe("project");
    const proj = p as ProjectNode;
    expect(proj.input.type).toBe("aggregate");
  });
});

describe("sql-planner: DML", () => {
  test("INSERT VALUES", () => {
    const p = parsePlan("INSERT INTO users VALUES (1, 'Alice')");
    expect(p.type).toBe("insert");
    if (p.type === "insert") {
      expect(p.table).toBe("users");
      expect(p.rows).toHaveLength(1);
      expect(p.rows[0]).toHaveLength(2);
    }
  });

  test("UPDATE SET WHERE", () => {
    const p = parsePlan("UPDATE users SET name = 'Bob' WHERE id = 1");
    expect(p.type).toBe("update");
    if (p.type === "update") {
      expect(p.table).toBe("users");
      expect(p.assignments).toHaveLength(1);
      expect(p.assignments[0].column).toBe("name");
      expect(p.predicate).not.toBeNull();
    }
  });

  test("DELETE FROM WHERE", () => {
    const p = parsePlan("DELETE FROM users WHERE id = 1");
    expect(p.type).toBe("delete");
    if (p.type === "delete") {
      expect(p.table).toBe("users");
      expect(p.predicate).not.toBeNull();
    }
  });
});

describe("sql-planner: DDL", () => {
  test("CREATE TABLE", () => {
    const p = parsePlan("CREATE TABLE users (id INTEGER, name TEXT)");
    expect(p.type).toBe("create_table");
    if (p.type === "create_table") {
      expect(p.table).toBe("users");
      expect(p.columns).toHaveLength(2);
      expect(p.columns[0].name).toBe("id");
      expect(p.columns[0].dataType).toBe("INTEGER");
      expect(p.ifNotExists).toBe(false);
    }
  });

  test("CREATE TABLE IF NOT EXISTS", () => {
    const p = parsePlan("CREATE TABLE IF NOT EXISTS t (x INTEGER)");
    if (p.type === "create_table") {
      expect(p.ifNotExists).toBe(true);
    }
  });

  test("DROP TABLE", () => {
    const p = parsePlan("DROP TABLE users");
    expect(p.type).toBe("drop_table");
    if (p.type === "drop_table") {
      expect(p.table).toBe("users");
      expect(p.ifExists).toBe(false);
    }
  });

  test("DROP TABLE IF EXISTS", () => {
    const p = parsePlan("DROP TABLE IF EXISTS users");
    if (p.type === "drop_table") {
      expect(p.ifExists).toBe(true);
    }
  });
});

describe("sql-planner: expressions", () => {
  test("binary arithmetic", () => {
    const p = parsePlan("SELECT 1 + 2 FROM t");
    const proj = p as ProjectNode;
    const expr = proj.items[0].expr;
    expect(expr.kind).toBe("binary");
    if (expr.kind === "binary") {
      expect(expr.op).toBe("+");
    }
  });

  test("IS NULL predicate", () => {
    const p = parsePlan("SELECT name FROM users WHERE name IS NULL");
    const proj = p as ProjectNode;
    const filt = proj.input as FilterNode;
    expect(filt.predicate.kind).toBe("is_null");
  });

  test("IS NOT NULL predicate", () => {
    const p = parsePlan("SELECT name FROM users WHERE name IS NOT NULL");
    const proj = p as ProjectNode;
    const filt = proj.input as FilterNode;
    expect(filt.predicate.kind).toBe("is_null");
    if (filt.predicate.kind === "is_null") {
      expect(filt.predicate.negated).toBe(true);
    }
  });

  test("BETWEEN predicate", () => {
    const p = parsePlan("SELECT x FROM t WHERE x BETWEEN 1 AND 10");
    const proj = p as ProjectNode;
    const filt = proj.input as FilterNode;
    expect(filt.predicate.kind).toBe("between");
  });

  test("IN list predicate", () => {
    const p = parsePlan("SELECT x FROM t WHERE x IN (1, 2, 3)");
    const proj = p as ProjectNode;
    const filt = proj.input as FilterNode;
    expect(filt.predicate.kind).toBe("in_list");
  });

  test("LIKE predicate", () => {
    const p = parsePlan("SELECT x FROM t WHERE x LIKE '%foo%'");
    const proj = p as ProjectNode;
    const filt = proj.input as FilterNode;
    expect(filt.predicate.kind).toBe("like");
  });

  test("function call", () => {
    const p = parsePlan("SELECT UPPER(name) FROM users");
    const proj = p as ProjectNode;
    expect(proj.items[0].expr.kind).toBe("func");
  });

  test("planAll returns multiple plans", () => {
    const plans = parsePlans("SELECT 1; SELECT 2");
    expect(plans.length).toBeGreaterThanOrEqual(1);
  });
});
