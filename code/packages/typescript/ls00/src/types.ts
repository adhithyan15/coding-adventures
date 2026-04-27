/**
 * types.ts -- all shared LSP data types used across the server
 *
 * These types mirror the LSP specification's TypeScript type definitions.
 * The LSP spec lives at:
 * https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
 *
 * # Coordinate System
 *
 * LSP uses a 0-based, line/character coordinate system. Line 0, character 0 is
 * the very first character of the file. This differs from most editors (which
 * display 1-based line numbers) and from our lexer (which emits 1-based tokens).
 * The LanguageBridge is responsible for converting.
 *
 * # UTF-16 Code Units
 *
 * LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
 * Unicode codepoints. This is a historical artifact: VS Code is built on
 * TypeScript, which uses UTF-16 strings internally. In JavaScript, strings
 * ARE already UTF-16 internally, so `string.length` gives UTF-16 code units
 * and `string.charCodeAt()` gives UTF-16 code unit values. This makes offset
 * conversion simpler than in Go, Rust, or Python.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Position and Range -- the two most fundamental LSP types
// ---------------------------------------------------------------------------

/**
 * Position is a cursor position in a document.
 *
 * Both `line` and `character` are 0-based. `character` is measured in UTF-16
 * code units (see the module doc above for why).
 *
 * Example: in the string "hello guitar-emoji world", the guitar emoji occupies
 * UTF-16 characters 6 and 7 (it requires two UTF-16 surrogates). "world" starts
 * at UTF-16 character 8.
 */
export interface Position {
  line: number;
  character: number;
}

/**
 * Range is a span of text in a document, from `start` (inclusive) to `end` (exclusive).
 *
 * Analogy: think of it like a text selection. `start` is where the cursor lands
 * when you click, `end` is where you drag to.
 */
export interface Range {
  start: Position;
  end: Position;
}

/**
 * Location is a position in a specific file.
 *
 * URI uses the "file://" scheme, e.g., "file:///home/user/main.ts".
 */
export interface Location {
  uri: string;
  range: Range;
}

// ---------------------------------------------------------------------------
// Diagnostics -- errors, warnings, and hints
// ---------------------------------------------------------------------------

/**
 * DiagnosticSeverity represents how serious a diagnostic is.
 * These match the LSP integer codes.
 *
 *   1 = Error (red squiggle)
 *   2 = Warning (yellow squiggle)
 *   3 = Information (blue squiggle)
 *   4 = Hint (dim underline or suggestion)
 */
export enum DiagnosticSeverity {
  /** A hard error; the code cannot run or compile. */
  Error = 1,
  /** Potentially problematic, but not blocking. */
  Warning = 2,
  /** Informational message. */
  Information = 3,
  /** A suggestion (e.g., "consider using const"). */
  Hint = 4,
}

/**
 * Diagnostic is an error, warning, or hint to display in the editor.
 *
 * The editor renders diagnostics as underlined squiggles, with the message
 * shown on hover. Red squiggles = Error, yellow = Warning, blue = Info.
 */
export interface Diagnostic {
  range: Range;
  severity: DiagnosticSeverity;
  message: string;
  /** Optional error code (e.g., "E001"). */
  code?: string;
}

// ---------------------------------------------------------------------------
// Tokens -- lexer output
// ---------------------------------------------------------------------------

/**
 * Token is a single lexical token from the language's lexer.
 *
 * The bridge's `tokenize()` method returns an array of these. The LSP server
 * uses tokens to provide semantic syntax highlighting (SemanticTokensProvider).
 *
 * Note: `line` and `column` are 1-based (matching most lexers). The bridge must
 * convert to 0-based when building SemanticToken values for the LSP response.
 */
export interface Token {
  type: string;   // e.g. "KEYWORD", "IDENTIFIER", "STRING_LIT"
  value: string;  // the actual source text, e.g. "let" or "myVar"
  line: number;   // 1-based line number
  column: number; // 1-based column number
}

// ---------------------------------------------------------------------------
// Text edits -- used by formatting and rename
// ---------------------------------------------------------------------------

/**
 * TextEdit is a single text replacement in a document.
 *
 * Used by formatting (replace the whole file) and rename (replace each occurrence).
 * `newText` replaces the content at `range`. If `newText` is empty, the range is deleted.
 */
export interface TextEdit {
  range: Range;
  newText: string;
}

/**
 * WorkspaceEdit groups TextEdits across potentially multiple files.
 *
 * For rename operations that affect a single file, `changes` will have one key.
 * For multi-file projects, a rename may produce edits across many files.
 */
export interface WorkspaceEdit {
  changes: Record<string, TextEdit[]>; // uri -> edits
}

// ---------------------------------------------------------------------------
// Hover -- tooltip on mouse-over
// ---------------------------------------------------------------------------

/**
 * HoverResult is the content to show in the hover popup.
 *
 * `contents` is Markdown text. VS Code renders it with syntax highlighting,
 * bold/italic, code blocks, etc. `range` is optional -- if set, it highlights
 * the symbol in the editor when the hover popup is shown.
 */
