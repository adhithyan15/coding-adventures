/**
 * Error Types for JSON Serialization
 *
 * Serialization can fail when:
 *   - A JsonNumber contains Infinity or NaN (no JSON representation)
 *   - A native value is not JSON-compatible (delegated to json-value's fromNative)
 *
 * These are programming errors, not user input errors -- the caller should
 * ensure values are JSON-compatible before serializing.
 */

/**
 * Error thrown when JSON serialization fails.
 *
 * @example
 *     serialize(jsonNumber(Infinity))  // throws JsonSerializerError
 */
export class JsonSerializerError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "JsonSerializerError";
  }
}
