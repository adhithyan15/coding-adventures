// ============================================================================
// GrammarParser.swift — Grammar-driven parser with packrat memoization.
// ============================================================================
//
// Instead of hardcoding grammar rules as Swift methods (one method per rule),
// this parser reads grammar rules from a ParserGrammar (parsed from a .grammar
// file by grammar-tools) and interprets them at runtime. The same Swift code
// can parse Python, Ruby, or any language -- just swap the .grammar file.
//
// ============================================================================
// HOW IT WORKS
// ============================================================================
//
// The GrammarParser receives two inputs:
//
// 1. A ParserGrammar -- parsed from a .grammar file.
// 2. A list of Token objects -- the output of the lexer.
//
// The parser walks the grammar rule tree, trying to match each element
// against the token stream. Each EBNF element type has a natural
// interpretation:
//
// - RuleReference (lowercase): Recursively parse that grammar rule.
// - TokenReference (UPPERCASE): Match a token of that type.
// - Sequence (A B C): Match A, then B, then C -- all must succeed.
// - Alternation (A | B | C): Try A; if fail, backtrack and try B.
// - Repetition ({ A }): Match zero or more times.
// - Optional ([ A ]): Match zero or one time.
// - Literal ("++"): Match a token whose text value is that string.
// - Group (( A )): Just a parenthesized sub-expression.
// - PositiveLookahead (& A): Succeed if A matches, don't consume.
// - NegativeLookahead (! A): Succeed if A doesn't match, don't consume.
// - OneOrMore (A +): Match one or more times.
// - SeparatedRepetition (A // B): Match A (B A)* -- one+ A separated by B.
//
// ============================================================================
// PACKRAT MEMOIZATION
// ============================================================================
//
// Packrat parsing caches the result of every (rule, position) pair. When the
// parser tries to match a rule at a position it has already visited, it
// returns the cached result immediately. This prevents the exponential
// backtracking that naive recursive descent can suffer from.
//
// Cache key: a string "ruleIndex,position" for fast lookup.
// Cache value: MemoEntry with children, endPos, and ok flag.
//
// ============================================================================
// LEFT RECURSION (WARTH'S ALGORITHM)
// ============================================================================
//
// Standard packrat parsing fails on left-recursive rules like:
//
//     expression = expression PLUS term | term ;
//
// because the parser enters infinite recursion. We use the algorithm from
// Warth et al., "Packrat Parsers Can Support Left Recursion" (2008):
//
// 1. Before parsing a rule, seed the memo with a failure entry.
// 2. If the rule references itself at the same position, the memo returns
//    failure, breaking the recursion.
// 3. After a successful parse, iteratively try to grow the match.
// 4. Each iteration re-parses with the previous result cached, allowing
//    the left-recursive branch to consume more input.
// 5. Stop when the match can no longer grow.
//
// ============================================================================
// FURTHEST-FAILURE ERROR REPORTING
// ============================================================================
//
// When parsing fails, naive parsers report the error at the entry point,
// which is often unhelpful. We track the *furthest* position the parser
// reached during any attempt, and report what was expected there.
//
// Example: parsing `1 + + 3` with an arithmetic grammar would report
// "Expected NUMBER at position 4" (the second +), not "Failed to parse
// at position 0".
//
// ============================================================================

import Lexer
import GrammarTools

// ---------------------------------------------------------------------------
// MARK: - Parse Error
// ---------------------------------------------------------------------------

/// Error thrown when the grammar-driven parser fails.
///
/// Carries the offending token (if available) for position reporting.
///
public struct GrammarParseError: Error, Sendable, Equatable {
    public let message: String
    public let token: Token?

    public init(_ message: String, token: Token? = nil) {
        self.message = message
        self.token = token
    }
}

extension GrammarParseError: CustomStringConvertible {
    public var description: String {
        if let tok = token {
            return "Parse error at \(tok.line):\(tok.column): \(message)"
        }
        return "Parse error: \(message)"
    }
}

// ---------------------------------------------------------------------------
// MARK: - Memo Entry
// ---------------------------------------------------------------------------

/// A cached result from the packrat memoization table.
///
/// Each entry records:
/// - Whether the rule matched at this position (ok).
/// - The matched children (nil if failed).
/// - The position after the match (endPos).
///
private struct MemoEntry {
    let children: [ASTChild]?
    let endPos: Int
    let ok: Bool
}

// ---------------------------------------------------------------------------
// MARK: - Grammar Parser
// ---------------------------------------------------------------------------

