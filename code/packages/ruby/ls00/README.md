# coding_adventures_ls00

Generic Language Server Protocol (LSP) framework for Ruby.

## What is this?

This gem implements the protocol boilerplate that every LSP server needs. A language author only writes a "bridge" object that connects their lexer/parser to this framework. The framework handles:

- JSON-RPC transport over stdio (via the `coding_adventures_json_rpc` gem)
- Document synchronization (open/change/close lifecycle)
- Parse result caching (avoids re-parsing unchanged documents)
- Capability negotiation (only advertises features the bridge supports)
- Semantic token encoding (compact delta format)
- UTF-16 offset conversion (LSP uses UTF-16, Ruby uses UTF-8)

## Architecture

```
Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

## How to use

1. Create a bridge object implementing `tokenize(source)` and `parse(source)` (plus any optional methods like `hover`, `definition`, etc.)
2. Create a server:
   ```ruby
   server = CodingAdventures::Ls00::LspServer.new(bridge, STDIN, STDOUT)
   ```
3. Call `server.serve` -- it blocks until the editor closes the connection.

## Bridge Interface (Duck Typing)

### Required methods

- `tokenize(source)` -- returns an array of `Token` structs
- `parse(source)` -- returns `[ast, [Diagnostic, ...]]`

### Optional methods (checked via `respond_to?`)

- `hover(ast, pos)` -- returns `HoverResult` or `nil`
- `definition(ast, pos, uri)` -- returns `Location` or `nil`
- `references(ast, pos, uri, include_declaration)` -- returns `[Location, ...]`
- `completion(ast, pos)` -- returns `[CompletionItem, ...]`
- `rename(ast, pos, new_name)` -- returns `WorkspaceEdit` or `nil`
- `semantic_tokens(source, tokens)` -- returns `[SemanticToken, ...]`
- `document_symbols(ast)` -- returns `[DocumentSymbol, ...]`
- `folding_ranges(ast)` -- returns `[FoldingRange, ...]`
- `signature_help(ast, pos)` -- returns `SignatureHelpResult` or `nil`
- `format(source)` -- returns `[TextEdit, ...]`

## Dependencies

- `coding_adventures_json_rpc` -- JSON-RPC 2.0 transport layer

## Running tests

```bash
bundle install
bundle exec rake test
```
