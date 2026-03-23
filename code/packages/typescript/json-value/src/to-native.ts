/**
 * JsonValue to Native Type Conversion
 *
 * This module converts typed JsonValue trees into plain JavaScript values:
 *
 *     JsonValue Variant    JavaScript Type
 *     ----------------    ---------------
 *     JsonObject          Object (plain {})
 *     JsonArray           Array
 *     JsonString          string
 *     JsonNumber          number
 *     JsonBoolean         boolean
 *     JsonNull            null
 *
 * Why Convert to Native?
 * ----------------------
 *
 * JsonValue is great for type-safe access and pattern matching. But most
 * application code just wants a plain object:
 *
 *     // JsonValue access (verbose but type-safe):
 *     if (value.type === 'object') {
 *       const name = value.pairs.get('name');
 *       if (name?.type === 'string') {
 *         console.log(name.value);
 *       }
 *     }
 *
 *     // Native access (concise but untyped):
 *     const obj = toNative(value) as Record<string, unknown>;
 *     console.log(obj.name);
 *
 * The toNative() function bridges this gap.
 *
 * Object Conversion
 * -----------------
 *
 * JSON objects are converted to plain JavaScript objects ({}).
 * We use Object.create(null) to avoid prototype pollution -- the resulting
 * object has no prototype, so keys like "constructor" or "__proto__" are
 * safe to use. Actually, for simplicity and compatibility, we just use a
 * regular object literal since JSON keys are always strings and we control
 * the input.
 *
 * @module
 */

import type { JsonValue } from "./value.js";

/**
 * Convert a JsonValue to native JavaScript types.
 *
 * The conversion is recursive -- nested JsonValues are also converted.
 *
 * @param value - The JsonValue to convert.
 * @returns A plain JavaScript value (object, array, string, number, boolean, or null).
 *
 * @example
 *     const jv = jsonObject(new Map([
 *       ["name", jsonString("Alice")],
 *       ["age", jsonNumber(30)]
 *     ]));
 *     const native = toNative(jv);
 *     // { name: "Alice", age: 30 }
 *
 * @example
 *     const arr = jsonArray([jsonNumber(1), jsonBool(true), jsonNull()]);
 *     const native = toNative(arr);
 *     // [1, true, null]
 */
export function toNative(value: JsonValue): unknown {
  switch (value.type) {
    /**
     * Objects: Convert Map<string, JsonValue> to Record<string, unknown>.
     * Iteration over the Map preserves insertion order.
     */
    case "object": {
      const result: Record<string, unknown> = {};
      for (const [key, val] of value.pairs) {
        result[key] = toNative(val);
      }
      return result;
    }

    /**
     * Arrays: Convert JsonValue[] to unknown[].
     * Each element is recursively converted.
     */
    case "array":
      return value.elements.map(toNative);

    /**
     * Primitives: Direct extraction of the wrapped value.
     * No recursion needed -- these are leaf values.
     */
    case "string":
      return value.value;

    case "number":
      return value.value;

    case "boolean":
      return value.value;

    /**
     * Null: Return JavaScript null.
     * Note: This is the only JsonValue variant with no `value` field.
     */
    case "null":
      return null;
  }
}