/// A grammar-driven parser with packrat memoization and left-recursion support.
///
/// Usage:
///
///     let parser = GrammarParser(tokens: lexerOutput, grammar: parserGrammar)
///     let ast = try parser.parse()
///
/// The parser interprets the grammar rules at runtime, matching each element
/// against the token stream. It uses packrat memoization to avoid exponential
/// backtracking and Warth's algorithm for left-recursive grammars.
///
public final class GrammarParser: @unchecked Sendable {
    /// The token stream to parse.
    private var tokens: [Token]

    /// The grammar rules to interpret.
    private let grammar: ParserGrammar

    /// Current position in the token stream.
    private var pos: Int = 0

    /// Map from rule name to rule definition.
    private let rules: [String: GrammarRule]

    /// Index of each rule name for memo key generation.
    private let ruleIndex: [String: Int]

    /// Whether newlines are significant in this grammar.
    private let newlinesSignificant: Bool

    /// Packrat memoization cache: "ruleIndex,position" -> MemoEntry.
    private var memo: [String: MemoEntry] = [:]

    /// Furthest position reached during parsing (for error reporting).
    private var furthestPos: Int = 0

    /// What was expected at the furthest position.
    private var furthestExpected: [String] = []

    /// Pre-parse hooks: transform token list before parsing.
    private var preParseHooks: [(inout [Token]) -> Void] = []

    /// Post-parse hooks: transform AST after parsing.
    private var postParseHooks: [(ASTNode) -> ASTNode] = []

    /// Create a new grammar parser.
    ///
    /// - Parameters:
    ///   - tokens: The token stream from the lexer.
    ///   - grammar: The parser grammar from grammar-tools.
    ///
    public init(tokens: [Token], grammar: ParserGrammar) {
        self.tokens = tokens
        self.grammar = grammar

        var ruleMap: [String: GrammarRule] = [:]
        var indexMap: [String: Int] = [:]
        for (i, rule) in grammar.rules.enumerated() {
            ruleMap[rule.name] = rule
            indexMap[rule.name] = i
        }
        self.rules = ruleMap
        self.ruleIndex = indexMap
        self.newlinesSignificant = GrammarParser.grammarReferencesNewline(grammar)
    }

    /// Whether newlines are treated as significant tokens in this grammar.
    public var isNewlinesSignificant: Bool {
        newlinesSignificant
    }

    // -----------------------------------------------------------------------
    // MARK: - Hooks
    // -----------------------------------------------------------------------

    /// Register a token transform to run before parsing.
    ///
    /// The hook receives the token list by inout reference and may modify it.
    /// Multiple hooks compose left-to-right.
    ///
    public func addPreParse(_ hook: @escaping (inout [Token]) -> Void) {
        preParseHooks.append(hook)
    }

    /// Register an AST transform to run after parsing.
    ///
    /// The hook receives the final AST and returns a (possibly modified) AST.
    /// Multiple hooks compose left-to-right.
    ///
    public func addPostParse(_ hook: @escaping (ASTNode) -> ASTNode) {
        postParseHooks.append(hook)
    }

    // -----------------------------------------------------------------------
    // MARK: - Main Parse Entry Point
    // -----------------------------------------------------------------------

    /// Parse the token stream using the first grammar rule as entry point.
    ///
    /// - Returns: The root AST node.
    /// - Throws: `GrammarParseError` if parsing fails.
    ///
    public func parse() throws -> ASTNode {
        // Stage 1: Pre-parse hooks
        for hook in preParseHooks {
            hook(&tokens)
        }

        guard !grammar.rules.isEmpty else {
            throw GrammarParseError("Grammar has no rules")
        }

        let entryRule = grammar.rules[0]
        guard let result = parseRule(entryRule.name) else {
            let tok = current()
            if !furthestExpected.isEmpty {
                let expected = furthestExpected.joined(separator: " or ")
                let furthestTok = furthestPos < tokens.count
                    ? tokens[furthestPos]
                    : tok
                throw GrammarParseError(
                    "Expected \(expected), got \"\(furthestTok.value)\"",
                    token: furthestTok
                )
            }
            throw GrammarParseError("Failed to parse", token: tok)
        }

        // Skip trailing newlines
        while pos < tokens.count && current().type == "NEWLINE" {
            pos += 1
        }

        // Verify all tokens consumed
        if pos < tokens.count && current().type != "EOF" {
            let tok = current()
            if !furthestExpected.isEmpty && furthestPos > pos {
                let expected = furthestExpected.joined(separator: " or ")
                let furthestTok = furthestPos < tokens.count
                    ? tokens[furthestPos]
                    : tok
                throw GrammarParseError(
                    "Expected \(expected), got \"\(furthestTok.value)\"",
                    token: furthestTok
                )
            }
            throw GrammarParseError(
                "Unexpected token: \"\(tok.value)\"",
                token: tok
            )
        }

        // Stage 2: Post-parse hooks
        var ast = result
        for hook in postParseHooks {
            ast = hook(ast)
        }

        return ast
    }

