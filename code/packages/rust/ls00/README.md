# ls00 -- Generic LSP Framework (Rust)

A generic Language Server Protocol (LSP) server framework that language-specific
"bridges" plug into. This is the Rust port of the Go implementation at
`code/packages/go/ls00/`.

## What it does

When you open a source file in VS Code and see red squiggles under syntax errors,
autocomplete suggestions, or "Go to Definition" -- none of that is built into the
editor. It comes from a **language server**: a separate process that communicates
with the editor over the Language Server Protocol.

This crate is the *generic* half -- it handles all the protocol boilerplate. A
language author only implements the `LanguageBridge` trait that connects their
lexer/parser to this framework.

## Architecture

```
Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

## How to use

1. Implement the `LanguageBridge` trait for your language.
2. Create an `LspServer` with `LspServer::new(bridge, reader, writer)`.
3. Call `server.serve()` -- it blocks until the editor closes the connection.

```rust
use coding_adventures_ls00::language_bridge::LanguageBridge;
use coding_adventures_ls00::server::LspServer;
use coding_adventures_ls00::types::*;
use std::any::Any;
use std::io::{BufReader, BufWriter};

struct MyBridge;

impl LanguageBridge for MyBridge {
    fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
        Ok(vec![]) // your lexer here
    }

    fn parse(&self, source: &str) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
        Ok((Box::new(()), vec![])) // your parser here
    }

    // Override supports_hover(), hover(), etc. for optional features
}

fn main() {
    let bridge = Box::new(MyBridge);
    let reader = BufReader::new(std::io::stdin());
    let writer = BufWriter::new(std::io::stdout());
    let mut server = LspServer::new(bridge, reader, writer);
    server.serve();
}
```

## Supported LSP Features

| Feature | Method | Bridge Method |
|---------|--------|---------------|
| Diagnostics | `textDocument/publishDiagnostics` | `parse()` (always) |
| Hover | `textDocument/hover` | `hover()` |
| Go to Definition | `textDocument/definition` | `definition()` |
| Find References | `textDocument/references` | `references()` |
| Autocomplete | `textDocument/completion` | `completion()` |
| Rename | `textDocument/rename` | `rename()` |
| Semantic Tokens | `textDocument/semanticTokens/full` | `semantic_tokens()` |
| Document Symbols | `textDocument/documentSymbol` | `document_symbols()` |
| Code Folding | `textDocument/foldingRange` | `folding_ranges()` |
| Signature Help | `textDocument/signatureHelp` | `signature_help()` |
| Formatting | `textDocument/formatting` | `format()` |

## Dependencies

- `coding-adventures-json-rpc` -- JSON-RPC 2.0 transport layer
- `serde` / `serde_json` -- JSON serialization
