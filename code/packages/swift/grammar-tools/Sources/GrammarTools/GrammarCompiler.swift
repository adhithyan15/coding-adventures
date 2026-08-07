// ============================================================================
// GrammarCompiler.swift — emit a parsed grammar as Swift source
// ============================================================================
//
// The lexer/parser packages must NOT read `code/grammars/**` from disk at run
// time (that couples them to the monorepo layout and breaks published,
// standalone packages). Instead, each package embeds its grammar as *compiled*
// Swift: a generated `_Grammar.swift` whose constants reconstruct the
// `TokenGrammar` / `ParserGrammar` value directly, with no parsing at run time.
//
// This file is the code generator's core. Given an already-parsed grammar, it
// renders the Swift *expression* that rebuilds that exact value. The parse
// happens once, at generation time, using the very same `parseTokenGrammar` /
// `parseParserGrammar` the packages used to call at run time — so the embedded
// value is guaranteed identical to what reading-and-parsing the file produced.
//
// The `grammar-tools-embed` executable target wraps these expressions in a
// module-private `EmbeddedGrammar` enum keyed by version; see its `main.swift`.
//
// # Round-trip contract
//
//   compile(parse(text))  ==  parse(text)
//
// i.e. compiling a grammar to Swift and then evaluating that Swift yields a
// value `==` to the original. The unit tests assert this for real grammars.
// ============================================================================

// ----------------------------------------------------------------------------
// MARK: - Scalar literals
// ----------------------------------------------------------------------------

/// Render a Swift string literal that reproduces `value` verbatim.
///
/// Grammar patterns are regex source full of backslashes (`\d`, `\(`, `\\`) and
/// quotes, so escaping must be exhaustive and lossless. We escape the two
/// characters that are structurally significant in a Swift string literal
/// (`\` and `"`), map the common control characters to their short escapes,
/// and spell every other control character as a `\u{…}` scalar escape. All
/// remaining characters — including any Unicode — are emitted as themselves.
public func swiftStringLiteral(_ value: String) -> String {
    var out = "\""
    for scalar in value.unicodeScalars {
        switch scalar {
        case "\\": out += "\\\\"
        case "\"": out += "\\\""
        case "\n": out += "\\n"
        case "\r": out += "\\r"
        case "\t": out += "\\t"
        case "\0": out += "\\0"
        default:
            if scalar.value < 0x20 || scalar.value == 0x7F {
                out += "\\u{" + String(scalar.value, radix: 16, uppercase: true) + "}"
            } else {
                out.unicodeScalars.append(scalar)
            }
        }
    }
    out += "\""
    return out
}

private func swiftOptionalString(_ value: String?) -> String {
    guard let value else { return "nil" }
    return swiftStringLiteral(value)
}

private func swiftOptionalBool(_ value: Bool?) -> String {
    guard let value else { return "nil" }
    return value ? "true" : "false"
}

private func swiftStringArray(_ values: [String]) -> String {
    if values.isEmpty { return "[]" }
    return "[" + values.map(swiftStringLiteral).joined(separator: ", ") + "]"
}

private func swiftOptionalStringArray(_ values: [String]?) -> String {
    guard let values else { return "nil" }
    return swiftStringArray(values)
}

/// Indent every line of `block` by `spaces` columns.
private func indent(_ block: String, by spaces: Int) -> String {
    let pad = String(repeating: " ", count: spaces)
    return block
        .split(separator: "\n", omittingEmptySubsequences: false)
        .map { $0.isEmpty ? "" : pad + $0 }
        .joined(separator: "\n")
}

// ----------------------------------------------------------------------------
// MARK: - TokenGrammar
// ----------------------------------------------------------------------------

private func compileTokenDefinition(_ definition: TokenDefinition) -> String {
    "TokenDefinition("
        + "name: \(swiftStringLiteral(definition.name)), "
        + "pattern: \(swiftStringLiteral(definition.pattern)), "
        + "isRegex: \(definition.isRegex ? "true" : "false"), "
        + "lineNumber: \(definition.lineNumber), "
        + "alias: \(swiftOptionalString(definition.alias)))"
}

/// Render a `[TokenDefinition]` as a multi-line array literal (one per line).
private func compileTokenDefinitionList(_ definitions: [TokenDefinition]) -> String {
    if definitions.isEmpty { return "[]" }
    let items = definitions
        .map { "    " + compileTokenDefinition($0) + "," }
        .joined(separator: "\n")
    return "[\n\(items)\n]"
}

private func compileOptionalTokenDefinitionList(_ definitions: [TokenDefinition]?) -> String {
    guard let definitions else { return "nil" }
    return compileTokenDefinitionList(definitions)
}

