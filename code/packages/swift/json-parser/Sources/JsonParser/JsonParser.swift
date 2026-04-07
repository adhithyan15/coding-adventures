// JsonParser.swift
// ============================================================================
// Recursive-descent parser: [Token] → JsonValue
// ============================================================================
//
// What is a parser?
// -----------------
// A parser takes a flat sequence of tokens (produced by the lexer) and builds
// a tree (or in JSON's case, a nested value). The tree reflects the structure
// of the input — which values are nested inside which objects or arrays.
//
// What is a recursive-descent parser?
// ------------------------------------
// "Recursive descent" means we write one function per grammar rule, and those
// functions call each other. When the grammar is recursive (arrays contain
// values which can themselves be arrays), the functions recurse.
//
// JSON's grammar (simplified from RFC 8259):
//
//   value  → null | true | false | number | string | array | object
//   array  → '[' ']'
//           | '[' value (',' value)* ']'
//   object → '{' '}'
//           | '{' pair (',' pair)* '}'
//   pair   → string ':' value
//
// Each grammar rule maps to a function in this file:
//   `value`  → parseValue(_:_:)
//   `array`  → parseArray(_:_:)
//   `object` → parseObject(_:_:)
//
// How does the parser track state?
// ---------------------------------
// We use a single integer `pos` (an `inout` parameter) that tracks which
// token we're currently looking at. Each parse function:
//   1. Reads tokens starting at `pos`
//   2. Advances `pos` past the tokens it consumed
//   3. Returns the parsed value
//
// `inout` in Swift is like a pointer: changes to `pos` inside the function
// are visible to the caller. This lets recursive calls all share the same
// position counter.
//
// Error handling
// --------------
// We use Swift's `throws` mechanism. When the input doesn't match what the
// grammar expects (e.g., we see a `}` when expecting a value), we throw a
// `JsonParseError` with a descriptive message. The entire parse is aborted
// and the error propagates up to the caller.
// ============================================================================

import JsonValue
import JsonLexer

// ============================================================================
// Error type
// ============================================================================

/// An error produced when the parser encounters tokens in an order that
/// does not match the JSON grammar.
///
/// Examples:
///   - `{"key" 42}` — missing `:` between key and value
///   - `[1 2]`      — missing `,` between array elements
///   - `{42: "v"}`  — object key must be a string, not a number
public struct JsonParseError: Error, Sendable {
    public let message: String
    public init(_ message: String) { self.message = message }
}

// ============================================================================
// Parser
// ============================================================================

/// A stateless JSON parser implementing recursive descent.
///
/// Stateless means all mutable state lives in local variables inside parse
/// methods — the struct itself has no stored properties. This makes
/// `JsonParser` trivially `Sendable`.
///
/// Usage:
/// ```swift
/// let parser = JsonParser()
/// let value  = try parser.parse("{\"x\": 1}")
/// // → JsonValue.object([(key: "x", value: .number(1))])
/// ```
public struct JsonParser: Sendable {
    public init() {}

    // =========================================================================
    // MARK: — Public API
    // =========================================================================

    /// Parse a JSON string directly into a `JsonValue`.
    ///
    /// This is the most convenient entry point — it internally calls the
    /// lexer and then the token-based parse. Both lexing errors (`JsonLexError`)
    /// and parsing errors (`JsonParseError`) propagate to the caller.
    ///
    /// - Parameter input: A UTF-8 JSON string.
    /// - Returns: The parsed `JsonValue`.
    /// - Throws: `JsonLexError` or `JsonParseError` on malformed input.
    public func parse(_ input: String) throws -> JsonValue {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize(input)
        return try parseTokens(tokens)
    }

    /// Parse a pre-lexed token array into a `JsonValue`.
    ///
    /// Use this when you already have tokens (e.g., to parse multiple values
    /// from a single token stream, or for testing).
    ///
    /// - Parameter tokens: An array of `Token` values from `JsonLexer`.
    /// - Returns: The parsed `JsonValue`.
    /// - Throws: `JsonParseError` if the tokens do not form a valid JSON value.
    public func parseTokens(_ tokens: [Token]) throws -> JsonValue {
        var pos = 0
        let result = try parseValue(tokens, &pos)
        // After parsing a complete JSON value, no tokens should remain.
        // Extra tokens indicate malformed input like `1 2` or `{} {}`.
        if pos != tokens.count {
            throw JsonParseError("Unexpected tokens after end of JSON value at position \(pos)")
        }
        return result
    }

    // =========================================================================
    // MARK: — Grammar rules (private)
    // =========================================================================

    /// Parse any JSON value starting at `tokens[pos]`.
    ///
    /// This function dispatches to specialized parsers based on the current
    /// token kind. It implements the `value` rule in the JSON grammar.
    private func parseValue(_ tokens: [Token], _ pos: inout Int) throws -> JsonValue {
        guard pos < tokens.count else {
            throw JsonParseError("Unexpected end of input — expected a JSON value")
        }

        switch tokens[pos].kind {
        case .nullLit:
            // Consume the `null` token and return the null value.
            pos += 1
            return .null

        case .trueLit:
            pos += 1
            return .bool(true)

        case .falseLit:
            pos += 1
            return .bool(false)

        case .numberLit(let n):
            pos += 1
            return .number(n)

        case .stringLit(let s):
            pos += 1
            return .string(s)

        case .leftBracket:
            // Delegate array parsing to parseArray
            return try parseArray(tokens, &pos)

        case .leftBrace:
            // Delegate object parsing to parseObject
            return try parseObject(tokens, &pos)

        default:
            // Any other token (}, ], :, ,) cannot start a value
            throw JsonParseError("Unexpected token \(tokens[pos].kind) — expected a JSON value")
        }
    }

