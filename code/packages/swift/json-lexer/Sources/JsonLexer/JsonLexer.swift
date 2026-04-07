// JsonLexer.swift
// ============================================================================
// Tokenize JSON text into a flat sequence of typed tokens.
// ============================================================================
//
// What is a lexer (tokenizer)?
// ----------------------------
// A lexer is the first stage of a parser pipeline. It converts a raw stream
// of characters into a stream of *tokens* — the smallest meaningful units
// of the language. Think of it as breaking a sentence into words before you
// try to understand its grammar.
//
// For JSON, the tokens are:
//   { } [ ] : ,         — structural punctuation
//   "hello"             — string literal
//   42  3.14  -1e5      — number literals
//   true  false  null   — keyword literals
//
// Example:
//   Input:   {"name":"Alice","age":30}
//   Tokens:  { "name" : "Alice" , "age" : 30 }
//             ↑ leftBrace
//                ↑ stringLit("name")
//                      ↑ colon
//                         ↑ stringLit("Alice")
//                               ↑ comma
//                                  ↑ stringLit("age")
//                                        ↑ colon
//                                           ↑ numberLit(30)
//                                              ↑ rightBrace
//
// Why separate lexing from parsing?
// -----------------------------------
// It is easier to write each stage when it has a single responsibility:
//   - Lexer:  "What are the atoms?"
//   - Parser: "What do the atoms MEAN?"
// This separation also makes error messages more precise — a lexer error says
// "unexpected character at position 42", while a parser error says "expected
// ':' after object key".
//
// Implementation approach
// -----------------------
// We walk the input as a [Unicode.Scalar] array (not String.Index) for two
// reasons:
//   1. Simpler index arithmetic (i += 1 vs calling index(after:))
//   2. Unicode scalars map 1:1 to JSON's specified character set — JSON
//      strings are defined over Unicode code points, not grapheme clusters.
//
// RFC 8259 defines JSON as Unicode text encoded in UTF-8. All JSON structural
// characters ({}[],:) are ASCII (code points < 128), so comparing Unicode
// scalars works correctly.
// ============================================================================

// ============================================================================
// Token types
// ============================================================================

/// The kind of a JSON token, with an associated value for string and number
/// literals. All other token types carry no additional data.
///
/// Why an enum?
/// Each token type has a distinct "shape" — a string literal has a decoded
/// String payload, a number literal has a Double payload, but a `{` has no
/// payload at all. An enum with associated values models this precisely.
public enum TokenKind: Sendable, Equatable {
    // Structural punctuation — these form the "skeleton" of JSON
    case leftBrace      // {   — begins an object
    case rightBrace     // }   — ends an object
    case leftBracket    // [   — begins an array
    case rightBracket   // ]   — ends an array
    case colon          // :   — separates a key from its value in an object
    case comma          // ,   — separates items in arrays/objects

    // Value-carrying tokens
    case stringLit(String)  // "..." with all escape sequences decoded
    case numberLit(Double)  // integer, decimal, or scientific notation

    // Keyword literals (no additional data needed — the type IS the value)
    case trueLit    // true
    case falseLit   // false
    case nullLit    // null
}

/// A single token produced by the lexer.
///
/// Each token records:
///   - `kind`   — what kind of token this is, with optional payload
///   - `offset` — the byte position in the original source string
///
/// The offset is useful for error messages ("unexpected character at offset 42")
/// and for building source maps (mapping tokens back to their original position).
public struct Token: Sendable, Equatable {
    public let kind: TokenKind
    public let offset: Int  // byte offset in source

    public init(kind: TokenKind, offset: Int) {
        self.kind = kind
        self.offset = offset
    }
}

// ============================================================================
// Error type
// ============================================================================

/// An error produced when the lexer encounters input that does not conform
/// to the JSON specification.
///
/// Examples of lex errors:
///   - An unexpected character like `@` or `#`
///   - A string that is never closed (`"hello` without closing `"`)
///   - An invalid escape sequence like `\q`
///   - A number that starts correctly but is malformed (`1.2.3`)
public struct JsonLexError: Error, Sendable {
    public let message: String
    public let offset: Int  // where in the source the error occurred

    public init(_ message: String, at offset: Int) {
        self.message = message
        self.offset = offset
    }
}

// ============================================================================
// Lexer
// ============================================================================