export interface HoverResult {
  contents: string; // Markdown
  range?: Range;
}

// ---------------------------------------------------------------------------
// Completion -- autocomplete
// ---------------------------------------------------------------------------

/**
 * CompletionItemKind classifies completion items so the editor can show
 * the right icon (function icon, variable icon, keyword icon, etc.).
 */
export enum CompletionItemKind {
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

/**
 * CompletionItem is a single autocomplete suggestion.
 *
 * When the user triggers autocomplete (e.g., by pressing Ctrl+Space or typing
 * after a dot), the editor shows a dropdown list of CompletionItems.
 */
export interface CompletionItem {
  label: string;
  kind?: CompletionItemKind;
  detail?: string;
  documentation?: string;
  insertText?: string;
  insertTextFormat?: number; // 1=plain, 2=snippet
}

// ---------------------------------------------------------------------------
// Semantic tokens -- accurate syntax highlighting
// ---------------------------------------------------------------------------

/**
 * SemanticToken is one token's contribution to the semantic highlighting pass.
 *
 * Semantic tokens are the "second pass" of syntax highlighting. The editor's
 * grammar-based highlighter (TextMate/tmLanguage) does a fast regex pass first.
 * Semantic tokens layer on top with accurate, context-aware type information.
 *
 * For example, a variable named `string` is just an IDENTIFIER in the grammar
 * pass. But semantic tokens can label it as "variable" so the editor gives it
 * a different color from the built-in keyword "string".
 *
 * `line` and `character` are 0-based. `tokenType` and `modifiers` reference
 * entries in the legend returned by `semanticTokenLegend()`.
 */
export interface SemanticToken {
  line: number;      // 0-based
  character: number; // 0-based, UTF-16 code units
  length: number;    // in UTF-16 code units
  tokenType: string; // must match an entry in the legend's tokenTypes
  modifiers: string[];
}

// ---------------------------------------------------------------------------
// Document symbols -- outline panel
// ---------------------------------------------------------------------------

/**
 * SymbolKind classifies document symbols for the outline panel.
 * These match the LSP integer codes (1-based).
 */
export enum SymbolKind {
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

/**
 * DocumentSymbol is one entry in the document outline panel.
 *
 * The outline shows a tree of named symbols (functions, classes, variables).
 * `children` allows nesting: a class symbol can have method symbols as children.
 *
 * `range` covers the entire symbol (including its body). `selectionRange` is the
 * smaller range of just the symbol's name (used to highlight the name when
 * the user clicks the outline entry).
 */
export interface DocumentSymbol {
  name: string;
  kind: SymbolKind;
  range: Range;
  selectionRange: Range;
  children?: DocumentSymbol[];
}

// ---------------------------------------------------------------------------
// Folding ranges -- collapsible code blocks
// ---------------------------------------------------------------------------

/**
 * FoldingRange is a collapsible region of the document.
 *
 * The editor shows a collapse arrow in the gutter next to `startLine`. When
 * collapsed, lines startLine+1 through endLine are hidden. `kind` is one of
 * "region", "imports", or "comment".
 */
export interface FoldingRange {
  startLine: number; // 0-based
  endLine: number;   // 0-based
  kind?: string;
}

// ---------------------------------------------------------------------------
// Signature help -- function signature tooltip
// ---------------------------------------------------------------------------

/**
 * ParameterInformation is one parameter in a function signature.
 */
export interface ParameterInformation {
  label: string;
  documentation?: string;
}

/**
 * SignatureInformation is one function overload's full signature.
 */
export interface SignatureInformation {
  label: string;
  documentation?: string;
  parameters?: ParameterInformation[];
}

/**
 * SignatureHelpResult is shown as a tooltip when the user is typing a function call.
 *
 * It shows the function signature with the current parameter highlighted.
 * `activeSignature` indexes into `signatures`. `activeParameter` indexes into
 * that signature's `parameters`.
 *
 * Example: typing `foo(a, |` (cursor after the comma) would show Signatures[0]
 * with activeParameter=1 (highlighting the second parameter).
 */
export interface SignatureHelpResult {
  signatures: SignatureInformation[];
  activeSignature: number;
  activeParameter: number;
}

// ---------------------------------------------------------------------------
// Text change -- incremental document updates
// ---------------------------------------------------------------------------

/**
 * TextChange describes one incremental change to a document.
 *
 * If `range` is undefined, `newText` replaces the ENTIRE document content (full sync).
 * If `range` is defined, `newText` replaces just the specified range (incremental sync).
 *
 * The LSP textDocumentSync capability controls which mode the editor uses:
 *   - textDocumentSync=1: full sync (range is always undefined)
 *   - textDocumentSync=2: incremental sync (range specifies what changed)
 *
 * We advertise textDocumentSync=2 (incremental) in our capabilities, but
 * we handle both modes for robustness.
 */
export interface TextChange {
  range?: Range; // undefined = full replacement
  newText: string;
}
