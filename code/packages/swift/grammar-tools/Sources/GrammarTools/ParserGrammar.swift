// ============================================================================
// ParserGrammar.swift — Parser and validator for .grammar files.
// ============================================================================
//
// A .grammar file describes the syntactic structure of a programming language
// using EBNF (Extended Backus-Naur Form). Where a .tokens file says "these
// are the words," a .grammar file says "these are the sentences."
//
// ============================================================================
// EBNF: A BRIEF HISTORY
// ============================================================================
//
// BNF (Backus-Naur Form) was invented in the late 1950s by John Backus and
// Peter Naur to describe the syntax of ALGOL 60. It was one of the first
// formal notations for programming language grammars. EBNF extends BNF with
// three conveniences:
//
//     { x }   -- zero or more repetitions of x
//     [ x ]   -- optional x (shorthand for x | epsilon)
//     ( x )   -- grouping (to clarify precedence in alternations)
//
// These extensions don't add any theoretical power -- anything expressible in
// EBNF can be written in plain BNF -- but they make grammars dramatically
// more readable.
//
// ============================================================================
// EXTENSIONS BEYOND STANDARD EBNF
// ============================================================================
//
// This parser supports several extensions that go beyond standard EBNF:
//
// - **Positive lookahead** (`& element`): Succeeds if the element would match
//   at the current position, but does NOT consume any input. Written with `&`
//   prefix. Useful for disambiguation: `& NEWLINE` checks that a newline
//   follows without consuming it.
//
// - **Negative lookahead** (`! element`): Succeeds if the element would NOT
//   match at the current position. Does not consume input. Written with `!`
//   prefix. Example: `! "else" NAME` matches any NAME that is not "else".
//
// - **One-or-more repetition** (`element +`): Matches one or more occurrences.
//   Written as `element +` (with a postfix `+`). Equivalent to
//   `element { element }` but more concise.
//
// - **Separated repetition** (`element // separator`): Matches one or more
//   occurrences of element separated by separator. Written as
//   `element // separator`. Example: `expression // COMMA` matches
//   `expression (COMMA expression)*`. Useful for argument lists, parameter
//   lists, and other comma-separated constructs.
//
// ============================================================================
// THE RECURSIVE DESCENT PARSER
// ============================================================================
//
// This module contains a hand-written recursive descent parser for the EBNF
// notation. The grammar of the grammar (the "meta-grammar") is:
//
//     grammar_file  = { rule } ;
//     rule          = rule_name "=" body ";" ;
//     body          = sequence { "|" sequence } ;
//     sequence      = { element } ;
//     element       = "&" atom                  -- positive lookahead
//                   | "!" atom                  -- negative lookahead
//                   | atom "+" [ "//" atom ]    -- one-or-more / separated
//                   | atom "//" atom            -- separated repetition
//                   | atom ;
//     atom          = rule_ref | token_ref | literal
//                   | "{" body "}"
//                   | "[" body "]"
//                   | "(" body ")" ;
//
// Each level becomes a function in the parser.
//
// ============================================================================

import Foundation

// ---------------------------------------------------------------------------
// MARK: - Errors
// ---------------------------------------------------------------------------

/// Thrown when a .grammar file cannot be parsed.
///
public struct ParserGrammarError: Error, Sendable, Equatable {
    public let message: String
    public let lineNumber: Int

    public init(_ message: String, lineNumber: Int) {
        self.message = message
        self.lineNumber = lineNumber
    }
}

extension ParserGrammarError: CustomStringConvertible {
    public var description: String {
        "Line \(lineNumber): \(message)"
    }
}

