/**
 * Core Serializer -- JsonValue to JSON text
 *
 * This module implements the two core serialization functions:
 *
 *   - serialize(value)       --> compact JSON text (no whitespace)
 *   - serializePretty(value) --> pretty-printed JSON text (with indentation)
 *
 * Algorithm
 * ---------
 *
 * Both functions use recursive dispatch on the JsonValue discriminant:
 *
 *     JsonValue Variant    Compact Output         Pretty Output
 *     ----------------    ---------------        ---------------
 *     null                "null"                 "null"
 *     boolean             "true"/"false"         "true"/"false"
 *     number              "42" / "3.14"          "42" / "3.14"
 *     string              '"hello"'              '"hello"'
 *     array               '[1,2,3]'              '[\n  1,\n  2\n]'
 *     object              '{"a":1}'              '{\n  "a": 1\n}'
 *
 * String Escaping
 * ---------------
 *
 * Per RFC 8259, certain characters MUST be escaped in JSON strings:
 *
 *     Character        Escape    Reason
 *     ---------        ------    ------
 *     " (quote)        \"        String delimiter
 *     \ (backslash)    \\        Escape character
 *     Backspace        \b        Control char (U+0008)
 *     Form feed        \f        Control char (U+000C)
 *     Newline          \n        Control char (U+000A)
 *     Carriage return  \r        Control char (U+000D)
 *     Tab              \t        Control char (U+0009)
 *     U+0000-U+001F    \uXXXX   All other control characters
 *
 * Forward slash (/) is NOT escaped. RFC 8259 allows but does not require it.
 *
 * @module
 */

import type { JsonValue } from "coding-adventures-json-value";
import type { SerializerConfig } from "./config.js";
import { resolveConfig } from "./config.js";
import { JsonSerializerError } from "./errors.js";

// =============================================================================
// COMPACT SERIALIZATION
// =============================================================================

/**
 * Serialize a JsonValue to compact JSON text.
 *
 * No unnecessary whitespace. Suitable for wire transmission, storage,
 * and any context where output size matters.
 *
 * @param value - The JsonValue to serialize.
 * @returns Compact JSON text.
 * @throws JsonSerializerError if the value contains Infinity or NaN.
 *
 * @example
 *     serialize(jsonNull())                    // "null"
 *     serialize(jsonNumber(42))                // "42"
 *     serialize(jsonString("hello"))           // '"hello"'
 *     serialize(jsonObject([["a", jsonNumber(1)]]))  // '{"a":1}'
 */
export function serialize(value: JsonValue): string {
  switch (value.type) {
    /**
     * Null: The simplest case. JSON null is always the literal "null".
     */
    case "null":
      return "null";

    /**
     * Boolean: JSON booleans are the literals "true" and "false".
     * Note the lowercase -- JSON, unlike some languages, uses lowercase.
     */
    case "boolean":
      return value.value ? "true" : "false";

    /**
     * Number: Convert to string, but check for non-finite values first.
     *
     * JSON cannot represent Infinity or NaN. IEEE 754 defines these as
     * special floating-point values, but they have no JSON equivalent.
     * We throw rather than silently converting to null (which some
     * serializers do) because that would lose information.
     */
    case "number":
      return serializeNumber(value.value, value.isInteger);

    /**
     * String: Wrap in quotes and escape special characters.
     */
    case "string":
      return serializeString(value.value);

    /**
     * Array: Recursively serialize each element, join with commas.
     * Empty arrays are always "[]" (no whitespace).
     */
    case "array": {
      if (value.elements.length === 0) return "[]";
      const parts = value.elements.map(serialize);
      return "[" + parts.join(",") + "]";
    }

    /**
     * Object: Recursively serialize each key-value pair.
     * Keys are always strings (JSON requirement).
     * Empty objects are always "{}" (no whitespace).
     */
    case "object": {
      if (value.pairs.size === 0) return "{}";
      const parts: string[] = [];
      for (const [key, val] of value.pairs) {
        parts.push(serializeString(key) + ":" + serialize(val));
      }
      return "{" + parts.join(",") + "}";
    }
  }
}

// =============================================================================
// PRETTY SERIALIZATION
// =============================================================================

/**
 * Serialize a JsonValue to pretty-printed JSON text.
 *
 * Uses configurable indentation, optional key sorting, and optional
 * trailing newlines.
 *
 * @param value - The JsonValue to serialize.
 * @param config - Optional formatting configuration. Uses defaults if omitted.
 * @returns Pretty-printed JSON text.
 * @throws JsonSerializerError if the value contains Infinity or NaN.
 *
 * @example
 *     serializePretty(jsonObject([["a", jsonNumber(1)]]))
 *     // '{\n  "a": 1\n}'
 *
 * @example
 *     serializePretty(value, { indentSize: 4, sortKeys: true })
 */
export function serializePretty(
  value: JsonValue,
  config?: SerializerConfig
): string {
  const resolved = resolveConfig(config);
  const result = serializePrettyRecursive(value, resolved, 0);

  if (resolved.trailingNewline) {
    return result + "\n";
  }
  return result;
}

