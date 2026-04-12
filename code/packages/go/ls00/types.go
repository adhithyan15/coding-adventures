// Package ls00 implements a generic Language Server Protocol (LSP) framework.
//
// # What is the Language Server Protocol?
//
// When you open a source file in VS Code and see red squiggles under syntax
// errors, autocomplete suggestions, or "Go to Definition" — none of that is
// built into the editor. It comes from a *language server*: a separate process
// that communicates with the editor over the Language Server Protocol.
//
// LSP was invented by Microsoft to solve the M×N problem:
//
//	M editors × N languages = M×N integrations to write
//
// With LSP, each language writes one server, and every LSP-aware editor gets
// all features automatically. This package is the *generic* half — it handles
// all the protocol boilerplate. A language author only writes the LanguageBridge
// (see language_bridge.go) that connects their lexer/parser to this framework.
//
// # Architecture
//
//	Lexer → Parser → [LanguageBridge] → [LspServer] → VS Code / Neovim / Emacs
//
// # JSON-RPC over stdio
//
// Like the Debug Adapter Protocol (DAP), LSP speaks JSON-RPC over stdio.
// Each message is Content-Length-framed (same format as HTTP headers). The
// underlying transport is handled by the json-rpc package.
//
// # How to use this package
//
//  1. Implement the LanguageBridge interface (and any optional provider interfaces)
//     for your language.
//  2. Call NewLspServer(bridge, os.Stdin, os.Stdout).
//  3. Call server.Serve() — it blocks until the editor closes the connection.
package ls00

// types.go — all shared LSP data types used across the server
//
// These types mirror the LSP specification's TypeScript type definitions,
// translated to idiomatic Go. The LSP spec lives at:
// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
//
// # Coordinate System
//
// LSP uses a 0-based, line/character coordinate system. Line 0, character 0 is
// the very first character of the file. This differs from most editors (which
// display 1-based line numbers) and from our lexer (which emits 1-based tokens).
// The LanguageBridge is responsible for converting.
//
// # UTF-16 Code Units
//
// LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
// Unicode codepoints. This is a historical artifact: VS Code is built on
// TypeScript, which uses UTF-16 strings internally. See document_manager.go
// for the conversion function and a detailed explanation of why this matters.

// Position is a cursor position in a document.
//
// Both Line and Character are 0-based. Character is measured in UTF-16 code
// units (see the package doc above for why).
//
// Example: in the string "hello 🎸 world", the guitar emoji (🎸) occupies
// UTF-16 characters 6 and 7 (it requires two UTF-16 surrogates). "world" starts
// at UTF-16 character 8.
type Position struct {
	Line      int `json:"line"`
	Character int `json:"character"`
}

// Range is a span of text in a document, from Start (inclusive) to End (exclusive).
//
// Analogy: think of it like a text selection. Start is where the cursor lands
// when you click, End is where you drag to.
type Range struct {
	Start Position `json:"start"`
	End   Position `json:"end"`
}

// Location is a position in a specific file.
//
// URI uses the "file://" scheme, e.g., "file:///home/user/main.go".
type Location struct {
	URI   string `json:"uri"`
	Range Range  `json:"range"`
}

// DiagnosticSeverity represents how serious a diagnostic is.
// These match the LSP integer codes.
type DiagnosticSeverity int

const (
	// SeverityError (1) — a hard error; the code cannot run or compile.
	SeverityError DiagnosticSeverity = 1
	// SeverityWarning (2) — potentially problematic, but not blocking.
	SeverityWarning DiagnosticSeverity = 2
	// SeverityInformation (3) — informational message.
	SeverityInformation DiagnosticSeverity = 3
	// SeverityHint (4) — a suggestion (e.g., "consider using const").
	SeverityHint DiagnosticSeverity = 4
)

// Diagnostic is an error, warning, or hint to display in the editor.
//
// The editor renders diagnostics as underlined squiggles, with the message
// shown on hover. Red squiggles = Error, yellow = Warning, blue = Info.
type Diagnostic struct {
	Range    Range              `json:"range"`
	Severity DiagnosticSeverity `json:"severity"`
	Message  string             `json:"message"`
	Code     string             `json:"code,omitempty"` // optional: e.g. "E001"
}

