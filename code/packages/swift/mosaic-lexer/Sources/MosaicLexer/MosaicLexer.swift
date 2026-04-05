// MosaicLexer.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MosaicLexer — Hand-written tokenizer for the Mosaic component language
// ============================================================================
//
// Mosaic is a declarative component description language. A .mosaic file
// declares one UI component with typed slots and a visual tree, like:
//
//   component ProfileCard {
//     slot avatar-url: image;
//     slot display-name: text;
//     Column {
//       Text { content: @display-name; }
//     }
//   }
//
// This lexer converts source text into a flat list of Token values.
// It is a single-pass, hand-written scanner — no grammar-tools dependency.
//
// Token taxonomy (in priority order):
//   1. Whitespace + comments  → skipped (not emitted)
//   2. String literals        → STRING
//   3. Dimensions             → DIMENSION (must precede NUMBER)
//   4. Numbers                → NUMBER
//   5. Hex colors             → HEX_COLOR
//   6. Keywords               → COMPONENT, SLOT, IMPORT, FROM, AS, WHEN, EACH, TRUE, FALSE
//   7. Type keywords          → KEYWORD (text, number, bool, image, color, node, list)
//   8. Identifiers            → NAME  (may contain hyphens: e.g., corner-radius)
//   9. Punctuation            → LBRACE RBRACE COLON SEMICOLON AT LANGLE RANGLE COMMA DOT EQUALS
//  10. Unknown                → throws LexError
//
// ============================================================================

// ============================================================================
// Token
// ============================================================================

/// A single token produced by the lexer.
///
/// Every token carries its type label, raw source value, and source position
/// so that error messages in downstream stages can point back to the original
/// source text.
///
/// Example:
///   source "16dp"  → Token(type: "DIMENSION", value: "16dp",  line: 3, column: 12)
///   source "@title"→ Token(type: "AT",        value: "@",      line: 5, column: 5)
///              then  Token(type: "NAME",       value: "title",  line: 5, column: 6)
public struct Token: Equatable, CustomStringConvertible {
    /// The token category. One of the string constants described in the taxonomy above.
    public let type: String
    /// The raw substring from the source that was matched.
    public let value: String
    /// 1-based line number.
    public let line: Int
    /// 1-based column number (byte offset, not Unicode codepoint).
    public let column: Int

    public init(type: String, value: String, line: Int, column: Int) {
        self.type = type
        self.value = value
        self.line = line
        self.column = column
    }

    public var description: String {
        "\(type)(\(value.debugDescription)) at \(line):\(column)"
    }
}

// ============================================================================
// LexError
// ============================================================================

/// Thrown when the lexer encounters a character it cannot classify.
public struct LexError: Error, CustomStringConvertible {
    public let message: String
    public let line: Int
    public let column: Int

    public var description: String { "LexError at \(line):\(column): \(message)" }
}

// ============================================================================
// Keyword sets
// ============================================================================

/// Reserved words that become their own token type rather than NAME.
/// These are the structural keywords of the language.
private let structuralKeywords: Set<String> = [
    "component", "slot", "import", "from", "as", "when", "each", "true", "false",
]

/// Type keywords that remain KEYWORD tokens (used for slot types and property
/// values that happen to share names with type annotations).
private let typeKeywords: Set<String> = [
    "text", "number", "bool", "image", "color", "node", "list",
]

// ============================================================================
// Tokenizer — main entry point
// ============================================================================

/// Tokenize a Mosaic source string.
///
/// This is the primary public API of this module. Whitespace and comments are
/// consumed but never emitted. All other source text becomes a Token.
///
/// - Parameter source: Full contents of a `.mosaic` file.
/// - Returns: Ordered array of tokens from first to last in the source.
/// - Throws: `LexError` if an unexpected character is encountered.
///
/// Example:
///
///     let tokens = try tokenize("component Card { }")
///     // → [Token("COMPONENT","component",1,1), Token("NAME","Card",1,11),
///     //    Token("LBRACE","{",1,16), Token("RBRACE","}",1,18)]
///
public func tokenize(_ source: String) throws -> [Token] {
    var scanner = Scanner(source: source)
    return try scanner.scanAll()
}

// ============================================================================
// Scanner — internal state machine
// ============================================================================

/// The scanner maintains a cursor into the source string and emits tokens
/// one at a time when `scanAll()` is called.
///
/// Design notes:
///   - All characters are processed via Unicode scalar values so that the
///     column counter is byte-accurate (Swift `String.Index` advances by
///     Unicode scalar, not byte, but for ASCII-dominant Mosaic source the
///     difference is negligible).
///   - Hyphens inside identifiers are only allowed when the preceding char
///     was alphanumeric and the following char is also alphanumeric/underscore.
///     This prevents `-` from being consumed into a NAME when it starts a
///     negative number.
private struct Scanner {
    private let chars: [Character]
    private var pos: Int = 0
    private var line: Int = 1
    private var column: Int = 1