    /// Parse a JSON array starting at `tokens[pos]`.
    ///
    /// Grammar:
    ///   array → '[' ']'
    ///         | '[' value (',' value)* ']'
    ///
    /// After this function returns, `pos` points to the token AFTER `]`.
    private func parseArray(_ tokens: [Token], _ pos: inout Int) throws -> JsonValue {
        // Consume the opening '['
        // Precondition: tokens[pos].kind == .leftBracket
        pos += 1

        var elements: [JsonValue] = []

        // Check for the empty array case: `[]`
        if pos < tokens.count, case .rightBracket = tokens[pos].kind {
            pos += 1  // consume ']'
            return .array([])
        }

        // Parse the first element (we know there is at least one because we
        // didn't hit ']' above).
        let first = try parseValue(tokens, &pos)
        elements.append(first)

        // Parse any additional elements, each preceded by a comma.
        //
        // Loop invariant: after each iteration, `pos` points to either:
        //   - a `,` token (there are more elements), or
        //   - a `]` token (we're done), or
        //   - anything else (which is a syntax error).
        while true {
            guard pos < tokens.count else {
                throw JsonParseError("Unterminated array — expected ']' or ','")
            }

            switch tokens[pos].kind {
            case .rightBracket:
                // End of array — consume ']' and return.
                pos += 1
                return .array(elements)

            case .comma:
                // Separator — consume ',' and parse the next element.
                pos += 1

                // Disallow trailing commas: `[1, 2,]` is not valid JSON.
                // After a comma we must see a value, not `]`.
                guard pos < tokens.count else {
                    throw JsonParseError("Expected value after ',' in array")
                }
                if case .rightBracket = tokens[pos].kind {
                    throw JsonParseError("Trailing comma in array is not valid JSON")
                }

                let element = try parseValue(tokens, &pos)
                elements.append(element)

            default:
                throw JsonParseError("Expected ',' or ']' in array, got \(tokens[pos].kind)")
            }
        }
    }

    /// Parse a JSON object starting at `tokens[pos]`.
    ///
    /// Grammar:
    ///   object → '{' '}'
    ///          | '{' pair (',' pair)* '}'
    ///   pair   → string ':' value
    ///
    /// After this function returns, `pos` points to the token AFTER `}`.
    private func parseObject(_ tokens: [Token], _ pos: inout Int) throws -> JsonValue {
        // Consume the opening '{'
        // Precondition: tokens[pos].kind == .leftBrace
        pos += 1

        var pairs: [(key: String, value: JsonValue)] = []

        // Check for the empty object case: `{}`
        if pos < tokens.count, case .rightBrace = tokens[pos].kind {
            pos += 1  // consume '}'
            return .object([])
        }

        // Parse the first key-value pair.
        let firstPair = try parsePair(tokens, &pos)
        pairs.append(firstPair)

        // Parse any additional pairs, each preceded by a comma.
        while true {
            guard pos < tokens.count else {
                throw JsonParseError("Unterminated object — expected '}' or ','")
            }

            switch tokens[pos].kind {
            case .rightBrace:
                // End of object — consume '}' and return.
                pos += 1
                return .object(pairs)

            case .comma:
                // Separator — consume ',' and parse the next pair.
                pos += 1

                // Disallow trailing commas: `{"a":1,}` is not valid JSON.
                guard pos < tokens.count else {
                    throw JsonParseError("Expected key-value pair after ',' in object")
                }
                if case .rightBrace = tokens[pos].kind {
                    throw JsonParseError("Trailing comma in object is not valid JSON")
                }

                let pair = try parsePair(tokens, &pos)
                pairs.append(pair)

            default:
                throw JsonParseError("Expected ',' or '}' in object, got \(tokens[pos].kind)")
            }
        }
    }

    /// Parse a single key-value pair: `string ':' value`.
    ///
    /// - Precondition: `tokens[pos]` is the key token (must be `.stringLit`).
    /// - Postcondition: `pos` points to the token after the value.
    private func parsePair(
        _ tokens: [Token],
        _ pos: inout Int
    ) throws -> (key: String, value: JsonValue) {
        // The key must be a string literal (RFC 8259 §4).
        guard pos < tokens.count, case .stringLit(let key) = tokens[pos].kind else {
            let got = pos < tokens.count ? "\(tokens[pos].kind)" : "end of input"
            throw JsonParseError("Object key must be a string, got \(got)")
        }
        pos += 1  // consume the key

        // After the key there must be a colon.
        guard pos < tokens.count, case .colon = tokens[pos].kind else {
            let got = pos < tokens.count ? "\(tokens[pos].kind)" : "end of input"
            throw JsonParseError("Expected ':' after object key, got \(got)")
        }
        pos += 1  // consume the colon

        // Parse the value (any JSON value type).
        let value = try parseValue(tokens, &pos)

        return (key: key, value: value)
    }
}
