/**
 * Tests for the JSON Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses JSON
 * text when loaded with the `json.grammar` file.
 *
 * The JSON grammar's top-level rule is `value` -- any valid JSON text is a
 * single value (an object, array, string, number, boolean, or null).
 *
 * Test Strategy
 * -------------
 *
 * Each test parses a JSON string and then uses helper functions to walk the
 * resulting AST, looking for specific node types. This approach is more robust
 * than checking exact tree structure, because the grammar may wrap nodes in
 * multiple layers of rules.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Primitive values** -- strings, numbers, true, false, null
 *   2. **Objects** -- empty, single pair, multiple pairs
 *   3. **Arrays** -- empty, flat, mixed types
 *   4. **Nested structures** -- objects in arrays, arrays in objects, deep nesting
 *   5. **Error cases** -- invalid JSON that should cause parse errors
 */

import { describe, it, expect } from "vitest";
import { parseJSON } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This is the workhorse helper for these tests. Since the grammar wraps
 * values in the top-level "value" rule, we need to search the entire tree
 * to find the nodes we care about.
 *
 * @param node - The root node to search from.
 * @param ruleName - The grammar rule name to find (e.g., "object", "pair").
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

describe("primitive values", () => {
  it("parses a string value", () => {
    /**
     * A bare string is the simplest JSON value. The top-level rule
     * should be "value" and the tree should contain a STRING token.
     */
    const ast = parseJSON('"hello"');
    expect(ast.ruleName).toBe("value");

    const tokens = findTokens(ast);
    const stringTokens = tokens.filter((t) => t.type === "STRING");
    expect(stringTokens).toHaveLength(1);
  });

  it("parses a number value", () => {
    /**
     * A bare number is a valid JSON value. The parser wraps it in a
     * "value" node containing a NUMBER token.
     */
    const ast = parseJSON("42");
    expect(ast.ruleName).toBe("value");

    const tokens = findTokens(ast);
    const numberTokens = tokens.filter((t) => t.type === "NUMBER");
    expect(numberTokens).toHaveLength(1);
  });

  it("parses a negative number", () => {
    /**
     * Negative numbers are a single NUMBER token in JSON (the minus
     * is part of the token, not a separate operator).
     */
    const ast = parseJSON("-3.14");
    const tokens = findTokens(ast);
    const numberTokens = tokens.filter((t) => t.type === "NUMBER");
    expect(numberTokens).toHaveLength(1);
    expect(numberTokens[0].value).toBe("-3.14");
  });

  it("parses true", () => {
    /**
     * The boolean literal `true` is a valid JSON value on its own.
     */
    const ast = parseJSON("true");
    expect(ast.ruleName).toBe("value");

    const tokens = findTokens(ast);
    const trueTokens = tokens.filter((t) => t.type === "TRUE");
    expect(trueTokens).toHaveLength(1);
  });

  it("parses false", () => {
    /**
     * The boolean literal `false` is a valid JSON value on its own.
     */
    const ast = parseJSON("false");
    const tokens = findTokens(ast);
    const falseTokens = tokens.filter((t) => t.type === "FALSE");
    expect(falseTokens).toHaveLength(1);
  });

  it("parses null", () => {
    /**
     * The null literal is a valid JSON value on its own.
     */
    const ast = parseJSON("null");
    const tokens = findTokens(ast);
    const nullTokens = tokens.filter((t) => t.type === "NULL");
    expect(nullTokens).toHaveLength(1);
  });
});

describe("objects", () => {
  it("parses an empty object", () => {
    /**
     * The simplest JSON object: {}
     * The parser should produce a "value" node wrapping an "object" node.
     */
    const ast = parseJSON("{}");
    expect(ast.ruleName).toBe("value");

    const objectNodes = findNodes(ast, "object");
    expect(objectNodes).toHaveLength(1);
  });

  it("parses an object with one pair", () => {
    /**
     * An object with a single key-value pair:
     *   {"name": "Alice"}
     *
     * The "object" node should contain one "pair" node.
     */
    const ast = parseJSON('{"name": "Alice"}');

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(1);

    // The pair should contain a STRING key and a STRING value
    const tokens = findTokens(pairNodes[0]);
    const stringTokens = tokens.filter((t) => t.type === "STRING");
    expect(stringTokens).toHaveLength(2);
  });

  it("parses an object with multiple pairs", () => {
    /**
     * An object with multiple key-value pairs separated by commas:
     *   {"name": "Alice", "age": 30, "active": true}
     */
    const ast = parseJSON('{"name": "Alice", "age": 30, "active": true}');

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(3);
  });

  it("parses an object with different value types", () => {
    /**
     * JSON objects can have values of any type: string, number, boolean,
     * null, array, or nested object.
     */
    const source = '{"s": "text", "n": 42, "b": true, "x": null}';
    const ast = parseJSON(source);

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(4);

    // Verify diverse token types across all pairs
    const allTokens = findTokens(ast);
    expect(allTokens.some((t) => t.type === "STRING")).toBe(true);
    expect(allTokens.some((t) => t.type === "NUMBER")).toBe(true);
    expect(allTokens.some((t) => t.type === "TRUE")).toBe(true);
    expect(allTokens.some((t) => t.type === "NULL")).toBe(true);
  });
});

