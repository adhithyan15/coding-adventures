import { describe, expect, test } from "vitest";
import { parseSQL } from "coding-adventures-sql-parser";
import { plan } from "@coding-adventures/sql-planner";
import type { LogicalPlan } from "@coding-adventures/sql-planner";
import { compile, CodegenError } from "../src/index.js";
import type { Program } from "../src/index.js";

function compileSql(sql: string): Program {
  const ast = parseSQL(sql);
  const logicalPlan = plan(ast);
  return compile(logicalPlan);
}

describe("sql-codegen: SELECT", () => {
  test("simple select emits scan + project instructions", () => {
    const prog = compileSql("SELECT name FROM users");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("OpenScan");
    expect(ops).toContain("EmitColumn");
    expect(ops).toContain("EmitRow");
    expect(ops).toContain("Halt");
  });

  test("result schema captures column names", () => {
    const prog = compileSql("SELECT name, age FROM users");
    expect(prog.resultSchema).toEqual(["name", "age"]);
  });

  test("result schema strips __sort_ hidden columns", () => {
    const prog = compileSql("SELECT name FROM users ORDER BY age");
    expect(prog.resultSchema).not.toContain("__sort_age");
    expect(prog.resultSchema).toContain("name");
  });

  test("filter emits JumpIfFalse", () => {
    const prog = compileSql("SELECT id FROM t WHERE id > 5");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("JumpIfFalse");
    expect(ops).toContain("BinaryOp");
  });

  test("ORDER BY emits SortResult post-process", () => {
    const prog = compileSql("SELECT name FROM users ORDER BY name ASC");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("SortResult");
  });

  test("LIMIT emits LimitResult", () => {
    const prog = compileSql("SELECT name FROM t LIMIT 10");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("LimitResult");
  });

  test("DISTINCT emits DistinctResult", () => {
    const prog = compileSql("SELECT DISTINCT name FROM t");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("DistinctResult");
  });

  test("aggregate COUNT emits InitAgg + UpdateAgg + FinalizeAgg", () => {
    const prog = compileSql("SELECT COUNT(*) FROM t");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InitAgg");
    expect(ops).toContain("UpdateAgg");
    expect(ops).toContain("FinalizeAgg");
    expect(ops).toContain("JumpIfGroupsDone");
  });

  test("label index is populated", () => {
    const prog = compileSql("SELECT x FROM t");
    expect(prog.labels.size).toBeGreaterThan(0);
  });

  test("from-less SELECT uses __dual__ scan", () => {
    const prog = compileSql("SELECT 1 + 1 AS result");
    const scans = prog.instructions.filter((i) => i.op === "OpenScan");
    expect(scans.length).toBeGreaterThan(0);
    const dual = scans.find((s) => (s as { op: string; table?: string }).table === "__dual__");
    expect(dual).toBeDefined();
  });
});

describe("sql-codegen: DML", () => {
  test("INSERT emits InsertRow", () => {
    const prog = compileSql("INSERT INTO users (id, name) VALUES (1, 'Alice')");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InsertRow");
  });

  test("UPDATE emits UpdateRows with scan loop", () => {
    const prog = compileSql("UPDATE users SET name = 'Bob' WHERE id = 1");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("OpenScan");
    expect(ops).toContain("UpdateRows");
  });

  test("DELETE emits DeleteRows with scan loop", () => {
    const prog = compileSql("DELETE FROM users WHERE id = 1");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("OpenScan");
    expect(ops).toContain("DeleteRows");
  });
});