    // -----------------------------------------------------------------------
    // MARK: - Helpers
    // -----------------------------------------------------------------------

    /// Get the current token without consuming it.
    private func current() -> Token {
        if pos < tokens.count {
            return tokens[pos]
        }
        return tokens[tokens.count - 1] // EOF
    }

    /// Record a failure expectation at the current position.
    ///
    /// If we are at the furthest position, add to the expected list.
    /// If we are past it, reset the list.
    ///
    private func recordFailure(_ expected: String) {
        if pos > furthestPos {
            furthestPos = pos
            furthestExpected = [expected]
        } else if pos == furthestPos {
            if !furthestExpected.contains(expected) {
                furthestExpected.append(expected)
            }
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - Newline Detection
    // -----------------------------------------------------------------------

    /// Check if any rule in the grammar references NEWLINE tokens.
    ///
    /// If no rule references NEWLINE, newlines are treated as insignificant
    /// (auto-skipped during token matching). If any rule references NEWLINE,
    /// they are treated as significant.
    ///
    private static func grammarReferencesNewline(_ grammar: ParserGrammar) -> Bool {
        for rule in grammar.rules {
            if elementReferencesNewline(rule.body) {
                return true
            }
        }
        return false
    }

    private static func elementReferencesNewline(_ element: GrammarElement) -> Bool {
        switch element {
        case .tokenReference(let name):
            return name == "NEWLINE"
        case .sequence(let elements):
            return elements.contains { elementReferencesNewline($0) }
        case .alternation(let choices):
            return choices.contains { elementReferencesNewline($0) }
        case .repetition(let inner), .optional(let inner), .group(let inner),
             .positiveLookahead(let inner), .negativeLookahead(let inner),
             .oneOrMore(let inner):
            return elementReferencesNewline(inner)
        case .separatedRepetition(let element, let separator):
            return elementReferencesNewline(element) || elementReferencesNewline(separator)
        case .ruleReference, .literal:
            return false
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - Rule Parsing (Packrat + Warth Left Recursion)
    // -----------------------------------------------------------------------

    /// Parse a named grammar rule at the current position.
    ///
    /// Uses packrat memoization and Warth's left-recursion algorithm:
    ///
    /// 1. Check the memo cache -- if we've already tried this (rule, pos),
    ///    return the cached result immediately.
    /// 2. Seed the memo with a failure entry to break left recursion.
    /// 3. Parse the rule body.
    /// 4. If successful and the rule might be left-recursive, iteratively
    ///    try to grow the match.
    ///
    private func parseRule(_ ruleName: String) -> ASTNode? {
        guard let rule = rules[ruleName] else { return nil }

        // Check memo cache
        if let idx = ruleIndex[ruleName] {
            let key = "\(idx),\(pos)"
            if let cached = memo[key] {
                pos = cached.endPos
                if !cached.ok { return nil }
                return ASTNode(
                    ruleName: ruleName,
                    children: cached.children ?? []
                )
            }
        }

        let startPos = pos

        // Seed memo with failure (breaks left recursion)
        if let idx = ruleIndex[ruleName] {
            let key = "\(idx),\(startPos)"
            memo[key] = MemoEntry(children: nil, endPos: startPos, ok: false)
        }

        var children = matchElement(rule.body)

        // Cache result
        if let idx = ruleIndex[ruleName] {
            let key = "\(idx),\(startPos)"
            if let kids = children {
                memo[key] = MemoEntry(children: kids, endPos: pos, ok: true)
            } else {
                memo[key] = MemoEntry(children: nil, endPos: pos, ok: false)
            }

            // Warth: iteratively grow the match for left-recursive rules.
            // Each iteration re-parses with the previous result cached,
            // allowing the left-recursive branch to consume more input.
            if children != nil {
                while true {
                    let prevEnd = pos
                    pos = startPos
                    memo[key] = MemoEntry(children: children, endPos: prevEnd, ok: true)
                    let newChildren = matchElement(rule.body)
                    if newChildren == nil || pos <= prevEnd {
                        pos = prevEnd
                        memo[key] = MemoEntry(children: children, endPos: prevEnd, ok: true)
                        break
                    }
                    children = newChildren
                }
            }
        }

        guard let finalChildren = children else {
            pos = startPos
            recordFailure(ruleName)
            return nil
        }

        return ASTNode(ruleName: ruleName, children: finalChildren)
    }

    // -----------------------------------------------------------------------
    // MARK: - Element Matching
    // -----------------------------------------------------------------------

    /// Match a grammar element against the current position in the token stream.
    ///
    /// Returns the matched children on success, or nil on failure.
    /// On failure, the position is restored to where it was before the attempt.
    ///
    /// This is the core dispatch function -- it handles every `GrammarElement`
    /// variant. Each case implements the natural interpretation of the EBNF
    /// construct.
    ///
    private func matchElement(_ element: GrammarElement) -> [ASTChild]? {
        let savePos = pos

        switch element {
        // ----- Sequence: A B C -- all must match in order -----
        case .sequence(let elements):
            var children: [ASTChild] = []
            for sub in elements {
                guard let result = matchElement(sub) else {
                    pos = savePos
                    return nil
                }
                children.append(contentsOf: result)
            }
            return children

        // ----- Alternation: A | B | C -- try each, take first success -----
        case .alternation(let choices):
            for choice in choices {
                pos = savePos
                if let result = matchElement(choice) {
                    return result
                }
            }
            pos = savePos
            return nil

        // ----- Repetition: { A } -- zero or more -----
        case .repetition(let inner):
            var children: [ASTChild] = []
            while true {
                let saveRep = pos
                guard let result = matchElement(inner) else {
                    pos = saveRep
                    break
                }
                children.append(contentsOf: result)
            }
            return children

        // ----- Optional: [ A ] -- zero or one -----
        case .optional(let inner):
            if let result = matchElement(inner) {
                return result
            }
            return []

        // ----- Group: ( A ) -- just evaluate the inner element -----
        case .group(let inner):
            return matchElement(inner)

        // ----- Token reference: match a token of the expected type -----
        case .tokenReference(let name):
            return matchTokenReference(name)

        // ----- Rule reference: recursively parse the named rule -----
        case .ruleReference(let name):
            if let node = parseRule(name) {
                return [.node(node)]
            }
            pos = savePos
            return nil

        // ----- Literal: match a token whose value equals the string -----
        case .literal(let value):
            var token = current()
            // Skip insignificant newlines
            if !newlinesSignificant {
                while token.type == "NEWLINE" {
                    pos += 1
                    token = current()
                }
            }
            if token.value == value {
                pos += 1
                return [.token(token)]
            }
            recordFailure("\"\(value)\"")
            return nil

        // ----- Positive lookahead: & A -- match without consuming -----
        case .positiveLookahead(let inner):
            let result = matchElement(inner)
            pos = savePos  // Always restore position (lookahead doesn't consume)
            if result != nil {
                return []  // Success, but no children (nothing consumed)
            }
            return nil

        // ----- Negative lookahead: ! A -- succeed if A doesn't match -----
        case .negativeLookahead(let inner):
            let result = matchElement(inner)
            pos = savePos  // Always restore position
            if result == nil {
                return []  // A didn't match, so negative lookahead succeeds
            }
            return nil  // A matched, so negative lookahead fails

        // ----- One-or-more: A + -- one or more occurrences -----
        case .oneOrMore(let inner):
            // Must match at least once
            guard let first = matchElement(inner) else {
                pos = savePos
                return nil
            }
            var children = first
            // Then match zero or more additional times
            while true {
                let saveRep = pos
                guard let result = matchElement(inner) else {
                    pos = saveRep
                    break
                }
                children.append(contentsOf: result)
            }
            return children

        // ----- Separated repetition: A // B -- one+ A separated by B -----
        case .separatedRepetition(let elementPart, let separatorPart):
            // Must match the element at least once
            guard let first = matchElement(elementPart) else {
                pos = savePos
                return nil
            }
            var children = first
            // Then match (separator element)* zero or more times
            while true {
                let saveRep = pos
                guard let sepResult = matchElement(separatorPart) else {
                    pos = saveRep
                    break
                }
                guard let elemResult = matchElement(elementPart) else {
                    pos = saveRep
                    break
                }
                children.append(contentsOf: sepResult)
                children.append(contentsOf: elemResult)
            }
            return children
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - Token Reference Matching
    // -----------------------------------------------------------------------

    /// Match a token of the expected type at the current position.
    ///
    /// If newlines are insignificant and we're not looking for NEWLINE,
    /// skip over any NEWLINE tokens first.
    ///
    private func matchTokenReference(_ expectedType: String) -> [ASTChild]? {
        var token = current()

        // Skip insignificant newlines
        if !newlinesSignificant && expectedType != "NEWLINE" {
            while token.type == "NEWLINE" {
                pos += 1
                token = current()
            }
        }

        if token.type == expectedType {
            pos += 1
            return [.token(token)]
        }

        recordFailure(expectedType)
        return nil
    }
}
