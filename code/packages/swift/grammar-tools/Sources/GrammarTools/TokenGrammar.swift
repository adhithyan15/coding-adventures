// ============================================================================
// TokenGrammar.swift — Parser and validator for .tokens files.
// ============================================================================
//
// A .tokens file is a declarative description of the lexical grammar of a
// programming language. It lists every token the lexer should recognize, in
// priority order (first match wins), along with optional sections for
// keywords, reserved words, skip patterns, context-sensitive keywords,
// and named pattern groups.
//
// This module solves the "front half" of the grammar-tools pipeline: it reads
// a plain-text token specification and produces a structured `TokenGrammar`
// value that downstream tools (lexer generators, cross-validators) can
// consume.
//
// ============================================================================
// FILE FORMAT OVERVIEW
// ============================================================================
//
// Each non-blank, non-comment line in a .tokens file has one of these forms:
//
//   TOKEN_NAME = /regex_pattern/           -- a regex-based token
//   TOKEN_NAME = "literal_string"          -- a literal-string token
//   TOKEN_NAME = /regex/ -> ALIAS          -- emits token type ALIAS instead
//   TOKEN_NAME = "literal" -> ALIAS        -- same for literals
//   mode: indentation                      -- sets the lexer mode
//   keywords:                              -- begins the keywords section
//   reserved:                              -- begins the reserved keywords section
//   context_keywords:                      -- begins the context keywords section
//   layout_keywords:                       -- begins the layout keywords section
//   skip:                                  -- begins the skip patterns section
//   group NAME:                            -- begins a named pattern group
//
// Lines starting with # are comments. Blank lines are ignored.
//
// The keywords section lists one reserved word per line (indented). Keywords
// are identifiers that the lexer recognizes as NAME tokens but then
// reclassifies. For instance, `if` matches the NAME pattern but is promoted
// to an IF keyword.
//
// The context_keywords section lists words that are keywords in some syntactic
// positions but identifiers in others. For example, JavaScript's `async`,
// `yield`, `get`, `set` are sometimes keywords and sometimes identifiers.
// The lexer emits these as NAME tokens with the TOKEN_CONTEXT_KEYWORD flag
// set, leaving keyword-vs-identifier disambiguation to the parser.
//
// Pattern groups (group NAME:) enable context-sensitive lexing: the lexer
// maintains a stack of active groups and only tries patterns from the group
// on top of the stack.
//
// Magic comments (# @key value) carry structured metadata:
//   # @version N             -- integer schema version (default 0)
//   # @case_insensitive true -- enable case-insensitive matching
//
// ============================================================================
// DESIGN DECISIONS
// ============================================================================
//
// Why hand-parse instead of using a parser library? Because the format is
// simple enough that a line-by-line parser is clearer, faster, and produces
// better error messages. Every error includes the line number where the
// problem occurred.
//
// Why structs instead of classes? Because we want lightweight, plain data
// values that are easy to copy, compare, and test. Swift structs give us
// value semantics with zero reference-counting overhead for small types.
//
// ============================================================================

import Foundation

// ---------------------------------------------------------------------------
// MARK: - Errors
// ---------------------------------------------------------------------------

/// Thrown when a .tokens file cannot be parsed.
///
/// Properties:
///   - message: Human-readable description of the problem.
///   - lineNumber: 1-based line number where the error occurred.
///
public struct TokenGrammarError: Error, Sendable, Equatable {
    public let message: String
    public let lineNumber: Int

    public init(_ message: String, lineNumber: Int) {
        self.message = message
        self.lineNumber = lineNumber
    }
}

extension TokenGrammarError: CustomStringConvertible {
    public var description: String {
        "Line \(lineNumber): \(message)"
    }
}

// ---------------------------------------------------------------------------
// MARK: - Data Model
// ---------------------------------------------------------------------------

