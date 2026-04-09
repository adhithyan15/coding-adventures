// JsonValue.swift
// ============================================================================
// A tagged-union type representing any JSON value.
// ============================================================================
//
// JSON (JavaScript Object Notation) is defined by RFC 8259. It specifies
// exactly SIX value types:
//
//   1. null    — the absence of a value
//   2. boolean — true or false
//   3. number  — a 64-bit IEEE 754 floating-point (Double covers this)
//   4. string  — a sequence of Unicode characters
//   5. array   — an ordered sequence of JSON values
//   6. object  — an ordered collection of key/value pairs
//
// The classic approach in dynamic languages (JavaScript, Python, Ruby) is to
// represent these as plain objects at runtime, relying on `typeof` or `class`
// checks. Swift gives us something better: a **tagged union** via `enum`.
//
// What is a tagged union?
// -----------------------
// A tagged union stores BOTH a "tag" (which case we're in) AND the associated
// data in a single value. The Swift compiler then forces us to handle every
// possible tag in a `switch` — we cannot accidentally forget a case. This is
// the foundation of "type safety": the compiler checks our logic for us.
//
// Example (Rust calls this an enum too; Haskell calls it an algebraic data
// type; ML calls it a variant type — same idea, different names):
//
//   let v: JsonValue = .array([.null, .bool(true), .number(42)])
//   switch v {
//   case .null:          print("nothing here")
//   case .bool(let b):  print("boolean: \(b)")
//   case .number(let n): print("number: \(n)")
//   case .string(let s): print("string: \(s)")
//   case .array(let a):  print("array of \(a.count) items")
//   case .object(let o): print("object with \(o.count) keys")
//   }
//
// Why ordered pairs for .object?
// --------------------------------
// RFC 8259 §4 says objects are "unordered" sets of key/value pairs, but in
// practice JSON parsers are expected to preserve insertion order (ECMAScript
// mandates it for JSON.parse). Using `[(key: String, value: JsonValue)]`
// instead of `[String: JsonValue]` (a Dictionary) lets us:
//   - Preserve the original key order from the source JSON
//   - Produce deterministic output when serializing
//   - Round-trip JSON without reordering keys
//
// ============================================================================

// `Sendable` is a Swift 6 protocol that marks a type as safe to share across
// concurrency domains (threads, Tasks, actors). Enums with Sendable payloads
// are automatically Sendable. We mark it explicitly so the compiler can verify
// our conformance and callers can use JsonValue across async boundaries.
public enum JsonValue: Sendable {
    /// Represents JSON `null` — the intentional absence of a value.
    case null

    /// Represents JSON `true` or `false`.
    case bool(Bool)

    /// Represents any JSON number. RFC 8259 does not specify precision, but
    /// IEEE 754 Double (64-bit) is the de-facto standard used by all major
    /// JSON implementations (JavaScript, Python's `json` module, etc.).
    case number(Double)

    /// Represents a JSON string. Escape sequences have already been decoded
    /// by the time a value reaches this type; `\n` in JSON becomes a real
    /// newline character in the Swift String.
    case string(String)

    /// Represents a JSON array — an ordered list of JSON values.
    /// Can contain mixed types: `[1, "hello", null, true]` is valid JSON.
    case array([JsonValue])

    /// Represents a JSON object — an ordered list of key/value pairs.
    ///
    /// We use an array of named tuples instead of a Dictionary because:
    ///   1. Dictionaries in Swift have undefined iteration order.
    ///   2. Many real-world JSON documents (config files, API responses)
    ///      rely on key order for human readability.
    ///   3. A serializer using a Dictionary would produce non-deterministic
    ///      output, making tests fragile.
    case object([(key: String, value: JsonValue)])
}

// ============================================================================
// MARK: — Equatable
// ============================================================================
//
// Equatable lets us write `v1 == v2`. Swift can auto-synthesize this for
// simple enums, but our `object` case uses a tuple array which the compiler
// cannot automatically compare — so we provide the implementation manually.
//
// The comparison is structural: two JsonValues are equal if and only if they
// have the same case AND the same payload. For objects, both the key order
// AND the key/value pairs must match (because we preserve insertion order).
extension JsonValue: Equatable {
    public static func == (lhs: JsonValue, rhs: JsonValue) -> Bool {
        switch (lhs, rhs) {
        case (.null, .null):
            // Both are null — trivially equal.
            return true
        case (.bool(let a), .bool(let b)):
            return a == b
        case (.number(let a), .number(let b)):
            return a == b
        case (.string(let a), .string(let b)):
            return a == b
        case (.array(let a), .array(let b)):
            // [JsonValue] is Equatable because JsonValue is Equatable.
            return a == b
        case (.object(let a), .object(let b)):
            // Tuple arrays are NOT automatically Equatable in Swift, so we
            // compare element-by-element using zip().
            guard a.count == b.count else { return false }
            return zip(a, b).allSatisfy { $0.key == $1.key && $0.value == $1.value }
        default:
            // Different cases are never equal (a number is not a string, etc.)
            return false
        }
    }
}