/**
 * The recursive workhorse for pretty serialization.
 *
 * @param value - Current JsonValue node.
 * @param config - Resolved (complete) configuration.
 * @param depth - Current nesting depth (0 = top level).
 * @returns Pretty-printed JSON text for this node and its children.
 */
function serializePrettyRecursive(
  value: JsonValue,
  config: Required<SerializerConfig>,
  depth: number
): string {
  /**
   * Build the indentation strings.
   *
   * indent       = one level of indentation (e.g., "  " for 2-space)
   * currentIndent = indentation at the current depth
   * nextIndent    = indentation at depth + 1 (for children)
   */
  const indent = config.indentChar.repeat(config.indentSize);
  const currentIndent = indent.repeat(depth);
  const nextIndent = indent.repeat(depth + 1);

  switch (value.type) {
    /**
     * Primitives have no internal structure to indent.
     * They serialize the same way in compact and pretty modes.
     */
    case "null":
      return "null";

    case "boolean":
      return value.value ? "true" : "false";

    case "number":
      return serializeNumber(value.value, value.isInteger);

    case "string":
      return serializeString(value.value);

    /**
     * Arrays: each element on its own line, indented one level deeper.
     *
     * Empty: []
     * Non-empty:
     *     [
     *       1,
     *       2,
     *       3
     *     ]
     */
    case "array": {
      if (value.elements.length === 0) return "[]";

      const lines = value.elements.map(
        (elem) =>
          nextIndent +
          serializePrettyRecursive(elem, config, depth + 1)
      );
      return "[\n" + lines.join(",\n") + "\n" + currentIndent + "]";
    }

    /**
     * Objects: each key-value pair on its own line, indented one level deeper.
     * A space separates the colon from the value (": " not ":").
     *
     * Empty: {}
     * Non-empty:
     *     {
     *       "name": "Alice",
     *       "age": 30
     *     }
     *
     * With sortKeys: keys are sorted alphabetically using localeCompare.
     */
    case "object": {
      if (value.pairs.size === 0) return "{}";

      let keys = Array.from(value.pairs.keys());
      if (config.sortKeys) {
        keys = keys.sort();
      }

      const lines = keys.map((key) => {
        const val = value.pairs.get(key)!;
        const valStr = serializePrettyRecursive(val, config, depth + 1);
        return nextIndent + serializeString(key) + ": " + valStr;
      });
      return "{\n" + lines.join(",\n") + "\n" + currentIndent + "}";
    }
  }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/**
 * Serialize a number to JSON text.
 *
 * JSON numbers must be finite. Infinity and NaN have no representation.
 *
 * For integers, we use the standard string conversion.
 * For floats, we use the standard string conversion which JavaScript
 * handles well (no trailing zeros, reasonable precision).
 *
 * @param n - The number to serialize.
 * @param isInteger - Whether to format as integer (no decimal point).
 * @returns The number as a JSON string.
 * @throws JsonSerializerError for Infinity or NaN.
 */
function serializeNumber(n: number, isInteger: boolean): string {
  if (!isFinite(n)) {
    throw new JsonSerializerError(
      `Cannot serialize ${n} to JSON. ` +
        "JSON does not support Infinity or NaN."
    );
  }

  /**
   * For integers, String() produces clean output: "42", "-17", "0".
   * For floats, String() produces: "3.14", "0.001", "1e+25".
   *
   * JavaScript's default number-to-string conversion follows the
   * ECMAScript specification's Number::toString, which produces
   * the shortest representation that round-trips exactly.
   */
  if (isInteger) {
    return String(n);
  }
  return String(n);
}

/**
 * Serialize a string to JSON text (with quotes and escaping).
 *
 * Per RFC 8259 Section 7, these characters must be escaped:
 *
 *     Char          Escape    Code Point
 *     ----          ------    ----------
 *     Quotation     \"        U+0022
 *     Backslash     \\        U+005C
 *     Backspace     \b        U+0008
 *     Form feed     \f        U+000C
 *     Newline       \n        U+000A
 *     Carriage ret  \r        U+000D
 *     Tab           \t        U+0009
 *     Control chars \uXXXX   U+0000 to U+001F (those not covered above)
 *
 * All other characters (including /, non-ASCII Unicode) pass through unchanged.
 *
 * @param s - The string to serialize.
 * @returns The string wrapped in quotes with special chars escaped.
 */
function serializeString(s: string): string {
  let result = '"';

  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    const code = s.charCodeAt(i);

    switch (ch) {
      case '"':
        result += '\\"';
        break;
      case "\\":
        result += "\\\\";
        break;
      case "\b":
        result += "\\b";
        break;
      case "\f":
        result += "\\f";
        break;
      case "\n":
        result += "\\n";
        break;
      case "\r":
        result += "\\r";
        break;
      case "\t":
        result += "\\t";
        break;
      default:
        /**
         * Check for control characters (U+0000 to U+001F).
         * These are the C0 control codes and must be escaped as \uXXXX.
         *
         * The named escapes above (\b, \f, \n, \r, \t) cover the most
         * common control characters. The remaining ones (U+0000-U+0007,
         * U+000E-U+001F) use the \uXXXX format.
         */
        if (code < 0x20) {
          result += "\\u" + code.toString(16).padStart(4, "0");
        } else {
          result += ch;
        }
    }
  }

  result += '"';
  return result;
}