/// A single token rule from a .tokens file.
///
/// Each definition maps a token name to a pattern. The pattern is either a
/// regex (written as /pattern/) or a literal string (written as "literal").
///
/// Example .tokens lines and their resulting definitions:
///
///   NUMBER = /[0-9]+/           -> TokenDefinition(name: "NUMBER",
///                                    pattern: "[0-9]+", isRegex: true)
///   PLUS   = "+"               -> TokenDefinition(name: "PLUS",
///                                    pattern: "+", isRegex: false)
///   IDENT  = /[a-z]+/ -> NAME  -> TokenDefinition(name: "IDENT",
///                                    pattern: "[a-z]+", isRegex: true,
///                                    alias: "NAME")
///
/// The alias field is used when the definition name differs from the token
/// type that the lexer should emit. When present, the lexer emits the alias
/// as the token type instead of the definition name.
///
public struct TokenDefinition: Sendable, Equatable {
    /// The token name, e.g. "NUMBER" or "PLUS".
    public let name: String

    /// The pattern string -- either a regex source (without delimiters)
    /// or a literal string (without quotes).
    public let pattern: String

    /// True if the pattern was written as /regex/, false if "literal".
    public let isRegex: Bool

    /// The 1-based line number where this definition appeared.
    public let lineNumber: Int

    /// Optional alias -- the token type the lexer emits instead of `name`.
    public let alias: String?

    public init(name: String, pattern: String, isRegex: Bool, lineNumber: Int, alias: String? = nil) {
        self.name = name
        self.pattern = pattern
        self.isRegex = isRegex
        self.lineNumber = lineNumber
        self.alias = alias
    }
}

/// A named set of token definitions that are active together.
///
/// When this group is at the top of the lexer's group stack, only these
/// patterns are tried during token matching. Skip patterns are global
/// and always tried regardless of the active group.
///
/// Pattern groups enable context-sensitive lexing. For example, an XML
/// lexer defines a "tag" group with patterns for attribute names, equals
/// signs, and attribute values. These patterns are only active inside
/// tags -- the callback pushes the "tag" group when `<` is matched and
/// pops it when `>` is matched.
///
/// Properties:
///   - name: The group name (lowercase identifier).
///   - definitions: Ordered list of token definitions in this group.
///
public struct PatternGroup: Sendable, Equatable {
    /// The group name, e.g. "tag" or "cdata".
    public let name: String

    /// Ordered list of token definitions in this group.
    public let definitions: [TokenDefinition]

    public init(name: String, definitions: [TokenDefinition]) {
        self.name = name
        self.definitions = definitions
    }
}

/// The complete contents of a parsed .tokens file.
///
/// This struct contains everything needed to configure a lexer for a
/// particular language. The `definitions` array is ordered -- the lexer
/// uses first-match-wins semantics, so order matters.
///
/// Optional fields (keywords, skipDefinitions, etc.) are only present when
/// the .tokens file contains the corresponding section. This keeps the
/// struct clean for consumers that don't use those features.
///
public struct TokenGrammar: Sendable, Equatable {
    /// Ordered list of token definitions (first-match-wins).
    public let definitions: [TokenDefinition]

    /// List of reserved words from the keywords: section.
    public let keywords: [String]

    /// Optional lexer mode (e.g. "indentation" or "layout").
    public let mode: String?

    /// Controls how STRING tokens are processed.
    public let escapeMode: String?

    /// Patterns matched and consumed without producing tokens.
    public let skipDefinitions: [TokenDefinition]?

    /// Keywords that are syntax errors if used as identifiers.
    public let reservedKeywords: [String]?

    /// Context-sensitive keywords -- words that are keywords in some
    /// positions but identifiers in others (e.g. async, yield, get, set).
    /// The lexer emits these as NAME tokens with TOKEN_CONTEXT_KEYWORD flag.
    public let contextKeywords: [String]?

    /// Keywords that introduce implicit layout blocks in layout mode.
    public let layoutKeywords: [String]?

