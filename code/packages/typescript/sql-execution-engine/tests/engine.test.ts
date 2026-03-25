/**
 * Integration tests for the SQL execution engine.
 *
 * Data Model:
 *
 * employees:
 *   id | name  | dept_id | salary | active
 *   1  | Alice | 1       | 90000  | true
 *   2  | Bob   | 2       | 75000  | true
 *   3  | Carol | 1       | 95000  | false
 *   4  | Dave  | null    | 60000  | true
 *
 * departments:
 *   id | name        | budget
 *   1  | Engineering | 500000
 *   2  | Marketing   | 200000
 */

import { describe, test, expect } from "vitest";
import {
  execute,
  executeAll,
  DataSource,
  QueryResult,
  TableNotFoundError,
  ColumnNotFoundError,
  ExecutionError,
} from "../src/index.js";
import type { Row, SqlValue } from "../src/index.js";

// ---------------------------------------------------------------------------
// In-memory test source
// ---------------------------------------------------------------------------

const EMPLOYEES: Row[] = [
  { id: 1, name: "Alice", dept_id: 1,    salary: 90000, active: true  },
  { id: 2, name: "Bob",   dept_id: 2,    salary: 75000, active: true  },
  { id: 3, name: "Carol", dept_id: 1,    salary: 95000, active: false },
  { id: 4, name: "Dave",  dept_id: null, salary: 60000, active: true  },
];

const DEPARTMENTS: Row[] = [
  { id: 1, name: "Engineering", budget: 500000 },
  { id: 2, name: "Marketing",   budget: 200000 },
];

class InMemorySource implements DataSource {
  schema(tableName: string): string[] {
    if (tableName === "employees") return ["id", "name", "dept_id", "salary", "active"];
    if (tableName === "departments") return ["id", "name", "budget"];
    throw new TableNotFoundError(tableName);
  }

  scan(tableName: string): Row[] {
    if (tableName === "employees") return EMPLOYEES.map((r) => ({ ...r }));
    if (tableName === "departments") return DEPARTMENTS.map((r) => ({ ...r }));
    throw new TableNotFoundError(tableName);
  }
}

const SOURCE = new InMemorySource();

function run(sql: string): QueryResult {
  return execute(sql, SOURCE);
}

// ---------------------------------------------------------------------------
// Test 1: SELECT *
// ---------------------------------------------------------------------------

describe("SELECT *", () => {
  test("returns all rows", () => {
    const result = run("SELECT * FROM employees");
    expect(result.rows).toHaveLength(4);
  });

  test("includes all columns", () => {
    const result = run("SELECT * FROM employees");
    expect(result.columns).toContain("name");
    expect(result.columns).toContain("salary");
  });
});

// ---------------------------------------------------------------------------
// Test 2: SELECT specific columns
// ---------------------------------------------------------------------------

describe("SELECT specific columns", () => {
  test("projects only requested columns", () => {
    const result = run("SELECT id, name FROM employees");
    expect(result.columns).toEqual(["id", "name"]);
    expect(result.rows).toHaveLength(4);
  });
});

// ---------------------------------------------------------------------------
// Test 3: AS alias
// ---------------------------------------------------------------------------

describe("AS alias", () => {
  test("renames the output column", () => {
    const result = run("SELECT id, name AS employee_name FROM employees");
    expect(result.columns).toContain("employee_name");
    expect(result.columns).not.toContain("name");
  });
});

// ---------------------------------------------------------------------------
// Test 4: WHERE salary > N
// ---------------------------------------------------------------------------

describe("WHERE numeric comparison", () => {
  test("filters rows by salary > 80000", () => {
    const result = run("SELECT name FROM employees WHERE salary > 80000");
    const names = new Set(result.rows.map((r) => r["name"]));
    expect(names).toEqual(new Set(["Alice", "Carol"]));
  });
});

// ---------------------------------------------------------------------------
// Test 5: WHERE boolean
// ---------------------------------------------------------------------------

describe("WHERE boolean", () => {
  test("active = TRUE returns 3 rows", () => {
    const result = run("SELECT name FROM employees WHERE active = TRUE");
    expect(result.rows).toHaveLength(3);
    const names = result.rows.map((r) => r["name"]);
    expect(names).not.toContain("Carol");
  });
});

// ---------------------------------------------------------------------------
// Test 6: WHERE IS NULL
// ---------------------------------------------------------------------------

describe("WHERE IS NULL", () => {
  test("returns only Dave", () => {
    const result = run("SELECT name FROM employees WHERE dept_id IS NULL");
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]["name"]).toBe("Dave");
  });
});

// ---------------------------------------------------------------------------
// Test 7: WHERE IS NOT NULL
// ---------------------------------------------------------------------------

describe("WHERE IS NOT NULL", () => {
  test("excludes Dave", () => {
    const result = run("SELECT name FROM employees WHERE dept_id IS NOT NULL");
    expect(result.rows).toHaveLength(3);
    const names = result.rows.map((r) => r["name"]);
    expect(names).not.toContain("Dave");
  });
});

// ---------------------------------------------------------------------------
// Test 8: WHERE BETWEEN
// ---------------------------------------------------------------------------

