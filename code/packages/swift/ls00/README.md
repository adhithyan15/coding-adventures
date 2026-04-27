# ls00 (Swift)

A generic Language Server Protocol (LSP) framework for Swift. Handles all protocol boilerplate so language authors only write a `LanguageBridge`.

## What is the Language Server Protocol?

When you open a source file in VS Code and see red squiggles under syntax errors, autocomplete suggestions, or "Go to Definition" -- none of that is built into the editor. It comes from a *language server*: a separate process communicating with the editor over LSP.

LSP solves the M x N problem: M editors x N languages = M x N integrations. With LSP, each language writes one server, and every LSP-aware editor gets all features automatically.

## Architecture

```
Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

## Usage

1. Implement the `LanguageBridge` protocol for your language:

```swift
import Ls00

struct MyBridge: LanguageBridge {
    func tokenize(source: String) -> ([Token], Error?) {
        // your lexer here
    }
    func parse(source: String) -> (ASTNode?, [Diagnostic], Error?) {
        // your parser here
    }
}
```

2. Optionally implement provider protocols for additional features:

```swift
extension MyBridge: HoverProvider {
    func hover(ast: ASTNode, pos: Position) -> (HoverResult?, Error?) {
        // return hover content
    }
}
```

3. Create and serve:

```swift
let server = LspServer(bridge: MyBridge(), input: stdinData, output: stdoutTarget)
server.serve()
```

## Supported LSP Methods

| Method | Type | Provider |
|--------|------|----------|
| initialize | request | (built-in) |
| shutdown | request | (built-in) |
| exit | notification | (built-in) |
| textDocument/didOpen | notification | (built-in) |
| textDocument/didChange | notification | (built-in) |
| textDocument/didClose | notification | (built-in) |
| textDocument/didSave | notification | (built-in) |
| textDocument/hover | request | HoverProvider |
| textDocument/definition | request | DefinitionProvider |
| textDocument/references | request | ReferencesProvider |
| textDocument/completion | request | CompletionProvider |
| textDocument/rename | request | RenameProvider |
| textDocument/documentSymbol | request | DocumentSymbolsProvider |
| textDocument/semanticTokens/full | request | SemanticTokensProvider |
| textDocument/foldingRange | request | FoldingRangesProvider |
| textDocument/signatureHelp | request | SignatureHelpProvider |
| textDocument/formatting | request | FormatProvider |

## Dependencies

- `JsonRpc` package (via relative path `../json-rpc`)
- `Rpc` package (transitive, via json-rpc)