// ---------------------------------------------------------------------------
// MARK: - Grammar Element (Sum Type)
// ---------------------------------------------------------------------------
//
// These cases form a tree that represents the parsed body of a grammar
// rule. Together they can express anything that EBNF can express, plus
// the extensions (lookahead, one-or-more, separated repetition).
//
// Swift enums with associated values are the natural representation of
// a tagged union / sum type / algebraic data type. Each case carries
// exactly the data needed for that variant:
//
//   .ruleReference("expression")   -- refers to another grammar rule
//   .tokenReference("NUMBER")      -- matches a token of that type
//   .literal("+")                  -- matches a token with that text
//   .sequence([...])               -- A followed by B followed by C
//   .alternation([...])            -- A or B or C
//   .repetition(element)           -- zero or more
//   .optional(element)             -- zero or one
//   .group(element)                -- parenthesized grouping
//   .positiveLookahead(element)    -- succeed if matches, don't consume
//   .negativeLookahead(element)    -- succeed if doesn't match, don't consume
//   .oneOrMore(element)            -- one or more
//   .separatedRepetition(element, separator) -- one+ separated by separator
//
// Exhaustive switch checking ensures we handle every variant.
// ---------------------------------------------------------------------------

/// A grammar element -- one node in the EBNF syntax tree.
///
/// This enum represents every kind of construct that can appear in the
/// body of a grammar rule. The parser builds a tree of these elements
/// when parsing a .grammar file.
///
public indirect enum GrammarElement: Sendable, Equatable {
    /// A reference to another grammar rule (lowercase name).
    ///
    /// In EBNF, `expression` refers to the rule named "expression". Rule
    /// references are always lowercase by convention, distinguishing them
    /// from token references (UPPERCASE).
    ///
    case ruleReference(String)

    /// A reference to a token type (UPPERCASE name).
    ///
    /// In EBNF, `NUMBER` refers to the token type NUMBER from the .tokens
    /// file. Token references are always UPPERCASE by convention.
    ///
    case tokenReference(String)

    /// A literal string match, written as "..." in EBNF.
    ///
    /// Less common than token references -- usually you define tokens in
    /// the .tokens file. But sometimes it's convenient to write a literal
    /// directly in the grammar.
    ///
    case literal(String)

    /// A sequence of elements that must appear in order.
    ///
    /// In EBNF, juxtaposition means sequencing: A B C means "A followed
    /// by B followed by C." This is the most fundamental combinator.
    ///
    case sequence([GrammarElement])

    /// A choice between alternatives, written with | in EBNF.
    ///
    /// A | B | C means "either A, or B, or C." The parser tries each
    /// alternative in order.
    ///
    case alternation([GrammarElement])

    /// Zero-or-more repetition, written as { x } in EBNF.
    ///
    /// { statement } means "zero or more statements."
    ///
    case repetition(GrammarElement)

    /// Optional element, written as [ x ] in EBNF.
    ///
    /// [ ELSE block ] means "optionally an ELSE followed by a block."
    ///
    case optional(GrammarElement)

    /// Explicit grouping, written as ( x ) in EBNF.
    ///
    /// ( PLUS | MINUS ) groups the alternation so it can be used as a
    /// single element in a sequence.
    ///
    case group(GrammarElement)

    /// Positive lookahead: succeed if the element matches at the current
    /// position, but do NOT consume any input. Written as `& element`.
    ///
    /// Example: `& NEWLINE` checks that a newline follows without consuming
    /// it, allowing subsequent rules to also see the newline.
    ///
    case positiveLookahead(GrammarElement)

    /// Negative lookahead: succeed if the element does NOT match at the
    /// current position. Does not consume input. Written as `! element`.
    ///
    /// Example: `! "else" NAME` matches any NAME that is not "else".
    ///
    case negativeLookahead(GrammarElement)

    /// One-or-more repetition, written as `element +` (postfix plus).
    ///
    /// Equivalent to `element { element }` but more concise.
    /// `statement +` means "one or more statements."
    ///
    case oneOrMore(GrammarElement)

    /// Separated repetition: one or more of `element` separated by
    /// `separator`. Written as `element // separator`.
    ///
    /// Example: `expression // COMMA` matches `expression (COMMA expression)*`.
    /// Useful for argument lists, parameter lists, etc.
    ///
    case separatedRepetition(element: GrammarElement, separator: GrammarElement)
}

