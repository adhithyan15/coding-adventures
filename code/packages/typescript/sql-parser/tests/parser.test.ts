/**
 * Tests for the SQL Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses SQL
 * text when loaded with the `sql.grammar` file.
 *
 * The SQL grammar's top-level rule is `program` — any valid SQL text is a
 * program containing one or more statements separated by semicolons.
 *
 * Because sql.tokens uses @case_insensitive true, all SQL keywords are
 * normalized to uppercase by the lexer. The parser grammar uses quoted
 * strings (like "SELECT") to match KEYWORD tokens with uppercase values.
 * This means `select`, `SELECT`, and `Select` all parse identically.
 *
 * Test Strategy
 * -------------
 *
 * Each test parses a SQL string, then uses helper functions to walk the
 * resulting AST, looking for specific node types or tokens. This approach
 * is more robust than checking exact tree structure.
 *
 * Test Categories
 * ---------------
 *
 *   1.  **Basic SELECT** -- SELECT 1, SELECT *, simple from clause
 *   2.  **Case-insensitive keywords** -- select/SELECT/Select all parse
 *   3.  **WHERE clause** -- filtering with expressions
 *   4.  **Multiple columns** -- comma-separated select list
 *   5.  **AS aliases** -- column and table aliases
 *   6.  **ORDER BY** -- ASC, DESC
 *   7.  **LIMIT / OFFSET** -- result pagination
 *   8.  **GROUP BY / HAVING** -- aggregation
 *   9.  **JOIN** -- INNER, LEFT, etc.
 *   10. **INSERT INTO VALUES**
 *   11. **UPDATE SET WHERE**
 *   12. **DELETE FROM**
 *   13. **CREATE TABLE IF NOT EXISTS**
 *   14. **DROP TABLE IF EXISTS**
 *   15. **Expression forms** -- arithmetic, comparison, AND/OR/NOT, BETWEEN, IN, LIKE, IS NULL
 *   16. **Multiple statements** -- separated by semicolons
 *   17. **Error cases** -- invalid SQL throws
 *   18. **Factory function** -- createSQLParser works
 */

import { describe, it, expect } from "vitest";
import { parseSQL, createSQLParser } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// AST traversal helpers
// ---------------------------------------------------------------------------

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This is the workhorse helper for these tests. Since the grammar wraps
 * statements inside "program" → "statement" → specific statement rules,
 * we need to search the entire tree to find the nodes we care about.
 */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

/**
 * Collect all leaf tokens from an AST subtree.
 *
 * Flattens the tree into a list of tokens, which makes it easy to check
 * what tokens are present without worrying about nesting structure.
 */
function findTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child));
    } else {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

/**
 * Check whether any token in the subtree has the given type and value.
 */
function hasToken(node: ASTNode, type: string, value?: string): boolean {
  const tokens = findTokens(node);
  return tokens.some((t) => t.type === type && (value === undefined || t.value === value));
}

// ---------------------------------------------------------------------------
// Basic SELECT
// ---------------------------------------------------------------------------

describe("basic SELECT", () => {
  it("parses a minimal SELECT to root 'program' node", () => {
    /**
     * A minimal SELECT with a literal value and a FROM clause.
     * The top-level rule in sql.grammar is "program", so the AST root
     * should always have ruleName === "program".
     *
     * Note: the sql.grammar requires FROM — SELECT without FROM is not
     * part of this ANSI SQL subset grammar.
     */
    const ast = parseSQL("SELECT 1 FROM t");
    expect(ast.ruleName).toBe("program");
  });

  it("parses SELECT * FROM table", () => {
    /**
     * SELECT * uses the STAR token to mean "all columns". The grammar
     * defines: select_list = STAR | select_item { "," select_item }
     * so STAR at the head of the select list is valid.
     */
    const ast = parseSQL("SELECT * FROM users");
    expect(ast.ruleName).toBe("program");

    const selectNodes = findNodes(ast, "select_stmt");
    expect(selectNodes).toHaveLength(1);

    // The STAR token should appear somewhere in the tree
    expect(hasToken(ast, "STAR")).toBe(true);
  });

  it("parses case-insensitive 'select' (lowercase)", () => {
    /**
     * The sql-lexer normalizes 'select' to KEYWORD("SELECT") before
     * the parser sees it. The parser grammar uses "SELECT" (uppercase),
     * so this should parse identically to uppercase SELECT.
     */
    const ast = parseSQL("select 1 from users");
    expect(ast.ruleName).toBe("program");

    const selectNodes = findNodes(ast, "select_stmt");
    expect(selectNodes).toHaveLength(1);
  });

  it("parses case-insensitive 'Select' (mixed-case)", () => {
    /**
     * Mixed-case keywords also normalize to uppercase via the lexer.
     */
    const ast = parseSQL("Select * From users");
    expect(ast.ruleName).toBe("program");

    const selectNodes = findNodes(ast, "select_stmt");
    expect(selectNodes).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// WHERE clause
// ---------------------------------------------------------------------------

describe("WHERE clause", () => {
  it("parses SELECT with WHERE clause", () => {
    /**
     * WHERE filters rows based on a condition. The grammar rule is:
     *   where_clause = "WHERE" expr
     *
     * The condition is an expression — in this case a simple comparison.
     */
    const ast = parseSQL("SELECT id FROM users WHERE id = 1");
    expect(ast.ruleName).toBe("program");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);
  });

  it("parses WHERE with AND condition", () => {
    /**
     * Multiple conditions combined with AND. The grammar's expression
     * hierarchy handles AND: and_expr = not_expr { "AND" not_expr }
     */
    const ast = parseSQL("SELECT id FROM users WHERE age > 18 AND active = 1");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Multiple columns
// ---------------------------------------------------------------------------

describe("multiple columns", () => {
  it("parses SELECT with multiple comma-separated columns", () => {
    /**
     * Comma-separated select items: select_list = select_item { "," select_item }
     */
    const ast = parseSQL("SELECT id, name, email FROM users");

    const selectItems = findNodes(ast, "select_item");
    expect(selectItems.length).toBeGreaterThanOrEqual(3);
  });
});

// ---------------------------------------------------------------------------
// AS aliases
// ---------------------------------------------------------------------------

describe("AS aliases", () => {
  it("parses SELECT column AS alias", () => {
    /**
     * Column aliases use the AS keyword followed by a NAME:
     *   select_item = expr [ "AS" NAME ]
     *
     * Aliases make result columns easier to reference in application code.
     */
    const ast = parseSQL("SELECT name AS username FROM users");

    const selectItems = findNodes(ast, "select_item");
    expect(selectItems.length).toBeGreaterThanOrEqual(1);

    // AS keyword should appear in the tree
    expect(hasToken(ast, "KEYWORD", "AS")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// ORDER BY
// ---------------------------------------------------------------------------

describe("ORDER BY", () => {
  it("parses SELECT with ORDER BY", () => {
    /**
     * ORDER BY sorts the result set. The grammar rule is:
     *   order_clause = "ORDER" "BY" order_item { "," order_item }
     *   order_item   = expr [ "ASC" | "DESC" ]
     */
    const ast = parseSQL("SELECT id FROM users ORDER BY name");

    const orderClauses = findNodes(ast, "order_clause");
    expect(orderClauses).toHaveLength(1);
  });

  it("parses ORDER BY with ASC", () => {
    /**
     * Explicit ascending sort direction.
     */
    const ast = parseSQL("SELECT id FROM users ORDER BY name ASC");

    expect(hasToken(ast, "KEYWORD", "ASC")).toBe(true);
  });

  it("parses ORDER BY with DESC", () => {
    /**
     * Descending sort direction — commonly used for "most recent first".
     */
    const ast = parseSQL("SELECT id FROM users ORDER BY created_at DESC");

    expect(hasToken(ast, "KEYWORD", "DESC")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// LIMIT and OFFSET
// ---------------------------------------------------------------------------

describe("LIMIT and OFFSET", () => {
  it("parses SELECT with LIMIT", () => {
    /**
     * LIMIT restricts the number of rows returned:
     *   limit_clause = "LIMIT" NUMBER [ "OFFSET" NUMBER ]
     */
    const ast = parseSQL("SELECT id FROM users LIMIT 10");

    const limitClauses = findNodes(ast, "limit_clause");
    expect(limitClauses).toHaveLength(1);
  });

  it("parses SELECT with LIMIT and OFFSET", () => {
    /**
     * LIMIT + OFFSET implements pagination:
     *   - LIMIT 10 OFFSET 20 means "skip 20 rows, return the next 10"
     *   - This is equivalent to "page 3 of 10 results per page"
     */
    const ast = parseSQL("SELECT id FROM users LIMIT 10 OFFSET 20");

    const limitClauses = findNodes(ast, "limit_clause");
    expect(limitClauses).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "OFFSET")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// GROUP BY and HAVING
// ---------------------------------------------------------------------------

describe("GROUP BY and HAVING", () => {
  it("parses SELECT with GROUP BY", () => {
    /**
     * GROUP BY aggregates rows with matching values:
     *   group_clause = "GROUP" "BY" column_ref { "," column_ref }
     *
     * Combined with aggregate functions like COUNT(*) or SUM(amount),
     * GROUP BY produces one row per unique combination of values.
     */
    const ast = parseSQL("SELECT status, COUNT(*) FROM orders GROUP BY status");

    const groupClauses = findNodes(ast, "group_clause");
    expect(groupClauses).toHaveLength(1);
  });

  it("parses SELECT with GROUP BY and HAVING", () => {
    /**
     * HAVING filters groups (like WHERE but applied after aggregation):
     *   having_clause = "HAVING" expr
     */
    const ast = parseSQL(
      "SELECT status, COUNT(*) FROM orders GROUP BY status HAVING COUNT(*) > 5"
    );

    const groupClauses = findNodes(ast, "group_clause");
    expect(groupClauses).toHaveLength(1);

    const havingClauses = findNodes(ast, "having_clause");
    expect(havingClauses).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// JOIN
// ---------------------------------------------------------------------------

describe("JOIN", () => {
  it("parses SELECT with INNER JOIN", () => {
    /**
     * JOIN combines rows from two tables based on a condition:
     *   join_clause = join_type "JOIN" table_ref "ON" expr
     *
     * INNER JOIN returns only rows where the condition matches in both tables.
     */
    const ast = parseSQL(
      "SELECT u.id, o.amount FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id"
    );

    const joinClauses = findNodes(ast, "join_clause");
    expect(joinClauses).toHaveLength(1);
  });

  it("parses SELECT with LEFT JOIN", () => {
    /**
     * LEFT JOIN returns all rows from the left table, plus matching rows
     * from the right table (NULL if no match exists).
     */
    const ast = parseSQL(
      "SELECT u.id FROM users AS u LEFT JOIN orders AS o ON u.id = o.user_id"
    );

    const joinClauses = findNodes(ast, "join_clause");
    expect(joinClauses).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// INSERT INTO VALUES
// ---------------------------------------------------------------------------

describe("INSERT INTO VALUES", () => {
  it("parses a simple INSERT statement", () => {
    /**
     * INSERT INTO adds new rows to a table:
     *   insert_stmt = "INSERT" "INTO" NAME
     *                 [ "(" NAME { "," NAME } ")" ]
     *                 "VALUES" row_value { "," row_value }
     *
     * The column list is optional — if omitted, values are matched
     * positionally to all table columns.
     */
    const ast = parseSQL("INSERT INTO users (name, age) VALUES ('Alice', 30)");
    expect(ast.ruleName).toBe("program");

    const insertNodes = findNodes(ast, "insert_stmt");
    expect(insertNodes).toHaveLength(1);
  });

  it("parses INSERT without column list", () => {
    /**
     * INSERT without explicit column names inserts values in column order.
     */
    const ast = parseSQL("INSERT INTO users VALUES (1, 'Bob', 25)");

    const insertNodes = findNodes(ast, "insert_stmt");
    expect(insertNodes).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// UPDATE SET WHERE
// ---------------------------------------------------------------------------

describe("UPDATE SET WHERE", () => {
  it("parses a simple UPDATE statement", () => {
    /**
     * UPDATE modifies existing rows:
     *   update_stmt = "UPDATE" NAME "SET" assignment { "," assignment }
     *                 [ where_clause ]
     */
    const ast = parseSQL("UPDATE users SET name = 'Alice' WHERE id = 1");
    expect(ast.ruleName).toBe("program");

    const updateNodes = findNodes(ast, "update_stmt");
    expect(updateNodes).toHaveLength(1);
  });

  it("parses UPDATE with multiple assignments", () => {
    /**
     * Multiple column assignments are separated by commas.
     */
    const ast = parseSQL("UPDATE users SET name = 'Alice', age = 30 WHERE id = 1");

    const updateNodes = findNodes(ast, "update_stmt");
    expect(updateNodes).toHaveLength(1);

    const assignments = findNodes(ast, "assignment");
    expect(assignments.length).toBeGreaterThanOrEqual(2);
  });
});

// ---------------------------------------------------------------------------
// DELETE FROM
// ---------------------------------------------------------------------------

describe("DELETE FROM", () => {
  it("parses a simple DELETE statement", () => {
    /**
     * DELETE removes rows from a table:
     *   delete_stmt = "DELETE" "FROM" NAME [ where_clause ]
     *
     * Always include a WHERE clause in real code — without it, all rows
     * in the table are deleted.
     */
    const ast = parseSQL("DELETE FROM users WHERE id = 1");
    expect(ast.ruleName).toBe("program");

    const deleteNodes = findNodes(ast, "delete_stmt");
    expect(deleteNodes).toHaveLength(1);
  });

  it("parses DELETE without WHERE (deletes all rows)", () => {
    /**
     * DELETE FROM table with no WHERE clause deletes every row.
     * The grammar allows this because WHERE is optional.
     */
    const ast = parseSQL("DELETE FROM temp_data");

    const deleteNodes = findNodes(ast, "delete_stmt");
    expect(deleteNodes).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// CREATE TABLE IF NOT EXISTS
// ---------------------------------------------------------------------------

describe("CREATE TABLE", () => {
  it("parses CREATE TABLE", () => {
    /**
     * CREATE TABLE defines a new table schema:
     *   create_table_stmt = "CREATE" "TABLE" [ "IF" "NOT" "EXISTS" ] NAME
     *                       "(" col_def { "," col_def } ")"
     */
    const ast = parseSQL("CREATE TABLE users (id INTEGER, name TEXT)");
    expect(ast.ruleName).toBe("program");

    const createNodes = findNodes(ast, "create_table_stmt");
    expect(createNodes).toHaveLength(1);
  });

  it("parses CREATE TABLE IF NOT EXISTS", () => {
    /**
     * IF NOT EXISTS prevents an error if the table already exists.
     * This is the idiomatic way to write table creation in migration scripts.
     */
    const ast = parseSQL(
      "CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT)"
    );

    const createNodes = findNodes(ast, "create_table_stmt");
    expect(createNodes).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "IF")).toBe(true);
    expect(hasToken(ast, "KEYWORD", "EXISTS")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// DROP TABLE IF EXISTS
// ---------------------------------------------------------------------------

describe("DROP TABLE", () => {
  it("parses DROP TABLE", () => {
    /**
     * DROP TABLE removes a table and all its data:
     *   drop_table_stmt = "DROP" "TABLE" [ "IF" "EXISTS" ] NAME
     */
    const ast = parseSQL("DROP TABLE users");
    expect(ast.ruleName).toBe("program");

    const dropNodes = findNodes(ast, "drop_table_stmt");
    expect(dropNodes).toHaveLength(1);
  });

  it("parses DROP TABLE IF EXISTS", () => {
    /**
     * IF EXISTS prevents an error if the table does not exist.
     */
    const ast = parseSQL("DROP TABLE IF EXISTS users");

    const dropNodes = findNodes(ast, "drop_table_stmt");
    expect(dropNodes).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "IF")).toBe(true);
    expect(hasToken(ast, "KEYWORD", "EXISTS")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Expression forms
// ---------------------------------------------------------------------------

describe("expression forms", () => {
  it("parses arithmetic expression in WHERE", () => {
    /**
     * SQL expressions support arithmetic operators: +, -, *, /, %
     * These follow standard operator precedence (multiplication before addition).
     */
    const ast = parseSQL("SELECT id FROM t WHERE price * 2 > 100");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);
  });

  it("parses comparison with != operator", () => {
    /**
     * Inequality comparisons use != or <> (both are NOT_EQUALS tokens).
     * The grammar handles: cmp_op = "=" | NOT_EQUALS | "<" | ">" | "<=" | ">="
     */
    const ast = parseSQL("SELECT id FROM t WHERE status != 'active'");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);
  });

  it("parses AND expression", () => {
    /**
     * AND combines two conditions (both must be true).
     * Operator precedence: AND binds tighter than OR.
     */
    const ast = parseSQL("SELECT id FROM t WHERE a = 1 AND b = 2");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);
  });

  it("parses OR expression", () => {
    /**
     * OR combines two conditions (at least one must be true).
     */
    const ast = parseSQL("SELECT id FROM t WHERE a = 1 OR b = 2");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);
  });

  it("parses NOT expression", () => {
    /**
     * NOT negates a condition.
     */
    const ast = parseSQL("SELECT id FROM t WHERE NOT active = 1");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "NOT")).toBe(true);
  });

  it("parses BETWEEN expression", () => {
    /**
     * BETWEEN is a range comparison shorthand:
     *   x BETWEEN low AND high  ≡  x >= low AND x <= high
     *
     * Grammar: comparison = additive [ "BETWEEN" additive "AND" additive ... ]
     */
    const ast = parseSQL("SELECT id FROM t WHERE age BETWEEN 18 AND 65");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "BETWEEN")).toBe(true);
  });

  it("parses IN expression", () => {
    /**
     * IN checks if a value matches any item in a list:
     *   x IN (val1, val2, val3)
     *
     * More readable than chained OR conditions.
     */
    const ast = parseSQL("SELECT id FROM t WHERE status IN ('active', 'pending')");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "IN")).toBe(true);
  });

  it("parses LIKE expression", () => {
    /**
     * LIKE performs pattern matching with wildcards:
     *   % matches any sequence of characters
     *   _ matches exactly one character
     *
     * Example: name LIKE 'Al%' matches 'Alice', 'Albert', etc.
     */
    const ast = parseSQL("SELECT id FROM t WHERE name LIKE 'Al%'");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "LIKE")).toBe(true);
  });

  it("parses IS NULL expression", () => {
    /**
     * IS NULL tests whether a value is NULL (unknown/absent).
     * You cannot use = NULL — you must use IS NULL.
     *
     * Grammar: comparison = additive [ ... | "IS" "NULL" | "IS" "NOT" "NULL" ]
     */
    const ast = parseSQL("SELECT id FROM t WHERE email IS NULL");

    const whereClauses = findNodes(ast, "where_clause");
    expect(whereClauses).toHaveLength(1);

    expect(hasToken(ast, "KEYWORD", "IS")).toBe(true);
    expect(hasToken(ast, "KEYWORD", "NULL")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Multiple statements
// ---------------------------------------------------------------------------

describe("multiple statements", () => {
  it("parses two statements separated by semicolons", () => {
    /**
     * The grammar rule for program is:
     *   program = statement { ";" statement } [ ";" ]
     *
     * Multiple statements in a single SQL script are separated by semicolons.
     * This is how database migration files and SQL scripts work.
     */
    const ast = parseSQL("SELECT 1 FROM t; SELECT 2 FROM t");
    expect(ast.ruleName).toBe("program");

    const selectNodes = findNodes(ast, "select_stmt");
    expect(selectNodes).toHaveLength(2);
  });

  it("parses multiple DML statements", () => {
    /**
     * A typical database migration might include INSERT, UPDATE, and DELETE
     * statements in sequence.
     */
    const ast = parseSQL(
      "INSERT INTO t VALUES (1); UPDATE t SET x = 2 WHERE id = 1; DELETE FROM t WHERE id = 2"
    );

    const insertNodes = findNodes(ast, "insert_stmt");
    expect(insertNodes).toHaveLength(1);

    const updateNodes = findNodes(ast, "update_stmt");
    expect(updateNodes).toHaveLength(1);

    const deleteNodes = findNodes(ast, "delete_stmt");
    expect(deleteNodes).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

describe("error cases", () => {
  it("rejects invalid SQL and returns an error", () => {
    /**
     * Completely invalid SQL that cannot match any grammar rule should
     * cause the parser to throw a ParseError. This ensures the parser
     * fails fast on bad input rather than silently producing a wrong AST.
     */
    expect(() => parseSQL("THIS IS NOT VALID SQL @@@")).toThrow();
  });

  it("rejects SQL with syntax errors", () => {
    /**
     * A SELECT with a missing FROM clause is syntactically invalid.
     * The parser should throw because the grammar requires FROM.
     */
    expect(() => parseSQL("SELECT WHERE id = 1")).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Factory function
// ---------------------------------------------------------------------------

describe("createSQLParser factory", () => {
  it("createSQLParser works and returns a program node", () => {
    /**
     * The createSQLParser factory function is an alias for parseSQL.
     * It should produce the same AST as calling parseSQL directly.
     */
    const ast = createSQLParser("SELECT id FROM t");
    expect(ast).not.toBeNull();
    expect(ast.ruleName).toBe("program");
  });

  it("createSQLParser and parseSQL produce identical results", () => {
    /**
     * Both functions should produce structurally identical ASTs
     * for the same input.
     */
    const source = "SELECT id, name FROM users WHERE id = 1";
    const fromFactory = createSQLParser(source);
    const fromFunction = parseSQL(source);

    // Both should be program nodes with the same structure
    expect(fromFactory.ruleName).toBe(fromFunction.ruleName);
    expect(fromFactory.children.length).toBe(fromFunction.children.length);
  });
});
