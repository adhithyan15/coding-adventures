/**
 * AST to JsonValue Conversion
 *
 * This module implements the core tree walk that converts a generic ASTNode
 * tree (produced by json-parser) into a typed JsonValue tree.
 *
 * The Algorithm
 * -------------
 *
 * The json-parser produces a tree where:
 *   - Interior nodes are ASTNode objects with a `ruleName` and `children`
 *   - Leaf nodes are Token objects with a `type` and `value`
 *
 * The tree walk dispatches on the node type:
 *
 *     ASTNode("value")  --> unwrap: find the meaningful child and recurse
 *     ASTNode("object") --> collect pairs into a Map
 *     ASTNode("pair")   --> extract key (STRING token) and value (recurse)
 *     ASTNode("array")  --> collect elements into an array
 *     Token("STRING")   --> JsonString
 *     Token("NUMBER")   --> JsonNumber (integer if no decimal/exponent)
 *     Token("TRUE")     --> JsonBoolean(true)
 *     Token("FALSE")    --> JsonBoolean(false)
 *     Token("NULL")     --> JsonNull
 *
 * Example Walk
 * ------------
 *
 * For the JSON text: `{"name": "Alice"}`
 *
 *     ASTNode("value")                        <-- fromAST starts here
 *       ASTNode("object")                     <-- recurse into object
 *         Token(LBRACE, "{")                  <-- skip (structural)
 *         ASTNode("pair")                     <-- process pair
 *           Token(STRING, '"name"')           <-- key = "name"
 *           Token(COLON, ":")                 <-- skip (structural)
 *           ASTNode("value")                  <-- recurse for value
 *             Token(STRING, '"Alice"')        <-- JsonString("Alice")
 *         Token(RBRACE, "}")                  <-- skip (structural)
 *
 *     Result: { type: 'object', pairs: Map { 'name' => { type: 'string', value: 'Alice' } } }
 *
 * @module
 */

import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import { isASTNode } from "@coding-adventures/parser";

import type { JsonValue } from "./value.js";
import {
  jsonObject,
  jsonArray,
  jsonString,
  jsonNumber,
  jsonBool,
  jsonNull,
} from "./value.js";
import { JsonValueError } from "./errors.js";

// =============================================================================
// TOKEN TYPE CONSTANTS
// =============================================================================
//
// These match the token types emitted by the json-lexer.
// Using constants prevents typos and makes it easy to find all references.

/** Token types that carry JSON values (as opposed to structural punctuation). */
const VALUE_TOKEN_TYPES = new Set([
  "STRING",
  "NUMBER",
  "TRUE",
  "FALSE",
  "NULL",
]);

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Convert a json-parser AST node into a typed JsonValue.
 *
 * This is the entry point for AST-to-value conversion. It handles both
 * ASTNode (interior nodes with rule names) and Token (leaf nodes with types).
 *
 * @param node - The root ASTNode produced by parseJSON(). Typically has
 *               ruleName "value".
 * @returns A JsonValue representing the parsed JSON data.
 * @throws JsonValueError if the AST contains unexpected structure.
 *
 * @example
 *     import { parseJSON } from "@coding-adventures/json-parser";
 *     import { fromAST } from "coding-adventures-json-value";
 *
 *     const ast = parseJSON('{"name": "Alice"}');
 *     const value = fromAST(ast);
 *     // value.type === 'object'
 *     // value.pairs.get('name')?.type === 'string'
 */
export function fromAST(node: ASTNode): JsonValue {
  return convertNode(node);
}

// =============================================================================
// INTERNAL CONVERSION FUNCTIONS
// =============================================================================

/**
 * Convert an ASTNode to a JsonValue by dispatching on its rule name.
 *
 * The json.grammar defines four rules:
 *   - "value"  -- wraps exactly one meaningful child
 *   - "object" -- LBRACE [ pair { COMMA pair } ] RBRACE
 *   - "pair"   -- STRING COLON value
 *   - "array"  -- LBRACKET [ value { COMMA value } ] RBRACKET
 */
