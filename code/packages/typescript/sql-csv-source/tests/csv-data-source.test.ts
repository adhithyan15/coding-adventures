/**
 * csv-data-source.test.ts
 * ───────────────────────
 * End-to-end tests for CsvDataSource against real CSV fixture files.
 *
 * These tests exercise the full pipeline:
 *   CSV files on disk → CsvDataSource → sql-execution-engine → QueryResult
 *
 * Fixture data:
 *
 *   employees.csv
 *     id  | name  | dept_id | salary | active
 *     1   | Alice | 1       | 90000  | true
 *     2   | Bob   | 2       | 75000  | true
 *     3   | Carol | 1       | 95000  | false
 *     4   | Dave  | (null)  | 60000  | true
 *
 *   departments.csv
 *     id | name        | budget
 *     1  | Engineering | 500000
 *     2  | Marketing   | 200000
 *
 * File location strategy:
 *   Tests run in Node.js (ESM) via vitest. We use `import.meta.url` to
 *   find the test file's own location, then navigate to the fixtures
 *   directory relative to it. This is the standard ESM-safe pattern for
 *   test fixtures (no `__dirname` available in ESM).
 */

import { describe, test, expect } from "vitest";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { execute } from "@coding-adventures/sql-execution-engine";
import { TableNotFoundError } from "@coding-adventures/sql-execution-engine";
import { CsvDataSource, coerce } from "../src/index.js";

// ---------------------------------------------------------------------------
// Locate fixture files using ESM-compatible __dirname equivalent.
// ---------------------------------------------------------------------------

// import.meta.url is the URL of THIS file: file:///…/tests/csv-data-source.test.ts
// fileURLToPath converts it to a system path.
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const FIXTURES = join(__dirname, "fixtures");

// ---------------------------------------------------------------------------
// Unit tests for the coerce() helper
// ---------------------------------------------------------------------------

describe("coerce()", () => {
  test("empty string → null", () => {
    expect(coerce("")).toBeNull();
  });

  test('"true" → true', () => {
    expect(coerce("true")).toBe(true);
  });

  test('"false" → false', () => {
    expect(coerce("false")).toBe(false);
  });

  test("integer string → number", () => {
    expect(coerce("42")).toBe(42);
    expect(typeof coerce("42")).toBe("number");
  });

  test("negative integer", () => {
    expect(coerce("-7")).toBe(-7);
  });

  test("zero", () => {
    expect(coerce("0")).toBe(0);
  });

  test("float string → number", () => {
    const result = coerce("3.14");
    expect(typeof result).toBe("number");
    expect(Math.abs((result as number) - 3.14)).toBeLessThan(1e-9);
  });

  test("decimal string not misread as integer", () => {
    // "42.5" should NOT be treated as integer 42.
    expect(coerce("42.5")).toBe(42.5);
  });

  test("plain string passes through", () => {
    expect(coerce("hello")).toBe("hello");
  });

  test("string with spaces passes through", () => {
    expect(coerce("Alice Smith")).toBe("Alice Smith");
  });
});

// ---------------------------------------------------------------------------
// Unit tests for schema()
// ---------------------------------------------------------------------------

describe("schema()", () => {
  const source = new CsvDataSource(FIXTURES);

  test("employees columns in header order", () => {
    expect(source.schema("employees")).toEqual([
      "id",
      "name",
      "dept_id",
      "salary",
      "active",
    ]);
  });

  test("departments columns in header order", () => {
    expect(source.schema("departments")).toEqual(["id", "name", "budget"]);
  });

  test("unknown table throws TableNotFoundError", () => {
    expect(() => source.schema("nonexistent")).toThrow(TableNotFoundError);
  });
});

// ---------------------------------------------------------------------------
// Unit tests for scan()
// ---------------------------------------------------------------------------

