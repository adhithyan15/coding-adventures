/**
 * JsonValue -- the discriminated union type for JSON data.
 *
 * JSON defines exactly six value types. This module provides:
 *   1. TypeScript types for each variant (using discriminated unions)
 *   2. Factory functions for constructing each variant
 *
 * Discriminated Unions in TypeScript
 * ----------------------------------
 *
 * A discriminated union uses a shared literal field (the "discriminant") to
 * let TypeScript narrow the type. Here, the discriminant is `type`:
 *
 *     function describe(v: JsonValue): string {
 *       switch (v.type) {
 *         case 'string':  return `String: ${v.value}`;   // TS knows v.value is string
 *         case 'number':  return `Number: ${v.value}`;   // TS knows v.value is number
 *         case 'boolean': return `Bool: ${v.value}`;     // TS knows v.value is boolean
 *         case 'null':    return 'Null';                  // TS knows no .value exists
 *         case 'array':   return `Array(${v.elements.length})`;
 *         case 'object':  return `Object(${v.pairs.size})`;
 *       }
 *     }
 *
 * The compiler ensures exhaustive handling -- if you add a new variant,
 * every switch statement must be updated.
 *
 * @module
 */

// =============================================================================
// TYPE DEFINITIONS
// =============================================================================

/**
 * A JSON object -- an ordered collection of key-value pairs.
 *
 * Why Map instead of Record<string, JsonValue>?
 *   - Map preserves insertion order (guaranteed by the spec)
 *   - Map has a cleaner API for iteration (.entries(), .keys(), .values())
 *   - Map allows any string as a key (no prototype pollution concerns)
 */
export type JsonObject = {
  readonly type: "object";
  readonly pairs: Map<string, JsonValue>;
};

/**
 * A JSON array -- an ordered sequence of values.
 *
 * Arrays can contain mixed types: [1, "two", true, null, [3], {"four": 4}]
 * This is why elements are typed as JsonValue[], not a specific variant.
 */
export type JsonArray = {
  readonly type: "array";
  readonly elements: JsonValue[];
};

/**
 * A JSON string value.
 *
 * The value is already unescaped -- "\n" in the JSON source becomes a real
 * newline character in `value`. The lexer handles escape processing.
 */
export type JsonString = {
  readonly type: "string";
  readonly value: string;
};

/**
 * A JSON number value.
 *
 * JSON doesn't distinguish integer and float, but practically:
 *   - `42` has no decimal point or exponent --> isInteger = true
 *   - `3.14` has a decimal point --> isInteger = false
 *   - `1e10` has an exponent --> isInteger = false
 *
 * The `isInteger` flag preserves this distinction for round-trip fidelity.
 * The actual `value` is always a JavaScript `number` (IEEE 754 double).
 */
export type JsonNumber = {
  readonly type: "number";
  readonly value: number;
  readonly isInteger: boolean;
};

/**
 * A JSON boolean value -- true or false.
 */
export type JsonBoolean = {
  readonly type: "boolean";
  readonly value: boolean;
};

/**
 * JSON null -- the absence of a value.
 *
 * Unlike JavaScript's null/undefined split, JSON has exactly one "nothing"
 * value: null. There is no "undefined" in JSON.
 */
export type JsonNull = {
  readonly type: "null";
};

/**
 * The union of all JSON value types.
 *
 * This is the primary type used throughout the json-value package.
 * The `type` field discriminates between variants, enabling TypeScript's
 * type narrowing in switch statements and conditionals.
 */
export type JsonValue =
  | JsonObject
  | JsonArray
  | JsonString
  | JsonNumber
  | JsonBoolean
  | JsonNull;

// =============================================================================
// FACTORY FUNCTIONS
// =============================================================================
//
// Why factory functions instead of raw object literals?
//
// 1. **Consistency.** Every JsonValue is created through a known path.
// 2. **Validation.** Factories can enforce invariants (e.g., NaN check on numbers).
// 3. **Readability.** `jsonString("hello")` reads better than `{ type: 'string', value: 'hello' }`.
// 4. **Refactoring safety.** If the internal representation changes, only the
//    factories need updating -- call sites stay the same.

/**
 * Create a JSON object value.
 *
 * @param pairs - A Map of string keys to JsonValue values, or an array of
 *                [key, value] pairs. If omitted, creates an empty object.
 *
 * @example
 *     const obj = jsonObject(new Map([["name", jsonString("Alice")]]));
 *     // { type: 'object', pairs: Map { 'name' => { type: 'string', value: 'Alice' } } }
 *
 * @example
 *     const empty = jsonObject();
 *     // { type: 'object', pairs: Map {} }
 */
export function jsonObject(
  pairs?: Map<string, JsonValue> | [string, JsonValue][]
): JsonObject {
  if (pairs === undefined) {
    return { type: "object", pairs: new Map() };
  }
  if (pairs instanceof Map) {
    return { type: "object", pairs };
  }
  return { type: "object", pairs: new Map(pairs) };
}

/**
 * Create a JSON array value.
 *
 * @param elements - The array elements. If omitted, creates an empty array.
 *
 * @example
 *     const arr = jsonArray([jsonNumber(1), jsonNumber(2), jsonNumber(3)]);
 */
export function jsonArray(elements?: JsonValue[]): JsonArray {
  return { type: "array", elements: elements ?? [] };
}

/**
 * Create a JSON string value.
 *
 * @param value - The string content (already unescaped).
 *
 * @example
 *     const s = jsonString("hello");
 *     // { type: 'string', value: 'hello' }
 */
export function jsonString(value: string): JsonString {
  return { type: "string", value };
}

/**
 * Create a JSON number value.
 *
 * @param value - The numeric value.
 * @param isInteger - Whether this number was written without a decimal point
 *                    or exponent in the original JSON. If omitted, determined
 *                    automatically using Number.isInteger().
 *
 * @example
 *     const n = jsonNumber(42);        // isInteger = true (auto-detected)
 *     const f = jsonNumber(3.14);      // isInteger = false (auto-detected)
 *     const e = jsonNumber(1e10, false); // explicit: exponent means float
 */
export function jsonNumber(value: number, isInteger?: boolean): JsonNumber {
  return {
    type: "number",
    value,
    isInteger: isInteger ?? Number.isInteger(value),
  };
}

/**
 * Create a JSON boolean value.
 *
 * @param value - true or false.
 *
 * @example
 *     const t = jsonBool(true);
 *     const f = jsonBool(false);
 */
export function jsonBool(value: boolean): JsonBoolean {
  return { type: "boolean", value };
}

/**
 * Create a JSON null value.
 *
 * JSON null represents the intentional absence of any value.
 * Unlike JavaScript's null/undefined distinction, JSON has only null.
 *
 * @example
 *     const n = jsonNull();
 *     // { type: 'null' }
 */
export function jsonNull(): JsonNull {
  return { type: "null" };
}
