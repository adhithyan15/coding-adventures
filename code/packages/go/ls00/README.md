# ls00 — Generic LSP Server Framework

A generic [Language Server Protocol (LSP)](https://microsoft.github.io/language-server-protocol/) server framework in Go. Language implementations plug into this framework by implementing a narrow `LanguageBridge` interface.

## Layer Position

```
Lexer → Parser → [LanguageBridge] → [ls00: Generic LSP Server] → VS Code / Neovim / Zed
```

This package handles all the protocol boilerplate. A language author only writes the `LanguageBridge` adapter over their existing lexer and parser.

## What is LSP?

LSP ends the M×N problem: instead of each editor integrating each language separately (M editors × N languages = M×N integrations), each language writes one server, and every LSP-aware editor gets all features automatically.

LSP speaks JSON-RPC over stdio. The underlying transport is handled by the `json-rpc` package; this package handles the LSP-specific protocol layer on top.

## Quick Start

```go
import "github.com/coding-adventures/ls00"

// 1. Implement LanguageBridge for your language.
type MyBridge struct{}

func (b *MyBridge) Tokenize(source string) ([]ls00.Token, error) {
    return myLexer.Tokenize(source)
}

func (b *MyBridge) Parse(source string) (ls00.ASTNode, []ls00.Diagnostic, error) {
    ast, errors, err := myParser.Parse(source)
    return ast, errors, err
}

// 2. Optionally implement feature interfaces.
func (b *MyBridge) Hover(ast ls00.ASTNode, pos ls00.Position) (*ls00.HoverResult, error) {
    // look up symbol at pos in ast, return Markdown
}

// 3. Start the server.
func main() {
    server := ls00.NewLspServer(&MyBridge{}, os.Stdin, os.Stdout)
    server.Serve() // blocks until the editor disconnects
}
```

## Features

| LSP Feature | User Experience | Required Interface |
|---|---|---|
| Diagnostics | Red/yellow squiggles | `LanguageBridge.Parse()` |
| Semantic tokens | Accurate syntax highlighting | `SemanticTokensProvider` |
| Hover | Type/doc popup on mouseover | `HoverProvider` |
| Go to Definition | Jump to declaration | `DefinitionProvider` |
| Find References | List all uses | `ReferencesProvider` |
| Autocomplete | Suggestions while typing | `CompletionProvider` |
| Rename | Rename symbol everywhere | `RenameProvider` |
| Document Symbols | File outline panel | `DocumentSymbolsProvider` |
| Code Folding | Collapsible blocks | `FoldingRangesProvider` |
| Signature Help | Parameter hints in calls | `SignatureHelpProvider` |
| Format on Save | Auto-formatting | `FormatProvider` |

Each feature is independently optional. The framework only advertises capabilities the bridge implements.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  LspServer                                                │
│                                                           │
│  ┌──────────────────┐   ┌──────────────────────────────┐ │
│  │ JSON-RPC Server  │   │ DocumentManager              │ │
│  │ (from json-rpc)  │   │ Tracks open files, versions  │ │
│  └────────┬─────────┘   └───────────────┬──────────────┘ │
│           │                             │                 │
│  ┌────────▼─────────────────────────────▼──────────────┐ │
│  │ Feature Handlers                                     │ │
│  │ hover, completion, definition, references, symbols,  │ │
│  │ diagnostics, semantic tokens, rename, folding, ...   │ │
│  └────────────────────────┬─────────────────────────────┘ │
└───────────────────────────┼──────────────────────────────┘
                            │  LanguageBridge interface
┌───────────────────────────▼──────────────────────────────┐
│  Your Language Bridge (LS01)                              │
│  Wraps your lexer + parser + symbol resolver              │
└──────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### Narrow Optional Interfaces (Idiomatic Go)

We use many small interfaces instead of one fat interface. A language bridge only implements the interfaces for features it supports:

```go
// Required minimum:
type LanguageBridge interface {
    Tokenize(source string) ([]Token, error)
    Parse(source string) (ASTNode, []Diagnostic, error)
}

// Optional — implement only what your language supports:
type HoverProvider interface { Hover(ast ASTNode, pos Position) (*HoverResult, error) }
type DefinitionProvider interface { Definition(ast ASTNode, pos Position, uri string) (*Location, error) }
// ... (8 more optional interfaces)
```

The server uses type assertions (`bridge.(HoverProvider)`) at runtime to detect capabilities. No stubs required.

### UTF-16 Offset Handling

LSP measures character positions in UTF-16 code units (a historical artifact from TypeScript). Go strings are UTF-8. The `DocumentManager` converts between them, handling BMP characters (1 UTF-16 unit), surrogate pairs like emoji (2 UTF-16 units = 4 UTF-8 bytes), and multi-byte characters like CJK (3 UTF-8 bytes = 1 UTF-16 unit).

### Parse Cache

The `ParseCache` avoids re-parsing on every keystroke. The cache key is `(uri, version)` — if the version matches, the cached AST is returned immediately.

### Push Diagnostics

After every `didOpen` and `didChange` event, the server proactively pushes `textDocument/publishDiagnostics` without the editor asking. This is the only server-initiated message in the protocol.

## Module Path

```
github.com/coding-adventures/ls00
```

## Dependencies

- `github.com/coding-adventures/json-rpc` (local package)
