//! All shared LSP data types used across the server.
//!
//! These types mirror the LSP specification's TypeScript type definitions,
//! translated to idiomatic Rust. The LSP spec lives at:
//! <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>
//!
//! # Coordinate System
//!
//! LSP uses a 0-based, line/character coordinate system. Line 0, character 0 is
//! the very first character of the file. This differs from most editors (which
//! display 1-based line numbers) and from our lexer (which emits 1-based tokens).
//! The `LanguageBridge` is responsible for converting.
//!
//! # UTF-16 Code Units
//!
//! LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
//! Unicode codepoints. This is a historical artifact: VS Code is built on
//! TypeScript, which uses UTF-16 strings internally. See `document_manager.rs`
//! for the conversion function and a detailed explanation of why this matters.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Position
// ---------------------------------------------------------------------------

/// A cursor position in a document.
///
/// Both `line` and `character` are 0-based. `character` is measured in UTF-16
/// code units (see the module doc above for why).
///
/// # Example
///
/// In the string `"hello 🎸 world"`, the guitar emoji (🎸) occupies
/// UTF-16 characters 6 and 7 (it requires two UTF-16 surrogates). `"world"`
/// starts at UTF-16 character 9.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: i32,
    pub character: i32,
}

// ---------------------------------------------------------------------------
// Range
// ---------------------------------------------------------------------------

/// A span of text in a document, from `start` (inclusive) to `end` (exclusive).
///
/// Analogy: think of it like a text selection. `start` is where the cursor
/// lands when you click, `end` is where you drag to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

// ---------------------------------------------------------------------------
// Location
// ---------------------------------------------------------------------------

/// A position in a specific file.
///
/// `uri` uses the `"file://"` scheme, e.g., `"file:///home/user/main.rs"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

// ---------------------------------------------------------------------------
// DiagnosticSeverity
// ---------------------------------------------------------------------------

/// How serious a diagnostic is. These match the LSP integer codes.
///
/// | Value | Meaning      | Editor rendering      |
/// |-------|--------------|-----------------------|
/// | 1     | Error        | Red squiggle          |
/// | 2     | Warning      | Yellow squiggle       |
/// | 3     | Information  | Blue squiggle         |
/// | 4     | Hint         | Faint dots            |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum DiagnosticSeverity {
    /// A hard error; the code cannot run or compile.
    Error = 1,
    /// Potentially problematic, but not blocking.
    Warning = 2,
    /// Informational message.
    Information = 3,
    /// A suggestion (e.g., "consider using const").
    Hint = 4,
}

// ---------------------------------------------------------------------------
// Diagnostic
// ---------------------------------------------------------------------------

/// An error, warning, or hint to display in the editor.
///
/// The editor renders diagnostics as underlined squiggles, with the message
/// shown on hover. Red squiggles = Error, yellow = Warning, blue = Info.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    /// Optional error code, e.g. `"E001"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// A single lexical token from the language's lexer.
///
/// The bridge's `tokenize()` method returns a `Vec` of these. The LSP server
/// uses tokens to provide semantic syntax highlighting (`SemanticTokensProvider`).
///
/// Note: `line` and `column` are 1-based (matching most lexers). The bridge
/// must convert to 0-based when building `SemanticToken` values for the LSP
/// response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// e.g. `"KEYWORD"`, `"IDENTIFIER"`, `"STRING_LIT"`
    pub token_type: String,
    /// The actual source text, e.g. `"let"` or `"myVar"`
    pub value: String,
    /// 1-based line number.
    pub line: i32,
    /// 1-based column number.
    pub column: i32,
}

// ---------------------------------------------------------------------------
// TextEdit
// ---------------------------------------------------------------------------

/// A single text replacement in a document.
///
/// Used by formatting (replace the whole file) and rename (replace each
/// occurrence). `new_text` replaces the content at `range`. If `new_text`
/// is empty, the range is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

// ---------------------------------------------------------------------------
// WorkspaceEdit
// ---------------------------------------------------------------------------

/// Groups `TextEdit`s across potentially multiple files.
///
/// For rename operations that affect a single file, `changes` will have one
/// key. For multi-file projects, a rename may produce edits across many files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    /// `uri` -> list of edits for that file.
    pub changes: HashMap<String, Vec<TextEdit>>,
}