// ---------------------------------------------------------------------------
// MARK: - Data Model
// ---------------------------------------------------------------------------

/// A single rule from a .grammar file.
///
/// Properties:
///   - name: The rule name (lowercase identifier).
///   - body: The parsed EBNF body as a tree of `GrammarElement` nodes.
///   - lineNumber: The 1-based line number where this rule appeared.
///
public struct GrammarRule: Sendable, Equatable {
    public let name: String
    public let body: GrammarElement
    public let lineNumber: Int

    public init(name: String, body: GrammarElement, lineNumber: Int) {
        self.name = name
        self.body = body
        self.lineNumber = lineNumber
    }
}

/// The complete contents of a parsed .grammar file.
///
/// Properties:
///   - rules: Ordered list of grammar rules. The first rule is the
///       entry point (start symbol).
///   - version: Grammar file version from `# @version N` magic comment.
///
public struct ParserGrammar: Sendable, Equatable {
    public let rules: [GrammarRule]
    public let version: Int

    public init(rules: [GrammarRule], version: Int = 0) {
        self.rules = rules
        self.version = version
    }
}

// ---------------------------------------------------------------------------
// MARK: - AST Traversal Helpers
// ---------------------------------------------------------------------------

/// Return all defined rule names.
///
public func ruleNames(_ grammar: ParserGrammar) -> Set<String> {
    Set(grammar.rules.map(\.name))
}

/// Return all UPPERCASE token names referenced anywhere in the grammar.
///
/// These should correspond to token names in the .tokens file.
///
public func tokenReferences(_ grammar: ParserGrammar) -> Set<String> {
    var refs = Set<String>()
    for rule in grammar.rules {
        collectTokenRefs(rule.body, into: &refs)
    }
    return refs
}

/// Return all lowercase rule names referenced anywhere in the grammar.
///
/// These should correspond to other rule names in this grammar.
///
public func ruleReferences(_ grammar: ParserGrammar) -> Set<String> {
    var refs = Set<String>()
    for rule in grammar.rules {
        collectRuleRefs(rule.body, into: &refs)
    }
    return refs
}

// ---------------------------------------------------------------------------
// MARK: - Internal AST Walkers
// ---------------------------------------------------------------------------
//
// These functions walk the grammar element tree to collect references.
// They use Swift's exhaustive switch on the enum to ensure every case
// is handled -- the compiler warns if you miss one.
// ---------------------------------------------------------------------------

private func collectTokenRefs(_ node: GrammarElement, into refs: inout Set<String>) {
    switch node {
    case .tokenReference(let name):
        refs.insert(name)
    case .ruleReference, .literal:
        break
    case .sequence(let elements):
        for e in elements { collectTokenRefs(e, into: &refs) }
    case .alternation(let choices):
        for c in choices { collectTokenRefs(c, into: &refs) }
    case .repetition(let element),
         .optional(let element),
         .group(let element),
         .positiveLookahead(let element),
         .negativeLookahead(let element),
         .oneOrMore(let element):
        collectTokenRefs(element, into: &refs)
    case .separatedRepetition(let element, let separator):
        collectTokenRefs(element, into: &refs)
        collectTokenRefs(separator, into: &refs)
    }
}

private func collectRuleRefs(_ node: GrammarElement, into refs: inout Set<String>) {
    switch node {
    case .ruleReference(let name):
        refs.insert(name)
    case .tokenReference, .literal:
        break
    case .sequence(let elements):
        for e in elements { collectRuleRefs(e, into: &refs) }
    case .alternation(let choices):
        for c in choices { collectRuleRefs(c, into: &refs) }
    case .repetition(let element),
         .optional(let element),
         .group(let element),
         .positiveLookahead(let element),
         .negativeLookahead(let element),
         .oneOrMore(let element):
        collectRuleRefs(element, into: &refs)
    case .separatedRepetition(let element, let separator):
        collectRuleRefs(element, into: &refs)
        collectRuleRefs(separator, into: &refs)
    }
}

