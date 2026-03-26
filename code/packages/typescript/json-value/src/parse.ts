/**
 * Convenience Functions -- text to JsonValue / native types
 *
 * These functions compose the full pipeline:
 *
 *     JSON text  -->  json-lexer  -->  json-parser  -->  AST  -->  JsonValue
 *                                                                      |
 *                                                              toNative() --> native
 *
 * Why Convenience Functions?
 * --------------------------
 *
 * Without them, parsing JSON requires four imports and three function calls:
 *
 *     import { parseJSON } from "@coding-adventures/json-parser";
 *     import { fromAST, toNative } from "coding-adventures-json-value";
 *
 *     const ast = parseJSON(text);
 *     const value = fromAST(ast);
 *     const native = toNative(value);
 *
 * With parse() and parseNative(), it's one import and one call:
 *
 *     import { parseNative } from "coding-adventures-json-value";
 *     const native = parseNative(text);
 *
 * This follows the "make the common case easy" principle.
 *
 * @module
 */

import { parseJSON } from "@coding-adventures/json-parser";
import type { JsonValue } from "./value.js";
import { fromAST } from "./from-ast.js";
import { toNative } from "./to-native.js";
import { JsonValueError } from "./errors.js";

/**
 * Parse JSON text into a typed JsonValue.
 *
 * Internally calls: parseJSON(text) --> AST --> fromAST(ast) --> JsonValue
 *
 * @param text - Valid JSON text.
 * @returns A JsonValue representing the parsed data.
 * @throws JsonValueError if the text is not valid JSON.
 *
 * @example
 *     const value = parse('{"name": "Alice"}');
 *     // value.type === 'object'
 *     // value.pairs.get('name')?.type === 'string'
 *
 * @example
 *     const arr = parse('[1, 2, 3]');
 *     // arr.type === 'array'
 *     // arr.elements.length === 3
 */
export function parse(text: string): JsonValue {
  try {
    const ast = parseJSON(text);
    return fromAST(ast);
  } catch (error) {
    /**
     * Wrap parser errors in JsonValueError for a consistent error type.
     * The original error message is preserved for debugging.
     */
    if (error instanceof JsonValueError) {
      throw error;
    }
    const message =
      error instanceof Error ? error.message : String(error);
    throw new JsonValueError(`Failed to parse JSON: ${message}`);
  }
}

/**
 * Parse JSON text directly into native JavaScript types.
 *
 * This is the most common use case: "give me a plain object from this JSON string."
 *
 * Equivalent to: toNative(parse(text))
 *
 * @param text - Valid JSON text.
 * @returns A plain JavaScript value (object, array, string, number, boolean, or null).
 * @throws JsonValueError if the text is not valid JSON.
 *
 * @example
 *     const data = parseNative('{"name": "Alice", "age": 30}');
 *     // { name: "Alice", age: 30 }
 *
 * @example
 *     const nums = parseNative('[1, 2, 3]');
 *     // [1, 2, 3]
 */
export function parseNative(text: string): unknown {
  return toNative(parse(text));
}
