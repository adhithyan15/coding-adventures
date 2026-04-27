// ============================================================================
// Types.swift — all shared LSP data types used across the server
// ============================================================================
//
// These types mirror the LSP specification's TypeScript type definitions,
// translated to idiomatic Swift. The LSP spec lives at:
// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
//
// # Coordinate System
//
// LSP uses a 0-based, line/character coordinate system. Line 0, character 0 is
// the very first character of the file. This differs from most editors (which
// display 1-based line numbers) and from most lexers (which emit 1-based tokens).
// The LanguageBridge is responsible for converting.
//
// # UTF-16 Code Units
//
// LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
// Unicode codepoints. This is a historical artifact: VS Code is built on
// TypeScript, which uses UTF-16 strings internally. See DocumentManager.swift
// for the conversion function and a detailed explanation of why this matters.
//
// ============================================================================

import Foundation

// ============================================================================
// Position — a cursor position in a document
// ============================================================================
//
// Both `line` and `character` are 0-based. Character is measured in UTF-16 code
// units (see the package doc above for why).
//
// Example: in the string "hello guitar-emoji world", the guitar emoji occupies
// UTF-16 characters 6 and 7 (it requires two UTF-16 surrogates). "world" starts
// at UTF-16 character 8.
//

/// A cursor position in a document, with 0-based line and character offsets.
///
/// The `character` offset is in UTF-16 code units, matching LSP's convention
/// inherited from VS Code's JavaScript string model.
public struct Position: Sendable, Equatable {
    /// 0-based line number.
    public let line: Int

    /// 0-based character offset within the line, in UTF-16 code units.
    public let character: Int

    public init(line: Int, character: Int) {
        self.line = line
        self.character = character
    }
}

// ============================================================================
// Range — a span of text from Start (inclusive) to End (exclusive)
// ============================================================================
//
// Analogy: think of it like a text selection. Start is where the cursor lands
// when you click, End is where you drag to.
//

/// A span of text in a document, from start (inclusive) to end (exclusive).
public struct Range: Sendable, Equatable {
    /// The start position (inclusive).
    public let start: Position

    /// The end position (exclusive).
    public let end: Position

    public init(start: Position, end: Position) {
        self.start = start
        self.end = end
    }
}

// ============================================================================
// Location — a position in a specific file
// ============================================================================
//
// URI uses the "file://" scheme, e.g., "file:///home/user/main.swift".
//

/// A position in a specific file, identified by URI.
public struct Location: Sendable, Equatable {
    /// The file URI (e.g. "file:///home/user/main.swift").
    public let uri: String

    /// The range within the file.
    public let range: Range

    public init(uri: String, range: Range) {
        self.uri = uri
        self.range = range
    }
}

// ============================================================================
// DiagnosticSeverity
// ============================================================================
//
// These match the LSP integer codes:
//   1 = Error, 2 = Warning, 3 = Information, 4 = Hint
//

/// How serious a diagnostic is. Matches LSP's integer codes.
public enum DiagnosticSeverity: Int, Sendable {
    /// A hard error; the code cannot run or compile. Red squiggle.
    case error = 1

    /// Potentially problematic, but not blocking. Yellow squiggle.
    case warning = 2

    /// Informational message. Blue squiggle.
    case information = 3

    /// A suggestion (e.g., "consider using let"). Subtle dots.
    case hint = 4
}

// ============================================================================
// Diagnostic — an error, warning, or hint to display in the editor
// ============================================================================
//
// The editor renders diagnostics as underlined squiggles, with the message
// shown on hover. Red = Error, Yellow = Warning, Blue = Info.
//

/// An error, warning, or hint displayed as a squiggle in the editor.
public struct Diagnostic: Sendable, Equatable {
    /// The range of text this diagnostic applies to.
    public let range: Range

    /// The severity level.
    public let severity: DiagnosticSeverity

    /// The human-readable diagnostic message.
    public let message: String

    /// Optional error code (e.g. "E001").
    public let code: String?

    public init(range: Range, severity: DiagnosticSeverity, message: String, code: String? = nil) {
        self.range = range
        self.severity = severity
        self.message = message
        self.code = code
    }
}

// ============================================================================
// Token — a single lexical token from the language's lexer
// ============================================================================
//
// The bridge's tokenize() method returns a slice of these. The LSP server uses
// tokens to provide semantic syntax highlighting (SemanticTokensProvider).
//
// Note: Line and Column are 1-based (matching most lexers). The bridge must
// convert to 0-based when building SemanticToken values for the LSP response.
//

