/**
 * Tests for the TOML Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses TOML
 * text when loaded with the `toml.grammar` file.
 *
 * The TOML grammar's top-level rule is `document` -- a TOML document is a
 * sequence of expressions (key-value pairs and table headers) separated by
 * newlines. This is different from JSON, whose top-level rule is `value`.
 *
 * Test Strategy
 * -------------
 *
 * Each test parses a TOML string and then uses helper functions to walk the
 * resulting AST, looking for specific node types. This approach is more robust
 * than checking exact tree structure, because the grammar may wrap nodes in
 * multiple layers of rules.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Simple key-value pairs** -- bare keys, quoted keys, dotted keys
 *   2. **Value types** -- strings, integers, floats, booleans, dates
 *   3. **Table headers** -- [table] and [[array-of-tables]]
 *   4. **Arrays** -- inline arrays, multi-line arrays
 *   5. **Inline tables** -- { key = value, ... }
 *   6. **Complete documents** -- multi-table documents with mixed value types
 *   7. **Edge cases** -- empty documents, comments-only, blank lines
 *   8. **Error cases** -- invalid TOML that should cause parse errors
 */

import { describe, it, expect } from "vitest";
import { parseTOML } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This is the workhorse helper for these tests. Since the grammar wraps
 * values in multiple layers of rules, we need to search the entire tree
 * to find the nodes we care about.
 *
 * @param node - The root node to search from.
 * @param ruleName - The grammar rule name to find (e.g., "keyval", "value").
 * @returns All nodes in the tree with the given ruleName.
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
 * what tokens are present in a particular subtree without worrying about
 * the exact nesting structure.
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
 * Helper: extract non-NEWLINE, non-EOF tokens from an AST for cleaner assertions.
 */
function meaningfulTokens(node: ASTNode): Token[] {
  return findTokens(node).filter(
    (t) => t.type !== "NEWLINE" && t.type !== "EOF",
  );
}

// =========================================================================
// Simple Key-Value Pairs
// =========================================================================

describe("simple key-value pairs", () => {
  it("parses a string key-value pair", () => {
    /**
     * The most basic TOML construct:
     *   title = "TOML Example"
     *
     * The top-level node should be "document", containing an "expression"
     * node, which contains a "keyval" node.
     */
    const ast = parseTOML('title = "TOML Example"');
    expect(ast.ruleName).toBe("document");

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);

    const tokens = meaningfulTokens(keyvals[0]);
    expect(tokens.some((t) => t.type === "BARE_KEY" && t.value === "title")).toBe(true);
    expect(tokens.some((t) => t.type === "BASIC_STRING" && t.value === "TOML Example")).toBe(true);
  });

  it("parses an integer key-value pair", () => {
    const ast = parseTOML("port = 8080");
    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);

    const tokens = meaningfulTokens(keyvals[0]);
    expect(tokens.some((t) => t.type === "INTEGER" && t.value === "8080")).toBe(true);
  });

  it("parses a float key-value pair", () => {
    const ast = parseTOML("pi = 3.14");
    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);

    const tokens = meaningfulTokens(keyvals[0]);
    expect(tokens.some((t) => t.type === "FLOAT" && t.value === "3.14")).toBe(true);
  });

  it("parses a boolean key-value pair", () => {
    const ast = parseTOML("enabled = true");
    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);

    const tokens = meaningfulTokens(keyvals[0]);
    expect(tokens.some((t) => t.type === "TRUE")).toBe(true);
  });

  it("parses a dotted key", () => {
    /**
     * Dotted keys create implicit intermediate tables:
     *   a.b.c = 1
     * is equivalent to:
     *   [a.b]
     *   c = 1
     */
    const ast = parseTOML("a.b.c = 1");
    const keys = findNodes(ast, "key");
    expect(keys.length).toBeGreaterThanOrEqual(1);

    const tokens = meaningfulTokens(ast);
    const dots = tokens.filter((t) => t.type === "DOT");
    expect(dots).toHaveLength(2);
  });

  it("parses a quoted key", () => {
    const ast = parseTOML('"key with spaces" = "value"');
    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);
  });
});

// =========================================================================
// Value Types
// =========================================================================

