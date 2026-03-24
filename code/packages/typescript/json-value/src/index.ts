/**
 * JSON Value -- typed intermediate representation for JSON data.
 *
 * This package sits between the json-parser (which produces generic AST nodes)
 * and application code (which wants typed, meaningful data). It answers the
 * question: "What does this JSON *mean*?"
 *
 * The Pipeline
 * ------------
 *
 * JSON text flows through four stages:
 *
 *     '{"name": "Alice", "age": 30}'
 *          |
 *          v
 *     json-lexer  -->  Token stream
 *          |
 *          v
 *     json-parser -->  ASTNode tree (generic, untyped)
 *          |
 *          v
 *     json-value  -->  JsonValue tree (THIS PACKAGE)
 *          |
 *          v
 *     Application code can use JsonValue directly (type-safe access,
 *     pattern matching) or convert to native JS types via toNative().
 *
 * Why JsonValue Instead of Plain Objects?
 * ---------------------------------------
 *
 * JavaScript already has JSON.parse(). So why bother with a typed intermediate?
 *
 * 1. **Type discrimination.** A plain `number` in JS doesn't tell you whether
 *    the original JSON had `42` (integer) or `42.0` (float). JsonValue preserves
 *    this distinction via the `isInteger` flag on number values.
 *
 * 2. **Pattern matching.** The discriminated union with a `type` field lets
 *    TypeScript narrow types in switch statements:
 *
 *        switch (value.type) {
 *          case 'string': console.log(value.value.toUpperCase()); break;
 *          case 'number': console.log(value.value * 2); break;
 *        }
 *
 * 3. **Round-trip fidelity.** Converting to native and back loses information
 *    (integer vs float, key order in objects). JsonValue preserves everything.
 *
 * 4. **Zero dependencies beyond our own stack.** We built the lexer, parser,
 *    and now the value layer -- learning all the way.
 *
 * The JsonValue Type
 * ------------------
 *
 * JSON has exactly six value types. Our discriminated union mirrors them:
 *
 *     JsonValue
 *       |-- { type: 'object',  pairs: Map<string, JsonValue> }
 *       |-- { type: 'array',   elements: JsonValue[] }
 *       |-- { type: 'string',  value: string }
 *       |-- { type: 'number',  value: number, isInteger: boolean }
 *       |-- { type: 'boolean', value: boolean }
 *       |-- { type: 'null' }
 *
 * The `type` field is the discriminant -- TypeScript uses it to narrow the
 * union in conditionals and switch statements.
 *
 * Why Map for Objects?
 * --------------------
 *
 * RFC 8259 says JSON objects are "unordered collections," but practically,
 * insertion order matters for:
 *   - Human readability (people expect keys in a predictable order)
 *   - Round-trip fidelity (parse then serialize should preserve order)
 *   - Deterministic output (same input always produces same output)
 *
 * JavaScript's `Map` preserves insertion order, making it ideal for this.
 *
 * @module
 */

export {
  type JsonValue,
  type JsonObject,
  type JsonArray,
  type JsonString,
  type JsonNumber,
  type JsonBoolean,
  type JsonNull,
  jsonObject,
  jsonArray,
  jsonString,
  jsonNumber,
  jsonBool,
  jsonNull,
} from "./value.js";

export { fromAST } from "./from-ast.js";
export { toNative } from "./to-native.js";
export { fromNative } from "./from-native.js";
export { parse, parseNative } from "./parse.js";
export { JsonValueError } from "./errors.js";