private func compileGroups(_ groups: [String: PatternGroup]?) -> String {
    guard let groups else { return "nil" }
    if groups.isEmpty { return "[:]" }
    // Sort by key so the generated file is deterministic regardless of the
    // dictionary's hash ordering.
    let entries = groups.keys.sorted().map { key -> String in
        let group = groups[key]!
        let defs = indent(compileTokenDefinitionList(group.definitions), by: 8)
        return "    \(swiftStringLiteral(key)): PatternGroup(\n"
            + "        name: \(swiftStringLiteral(group.name)),\n"
            + "        definitions: \(defs.trimmingLeadingSpaces())),"
    }.joined(separator: "\n")
    return "[\n\(entries)\n]"
}

/// Render the Swift expression that reconstructs `grammar`.
public func compileTokenGrammarExpression(_ grammar: TokenGrammar) -> String {
    let definitions = indent(compileTokenDefinitionList(grammar.definitions), by: 4).trimmingLeadingSpaces()
    let skip = indent(compileOptionalTokenDefinitionList(grammar.skipDefinitions), by: 4).trimmingLeadingSpaces()
    let groups = indent(compileGroups(grammar.groups), by: 4).trimmingLeadingSpaces()
    return "TokenGrammar(\n"
        + "    definitions: \(definitions),\n"
        + "    keywords: \(swiftStringArray(grammar.keywords)),\n"
        + "    mode: \(swiftOptionalString(grammar.mode)),\n"
        + "    escapeMode: \(swiftOptionalString(grammar.escapeMode)),\n"
        + "    skipDefinitions: \(skip),\n"
        + "    reservedKeywords: \(swiftOptionalStringArray(grammar.reservedKeywords)),\n"
        + "    contextKeywords: \(swiftOptionalStringArray(grammar.contextKeywords)),\n"
        + "    layoutKeywords: \(swiftOptionalStringArray(grammar.layoutKeywords)),\n"
        + "    groups: \(groups),\n"
        + "    caseSensitive: \(swiftOptionalBool(grammar.caseSensitive)),\n"
        + "    version: \(grammar.version),\n"
        + "    caseInsensitive: \(grammar.caseInsensitive ? "true" : "false"))"
}

// ----------------------------------------------------------------------------
// MARK: - ParserGrammar
// ----------------------------------------------------------------------------

private func compileGrammarElement(_ element: GrammarElement) -> String {
    switch element {
    case .ruleReference(let name):
        return ".ruleReference(\(swiftStringLiteral(name)))"
    case .tokenReference(let name):
        return ".tokenReference(\(swiftStringLiteral(name)))"
    case .literal(let value):
        return ".literal(\(swiftStringLiteral(value)))"
    case .sequence(let elements):
        return ".sequence([" + elements.map(compileGrammarElement).joined(separator: ", ") + "])"
    case .alternation(let choices):
        return ".alternation([" + choices.map(compileGrammarElement).joined(separator: ", ") + "])"
    case .repetition(let child):
        return ".repetition(\(compileGrammarElement(child)))"
    case .optional(let child):
        return ".optional(\(compileGrammarElement(child)))"
    case .group(let child):
        return ".group(\(compileGrammarElement(child)))"
    case .positiveLookahead(let child):
        return ".positiveLookahead(\(compileGrammarElement(child)))"
    case .negativeLookahead(let child):
        return ".negativeLookahead(\(compileGrammarElement(child)))"
    case .oneOrMore(let child):
        return ".oneOrMore(\(compileGrammarElement(child)))"
    case .separatedRepetition(let child, let separator):
        return ".separatedRepetition(element: \(compileGrammarElement(child)), "
            + "separator: \(compileGrammarElement(separator)))"
    }
}

private func compileGrammarRule(_ rule: GrammarRule) -> String {
    "GrammarRule("
        + "name: \(swiftStringLiteral(rule.name)), "
        + "body: \(compileGrammarElement(rule.body)), "
        + "lineNumber: \(rule.lineNumber))"
}

/// Render the Swift expression that reconstructs `grammar`.
public func compileParserGrammarExpression(_ grammar: ParserGrammar) -> String {
    let rules: String
    if grammar.rules.isEmpty {
        rules = "[]"
    } else {
        let items = grammar.rules
            .map { "        " + compileGrammarRule($0) + "," }
            .joined(separator: "\n")
        rules = "[\n\(items)\n    ]"
    }
    return "ParserGrammar(\n"
        + "    rules: \(rules),\n"
        + "    version: \(grammar.version))"
}

// ----------------------------------------------------------------------------
// MARK: - Small string helper
// ----------------------------------------------------------------------------

extension String {
    /// Drop leading spaces from the first line only (used when a pre-indented
    /// block is spliced in after a `label: ` prefix that already supplies the
    /// opening column).
    fileprivate func trimmingLeadingSpaces() -> String {
        var copy = self
        while copy.hasPrefix(" ") { copy.removeFirst() }
        return copy
    }
}