function convertNode(node: ASTNode): JsonValue {
  switch (node.ruleName) {
    case "value":
      return convertValue(node);
    case "object":
      return convertObject(node);
    case "array":
      return convertArray(node);
    default:
      throw new JsonValueError(
        `Unexpected AST rule name: "${node.ruleName}". ` +
          `Expected "value", "object", or "array".`
      );
  }
}

/**
 * Convert a "value" AST node.
 *
 * The "value" rule wraps exactly one meaningful child, which is either:
 *   a) An ASTNode with ruleName "object" or "array"
 *   b) A Token with type STRING, NUMBER, TRUE, FALSE, or NULL
 *
 * We skip structural tokens (LBRACE, RBRACE, COMMA, COLON) because they
 * carry no semantic meaning -- they're just syntax.
 */
function convertValue(node: ASTNode): JsonValue {
  for (const child of node.children) {
    if (isASTNode(child)) {
      /**
       * The child is an interior node -- it must be "object" or "array".
       * Recurse into it.
       */
      return convertNode(child as ASTNode);
    } else {
      /**
       * The child is a Token. Check if it's a value-carrying token.
       * Structural tokens (LBRACE, COMMA, etc.) are ignored.
       */
      const token = child as Token;
      if (VALUE_TOKEN_TYPES.has(token.type)) {
        return convertToken(token);
      }
    }
  }

  throw new JsonValueError(
    'No meaningful child found in "value" AST node. ' +
      "Expected an object, array, string, number, boolean, or null."
  );
}

/**
 * Convert a Token to a JsonValue.
 *
 * Token Type    JSON Type    Example Token Value    JsonValue Result
 * ----------    ---------    -------------------    ----------------
 * STRING        string       "hello"                jsonString("hello")
 * NUMBER        number       42                     jsonNumber(42, true)
 * NUMBER        number       3.14                   jsonNumber(3.14, false)
 * TRUE          boolean      true                   jsonBool(true)
 * FALSE         boolean      false                  jsonBool(false)
 * NULL          null         null                   jsonNull()
 *
 * Note on STRING values: The lexer may or may not strip quotes. The json-lexer
 * in this project stores the raw token value with quotes. We strip them here.
 */
function convertToken(token: Token): JsonValue {
  switch (token.type) {
    case "STRING":
      return jsonString(unquoteString(token.value));

    case "NUMBER":
      return convertNumber(token.value);

    case "TRUE":
      return jsonBool(true);

    case "FALSE":
      return jsonBool(false);

    case "NULL":
      return jsonNull();

    default:
      throw new JsonValueError(
        `Unexpected token type "${token.type}" in value position. ` +
          `Expected STRING, NUMBER, TRUE, FALSE, or NULL.`
      );
  }
}

/**
 * Extract the string value from a STRING token.
 *
 * The grammar-driven lexer in this project uses `escapes: none` mode,
 * which means it:
 *   1. Strips the surrounding double quotes from string tokens
 *   2. Leaves escape sequences as raw text (e.g., \n stays as backslash + n)
 *
 * This function strips any remaining quotes, then processes the JSON escape
 * sequences defined by RFC 8259 section 7:
 *
 *     JSON source: "hello"     --> token value: hello     --> result: hello
 *     JSON source: "a\nb"     --> token value: a\nb      --> result: a<newline>b
 *     JSON source: "a\\b"     --> token value: a\\b      --> result: a\b
 *     JSON source: "\u0041"   --> token value: \u0041    --> result: A
 *
 * If the quotes are still present (e.g., from a different lexer configuration),
 * we strip them before processing escapes.
 */
function unquoteString(raw: string): string {
  /**
   * Check if the string still has surrounding quotes.
   * The grammar-driven lexer strips them, but we handle both cases
   * for robustness.
   */
  let content = raw;
  if (content.length >= 2 && content[0] === '"' && content[content.length - 1] === '"') {
    content = content.slice(1, -1);
  }

  return processJsonEscapes(content);
}

