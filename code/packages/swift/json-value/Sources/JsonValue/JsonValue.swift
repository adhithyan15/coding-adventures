import Foundation

public enum JsonValueError: Error {
    case message(String)
}

/// A discriminated union representing the six JSON value types.
///
/// JSON (RFC 8259) defines exactly six value types:
///   1. Object  -- an ordered collection of key-value pairs
///   2. Array   -- an ordered sequence of values
///   3. String  -- a sequence of Unicode characters
///   4. Number  -- an integer or floating-point number
///   5. Boolean -- true or false
///   6. Null    -- the absence of a value
public enum JsonValue: Equatable {
    case object([String: JsonValue])
    case array([JsonValue])
    case string(String)
    case number(Double)
    case bool(Bool)
    case null

    /// Dictionary-like subscript access for object types.
    /// Returns the value associated with the key, or nil if the value is not an object or the key is missing.
    public subscript(key: String) -> JsonValue? {
        if case .object(let dict) = self {
            return dict[key]
        }
        return nil
    }

    /// Array-like subscript access for array types.
    /// Returns the element at the index, or nil if the value is not an array or index is out of bounds.
    public subscript(index: Int) -> JsonValue? {
        if case .array(let arr) = self {
            guard index >= 0 && index < arr.count else { return nil }
            return arr[index]
        }
        return nil
    }
}