    /// Named pattern groups for context-sensitive lexing.
    public let groups: [String: PatternGroup]?

    /// Controls whether the lexer matches case-sensitively.
    public let caseSensitive: Bool?

    /// Grammar file version from `# @version N` magic comment. Defaults to 0.
    public let version: Int

    /// Whether the lexer should match case-insensitively.
    public let caseInsensitive: Bool

    public init(
        definitions: [TokenDefinition],
        keywords: [String],
        mode: String? = nil,
        escapeMode: String? = nil,
        skipDefinitions: [TokenDefinition]? = nil,
        reservedKeywords: [String]? = nil,
        contextKeywords: [String]? = nil,
        layoutKeywords: [String]? = nil,
        groups: [String: PatternGroup]? = nil,
        caseSensitive: Bool? = nil,
        version: Int = 0,
        caseInsensitive: Bool = false
    ) {
        self.definitions = definitions
        self.keywords = keywords
        self.mode = mode
        self.escapeMode = escapeMode
        self.skipDefinitions = skipDefinitions
        self.reservedKeywords = reservedKeywords
        self.contextKeywords = contextKeywords
        self.layoutKeywords = layoutKeywords
        self.groups = groups
        self.caseSensitive = caseSensitive
        self.version = version
        self.caseInsensitive = caseInsensitive
    }
}

// ---------------------------------------------------------------------------
// MARK: - Helper: Extract Token Names
// ---------------------------------------------------------------------------

/// Return the set of all defined token names.
///
/// When a definition has an alias, the alias is included in the set
/// (since that is the name the parser grammar references). The original
/// definition name is also included for completeness.
///
/// Includes names from all pattern groups, since group tokens can
/// also appear in parser grammars.
///
/// This is useful for cross-validation: the parser grammar references
/// tokens by name, and we need to check that every referenced token
/// actually exists.
///
public func tokenNames(_ grammar: TokenGrammar) -> Set<String> {
    var names = Set<String>()

    // Collect from top-level definitions
    for d in grammar.definitions {
        names.insert(d.name)
        if let alias = d.alias {
            names.insert(alias)
        }
    }

    // Collect from pattern groups
    if let groups = grammar.groups {
        for group in groups.values {
            for d in group.definitions {
                names.insert(d.name)
                if let alias = d.alias {
                    names.insert(alias)
                }
            }
        }
    }

    return names
}

/// Return the set of token names as the parser will see them.
///
/// For definitions with aliases, this returns the alias (not the
/// definition name), because that is what the lexer will emit and
/// what the parser grammar references.
///
/// For definitions without aliases, this returns the definition name.
///
public func effectiveTokenNames(_ grammar: TokenGrammar) -> Set<String> {
    var names = Set<String>()

    for d in grammar.definitions {
        names.insert(d.alias ?? d.name)
    }

    if let groups = grammar.groups {
        for group in groups.values {
            for d in group.definitions {
                names.insert(d.alias ?? d.name)
            }
        }
    }

    return names
}

// ---------------------------------------------------------------------------
// MARK: - Parser
// ---------------------------------------------------------------------------

/// Find the closing slash of a regex pattern, respecting character classes.
///
/// Inside a character class `[...]`, the `/` character is literal and does
/// not terminate the pattern. We track bracket depth to handle this:
///
///   /[a-z/]/   -- the / inside [...] is literal, the one after ] closes
///   /\//       -- escaped / is literal
///
/// Returns the index of the closing `/`, or -1 if not found.
///
private func findClosingSlash(_ s: String, startIndex: String.Index) -> String.Index? {
    var inBracket = false
    var i = s.index(after: startIndex)

    while i < s.endIndex {
        let ch = s[i]

        if ch == "\\" {
            // Skip escaped character
            i = s.index(after: i)
            if i < s.endIndex {
                i = s.index(after: i)
            }
            continue
        }

        if ch == "[" && !inBracket {
            inBracket = true
        } else if ch == "]" && inBracket {
            inBracket = false
        } else if ch == "/" && !inBracket {
            return i
        }

        i = s.index(after: i)
    }

    // Fallback: try the last / as a best-effort parse
    if let lastSlash = s.lastIndex(of: "/"), lastSlash > startIndex {
        return lastSlash
    }
    return nil
}

