// ============================================================================
// LanguageBridge.swift — LanguageBridge and optional provider protocols
// ============================================================================
//
// # Design Philosophy: Narrow Interfaces
//
// Swift's protocol-oriented design favors many small protocols over one large one.
// This mirrors Go's "interface segregation principle" (from SOLID).
//
// Compare two approaches:
//
//   APPROACH A — fat protocol (one big protocol, every method required):
//
//     protocol LanguageBridge {
//         func tokenize(source: String) -> ([Token], Error?)
//         func parse(source: String) -> (ASTNode?, [Diagnostic], Error?)
//         func hover(ast: ASTNode, pos: Position) -> (HoverResult?, Error?)
//         func definition(ast: ASTNode, pos: Position, uri: String) -> (Location?, Error?)
//         // ... 8 more methods, all REQUIRED
//     }
//
//     Problem: a language that only has a lexer and parser must implement ALL
//     methods, even the symbol-table features it doesn't have yet.
//
//   APPROACH B — narrow protocols (what we use):
//
//     protocol LanguageBridge {
//         func tokenize(source: String) -> ([Token], Error?)
//         func parse(source: String) -> (ASTNode?, [Diagnostic], Error?)
//     }
//     protocol HoverProvider { func hover(...) }
//     protocol DefinitionProvider { func definition(...) }
//
//     At runtime, the server checks: does bridge conform to HoverProvider?
//     If yes -> advertise hoverProvider:true and call bridge.hover().
//     If no  -> omit hover from capabilities. No stubs required.
//
// This matches the LSP spec's philosophy: capabilities are advertised, not
// assumed. An editor won't even try to ask for hover if the server didn't
// advertise it.
//
// # Runtime Capability Detection
//
// Detection uses Swift's `as?` protocol conformance check:
//
//   if let hp = bridge as? HoverProvider {
//       result = hp.hover(ast: ast, pos: pos)
//   }
//
// This is O(1) and checked at compile-time metadata tables.
//
// ============================================================================

import Foundation

// ============================================================================
// LanguageBridge — the required minimum every language must implement
// ============================================================================
//
// Tokenize and Parse are the foundation for all other features:
//   - Tokenize drives semantic token highlighting (accurate syntax coloring)
//   - Parse drives diagnostics, folding, and document symbols
//

/// The required minimum interface every language must implement.
///
/// `tokenize` and `parse` are the foundation. All other features (hover,
/// go-to-definition, etc.) are optional and declared as separate protocols.
/// The LspServer checks at runtime whether the bridge also conforms to those.
public protocol LanguageBridge: AnyObject {
    /// Lex the source string and return the token stream.
    ///
    /// Tokens are used for semantic highlighting. Each Token carries a type
    /// string, its value, and its 1-based line/column position.
    ///
    /// - Parameter source: The full source text to tokenize.
    /// - Returns: A tuple of (tokens, error). Error is nil on success.
    func tokenize(source: String) -> ([Token], Error?)

    /// Parse the source string and return the AST, diagnostics, and any fatal error.
    ///
    /// Even when there are syntax errors, parse should return a partial AST
    /// so that hover, folding, and symbol features can work on valid portions.
    ///
    /// - Parameter source: The full source text to parse.
    /// - Returns: (ast, diagnostics, fatalError). ast may be nil on total failure.
    func parse(source: String) -> (ASTNode?, [Diagnostic], Error?)
}

// ============================================================================
// Optional Provider Protocols
// ============================================================================
//
// Each protocol represents one optional LSP feature. A bridge implements only
// the features its language supports. The server uses `bridge as? HoverProvider`
// to detect support at runtime.
//

/// Enables hover tooltips (textDocument/hover).
///
/// When the user moves their mouse over a symbol, the editor sends a hover
/// request. The bridge returns Markdown text describing the symbol.
public protocol HoverProvider {
    /// Return hover information for the AST node at the given position.
    ///
    /// - Returns: (result, error). Result is nil if no hover info at position.
    func hover(ast: ASTNode, pos: Position) -> (HoverResult?, Error?)
}

/// Enables "Go to Definition" (textDocument/definition).
///
/// When the user presses F12 on a symbol, the bridge looks up where it was declared.
public protocol DefinitionProvider {
    /// Return the location where the symbol at pos was declared.
    ///
    /// - Returns: (location, error). Location is nil if symbol not found.
    func definition(ast: ASTNode, pos: Position, uri: String) -> (Location?, Error?)
}

/// Enables "Find All References" (textDocument/references).
public protocol ReferencesProvider {
    /// Return all uses of the symbol at pos.
    ///
    /// - Parameter includeDecl: If true, include the declaration location.
    func references(ast: ASTNode, pos: Position, uri: String, includeDecl: Bool) -> ([Location], Error?)
}

/// Enables autocomplete suggestions (textDocument/completion).
public protocol CompletionProvider {
    /// Return autocomplete suggestions valid at pos.
    func completion(ast: ASTNode, pos: Position) -> ([CompletionItem], Error?)
}

/// Enables symbol rename (textDocument/rename).
public protocol RenameProvider {
    /// Return the text edits needed to rename the symbol at pos.
    func rename(ast: ASTNode, pos: Position, newName: String) -> (WorkspaceEdit?, Error?)
}

/// Enables accurate syntax highlighting (textDocument/semanticTokens/full).
///
/// The bridge maps each token to a semantic type (keyword, string, variable, etc.).
/// The framework encodes the result into LSP's compact binary format.
public protocol SemanticTokensProvider {
    /// Return semantic token data for the whole document.
    ///
    /// - Parameter tokens: The output of tokenize() -- reuse rather than re-lexing.
    func semanticTokens(source: String, tokens: [Token]) -> ([SemanticToken], Error?)
}

/// Enables the document outline panel (textDocument/documentSymbol).
public protocol DocumentSymbolsProvider {
    /// Return the outline tree for the given AST.
    func documentSymbols(ast: ASTNode) -> ([DocumentSymbol], Error?)
}

/// Enables code folding (textDocument/foldingRange).
public protocol FoldingRangesProvider {
    /// Return collapsible regions derived from the AST structure.
    func foldingRanges(ast: ASTNode) -> ([FoldingRange], Error?)
}

/// Enables function signature hints (textDocument/signatureHelp).
public protocol SignatureHelpProvider {
    /// Return signature hint information for the call at pos.
    ///
    /// - Returns: (result, error). Result is nil if not inside a call.
    func signatureHelp(ast: ASTNode, pos: Position) -> (SignatureHelpResult?, Error?)
}

/// Enables document formatting (textDocument/formatting).
public protocol FormatProvider {
    /// Return the text edits needed to format the document.
    func format(source: String) -> ([TextEdit], Error?)
}
