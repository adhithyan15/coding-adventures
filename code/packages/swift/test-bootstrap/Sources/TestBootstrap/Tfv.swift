// ============================================================================
// Tfv.swift -- a strict validator for TFV v1 text (TF01 section 3)
// ============================================================================
//
// The corpus's `corpus_reject` cases carry malformed TFV documents that every
// lane's reader must REFUSE. A lane that skipped them would never learn its
// reader is too lenient, so this file is a real validator: it parses every
// token, enforces the grammar, and rejects duplicates.
//
// FORMAT (one record per line, space-separated tokens):
//
//     tfv 1                                    header, exactly this
//     <id> R<n> K<name> <value> ...            a record of n named fields
//
// Values: `N` null, `T`/`F` booleans, `I<int>` canonical decimal, `D<16 hex>` a
// float's bits, `X<hex>` bytes, `S<hex>` UTF-8 text, `L<n> v...` a list,
// `M<n> k v ...` a map. Hex is LOWERCASE and even-length. Blank lines and `#`
// comment lines are skipped whole. Duplicate record ids, field names and map
// keys are errors.

public struct TfvError: Error, Equatable {
    public let message: String
    init(_ m: String) { message = "invalid TFV: " + m }
}

public enum Tfv {
    public static let header = "tfv 1"
    public static let maxDepth = 64
    public static let maxIntDigits = 1000
    public static let maxInput = 1 << 20