/// A single lexical token from the language's lexer.
///
/// Line and column are 1-based, matching most lexer conventions.
/// The bridge must convert to 0-based for SemanticToken values.
public struct Token: Sendable, Equatable {
    /// Token type (e.g. "KEYWORD", "IDENTIFIER", "STRING_LIT").
    public let type: String

    /// The actual source text (e.g. "let", "myVar").
    public let value: String

    /// 1-based line number.
    public let line: Int

    /// 1-based column number.
    public let column: Int

    public init(type: String, value: String, line: Int, column: Int) {
        self.type = type
        self.value = value
        self.line = line
        self.column = column
    }
}

// ============================================================================
// ASTNode — the abstract syntax tree
// ============================================================================
//
// We use Any here because each language's parser returns its own concrete
// AST type. The LanguageBridge is responsible for accepting this and
// downcasting to the concrete type it knows about.
//

/// Type alias for the abstract syntax tree produced by the language's parser.
///
/// Each language has its own AST type. The bridge downcasts internally.
public typealias ASTNode = Any

// ============================================================================
// TextEdit — a single text replacement
// ============================================================================
//
// Used by formatting (replace the whole file) and rename (replace each occurrence).
// newText replaces the content at range. If newText is empty, the range is deleted.
//

/// A single text replacement in a document.
public struct TextEdit: Sendable, Equatable {
    /// The range of text to replace.
    public let range: Range

    /// The replacement text. Empty string means deletion.
    public let newText: String

    public init(range: Range, newText: String) {
        self.range = range
        self.newText = newText
    }
}

// ============================================================================
// WorkspaceEdit — TextEdits across potentially multiple files
// ============================================================================

/// Groups TextEdits across potentially multiple files.
///
/// For rename operations in a single file, `changes` has one key.
/// Multi-file renames produce edits across many files.
public struct WorkspaceEdit: Sendable {
    /// Map from file URI to the edits for that file.
    public let changes: [String: [TextEdit]]

    public init(changes: [String: [TextEdit]]) {
        self.changes = changes
    }
}

// ============================================================================
// HoverResult — content for the hover popup
// ============================================================================

/// The content to show in the hover popup.
///
/// `contents` is Markdown text. VS Code renders it with syntax highlighting,
/// bold/italic, code blocks, etc. `range` is optional -- if set, it highlights
/// the symbol in the editor while the hover popup is shown.
public struct HoverResult: Sendable {
    /// Markdown text to display.
    public let contents: String

    /// Optional range to highlight while the hover is shown.
    public let range: Range?

    public init(contents: String, range: Range? = nil) {
        self.contents = contents
        self.range = range
    }
}

// ============================================================================
// CompletionItemKind — icon classification for autocomplete items
// ============================================================================

/// Classifies completion items so the editor shows the right icon.
public enum CompletionItemKind: Int, Sendable {
    case text = 1, method, function, constructor, field,
         variable, `class`, interface, module, property,
         unit, value, `enum`, keyword, snippet,
         color, file, reference, folder, enumMember,
         constant, `struct`, event, `operator`, typeParameter
}

// ============================================================================
// CompletionItem — a single autocomplete suggestion
// ============================================================================

/// A single autocomplete suggestion shown in the editor's dropdown.
public struct CompletionItem: Sendable {
    /// The text shown in the dropdown list.
    public let label: String

    /// The icon to show (function, variable, keyword, etc.).
    public let kind: CompletionItemKind?

    /// Secondary text (e.g. the return type).
    public let detail: String?

    /// Documentation shown when the item is expanded.
    public let documentation: String?

    /// What to actually insert (defaults to label if nil).
    public let insertText: String?

    /// 1 = plain text, 2 = snippet (with tab stops like ${1:name}).
    public let insertTextFormat: Int?

    public init(label: String, kind: CompletionItemKind? = nil, detail: String? = nil,
                documentation: String? = nil, insertText: String? = nil, insertTextFormat: Int? = nil) {
        self.label = label
        self.kind = kind
        self.detail = detail
        self.documentation = documentation
        self.insertText = insertText
        self.insertTextFormat = insertTextFormat
    }
}

// ============================================================================
// SemanticToken — one token's contribution to semantic highlighting
// ============================================================================
//
// Line and Character are 0-based. TokenType and Modifiers reference entries
// in the legend returned by semanticTokenLegend().
//

/// One token's semantic highlighting data.
///
/// Semantic tokens are the "second pass" of syntax highlighting, layered
/// on top of the grammar-based TextMate pass with context-aware type info.
public struct SemanticToken: Sendable {
    /// 0-based line number.
    public let line: Int