describe("value types", () => {
  it("parses a literal string value", () => {
    const ast = parseTOML("path = 'C:\\\\Users'");
    const tokens = meaningfulTokens(ast);
    expect(tokens.some((t) => t.type === "LITERAL_STRING")).toBe(true);
  });

  it("parses a date value", () => {
    const ast = parseTOML("birthday = 1979-05-27");
    const tokens = meaningfulTokens(ast);
    expect(tokens.some((t) => t.type === "LOCAL_DATE")).toBe(true);
  });

  it("parses a datetime value", () => {
    const ast = parseTOML("created = 1979-05-27T07:32:00Z");
    const tokens = meaningfulTokens(ast);
    expect(tokens.some((t) => t.type === "OFFSET_DATETIME")).toBe(true);
  });

  it("parses a time value", () => {
    const ast = parseTOML("alarm = 07:32:00");
    const tokens = meaningfulTokens(ast);
    expect(tokens.some((t) => t.type === "LOCAL_TIME")).toBe(true);
  });

  it("parses a negative integer value", () => {
    const ast = parseTOML("offset = -42");
    const tokens = meaningfulTokens(ast);
    expect(tokens.some((t) => t.type === "INTEGER" && t.value === "-42")).toBe(true);
  });

  it("parses special float values", () => {
    const ast = parseTOML("x = inf\ny = nan");
    const tokens = meaningfulTokens(ast);
    expect(tokens.some((t) => t.type === "FLOAT" && t.value === "inf")).toBe(true);
    expect(tokens.some((t) => t.type === "FLOAT" && t.value === "nan")).toBe(true);
  });
});

// =========================================================================
// Table Headers
// =========================================================================

describe("table headers", () => {
  it("parses a simple table header", () => {
    /**
     * A table header switches the current table:
     *   [server]
     *   host = "localhost"
     */
    const ast = parseTOML('[server]\nhost = "localhost"');
    expect(ast.ruleName).toBe("document");

    const tableHeaders = findNodes(ast, "table_header");
    expect(tableHeaders).toHaveLength(1);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);
  });

  it("parses a dotted table header", () => {
    /**
     * Dotted table headers create nested tables:
     *   [a.b.c]
     */
    const ast = parseTOML("[a.b.c]\nkey = 1");
    const tableHeaders = findNodes(ast, "table_header");
    expect(tableHeaders).toHaveLength(1);

    const tokens = meaningfulTokens(tableHeaders[0]);
    const dots = tokens.filter((t) => t.type === "DOT");
    expect(dots).toHaveLength(2);
  });

  it("parses multiple table headers", () => {
    /**
     * A document can have multiple table headers:
     *   [server]
     *   host = "localhost"
     *
     *   [database]
     *   port = 5432
     */
    const source = '[server]\nhost = "localhost"\n\n[database]\nport = 5432';
    const ast = parseTOML(source);

    const tableHeaders = findNodes(ast, "table_header");
    expect(tableHeaders).toHaveLength(2);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(2);
  });
});

describe("array-of-tables headers", () => {
  it("parses an array-of-tables header", () => {
    /**
     * Array-of-tables headers use double brackets:
     *   [[products]]
     *   name = "Hammer"
     *
     * Each [[products]] creates a new element in the "products" array.
     */
    const ast = parseTOML('[[products]]\nname = "Hammer"');

    const arrayTableHeaders = findNodes(ast, "array_table_header");
    expect(arrayTableHeaders).toHaveLength(1);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(1);
  });

  it("parses multiple array-of-tables headers", () => {
    const source = '[[products]]\nname = "Hammer"\n\n[[products]]\nname = "Nail"';
    const ast = parseTOML(source);

    const arrayTableHeaders = findNodes(ast, "array_table_header");
    expect(arrayTableHeaders).toHaveLength(2);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(2);
  });
});

// =========================================================================
// Arrays
// =========================================================================

describe("arrays", () => {
  it("parses an inline array of integers", () => {
    const ast = parseTOML("numbers = [1, 2, 3]");
    const arrays = findNodes(ast, "array");
    expect(arrays).toHaveLength(1);

    const tokens = meaningfulTokens(arrays[0]);
    const ints = tokens.filter((t) => t.type === "INTEGER");
    expect(ints).toHaveLength(3);
  });

  it("parses an inline array of strings", () => {
    const ast = parseTOML('colors = ["red", "green", "blue"]');
    const arrays = findNodes(ast, "array");
    expect(arrays).toHaveLength(1);

    const tokens = meaningfulTokens(arrays[0]);
    const strings = tokens.filter((t) => t.type === "BASIC_STRING");
    expect(strings).toHaveLength(3);
  });

  it("parses an empty array", () => {
    const ast = parseTOML("empty = []");
    const arrays = findNodes(ast, "array");
    expect(arrays).toHaveLength(1);
  });

  it("parses a multi-line array", () => {
    /**
     * TOML arrays can span multiple lines. The grammar allows NEWLINE
     * tokens between elements:
     *   colors = [
     *     "red",
     *     "green",
     *     "blue",
     *   ]
     */
    const source = 'colors = [\n"red",\n"green",\n"blue",\n]';
    const ast = parseTOML(source);
    const arrays = findNodes(ast, "array");
    expect(arrays).toHaveLength(1);

    const tokens = meaningfulTokens(arrays[0]);
    const strings = tokens.filter((t) => t.type === "BASIC_STRING");
    expect(strings).toHaveLength(3);
  });

  it("parses a nested array", () => {
    const ast = parseTOML("matrix = [[1, 2], [3, 4]]");
    const arrays = findNodes(ast, "array");
    expect(arrays).toHaveLength(3); // outer + 2 inner
  });
});

