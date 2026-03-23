/**
 * Error Types for JSON Value Operations
 *
 * A single error class covers all failure modes in the json-value package:
 *   - Invalid AST structure (from_ast encounters unexpected nodes)
 *   - Non-JSON-compatible native types (from_native gets a function or class)
 *   - Parse failures (delegated from the underlying parser)
 *
 * Why One Error Class?
 * --------------------
 *
 * The json-value package has a small API surface. Splitting errors into
 * ASTConversionError, NativeConversionError, and ParseError would be
 * over-engineering. One class with a descriptive message is sufficient.
 * Users who need to distinguish error sources can check the message.
 */

/**
 * Error thrown when a JSON value operation fails.
 *
 * Common causes:
 *   - Passing a non-JSON-compatible type to fromNative() (e.g., a function)
 *   - Parsing invalid JSON text via parse() or parseNative()
 *   - Encountering an unexpected AST structure in fromAST()
 */
export class JsonValueError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "JsonValueError";
  }
}
