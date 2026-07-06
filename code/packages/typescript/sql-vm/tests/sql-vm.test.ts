import { describe, expect, test, beforeEach } from "vitest";
import { parseSQL } from "coding-adventures-sql-parser";
import { plan } from "@coding-adventures/sql-planner";
import { optimize } from "@coding-adventures/sql-optimizer";
import { compile } from "@coding-adventures/sql-codegen";
import type { Program } from "@coding-adventures/sql-codegen";
import { execute, VmError } from "../src/index.js";
import type { Database } from "../src/index.js";

function runSql(sql: string, db: Database) {
  const ast = parseSQL(sql);
  const logical = plan(ast);
  const optimized = optimize(logical);
  const program = compile(optimized);
  return execute(program, db);
}

function freshDb(): Database {
  const db: Database = new Map();
  return db;
}

// ---------------------------------------------------------------------------
// DDL
// ---------------------------------------------------------------------------
describe("sql-vm: DDL", () => {
  test("CREATE TABLE creates a table", () => {
    const db = freshDb();
    runSql("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", db);
    expect(db.has("users")).toBe(true);
    expect(db.get("users")!.columns).toContain("id");
  });

  test("CREATE TABLE IF NOT EXISTS is idempotent", () => {
    const db = freshDb();
    runSql("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", db);
    runSql("CREATE TABLE IF NOT EXISTS users (x INTEGER)", db);
    expect(db.get("users")!.columns).toContain("id");
    expect(db.get("users")!.columns).not.toContain("x");
  });

  test("DROP TABLE removes table", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("DROP TABLE t", db);
    expect(db.has("t")).toBe(false);
  });

  test("DROP TABLE IF EXISTS on missing table does not throw", () => {
    const db = freshDb();
    expect(() => runSql("DROP TABLE IF EXISTS t", db)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// DML — INSERT
// ---------------------------------------------------------------------------
describe("sql-vm: INSERT", () => {
  test("INSERT adds rows", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (id INTEGER, val TEXT)", db);
    runSql("INSERT INTO t (id, val) VALUES (1, 'hello')", db);
    runSql("INSERT INTO t (id, val) VALUES (2, 'world')", db);
    expect(db.get("t")!.rows.length).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// DML — UPDATE
// ---------------------------------------------------------------------------
describe("sql-vm: UPDATE", () => {
  test("UPDATE modifies matching rows", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (id INTEGER, val TEXT)", db);
    runSql("INSERT INTO t (id, val) VALUES (1, 'a')", db);
    runSql("INSERT INTO t (id, val) VALUES (2, 'b')", db);
    runSql("UPDATE t SET val = 'updated' WHERE id = 1", db);
    const rows = db.get("t")!.rows;
    expect(rows.find((r) => r["id"] === 1)!["val"]).toBe("updated");
    expect(rows.find((r) => r["id"] === 2)!["val"]).toBe("b");
  });
});

// ---------------------------------------------------------------------------
// DML — DELETE
// ---------------------------------------------------------------------------
describe("sql-vm: DELETE", () => {
  test("DELETE removes matching rows", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (id INTEGER, val TEXT)", db);
    runSql("INSERT INTO t (id, val) VALUES (1, 'a')", db);
    runSql("INSERT INTO t (id, val) VALUES (2, 'b')", db);
    runSql("DELETE FROM t WHERE id = 1", db);
    expect(db.get("t")!.rows.length).toBe(1);
    expect(db.get("t")!.rows[0]["id"]).toBe(2);
  });

  test("DELETE without WHERE removes all rows", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (id INTEGER)", db);
    runSql("INSERT INTO t (id) VALUES (1)", db);
    runSql("INSERT INTO t (id) VALUES (2)", db);
    runSql("DELETE FROM t", db);
    expect(db.get("t")!.rows.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// SELECT
// ---------------------------------------------------------------------------
describe("sql-vm: SELECT basic", () => {
  let db: Database;

  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)", db);
    runSql("INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30)", db);
    runSql("INSERT INTO users (id, name, age) VALUES (2, 'Bob', 25)", db);
    runSql("INSERT INTO users (id, name, age) VALUES (3, 'Charlie', 35)", db);
  });

  test("SELECT * returns all columns and rows", () => {
    const r = runSql("SELECT * FROM users", db);
    expect(r.rows.length).toBe(3);
  });

  test("SELECT column returns correct values", () => {
    const r = runSql("SELECT name FROM users WHERE id = 1", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0]).toContain("Alice");
  });

  test("ORDER BY ASC sorts correctly", () => {
    const r = runSql("SELECT name FROM users ORDER BY age ASC", db);
    expect(r.rows[0]).toContain("Bob");
    expect(r.rows[2]).toContain("Charlie");
  });

  test("ORDER BY DESC sorts correctly", () => {
    const r = runSql("SELECT name FROM users ORDER BY age DESC", db);
    expect(r.rows[0]).toContain("Charlie");
  });

  test("LIMIT restricts results", () => {
    const r = runSql("SELECT name FROM users LIMIT 2", db);
    expect(r.rows.length).toBe(2);
  });

  test("DISTINCT eliminates duplicates", () => {
    runSql("INSERT INTO users (id, name, age) VALUES (4, 'Alice', 40)", db);
    const r = runSql("SELECT DISTINCT name FROM users", db);
    const names = r.rows.map((row) => row[0]);
    expect(names.filter((n) => n === "Alice").length).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Aggregates
// ---------------------------------------------------------------------------
describe("sql-vm: aggregates", () => {
  let db: Database;

  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE sales (dept TEXT, amount INTEGER)", db);
    runSql("INSERT INTO sales (dept, amount) VALUES ('eng', 100)", db);
    runSql("INSERT INTO sales (dept, amount) VALUES ('eng', 200)", db);
    runSql("INSERT INTO sales (dept, amount) VALUES ('hr', 50)", db);
  });

  test("COUNT(*) returns row count", () => {
    const r = runSql("SELECT COUNT(*) FROM sales", db);
    expect(r.rows[0][0]).toBe(3);
  });

  test("SUM returns total", () => {
    const r = runSql("SELECT SUM(amount) FROM sales", db);
    expect(r.rows[0][0]).toBe(350);
  });

  test("GROUP BY partitions results", () => {
    const r = runSql("SELECT dept, SUM(amount) FROM sales GROUP BY dept ORDER BY dept ASC", db);
    expect(r.rows.length).toBe(2);
    const eng = r.rows.find((row) => row[0] === "eng");
    expect(eng?.[1]).toBe(300);
  });
});

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------
describe("sql-vm: expressions", () => {
  let db: Database;

  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE t (a TEXT, b TEXT, n INTEGER)", db);
    runSql("INSERT INTO t (a, b, n) VALUES ('hello', 'world', 5)", db);
    runSql("INSERT INTO t (a, b, n) VALUES (NULL, 'null-test', NULL)", db);
  });

  test("string concat ||", () => {
    const r = runSql("SELECT a || ' ' || b FROM t WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("hello world");
  });

  test("COALESCE returns first non-null", () => {
    const r = runSql("SELECT COALESCE(a, b) FROM t WHERE a IS NULL", db);
    expect(r.rows[0][0]).toBe("null-test");
  });

  test("IS NULL filter", () => {
    const r = runSql("SELECT b FROM t WHERE a IS NULL", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe("null-test");
  });

  test("IS NOT NULL filter", () => {
    const r = runSql("SELECT a FROM t WHERE a IS NOT NULL", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe("hello");
  });

  test("arithmetic expressions", () => {
    const r = runSql("SELECT n * 2 FROM t WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(10);
  });
});

// ---------------------------------------------------------------------------
// FROM-less SELECT
// ---------------------------------------------------------------------------
describe("sql-vm: FROM-less SELECT", () => {
  test("SELECT literal expression", () => {
    const db = freshDb();
    const r = runSql("SELECT 1 + 1 AS result", db);
    expect(r.rows[0][0]).toBe(2);
  });

  test("SELECT string concat", () => {
    const db = freshDb();
    const r = runSql("SELECT 'hello' || ' ' || 'world' AS greeting", db);
    expect(r.rows[0][0]).toBe("hello world");
  });
});

// ---------------------------------------------------------------------------
// NULL in ORDER BY (fixture 23)
// ---------------------------------------------------------------------------
describe("sql-vm: NULL ordering", () => {
  test("NULLs sort last by default in ASC order", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (val INTEGER)", db);
    runSql("INSERT INTO t (val) VALUES (3)", db);
    runSql("INSERT INTO t (val) VALUES (NULL)", db);
    runSql("INSERT INTO t (val) VALUES (1)", db);
    // Planner sets nullsLast=true for ASC (matches PostgreSQL default).
    const r = runSql("SELECT val FROM t ORDER BY val ASC", db);
    expect(r.rows[0][0]).toBe(1);
    expect(r.rows[1][0]).toBe(3);
    expect(r.rows[2][0]).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// HAVING (fixture 24)
// ---------------------------------------------------------------------------
describe("sql-vm: HAVING", () => {
  test("HAVING filters aggregate groups", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (dept TEXT, n INTEGER)", db);
    runSql("INSERT INTO t (dept, n) VALUES ('a', 1)", db);
    runSql("INSERT INTO t (dept, n) VALUES ('a', 2)", db);
    runSql("INSERT INTO t (dept, n) VALUES ('b', 1)", db);
    const r = runSql("SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 1", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe("a");
  });
});

// ---------------------------------------------------------------------------
// JOIN
// ---------------------------------------------------------------------------
describe("sql-vm: JOIN (manual plan)", () => {
  test("INNER JOIN pairs matching rows", () => {
    const db = freshDb();
    runSql("CREATE TABLE users (id INTEGER, name TEXT)", db);
    runSql("CREATE TABLE orders (user_id INTEGER, product TEXT)", db);
    runSql("INSERT INTO users (id, name) VALUES (1, 'Alice')", db);
    runSql("INSERT INTO users (id, name) VALUES (2, 'Bob')", db);
    runSql("INSERT INTO orders (user_id, product) VALUES (1, 'book')", db);
    runSql("INSERT INTO orders (user_id, product) VALUES (1, 'pen')", db);
    runSql("INSERT INTO orders (user_id, product) VALUES (2, 'cup')", db);

    // Grammar doesn't support JOIN syntax; construct the plan manually.
    const joinPlan = {
      type: "project" as const,
      items: [
        { expr: { kind: "column", table: "users", name: "name" }, alias: null },
        { expr: { kind: "column", table: "orders", name: "product" }, alias: null },
      ],
      input: {
        type: "join" as const,
        left: { type: "scan" as const, table: "users" },
        right: { type: "scan" as const, table: "orders" },
        condition: {
          kind: "binary", op: "=",
          left: { kind: "column", table: "users", name: "id" },
          right: { kind: "column", table: "orders", name: "user_id" },
        },
        joinType: "inner" as const,
      },
    };
    const program = compile(joinPlan as Parameters<typeof compile>[0]);
    const result = execute(program, db);
    expect(result.rows.length).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// Arithmetic and comparison operators
// ---------------------------------------------------------------------------
describe("sql-vm: arithmetic operators", () => {
  let db: Database;
  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE n (a INTEGER, b INTEGER)", db);
    runSql("INSERT INTO n (a, b) VALUES (10, 3)", db);
    runSql("INSERT INTO n (a, b) VALUES (NULL, 5)", db);
  });

  test("subtraction", () => {
    const r = runSql("SELECT a - b FROM n WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(7);
  });

  test("multiplication", () => {
    const r = runSql("SELECT a * b FROM n WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(30);
  });

  test("division", () => {
    const r = runSql("SELECT a / b FROM n WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBeCloseTo(3.333, 2);
  });

  test("division by zero returns null", () => {
    const r = runSql("SELECT a / 0 FROM n WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("modulo", () => {
    const r = runSql("SELECT a % b FROM n WHERE a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(1);
  });

  test("NULL propagation: NULL + 1 = NULL", () => {
    const r = runSql("SELECT a + 1 FROM n WHERE a IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("AND: false AND null = false", () => {
    const r = runSql("SELECT b FROM n WHERE 1=0 AND a IS NULL", db);
    expect(r.rows.length).toBe(0);
  });

  test("AND: null AND null = null (falsy → skip)", () => {
    const r = runSql("SELECT a FROM n WHERE a IS NULL AND b = 99", db);
    expect(r.rows.length).toBe(0);
  });

  test("OR: true OR null = true", () => {
    const r = runSql("SELECT b FROM n WHERE 1=1 OR a IS NULL", db);
    expect(r.rows.length).toBe(2);
  });

  test("comparison <, >, <=, >=", () => {
    const r = runSql("SELECT a FROM n WHERE a >= 10 AND a <= 10", db);
    expect(r.rows[0][0]).toBe(10);
  });

  test("comparison < and >", () => {
    const r = runSql("SELECT a FROM n WHERE a < 11 AND a > 9 AND a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(10);
  });

  test("addition with non-numeric types returns null", () => {
    const r = runSql("SELECT a + 'hello' FROM n WHERE a IS NOT NULL", db);
    // number + string → null (non-numeric operand)
    expect(r.rows[0][0]).toBeNull();
  });

  test("NOT equal <>", () => {
    const r = runSql("SELECT a FROM n WHERE a <> 99 AND a IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(10);
  });
});

// ---------------------------------------------------------------------------
// BETWEEN / LIKE / IN
// ---------------------------------------------------------------------------
describe("sql-vm: BETWEEN, LIKE, IN", () => {
  let db: Database;
  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE t (x INTEGER, s TEXT)", db);
    runSql("INSERT INTO t (x, s) VALUES (5, 'hello')", db);
    runSql("INSERT INTO t (x, s) VALUES (15, 'world')", db);
    runSql("INSERT INTO t (x, s) VALUES (NULL, 'foo')", db);
  });

  test("BETWEEN matches range", () => {
    const r = runSql("SELECT x FROM t WHERE x BETWEEN 1 AND 10", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe(5);
  });

  test("NOT BETWEEN excludes range", () => {
    const r = runSql("SELECT x FROM t WHERE x NOT BETWEEN 1 AND 10 AND x IS NOT NULL", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe(15);
  });

  test("BETWEEN with NULL value returns null (no match)", () => {
    const r = runSql("SELECT x FROM t WHERE x BETWEEN 1 AND 100", db);
    expect(r.rows.length).toBe(2); // NULL row is excluded
  });

  test("LIKE with % wildcard", () => {
    const r = runSql("SELECT s FROM t WHERE s LIKE 'h%'", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe("hello");
  });

  test("LIKE with _ wildcard", () => {
    const r = runSql("SELECT s FROM t WHERE s LIKE 'f__'", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe("foo");
  });

  test("NOT LIKE", () => {
    const r = runSql("SELECT s FROM t WHERE s NOT LIKE 'h%'", db);
    expect(r.rows.length).toBe(2); // world, foo
  });

  test("LIKE with NULL value returns null (row excluded)", () => {
    // Insert a row where s is NULL, then LIKE it — LikeInstr pushes null → not truthy → excluded.
    const db2 = freshDb();
    runSql("CREATE TABLE t2 (s TEXT)", db2);
    runSql("INSERT INTO t2 (s) VALUES (NULL)", db2);
    const r = runSql("SELECT s FROM t2 WHERE s LIKE '%x%'", db2);
    expect(r.rows.length).toBe(0);
  });

  test("IN list matches", () => {
    const r = runSql("SELECT x FROM t WHERE x IN (5, 15)", db);
    expect(r.rows.length).toBe(2);
  });

  test("NOT IN excludes", () => {
    const r = runSql("SELECT x FROM t WHERE x NOT IN (5) AND x IS NOT NULL", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe(15);
  });
});

// ---------------------------------------------------------------------------
// Built-in functions
// ---------------------------------------------------------------------------
describe("sql-vm: built-in functions", () => {
  let db: Database;
  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE t (s TEXT, n INTEGER)", db);
    runSql("INSERT INTO t (s, n) VALUES ('  Hello World  ', 42)", db);
    runSql("INSERT INTO t (s, n) VALUES (NULL, NULL)", db);
  });

  test("UPPER", () => {
    const r = runSql("SELECT UPPER(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("  HELLO WORLD  ");
  });

  test("LOWER", () => {
    const r = runSql("SELECT LOWER(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("  hello world  ");
  });

  test("LENGTH", () => {
    const r = runSql("SELECT LENGTH(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe(15);
  });

  test("TRIM", () => {
    const r = runSql("SELECT TRIM(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("Hello World");
  });

  test("LTRIM", () => {
    const r = runSql("SELECT LTRIM(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("Hello World  ");
  });

  test("RTRIM", () => {
    const r = runSql("SELECT RTRIM(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("  Hello World");
  });

  test("REPLACE", () => {
    const r = runSql("SELECT REPLACE(s, 'World', 'SQL') FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toContain("SQL");
  });

  test("SUBSTR", () => {
    const r = runSql("SELECT SUBSTR(s, 3, 5) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("Hello");
  });

  test("ABS", () => {
    const r = runSql("SELECT ABS(-42) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBe(42);
  });

  test("ROUND", () => {
    const r = runSql("SELECT ROUND(3.14159, 2) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBeCloseTo(3.14, 2);
  });

  test("NULLIF returns null when equal", () => {
    const r = runSql("SELECT NULLIF(n, 42) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("NULLIF returns first arg when not equal", () => {
    const r = runSql("SELECT NULLIF(n, 99) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBe(42);
  });

  test("IFNULL returns second arg when first is null", () => {
    const r = runSql("SELECT IFNULL(s, 'fallback') FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBe("fallback");
  });

  test("IFNULL returns first arg when not null", () => {
    const r = runSql("SELECT IFNULL(n, 0) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBe(42);
  });

  test("TYPEOF integer", () => {
    const r = runSql("SELECT TYPEOF(n) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBe("integer");
  });

  test("TYPEOF text", () => {
    const r = runSql("SELECT TYPEOF(s) FROM t WHERE s IS NOT NULL", db);
    expect(r.rows[0][0]).toBe("text");
  });

  test("TYPEOF null", () => {
    const r = runSql("SELECT TYPEOF(n) FROM t WHERE n IS NULL", db);
    expect(r.rows[0][0]).toBe("null");
  });

  test("function null passthrough: UPPER(NULL) = NULL", () => {
    const r = runSql("SELECT UPPER(s) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("LOWER(NULL) = NULL", () => {
    const r = runSql("SELECT LOWER(s) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("LENGTH(NULL) = NULL", () => {
    const r = runSql("SELECT LENGTH(s) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("TRIM(NULL) = NULL", () => {
    const r = runSql("SELECT TRIM(s) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("LTRIM(NULL) = NULL", () => {
    const r = runSql("SELECT LTRIM(s) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("RTRIM(NULL) = NULL", () => {
    const r = runSql("SELECT RTRIM(s) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("ABS(NULL) = NULL", () => {
    const r = runSql("SELECT ABS(n) FROM t WHERE n IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("ROUND(NULL) = NULL", () => {
    const r = runSql("SELECT ROUND(n) FROM t WHERE n IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("SUBSTR(NULL, 1) = NULL", () => {
    const r = runSql("SELECT SUBSTR(s, 1) FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("REPLACE(NULL, ...) = NULL", () => {
    const r = runSql("SELECT REPLACE(s, 'x', 'y') FROM t WHERE s IS NULL", db);
    expect(r.rows[0][0]).toBeNull();
  });

  test("TYPEOF(boolean) returns 'integer'", () => {
    const r = runSql("SELECT TYPEOF(1=1) FROM t WHERE n = 42", db);
    expect(r.rows[0][0]).toBe("integer");
  });

  test("unknown function throws VmError", () => {
    const db2 = freshDb();
    runSql("CREATE TABLE t2 (x INTEGER)", db2);
    runSql("INSERT INTO t2 (x) VALUES (1)", db2);
    // Construct a program with an unknown CallFunc to bypass parser restrictions.
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t2" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "CallFunc", name: "no_such_func", arity: 0 },
        { op: "EmitColumn", name: "result" },
        { op: "EmitRow" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Label", name: "loop" },
        { op: "Halt" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["done", 9], ["loop", 7]]),
      resultSchema: ["result"],
    };
    expect(() => execute(prog, db2)).toThrow(VmError);
  });
});

// ---------------------------------------------------------------------------
// Aggregate functions — MIN, MAX, AVG, GROUP_CONCAT
// ---------------------------------------------------------------------------
describe("sql-vm: aggregate MIN / MAX / AVG / GROUP_CONCAT", () => {
  let db: Database;
  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE t (val INTEGER, grp TEXT)", db);
    runSql("INSERT INTO t (val, grp) VALUES (3, 'a')", db);
    runSql("INSERT INTO t (val, grp) VALUES (1, 'a')", db);
    runSql("INSERT INTO t (val, grp) VALUES (5, 'b')", db);
    runSql("INSERT INTO t (val, grp) VALUES (NULL, 'a')", db);
  });

  test("MIN ignores NULL", () => {
    const r = runSql("SELECT MIN(val) FROM t", db);
    expect(r.rows[0][0]).toBe(1);
  });

  test("MAX ignores NULL", () => {
    const r = runSql("SELECT MAX(val) FROM t", db);
    expect(r.rows[0][0]).toBe(5);
  });

  test("AVG ignores NULL", () => {
    const r = runSql("SELECT AVG(val) FROM t", db);
    expect(r.rows[0][0]).toBeCloseTo(3, 1); // (3+1+5)/3
  });

  test("SUM aggregates values", () => {
    const r = runSql("SELECT SUM(val) FROM t", db);
    expect(r.rows[0][0]).toBe(9); // 3+1+5 (NULL excluded)
  });

  test("COUNT(*) counts all rows including nulls", () => {
    const r = runSql("SELECT COUNT(*) FROM t", db);
    expect(r.rows[0][0]).toBe(4);
  });

});

// ---------------------------------------------------------------------------
// IIF (covers isTruthy with number and string)
// ---------------------------------------------------------------------------
describe("sql-vm: IIF function", () => {
  let db: Database;
  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
  });

  test("IIF with true-number condition returns then-branch", () => {
    const r = runSql("SELECT IIF(1, 'yes', 'no') FROM t", db);
    expect(r.rows[0][0]).toBe("yes");
  });

  test("IIF with zero condition returns else-branch", () => {
    const r = runSql("SELECT IIF(0, 'yes', 'no') FROM t", db);
    expect(r.rows[0][0]).toBe("no");
  });

  test("IIF with non-empty string condition returns then-branch", () => {
    const r = runSql("SELECT IIF('hello', 'yes', 'no') FROM t", db);
    expect(r.rows[0][0]).toBe("yes");
  });

  test("IIF with null condition returns else-branch", () => {
    const r = runSql("SELECT IIF(NULL, 'yes', 'no') FROM t", db);
    expect(r.rows[0][0]).toBe("no");
  });
});

// ---------------------------------------------------------------------------
// SELECT DISTINCT
// ---------------------------------------------------------------------------
describe("sql-vm: SELECT DISTINCT", () => {
  test("DISTINCT removes duplicate rows", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (col TEXT)", db);
    runSql("INSERT INTO t (col) VALUES ('a')", db);
    runSql("INSERT INTO t (col) VALUES ('b')", db);
    runSql("INSERT INTO t (col) VALUES ('a')", db);
    const r = runSql("SELECT DISTINCT col FROM t", db);
    expect(r.rows.length).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Additional built-in function branches
// ---------------------------------------------------------------------------
describe("sql-vm: additional built-in function branches", () => {
  let db: Database;
  beforeEach(() => {
    db = freshDb();
    runSql("CREATE TABLE t (s TEXT, n REAL)", db);
    runSql("INSERT INTO t (s, n) VALUES ('hello', 3.14)", db);
  });

  test("TYPEOF real returns 'real'", () => {
    const r = runSql("SELECT TYPEOF(n) FROM t", db);
    expect(r.rows[0][0]).toBe("real");
  });

  test("ROUND with no places arg rounds to integer", () => {
    const r = runSql("SELECT ROUND(3.7) FROM t", db);
    expect(r.rows[0][0]).toBe(4);
  });

  test("SUBSTR with no length returns rest of string", () => {
    const r = runSql("SELECT SUBSTR(s, 3) FROM t", db);
    expect(r.rows[0][0]).toBe("llo");
  });

  test("string concatenation ||", () => {
    const r = runSql("SELECT 'foo' || 'bar' FROM t", db);
    expect(r.rows[0][0]).toBe("foobar");
  });

  test("unary minus negates value", () => {
    const r = runSql("SELECT -n FROM t", db);
    expect(r.rows[0][0]).toBeCloseTo(-3.14, 2);
  });

  test("unary NOT negates boolean", () => {
    const r = runSql("SELECT s FROM t WHERE NOT (n > 100)", db);
    expect(r.rows[0][0]).toBe("hello");
  });

  test("COALESCE returns first non-null", () => {
    const r = runSql("SELECT COALESCE(NULL, n, 999) FROM t", db);
    expect(r.rows[0][0]).toBeCloseTo(3.14, 2);
  });
});

// ---------------------------------------------------------------------------
// Null sort ordering (covers sortRows null branches)
// ---------------------------------------------------------------------------
describe("sql-vm: null sort ordering", () => {
  test("NULLs sort first in DESC order (nullsLast=false)", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (val INTEGER)", db);
    runSql("INSERT INTO t (val) VALUES (3)", db);
    runSql("INSERT INTO t (val) VALUES (NULL)", db);
    runSql("INSERT INTO t (val) VALUES (1)", db);
    const r = runSql("SELECT val FROM t ORDER BY val DESC", db);
    // DESC → nullsLast=false → nulls sort first
    expect(r.rows[0][0]).toBeNull();
    expect(r.rows[1][0]).toBe(3);
    expect(r.rows[2][0]).toBe(1);
  });

  test("string sort in ORDER BY", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (s TEXT)", db);
    runSql("INSERT INTO t (s) VALUES ('banana')", db);
    runSql("INSERT INTO t (s) VALUES ('apple')", db);
    runSql("INSERT INTO t (s) VALUES ('cherry')", db);
    const r = runSql("SELECT s FROM t ORDER BY s ASC", db);
    expect(r.rows[0][0]).toBe("apple");
    expect(r.rows[1][0]).toBe("banana");
    expect(r.rows[2][0]).toBe("cherry");
  });
});

// ---------------------------------------------------------------------------
// Additional error/edge-case instructions via manual programs
// ---------------------------------------------------------------------------
describe("sql-vm: Coalesce instruction via manual program", () => {
  test("Coalesce instruction returns first non-null", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    // Build a program that uses the Coalesce instruction with 3 args
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadNull" },
        { op: "LoadNull" },
        { op: "LoadConst", value: 42 },
        { op: "Coalesce", arity: 3 },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "loop" },
        { op: "Halt" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["done", 13], ["loop", 11]]),
      resultSchema: ["v"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBe(42);
  });
});

// ---------------------------------------------------------------------------
// AND/OR null-returning branches
// ---------------------------------------------------------------------------
describe("sql-vm: AND/OR null-returns", () => {
  test("true AND null = null (row excluded)", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (a INTEGER)", db);
    runSql("INSERT INTO t (a) VALUES (NULL)", db);
    // WHERE 1=1 AND a = 1  → true AND null → null → row excluded
    const r = runSql("SELECT a FROM t WHERE 1=1 AND a = 1", db);
    expect(r.rows.length).toBe(0);
  });

  test("false OR null = null (row excluded)", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (a INTEGER)", db);
    runSql("INSERT INTO t (a) VALUES (NULL)", db);
    // WHERE 1=0 OR a = 1  → false OR null → null → row excluded
    const r = runSql("SELECT a FROM t WHERE 1=0 OR a = 1", db);
    expect(r.rows.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Manual program edge cases for evalBinaryOp / evalUnaryOp / callBuiltinFunc
// ---------------------------------------------------------------------------
describe("sql-vm: manual program edge cases", () => {
  test("unary + is identity", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (7)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "UnaryOp", operator: "+" },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 1], ["done", 10]]),
      resultSchema: ["v"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBe(7);
  });

  test("lowercase 'and' binary op works", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadConst", value: true },
        { op: "LoadConst", value: true },
        { op: "BinaryOp", operator: "and" },
        { op: "JumpIfFalse", label: "skip" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "Label", name: "skip" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 1], ["skip", 11], ["done", 14]]),
      resultSchema: ["v"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBe(1);
  });

  test("lowercase 'or' binary op works", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadConst", value: false },
        { op: "LoadConst", value: true },
        { op: "BinaryOp", operator: "or" },
        { op: "JumpIfFalse", label: "skip" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "Label", name: "skip" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 1], ["skip", 11], ["done", 14]]),
      resultSchema: ["v"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBe(1);
  });

  test("unknown binary op throws VmError", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "BeginRow" },
        { op: "LoadConst", value: 1 },
        { op: "LoadConst", value: 1 },
        { op: "BinaryOp", operator: "$$bad$$" },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "Halt" },
      ],
      labels: new Map(),
      resultSchema: ["v"],
    };
    expect(() => execute(prog, db)).toThrow(VmError);
  });

  test("unknown unary op throws VmError", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "BeginRow" },
        { op: "LoadConst", value: 1 },
        { op: "UnaryOp", operator: "$$bad$$" },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "Halt" },
      ],
      labels: new Map(),
      resultSchema: ["v"],
    };
    expect(() => execute(prog, db)).toThrow(VmError);
  });

  test("coalesce via CallFunc returns first non-null", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadNull" },
        { op: "LoadConst", value: 99 },
        { op: "CallFunc", name: "coalesce", arity: 2 },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 1], ["done", 12]]),
      resultSchema: ["v"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBe(99);
  });

  test("SetResultSchema overrides the resultColumns", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (5)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "EmitColumn", name: "x" },
        { op: "EmitRow" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "SetResultSchema", columns: ["renamed_x"] },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 1], ["done", 9]]),
      resultSchema: ["x"],
    };
    const result = execute(prog, db);
    expect(result.columns).toEqual(["renamed_x"]);
  });

  test("FinalizeAgg with out-of-range slot pushes null", () => {
    // Covers line 371: `this.push(null)` when slot is undefined in the group.
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "InitAgg", slots: 1 },
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "UpdateAgg", slot: 0, func: "SUM" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "BeginRow" },
        // slot 99 is out of range → group.slots[99] is undefined → pushes null (line 371)
        { op: "FinalizeAgg", slot: 99, func: "SUM", alias: "out_of_range" },
        { op: "EmitColumn", name: "v" },
        { op: "EmitRow" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 2], ["done", 8]]),
      resultSchema: ["v"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBeNull();
  });

  test("SortResult with unknown key column is a no-op continue", () => {
    // Covers the `if (idx === undefined) continue;` branch in sortRows.
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (2)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog: Program = {
      instructions: [
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "BeginRow" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "EmitColumn", name: "x" },
        { op: "EmitRow" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "SortResult", keys: [{ column: "nonexistent_col", ascending: true, nullsLast: true }], stripPrefix: "" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 1], ["done", 9]]),
      resultSchema: ["x"],
    };
    const result = execute(prog, db);
    // Sort key is nonexistent so order is unchanged — both rows present
    expect(result.rows.length).toBe(2);
  });

  test("JumpIfTrue jumps when truthy, falls through when falsy", () => {
    const db = freshDb();
    // Test the truthy path (jump taken)
    const prog1: Program = {
      instructions: [
        { op: "LoadConst", value: 1 },
        { op: "JumpIfTrue", label: "done" },
        { op: "Halt" },
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["done", 3]]),
      resultSchema: [],
    };
    expect(() => execute(prog1, db)).not.toThrow();

    // Test the falsy path (no jump, ip++ executed at line 535)
    const prog2: Program = {
      instructions: [
        { op: "LoadConst", value: false },
        { op: "JumpIfTrue", label: "done" },
        { op: "Halt" },  // should reach here
        { op: "Label", name: "done" },
        { op: "Halt" },
      ],
      labels: new Map([["done", 3]]),
      resultSchema: [],
    };
    expect(() => execute(prog2, db)).not.toThrow();
  });

  test("FinalizeAgg with no accumulated groups produces null", () => {
    // Covers the FinalizeAgg fallback (line 374) when no group was accumulated.
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    // No rows inserted — scan exhausted immediately, no UpdateAgg runs.
    const prog: Program = {
      instructions: [
        { op: "InitAgg", slots: 1 },
        { op: "OpenScan", cursorId: 0, table: "t" },
        { op: "Label", name: "loop" },
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        { op: "LoadColumn", cursorId: 0, column: "x" },
        { op: "UpdateAgg", slot: 0, func: "SUM" },
        { op: "AdvanceCursor", cursorId: 0 },
        { op: "Jump", label: "loop" },
        { op: "Label", name: "done" },
        { op: "BeginRow" },
        { op: "FinalizeAgg", slot: 0, func: "SUM", alias: "s" },
        { op: "EmitColumn", name: "s" },
        { op: "EmitRow" },
        { op: "Halt" },
      ],
      labels: new Map([["loop", 2], ["done", 8]]),
      resultSchema: ["s"],
    };
    const result = execute(prog, db);
    // With no rows, the global group is never created so FinalizeAgg pushes null (line 374)
    expect(result.rows[0][0]).toBeNull();
  });
});

describe("sql-vm: GROUP_CONCAT via manual program", () => {
  test("GROUP_CONCAT accumulator via UpdateAgg/FinalizeAgg", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (s TEXT)", db);
    runSql("INSERT INTO t (s) VALUES ('a')", db);
    runSql("INSERT INTO t (s) VALUES ('b')", db);
    runSql("INSERT INTO t (s) VALUES ('c')", db);
    // Manual program: scan loop → accumulate → finalize once.
    // Label "scan" must be BEFORE JumpIfExhausted so Jump → "scan" re-enters the check.
    const prog: Program = {
      instructions: [
        // 0
        { op: "InitAgg", slots: 1 },
        // 1
        { op: "OpenScan", cursorId: 0, table: "t" },
        // 2  ← scan loop start
        { op: "Label", name: "scan" },
        // 3
        { op: "JumpIfExhausted", cursorId: 0, label: "done" },
        // 4
        { op: "LoadColumn", cursorId: 0, column: "s" },
        // 5
        { op: "UpdateAgg", slot: 0, func: "GROUP_CONCAT" },
        // 6
        { op: "AdvanceCursor", cursorId: 0 },
        // 7
        { op: "Jump", label: "scan" },
        // 8  ← done
        { op: "Label", name: "done" },
        // 9
        { op: "BeginRow" },
        // 10
        { op: "FinalizeAgg", slot: 0, func: "GROUP_CONCAT", alias: "gc" },
        // 11
        { op: "EmitColumn", name: "gc" },
        // 12
        { op: "EmitRow" },
        // 13
        { op: "Halt" },
      ],
      labels: new Map([["scan", 2], ["done", 8]]),
      resultSchema: ["gc"],
    };
    const result = execute(prog, db);
    expect(result.rows[0][0]).toBe("a,b,c");
  });
});

// ---------------------------------------------------------------------------
// Transactions (no-ops, just cover the instructions)
// ---------------------------------------------------------------------------
describe("sql-vm: transactions", () => {
  test("transaction instructions are no-ops", () => {
    const db = freshDb();
    const prog: Program = {
      instructions: [
        { op: "BeginTransaction" },
        { op: "CommitTransaction" },
        { op: "RollbackTransaction" },
        { op: "Halt" },
      ],
      labels: new Map(),
      resultSchema: [],
    };
    expect(() => execute(prog, db)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Multiple ORDER BY keys
// ---------------------------------------------------------------------------
describe("sql-vm: multiple ORDER BY keys", () => {
  test("ORDER BY two columns", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (a INTEGER, b INTEGER)", db);
    runSql("INSERT INTO t (a, b) VALUES (1, 2)", db);
    runSql("INSERT INTO t (a, b) VALUES (1, 1)", db);
    runSql("INSERT INTO t (a, b) VALUES (2, 0)", db);
    const r = runSql("SELECT a, b FROM t ORDER BY a ASC, b ASC", db);
    expect(r.rows[0]).toEqual([1, 1]);
    expect(r.rows[1]).toEqual([1, 2]);
    expect(r.rows[2]).toEqual([2, 0]);
  });
});

// ---------------------------------------------------------------------------
// LIMIT + OFFSET
// ---------------------------------------------------------------------------
describe("sql-vm: LIMIT with OFFSET", () => {
  test("LIMIT 1 OFFSET 1 skips first row", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (id INTEGER)", db);
    runSql("INSERT INTO t (id) VALUES (1)", db);
    runSql("INSERT INTO t (id) VALUES (2)", db);
    runSql("INSERT INTO t (id) VALUES (3)", db);
    const r = runSql("SELECT id FROM t LIMIT 1 OFFSET 1", db);
    expect(r.rows.length).toBe(1);
    expect(r.rows[0][0]).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// rowsAffected in QueryResult
// ---------------------------------------------------------------------------
describe("sql-vm: rowsAffected", () => {
  test("SELECT returns rowsAffected = -1", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    runSql("INSERT INTO t (x) VALUES (1)", db);
    const prog = compile(optimize(plan(parseSQL("SELECT x FROM t"))));
    const result = execute(prog, db);
    expect(result.rowsAffected).toBe(-1);
  });

  test("INSERT returns rowsAffected = number of rows inserted", () => {
    const db = freshDb();
    runSql("CREATE TABLE t (x INTEGER)", db);
    const prog = compile(optimize(plan(parseSQL("INSERT INTO t (x) VALUES (1), (2), (3)"))));
    const result = execute(prog, db);
    expect(result.rowsAffected).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------
describe("sql-vm: errors", () => {
  test("SELECT from unknown table throws VmError", () => {
    const db = freshDb();
    expect(() => runSql("SELECT * FROM nonexistent", db)).toThrow(VmError);
  });

  test("unknown instruction throws VmError", () => {
    const db = freshDb();
    const prog: Program = {
      instructions: [{ op: "Halt" }],  // will be replaced via cast
      labels: new Map(),
      resultSchema: [],
    };
    // Inject a bad opcode at runtime to hit the default branch.
    (prog.instructions as unknown as Array<{ op: string }>)[0] = { op: "BadInstruction" };
    expect(() => execute(prog, db)).toThrow(VmError);
  });

  test("unknown label throws VmError", () => {
    const db = freshDb();
    const prog: Program = {
      instructions: [
        { op: "Jump", label: "nonexistent_label" },
        { op: "Halt" },
      ],
      labels: new Map(),
      resultSchema: [],
    };
    expect(() => execute(prog, db)).toThrow(VmError);
  });

  test("stack underflow throws VmError", () => {
    const db = freshDb();
    const prog: Program = {
      instructions: [
        { op: "Pop" },  // pop from empty stack
        { op: "Halt" },
      ],
      labels: new Map(),
      resultSchema: [],
    };
    expect(() => execute(prog, db)).toThrow(VmError);
  });
});