/// A stateless JSON lexer.
///
/// "Stateless" means the lexer itself holds no mutable state — all state
/// lives in local variables inside `tokenize(_:)`. This makes `JsonLexer`
/// trivially `Sendable` and safe to share across concurrent contexts.
///
/// Usage:
/// ```swift
/// let lexer = JsonLexer()
/// let tokens = try lexer.tokenize("{\"x\": 1}")
/// // → [Token(.leftBrace, 0), Token(.stringLit("x"), 1), ...]
/// ```
public struct JsonLexer: Sendable {
    public init() {}

    /// Tokenize the full JSON input string.
    ///
    /// - Parameter input: A UTF-8 encoded JSON string.
    /// - Returns: An array of `Token` values in source order.
    /// - Throws: `JsonLexError` if any character cannot be tokenized.
    ///
    /// The returned array does NOT include whitespace tokens — whitespace is
    /// consumed silently as a separator, as specified in RFC 8259 §2.
    public func tokenize(_ input: String) throws -> [Token] {
        // Convert to Unicode scalar array for O(1) random access by index.
        // JSON is defined over Unicode code points, so Unicode.Scalar is the
        // right unit. Grapheme clusters (what String.Index operates on) would
        // complicate escape sequence handling.
        let chars = Array(input.unicodeScalars)
        var tokens: [Token] = []
        var i = 0  // current position in the chars array

        while i < chars.count {
            let offset = i      // save start position for token's offset
            let c = chars[i]

            switch c {
            // ----------------------------------------------------------------
            // Whitespace: RFC 8259 defines these four characters as ignorable
            // insignificant whitespace between tokens.
            // ----------------------------------------------------------------
            case " ", "\t", "\n", "\r":
                i += 1  // skip, no token emitted

            // ----------------------------------------------------------------
            // Structural characters — single-char tokens
            // ----------------------------------------------------------------
            case "{":
                tokens.append(Token(kind: .leftBrace, offset: offset))
                i += 1
            case "}":
                tokens.append(Token(kind: .rightBrace, offset: offset))
                i += 1
            case "[":
                tokens.append(Token(kind: .leftBracket, offset: offset))
                i += 1
            case "]":
                tokens.append(Token(kind: .rightBracket, offset: offset))
                i += 1
            case ":":
                tokens.append(Token(kind: .colon, offset: offset))
                i += 1
            case ",":
                tokens.append(Token(kind: .comma, offset: offset))
                i += 1

            // ----------------------------------------------------------------
            // String literal
            // ----------------------------------------------------------------
            case "\"":
                // lexString returns the decoded string content and the index
                // of the character AFTER the closing `"`.
                let (str, end) = try lexString(chars, from: i)
                tokens.append(Token(kind: .stringLit(str), offset: offset))
                i = end

            // ----------------------------------------------------------------
            // Number literal — may start with `-` or a digit
            // ----------------------------------------------------------------
            case "-", "0"..."9":
                let (num, end) = try lexNumber(chars, from: i)
                tokens.append(Token(kind: .numberLit(num), offset: offset))
                i = end

            // ----------------------------------------------------------------
            // Keyword literals — "true", "false", "null"
            //
            // We check character-by-character instead of using String slicing
            // to avoid creating intermediate String objects. This is a minor
            // performance optimization.
            // ----------------------------------------------------------------
            case "t":
                // Must be exactly "true" (4 chars)
                guard i + 3 < chars.count,
                      chars[i+1] == "r", chars[i+2] == "u", chars[i+3] == "e" else {
                    throw JsonLexError("Expected 'true' keyword", at: offset)
                }
                tokens.append(Token(kind: .trueLit, offset: offset))
                i += 4

            case "f":
                // Must be exactly "false" (5 chars)
                guard i + 4 < chars.count,
                      chars[i+1] == "a", chars[i+2] == "l",
                      chars[i+3] == "s", chars[i+4] == "e" else {
                    throw JsonLexError("Expected 'false' keyword", at: offset)
                }
                tokens.append(Token(kind: .falseLit, offset: offset))
                i += 5

            case "n":
                // Must be exactly "null" (4 chars)
                guard i + 3 < chars.count,
                      chars[i+1] == "u", chars[i+2] == "l", chars[i+3] == "l" else {
                    throw JsonLexError("Expected 'null' keyword", at: offset)
                }
                tokens.append(Token(kind: .nullLit, offset: offset))
                i += 4

            default:
                throw JsonLexError("Unexpected character '\(c)'", at: offset)
            }
        }
        return tokens
    }

