// ============================================================================
// StrictJson.swift -- a hand-written, strict JSON reader for the corpus
// ============================================================================
//
// WHY NOT `JSONSerialization`?  Foundation's parser accepts duplicate object
// keys (the last silently wins) and converts numbers to `Double`/`NSNumber`, so
// a 128-bit corpus integer would lose precision. A conformance corpus is a
// security-sensitive input: an attacker who can slip a duplicate key past the
// reader can make the engine "pass" a corpus the other lanes reject. So we parse
// ourselves, strictly:
//
//   * at most `maxBytes` (8 MiB) of input, `maxDepth` (64) levels of nesting and
//     `maxNodes` (200,000) values (8 MiB of `0,` would otherwise be ~4 million
//     heap nodes);
//   * the document must be valid UTF-8: overlong forms, surrogate code points
//     and values above U+10FFFF are refused;
//   * no duplicate object keys, no trailing data, no leading zeros, no raw
//     control characters in strings, no lone surrogate escapes;
//   * numbers are kept as TEXT (`.number("123")`), never converted.

public struct JsonError: Error, Equatable {
    public let message: String
    init(_ message: String) { self.message = message }
}

public indirect enum Json {
    case null
    case bool(Bool)
    case number(String)
    case string(String)
    case array([Json])
    case object([(key: String, value: Json)])

    /// The value of `key` when this is an object that has it. Keys are compared
    /// by their Unicode SCALARS (bytes), not by Swift's canonical equivalence:
    /// "\u{e9}" and "e\u{301}" are different keys, exactly as in every other lane.
    public subscript(key: String) -> Json? {
        guard case .object(let pairs) = self else { return nil }
        return pairs.first { $0.key.unicodeScalars.elementsEqual(key.unicodeScalars) }?.value
    }
}

public enum StrictJson {
    public static let maxDepth = 64
    public static let maxNodes = 200_000
    public static let maxBytes = 8 << 20

    /// Parses a complete document, or throws `JsonError`.
    public static func parse(_ data: [UInt8]) throws -> Json {
        if data.count > maxBytes { throw JsonError("\(data.count) bytes exceeds \(maxBytes)") }
        if !isValidUTF8(data) { throw JsonError("document is not valid UTF-8") }
        var r = Reader(data)
        return try r.document()
    }

    /// Strict UTF-8 validation (RFC 3629): shortest forms only, no surrogates,
    /// nothing above U+10FFFF.
    public static func isValidUTF8(_ b: [UInt8]) -> Bool {
        var i = 0
        func cont(_ at: Int, _ lo: UInt8 = 0x80, _ hi: UInt8 = 0xBF) -> Bool {
            at < b.count && b[at] >= lo && b[at] <= hi
        }
        while i < b.count {
            let c = b[i]
            switch c {
            case 0x00...0x7F: i += 1
            case 0xC2...0xDF:
                if !cont(i + 1) { return false }
                i += 2
            case 0xE0:
                if !cont(i + 1, 0xA0) || !cont(i + 2) { return false }
                i += 3
            case 0xE1...0xEC, 0xEE...0xEF:
                if !cont(i + 1) || !cont(i + 2) { return false }
                i += 3
            case 0xED:
                if !cont(i + 1, 0x80, 0x9F) || !cont(i + 2) { return false }
                i += 3
            case 0xF0:
                if !cont(i + 1, 0x90) || !cont(i + 2) || !cont(i + 3) { return false }
                i += 4
            case 0xF1...0xF3:
                if !cont(i + 1) || !cont(i + 2) || !cont(i + 3) { return false }
                i += 4
            case 0xF4:
                if !cont(i + 1, 0x80, 0x8F) || !cont(i + 2) || !cont(i + 3) { return false }
                i += 4
            default: return false
            }
        }
        return true
    }

    private struct Reader {
        let s: [UInt8]
        var pos = 0
        var nodes = 0

        init(_ s: [UInt8]) { self.s = s }

        mutating func document() throws -> Json {
            skipWs()
            let v = try value(0)
            skipWs()
            if pos != s.count { throw JsonError("trailing data after the document") }
            return v
        }

        mutating func skipWs() {
            while pos < s.count, s[pos] == 0x20 || s[pos] == 0x09 || s[pos] == 0x0A || s[pos] == 0x0D { pos += 1 }
        }

        func err(_ what: String) -> JsonError { JsonError("\(what) at offset \(pos)") }

        func peek() throws -> UInt8 {
            if pos >= s.count { throw err("unexpected end of input") }
            return s[pos]
        }

        mutating func value(_ depth: Int) throws -> Json {
            nodes += 1
            if nodes > StrictJson.maxNodes { throw JsonError("more than \(StrictJson.maxNodes) values in one document") }
            let c = try peek()
            switch c {
            case UInt8(ascii: "{"), UInt8(ascii: "["):
                // Exactly `maxDepth` (64) nested containers are accepted and a
                // 65th is refused (the C# lane's rule). `depth` counts the
                // containers already open, so scalars at depth 64 are fine.
                if depth >= StrictJson.maxDepth { throw JsonError("nesting deeper than \(StrictJson.maxDepth)") }
                return c == UInt8(ascii: "{") ? try object(depth) : try array(depth)
            case UInt8(ascii: "\""): return .string(try string())
            case UInt8(ascii: "t"): try literal("true"); return .bool(true)
            case UInt8(ascii: "f"): try literal("false"); return .bool(false)
            case UInt8(ascii: "n"): try literal("null"); return .null
            default:
                if c == UInt8(ascii: "-") || (c >= 0x30 && c <= 0x39) { return try number() }
                throw err("unexpected character")
            }
        }