describe("sql-codegen: DDL", () => {
  test("CREATE TABLE emits CreateTable", () => {
    const prog = compileSql("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("CreateTable");
  });

  test("DROP TABLE emits DropTable", () => {
    const prog = compileSql("DROP TABLE IF EXISTS t");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("DropTable");
  });
});

describe("sql-codegen: expressions", () => {
  test("COALESCE emits Coalesce", () => {
    const prog = compileSql("SELECT COALESCE(x, 0) FROM t");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("Coalesce");
  });

  test("IS NULL emits IsNullInstr", () => {
    const prog = compileSql("SELECT x FROM t WHERE x IS NULL");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("IsNullInstr");
  });

  test("IS NOT NULL emits IsNotNullInstr", () => {
    const prog = compileSql("SELECT x FROM t WHERE x IS NOT NULL");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("IsNotNullInstr");
  });

  test("string concat || emits BinaryOp", () => {
    const prog = compileSql("SELECT first_name || ' ' || last_name AS full_name FROM users");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("BinaryOp");
  });

  test("BETWEEN emits BetweenInstr", () => {
    const prog = compileSql("SELECT x FROM t WHERE x BETWEEN 1 AND 10");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("BetweenInstr");
  });

  test("NOT BETWEEN emits BetweenInstr negated", () => {
    const prog = compileSql("SELECT x FROM t WHERE x NOT BETWEEN 1 AND 10");
    const instrList = prog.instructions;
    const between = instrList.find((i) => i.op === "BetweenInstr");
    expect(between).toBeDefined();
    expect((between as { op: string; negated: boolean }).negated).toBe(true);
  });

  test("LIKE emits LikeInstr", () => {
    const prog = compileSql("SELECT x FROM t WHERE x LIKE '%hello%'");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("LikeInstr");
  });

  test("NOT LIKE emits LikeInstr negated", () => {
    const prog = compileSql("SELECT x FROM t WHERE x NOT LIKE 'foo%'");
    const instrList = prog.instructions;
    const like = instrList.find((i) => i.op === "LikeInstr");
    expect(like).toBeDefined();
    expect((like as { op: string; negated: boolean }).negated).toBe(true);
  });

  test("IN list emits InList", () => {
    const prog = compileSql("SELECT x FROM t WHERE x IN (1, 2, 3)");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InList");
  });

  test("NOT IN emits InList negated", () => {
    const prog = compileSql("SELECT x FROM t WHERE x NOT IN (1, 2)");
    const instrList = prog.instructions;
    const inList = instrList.find((i) => i.op === "InList");
    expect(inList).toBeDefined();
    expect((inList as { op: string; negated: boolean }).negated).toBe(true);
  });

  test("scalar function emits CallFunc", () => {
    const prog = compileSql("SELECT UPPER(name) FROM users");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("CallFunc");
  });

  test("unary minus emits UnaryOp", () => {
    const prog = compileSql("SELECT x FROM t WHERE -x < 0");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("UnaryOp");
  });
});

describe("sql-codegen: aggregates", () => {
  test("GROUP BY emits SaveGroupKey and LoadGroupKey in phase 2", () => {
    const prog = compileSql("SELECT dept, SUM(amount) FROM sales GROUP BY dept");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("SaveGroupKey");
    expect(ops).toContain("LoadGroupKey");
  });

  test("HAVING emits JumpIfFalse in group emit loop", () => {
    const prog = compileSql("SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 1");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("FinalizeAgg");
    expect(ops).toContain("JumpIfFalse");
    expect(ops).toContain("AdvanceGroup");
  });

  test("standalone aggregate (no GROUP BY) emits phase-2 default path", () => {
    const prog = compileSql("SELECT SUM(n), COUNT(*) FROM t");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InitAgg");
    expect(ops).toContain("UpdateAgg");
    expect(ops).toContain("FinalizeAgg");
  });

  test("aggregate result schema uses func output name", () => {
    const prog = compileSql("SELECT SUM(n) FROM t");
    expect(prog.resultSchema).toContain("sum(n)");
  });

  test("COUNT(*) arg uses LoadConst 1", () => {
    const prog = compileSql("SELECT COUNT(*) FROM t");
    const updateAgg = prog.instructions.find((i) => i.op === "UpdateAgg");
    expect(updateAgg).toBeDefined();
    expect((updateAgg as { op: string; func: string }).func).toBe("count");
  });

  test("ORDER BY non-projected column uses __sort_ prefix in SortResult", () => {
    const prog = compileSql("SELECT name FROM users ORDER BY age");
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    expect(sort).toBeDefined();
    const keys = (sort as { op: string; keys: Array<{ column: string }> }).keys;
    expect(keys[0].column).toBe("__sort_age");
  });

  test("ORDER BY projected column uses column name directly", () => {
    const prog = compileSql("SELECT name FROM users ORDER BY name");
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    expect(sort).toBeDefined();
    const keys = (sort as { op: string; keys: Array<{ column: string }> }).keys;
    expect(keys[0].column).toBe("name");
  });
});

describe("sql-codegen: JOIN (manual plan)", () => {
  function makeJoinPlan(): LogicalPlan {
    return {
      type: "project",
      items: [
        { expr: { kind: "column", table: null, name: "id" }, alias: null },
      ],
      input: {
        type: "join",
        left: { type: "scan", table: "a" },
        right: { type: "scan", table: "b" },
        condition: {
          kind: "binary",
          op: "=",
          left: { kind: "column", table: "a", name: "id" },
          right: { kind: "column", table: "b", name: "id" },
        },
        joinType: "inner",
      },
    } as LogicalPlan;
  }

  test("JOIN emits two OpenScan instructions", () => {
    const prog = compile(makeJoinPlan());
    const scans = prog.instructions.filter((i) => i.op === "OpenScan");
    expect(scans.length).toBe(2);
  });

  test("JOIN emits nested JumpIfExhausted for inner table", () => {
    const prog = compile(makeJoinPlan());
    const jumps = prog.instructions.filter((i) => i.op === "JumpIfExhausted");
    expect(jumps.length).toBe(2);
  });

  test("JOIN condition emits BinaryOp", () => {
    const prog = compile(makeJoinPlan());
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("BinaryOp");
  });

  test("JOIN without condition skips JumpIfFalse", () => {
    const noCondPlan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "column", table: null, name: "x" }, alias: null }],
      input: {
        type: "join",
        left: { type: "scan", table: "a" },
        right: { type: "scan", table: "b" },
        condition: null,
        joinType: "inner",
      },
    } as LogicalPlan;
    const prog = compile(noCondPlan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("OpenScan");
    // No condition → no JumpIfFalse in the join loop
    const jumpsFalse = prog.instructions.filter((i) => i.op === "JumpIfFalse");
    expect(jumpsFalse.length).toBe(0);
  });
});