/**
 * Process JSON string escape sequences according to RFC 8259 section 7.
 *
 * The JSON lexer uses `escapes: none` mode, which means it strips the
 * surrounding quotes but leaves escape sequences as raw text. This
 * function converts those raw escape sequences into their actual
 * character values:
 *
 * | Escape   | Character              |
 * |----------|------------------------|
 * | `\\`     | reverse solidus U+005C |
 * | `\/`     | solidus U+002F         |
 * | `\"`     | quotation mark U+0022  |
 * | `\b`     | backspace U+0008       |
 * | `\f`     | form feed U+000C       |
 * | `\n`     | line feed U+000A       |
 * | `\r`     | carriage return U+000D |
 * | `\t`     | tab U+0009             |
 * | `\uXXXX` | Unicode code point     |
 *
 * The algorithm walks the string character by character. When it sees a
 * backslash, it looks at the next character to determine which escape
 * sequence to decode. Non-escape characters pass through unchanged.
 */
function processJsonEscapes(s: string): string {
  /**
   * Fast path: if there are no backslashes, there's nothing to process.
   * This avoids allocating a new string for the common case.
   */
  if (!s.includes("\\")) {
    return s;
  }

  const result: string[] = [];
  let i = 0;

  while (i < s.length) {
    if (s[i] === "\\" && i + 1 < s.length) {
      const next = s[i + 1];
      switch (next) {
        case '"':
          result.push('"');
          i += 2;
          break;
        case "\\":
          result.push("\\");
          i += 2;
          break;
        case "/":
          result.push("/");
          i += 2;
          break;
        case "b":
          result.push("\b");
          i += 2;
          break;
        case "f":
          result.push("\f");
          i += 2;
          break;
        case "n":
          result.push("\n");
          i += 2;
          break;
        case "r":
          result.push("\r");
          i += 2;
          break;
        case "t":
          result.push("\t");
          i += 2;
          break;
        case "u": {
          /**
           * Unicode escape: \uXXXX where XXXX is exactly 4 hex digits.
           * We parse the 4-digit hex value and convert to a character.
           * If there aren't enough digits, we emit the raw text.
           */
          if (i + 5 < s.length) {
            const hex = s.substring(i + 2, i + 6);
            const codePoint = parseInt(hex, 16);
            if (!isNaN(codePoint)) {
              result.push(String.fromCharCode(codePoint));
              i += 6;
              break;
            }
          }
          // Malformed \u escape — emit as-is
          result.push(next);
          i += 2;
          break;
        }
        default:
          /**
           * Unknown escape — emit the character after the backslash.
           * This matches the behavior of most JSON parsers for
           * unrecognized escape sequences.
           */
          result.push(next);
          i += 2;
          break;
      }
    } else {
      result.push(s[i]);
      i += 1;
    }
  }

  return result.join("");
}

/**
 * Convert a NUMBER token value to a JsonNumber.
 *
 * The key decision: is this number an integer or a float?
 *
 *     Token Value    Has '.' or 'e'/'E'?    Result
 *     -----------    -------------------    ------
 *     "42"           No                     jsonNumber(42, true)
 *     "-17"          No                     jsonNumber(-17, true)
 *     "0"            No                     jsonNumber(0, true)
 *     "3.14"         Yes (.)                jsonNumber(3.14, false)
 *     "1e10"         Yes (e)                jsonNumber(1e10, false)
 *     "2.5E-3"       Yes (. and E)          jsonNumber(0.0025, false)
 *
 * We check the raw string representation (not the parsed value) because
 * `1e10` parses to `10000000000` which Number.isInteger() says is an integer,
 * but the JSON source used scientific notation, indicating a float.
 */
function convertNumber(raw: string): JsonValue {
  const value = Number(raw);

  /**
   * Check the raw string for decimal points or exponent markers.
   * This preserves the original intent: `42` is integer, `42.0` is float.
   */
  const isInteger = !raw.includes(".") && !raw.includes("e") && !raw.includes("E");

  return jsonNumber(value, isInteger);
}