    init(source: String) {
        self.chars = Array(source)
    }

    // -------------------------------------------------------------------------
    // Main scan loop
    // -------------------------------------------------------------------------

    mutating func scanAll() throws -> [Token] {
        var tokens: [Token] = []
        while !isAtEnd() {
            if let tok = try nextToken() {
                tokens.append(tok)
            }
        }
        return tokens
    }

    // -------------------------------------------------------------------------
    // Single-token dispatch
    // -------------------------------------------------------------------------

    /// Consume one logical unit from the source, returning a Token or nil
    /// (for skipped whitespace/comments).
    mutating func nextToken() throws -> Token? {
        skipWhitespaceAndComments()
        guard !isAtEnd() else { return nil }

        let startLine = line
        let startCol = column
        let c = current()

        // String literal "..."
        if c == "\"" {
            let s = try scanString(startLine: startLine, startCol: startCol)
            return Token(type: "STRING", value: s, line: startLine, column: startCol)
        }

        // Hex color #...
        if c == "#" {
            let s = try scanHexColor(startLine: startLine, startCol: startCol)
            return Token(type: "HEX_COLOR", value: s, line: startLine, column: startCol)
        }

        // Number or dimension (possibly negative)
        // "-" only starts a number if followed by a digit or decimal point
        if isDigit(c) || (c == "-" && posAhead(1) != nil && (isDigit(posAhead(1)!) || posAhead(1)! == ".")) {
            return try scanNumberOrDimension(startLine: startLine, startCol: startCol)
        }

        // Single-character punctuation
        if let tok = scanPunctuation(c, line: startLine, col: startCol) {
            advance()
            return tok
        }

        // Identifiers and keywords (start with letter or underscore)
        if isAlpha(c) {
            let ident = scanIdent()
            let type = classifyIdent(ident)
            return Token(type: type, value: ident, line: startLine, column: startCol)
        }

        throw LexError(
            message: "Unexpected character '\(c)' (U+\(c.asciiValue.map { String($0, radix: 16) } ?? "?"))",
            line: startLine, column: startCol
        )
    }

    // -------------------------------------------------------------------------
    // Skip whitespace and // line comments
    // -------------------------------------------------------------------------

    mutating func skipWhitespaceAndComments() {
        while !isAtEnd() {
            let c = current()
            if c == " " || c == "\t" || c == "\r" || c == "\n" {
                advance()
            } else if c == "/" && posAhead(1) == "/" {
                // Line comment: consume until end of line
                while !isAtEnd() && current() != "\n" {
                    advance()
                }
            } else if c == "/" && posAhead(1) == "*" {
                // Block comment: consume until */
                advance(); advance() // consume /*
                while !isAtEnd() {
                    if current() == "*" && posAhead(1) == "/" {
                        advance(); advance() // consume */
                        break
                    }
                    advance()
                }
            } else {
                break
            }
        }
    }

    // -------------------------------------------------------------------------
    // String literal "..."
    // -------------------------------------------------------------------------

    /// Scans a double-quoted string, handling standard escape sequences.
    /// Returns the full raw token value including the surrounding quotes.
    mutating func scanString(startLine: Int, startCol: Int) throws -> String {
        var result = "\""
        advance() // consume opening "
        while !isAtEnd() && current() != "\"" {
            let c = current()
            if c == "\\" {
                advance()
                guard !isAtEnd() else {
                    throw LexError(message: "Unterminated string escape", line: startLine, column: startCol)
                }
                result += "\\\(current())"
                advance()
            } else if c == "\n" {
                throw LexError(message: "Unterminated string literal (newline in string)", line: startLine, column: startCol)
            } else {
                result.append(c)
                advance()
            }
        }
        guard !isAtEnd() else {
            throw LexError(message: "Unterminated string literal", line: startLine, column: startCol)
        }
        result += "\""
        advance() // consume closing "
        return result
    }

    // -------------------------------------------------------------------------
    // Hex color #rrggbb / #rgb / #rrggbbaa
    // -------------------------------------------------------------------------

    mutating func scanHexColor(startLine: Int, startCol: Int) throws -> String {
        var result = "#"
        advance() // consume #
        while !isAtEnd() && isHexDigit(current()) {
            result.append(current())
            advance()
        }
        let hexPart = String(result.dropFirst())
        guard hexPart.count == 3 || hexPart.count == 6 || hexPart.count == 8 else {
            throw LexError(
                message: "Invalid hex color '\(result)': expected 3, 6, or 8 hex digits",
                line: startLine, column: startCol
            )
        }
        return result
    }

