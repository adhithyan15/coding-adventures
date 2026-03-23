/**
 * Serializer Configuration
 *
 * Controls how pretty-printed JSON output is formatted.
 *
 * Default Behavior
 * ----------------
 *
 * With no config (or default config), serialize_pretty produces:
 *   - 2-space indentation
 *   - Preserved insertion order for object keys
 *   - No trailing newline
 *
 * Example with default config:
 *
 *     {
 *       "name": "Alice",
 *       "age": 30
 *     }
 *
 * Example with custom config (4 spaces, sorted keys, trailing newline):
 *
 *     {
 *         "age": 30,
 *         "name": "Alice"
 *     }
 *     <newline>
 *
 * @module
 */

/**
 * Configuration for JSON pretty-printing.
 *
 * All fields are optional. Omitted fields use defaults.
 */
export interface SerializerConfig {
  /**
   * Number of indent characters per indentation level.
   * Default: 2
   *
   * Common values:
   *   - 2 (compact but readable -- popular in JavaScript/TypeScript)
   *   - 4 (spacious -- popular in Python)
   *   - 1 (when using tabs)
   */
  indentSize?: number;

  /**
   * Character to use for indentation. Must be ' ' (space) or '\t' (tab).
   * Default: ' ' (space)
   *
   * The tabs-vs-spaces debate is eternal. We support both and take no sides.
   */
  indentChar?: string;

  /**
   * Whether to sort object keys alphabetically.
   * Default: false (preserve insertion order)
   *
   * Sorting is useful for:
   *   - Deterministic output (same input always produces same output)
   *   - Diffing (changes are easier to spot when keys are sorted)
   *   - Canonical form (e.g., for content-addressable storage)
   *
   * Insertion order is useful for:
   *   - Round-trip fidelity (parse then serialize preserves original order)
   *   - Human-authored JSON (keys are in a deliberate order)
   */
  sortKeys?: boolean;

  /**
   * Whether to add a newline character at the end of the output.
   * Default: false
   *
   * POSIX convention says text files should end with a newline.
   * Set this to true when writing JSON to files.
   */
  trailingNewline?: boolean;
}

/**
 * The default configuration. Used when no config is provided.
 */
export const DEFAULT_CONFIG: Required<SerializerConfig> = {
  indentSize: 2,
  indentChar: " ",
  sortKeys: false,
  trailingNewline: false,
};

/**
 * Merge a partial config with defaults to get a complete config.
 *
 * Any field not provided in the input uses the default value.
 * This lets callers specify only the fields they care about:
 *
 *     resolveConfig({ indentSize: 4 })
 *     // { indentSize: 4, indentChar: ' ', sortKeys: false, trailingNewline: false }
 */
export function resolveConfig(
  config?: SerializerConfig
): Required<SerializerConfig> {
  if (!config) return { ...DEFAULT_CONFIG };
  return {
    indentSize: config.indentSize ?? DEFAULT_CONFIG.indentSize,
    indentChar: config.indentChar ?? DEFAULT_CONFIG.indentChar,
    sortKeys: config.sortKeys ?? DEFAULT_CONFIG.sortKeys,
    trailingNewline: config.trailingNewline ?? DEFAULT_CONFIG.trailingNewline,
  };
}