        mutating func literal(_ word: String) throws {
            let w = Array(word.utf8)
            if pos + w.count > s.count || Array(s[pos..<pos + w.count]) != w { throw err("bad literal") }
            pos += w.count
        }

        mutating func object(_ depth: Int) throws -> Json {
            pos += 1
            var out: [(key: String, value: Json)] = []
            var seen = Set<[UInt8]>()   // byte-exact: NFC and NFD spellings are distinct keys
            skipWs()
            if try peek() == UInt8(ascii: "}") { pos += 1; return .object(out) }
            while true {
                skipWs()
                if try peek() != UInt8(ascii: "\"") { throw err("object key is not a string") }
                let key = try string()
                if !seen.insert(Array(key.utf8)).inserted { throw JsonError("duplicate JSON key \"\(clip(key, 64))\"") }
                skipWs()
                if try peek() != UInt8(ascii: ":") { throw err("expected ':'") }
                pos += 1
                skipWs()
                out.append((key, try value(depth + 1)))
                skipWs()
                let c = try peek()
                pos += 1
                if c == UInt8(ascii: "}") { return .object(out) }
                if c != UInt8(ascii: ",") { throw err("expected ',' or '}'") }
            }
        }

        mutating func array(_ depth: Int) throws -> Json {
            pos += 1
            var out: [Json] = []
            skipWs()
            if try peek() == UInt8(ascii: "]") { pos += 1; return .array(out) }
            while true {
                skipWs()
                out.append(try value(depth + 1))
                skipWs()
                let c = try peek()
                pos += 1
                if c == UInt8(ascii: "]") { return .array(out) }
                if c != UInt8(ascii: ",") { throw err("expected ',' or ']'") }
            }
        }

        mutating func hex4() throws -> UInt32 {
            if pos + 4 > s.count { throw err("short \\u escape") }
            var code: UInt32 = 0
            for i in 0..<4 {
                let d = s[pos + i]
                let v: UInt32
                switch d {
                case 0x30...0x39: v = UInt32(d - 0x30)
                case 0x61...0x66: v = UInt32(d - 0x61 + 10)
                case 0x41...0x46: v = UInt32(d - 0x41 + 10)
                default: throw err("bad \\u escape")
                }
                code = code * 16 + v
            }
            pos += 4
            return code
        }

        mutating func string() throws -> String {
            pos += 1   // opening quote
            var buf: [UInt8] = []
            while true {
                if pos >= s.count { throw err("unterminated string") }
                let c = s[pos]
                pos += 1
                if c == UInt8(ascii: "\"") { return String(decoding: buf, as: UTF8.self) }
                if c < 0x20 { throw err("raw control character in string") }
                if c != UInt8(ascii: "\\") { buf.append(c); continue }
                if pos >= s.count { throw err("unterminated escape") }
                let e = s[pos]
                pos += 1
                switch e {
                case UInt8(ascii: "\""), UInt8(ascii: "\\"), UInt8(ascii: "/"): buf.append(e)
                case UInt8(ascii: "b"): buf.append(0x08)
                case UInt8(ascii: "f"): buf.append(0x0C)
                case UInt8(ascii: "n"): buf.append(0x0A)
                case UInt8(ascii: "r"): buf.append(0x0D)
                case UInt8(ascii: "t"): buf.append(0x09)
                case UInt8(ascii: "u"):
                    var code = try hex4()
                    if code >= 0xDC00 && code <= 0xDFFF { throw err("lone low surrogate") }
                    if code >= 0xD800 && code <= 0xDBFF {
                        // A high surrogate must be followed by \uDC00-\uDFFF.
                        if pos + 2 > s.count || s[pos] != UInt8(ascii: "\\") || s[pos + 1] != UInt8(ascii: "u") {
                            throw err("lone high surrogate")
                        }
                        pos += 2
                        let low = try hex4()
                        if low < 0xDC00 || low > 0xDFFF { throw err("bad surrogate pair") }
                        code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00)
                    }
                    buf.append(contentsOf: Array(String(Unicode.Scalar(code)!).utf8))
                default: throw err("bad escape")
                }
            }
        }

        mutating func digits() throws {
            let begin = pos
            while pos < s.count, s[pos] >= 0x30, s[pos] <= 0x39 { pos += 1 }
            if pos == begin { throw err("digits expected") }
        }

        mutating func number() throws -> Json {
            let start = pos
            if try peek() == UInt8(ascii: "-") { pos += 1 }
            let c = try peek()
            if c == 0x30 {
                pos += 1
            } else if c >= 0x31 && c <= 0x39 {
                while pos < s.count, s[pos] >= 0x30, s[pos] <= 0x39 { pos += 1 }
            } else {
                throw err("bad number")
            }
            if pos < s.count, s[pos] == UInt8(ascii: ".") { pos += 1; try digits() }
            if pos < s.count, s[pos] == UInt8(ascii: "e") || s[pos] == UInt8(ascii: "E") {
                pos += 1
                if pos < s.count, s[pos] == UInt8(ascii: "+") || s[pos] == UInt8(ascii: "-") { pos += 1 }
                try digits()
            }
            return .number(String(decoding: s[start..<pos], as: UTF8.self))
        }
    }
}