describe("WHERE BETWEEN", () => {
  test("salary BETWEEN 70000 AND 90000", () => {
    const result = run("SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000");
    const names = new Set(result.rows.map((r) => r["name"]));
    expect(names).toEqual(new Set(["Alice", "Bob"]));
  });
});

// ---------------------------------------------------------------------------
// Test 9: WHERE IN
// ---------------------------------------------------------------------------

describe("WHERE IN", () => {
  test("id IN (1, 3) returns Alice and Carol", () => {
    const result = run("SELECT name FROM employees WHERE id IN (1, 3)");
    const names = new Set(result.rows.map((r) => r["name"]));
    expect(names).toEqual(new Set(["Alice", "Carol"]));
  });
});

// ---------------------------------------------------------------------------
// Test 10: WHERE LIKE
// ---------------------------------------------------------------------------

describe("WHERE LIKE", () => {
  test("name LIKE 'A%' returns Alice", () => {
    const result = run("SELECT name FROM employees WHERE name LIKE 'A%'");
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]["name"]).toBe("Alice");
  });

  test("name LIKE '%ob' returns Bob", () => {
    const result = run("SELECT name FROM employees WHERE name LIKE '%ob'");
    expect(result.rows.map((r) => r["name"])).toContain("Bob");
  });
});

// ---------------------------------------------------------------------------
// Test 11: WHERE AND / OR / NOT
// ---------------------------------------------------------------------------

describe("WHERE logical operators", () => {
  test("AND narrows results", () => {
    const result = run("SELECT name FROM employees WHERE salary > 70000 AND active = TRUE");
    const names = new Set(result.rows.map((r) => r["name"]));
    expect(names).toEqual(new Set(["Alice", "Bob"]));
  });

  test("NOT inverts result", () => {
    const result = run("SELECT name FROM employees WHERE NOT active = TRUE");
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]["name"]).toBe("Carol");
  });
});

// ---------------------------------------------------------------------------
// Test 12: ORDER BY
// ---------------------------------------------------------------------------

describe("ORDER BY", () => {
  test("salary DESC — Carol first, Dave last", () => {
    const result = run("SELECT name FROM employees ORDER BY salary DESC");
    const names = result.rows.map((r) => r["name"] as string);
    expect(names[0]).toBe("Carol");
    expect(names[names.length - 1]).toBe("Dave");
  });

  test("name ASC — alphabetical order", () => {
    const result = run("SELECT name FROM employees ORDER BY name ASC");
    const names = result.rows.map((r) => r["name"] as string);
    const sorted = [...names].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));
    expect(names).toEqual(sorted);
  });
});

// ---------------------------------------------------------------------------
// Test 13: LIMIT and OFFSET
// ---------------------------------------------------------------------------

describe("LIMIT and OFFSET", () => {
  test("LIMIT 2 returns 2 rows", () => {
    const result = run("SELECT id FROM employees LIMIT 2");
    expect(result.rows).toHaveLength(2);
  });

  test("LIMIT 2 OFFSET 1 returns the second page", () => {
    const all = run("SELECT id FROM employees ORDER BY id ASC");
    const page = run("SELECT id FROM employees ORDER BY id ASC LIMIT 2 OFFSET 1");
    expect(page.rows).toHaveLength(2);
    expect(page.rows[0]["id"]).toBe(all.rows[1]["id"]);
  });
});

// ---------------------------------------------------------------------------
// Test 14: SELECT DISTINCT
// ---------------------------------------------------------------------------

describe("SELECT DISTINCT", () => {
  test("returns 3 distinct dept_id values (1, 2, null)", () => {
    const result = run("SELECT DISTINCT dept_id FROM employees");
    expect(result.rows).toHaveLength(3);
  });
});

// ---------------------------------------------------------------------------
// Test 15: INNER JOIN
// ---------------------------------------------------------------------------

describe("INNER JOIN", () => {
  test("excludes Dave (NULL dept_id)", () => {
    const result = run(
      "SELECT employees.name, departments.name " +
      "FROM employees INNER JOIN departments " +
      "ON employees.dept_id = departments.id"
    );
    expect(result.rows).toHaveLength(3);
  });
});

// ---------------------------------------------------------------------------
// Test 16: LEFT JOIN
// ---------------------------------------------------------------------------