/// Parse a single token definition line into a TokenDefinition.
///
/// Handles both forms:
///   NAME = /pattern/
///   NAME = "literal"
///   NAME = /pattern/ -> ALIAS
///   NAME = "literal" -> ALIAS
///
private func parseDefinition(
    namePart: String,
    patternPart: String,
    lineNumber: Int
) throws -> TokenDefinition {
    guard !patternPart.isEmpty else {
        throw TokenGrammarError(
            "Missing pattern after '=' for token '\(namePart)'",
            lineNumber: lineNumber
        )
    }

    if patternPart.hasPrefix("/") {
        // Regex pattern -- find the closing /
        guard let closingIdx = findClosingSlash(patternPart, startIndex: patternPart.startIndex) else {
            throw TokenGrammarError(
                "Unclosed regex pattern for token '\(namePart)'",
                lineNumber: lineNumber
            )
        }

        let regexBody = String(patternPart[patternPart.index(after: patternPart.startIndex)..<closingIdx])
        guard !regexBody.isEmpty else {
            throw TokenGrammarError(
                "Empty regex pattern for token '\(namePart)'",
                lineNumber: lineNumber
            )
        }

        // Check for -> ALIAS in the remainder
        let remainder = String(patternPart[patternPart.index(after: closingIdx)...]).trimmingCharacters(in: .whitespaces)
        var alias: String?
        if remainder.hasPrefix("->") {
            alias = String(remainder.dropFirst(2)).trimmingCharacters(in: .whitespaces)
            if alias?.isEmpty ?? true {
                throw TokenGrammarError(
                    "Missing alias name after '->' for token '\(namePart)'",
                    lineNumber: lineNumber
                )
            }
        } else if !remainder.isEmpty {
            throw TokenGrammarError(
                "Unexpected text after regex pattern for token '\(namePart)': '\(remainder)'",
                lineNumber: lineNumber
            )
        }

        return TokenDefinition(name: namePart, pattern: regexBody, isRegex: true, lineNumber: lineNumber, alias: alias)
    } else if patternPart.hasPrefix("\"") {
        // Literal pattern -- find the closing "
        let afterQuote = patternPart.index(after: patternPart.startIndex)
        guard let closingQuote = patternPart[afterQuote...].firstIndex(of: "\"") else {
            throw TokenGrammarError(
                "Unclosed literal pattern for token '\(namePart)'",
                lineNumber: lineNumber
            )
        }

        let literalBody = String(patternPart[afterQuote..<closingQuote])
        guard !literalBody.isEmpty else {
            throw TokenGrammarError(
                "Empty literal pattern for token '\(namePart)'",
                lineNumber: lineNumber
            )
        }

        // Check for -> ALIAS in the remainder
        let litRemainder = String(patternPart[patternPart.index(after: closingQuote)...]).trimmingCharacters(in: .whitespaces)
        var alias: String?
        if litRemainder.hasPrefix("->") {
            alias = String(litRemainder.dropFirst(2)).trimmingCharacters(in: .whitespaces)
            if alias?.isEmpty ?? true {
                throw TokenGrammarError(
                    "Missing alias name after '->' for token '\(namePart)'",
                    lineNumber: lineNumber
                )
            }
        } else if !litRemainder.isEmpty {
            throw TokenGrammarError(
                "Unexpected text after literal pattern for token '\(namePart)': '\(litRemainder)'",
                lineNumber: lineNumber
            )
        }

        return TokenDefinition(name: namePart, pattern: literalBody, isRegex: false, lineNumber: lineNumber, alias: alias)
    } else {
        throw TokenGrammarError(
            "Pattern for token '\(namePart)' must be /regex/ or \"literal\", got: '\(patternPart)'",
            lineNumber: lineNumber
        )
    }
}