    /// 0-based character offset in UTF-16 code units.
    public let character: Int

    /// Length in UTF-16 code units.
    public let length: Int

    /// Must match an entry in the semantic token legend's tokenTypes.
    public let tokenType: String

    /// Subset of the legend's tokenModifiers.
    public let modifiers: [String]

    public init(line: Int, character: Int, length: Int, tokenType: String, modifiers: [String] = []) {
        self.line = line
        self.character = character
        self.length = length
        self.tokenType = tokenType
        self.modifiers = modifiers
    }
}

// ============================================================================
// SymbolKind — classification for document outline symbols
// ============================================================================

/// Classifies document symbols for the outline panel. Matches LSP's 1-based codes.
public enum SymbolKind: Int, Sendable {
    case file = 1, module, namespace, package, `class`,
         method, property, field, constructor, `enum`,
         interface, function, variable, constant, string,
         number, boolean, array, object, key,
         null, enumMember, `struct`, event, `operator`,
         typeParameter
}

// ============================================================================
// DocumentSymbol — one entry in the document outline panel
// ============================================================================

/// One entry in the document outline (Explorer > OUTLINE in VS Code).
///
/// Children allows nesting: a class symbol can have method symbols as children.
/// `range` covers the entire symbol (including body). `selectionRange` is just
/// the name portion (highlighted when user clicks the outline entry).
public struct DocumentSymbol: Sendable {
    /// The symbol name (e.g. "main", "MyClass").
    public let name: String

    /// The symbol kind (function, variable, class, etc.).
    public let kind: SymbolKind

    /// Range covering the entire symbol including its body.
    public let range: Range

    /// Range covering just the symbol's name.
    public let selectionRange: Range

    /// Nested symbols (e.g. methods inside a class).
    public let children: [DocumentSymbol]

    public init(name: String, kind: SymbolKind, range: Range, selectionRange: Range, children: [DocumentSymbol] = []) {
        self.name = name
        self.kind = kind
        self.range = range
        self.selectionRange = selectionRange
        self.children = children
    }
}

// ============================================================================
// FoldingRange — a collapsible region of the document
// ============================================================================

/// A collapsible region in the editor's code folding.
///
/// The editor shows a collapse arrow in the gutter next to `startLine`.
/// When collapsed, lines startLine+1 through endLine are hidden.
public struct FoldingRange: Sendable {
    /// 0-based start line (the line with the collapse arrow).
    public let startLine: Int

    /// 0-based end line (inclusive).
    public let endLine: Int

    /// One of "region", "imports", or "comment" (optional).
    public let kind: String?

    public init(startLine: Int, endLine: Int, kind: String? = nil) {
        self.startLine = startLine
        self.endLine = endLine
        self.kind = kind
    }
}

// ============================================================================
// ParameterInformation — one parameter in a function signature
// ============================================================================

/// One parameter in a function signature tooltip.
public struct ParameterInformation: Sendable {
    /// The parameter label (e.g. "count: Int").
    public let label: String

    /// Optional documentation for this parameter.
    public let documentation: String?

    public init(label: String, documentation: String? = nil) {
        self.label = label
        self.documentation = documentation
    }
}

// ============================================================================
// SignatureInformation — one function overload's full signature
// ============================================================================

/// One function overload's full signature.
public struct SignatureInformation: Sendable {
    /// The full signature label (e.g. "foo(a: Int, b: String) -> Bool").
    public let label: String

    /// Optional documentation for this signature.
    public let documentation: String?

    /// The parameters in this signature.
    public let parameters: [ParameterInformation]

    public init(label: String, documentation: String? = nil, parameters: [ParameterInformation] = []) {
        self.label = label
        self.documentation = documentation
        self.parameters = parameters
    }
}

// ============================================================================
// SignatureHelpResult — tooltip during function call typing
// ============================================================================
//
// Shows the function signature with the current parameter highlighted.
// ActiveSignature indexes into Signatures. ActiveParameter indexes into
// that signature's Parameters.
//

/// Signature help result shown while the user types a function call.
public struct SignatureHelpResult: Sendable {
    /// Available signatures (overloads).
    public let signatures: [SignatureInformation]

    /// Index into `signatures` for the active overload.
    public let activeSignature: Int

    /// Index into the active signature's parameters for the current parameter.
    public let activeParameter: Int

    public init(signatures: [SignatureInformation], activeSignature: Int, activeParameter: Int) {
        self.signatures = signatures
        self.activeSignature = activeSignature
        self.activeParameter = activeParameter
    }
}
