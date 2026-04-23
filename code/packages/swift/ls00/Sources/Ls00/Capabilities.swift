// ============================================================================
// Capabilities.swift — BuildCapabilities, SemanticTokenLegend, and encoding
// ============================================================================
//
// # What Are Capabilities?
//
// During the LSP initialize handshake, the server sends back a "capabilities"
// object telling the editor which LSP features it supports. The editor uses this
// to decide which requests to send.
//
// Building capabilities dynamically (based on the bridge's protocol conformance)
// means the server is always honest about what it can do. A bridge that only has
// a lexer and parser gets minimal capabilities. A full-featured bridge with a
// symbol table gets the full set.
//
// # Semantic Token Legend
//
// Semantic tokens use a compact binary encoding. Instead of sending
// {"type":"keyword"} per token, LSP sends an integer index into a legend.
// The legend is declared in the capabilities so the editor knows what each
// index means.
//
// ============================================================================

import Foundation

/// Inspect the bridge at runtime and return the LSP capabilities object.
///
/// Uses Swift's `as?` protocol conformance checks to determine which optional
/// provider protocols the bridge implements. Only advertises capabilities for
/// features the bridge actually supports.
///
/// - Parameter bridge: The language bridge to inspect.
/// - Returns: A dictionary suitable for the "capabilities" field of the initialize response.
public func buildCapabilities(_ bridge: LanguageBridge) -> [String: Any] {
    // textDocumentSync=2 means "incremental": the editor sends only changed
    // ranges, not the full file. We always advertise this.
    var caps: [String: Any] = [
        "textDocumentSync": 2,
    ]

    // Check each optional provider protocol via `as?`.

    if bridge is HoverProvider {
        caps["hoverProvider"] = true
    }

    if bridge is DefinitionProvider {
        caps["definitionProvider"] = true
    }

    if bridge is ReferencesProvider {
        caps["referencesProvider"] = true
    }

    if bridge is CompletionProvider {
        caps["completionProvider"] = [
            "triggerCharacters": [" ", "."],
        ] as [String: Any]
    }

    if bridge is RenameProvider {
        caps["renameProvider"] = true
    }

    if bridge is DocumentSymbolsProvider {
        caps["documentSymbolProvider"] = true
    }

    if bridge is FoldingRangesProvider {
        caps["foldingRangeProvider"] = true
    }

    if bridge is SignatureHelpProvider {
        caps["signatureHelpProvider"] = [
            "triggerCharacters": ["(", ","],
        ] as [String: Any]
    }

    if bridge is FormatProvider {
        caps["documentFormattingProvider"] = true
    }

    if bridge is SemanticTokensProvider {
        caps["semanticTokensProvider"] = [
            "legend": [
                "tokenTypes": semanticTokenLegend().tokenTypes,
                "tokenModifiers": semanticTokenLegend().tokenModifiers,
            ] as [String: Any],
            "full": true,
        ] as [String: Any]
    }

    return caps
}

// ============================================================================
// SemanticTokenLegendData
// ============================================================================

/// Holds the legend arrays for semantic tokens.
/// The editor uses these to decode the compact integer encoding.
public struct SemanticTokenLegendData: Sendable {
    /// The token type names, indexed by their integer code.
    public let tokenTypes: [String]

    /// The token modifier names, indexed by bit position.
    public let tokenModifiers: [String]
}

/// Return the full legend for all supported semantic token types and modifiers.
///
/// The ordering matters: index 0 in tokenTypes corresponds to "namespace",
/// index 1 to "type", etc. These match the standard LSP token types.
public func semanticTokenLegend() -> SemanticTokenLegendData {
    return SemanticTokenLegendData(
        tokenTypes: [
            "namespace",     // 0
            "type",          // 1
            "class",         // 2
            "enum",          // 3
            "interface",     // 4
            "struct",        // 5
            "typeParameter", // 6
            "parameter",     // 7
            "variable",      // 8
            "property",      // 9
            "enumMember",    // 10
            "event",         // 11
            "function",      // 12
            "method",        // 13
            "macro",         // 14
            "keyword",       // 15
            "modifier",      // 16
            "comment",       // 17
            "string",        // 18
            "number",        // 19
            "regexp",        // 20
            "operator",      // 21
            "decorator",     // 22
        ],
        tokenModifiers: [
            "declaration",    // bit 0
            "definition",     // bit 1
            "readonly",       // bit 2
            "static",         // bit 3
            "deprecated",     // bit 4
            "abstract",       // bit 5
            "async",          // bit 6
            "modification",   // bit 7
            "documentation",  // bit 8
            "defaultLibrary", // bit 9
        ]
    )
}

// ============================================================================
// Token type and modifier index lookups
// ============================================================================

/// Return the integer index for a semantic token type string.
/// Returns -1 if the type is not in the legend.
func tokenTypeIndex(_ tokenType: String) -> Int {
    let legend = semanticTokenLegend()
    for (i, t) in legend.tokenTypes.enumerated() {
        if t == tokenType { return i }
    }
    return -1
}

/// Return the bitmask for a list of modifier strings.
///
/// Each modifier corresponds to a bit position:
///   "declaration" -> bit 0 -> value 1
///   "definition"  -> bit 1 -> value 2
///   both          -> value 3 (bitwise OR)
func tokenModifierMask(_ modifiers: [String]) -> Int {
    let legend = semanticTokenLegend()
    var mask = 0
    for mod in modifiers {
        for (i, m) in legend.tokenModifiers.enumerated() {
            if m == mod {
                mask |= (1 << i)
                break
            }
        }
    }
    return mask
}

// ============================================================================
// EncodeSemanticTokens
// ============================================================================
//
// LSP encodes semantic tokens as a flat array of integers, grouped in 5-tuples:
//
//   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
//
// Where "delta" means the difference from the PREVIOUS token's position.
// When deltaLine > 0, deltaStartChar is absolute for that line.
// When deltaLine == 0, deltaStartChar is relative to previous token.
//

/// Convert a list of SemanticTokens to LSP's compact delta-format integer array.
///
/// Tokens are sorted by (line, character) before encoding. Unknown token types
/// are silently skipped.
///
/// - Parameter tokens: The semantic tokens to encode.
/// - Returns: A flat array of integers in groups of 5.
public func encodeSemanticTokens(_ tokens: [SemanticToken]) -> [Int] {
    if tokens.isEmpty { return [] }

    // Sort by (line, character) ascending.
    let sorted = tokens.sorted { a, b in
        if a.line != b.line { return a.line < b.line }
        return a.character < b.character
    }

    var data: [Int] = []
    data.reserveCapacity(sorted.count * 5)
    var prevLine = 0
    var prevChar = 0

    for tok in sorted {
        let typeIdx = tokenTypeIndex(tok.tokenType)
        if typeIdx == -1 { continue } // unknown type -- skip

        let deltaLine = tok.line - prevLine
        let deltaChar: Int
        if deltaLine == 0 {
            // Same line: character offset relative to previous token.
            deltaChar = tok.character - prevChar
        } else {
            // Different line: character offset absolute (from line start).
            deltaChar = tok.character
        }

        let modMask = tokenModifierMask(tok.modifiers)

        data.append(contentsOf: [deltaLine, deltaChar, tok.length, typeIdx, modMask])

        prevLine = tok.line
        prevChar = tok.character
    }

    return data
}
