// JsonSerializer.swift
// ============================================================================
// Serialize a JsonValue to a JSON string (compact or pretty-printed).
// ============================================================================
//
// What is serialization?
// ----------------------
// Serialization (also called encoding or marshalling) converts an in-memory
// data structure into a format suitable for storage or transmission. For JSON,
// that means converting a `JsonValue` tree into a UTF-8 text string.
//
// This is the inverse of parsing:
//   serialize:   JsonValue → String   (this file)
//   deserialize: String → JsonValue   (via JsonParser)
//
// Two output modes
// ----------------
// We support two modes:
//
// 1. **Compact** (default): No extra whitespace. Produces the smallest
//    possible output. Good for network transmission or machine-readable output.
//    Example:  {"name":"Alice","scores":[10,20,30]}
//
// 2. **Pretty**: Indented with 2 spaces per nesting level. Good for human
//    reading, debugging, and config files.
//    Example:
//    {
//      "name": "Alice",
//      "scores": [
//        10,
//        20,
//        30
//      ]
//    }
//
// String encoding
// ---------------
// The core challenge in JSON serialization is encoding strings. The JSON
// spec (RFC 8259 §7) requires that certain characters be escaped:
//   - `"` → `\"`   (otherwise it would end the string prematurely)
//   - `\` → `\\`   (otherwise it would start an escape sequence)
//   - Control chars (U+0000..U+001F) → `\uXXXX` or named escapes
//
// The named escapes (`\n`, `\t`, `\r`, etc.) are shorter and more readable
// than `\u000A`, `\u0009`, etc.
//
// Number formatting
// -----------------
// JSON numbers should not have unnecessary trailing decimal points or zeros.
// `42.0` should be output as `42`, not `42.0`. We use `truncatingRemainder`
// to detect whole numbers and format them as integers.
//
// ============================================================================

import JsonValue
import JsonParser

/// Serializes `JsonValue` to a JSON string, and deserializes JSON strings
/// back to `JsonValue`.
///
/// ```swift
/// let s = JsonSerializer()
/// let json = s.serialize(.object([("name", .string("Alice")), ("age", .number(30))]))
/// // → {"name":"Alice","age":30}
///
/// let p = JsonSerializer(mode: .pretty)
/// let pretty = p.serialize(.array([.number(1), .number(2)]))
/// // → [\n  1,\n  2\n]
/// ```
public struct JsonSerializer: Sendable {

    // =========================================================================
    // MARK: — Mode
    // =========================================================================

    /// The output mode for serialization.
    ///
    /// - `compact`: No extra whitespace (default). Produces minimal output.
    /// - `pretty`:  Indented with 2 spaces per nesting level.
    public enum Mode: Sendable {
        case compact
        case pretty
    }

    public let mode: Mode

    public init(mode: Mode = .compact) {
        self.mode = mode
    }

    // =========================================================================
    // MARK: — Public API
    // =========================================================================

    /// Serialize a `JsonValue` to a JSON string.
    ///
    /// The output is always valid JSON — it can be passed back to `deserialize`
    /// or any standard JSON parser and produce an equal `JsonValue`.
    ///
    /// - Parameter value: The `JsonValue` to serialize.
    /// - Returns: A UTF-8 JSON string.
    public func serialize(_ value: JsonValue) -> String {
        switch mode {
        case .compact:
            return serializeCompact(value)
        case .pretty:
            return serializePretty(value, depth: 0)
        }
    }

    /// Deserialize a JSON string into a `JsonValue`.
    ///
    /// This is a convenience wrapper around `JsonParser.parse(_:)`. It makes
    /// `JsonSerializer` a complete bidirectional codec:
    ///
    /// ```swift
    /// let s = JsonSerializer()
    /// let v = try s.deserialize("{\"x\":1}")
    /// ```
    ///
    /// - Parameter input: A UTF-8 JSON string.
    /// - Returns: The parsed `JsonValue`.
    /// - Throws: `JsonLexError` or `JsonParseError` on malformed input.
    public func deserialize(_ input: String) throws -> JsonValue {
        let parser = JsonParser()
        return try parser.parse(input)
    }

    // =========================================================================
    // MARK: — Compact serialization
    // =========================================================================
    //
    // Compact mode produces output with no extra whitespace — only the
    // characters required by the JSON grammar. This minimizes payload size.

    private func serializeCompact(_ value: JsonValue) -> String {
        switch value {
        case .null:
            return "null"

        case .bool(let b):
            return b ? "true" : "false"

        case .number(let n):
            return formatNumber(n)

        case .string(let s):
            return encodeString(s)

        case .array(let elements):
            // Join elements with commas, wrap in brackets.
            let inner = elements.map { serializeCompact($0) }.joined(separator: ",")
            return "[\(inner)]"

        case .object(let pairs):
            // Each pair is key:value (no space around colon in compact mode).
            let inner = pairs.map { encodeString($0.key) + ":" + serializeCompact($0.value) }
            return "{\(inner.joined(separator: ","))}"
        }
    }

    // =========================================================================
    // MARK: — Pretty serialization
    // =========================================================================
    //
    // Pretty mode indents each nesting level by 2 spaces. The `depth`
    // parameter tracks the current indentation level:
    //   - depth 0: top-level (no indentation)
    //   - depth 1: inside a top-level array or object (2 spaces)
    //   - depth 2: inside a nested structure (4 spaces)
    //   - etc.
    //
    // Empty arrays and objects are rendered on one line (`[]`, `{}`),
    // which is both valid JSON and more readable than a multi-line empty
    // container.