    private static func q(_ s: String) -> String {
        let shown = s.count > 64 ? String(s.prefix(64)) + "..." : s
        return "\"" + shown.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"") + "\""
    }

    /// True when `text` is a valid TFV document.
    public static func isValid(_ text: String) -> Bool {
        (try? validate(text)) != nil
    }

    public static func validate(_ text: String) throws {
        let bytes = Array(text.utf8)
        if bytes.count > maxInput { throw TfvError("input of \(bytes.count) bytes exceeds \(maxInput)") }
        if !StrictJson.isValidUTF8(bytes) { throw TfvError("document is not valid UTF-8") }
        var lines = bytes.split(separator: 0x0A, omittingEmptySubsequences: false).map { Array($0) }
        if let last = lines.last, last.isEmpty { lines.removeLast() }   // the final newline is optional
        guard let first = lines.first, String(decoding: first, as: UTF8.self) == header else {
            throw TfvError("the first line must be exactly \(q(header))")
        }
        var seenIds = Set<String>()
        for n in 1..<max(lines.count, 1) {
            let line = lines[n]
            if line.isEmpty || line[0] == UInt8(ascii: "#") { continue }
            let id: String
            do {
                id = try validateRecord(line)
            } catch let e as TfvError {
                throw TfvError("line \(n + 1): " + e.message.dropFirst("invalid TFV: ".count))
            }
            if !seenIds.insert(id).inserted { throw TfvError("line \(n + 1): duplicate record id \(q(id))") }
        }
    }

    private static func isIdChar(_ c: UInt8) -> Bool {
        (c >= 0x61 && c <= 0x7A) || (c >= 0x41 && c <= 0x5A) || (c >= 0x30 && c <= 0x39)
            || c == UInt8(ascii: ".") || c == UInt8(ascii: "_") || c == UInt8(ascii: "/") || c == UInt8(ascii: "-")
    }

    private static func validFieldName(_ s: [UInt8]) -> Bool {
        guard let f = s.first, f >= 0x61, f <= 0x7A else { return false }
        return s.dropFirst().allSatisfy { ($0 >= 0x61 && $0 <= 0x7A) || ($0 >= 0x30 && $0 <= 0x39) || $0 == UInt8(ascii: "_") }
    }

    private struct Reader {
        let tokens: [[UInt8]]
        var pos: Int

        mutating func next() throws -> [UInt8] {
            if pos >= tokens.count { throw TfvError("record ends early") }
            pos += 1
            return tokens[pos - 1]
        }

        /// Returns a deep-equality key: equal for two values exactly when TF01
        /// section 3.1 calls them deep-equal.
        mutating func value(_ depth: Int) throws -> String {
            let tok = try next()
            let tag = tok[0]
            let body = Array(tok.dropFirst())
            let text = String(decoding: tok, as: UTF8.self)
            switch tag {
            case UInt8(ascii: "N"), UInt8(ascii: "T"), UInt8(ascii: "F"):
                if !body.isEmpty { throw TfvError("unexpected body in \(q(text))") }
                return String(UnicodeScalar(tag))
            case UInt8(ascii: "I"):
                try validInt(body)
                return text
            case UInt8(ascii: "D"):
                guard body.count == 16, let raw = lowerHexBytes(body) else {
                    throw TfvError("float needs exactly 16 lowercase hex digits: \(q(text))")
                }
                // Every NaN equals every NaN; -0 and +0 stay distinct.
                let exp = (Int(raw[0] & 0x7f) << 4) | Int(raw[1] >> 4)
                var mantissaZero = (raw[1] & 0x0f) == 0
                for i in 2..<raw.count { mantissaZero = mantissaZero && raw[i] == 0 }
                return exp == 0x7ff && !mantissaZero ? "Dnan" : text
            case UInt8(ascii: "X"):
                if lowerHexBytes(body) == nil { throw TfvError("bytes need even-length lowercase hex: \(q(text))") }
                return text
            case UInt8(ascii: "S"):
                guard let raw = lowerHexBytes(body) else {
                    throw TfvError("text needs even-length lowercase hex: \(q(text))")
                }
                if !StrictJson.isValidUTF8(raw) { throw TfvError("text is not strict UTF-8: \(q(text))") }
                return text
            case UInt8(ascii: "L"), UInt8(ascii: "M"):
                return try container(tag, body, depth)
            default:
                throw TfvError("unknown tag in \(q(text))")
            }
        }

        mutating func container(_ tag: UInt8, _ body: [UInt8], _ depth: Int) throws -> String {
            if depth + 1 > Tfv.maxDepth { throw TfvError("nesting deeper than \(Tfv.maxDepth)") }
            let count = try canonicalNat(body)
            if tag == UInt8(ascii: "L") {
                var b = "L["
                for _ in 0..<count { b += try value(depth + 1) + "," }
                return b + "]"
            }
            var keys = Set<String>()
            var entries: [String] = []
            for _ in 0..<count {
                let k = try value(depth + 1)
                let v = try value(depth + 1)
                if !keys.insert(k).inserted { throw TfvError("duplicate map key") }
                entries.append("(\(k)=>\(v))")
            }
            return "M{" + entries.sorted().joined() + "}"
        }
    }

    private static func validateRecord(_ line: [UInt8]) throws -> String {
        if line.contains(0x0D) { throw TfvError("carriage return in a record line") }
        let tokens = line.split(separator: 0x20, omittingEmptySubsequences: false).map { Array($0) }
        for t in tokens where t.isEmpty { throw TfvError("leading, trailing or doubled space") }
        if tokens.count < 2 { throw TfvError("a record needs an id and R<n>") }
        let id = tokens[0]
        if !id.allSatisfy(isIdChar) { throw TfvError("bad record id \(q(String(decoding: id, as: UTF8.self)))") }
        if tokens[1][0] != UInt8(ascii: "R") {
            throw TfvError("expected R<n>, got \(q(String(decoding: tokens[1], as: UTF8.self)))")
        }
        let count = try canonicalNat(Array(tokens[1].dropFirst()))
        var r = Reader(tokens: tokens, pos: 2)
        var seen = Set<[UInt8]>()
        for _ in 0..<count {
            let keyTok = try r.next()
            if keyTok[0] != UInt8(ascii: "K") || !validFieldName(Array(keyTok.dropFirst())) {
                throw TfvError("bad field key \(q(String(decoding: keyTok, as: UTF8.self)))")
            }
            if !seen.insert(keyTok).inserted {
                throw TfvError("duplicate field \(q(String(decoding: keyTok.dropFirst(), as: UTF8.self)))")
            }
            _ = try r.value(0)
        }
        if r.pos != tokens.count { throw TfvError("trailing tokens after the last field") }
        return String(decoding: id, as: UTF8.self)
    }

    /// 0 or digits without a leading zero. A count too large for the tokens
    /// present is rejected later as "record ends early"; no allocation depends on it.
    private static func canonicalNat(_ s: [UInt8]) throws -> Int {
        let text = String(decoding: s, as: UTF8.self)
        if s.isEmpty { throw TfvError("empty count") }
        if !s.allSatisfy({ $0 >= 0x30 && $0 <= 0x39 }) { throw TfvError("non-digit in count \(q(text))") }
        if s.count > 1 && s[0] == 0x30 { throw TfvError("leading zero in count \(q(text))") }
        if s.count > 9 { throw TfvError("count \(q(text)) too large") }
        return Int(text)!
    }

    private static func lowerHexBytes(_ s: [UInt8]) -> [UInt8]? {
        if s.count % 2 != 0 { return nil }
        var out: [UInt8] = []
        var i = 0
        while i < s.count {
            guard let hi = hexVal(s[i]), let lo = hexVal(s[i + 1]) else { return nil }
            out.append(hi << 4 | lo)
            i += 2
        }
        return out
    }

    private static func hexVal(_ c: UInt8) -> UInt8? {
        if c >= 0x30 && c <= 0x39 { return c - 0x30 }
        if c >= 0x61 && c <= 0x66 { return c - 0x61 + 10 }
        return nil
    }

    private static func validInt(_ body: [UInt8]) throws {
        let neg = body.first == UInt8(ascii: "-")
        let digits = neg ? Array(body.dropFirst()) : body
        let text = String(decoding: body, as: UTF8.self)
        if digits.isEmpty { throw TfvError("empty integer") }
        if !digits.allSatisfy({ $0 >= 0x30 && $0 <= 0x39 }) { throw TfvError("non-digit in integer \(q(text))") }
        if digits[0] == 0x30 && (digits.count > 1 || neg) { throw TfvError("non-canonical integer \(q(text))") }
        if digits.count > maxIntDigits { throw TfvError("integer of \(digits.count) digits exceeds \(maxIntDigits)") }
    }
}

private extension String {
    /// A Foundation-free `replacingOccurrences` for the two single-character quotes.
    func replacingOccurrences(of target: String, with replacement: String) -> String {
        var out = ""
        for ch in self {
            out += String(ch) == target ? replacement : String(ch)
        }
        return out
    }
}