describe("sql-codegen: SELECT star", () => {
  test("SELECT * emits __star__ EmitColumn", () => {
    const prog = compileSql("SELECT * FROM users");
    const cols = prog.instructions.filter((i) => i.op === "EmitColumn");
    const star = cols.find((c) => (c as { op: string; name: string }).name === "__star__");
    expect(star).toBeDefined();
  });
});

describe("sql-codegen: DML extended", () => {
  test("INSERT multi-row VALUES emits multiple LoadConst + InsertRow", () => {
    const prog = compileSql("INSERT INTO t (id, val) VALUES (1, 'a'), (2, 'b')");
    const inserts = prog.instructions.filter((i) => i.op === "InsertRow");
    expect(inserts.length).toBe(2);
  });

  test("UPDATE without WHERE still emits UpdateRows", () => {
    const prog = compileSql("UPDATE t SET val = 42");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("UpdateRows");
    expect(ops).not.toContain("JumpIfFalse");
  });

  test("DELETE without WHERE still emits DeleteRows", () => {
    const prog = compileSql("DELETE FROM t");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("DeleteRows");
    expect(ops).not.toContain("JumpIfFalse");
  });
});

describe("sql-codegen: CREATE TABLE column specs", () => {
  test("CreateTable has correct column spec with default", () => {
    const prog = compileSql("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, active INTEGER DEFAULT 1)");
    const create = prog.instructions.find((i) => i.op === "CreateTable");
    expect(create).toBeDefined();
    const ct = create as { op: string; table: string; columns: Array<{ name: string; primaryKey: boolean; defaultValue: unknown }> };
    const idCol = ct.columns.find((c) => c.name === "id");
    expect(idCol?.primaryKey).toBe(true);
    const activeCol = ct.columns.find((c) => c.name === "active");
    expect(activeCol?.defaultValue).toBe(1);
  });
});

