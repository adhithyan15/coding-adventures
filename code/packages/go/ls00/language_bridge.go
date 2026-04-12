package ls00

// language_bridge.go — LanguageBridge and optional provider interfaces
//
// # Design Philosophy: Narrow Interfaces
//
// Go's idiomatic interface design favors many small interfaces over one large one.
// This is sometimes called the "interface segregation principle" (from SOLID).
//
// A common Go proverb: "The bigger the interface, the weaker the abstraction."
//
// Compare two approaches:
//
//   APPROACH A — fat interface (one big interface, every method required):
//
//     type LanguageBridge interface {
//         Tokenize(source string) ([]Token, error)
//         Parse(source string) (ASTNode, []Diagnostic, error)
//         Hover(ast ASTNode, pos Position) (*HoverResult, error)
//         Definition(ast ASTNode, pos Position, uri string) (*Location, error)
//         // ... 8 more methods, all REQUIRED
//     }
//
//     Problem: a language that only has a lexer and parser must implement ALL
//     methods, even the symbol-table features it doesn't have yet. It's forced
//     to return errors or nils from 8 methods it doesn't support.
//
//   APPROACH B — narrow interfaces (what we use):
//
//     type LanguageBridge interface {
//         Tokenize(source string) ([]Token, error)
//         Parse(source string) (ASTNode, []Diagnostic, error)
//     }
//     type HoverProvider interface { Hover(...) }
//     type DefinitionProvider interface { Definition(...) }
//     // ...
//
//     At runtime, the server checks: does bridge implement HoverProvider?
//     If yes → advertise hoverProvider:true and call bridge.Hover().
//     If no  → omit hover from capabilities. No stubs required.
//
// This matches the LSP spec's philosophy: capabilities are advertised, not
// assumed. An editor won't even try to ask for hover if the server didn't
// advertise it. The result is that a phase-1 bridge (lexer+parser only) works
// perfectly without stubs.
//
// # Runtime Capability Detection
//
// Detection uses Go's type assertion:
//
//   if hp, ok := bridge.(HoverProvider); ok {
//       result, err = hp.Hover(ast, pos)
//   }
//
// This is O(1) and incurs no reflection cost at runtime beyond the initial
// interface table lookup.

// LanguageBridge is the required minimum interface every language must implement.
//
// Tokenize and Parse are the foundation for all other features:
//   - Tokenize drives semantic token highlighting (accurate syntax coloring)
//   - Parse drives diagnostics, folding, and document symbols
//
// All other features (hover, go-to-definition, etc.) are optional and declared
// as separate interfaces below. The LspServer checks at runtime whether the
// bridge also implements those interfaces.
type LanguageBridge interface {
	// Tokenize lexes the source string and returns the token stream.
	//
	// The tokens are used for semantic highlighting. Each Token carries a Type
	// string (e.g. "KEYWORD", "IDENTIFIER"), its Value, and its 1-based Line
	// and Column position. The bridge is responsible for converting 1-based
	// positions to 0-based before building SemanticToken values.
	Tokenize(source string) ([]Token, error)

	// Parse parses the source string and returns:
	//   - ast:         the parsed abstract syntax tree (may be partial on error)
	//   - diagnostics: parse errors and warnings as LSP Diagnostic objects
	//   - err:         a fatal error (nil if parsing produced any AST at all)
	//
	// Even when there are syntax errors, Parse should return a partial AST.
	// This allows hover, folding, and symbol features to continue working on
	// the valid portions of the file. Good parser design includes error recovery
	// for exactly this reason.
	Parse(source string) (ASTNode, []Diagnostic, error)
}

// ─── Optional Provider Interfaces ────────────────────────────────────────────
//
// Each of the following interfaces represents one optional LSP feature.
// A bridge implements only the features its language supports.
// The server uses type assertions (bridge.(HoverProvider)) to detect support.
//
// None of these interfaces embed LanguageBridge — they are purely additive.
// A bridge struct simply implements whichever methods it needs.

// HoverProvider enables hover tooltips.
//
// When the user moves their mouse over a symbol, the editor sends
// textDocument/hover with the cursor position. The bridge should return
// Markdown text describing the symbol (type, documentation, etc.).
type HoverProvider interface {
	// Hover returns hover information for the AST node at the given position.
	//
	// Returns:
	//   - (*HoverResult, nil) — hover content to display
	//   - (nil, nil)          — no hover info at this position (not an error)
	//   - (nil, error)        — something went wrong
	Hover(ast ASTNode, pos Position) (*HoverResult, error)
}