describe("LEFT JOIN", () => {
  test("includes Dave with null department", () => {
    const result = run(
      "SELECT employees.name " +
      "FROM employees LEFT JOIN departments " +
      "ON employees.dept_id = departments.id"
    );
    expect(result.rows).toHaveLength(4);
    const names = result.rows.map((r) => r["employees.name"]);
    expect(names).toContain("Dave");
  });

  test("Dave has null dept_name", () => {
    const result = run(
      "SELECT employees.name, departments.name AS dept_name " +
      "FROM employees LEFT JOIN departments " +
      "ON employees.dept_id = departments.id"
    );
    const daveRow = result.rows.find((r) => r["employees.name"] === "Dave");
    expect(daveRow).toBeDefined();
    expect(daveRow!["dept_name"]).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Test 17: COUNT(*) and AVG
// ---------------------------------------------------------------------------

describe("Aggregate functions", () => {
  test("COUNT(*) returns 4", () => {
    const result = run("SELECT COUNT(*) FROM employees");
    expect(result.rows).toHaveLength(1);
    const val = Object.values(result.rows[0])[0];
    expect(val).toBe(4);
  });

  test("AVG(salary) returns correct average", () => {
    const result = run("SELECT AVG(salary) FROM employees");
    expect(result.rows).toHaveLength(1);
    const val = Object.values(result.rows[0])[0] as number;
    const expected = (90000 + 75000 + 95000 + 60000) / 4;
    expect(val).toBeCloseTo(expected, 1);
  });
});

// ---------------------------------------------------------------------------
// Test 18: GROUP BY
// ---------------------------------------------------------------------------

describe("GROUP BY", () => {
  test("produces 3 groups (dept 1, dept 2, null)", () => {
    const result = run("SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id");
    expect(result.rows).toHaveLength(3);
  });

  test("SUM(salary) per dept is correct", () => {
    const result = run(
      "SELECT dept_id, SUM(salary) FROM employees " +
      "WHERE dept_id IS NOT NULL GROUP BY dept_id"
    );
    const byDept = Object.fromEntries(result.rows.map((r) => [r["dept_id"], r["SUM(salary)"]]));
    expect(byDept[1]).toBe(185000);
    expect(byDept[2]).toBe(75000);
  });
});

// ---------------------------------------------------------------------------
// Test 19: HAVING
// ---------------------------------------------------------------------------

describe("HAVING", () => {
  test("filters groups by SUM(salary) > 100000", () => {
    const result = run(
      "SELECT dept_id, SUM(salary) FROM employees " +
      "WHERE dept_id IS NOT NULL " +
      "GROUP BY dept_id " +
      "HAVING SUM(salary) > 100000"
    );
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]["dept_id"]).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Test 20: Arithmetic
// ---------------------------------------------------------------------------

describe("Arithmetic in SELECT", () => {
  test("salary * 1.1 AS adjusted", () => {
    const result = run("SELECT salary * 1.1 AS adjusted FROM employees WHERE id = 1");
    expect(result.rows).toHaveLength(1);
    const adj = result.rows[0]["adjusted"] as number;
    expect(adj).toBeCloseTo(99000, 0);
  });
});

// ---------------------------------------------------------------------------
// Test 21: TableNotFoundError
// ---------------------------------------------------------------------------

describe("TableNotFoundError", () => {
  test("unknown table throws TableNotFoundError", () => {
    expect(() => run("SELECT * FROM nonexistent")).toThrow(TableNotFoundError);
  });

  test("TableNotFoundError is ExecutionError", () => {
    expect(() => run("SELECT * FROM nonexistent")).toThrow(ExecutionError);
  });

  test("tableName attribute is set", () => {
    try {
      run("SELECT * FROM nonexistent");
    } catch (e) {
      expect(e).toBeInstanceOf(TableNotFoundError);
      expect((e as TableNotFoundError).tableName).toBe("nonexistent");
    }
  });
});

// ---------------------------------------------------------------------------
// Test 22: ColumnNotFoundError
// ---------------------------------------------------------------------------

describe("ColumnNotFoundError", () => {
  test("unknown column in WHERE throws ColumnNotFoundError", () => {
    expect(() => run("SELECT id FROM employees WHERE fake_col = 1")).toThrow(ColumnNotFoundError);
  });

  test("columnName attribute is set", () => {
    try {
      run("SELECT id FROM employees WHERE fake_col = 1");
    } catch (e) {
      expect(e).toBeInstanceOf(ColumnNotFoundError);
      expect((e as ColumnNotFoundError).columnName).toBe("fake_col");
    }
  });
});

// ---------------------------------------------------------------------------
// Test 23: executeAll
// ---------------------------------------------------------------------------

describe("executeAll", () => {
  test("executes multiple statements", () => {
    const results = executeAll(
      "SELECT id FROM employees; SELECT id FROM departments",
      SOURCE
    );
    expect(results).toHaveLength(2);
    expect(results[0].rows).toHaveLength(4);
    expect(results[1].rows).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// Test 24: MIN / MAX
// ---------------------------------------------------------------------------

describe("MIN / MAX", () => {
  test("MIN(salary) is 60000", () => {
    const result = run("SELECT MIN(salary) FROM employees");
    const val = Object.values(result.rows[0])[0];
    expect(val).toBe(60000);
  });

  test("MAX(salary) is 95000", () => {
    const result = run("SELECT MAX(salary) FROM employees");
    const val = Object.values(result.rows[0])[0];
    expect(val).toBe(95000);
  });
});

// ---------------------------------------------------------------------------
// Test 25: COUNT(col) skips NULLs
// ---------------------------------------------------------------------------

describe("COUNT(col) skips NULLs", () => {
  test("COUNT(dept_id) = 3 (Dave excluded)", () => {
    const result = run("SELECT COUNT(dept_id) FROM employees");
    const val = Object.values(result.rows[0])[0];
    expect(val).toBe(3);
  });
});