// Token is a single lexical token from the language's lexer.
//
// The bridge's Tokenize() method returns a slice of these. The LSP server uses
// tokens to provide semantic syntax highlighting (SemanticTokensProvider).
//
// Note: Line and Column are 1-based (matching most lexers). The bridge must
// convert to 0-based when building SemanticToken values for the LSP response.
type Token struct {
	Type   string // e.g. "KEYWORD", "IDENTIFIER", "STRING_LIT"
	Value  string // the actual source text, e.g. "let" or "myVar"
	Line   int    // 1-based line number
	Column int    // 1-based column number
}

// ASTNode is the abstract syntax tree produced by the language's parser.
//
// We use an empty interface here because each language's parser returns its
// own concrete AST type. The LanguageBridge is responsible for accepting this
// interface and downcasting to the concrete type it knows about.
//
// Why not use a generic? In Go generics, each instantiation is a separate type.
// An interface{} allows any language bridge to store any AST without the
// framework needing to know the concrete type at compile time. The bridge does
// the type assertion internally.
type ASTNode interface{}

// TextEdit is a single text replacement in a document.
//
// Used by formatting (replace the whole file) and rename (replace each occurrence).
// NewText replaces the content at Range. If NewText is empty, the range is deleted.
type TextEdit struct {
	Range   Range  `json:"range"`
	NewText string `json:"newText"`
}

// WorkspaceEdit groups TextEdits across potentially multiple files.
//
// For rename operations that affect a single file, Changes will have one key.
// For multi-file projects, a rename may produce edits across many files.
type WorkspaceEdit struct {
	Changes map[string][]TextEdit `json:"changes"` // uri → edits
}

// HoverResult is the content to show in the hover popup.
//
// Contents is Markdown text. VS Code renders it with syntax highlighting,
// bold/italic, code blocks, etc. Range is optional — if set, it highlights
// the symbol in the editor when the hover popup is shown.
type HoverResult struct {
	Contents string `json:"contents"` // Markdown
	Range    *Range `json:"range,omitempty"`
}

// CompletionItemKind classifies completion items so the editor can show
// the right icon (function icon, variable icon, keyword icon, etc.).
type CompletionItemKind int

const (
	CompletionText          CompletionItemKind = 1
	CompletionMethod        CompletionItemKind = 2
	CompletionFunction      CompletionItemKind = 3
	CompletionConstructor   CompletionItemKind = 4
	CompletionField         CompletionItemKind = 5
	CompletionVariable      CompletionItemKind = 6
	CompletionClass         CompletionItemKind = 7
	CompletionInterface     CompletionItemKind = 8
	CompletionModule        CompletionItemKind = 9
	CompletionProperty      CompletionItemKind = 10
	CompletionUnit          CompletionItemKind = 11
	CompletionValue         CompletionItemKind = 12
	CompletionEnum          CompletionItemKind = 13
	CompletionKeyword       CompletionItemKind = 14
	CompletionSnippet       CompletionItemKind = 15
	CompletionColor         CompletionItemKind = 16
	CompletionFile          CompletionItemKind = 17
	CompletionReference     CompletionItemKind = 18
	CompletionFolder        CompletionItemKind = 19
	CompletionEnumMember    CompletionItemKind = 20
	CompletionConstant      CompletionItemKind = 21
	CompletionStruct        CompletionItemKind = 22
	CompletionEvent         CompletionItemKind = 23
	CompletionOperator      CompletionItemKind = 24
	CompletionTypeParameter CompletionItemKind = 25
)

// CompletionItem is a single autocomplete suggestion.
//
// When the user triggers autocomplete (e.g., by pressing Ctrl+Space or typing
// after a dot), the editor shows a dropdown list of CompletionItems.
type CompletionItem struct {
	Label            string             `json:"label"`
	Kind             CompletionItemKind `json:"kind,omitempty"`
	Detail           string             `json:"detail,omitempty"`
	Documentation    string             `json:"documentation,omitempty"`
	InsertText       string             `json:"insertText,omitempty"`
	InsertTextFormat int                `json:"insertTextFormat,omitempty"` // 1=plain, 2=snippet
}

