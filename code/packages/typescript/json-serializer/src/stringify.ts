/**
 * Convenience API -- native types to JSON text
 *
 * These functions compose fromNative() with serialize/serializePretty,
 * providing a one-call path from native JavaScript values to JSON text.
 *
 * This is the "just give me a JSON string" API. Users who don't care about
 * JsonValue intermediate representation use these functions directly.
 *
 *     stringify({ name: "Alice", age: 30 })
 *     // '{"name":"Alice","age":30}'
 *
 *     stringifyPretty({ name: "Alice", age: 30 })
 *     // '{\n  "name": "Alice",\n  "age": 30\n}'
 *
 * @module
 */

import { fromNative } from "@coding-adventures/json-value";
import type { SerializerConfig } from "./config.js";
import { serialize, serializePretty } from "./serializer.js";

/**
 * Convert native JavaScript types to compact JSON text.
 *
 * Equivalent to: serialize(fromNative(value))
 *
 * @param value - A JSON-compatible JavaScript value.
 * @returns Compact JSON text.
 * @throws JsonValueError if the value contains non-JSON-compatible types.
 *
 * @example
 *     stringify({ name: "Alice", age: 30 })   // '{"name":"Alice","age":30}'
 *     stringify([1, 2, 3])                     // '[1,2,3]'
 *     stringify("hello")                       // '"hello"'
 *     stringify(42)                             // '42'
 *     stringify(true)                           // 'true'
 *     stringify(null)                           // 'null'
 */
export function stringify(value: unknown): string {
  return serialize(fromNative(value));
}

/**
 * Convert native JavaScript types to pretty-printed JSON text.
 *
 * Equivalent to: serializePretty(fromNative(value), config)
 *
 * @param value - A JSON-compatible JavaScript value.
 * @param config - Optional formatting configuration.
 * @returns Pretty-printed JSON text.
 * @throws JsonValueError if the value contains non-JSON-compatible types.
 *
 * @example
 *     stringifyPretty({ name: "Alice" })
 *     // '{\n  "name": "Alice"\n}'
 *
 *     stringifyPretty({ b: 2, a: 1 }, { sortKeys: true })
 *     // '{\n  "a": 1,\n  "b": 2\n}'
 */
export function stringifyPretty(
  value: unknown,
  config?: SerializerConfig
): string {
  return serializePretty(fromNative(value), config);
}