    private func serializePretty(_ value: JsonValue, depth: Int) -> String {
        // `indent` is the indentation for the CURRENT depth's closing bracket.
        // `innerIndent` is the indentation for items INSIDE this structure.
        let indent = String(repeating: "  ", count: depth)
        let innerIndent = String(repeating: "  ", count: depth + 1)

        switch value {
        case .null:
            return "null"
        case .bool(let b):
            return b ? "true" : "false"
        case .number(let n):
            return formatNumber(n)
        case .string(let s):
            return encodeString(s)

        case .array(let elements):
            // Empty arrays are compact.
            if elements.isEmpty { return "[]" }
            // Each element on its own indented line.
            let items = elements.map { innerIndent + serializePretty($0, depth: depth + 1) }
            return "[\n" + items.joined(separator: ",\n") + "\n" + indent + "]"

        case .object(let pairs):
            // Empty objects are compact.
            if pairs.isEmpty { return "{}" }
            // Each pair on its own indented line, with `: ` (space after colon).
            let items = pairs.map {
                innerIndent + encodeString($0.key) + ": " + serializePretty($0.value, depth: depth + 1)
            }
            return "{\n" + items.joined(separator: ",\n") + "\n" + indent + "}"
        }
    }

    // =========================================================================
    // MARK: — Number formatting
    // =========================================================================

    /// Format a Double as a JSON number string.
    ///
    /// Key decisions:
    /// - Whole numbers (42.0) are formatted as integers ("42"), not "42.0".
    ///   This matches the output of JavaScript's JSON.stringify and Python's
    ///   json.dumps for integer-valued doubles.
    /// - We use Int64 for whole numbers to avoid scientific notation on large
    ///   integers (Double's default String conversion would give "1e+20" for
    ///   100_000_000_000_000_000_000, which is valid JSON but less readable).
    /// - NaN and ±Infinity are NOT valid JSON numbers. If somehow a .number
    ///   case contains one of these, we fall back to Swift's default String
    ///   representation (which is "nan", "inf", "-inf"). These are NOT valid
    ///   JSON and will cause errors if re-parsed, but it's better to surface
    ///   the problem than silently produce garbage.
    private func formatNumber(_ n: Double) -> String {
        // Detect whole numbers: truncatingRemainder divides by 1 and keeps
        // the fractional part. If there's no fractional part, it's an integer.
        if n.truncatingRemainder(dividingBy: 1) == 0 && !n.isInfinite && !n.isNaN {
            // Bounds check: Int64 can hold up to 2^63−1 ≈ 9.2×10^18.
            // Doubles outside this range fall back to the floating-point form.
            if n >= Double(Int64.min) && n <= Double(Int64.max) {
                return String(Int64(n))
            }
        }
        return String(n)
    }

    // =========================================================================
    // MARK: — Helpers
    // =========================================================================

    /// Format a Unicode scalar value as a 4-digit lowercase hex string (no prefix).
    ///
    /// Used for \uXXXX escape sequences. We implement this without Foundation
    /// to keep the package dependency-free.
    ///
    /// Example: hex4(0x0001) → "0001", hex4(0x001F) → "001f"
    private func hex4(_ value: UInt32) -> String {
        let hexChars: [Character] = ["0","1","2","3","4","5","6","7","8","9","a","b","c","d","e","f"]
        let d0 = hexChars[Int((value >> 12) & 0xF)]
        let d1 = hexChars[Int((value >>  8) & 0xF)]
        let d2 = hexChars[Int((value >>  4) & 0xF)]
        let d3 = hexChars[Int((value >>  0) & 0xF)]
        return String([d0, d1, d2, d3])
    }

    // =========================================================================
    // MARK: — String encoding
    // =========================================================================

    /// Encode a Swift String as a JSON string literal, including the wrapping
    /// double quotes and all necessary escape sequences.
    ///
    /// RFC 8259 §7 requires these characters to be escaped:
    ///   U+0022 (") → \"
    ///   U+005C (\) → \\
    ///   U+0000..U+001F (control chars) → \uXXXX or named escape
    ///
    /// The `/` (U+002F) can optionally be escaped as `\/`. We do NOT escape
    /// it unless it appears in a context that requires it (it doesn't for
    /// standard JSON), keeping our output cleaner.
    ///
    /// We iterate over `unicodeScalars` instead of `characters` so that we
    /// can compare Unicode code points directly against the thresholds above.
    /// Grapheme clusters (what `for c in string` iterates) combine code points
    /// and would complicate the comparison.
    private func encodeString(_ s: String) -> String {
        var result = "\""
        for scalar in s.unicodeScalars {
            switch scalar {
            case "\"":
                // Double quote MUST be escaped — it would end the string.
                result += "\\\""
            case "\\":
                // Backslash MUST be escaped — it starts escape sequences.
                result += "\\\\"
            case "\n":
                // Newline — use the compact named escape.
                result += "\\n"
            case "\r":
                // Carriage return — use the compact named escape.
                result += "\\r"
            case "\t":
                // Tab — use the compact named escape.
                result += "\\t"
            case "\u{08}":
                // Backspace (U+0008) — use the compact named escape.
                result += "\\b"
            case "\u{0C}":
                // Form feed (U+000C) — use the compact named escape.
                result += "\\f"
            case _ where scalar.value < 0x20:
                // Other control characters (U+0000..U+001F) that don't have
                // named escapes must use the \uXXXX form.
                // We build the hex string manually to avoid Foundation dependency.
                result += "\\u" + hex4(scalar.value)
            default:
                // All other characters (ASCII printable + Unicode) are safe
                // to include directly as-is.
                result += String(scalar)
            }
        }
        result += "\""
        return result
    }
}