    // =========================================================================
    // MARK: — String lexing
    // =========================================================================
    //
    // JSON string syntax (RFC 8259 §7):
    //
    //   string = '"' *char '"'
    //   char   = any Unicode code point except " or \
    //          | '\' escape
    //   escape = '"' | '\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u' XXXX
    //
    // The `\uXXXX` form encodes a Unicode BMP code point as 4 hex digits.
    // Surrogate pairs (`\uD800\uDC00`) encode code points above U+FFFF.
    //
    // We decode escape sequences eagerly — the returned String contains the
    // actual Unicode characters, not the raw `\n` etc. sequences.

    private func lexString(
        _ chars: [Unicode.Scalar],
        from start: Int
    ) throws -> (String, Int) {
        // `start` points at the opening `"`.
        // We begin scanning at start+1 (the first content character).
        var i = start + 1
        var result = ""

        while i < chars.count {
            let c = chars[i]

            if c == "\"" {
                // Closing quote — done. Return decoded string and position
                // AFTER the closing `"`.
                return (result, i + 1)
            } else if c == "\\" {
                // Escape sequence. The next character determines the escape.
                guard i + 1 < chars.count else {
                    throw JsonLexError("Unexpected end of input in string escape", at: i)
                }
                let esc = chars[i + 1]
                switch esc {
                case "\"": result.append("\""); i += 2
                case "\\": result.append("\\"); i += 2
                case "/":  result.append("/");  i += 2  // \/ is valid in JSON
                case "b":  result.append("\u{08}"); i += 2  // backspace
                case "f":  result.append("\u{0C}"); i += 2  // form feed
                case "n":  result.append("\n");    i += 2
                case "r":  result.append("\r");    i += 2
                case "t":  result.append("\t");    i += 2
                case "u":
                    // Unicode escape: \uXXXX where XXXX is 4 hex digits.
                    // May be a surrogate pair (two \uXXXX sequences).
                    let (scalar, end) = try lexUnicodeEscape(chars, from: i)
                    result.unicodeScalars.append(scalar)
                    i = end
                default:
                    throw JsonLexError("Invalid escape sequence '\\\\\\(esc)'", at: i)
                }
            } else if c.value < 0x20 {
                // RFC 8259 §7: control characters must be escaped. Raw
                // control characters in a string literal are not valid JSON.
                throw JsonLexError("Unescaped control character in string", at: i)
            } else {
                result.unicodeScalars.append(c)
                i += 1
            }
        }

        // If we reach here, the string was never closed.
        throw JsonLexError("Unterminated string literal", at: start)
    }

    /// Parse a `\uXXXX` escape sequence (and optionally a surrogate pair).
    ///
    /// - Parameter start: index of the `\` character
    /// - Returns: the decoded Unicode scalar and the index after the escape
    ///
    /// Surrogate pairs in JSON:
    /// Code points above U+FFFF cannot be encoded as a single \uXXXX because
    /// XXXX is only 4 hex digits (max U+FFFF). Instead, JSON uses UTF-16
    /// surrogate pairs: two \uXXXX sequences where the first is a "high
    /// surrogate" (U+D800..U+DBFF) and the second is a "low surrogate"
    /// (U+DC00..U+DFFF). Together they encode one supplementary code point.
    ///
    /// Formula: codePoint = 0x10000 + (high - 0xD800) × 0x400 + (low - 0xDC00)
    private func lexUnicodeEscape(
        _ chars: [Unicode.Scalar],
        from start: Int
    ) throws -> (Unicode.Scalar, Int) {
        // start points at `\`; start+1 is `u`; start+2..5 are the hex digits
        guard start + 5 < chars.count else {
            throw JsonLexError("Incomplete \\uXXXX escape", at: start)
        }
        let h1 = try parseHex4(chars, from: start + 2)

        // Check if this is a high surrogate (UTF-16 pair, first half)
        if h1 >= 0xD800 && h1 <= 0xDBFF {
            // Expect a low surrogate immediately after: \uXXXX
            guard start + 11 < chars.count,
                  chars[start + 6] == "\\", chars[start + 7] == "u" else {
                throw JsonLexError("High surrogate must be followed by low surrogate", at: start)
            }
            let h2 = try parseHex4(chars, from: start + 8)
            guard h2 >= 0xDC00 && h2 <= 0xDFFF else {
                throw JsonLexError("Expected low surrogate after high surrogate", at: start + 6)
            }
            // Decode the surrogate pair to a supplementary code point
            let codePoint: UInt32 = 0x10000 + (UInt32(h1 - 0xD800) << 10) + UInt32(h2 - 0xDC00)
            guard let scalar = Unicode.Scalar(codePoint) else {
                throw JsonLexError("Invalid Unicode code point U+\(String(codePoint, radix: 16))", at: start)
            }
            return (scalar, start + 12)
        }

        // Single BMP code point (no surrogate needed)
        guard let scalar = Unicode.Scalar(h1) else {
            throw JsonLexError("Invalid Unicode code point U+\(String(h1, radix: 16))", at: start)
        }
        return (scalar, start + 6)
    }

