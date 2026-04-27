# ls00 -- Generic LSP Framework (Elixir)

A generic Language Server Protocol (LSP) framework that language-specific "bridges" plug into via Elixir behaviours. Port of the Go implementation at `code/packages/go/ls00/`.

## What is the Language Server Protocol?

LSP solves the M x N problem: M editors x N languages = M x N integrations. With LSP, each language writes one server, and every LSP-aware editor gets all features automatically.

This package is the *generic* half -- it handles all the protocol boilerplate. A language author only writes the `Ls00.LanguageBridge` behaviour that connects their lexer/parser to this framework.

## Architecture

```
Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

## Usage

1. Implement the `Ls00.LanguageBridge` behaviour:

```elixir
defmodule MyLanguage.Bridge do
  @behaviour Ls00.LanguageBridge

  @impl true
  def tokenize(source) do
    {:ok, MyLexer.lex(source)}
  end

  @impl true
  def parse(source) do
    {ast, diagnostics} = MyParser.parse(source)
    {:ok, ast, diagnostics}
  end

  # Optional: implement hover, definition, etc.
  @impl true
  def hover(ast, pos) do
    {:ok, %Ls00.Types.HoverResult{contents: "**symbol** info"}}
  end
end
```

2. Start the server:

```elixir
server = Ls00.Server.new(MyLanguage.Bridge, :stdio, :stdio)
Ls00.Server.serve(server)
```

## Features

The framework supports all 10 optional LSP features via `@optional_callbacks`:

| Feature | Callback | LSP Method |
|---------|----------|------------|
| Hover | `hover/2` | textDocument/hover |
| Go to Definition | `definition/3` | textDocument/definition |
| Find References | `references/4` | textDocument/references |
| Autocomplete | `completion/2` | textDocument/completion |
| Rename | `rename/3` | textDocument/rename |
| Semantic Tokens | `semantic_tokens/2` | textDocument/semanticTokens/full |
| Document Symbols | `document_symbols/1` | textDocument/documentSymbol |
| Code Folding | `folding_ranges/1` | textDocument/foldingRange |
| Signature Help | `signature_help/2` | textDocument/signatureHelp |
| Formatting | `format/1` | textDocument/formatting |

Capability detection uses `function_exported?/3` at runtime -- only features the bridge implements are advertised to the editor.

## Dependencies

- `coding_adventures_json_rpc` (path dependency at `../json_rpc`)

## Module Map

| Module | Role |
|--------|------|
| `Ls00` | Top-level module |
| `Ls00.Types` | All LSP types as structs/typespecs |
| `Ls00.LanguageBridge` | `@behaviour` with required + optional callbacks |
| `Ls00.DocumentManager` | Document tracking + UTF-16 conversion |
| `Ls00.ParseCache` | Cache with {uri, version} key |
| `Ls00.Capabilities` | build_capabilities + semantic token encoding |
| `Ls00.LspErrors` | Error code constants |
| `Ls00.Server` | LspServer wiring everything together |
| `Ls00.Handlers` | All handler functions |