// SemanticToken is one token's contribution to the semantic highlighting pass.
//
// Semantic tokens are the "second pass" of syntax highlighting. The editor's
// grammar-based highlighter (TextMate/tmLanguage) does a fast regex pass first.
// Semantic tokens layer on top with accurate, context-aware type information.
//
// For example, a variable named `string` is just an IDENTIFIER in the grammar
// pass. But semantic tokens can label it as "variable" so the editor gives it
// a different color from the built-in keyword "string".
//
// Line and Character are 0-based. TokenType and Modifiers reference entries in
// the legend returned by SemanticTokenLegend() (see capabilities.go).
type SemanticToken struct {
	Line      int      // 0-based
	Character int      // 0-based, UTF-16 code units
	Length    int      // in UTF-16 code units
	TokenType string   // must match an entry in SemanticTokenLegend().TokenTypes
	Modifiers []string // subset of SemanticTokenLegend().TokenModifiers
}

// SymbolKind classifies document symbols for the outline panel.
// These match the LSP integer codes (1-based).
type SymbolKind int

const (
	SymbolFile          SymbolKind = 1
	SymbolModule        SymbolKind = 2
	SymbolNamespace     SymbolKind = 3
	SymbolPackage       SymbolKind = 4
	SymbolClass         SymbolKind = 5
	SymbolMethod        SymbolKind = 6
	SymbolProperty      SymbolKind = 7
	SymbolField         SymbolKind = 8
	SymbolConstructor   SymbolKind = 9
	SymbolEnum          SymbolKind = 10
	SymbolInterface     SymbolKind = 11
	SymbolFunction      SymbolKind = 12
	SymbolVariable      SymbolKind = 13
	SymbolConstant      SymbolKind = 14
	SymbolString        SymbolKind = 15
	SymbolNumber        SymbolKind = 16
	SymbolBoolean       SymbolKind = 17
	SymbolArray         SymbolKind = 18
	SymbolObject        SymbolKind = 19
	SymbolKey           SymbolKind = 20
	SymbolNull          SymbolKind = 21
	SymbolEnumMember    SymbolKind = 22
	SymbolStruct        SymbolKind = 23
	SymbolEvent         SymbolKind = 24
	SymbolOperator      SymbolKind = 25
	SymbolTypeParameter SymbolKind = 26
)

// DocumentSymbol is one entry in the document outline panel.
//
// The outline shows a tree of named symbols (functions, classes, variables).
// Children allows nesting: a class symbol can have method symbols as children.
//
// Range covers the entire symbol (including its body). SelectionRange is the
// smaller range of just the symbol's name (used to highlight the name when
// the user clicks the outline entry).
type DocumentSymbol struct {
	Name           string           `json:"name"`
	Kind           SymbolKind       `json:"kind"`
	Range          Range            `json:"range"`
	SelectionRange Range            `json:"selectionRange"`
	Children       []DocumentSymbol `json:"children,omitempty"`
}

// FoldingRange is a collapsible region of the document.
//
// The editor shows a collapse arrow in the gutter next to StartLine. When
// collapsed, lines StartLine+1 through EndLine are hidden. Kind is one of
// "region", "imports", or "comment".
type FoldingRange struct {
	StartLine int    `json:"startLine"` // 0-based
	EndLine   int    `json:"endLine"`   // 0-based
	Kind      string `json:"kind,omitempty"`
}

// ParameterInformation is one parameter in a function signature.
type ParameterInformation struct {
	Label         string `json:"label"`
	Documentation string `json:"documentation,omitempty"`
}

// SignatureInformation is one function overload's full signature.
type SignatureInformation struct {
	Label         string                 `json:"label"`
	Documentation string                 `json:"documentation,omitempty"`
	Parameters    []ParameterInformation `json:"parameters,omitempty"`
}

// SignatureHelpResult is shown as a tooltip when the user is typing a function call.
//
// It shows the function signature with the current parameter highlighted.
// ActiveSignature indexes into Signatures. ActiveParameter indexes into
// that signature's Parameters.
//
// Example: typing `foo(a, |` (cursor after the comma) would show Signatures[0]
// with ActiveParameter=1 (highlighting the second parameter).
type SignatureHelpResult struct {
	Signatures      []SignatureInformation `json:"signatures"`
	ActiveSignature int                   `json:"activeSignature"`
	ActiveParameter int                   `json:"activeParameter"`
}