/// Parse the text of a .tokens file into a `TokenGrammar`.
///
/// The parser operates line-by-line with a section tracker. It has
/// several modes:
///
/// 1. **Definition mode** (default) -- each line is a token definition.
/// 2. **Keywords mode** -- entered on `keywords:`, collects keyword names.
/// 3. **Reserved mode** -- entered on `reserved:`, collects reserved words.
/// 4. **Context keywords mode** -- entered on `context_keywords:`.
/// 5. **Layout keywords mode** -- entered on `layout_keywords:`.
/// 6. **Skip mode** -- entered on `skip:`, collects skip definitions.
/// 7. **Group mode** -- entered on `group NAME:`, collects group definitions.
///
/// A non-indented line exits any section and returns to definition mode.
///
/// - Parameter source: The full text content of a .tokens file.
/// - Returns: A `TokenGrammar` containing all parsed definitions and keywords.
/// - Throws: `TokenGrammarError` if any line cannot be parsed.
///
public func parseTokenGrammar(source: String) throws -> TokenGrammar {
    let lines = source.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
    var definitions: [TokenDefinition] = []
    var keywords: [String] = []
    var skipDefinitions: [TokenDefinition] = []
    var reservedKeywords: [String] = []
    var contextKeywords: [String] = []
    var layoutKeywords: [String] = []
    var groups: [String: PatternGroup] = [:]
    // Mutable builder for group definitions (PatternGroup is immutable)
    var groupBuilders: [String: [TokenDefinition]] = [:]
    var mode: String?
    var escapeMode: String?
    var caseSensitive: Bool = true

    // Magic comment state
    var version = 0
    var caseInsensitive = false

    let magicCommentPattern = try! NSRegularExpression(pattern: #"^#\s*@(\w+)\s*(.*)$"#)
    let identifierPattern = try! NSRegularExpression(pattern: #"^[a-zA-Z_][a-zA-Z0-9_]*$"#)
    let groupNamePattern = try! NSRegularExpression(pattern: #"^[a-z_][a-z0-9_]*$"#)

    let reservedGroupNames: Set<String> = [
        "default", "skip", "keywords", "reserved", "errors", "context_keywords", "layout_keywords",
    ]

    // Section tracking
    var currentSection = "definitions"

    for i in 0..<lines.count {
        let lineNumber = i + 1
        let line = lines[i].replacingOccurrences(of: "\\s+$", with: "", options: .regularExpression)
        let stripped = line.trimmingCharacters(in: .whitespaces)

        // Blank lines are always skipped.
        if stripped.isEmpty { continue }

        // Comment handling -- check for magic comments first
        if stripped.hasPrefix("#") {
            let nsStripped = stripped as NSString
            let range = NSRange(location: 0, length: nsStripped.length)
            if let match = magicCommentPattern.firstMatch(in: stripped, range: range) {
                let key = nsStripped.substring(with: match.range(at: 1))
                let value = nsStripped.substring(with: match.range(at: 2)).trimmingCharacters(in: .whitespaces)
                if key == "version" {
                    version = Int(value) ?? 0
                } else if key == "case_insensitive" {
                    caseInsensitive = value == "true"
                }
            }
            continue
        }

        // --- Mode directive ---
        if stripped.hasPrefix("mode:") || stripped.hasPrefix("mode :") {
            let colonIdx = stripped.firstIndex(of: ":")!
            let modeValue = String(stripped[stripped.index(after: colonIdx)...]).trimmingCharacters(in: .whitespaces)
            guard !modeValue.isEmpty else {
                throw TokenGrammarError("Missing mode value after 'mode:'", lineNumber: lineNumber)
            }
            mode = modeValue
            currentSection = "definitions"
            continue
        }

        // --- Escapes directive ---
        if stripped.hasPrefix("escapes:") || stripped.hasPrefix("escapes :") {
            let colonIdx = stripped.firstIndex(of: ":")!
            let escapesValue = String(stripped[stripped.index(after: colonIdx)...]).trimmingCharacters(in: .whitespaces)
            guard !escapesValue.isEmpty else {
                throw TokenGrammarError("Missing escapes value after 'escapes:'", lineNumber: lineNumber)
            }
            escapeMode = escapesValue
            currentSection = "definitions"
            continue
        }

        // --- Case-sensitive directive ---
        if stripped.hasPrefix("case_sensitive:") || stripped.hasPrefix("case_sensitive :") {
            let colonIdx = stripped.firstIndex(of: ":")!
            let csValue = String(stripped[stripped.index(after: colonIdx)...]).trimmingCharacters(in: .whitespaces)
            guard !csValue.isEmpty else {
                throw TokenGrammarError("Missing value after 'case_sensitive:'", lineNumber: lineNumber)
            }
            let lower = csValue.lowercased()
            if lower == "true" {
                caseSensitive = true
            } else if lower == "false" {
                caseSensitive = false
            } else {
                throw TokenGrammarError(
                    "Invalid case_sensitive value: '\(csValue)' (must be 'true' or 'false')",
                    lineNumber: lineNumber
                )
            }
            currentSection = "definitions"
            continue
        }

        // --- Group headers ---
        if stripped.hasPrefix("group ") && stripped.hasSuffix(":") {
            let groupName = String(stripped.dropFirst(6).dropLast(1)).trimmingCharacters(in: .whitespaces)
            guard !groupName.isEmpty else {
                throw TokenGrammarError("Missing group name after 'group'", lineNumber: lineNumber)
            }
            let nsGroupName = groupName as NSString
            guard groupNamePattern.firstMatch(in: groupName, range: NSRange(location: 0, length: nsGroupName.length)) != nil else {
                throw TokenGrammarError(
                    "Invalid group name: '\(groupName)' (must be a lowercase identifier like 'tag' or 'cdata')",
                    lineNumber: lineNumber
                )
            }
            guard !reservedGroupNames.contains(groupName) else {
                let sorted = reservedGroupNames.sorted().joined(separator: ", ")
                throw TokenGrammarError(
                    "Reserved group name: '\(groupName)' (cannot use \(sorted))",
                    lineNumber: lineNumber
                )
            }
            guard groupBuilders[groupName] == nil else {
                throw TokenGrammarError("Duplicate group name: '\(groupName)'", lineNumber: lineNumber)
            }
            groupBuilders[groupName] = []
            currentSection = "group:\(groupName)"
            continue
        }

        // --- Section headers ---
        if stripped == "keywords:" || stripped == "keywords :" {
            currentSection = "keywords"
            continue
        }
        if stripped == "skip:" || stripped == "skip :" {
            currentSection = "skip"
            continue
        }
        if stripped == "reserved:" || stripped == "reserved :" {
            currentSection = "reserved"
            continue
        }
        if stripped == "context_keywords:" || stripped == "context_keywords :" {
            currentSection = "context_keywords"
            continue
        }
        if stripped == "layout_keywords:" || stripped == "layout_keywords :" {
            currentSection = "layout_keywords"
            continue
        }
        if stripped == "errors:" || stripped == "errors :" {
            currentSection = "errors"
            continue
        }

        // --- Inside a section ---
        let isIndented = !line.isEmpty && (line.first == " " || line.first == "\t")

        if isIndented && currentSection == "keywords" {
            if !stripped.isEmpty {
                keywords.append(stripped)
            }
            continue
        }

        if isIndented && currentSection == "reserved" {
            if !stripped.isEmpty {
                reservedKeywords.append(stripped)
            }
            continue
        }

        if isIndented && currentSection == "context_keywords" {
            if !stripped.isEmpty {
                contextKeywords.append(stripped)
            }
            continue
        }

        if isIndented && currentSection == "layout_keywords" {
            if !stripped.isEmpty {
                layoutKeywords.append(stripped)
            }
            continue
        }

        if isIndented && currentSection == "errors" {
            // Error definitions are informational -- parsed but discarded.
            continue
        }

        if isIndented && currentSection == "skip" {
            guard let eqIdx = stripped.firstIndex(of: "=") else {
                throw TokenGrammarError(
                    "Expected skip definition (NAME = pattern), got: '\(stripped)'",
                    lineNumber: lineNumber
                )
            }
            let skipName = String(stripped[stripped.startIndex..<eqIdx]).trimmingCharacters(in: .whitespaces)
            let skipPattern = String(stripped[stripped.index(after: eqIdx)...]).trimmingCharacters(in: .whitespaces)
            guard !skipPattern.isEmpty else {
                throw TokenGrammarError(
                    "Missing pattern after '=' for skip token '\(skipName)'",
                    lineNumber: lineNumber
                )
            }
            try skipDefinitions.append(parseDefinition(namePart: skipName, patternPart: skipPattern, lineNumber: lineNumber))
            continue
        }

        // --- Inside a group section ---
        if isIndented && currentSection.hasPrefix("group:") {
            let groupName = String(currentSection.dropFirst(6))
            guard let eqIdx = stripped.firstIndex(of: "=") else {
                throw TokenGrammarError(
                    "Expected token definition in group '\(groupName)' (NAME = pattern), got: '\(stripped)'",
                    lineNumber: lineNumber
                )
            }
            let gName = String(stripped[stripped.startIndex..<eqIdx]).trimmingCharacters(in: .whitespaces)
            let gPattern = String(stripped[stripped.index(after: eqIdx)...]).trimmingCharacters(in: .whitespaces)
            guard !gName.isEmpty, !gPattern.isEmpty else {
                throw TokenGrammarError(
                    "Incomplete definition in group '\(groupName)': '\(stripped)'",
                    lineNumber: lineNumber
                )
            }
            let defn = try parseDefinition(namePart: gName, patternPart: gPattern, lineNumber: lineNumber)
            groupBuilders[groupName]?.append(defn)
            continue
        }

        // Non-indented line exits any section
        if !isIndented && currentSection != "definitions" {
            currentSection = "definitions"
        }

        // --- Token definition ---
        guard let eqIdx = line.firstIndex(of: "=") else {
            throw TokenGrammarError(
                "Expected token definition (NAME = pattern), got: '\(stripped)'",
                lineNumber: lineNumber
            )
        }

        let namePart = String(line[line.startIndex..<eqIdx]).trimmingCharacters(in: .whitespaces)
        let patternPart = String(line[line.index(after: eqIdx)...]).trimmingCharacters(in: .whitespaces)

        guard !namePart.isEmpty else {
            throw TokenGrammarError("Missing token name before '='", lineNumber: lineNumber)
        }

        let nsName = namePart as NSString
        guard identifierPattern.firstMatch(in: namePart, range: NSRange(location: 0, length: nsName.length)) != nil else {
            throw TokenGrammarError(
                "Invalid token name: '\(namePart)' (must be an identifier like NAME or PLUS_EQUALS)",
                lineNumber: lineNumber
            )
        }

        try definitions.append(parseDefinition(namePart: namePart, patternPart: patternPart, lineNumber: lineNumber))
    }

    // Build immutable PatternGroup values from the mutable builders
    for (name, defs) in groupBuilders {
        groups[name] = PatternGroup(name: name, definitions: defs)
    }

    let hasGroups = !groups.isEmpty

    return TokenGrammar(
        definitions: definitions,
        keywords: keywords,
        mode: mode,
        escapeMode: escapeMode,
        skipDefinitions: skipDefinitions.isEmpty ? nil : skipDefinitions,
        reservedKeywords: reservedKeywords.isEmpty ? nil : reservedKeywords,
        contextKeywords: contextKeywords.isEmpty ? nil : contextKeywords,
        layoutKeywords: layoutKeywords.isEmpty ? nil : layoutKeywords,
        groups: hasGroups ? groups : nil,
        caseSensitive: caseSensitive ? nil : false,
        version: version,
        caseInsensitive: caseInsensitive
    )
}

// ---------------------------------------------------------------------------
// MARK: - Validator
// ---------------------------------------------------------------------------

/// Validate a list of definitions for common problems.
///
/// Checks for:
/// - Duplicate token names
/// - Empty patterns
/// - Non-UPPER_CASE names
///
private func validateDefinitions(
    _ defs: [TokenDefinition],
    seenNames: inout [String: Int],
    issues: inout [String],
    label: String
) {
    for defn in defs {
        if let firstLine = seenNames[defn.name] {
            issues.append(
                "Line \(defn.lineNumber): Duplicate \(label) name " +
                "'\(defn.name)' (first defined on line \(firstLine))"
            )
        } else {
            seenNames[defn.name] = defn.lineNumber
        }

        if defn.pattern.isEmpty {
            issues.append(
                "Line \(defn.lineNumber): Empty pattern for \(label) '\(defn.name)'"
            )
        }

        if defn.name != defn.name.uppercased() {
            issues.append(
                "Line \(defn.lineNumber): \(label) name '\(defn.name)' should be UPPER_CASE"
            )
        }
    }
}

/// Check a parsed `TokenGrammar` for common problems.
///
/// This is a *lint* pass, not a parse pass -- the grammar has already been
/// parsed successfully. We check for semantic issues:
///
/// - Duplicate token names
/// - Empty patterns
/// - Non-UPPER_CASE names
/// - Unknown mode values
/// - Empty pattern groups
///
/// - Parameter grammar: A parsed `TokenGrammar` to validate.
/// - Returns: A list of warning/error strings. Empty means no issues.
///
public func validateTokenGrammar(_ grammar: TokenGrammar) -> [String] {
    var issues: [String] = []
    var seenNames: [String: Int] = [:]

    validateDefinitions(grammar.definitions, seenNames: &seenNames, issues: &issues, label: "token")

    if let skipDefs = grammar.skipDefinitions {
        validateDefinitions(skipDefs, seenNames: &seenNames, issues: &issues, label: "skip token")
    }

    // Validate mode value
    if let modeVal = grammar.mode, modeVal != "indentation", modeVal != "layout" {
        issues.append("Unknown mode: '\(modeVal)'")
    }

    if grammar.mode == "layout", (grammar.layoutKeywords ?? []).isEmpty {
        issues.append("Layout mode requires a non-empty layout_keywords section")
    }

    // Validate escapeMode value
    if let escVal = grammar.escapeMode, escVal != "none" {
        issues.append("Unknown escapes mode: '\(escVal)'")
    }

    // Validate pattern groups
    if let groups = grammar.groups {
        let groupNamePat = try! NSRegularExpression(pattern: #"^[a-z_][a-z0-9_]*$"#)
        for (groupName, group) in groups {
            let ns = groupName as NSString
            if groupNamePat.firstMatch(in: groupName, range: NSRange(location: 0, length: ns.length)) == nil {
                issues.append(
                    "Invalid group name '\(groupName)' (must be a lowercase identifier)"
                )
            }

            if group.definitions.isEmpty {
                issues.append(
                    "Empty pattern group '\(groupName)' (has no token definitions)"
                )
            }

            var groupSeen: [String: Int] = [:]
            validateDefinitions(
                group.definitions,
                seenNames: &groupSeen,
                issues: &issues,
                label: "group '\(groupName)' token"
            )
        }
    }

    return issues
}