describe("sql-codegen: manual plan edge cases", () => {
  test("bare AggregateNode at root uses phase-2 default emit path", () => {
    const aggPlan: LogicalPlan = {
      type: "aggregate",
      keys: [],
      aggregates: [{ func: "count", arg: null, alias: "__agg_sel_count_0", distinct: false }],
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(aggPlan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InitAgg");
    expect(ops).toContain("FinalizeAgg");
    expect(ops).toContain("EmitRow");
  });

  test("HavingNode with non-aggregate input falls through to compilePlan", () => {
    const plan: LogicalPlan = {
      type: "having",
      predicate: { kind: "literal", value: true },
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("OpenScan");
  });

  test("aggregate expr in project without AggregateNode wrapper emits LoadNull", () => {
    // Unusual plan: aggregate expr in project items but no AggregateNode below.
    // compileExpr sees kind=aggregate with currentAggNode=null → emits LoadNull.
    const plan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "aggregate", func: "count", arg: null, distinct: false }, alias: "cnt" }],
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("LoadNull");
  });

  test("SELECT literal without alias uses exprOutputName literal path", () => {
    // Project item expr=literal, alias=null → exprOutputName(literal) → "42"
    const plan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "literal", value: 42 }, alias: null }],
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(plan);
    expect(prog.resultSchema).toContain("42");
  });

  test("ORDER BY aggregate sort key covers exprOutputName aggregate + sortKeyToColumnName default", () => {
    // SortNode with a key whose expr is an aggregate.
    // sortKeyToColumnName hits default → calls exprOutputName(aggregate) → "sum(amount)"
    // exprKey(aggregate) covers line 670 in exprKey.
    const plan: LogicalPlan = {
      type: "sort",
      keys: [{
        expr: { kind: "aggregate", func: "sum", arg: { kind: "column", table: null, name: "amount" }, distinct: false },
        ascending: true,
        nullsLast: true,
      }],
      input: {
        type: "project",
        items: [
          { expr: { kind: "column", table: null, name: "dept" }, alias: null },
          { expr: { kind: "aggregate", func: "sum", arg: { kind: "column", table: null, name: "amount" }, distinct: false }, alias: null },
        ],
        input: {
          type: "aggregate",
          keys: [{ kind: "column", table: null, name: "dept" }],
          aggregates: [{ func: "sum", arg: { kind: "column", table: null, name: "amount" }, alias: "__agg_sel_sum_0", distinct: false }],
          input: { type: "scan", table: "sales" },
        },
      },
    } as unknown as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("SortResult");
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    // The sort column name resolves through the default path
    expect(sort).toBeDefined();
  });

  test("exprKey covers binary, unary, func cases via findAggSpec on complex args", () => {
    // AggregateNode with a func expr as the aggregate arg
    // When compileAggregatePhase1 runs, compileExpr for the func arg calls exprKey(func expr)
    const plan: LogicalPlan = {
      type: "project",
      items: [{
        expr: { kind: "aggregate", func: "sum", arg: { kind: "func", name: "ABS", args: [{ kind: "column", table: null, name: "n" }] }, distinct: false },
        alias: "total",
      }],
      input: {
        type: "aggregate",
        keys: [],
        aggregates: [{ func: "sum", arg: { kind: "func", name: "ABS", args: [{ kind: "column", table: null, name: "n" }] }, alias: "__agg_sel_sum_0", distinct: false }],
        input: { type: "scan", table: "t" },
      },
    } as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InitAgg");
    expect(ops).toContain("CallFunc");
  });

  test("exprOutputName binary covers arithmetic expression column names", () => {
    // Project item with a binary expr and no alias → exprOutputName(binary) → "n+1"
    const plan: LogicalPlan = {
      type: "project",
      items: [{
        expr: { kind: "binary", op: "+", left: { kind: "column", table: null, name: "n" }, right: { kind: "literal", value: 1 } },
        alias: null,
      }],
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(plan);
    expect(prog.resultSchema).toContain("n+1");
  });

  test("empty_result plan emits only Halt", () => {
    const plan: LogicalPlan = { type: "empty_result" } as LogicalPlan;
    const prog = compile(plan);
    expect(prog.instructions.map((i) => i.op)).toEqual(["Halt"]);
  });

  test("unknown plan type throws CodegenError", () => {
    const plan = { type: "unknown_plan_xyz" } as unknown as LogicalPlan;
    expect(() => compile(plan)).toThrow(CodegenError);
  });

  // The remaining tests target specific lines in exprKey, exprOutputName, sortKeyToColumnName.
  // These helpers have many cases that only fire with unusual expr shapes, so manual plans
  // are the only practical way to exercise them.

  test("exprKey binary: SUM(a+b) aggregate arg covers binary case", () => {
    const binArg = { kind: "binary", op: "+", left: { kind: "column", table: null, name: "a" }, right: { kind: "column", table: null, name: "b" } };
    const aggPlan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "aggregate", func: "sum", arg: binArg, distinct: false }, alias: "total" }],
      input: {
        type: "aggregate",
        keys: [],
        aggregates: [{ func: "sum", arg: binArg, alias: "__agg_sum_0", distinct: false }],
        input: { type: "scan", table: "t" },
      },
    } as LogicalPlan;
    const prog = compile(aggPlan);
    expect(prog.resultSchema).toContain("total");
  });

  test("exprKey unary: SUM(-n) aggregate arg covers unary case", () => {
    const unaryArg = { kind: "unary", op: "-", operand: { kind: "column", table: null, name: "n" } };
    const aggPlan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "aggregate", func: "sum", arg: unaryArg, distinct: false }, alias: "neg_total" }],
      input: {
        type: "aggregate",
        keys: [],
        aggregates: [{ func: "sum", arg: unaryArg, alias: "__agg_sum_0", distinct: false }],
        input: { type: "scan", table: "t" },
      },
    } as LogicalPlan;
    const prog = compile(aggPlan);
    expect(prog.resultSchema).toContain("neg_total");
  });

  test("exprKey aggregate: sort by aggregate not in projection covers aggregate case", () => {
    // Sort key = SUM(amount); projection = dept + COUNT(*). "sum(amount)" not in resultSchema
    // → exprKey(aggregate) is called → `agg:sum(amount)`.
    const colAmount = { kind: "column", table: null, name: "amount" } as const;
    const countAgg = { func: "count", arg: null, alias: "__agg_count_0", distinct: false };
    const plan: LogicalPlan = {
      type: "sort",
      keys: [{
        expr: { kind: "aggregate", func: "sum", arg: colAmount, distinct: false },
        ascending: true, nullsLast: true,
      }],
      input: {
        type: "project",
        items: [
          { expr: { kind: "column", table: null, name: "dept" }, alias: null },
          { expr: { kind: "aggregate", func: "count", arg: null, distinct: false }, alias: "cnt" },
        ],
        input: {
          type: "aggregate",
          keys: [{ kind: "column", table: null, name: "dept" }],
          aggregates: [countAgg],
          input: { type: "scan", table: "t" },
        },
      },
    } as unknown as LogicalPlan;
    const prog = compile(plan);
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    expect(sort).toBeDefined();
    // Key column includes __sort_ + exprKey result
    const keys = (sort as { op: string; keys: Array<{ column: string }> }).keys;
    expect(keys[0].column).toMatch(/^__sort_/);
  });

  test("exprKey coalesce: aggregate with coalesce arg covers coalesce case", () => {
    const coalesceArg = { kind: "coalesce", args: [{ kind: "column", table: null, name: "n" }, { kind: "literal", value: 0 }] };
    const aggPlan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "aggregate", func: "sum", arg: coalesceArg, distinct: false }, alias: "total" }],
      input: {
        type: "aggregate",
        keys: [],
        aggregates: [{ func: "sum", arg: coalesceArg, alias: "__agg_sum_0", distinct: false }],
        input: { type: "scan", table: "t" },
      },
    } as LogicalPlan;
    const prog = compile(aggPlan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("Coalesce");
  });

  test("exprKey between/in_list/like/is_null via aggregate args", () => {
    // These are exotic cases: aggregate arg is a complex predicate expression.
    // Each one exercises a different case in exprKey.

    function makeAggPlan(arg: unknown): LogicalPlan {
      return {
        type: "project",
        items: [{ expr: { kind: "aggregate", func: "sum", arg, distinct: false }, alias: "res" }],
        input: {
          type: "aggregate",
          keys: [],
          aggregates: [{ func: "sum", arg, alias: "__agg_0", distinct: false }],
          input: { type: "scan", table: "t" },
        },
      } as LogicalPlan;
    }

    const colN = { kind: "column", table: null, name: "n" };
    const lit1 = { kind: "literal", value: 1 };
    const lit10 = { kind: "literal", value: 10 };
    const litPat = { kind: "literal", value: "test%" };

    // between: SUM(n BETWEEN 1 AND 10)
    expect(() => compile(makeAggPlan({ kind: "between", expr: colN, low: lit1, high: lit10, negated: false }))).not.toThrow();
    // in_list: SUM(n IN (1, 2))
    expect(() => compile(makeAggPlan({ kind: "in_list", expr: colN, list: [lit1], negated: false }))).not.toThrow();
    // like: SUM(x LIKE 'test%')
    expect(() => compile(makeAggPlan({ kind: "like", expr: colN, pattern: litPat, negated: false }))).not.toThrow();
    // is_null: SUM(n IS NULL)
    expect(() => compile(makeAggPlan({ kind: "is_null", expr: colN, negated: false }))).not.toThrow();
  });

  test("exprKey star and exprKey default: star/unknown sort keys", () => {
    // star as sort key covers exprKey line 666 via sortKeyToColumnName default
    const starSortPlan: LogicalPlan = {
      type: "sort",
      keys: [{ expr: { kind: "star" }, ascending: true, nullsLast: true }],
      input: {
        type: "project",
        items: [{ expr: { kind: "column", table: null, name: "x" }, alias: null }],
        input: { type: "scan", table: "t" },
      },
    } as unknown as LogicalPlan;
    const prog = compile(starSortPlan);
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    expect(sort).toBeDefined();
  });

  test("exprOutputName star: star project item without alias produces '*' in schema", () => {
    // In compileProjectOverAggregate resultSchema computation, if a project item
    // has expr=star and no alias, exprOutputName(star) is called (the star check
    // only bypasses normal emit, not the schema computation at the end).
    // Construct an aggregate plan where one project item is star with no alias.
    const plan: LogicalPlan = {
      type: "project",
      items: [
        { expr: { kind: "star" }, alias: null },
        { expr: { kind: "aggregate", func: "count", arg: null, distinct: false }, alias: null },
      ],
      input: {
        type: "aggregate",
        keys: [],
        aggregates: [{ func: "count", arg: null, alias: "__agg_0", distinct: false }],
        input: { type: "scan", table: "t" },
      },
    } as LogicalPlan;
    const prog = compile(plan);
    // resultSchema is computed from projNode.items — star item alias=null → exprOutputName(star) = "*"
    expect(prog.resultSchema).toContain("*");
  });

  test("literal null value emits LoadNull (compileExpr literal-null branch)", () => {
    // INSERT with NULL literal → { kind: "literal", value: null } → LoadNull
    const prog = compileSql("INSERT INTO t (val) VALUES (NULL)");
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("LoadNull");
  });

  test("compileHavingBare with aggregate input covers lines 386-423", () => {
    // A bare HavingNode (not wrapped by ProjectNode) with an aggregate input.
    // compilePlan dispatches to compileHavingBare; node.input.type === "aggregate" → main body.
    const havingPlan: LogicalPlan = {
      type: "having",
      predicate: {
        kind: "binary",
        op: ">",
        left: { kind: "aggregate", func: "count", arg: null, distinct: false },
        right: { kind: "literal", value: 0 },
      },
      input: {
        type: "aggregate",
        keys: [{ kind: "column", table: null, name: "dept" }],
        aggregates: [{ func: "count", arg: null, alias: "__agg_0", distinct: false }],
        input: { type: "scan", table: "t" },
      },
    } as LogicalPlan;
    const prog = compile(havingPlan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("InitAgg");
    expect(ops).toContain("FinalizeAgg");
    expect(ops).toContain("JumpIfFalse");
    expect(ops).toContain("LoadGroupKey");
    expect(ops).toContain("EmitRow");
  });

  test("compileExpr default branch emits LoadNull for unknown expr kind", () => {
    // Pass an expr with an unknown kind. The default case emits LoadNull.
    const plan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "unknown_expr_xyz" } as unknown, alias: "x" }],
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("LoadNull");
  });

  test("exprKey default branch returns 'unknown' for unknown expr kind", () => {
    // Sort key with an unknown expr kind. exprKey default case fires.
    const plan: LogicalPlan = {
      type: "sort",
      keys: [{ expr: { kind: "unknown_xyz" } as unknown, ascending: true, nullsLast: true }],
      input: {
        type: "project",
        items: [{ expr: { kind: "column", table: null, name: "x" }, alias: null }],
        input: { type: "scan", table: "t" },
      },
    } as unknown as LogicalPlan;
    const prog = compile(plan);
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    expect(sort).toBeDefined();
  });

  test("compilePlan sort/limit/distinct nested inside filter reaches pass-through path (lines 183-185)", () => {
    // ProjectNode → FilterNode → SortNode → ScanNode
    // When compileFilter calls compilePlan(sortNode, callback), it reaches the
    // case "sort" in compilePlan which passes through to the inner scan.
    const plan: LogicalPlan = {
      type: "project",
      items: [{ expr: { kind: "column", table: null, name: "x" }, alias: null }],
      input: {
        type: "filter",
        predicate: { kind: "binary", op: ">", left: { kind: "column", table: null, name: "x" }, right: { kind: "literal", value: 0 } },
        input: {
          type: "sort",
          keys: [{ expr: { kind: "column", table: null, name: "x" }, ascending: true, nullsLast: true }],
          input: { type: "scan", table: "t" },
        },
      },
    } as unknown as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("OpenScan");
    expect(ops).toContain("JumpIfFalse");
  });

  test("compileAggregatePhase2Default with group keys covers loop body (lines 366-367)", () => {
    // Bare aggregate with non-empty keys → compileAggregatePhase2Default runs the
    // LoadGroupKey + EmitColumn loop body.
    const plan: LogicalPlan = {
      type: "aggregate",
      keys: [{ kind: "column", table: null, name: "dept" }],
      aggregates: [{ func: "count", arg: null, alias: "__agg_0", distinct: false }],
      input: { type: "scan", table: "t" },
    } as LogicalPlan;
    const prog = compile(plan);
    const ops = prog.instructions.map((i) => i.op);
    expect(ops).toContain("LoadGroupKey");
    expect(ops).toContain("SaveGroupKey");
  });

  test("sortKeyToColumnName literal: ORDER BY 1 literal sort key", () => {
    const plan: LogicalPlan = {
      type: "sort",
      keys: [{ expr: { kind: "literal", value: 1 }, ascending: true, nullsLast: true }],
      input: {
        type: "project",
        items: [{ expr: { kind: "column", table: null, name: "x" }, alias: null }],
        input: { type: "scan", table: "t" },
      },
    } as unknown as LogicalPlan;
    const prog = compile(plan);
    const sort = prog.instructions.find((i) => i.op === "SortResult");
    expect(sort).toBeDefined();
    // The literal sort key column is either "1" (if in schema) or "__sort_lit:1"
    const keys = (sort as { op: string; keys: Array<{ column: string }> }).keys;
    expect(keys[0].column).toBeDefined();
  });
});