describe("scan()", () => {
  const source = new CsvDataSource(FIXTURES);

  test("employees returns 4 rows", () => {
    expect(source.scan("employees")).toHaveLength(4);
  });

  test("Alice row — all types coerced", () => {
    const rows = source.scan("employees");
    const alice = rows[0];
    expect(alice["id"]).toBe(1);
    expect(alice["name"]).toBe("Alice");
    expect(alice["dept_id"]).toBe(1);
    expect(alice["salary"]).toBe(90000);
    expect(alice["active"]).toBe(true);
  });

  test("Carol active is false", () => {
    const carol = source.scan("employees")[2];
    expect(carol["active"]).toBe(false);
  });

  test("Dave dept_id is null (empty CSV field)", () => {
    const dave = source.scan("employees")[3];
    expect(dave["dept_id"]).toBeNull();
  });

  test("departments budget is a number", () => {
    const rows = source.scan("departments");
    expect(rows[0]["budget"]).toBe(500000);
    expect(typeof rows[0]["budget"]).toBe("number");
  });

  test("unknown table throws TableNotFoundError", () => {
    expect(() => source.scan("missing")).toThrow(TableNotFoundError);
  });
});

// ---------------------------------------------------------------------------
// End-to-end SQL query tests
// ---------------------------------------------------------------------------

describe("SQL queries via CsvDataSource", () => {
  const source = new CsvDataSource(FIXTURES);
  const run = (sql: string) => execute(sql, source);

  // ── Test 1: SELECT * FROM employees ──────────────────────────────────────
  describe("SELECT * FROM employees", () => {
    test("returns correct columns", () => {
      const result = run("SELECT * FROM employees");
      expect(result.columns).toEqual([
        "id",
        "name",
        "dept_id",
        "salary",
        "active",
      ]);
    });

    test("returns 4 rows", () => {
      const result = run("SELECT * FROM employees");
      expect(result.rows).toHaveLength(4);
    });

    test("values are coerced (not raw strings)", () => {
      const result = run("SELECT * FROM employees");
      const alice = result.rows[0];
      expect(alice["id"]).toBe(1);
      expect(alice["active"]).toBe(true);
      const dave = result.rows[3];
      expect(dave["dept_id"]).toBeNull();
    });
  });

  // ── Test 2: WHERE active = true ──────────────────────────────────────────
  test("SELECT name WHERE active = true — Alice, Bob, Dave", () => {
    const result = run("SELECT name FROM employees WHERE active = true");
    const names = result.rows.map((r) => r["name"] as string).sort();
    expect(names).toEqual(["Alice", "Bob", "Dave"]);
  });

  // ── Test 3: WHERE dept_id IS NULL ────────────────────────────────────────
  test("SELECT WHERE dept_id IS NULL — only Dave", () => {
    const result = run("SELECT * FROM employees WHERE dept_id IS NULL");
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]["name"]).toBe("Dave");
  });

  // ── Test 4: INNER JOIN ───────────────────────────────────────────────────
  test("INNER JOIN — 3 rows (Dave excluded)", () => {
    const result = run(
      "SELECT e.name, d.name " +
        "FROM employees AS e " +
        "INNER JOIN departments AS d ON e.dept_id = d.id"
    );
    expect(result.rows).toHaveLength(3);
    const empNames = result.rows
      .map((r) => r["e.name"] as string)
      .sort();
    expect(empNames).toEqual(["Alice", "Bob", "Carol"]);
  });

  // ── Test 5: GROUP BY ─────────────────────────────────────────────────────
  test("GROUP BY dept_id — 3 groups including NULL", () => {
    const result = run(
      "SELECT dept_id, COUNT(*) AS cnt FROM employees GROUP BY dept_id"
    );
    // Three groups: dept_id=1 (Alice+Carol), dept_id=2 (Bob), NULL (Dave).
    expect(result.rows).toHaveLength(3);
    const dept1 = result.rows.find((r) => r["dept_id"] === 1);
    expect(dept1?.["cnt"]).toBe(2);
  });

  // ── Test 6: ORDER BY + LIMIT ─────────────────────────────────────────────
  test("ORDER BY salary DESC LIMIT 2 — Carol, Alice", () => {
    const result = run(
      "SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 2"
    );
    expect(result.rows).toHaveLength(2);
    expect(result.rows[0]["name"]).toBe("Carol");
    expect(result.rows[0]["salary"]).toBe(95000);
    expect(result.rows[1]["name"]).toBe("Alice");
    expect(result.rows[1]["salary"]).toBe(90000);
  });

  // ── Test 7: Unknown table ────────────────────────────────────────────────
  test("unknown table throws TableNotFoundError", () => {
    expect(() => run("SELECT * FROM ghosts")).toThrow(TableNotFoundError);
  });
});