/**
 * Convert an "object" AST node to a JsonObject.
 *
 * Object structure in the AST:
 *
 *     ASTNode("object")
 *       Token(LBRACE, "{")       <-- skip
 *       ASTNode("pair")          <-- process: extract key + value
 *         Token(STRING, ...)     <-- key
 *         Token(COLON, ":")      <-- skip
 *         ASTNode("value")       <-- recurse for value
 *       Token(COMMA, ",")        <-- skip (if multi-pair)
 *       ASTNode("pair")          <-- process next pair
 *         ...
 *       Token(RBRACE, "}")       <-- skip
 *
 * We iterate over children, process only ASTNode("pair") children,
 * and skip all tokens (structural punctuation).
 */
function convertObject(node: ASTNode): JsonValue {
  const pairs = new Map<string, JsonValue>();

  for (const child of node.children) {
    if (isASTNode(child)) {
      const astChild = child as ASTNode;
      if (astChild.ruleName === "pair") {
        const [key, value] = convertPair(astChild);
        pairs.set(key, value);
      }
    }
  }

  return jsonObject(pairs);
}

/**
 * Convert a "pair" AST node to a [key, value] tuple.
 *
 * Pair structure:
 *
 *     ASTNode("pair")
 *       Token(STRING, '"name"')    <-- key (always first STRING token)
 *       Token(COLON, ":")          <-- skip
 *       ASTNode("value")           <-- recurse for value
 *
 * We find the STRING token for the key and the "value" ASTNode for the value.
 */
function convertPair(node: ASTNode): [string, JsonValue] {
  let key: string | null = null;
  let value: JsonValue | null = null;

  for (const child of node.children) {
    if (isASTNode(child)) {
      const astChild = child as ASTNode;
      if (astChild.ruleName === "value") {
        value = convertValue(astChild);
      }
    } else {
      const token = child as Token;
      if (token.type === "STRING" && key === null) {
        key = unquoteString(token.value);
      }
    }
  }

  if (key === null) {
    throw new JsonValueError(
      'No STRING token found in "pair" AST node for the key.'
    );
  }

  if (value === null) {
    throw new JsonValueError(
      `No value found in "pair" AST node for key "${key}".`
    );
  }

  return [key, value];
}

/**
 * Convert an "array" AST node to a JsonArray.
 *
 * Array structure in the AST:
 *
 *     ASTNode("array")
 *       Token(LBRACKET, "[")       <-- skip
 *       ASTNode("value")           <-- recurse for element
 *       Token(COMMA, ",")          <-- skip
 *       ASTNode("value")           <-- recurse for element
 *       Token(RBRACKET, "]")       <-- skip
 *
 * We iterate over children, recurse into ASTNode("value") children,
 * and skip all tokens.
 *
 * Edge case: array elements might appear as direct Token children rather
 * than wrapped in ASTNode("value"). We handle both cases.
 */
function convertArray(node: ASTNode): JsonValue {
  const elements: JsonValue[] = [];

  for (const child of node.children) {
    if (isASTNode(child)) {
      const astChild = child as ASTNode;
      if (astChild.ruleName === "value") {
        elements.push(convertValue(astChild));
      } else {
        /**
         * Handle unexpected but possible AST shapes -- if a child is an
         * ASTNode with a different rule name, try converting it anyway.
         */
        elements.push(convertNode(astChild));
      }
    } else {
      /**
       * Handle the edge case where array elements are direct Token children
       * rather than wrapped in ASTNode("value"). This can happen with some
       * grammar configurations.
       */
      const token = child as Token;
      if (VALUE_TOKEN_TYPES.has(token.type)) {
        elements.push(convertToken(token));
      }
      // Skip structural tokens: LBRACKET, RBRACKET, COMMA
    }
  }

  return jsonArray(elements);
}