    /// Parse 4 hex digits starting at `from` and return the 16-bit value.
    private func parseHex4(_ chars: [Unicode.Scalar], from start: Int) throws -> UInt32 {
        guard start + 3 < chars.count else {
            throw JsonLexError("Incomplete hex escape", at: start)
        }
        var value: UInt32 = 0
        for j in 0..<4 {
            let c = chars[start + j]
            guard let digit = hexDigit(c) else {
                throw JsonLexError("Invalid hex digit '\(c)'", at: start + j)
            }
            value = value * 16 + digit
        }
        return value
    }

    /// Convert a single hex character to its numeric value, or nil if invalid.
    private func hexDigit(_ c: Unicode.Scalar) -> UInt32? {
        switch c {
        case "0"..."9": return c.value - 48          // '0' = 48
        case "a"..."f": return c.value - 87          // 'a' = 97, 97-87=10
        case "A"..."F": return c.value - 55          // 'A' = 65, 65-55=10
        default:        return nil
        }
    }

    // =========================================================================
    // MARK: — Number lexing
    // =========================================================================
    //
    // JSON number syntax (RFC 8259 §6):
    //
    //   number = [ '-' ] int [ frac ] [ exp ]
    //   int    = '0' | [1-9] *digit
    //   frac   = '.' 1*digit
    //   exp    = ('e' | 'E') [ '+' | '-' ] 1*digit
    //
    // Leading zeros are NOT allowed (except for `0` itself). So `007` is
    // invalid JSON. Trailing decimals like `1.` are also invalid.
    //
    // We collect the number as a String and then use Double's initializer
    // (which matches the JSON spec for number syntax).

    private func lexNumber(
        _ chars: [Unicode.Scalar],
        from start: Int
    ) throws -> (Double, Int) {
        var i = start
        var raw = ""

        // Optional leading minus sign
        if i < chars.count && chars[i] == "-" {
            raw.append("-")
            i += 1
        }

        // Integer part — must start with a digit
        guard i < chars.count, isDigit(chars[i]) else {
            throw JsonLexError("Expected digit after '-'", at: i)
        }

        if chars[i] == "0" {
            // A leading `0` must NOT be followed by another digit (no octal)
            raw.append("0")
            i += 1
            if i < chars.count && isDigit(chars[i]) {
                throw JsonLexError("Leading zeros are not allowed in JSON numbers", at: start)
            }
        } else {
            // Consume a run of digits [1-9][0-9]*
            while i < chars.count && isDigit(chars[i]) {
                raw.unicodeScalars.append(chars[i])
                i += 1
            }
        }

        // Optional fractional part: '.' digit+
        if i < chars.count && chars[i] == "." {
            raw.append(".")
            i += 1
            guard i < chars.count && isDigit(chars[i]) else {
                throw JsonLexError("Expected digit after decimal point", at: i)
            }
            while i < chars.count && isDigit(chars[i]) {
                raw.unicodeScalars.append(chars[i])
                i += 1
            }
        }

        // Optional exponent part: ('e' | 'E') ['+' | '-'] digit+
        if i < chars.count && (chars[i] == "e" || chars[i] == "E") {
            raw.unicodeScalars.append(chars[i])
            i += 1
            if i < chars.count && (chars[i] == "+" || chars[i] == "-") {
                raw.unicodeScalars.append(chars[i])
                i += 1
            }
            guard i < chars.count && isDigit(chars[i]) else {
                throw JsonLexError("Expected digit in exponent", at: i)
            }
            while i < chars.count && isDigit(chars[i]) {
                raw.unicodeScalars.append(chars[i])
                i += 1
            }
        }

        // Parse the collected string as a Double.
        // Double(_:) accepts exactly the formats we've allowed above.
        guard let value = Double(raw) else {
            throw JsonLexError("Invalid number '\(raw)'", at: start)
        }
        return (value, i)
    }

    /// Returns true if `c` is an ASCII decimal digit (0–9).
    private func isDigit(_ c: Unicode.Scalar) -> Bool {
        c >= "0" && c <= "9"
    }
}