// ---------------------------------------------------------------------------
// MARK: - Tokenizer for .grammar Files
// ---------------------------------------------------------------------------
//
// Before we can parse the EBNF, we need to break the raw text into tokens.
// This is a simple hand-written tokenizer. The grammar notation uses only
// a few token types:
//
//   IDENT   -- an identifier (rule name or token reference)
//   STRING  -- a quoted literal "..."
//   EQUALS  -- the = sign
//   SEMI    -- the ; terminator
//   PIPE    -- the | alternation
//   LBRACE / RBRACE -- { }
//   LBRACKET / RBRACKET -- [ ]
//   LPAREN / RPAREN -- ( )
//   AMPERSAND -- the & positive lookahead prefix
//   EXCLAMATION -- the ! negative lookahead prefix
//   PLUS    -- the + one-or-more suffix
//   DOUBLESLASH -- the // separated repetition operator
//   EOF     -- end of input
// ---------------------------------------------------------------------------

/// Internal token type for the grammar file tokenizer.
private struct GrammarToken {
    let kind: String
    let value: String
    let line: Int
}

/// Tokenize a .grammar file into a list of grammar tokens.
///
/// This tokenizer is much simpler than the language lexers we're trying
/// to generate -- the grammar notation uses only a dozen token types.
///
private func tokenizeGrammar(_ source: String) throws -> [GrammarToken] {
    var tokens: [GrammarToken] = []
    let lines = source.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)

    for lineIdx in 0..<lines.count {
        let lineNumber = lineIdx + 1
        let line = lines[lineIdx]

        var i = line.startIndex
        while i < line.endIndex {
            let ch = line[i]

            // Skip whitespace
            if ch == " " || ch == "\t" {
                i = line.index(after: i)
                continue
            }

            // Skip inline comments
            if ch == "#" {
                break
            }

            // Single-character tokens
            if ch == "=" {
                tokens.append(GrammarToken(kind: "EQUALS", value: "=", line: lineNumber))
                i = line.index(after: i)
            } else if ch == ";" {
                tokens.append(GrammarToken(kind: "SEMI", value: ";", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "|" {
                tokens.append(GrammarToken(kind: "PIPE", value: "|", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "{" {
                tokens.append(GrammarToken(kind: "LBRACE", value: "{", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "}" {
                tokens.append(GrammarToken(kind: "RBRACE", value: "}", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "[" {
                tokens.append(GrammarToken(kind: "LBRACKET", value: "[", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "]" {
                tokens.append(GrammarToken(kind: "RBRACKET", value: "]", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "(" {
                tokens.append(GrammarToken(kind: "LPAREN", value: "(", line: lineNumber))
                i = line.index(after: i)
            } else if ch == ")" {
                tokens.append(GrammarToken(kind: "RPAREN", value: ")", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "&" {
                // Positive lookahead prefix
                tokens.append(GrammarToken(kind: "AMPERSAND", value: "&", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "!" {
                // Negative lookahead prefix
                tokens.append(GrammarToken(kind: "EXCLAMATION", value: "!", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "+" {
                // One-or-more suffix
                tokens.append(GrammarToken(kind: "PLUS", value: "+", line: lineNumber))
                i = line.index(after: i)
            } else if ch == "/" {
                // Check for // (separated repetition)
                let next = line.index(after: i)
                if next < line.endIndex && line[next] == "/" {
                    tokens.append(GrammarToken(kind: "DOUBLESLASH", value: "//", line: lineNumber))
                    i = line.index(after: next)
                } else {
                    throw ParserGrammarError(
                        "Unexpected character: '/'",
                        lineNumber: lineNumber
                    )
                }
            } else if ch == "\"" {
                // Quoted string literal
                var j = line.index(after: i)
                while j < line.endIndex && line[j] != "\"" {
                    if line[j] == "\\" {
                        j = line.index(after: j)
                    }
                    if j < line.endIndex {
                        j = line.index(after: j)
                    }
                }
                guard j < line.endIndex else {
                    throw ParserGrammarError("Unterminated string literal", lineNumber: lineNumber)
                }
                let content = String(line[line.index(after: i)..<j])
                tokens.append(GrammarToken(kind: "STRING", value: content, line: lineNumber))
                i = line.index(after: j)
            } else if ch.isLetter || ch == "_" {
                // Identifier
                var j = i
                while j < line.endIndex && (line[j].isLetter || line[j].isNumber || line[j] == "_") {
                    j = line.index(after: j)
                }
                let ident = String(line[i..<j])
                tokens.append(GrammarToken(kind: "IDENT", value: ident, line: lineNumber))
                i = j
            } else {
                throw ParserGrammarError(
                    "Unexpected character: '\(ch)'",
                    lineNumber: lineNumber
                )
            }
        }
    }

    tokens.append(GrammarToken(kind: "EOF", value: "", line: lines.count))
    return tokens
}

// ---------------------------------------------------------------------------
// MARK: - Recursive Descent Parser
// ---------------------------------------------------------------------------
//
// The parser consumes the token list and builds a tree of GrammarElement
// nodes. Each function corresponds to one level of the meta-grammar:
//
//   parseGrammarFile  ->  { rule }
//   parseRule         ->  name "=" body ";"
//   parseBody         ->  sequence { "|" sequence }
//   parseSequence     ->  { element }
//   parseElement      ->  "&" atom | "!" atom | atom ["+" ["//" atom]] | atom "//" atom
//   parseAtom         ->  ident | string | "{" body "}" | "[" body "]" | "(" body ")"
// ---------------------------------------------------------------------------

/// Internal recursive descent parser for .grammar files.
///
/// This class maintains a position cursor over the token list and provides
/// methods for each level of the meta-grammar. It's a class (not a struct)
/// because the `pos` state is mutated throughout parsing.
///
private class GrammarFileParser {
    let tokens: [GrammarToken]
    var pos: Int

    init(_ tokens: [GrammarToken]) {
        self.tokens = tokens
        self.pos = 0
    }

    /// Look at the current token without consuming it.
    func peek() -> GrammarToken {
        tokens[pos]
    }

    /// Consume and return the current token.
    func advance() -> GrammarToken {
        let tok = tokens[pos]
        pos += 1
        return tok
    }

    /// Consume a token of the expected kind, or throw an error.
    func expect(_ kind: String) throws -> GrammarToken {
        let tok = advance()
        guard tok.kind == kind else {
            throw ParserGrammarError(
                "Expected \(kind), got \(tok.kind) ('\(tok.value)')",
                lineNumber: tok.line
            )
        }
        return tok
    }

    // --- Top level: grammar file = { rule } ---

    /// Parse all rules in the grammar file.
    func parse() throws -> [GrammarRule] {
        var rules: [GrammarRule] = []
        while peek().kind != "EOF" {
            try rules.append(parseRule())
        }
        return rules
    }

    // --- rule = name "=" body ";" ---

    private func parseRule() throws -> GrammarRule {
        let nameTok = try expect("IDENT")
        _ = try expect("EQUALS")
        let body = try parseBody()
        _ = try expect("SEMI")
        return GrammarRule(name: nameTok.value, body: body, lineNumber: nameTok.line)
    }

    // --- body = sequence { "|" sequence } ---

    /// Parse alternation: one or more sequences separated by '|'.
    ///
    /// If there is only one sequence (no '|'), we return it directly
    /// rather than wrapping it in an .alternation node. This keeps the
    /// AST clean.
    ///
    private func parseBody() throws -> GrammarElement {
        let first = try parseSequence()
        var alternatives: [GrammarElement] = [first]

        while peek().kind == "PIPE" {
            _ = advance()
            try alternatives.append(parseSequence())
        }

        if alternatives.count == 1 {
            return alternatives[0]
        }
        return .alternation(alternatives)
    }

    // --- sequence = { element } ---

    /// Parse a sequence of elements.
    ///
    /// A sequence ends when we hit something that cannot start an element:
    /// '|', ';', '}', ']', ')' or EOF.
    ///
    private func parseSequence() throws -> GrammarElement {
        let stopKinds: Set<String> = ["PIPE", "SEMI", "RBRACE", "RBRACKET", "RPAREN", "EOF"]
        var elements: [GrammarElement] = []

        while !stopKinds.contains(peek().kind) {
            try elements.append(parseElement())
        }

        guard !elements.isEmpty else {
            throw ParserGrammarError(
                "Expected at least one element in sequence",
                lineNumber: peek().line
            )
        }

        if elements.count == 1 {
            return elements[0]
        }
        return .sequence(elements)
    }

    // --- element = "&" atom | "!" atom | atom ("+" | "//" atom)? ---

    /// Parse a single grammar element, including prefix and postfix operators.
    ///
    /// Prefix operators:
    ///   & -- positive lookahead
    ///   ! -- negative lookahead
    ///
    /// Postfix operators:
    ///   + -- one-or-more (optionally followed by // separator)
    ///   // -- separated repetition
    ///
    private func parseElement() throws -> GrammarElement {
        // Check for prefix operators
        if peek().kind == "AMPERSAND" {
            _ = advance()
            let atom = try parseAtom()
            return .positiveLookahead(atom)
        }

        if peek().kind == "EXCLAMATION" {
            _ = advance()
            let atom = try parseAtom()
            return .negativeLookahead(atom)
        }

        // Parse the base atom
        let atom = try parseAtom()

        // Check for postfix operators
        if peek().kind == "PLUS" {
            _ = advance()
            // Check for // separator after +
            if peek().kind == "DOUBLESLASH" {
                _ = advance()
                let sep = try parseAtom()
                return .separatedRepetition(element: atom, separator: sep)
            }
            return .oneOrMore(atom)
        }

        if peek().kind == "DOUBLESLASH" {
            _ = advance()
            let sep = try parseAtom()
            return .separatedRepetition(element: atom, separator: sep)
        }

        return atom
    }

    // --- atom = ident | string | "{" body "}" | "[" body "]" | "(" body ")" ---

    /// Parse an atomic grammar element.
    ///
    /// This is where the recursive descent happens: braces, brackets,
    /// and parentheses cause us to recurse back into parseBody.
    ///
    private func parseAtom() throws -> GrammarElement {
        let tok = peek()

        if tok.kind == "IDENT" {
            _ = advance()
            // UPPERCASE = token reference, lowercase = rule reference
            let isToken = tok.value == tok.value.uppercased()
                && tok.value.first?.isUppercase == true
            if isToken {
                return .tokenReference(tok.value)
            }
            return .ruleReference(tok.value)
        }

        if tok.kind == "STRING" {
            _ = advance()
            return .literal(tok.value)
        }

        if tok.kind == "LBRACE" {
            _ = advance()
            let body = try parseBody()
            _ = try expect("RBRACE")
            return .repetition(body)
        }

        if tok.kind == "LBRACKET" {
            _ = advance()
            let body = try parseBody()
            _ = try expect("RBRACKET")
            return .optional(body)
        }

        if tok.kind == "LPAREN" {
            _ = advance()
            let body = try parseBody()
            _ = try expect("RPAREN")
            return .group(body)
        }

        throw ParserGrammarError(
            "Unexpected token: \(tok.kind) ('\(tok.value)')",
            lineNumber: tok.line
        )
    }
}

// ---------------------------------------------------------------------------
// MARK: - Public API
// ---------------------------------------------------------------------------

/// Parse the text of a .grammar file into a `ParserGrammar`.
///
/// This function first scans comment lines for magic comments (`# @key value`),
/// then tokenizes the source and runs a recursive descent parser over the
/// token stream to produce an AST of grammar elements.
///
/// Magic comments supported:
///   # @version N  -- sets the version field (integer; default 0)
///
/// - Parameter source: The full text content of a .grammar file.
/// - Returns: A `ParserGrammar` containing all parsed rules.
/// - Throws: `ParserGrammarError` if the source cannot be parsed.
///
public func parseParserGrammar(source: String) throws -> ParserGrammar {
    // Pre-scan: collect magic comments
    var version = 0
    let magicPattern = try! NSRegularExpression(pattern: #"^#\s*@(\w+)\s*(.*)$"#)

    for rawLine in source.split(separator: "\n", omittingEmptySubsequences: false) {
        let stripped = rawLine.trimmingCharacters(in: .whitespaces)
        guard stripped.hasPrefix("#") else { continue }
        let ns = stripped as NSString
        if let match = magicPattern.firstMatch(in: stripped, range: NSRange(location: 0, length: ns.length)) {
            let key = ns.substring(with: match.range(at: 1))
            let value = ns.substring(with: match.range(at: 2)).trimmingCharacters(in: .whitespaces)
            if key == "version" {
                version = Int(value) ?? 0
            }
        }
    }

    // Main parse
    let tokens = try tokenizeGrammar(source)
    let parser = GrammarFileParser(tokens)
    let rules = try parser.parse()
    return ParserGrammar(rules: rules, version: version)
}

// ---------------------------------------------------------------------------
// MARK: - Validator
// ---------------------------------------------------------------------------

/// Check a parsed `ParserGrammar` for common problems.
///
/// Validation checks:
/// - **Undefined rule references**: A lowercase name is used but never defined.
/// - **Undefined token references**: An UPPERCASE name is not in the token set
///   (only checked if tokenNamesSet is provided).
/// - **Duplicate rule names**: Two rules with the same name.
/// - **Non-lowercase rule names**: Convention violation.
/// - **Unreachable rules**: Defined but never referenced (except the start rule).
///
/// - Parameters:
///   - grammar: A parsed `ParserGrammar` to validate.
///   - tokenNamesSet: Optional set of valid token names from a .tokens file.
/// - Returns: A list of warning/error strings. Empty means no issues.
///
public func validateParserGrammar(
    _ grammar: ParserGrammar,
    tokenNamesSet: Set<String>? = nil
) -> [String] {
    var issues: [String] = []
    let defined = ruleNames(grammar)
    let referencedRules = ruleReferences(grammar)
    let referencedTokens = tokenReferences(grammar)

    // Duplicate rule names
    var seen: [String: Int] = [:]
    for rule in grammar.rules {
        if let firstLine = seen[rule.name] {
            issues.append(
                "Line \(rule.lineNumber): Duplicate rule name " +
                "'\(rule.name)' (first defined on line \(firstLine))"
            )
        } else {
            seen[rule.name] = rule.lineNumber
        }
    }

    // Non-lowercase rule names
    for rule in grammar.rules {
        if rule.name != rule.name.lowercased() {
            issues.append(
                "Line \(rule.lineNumber): Rule name '\(rule.name)' should be lowercase"
            )
        }
    }

    // Undefined rule references
    for ref in referencedRules.sorted() {
        if !defined.contains(ref) {
            issues.append("Undefined rule reference: '\(ref)'")
        }
    }

    // Undefined token references
    if let tokenSet = tokenNamesSet {
        let syntheticTokens: Set<String> = ["NEWLINE", "INDENT", "DEDENT", "EOF"]
        for ref in referencedTokens.sorted() {
            if !tokenSet.contains(ref) && !syntheticTokens.contains(ref) {
                issues.append("Undefined token reference: '\(ref)'")
            }
        }
    }

    // Unreachable rules
    if let startRule = grammar.rules.first {
        for rule in grammar.rules {
            if rule.name != startRule.name && !referencedRules.contains(rule.name) {
                issues.append(
                    "Line \(rule.lineNumber): Rule '\(rule.name)' is " +
                    "defined but never referenced (unreachable)"
                )
            }
        }
    }

    return issues
}
