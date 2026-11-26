// ============================================================================
// Values.swift -- strict decoding of corpus values
// ============================================================================
//
// The corpus encodes typed values as JSON:
//
//     true / false / "text"                    booleans and strings, as is
//     [ v, v, ... ]                            a list
//     {"kind": "int",   "value": "-123"}       an integer, as canonical decimal TEXT
//     {"kind": "bytes", "hex": "00ff"}         bytes, as lowercase even-length hex
//
// Integers are text because JSON numbers lose precision beyond 2^53 in most
// readers, and the corpus holds 128-bit values. Decoding is STRICT: anything
// not exactly one of these shapes is an error (a reported problem), never
// silently skipped, and every size is capped.

/// A decoded corpus value. Equality is kind-sensitive: the integer 1 is not
/// the boolean true, and a byte string is not a list.
public indirect enum Value: Equatable {
    case bool(Bool)
    case int(BigInt)
    case string(String)
    case bytes([UInt8])
    case list([Value])
}

/// A malformed or unevaluable case; the message becomes a problem line.
public struct CaseError: Error {
    public let message: String
    init(_ m: String) { message = clip(m, Values.maxProblemLen) }
}

public enum Values {
    public static let maxSize = 10_000
    public static let maxIntDigits = 300
    public static let maxDepth = 64
    public static let maxProblemLen = 400
    public static let maxFragmentLen = 200

    /// A short rendering of a value for problem messages.
    public static func show(_ v: Any?) -> String {
        guard let v else { return "nil" }
        return clip(Canon.render(v), maxFragmentLen)
    }

    public static func isCanonicalInt(_ text: String) -> Bool {
        let digits = Array(text.utf8)
        let body = digits.first == UInt8(ascii: "-") ? Array(digits.dropFirst()) : digits
        if body.isEmpty || !body.allSatisfy({ $0 >= 0x30 && $0 <= 0x39 }) { return false }
        if body[0] == 0x30 { return text == "0" }
        return true
    }

    public static func isLowercaseHex(_ text: String) -> Bool {
        let b = Array(text.utf8)
        return b.count % 2 == 0 && b.allSatisfy { ($0 >= 0x30 && $0 <= 0x39) || ($0 >= 0x61 && $0 <= 0x66) }
    }

    private static func hexToBytes(_ text: String) -> [UInt8] {
        let b = Array(text.utf8)
        func v(_ c: UInt8) -> UInt8 { c <= 0x39 ? c - 0x30 : c - 0x61 + 10 }
        return stride(from: 0, to: b.count, by: 2).map { v(b[$0]) << 4 | v(b[$0 + 1]) }
    }

    public static func decode(_ node: Json) throws -> Value { try decode(node, 0) }

    // The depth cap lives here too: a runner may be handed a hand-built tree
    // that never went through the parser, and a deeper input is a problem, not
    // a stack overflow.
    private static func decode(_ node: Json, _ depth: Int) throws -> Value {
        if depth > maxDepth { throw CaseError("value nesting deeper than \(maxDepth)") }
        switch node {
        case .bool(let b): return .bool(b)
        case .string(let s): return .string(s)
        case .array(let items):
            if items.count > maxSize { throw CaseError("list exceeds \(maxSize) elements") }
            return .list(try items.map { try decode($0, depth + 1) })
        case .object(let pairs):
            let kind: String? = { if case .string(let k)? = node["kind"] { return k } else { return nil } }()
            if kind == "int" && pairs.count == 2 {
                guard case .string(let t)? = node["value"], isCanonicalInt(t) else {
                    throw CaseError("non-canonical int \(show(string(node["value"])))")
                }
                if t.utf8.count - (t.hasPrefix("-") ? 1 : 0) > maxIntDigits {
                    throw CaseError("int exceeds \(maxIntDigits) digits")
                }
                return .int(try BigInt.parse(t))
            }
            if kind == "bytes" && pairs.count == 2 {
                guard case .string(let t)? = node["hex"], isLowercaseHex(t) else {
                    throw CaseError("bytes hex must be even-length lowercase: \(show(string(node["hex"])))")
                }
                if t.utf8.count > 2 * maxSize { throw CaseError("bytes exceed \(maxSize) bytes") }
                return .bytes(hexToBytes(t))
            }
            throw CaseError("unrecognised value node")
        case .number, .null:
            throw CaseError("unsupported JSON value")
        }
    }

    private static func string(_ j: Json?) -> String? {
        if case .string(let s)? = j { return s }
        return nil
    }
}