// ============================================================================
// MARK: — CustomStringConvertible
// ============================================================================
//
// CustomStringConvertible adds a `description` property, which Swift uses
// whenever a value is interpolated into a string or printed. We produce
// compact JSON output (no extra whitespace). For pretty-printing, use the
// JsonSerializer package.
extension JsonValue: CustomStringConvertible {
    public var description: String {
        switch self {
        case .null:
            return "null"
        case .bool(let b):
            return b ? "true" : "false"
        case .number(let n):
            // Integers like 42.0 should render as "42", not "42.0".
            // truncatingRemainder divides by 1 and keeps the fractional part.
            // If there is no fractional part (remainder == 0), it's a whole number.
            if n.truncatingRemainder(dividingBy: 1) == 0 && !n.isInfinite && !n.isNaN {
                return String(Int64(n))
            }
            return String(n)
        case .string(let s):
            // Wrap in quotes. Note: this does NOT escape special characters.
            // For a fully spec-compliant serializer, use the JsonSerializer package.
            return "\"\(s)\""
        case .array(let a):
            return "[" + a.map { $0.description }.joined(separator: ", ") + "]"
        case .object(let o):
            let pairs = o.map { "\"\($0.key)\": \($0.value.description)" }
            return "{" + pairs.joined(separator: ", ") + "}"
        }
    }
}

// ============================================================================
// MARK: — Convenience Constructors and Accessors
// ============================================================================
//
// These helpers make working with JsonValue ergonomic. Instead of:
//   guard case .string(let s) = myValue else { ... }
// you can write:
//   let s = myValue.stringValue
//
// Subscript access lets you traverse JSON like a dictionary or array:
//   let name = response["user"]?["name"]?.stringValue
extension JsonValue {

    // -------------------------------------------------------------------------
    // Factory methods
    // -------------------------------------------------------------------------

    /// Construct a `.bool` without ambiguity with integer literals.
    ///
    /// In Swift, `false` could theoretically be coerced to 0 (an integer) in
    /// some contexts. This factory method makes the intent explicit.
    public static func from(_ b: Bool) -> JsonValue { .bool(b) }

    // -------------------------------------------------------------------------
    // Type-checked accessors
    // -------------------------------------------------------------------------
    // Each accessor returns `nil` when the value is a different case.
    // This mirrors Optional chaining in Swift — callers decide how to handle
    // the nil case (crash with `!`, provide a default with `??`, etc.).

    /// Returns the wrapped String if this is `.string(_)`, otherwise `nil`.
    public var stringValue: String? {
        guard case .string(let s) = self else { return nil }
        return s
    }

    /// Returns the wrapped Double if this is `.number(_)`, otherwise `nil`.
    public var doubleValue: Double? {
        guard case .number(let n) = self else { return nil }
        return n
    }

    /// Returns the wrapped Bool if this is `.bool(_)`, otherwise `nil`.
    public var boolValue: Bool? {
        guard case .bool(let b) = self else { return nil }
        return b
    }

    /// Returns the wrapped array if this is `.array(_)`, otherwise `nil`.
    public var arrayValue: [JsonValue]? {
        guard case .array(let a) = self else { return nil }
        return a
    }

    /// Returns the wrapped pairs if this is `.object(_)`, otherwise `nil`.
    public var objectValue: [(key: String, value: JsonValue)]? {
        guard case .object(let o) = self else { return nil }
        return o
    }

    /// `true` if and only if this is `.null`.
    public var isNull: Bool {
        if case .null = self { return true }
        return false
    }

    // -------------------------------------------------------------------------
    // Subscript access
    // -------------------------------------------------------------------------
    // Subscripts let you write `myJson["key"]` and `myJson[0]`.
    // They return Optional<JsonValue> so callers can chain safely:
    //   myJson["users"]?[0]?["name"]?.stringValue

    /// Look up a key in an `.object`. Returns `nil` if not an object or key
    /// is not present. Uses linear search (O(n)) because JSON objects are
    /// typically small and insertion-order is more important than lookup speed.
    public subscript(key: String) -> JsonValue? {
        guard case .object(let pairs) = self else { return nil }
        return pairs.first(where: { $0.key == key })?.value
    }

    /// Look up an index in an `.array`. Returns `nil` if not an array or
    /// index is out of bounds.
    public subscript(index: Int) -> JsonValue? {
        guard case .array(let a) = self, index >= 0, index < a.count else { return nil }
        return a[index]
    }
}
