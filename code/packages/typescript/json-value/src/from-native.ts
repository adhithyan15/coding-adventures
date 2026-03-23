/**
 * Native Type to JsonValue Conversion
 *
 * This module converts plain JavaScript values into typed JsonValue trees.
 * It's the inverse of toNative().
 *
 *     JavaScript Type    JsonValue Variant
 *     ---------------    ----------------
 *     Object ({})        JsonObject
 *     Array              JsonArray
 *     string             JsonString
 *     number             JsonNumber
 *     boolean            JsonBoolean
 *     null               JsonNull
 *
 * Error Cases
 * -----------
 *
 * Not all JavaScript values are JSON-compatible. The following will throw
 * a JsonValueError:
 *
 *   - undefined (JSON has no "undefined" concept)
 *   - Functions (not serializable)
 *   - Symbols (not serializable)
 *   - BigInt (no JSON representation)
 *   - Dates (would need to decide on a string format)
 *   - Maps, Sets (use objects and arrays instead)
 *   - Class instances (not plain objects)
 *   - Non-string object keys
 *
 * Why Be Strict?
 * --------------
 *
 * A lenient approach (silently converting Date to string, function to null, etc.)
 * would mask bugs. If the caller passes a Date, they probably expected it to be
 * serialized in a specific format -- silently converting it to some default
 * format would produce unexpected output. Better to fail loudly and let the
 * caller decide how to handle non-JSON types.
 *
 * @module
 */

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

/**
 * Convert native JavaScript types to a JsonValue.
 *
 * The conversion is recursive -- nested objects and arrays are also converted.
 *
 * @param value - A JSON-compatible JavaScript value.
 * @returns A JsonValue representing the input.
 * @throws JsonValueError if the value contains non-JSON-compatible types.
 *
 * @example
 *     const jv = fromNative({ name: "Alice", age: 30 });
 *     // { type: 'object', pairs: Map { 'name' => ..., 'age' => ... } }
 *
 * @example
 *     const jv = fromNative([1, "two", true, null]);
 *     // { type: 'array', elements: [...] }
 *
 * @example
 *     // Error cases:
 *     fromNative(undefined);   // throws JsonValueError
 *     fromNative(() => {});     // throws JsonValueError
 *     fromNative(Symbol());    // throws JsonValueError
 */
export function fromNative(value: unknown): JsonValue {
  /**
   * Null check comes first because typeof null === "object" in JavaScript.
   * This is one of the language's most infamous quirks, dating back to the
   * original implementation where null was represented as the zero pointer,
   * which had the "object" type tag.
   */
  if (value === null) {
    return jsonNull();
  }

  /**
   * Dispatch on the JavaScript type.
   *
   * The ordering here matters for readability but not correctness --
   * each branch is mutually exclusive.
   */
  switch (typeof value) {
    case "string":
      return jsonString(value);

    case "number": {
      /**
       * JSON cannot represent Infinity or NaN. These are JavaScript-specific
       * extensions to IEEE 754 that have no JSON equivalent.
       */
      if (!isFinite(value)) {
        throw new JsonValueError(
          `Cannot convert ${value} to JsonValue: JSON does not support Infinity or NaN.`
        );
      }
      return jsonNumber(value);
    }

    case "boolean":
      return jsonBool(value);

    case "undefined":
      throw new JsonValueError(
        "Cannot convert undefined to JsonValue. " +
          "JSON has no 'undefined' concept -- use null instead."
      );

    case "function":
      throw new JsonValueError(
        "Cannot convert a function to JsonValue. " +
          "Functions are not JSON-serializable."
      );

    case "symbol":
      throw new JsonValueError(
        "Cannot convert a Symbol to JsonValue. " +
          "Symbols are not JSON-serializable."
      );

    case "bigint":
      throw new JsonValueError(
        "Cannot convert a BigInt to JsonValue. " +
          "Use Number() first if the value fits in a double, or use a string."
      );

    case "object": {
      /**
       * At this point we know:
       *   - value is not null (checked above)
       *   - typeof value === "object"
       *
       * It could be an Array, a plain object, or a class instance.
       */

      if (Array.isArray(value)) {
        /**
         * Arrays: recursively convert each element.
         */
        return jsonArray(value.map(fromNative));
      }

      /**
       * Plain objects: convert to JsonObject.
       * We accept any object with string keys. Non-string keys (from
       * non-JSON-compatible objects) would be caught by the key check below.
       */
      const obj = value as Record<string, unknown>;
      const pairs = new Map<string, JsonValue>();

      for (const key of Object.keys(obj)) {
        if (typeof key !== "string") {
          throw new JsonValueError(
            `Cannot convert object with non-string key: ${String(key)}. ` +
              "JSON object keys must be strings."
          );
        }
        pairs.set(key, fromNative(obj[key]));
      }

      return jsonObject(pairs);
    }

    default:
      throw new JsonValueError(
        `Cannot convert value of type "${typeof value}" to JsonValue. ` +
          "Only string, number, boolean, null, object, and array are supported."
      );
  }
}