describe("arrays", () => {
  it("parses an empty array", () => {
    /**
     * The simplest JSON array: []
     * The parser should produce a "value" node wrapping an "array" node.
     */
    const ast = parseJSON("[]");
    expect(ast.ruleName).toBe("value");

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(1);
  });

  it("parses an array of numbers", () => {
    /**
     * A flat array of numbers: [1, 2, 3]
     * The array should contain three value nodes, each wrapping a NUMBER.
     */
    const ast = parseJSON("[1, 2, 3]");

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(1);

    const tokens = findTokens(arrayNodes[0]);
    const numberTokens = tokens.filter((t) => t.type === "NUMBER");
    expect(numberTokens).toHaveLength(3);
  });

  it("parses an array of mixed types", () => {
    /**
     * JSON arrays can contain elements of different types:
     *   [1, "two", true, null, false]
     */
    const ast = parseJSON('[1, "two", true, null, false]');

    const allTokens = findTokens(ast);
    expect(allTokens.some((t) => t.type === "NUMBER")).toBe(true);
    expect(allTokens.some((t) => t.type === "STRING")).toBe(true);
    expect(allTokens.some((t) => t.type === "TRUE")).toBe(true);
    expect(allTokens.some((t) => t.type === "NULL")).toBe(true);
    expect(allTokens.some((t) => t.type === "FALSE")).toBe(true);
  });

  it("parses a single-element array", () => {
    /**
     * An array with just one element: [42]
     */
    const ast = parseJSON("[42]");

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(1);

    const tokens = findTokens(arrayNodes[0]);
    const numberTokens = tokens.filter((t) => t.type === "NUMBER");
    expect(numberTokens).toHaveLength(1);
  });
});

describe("nested structures", () => {
  it("parses nested objects", () => {
    /**
     * Objects can contain other objects as values:
     *   {"outer": {"inner": 1}}
     *
     * This tests the recursive nature of the grammar: value -> object ->
     * pair -> value -> object -> ...
     */
    const ast = parseJSON('{"outer": {"inner": 1}}');

    const objectNodes = findNodes(ast, "object");
    expect(objectNodes).toHaveLength(2);
  });

  it("parses nested arrays", () => {
    /**
     * Arrays can contain other arrays:
     *   [[1, 2], [3, 4]]
     *
     * This tests: value -> array -> value -> array -> ...
     */
    const ast = parseJSON("[[1, 2], [3, 4]]");

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(3); // outer + 2 inner
  });

  it("parses an array of objects", () => {
    /**
     * A common pattern in APIs: an array of objects.
     *   [{"id": 1}, {"id": 2}]
     */
    const ast = parseJSON('[{"id": 1}, {"id": 2}]');

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(1);

    const objectNodes = findNodes(ast, "object");
    expect(objectNodes).toHaveLength(2);

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(2);
  });

  it("parses an object with array values", () => {
    /**
     * Objects can have arrays as values:
     *   {"items": [1, 2, 3], "tags": ["a", "b"]}
     */
    const ast = parseJSON('{"items": [1, 2, 3], "tags": ["a", "b"]}');

    const objectNodes = findNodes(ast, "object");
    expect(objectNodes).toHaveLength(1);

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(2);
  });

  it("parses deeply nested structures", () => {
    /**
     * JSON allows arbitrarily deep nesting. This tests three levels:
     *   {"a": {"b": {"c": 1}}}
     */
    const ast = parseJSON('{"a": {"b": {"c": 1}}}');

    const objectNodes = findNodes(ast, "object");
    expect(objectNodes).toHaveLength(3);

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(3);
  });

  it("parses a realistic API response", () => {
    /**
     * A realistic JSON structure mixing objects, arrays, strings,
     * numbers, booleans, and null:
     */
    const source = '{"users": [{"name": "Alice", "age": 30, "active": true}, {"name": "Bob", "age": null, "active": false}]}';
    const ast = parseJSON(source);

    expect(ast.ruleName).toBe("value");

    const objectNodes = findNodes(ast, "object");
    expect(objectNodes).toHaveLength(3); // outer + 2 user objects

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(1); // the users array

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(7); // users + 3 per user object
  });
});

describe("whitespace tolerance", () => {
  it("parses compact JSON (no whitespace)", () => {
    /**
     * JSON with no whitespace between tokens is perfectly valid.
     * The parser should handle it identically to spaced-out JSON.
     */
    const ast = parseJSON('{"a":1,"b":2}');

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(2);
  });

  it("parses multi-line JSON with indentation", () => {
    /**
     * Pretty-printed JSON with newlines and indentation. The lexer
     * skips all whitespace, so the parser sees the same token stream
     * as compact JSON.
     */
    const source = `{
  "name": "Alice",
  "scores": [
    100,
    95,
    87
  ]
}`;
    const ast = parseJSON(source);

    const pairNodes = findNodes(ast, "pair");
    expect(pairNodes).toHaveLength(2);

    const arrayNodes = findNodes(ast, "array");
    expect(arrayNodes).toHaveLength(1);
  });
});

describe("error cases", () => {
  it("rejects an empty string", () => {
    /**
     * An empty string is not valid JSON. The parser should throw
     * because it expects at least one value token.
     */
    expect(() => parseJSON("")).toThrow();
  });

  it("rejects a trailing comma in an object", () => {
    /**
     * Trailing commas are NOT allowed in JSON (unlike JavaScript).
     * {"a": 1,} should cause a parse error.
     */
    expect(() => parseJSON('{"a": 1,}')).toThrow();
  });

  it("rejects a trailing comma in an array", () => {
    /**
     * Trailing commas in arrays are also not allowed:
     * [1, 2,] should cause a parse error.
     */
    expect(() => parseJSON("[1, 2,]")).toThrow();
  });
});