// DefinitionProvider enables "Go to Definition" (F12 in VS Code).
//
// When the user right-clicks on a symbol and chooses "Go to Definition",
// the editor sends textDocument/definition. The bridge looks up the
// symbol in its symbol table and returns the location where it was declared.
type DefinitionProvider interface {
	// Definition returns the location where the symbol at pos was declared.
	//
	// Returns:
	//   - (*Location, nil) — the declaration location
	//   - (nil, nil)       — symbol not found (not an error)
	//   - (nil, error)     — something went wrong
	Definition(ast ASTNode, pos Position, uri string) (*Location, error)
}

// ReferencesProvider enables "Find All References".
//
// When the user right-clicks and chooses "Find All References", the editor
// sends textDocument/references. The bridge returns every location in the
// file (and optionally the declaration) where the symbol is used.
type ReferencesProvider interface {
	// References returns all uses of the symbol at pos.
	//
	// includeDecl: if true, include the declaration location in the results.
	References(ast ASTNode, pos Position, uri string, includeDecl bool) ([]Location, error)
}

// CompletionProvider enables autocomplete suggestions.
//
// When the user pauses typing or presses Ctrl+Space, the editor sends
// textDocument/completion. The bridge returns a list of valid completions
// at the cursor position (usually all names in scope).
type CompletionProvider interface {
	// Completion returns autocomplete suggestions valid at pos.
	Completion(ast ASTNode, pos Position) ([]CompletionItem, error)
}

// RenameProvider enables symbol rename (F2 in VS Code).
//
// When the user presses F2 on a symbol, the editor sends textDocument/rename.
// The bridge must find all occurrences of the symbol and return text edits
// that replace each one with newName.
type RenameProvider interface {
	// Rename returns the set of text edits needed to rename the symbol at pos.
	Rename(ast ASTNode, pos Position, newName string) (*WorkspaceEdit, error)
}

// SemanticTokensProvider enables accurate syntax highlighting.
//
// The editor's grammar-based highlighter (TextMate/tmLanguage) does a fast
// regex pass. Semantic tokens layer on top with context-aware type information.
//
// The bridge receives the full token stream from the lexer and maps each token
// to a semantic type (keyword, string, number, variable, function, etc.).
// The framework then encodes the result into LSP's compact binary format.
type SemanticTokensProvider interface {
	// SemanticTokens returns semantic token data for the whole document.
	//
	// tokens is the output of Tokenize() — the bridge should use these rather
	// than re-lexing. The returned SemanticTokens must be sorted by line, then
	// by character (ascending), because the LSP encoding is delta-based.
	SemanticTokens(source string, tokens []Token) ([]SemanticToken, error)
}

// DocumentSymbolsProvider enables the document outline panel.
//
// The outline panel (Explorer > OUTLINE in VS Code) shows a tree of named
// symbols in the file: functions, classes, variables, constants, etc.
// The bridge walks the AST looking for declaration nodes.
type DocumentSymbolsProvider interface {
	// DocumentSymbols returns the outline tree for the given AST.
	DocumentSymbols(ast ASTNode) ([]DocumentSymbol, error)
}

// FoldingRangesProvider enables code folding (collapsible blocks).
//
// The editor shows a collapse arrow in the gutter next to any foldable region.
// The bridge typically marks any AST node that spans multiple lines as foldable.
type FoldingRangesProvider interface {
	// FoldingRanges returns collapsible regions derived from the AST structure.
	FoldingRanges(ast ASTNode) ([]FoldingRange, error)
}

// SignatureHelpProvider enables function signature hints.
//
// When the user types the opening parenthesis of a function call, the editor
// sends textDocument/signatureHelp. The bridge returns the function's signature
// with the active parameter highlighted (based on how many commas the user has
// typed so far).
type SignatureHelpProvider interface {
	// SignatureHelp returns signature hint information for the call at pos.
	//
	// Returns:
	//   - (*SignatureHelpResult, nil) — signature data
	//   - (nil, nil)                 — not inside a call expression
	//   - (nil, error)               — something went wrong
	SignatureHelp(ast ASTNode, pos Position) (*SignatureHelpResult, error)
}

// FormatProvider enables document formatting (Format on Save).
//
// When the user saves (or explicitly formats), the editor sends
// textDocument/formatting. The bridge returns a list of text edits that
// transform the source into its canonical formatted form.
type FormatProvider interface {
	// Format returns the text edits needed to format the document.
	//
	// Typically this is a single edit replacing the entire file content.
	Format(source string) ([]TextEdit, error)
}