// =========================================================================
// Inline Tables
// =========================================================================

describe("inline tables", () => {
  it("parses a simple inline table", () => {
    /**
     * Inline tables are compact, single-line table definitions:
     *   point = { x = 1, y = 2 }
     */
    const ast = parseTOML("point = { x = 1, y = 2 }");
    const inlineTables = findNodes(ast, "inline_table");
    expect(inlineTables).toHaveLength(1);

    const keyvals = findNodes(inlineTables[0], "keyval");
    expect(keyvals).toHaveLength(2);
  });

  it("parses an empty inline table", () => {
    const ast = parseTOML("empty = {}");
    const inlineTables = findNodes(ast, "inline_table");
    expect(inlineTables).toHaveLength(1);
  });

  it("parses a single-pair inline table", () => {
    const ast = parseTOML("single = { key = 1 }");
    const inlineTables = findNodes(ast, "inline_table");
    expect(inlineTables).toHaveLength(1);

    const keyvals = findNodes(inlineTables[0], "keyval");
    expect(keyvals).toHaveLength(1);
  });
});

// =========================================================================
// Complete Documents
// =========================================================================

describe("complete documents", () => {
  it("parses a realistic TOML configuration", () => {
    /**
     * A realistic TOML document with multiple tables, arrays, and
     * different value types -- similar to a Cargo.toml or pyproject.toml.
     */
    const source = [
      'title = "My App"',
      "",
      "[server]",
      'host = "localhost"',
      "port = 8080",
      "debug = false",
      "",
      "[database]",
      'url = "postgres://localhost/mydb"',
      "pool_size = 5",
    ].join("\n");

    const ast = parseTOML(source);
    expect(ast.ruleName).toBe("document");

    const tableHeaders = findNodes(ast, "table_header");
    expect(tableHeaders).toHaveLength(2);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(6); // title + 3 server + 2 database
  });

  it("parses a document with array values", () => {
    const source = [
      "[package]",
      'name = "my-app"',
      'keywords = ["cli", "tool"]',
    ].join("\n");

    const ast = parseTOML(source);
    const arrays = findNodes(ast, "array");
    expect(arrays).toHaveLength(1);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(2);
  });

  it("parses a document with inline table values", () => {
    const source = [
      "[dependencies]",
      'serde = { version = "1.0", features = ["derive"] }',
    ].join("\n");

    const ast = parseTOML(source);
    const inlineTables = findNodes(ast, "inline_table");
    expect(inlineTables).toHaveLength(1);
  });
});

// =========================================================================
// Edge Cases
// =========================================================================

describe("edge cases", () => {
  it("parses an empty document", () => {
    /**
     * An empty TOML document is valid. The "document" rule matches
     * zero or more expressions, so an empty string produces a document
     * node with no expression children.
     */
    const ast = parseTOML("");
    expect(ast.ruleName).toBe("document");
  });

  it("parses a document with only blank lines", () => {
    const ast = parseTOML("\n\n\n");
    expect(ast.ruleName).toBe("document");
  });

  it("parses a document with only comments", () => {
    /**
     * Comments are skipped by the lexer, so a document with only
     * comments is equivalent to an empty document.
     */
    const ast = parseTOML("# This is a comment\n# Another comment");
    expect(ast.ruleName).toBe("document");
  });

  it("parses multiple key-value pairs", () => {
    const source = "a = 1\nb = 2\nc = 3";
    const ast = parseTOML(source);

    const keyvals = findNodes(ast, "keyval");
    expect(keyvals).toHaveLength(3);
  });
});

// =========================================================================
// Error Cases
// =========================================================================

describe("error cases", () => {
  it("rejects a key without a value", () => {
    /**
     * A bare key without an equals sign and value is not valid TOML.
     * The parser should throw because it expects key = value.
     */
    expect(() => parseTOML("key_without_value")).toThrow();
  });

  it("rejects a value without a key", () => {
    /**
     * A bare value (not preceded by key =) is not valid at the top level.
     * Only key-value pairs and table headers are valid expressions.
     */
    expect(() => parseTOML('"just a string"')).toThrow();
  });
});