    // -------------------------------------------------------------------------
    // Number / Dimension: -?[0-9]*\.?[0-9]+([a-zA-Z%]+)?
    // -------------------------------------------------------------------------

    mutating func scanNumberOrDimension(startLine: Int, startCol: Int) throws -> Token {
        var raw = ""
        // Optional leading minus
        if current() == "-" {
            raw.append("-")
            advance()
        }
        // Integer digits
        while !isAtEnd() && isDigit(current()) {
            raw.append(current())
            advance()
        }
        // Optional decimal part
        if !isAtEnd() && current() == "." && posAhead(1) != nil && isDigit(posAhead(1)!) {
            raw.append(".")
            advance()
            while !isAtEnd() && isDigit(current()) {
                raw.append(current())
                advance()
            }
        }
        // Optional unit suffix → DIMENSION
        if !isAtEnd() && (isAlpha(current()) || current() == "%") {
            while !isAtEnd() && (isAlphaNumeric(current()) || current() == "%") {
                raw.append(current())
                advance()
            }
            return Token(type: "DIMENSION", value: raw, line: startLine, column: startCol)
        }
        return Token(type: "NUMBER", value: raw, line: startLine, column: startCol)
    }

    // -------------------------------------------------------------------------
    // Identifier: [a-zA-Z_][a-zA-Z0-9_-]*
    // Hyphens are allowed mid-ident for CSS-style names like corner-radius
    // -------------------------------------------------------------------------

    mutating func scanIdent() -> String {
        var result = ""
        while !isAtEnd() {
            let c = current()
            if isAlphaNumeric(c) {
                result.append(c)
                advance()
            } else if c == "-" {
                // Allow hyphen only when surrounded by alphanumeric chars
                let next = posAhead(1)
                if let n = next, isAlphaNumeric(n) {
                    result.append(c)
                    advance()
                } else {
                    break
                }
            } else {
                break
            }
        }
        return result
    }

    /// Classify an identifier as a keyword token type or NAME.
    func classifyIdent(_ s: String) -> String {
        switch s {
        case "component": return "COMPONENT"
        case "slot":      return "SLOT"
        case "import":    return "IMPORT"
        case "from":      return "FROM"
        case "as":        return "AS"
        case "when":      return "WHEN"
        case "each":      return "EACH"
        case "true":      return "TRUE"
        case "false":     return "FALSE"
        default:
            if typeKeywords.contains(s) { return "KEYWORD" }
            return "NAME"
        }
    }

    // -------------------------------------------------------------------------
    // Punctuation
    // -------------------------------------------------------------------------

    func scanPunctuation(_ c: Character, line: Int, col: Int) -> Token? {
        switch c {
        case "{": return Token(type: "LBRACE",    value: "{", line: line, column: col)
        case "}": return Token(type: "RBRACE",    value: "}", line: line, column: col)
        case ":": return Token(type: "COLON",     value: ":", line: line, column: col)
        case ";": return Token(type: "SEMICOLON", value: ";", line: line, column: col)
        case "@": return Token(type: "AT",        value: "@", line: line, column: col)
        case "<": return Token(type: "LANGLE",    value: "<", line: line, column: col)
        case ">": return Token(type: "RANGLE",    value: ">", line: line, column: col)
        case ",": return Token(type: "COMMA",     value: ",", line: line, column: col)
        case ".": return Token(type: "DOT",       value: ".", line: line, column: col)
        case "=": return Token(type: "EQUALS",    value: "=", line: line, column: col)
        default:  return nil
        }
    }

    // -------------------------------------------------------------------------
    // Character utilities
    // -------------------------------------------------------------------------

    func isAtEnd() -> Bool { pos >= chars.count }
    func current() -> Character { chars[pos] }

    func posAhead(_ n: Int) -> Character? {
        let i = pos + n
        return i < chars.count ? chars[i] : nil
    }

    mutating func advance() {
        if !isAtEnd() {
            if chars[pos] == "\n" {
                line += 1
                column = 1
            } else {
                column += 1
            }
            pos += 1
        }
    }

    func isDigit(_ c: Character) -> Bool { c >= "0" && c <= "9" }
    func isAlpha(_ c: Character) -> Bool {
        (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c == "_"
    }
    func isAlphaNumeric(_ c: Character) -> Bool { isAlpha(c) || isDigit(c) }
    func isHexDigit(_ c: Character) -> Bool {
        isDigit(c) || (c >= "a" && c <= "f") || (c >= "A" && c <= "F")
    }
}
