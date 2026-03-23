/**
 * JSON Serializer -- converts JsonValue or native types to JSON text.
 *
 * This package is the final stage of the JSON pipeline:
 *
 *     JsonValue tree  -->  json-serializer  -->  JSON text
 *     native types    -->  json-serializer  -->  JSON text
 *
 * Two Output Modes
 * ----------------
 *
 * 1. **Compact** -- minimal whitespace, smallest output size.
 *    Used for wire transmission, storage, and machine-to-machine communication.
 *
 *        serialize(value)    --> '{"name":"Alice","age":30}'
 *        stringify(native)   --> '{"name":"Alice","age":30}'
 *
 * 2. **Pretty** -- human-readable with configurable indentation.
 *    Used for debugging, logging, configuration files, and human review.
 *
 *        serializePretty(value)    --> '{\n  "name": "Alice",\n  "age": 30\n}'
 *        stringifyPretty(native)   --> '{\n  "name": "Alice",\n  "age": 30\n}'
 *
 * Configuration
 * -------------
 *
 * Pretty-printing is configurable via SerializerConfig:
 *   - indentSize: spaces per level (default: 2)
 *   - indentChar: space or tab (default: space)
 *   - sortKeys: alphabetically sort object keys (default: false)
 *   - trailingNewline: add \n at end of output (default: false)
 *
 * @module
 */

export { type SerializerConfig } from "./config.js";
export { serialize, serializePretty } from "./serializer.js";
export { stringify, stringifyPretty } from "./stringify.js";
export { JsonSerializerError } from "./errors.js";