// ---------------------------------------------------------------------------
// HoverResult
// ---------------------------------------------------------------------------

/// The content to show in the hover popup.
///
/// `contents` is Markdown text. VS Code renders it with syntax highlighting,
/// bold/italic, code blocks, etc. `range` is optional -- if set, it highlights
/// the symbol in the editor when the hover popup is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverResult {
    /// Markdown text to display.
    pub contents: String,
    /// Optional: the range of the symbol being hovered.
    pub range: Option<Range>,
}

// ---------------------------------------------------------------------------
// CompletionItemKind
// ---------------------------------------------------------------------------

/// Classifies completion items so the editor can show the right icon
/// (function icon, variable icon, keyword icon, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
    EnumMember = 20,
    Constant = 21,
    Struct = 22,
    Event = 23,
    Operator = 24,
    TypeParameter = 25,
}

// ---------------------------------------------------------------------------
// CompletionItem
// ---------------------------------------------------------------------------

/// A single autocomplete suggestion.
///
/// When the user triggers autocomplete (e.g., by pressing Ctrl+Space or typing
/// after a dot), the editor shows a dropdown list of `CompletionItem`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<CompletionItemKind>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    /// 1 = plain text, 2 = snippet
    pub insert_text_format: Option<i32>,
}

// ---------------------------------------------------------------------------
// SemanticToken
// ---------------------------------------------------------------------------

/// One token's contribution to the semantic highlighting pass.
///
/// Semantic tokens are the "second pass" of syntax highlighting. The editor's
/// grammar-based highlighter (TextMate/tmLanguage) does a fast regex pass first.
/// Semantic tokens layer on top with accurate, context-aware type information.
///
/// `line` and `character` are 0-based. `token_type` and `modifiers` reference
/// entries in the legend returned by `semantic_token_legend()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    /// 0-based line number.
    pub line: i32,
    /// 0-based character offset in UTF-16 code units.
    pub character: i32,
    /// Length in UTF-16 code units.
    pub length: i32,
    /// Must match an entry in `SemanticTokenLegendData::token_types`.
    pub token_type: String,
    /// Subset of `SemanticTokenLegendData::token_modifiers`.
    pub modifiers: Vec<String>,
}

// ---------------------------------------------------------------------------
// SymbolKind
// ---------------------------------------------------------------------------

/// Classifies document symbols for the outline panel.
/// These match the LSP integer codes (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

// ---------------------------------------------------------------------------
// DocumentSymbol
// ---------------------------------------------------------------------------

/// One entry in the document outline panel.
///
/// The outline shows a tree of named symbols (functions, classes, variables).
/// `children` allows nesting: a class symbol can have method symbols as children.
///
/// `range` covers the entire symbol (including its body). `selection_range` is
/// the smaller range of just the symbol's name (used to highlight the name when
/// the user clicks the outline entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}

// ---------------------------------------------------------------------------
// FoldingRange
// ---------------------------------------------------------------------------

/// A collapsible region of the document.
///
/// The editor shows a collapse arrow in the gutter next to `start_line`. When
/// collapsed, lines `start_line+1` through `end_line` are hidden. `kind` is
/// one of `"region"`, `"imports"`, or `"comment"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldingRange {
    /// 0-based start line.
    pub start_line: i32,
    /// 0-based end line.
    pub end_line: i32,
    /// Optional kind: `"region"`, `"imports"`, or `"comment"`.
    pub kind: Option<String>,
}

// ---------------------------------------------------------------------------
// ParameterInformation
// ---------------------------------------------------------------------------

/// One parameter in a function signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

// ---------------------------------------------------------------------------
// SignatureInformation
// ---------------------------------------------------------------------------

/// One function overload's full signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInformation>,
}

// ---------------------------------------------------------------------------
// SignatureHelpResult
// ---------------------------------------------------------------------------

/// Shown as a tooltip when the user is typing a function call.
///
/// It shows the function signature with the current parameter highlighted.
/// `active_signature` indexes into `signatures`. `active_parameter` indexes
/// into that signature's `parameters`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpResult {
    pub signatures: Vec<SignatureInformation>,
    pub active_signature: i32,
    pub active_parameter: i32,
}
