# LS00 — Language Server Framework

## Overview

When you open a source file in VS Code and see red squiggles under syntax errors, autocomplete suggestions as you type, "Go to Definition" on a function call, or a rename that updates every occurrence of a variable — none of that is built into the editor. It comes from a **language server**: a separate process that the editor communicates with over the **Language Server Protocol (LSP)**.

This spec defines a **generic LSP framework** that every language in the toolchain can plug into. A language author provides a thin adapter (defined in `LS01`) over their existing lexer and parser packages. The framework handles all the protocol boilerplate, document state management, and feature coordination.

The result: every language gets editor intelligence **for free**, the same way every language gets debugger support through the DAP framework (`05e`).

## Layer Position

```
Lexer → Parser → [LS01 Language Bridge] → [THIS: Generic LSP Server] → VS Code
```

**Input from:** LS01 Language Bridge (language-specific adapter over lexer/parser packages).
**Output to:** Any LSP-compatible editor: VS Code, Neovim, Emacs, IntelliJ, Zed.

## What is LSP?

LSP was created by Microsoft for the same reason as DAP: to end the M×N problem. Before LSP, each editor had to implement language intelligence for each language — M editors × N languages = M×N integrations. With LSP, each language implements one server, and every editor that speaks LSP gets it.

Like DAP, LSP is JSON-RPC over stdio (or TCP):

```
Editor → Server:  { "jsonrpc": "2.0", "id": 1,  "method": "textDocument/hover",      "params": {...} }
Server → Editor:  { "jsonrpc": "2.0", "id": 1,  "result": { "contents": "integer" } }

Server → Editor:  { "jsonrpc": "2.0",             "method": "textDocument/publishDiagnostics", "params": {...} }
```

Requests have IDs and get responses. Notifications (like diagnostics) have no ID and get no response.

## Feature Coverage

| LSP Feature | What the user sees | Required from language |
|---|---|---|
| Diagnostics | Red/yellow squiggles for errors and warnings | Parser error list |
| Semantic tokens | Accurate syntax highlighting (beyond regex) | Lexer token types |
| Hover | Type/doc popup when mousing over a symbol | Symbol table |
| Go to Definition | Jump to where a variable/function is defined | Symbol table |
| Find References | All uses of a symbol | Symbol table |
| Completion | Autocomplete suggestions | Scope-aware symbol enumeration |
| Rename | Rename a symbol everywhere it appears | Symbol table |
| Document Symbols | File outline (functions, classes, variables) | AST |
| Code Folding | Collapse blocks and functions | AST |
| Signature Help | Function parameter hints while typing a call | Symbol table |
| Formatting | Auto-format on save | Optional: formatter |

Each feature is independently optional. The framework advertises only the capabilities the language bridge implements.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Generic LSP Server (this spec)                          │
│                                                          │
│  ┌──────────────────┐   ┌──────────────────────────────┐ │
│  │ Protocol Layer   │   │ Document Manager             │ │
│  │ JSON-RPC read/   │   │ Tracks open files, versions, │ │
│  │ write, dispatch  │   │ incremental parse cache      │ │
│  └────────┬─────────┘   └───────────────┬──────────────┘ │
│           │                             │                 │
│  ┌────────▼─────────────────────────────▼──────────────┐ │
│  │ Feature Handlers                                     │ │
│  │ hover, completion, definition, references, symbols,  │ │
│  │ diagnostics, semantic tokens, rename, folding, ...   │ │
│  └────────────────────────┬─────────────────────────────┘ │
└───────────────────────────┼──────────────────────────────┘
                            │  LanguageBridge behaviour
                            │  (per-language implementation)
┌───────────────────────────▼──────────────────────────────┐
│  Language Bridge (LS01)                                  │
│  Wraps the language's lexer + parser + symbol resolver   │
└──────────────────────────────────────────────────────────┘
```

The generic server has no knowledge of any specific language. The language bridge is the only language-specific code.

## The LanguageBridge Behaviour

The bridge is defined as an Elixir behaviour (interface). Every language implements it:

```elixir
defmodule LanguageServer.LanguageBridge do
  @moduledoc """
  The interface every language must implement to plug into the generic LSP server.
  Implement only the callbacks your language supports. Unimplemented callbacks
  return {:not_supported} and the framework omits that capability from the
  server's capabilities advertisement.
  """

  @type source      :: String.t()
  @type position    :: %{line: non_neg_integer(), character: non_neg_integer()}
  @type range       :: %{start: position(), end: position()}
  @type uri         :: String.t()

  # ── Required ──────────────────────────────────────────────────────────────

  @doc "Tokenise source and return tokens with type + position information."
  @callback tokenize(source()) :: {:ok, [token()]} | {:error, String.t()}

  @doc "Parse source into an AST and return any diagnostics found."
  @callback parse(source()) :: {:ok, ast(), [diagnostic()]} | {:error, String.t()}

  # ── Optional (return {:not_supported} to omit from capabilities) ──────────

  @doc "Return all named symbols in the document (for the outline panel)."
  @callback document_symbols(ast()) :: {:ok, [document_symbol()]} | :not_supported

  @doc "Return hover information for the AST node at the given position."
  @callback hover(ast(), position()) :: {:ok, hover_result()} | {:none} | :not_supported

  @doc "Return the definition location of the symbol at the given position."
  @callback definition(ast(), position(), uri()) :: {:ok, location()} | {:none} | :not_supported

  @doc "Return all references to the symbol at the given position."
  @callback references(ast(), position(), uri(), include_declaration :: boolean()) ::
    {:ok, [location()]} | :not_supported

  @doc "Return completion items valid at the given position."
  @callback completion(ast(), position()) :: {:ok, [completion_item()]} | :not_supported

  @doc "Rename the symbol at position to new_name, across the file."
  @callback rename(ast(), position(), new_name :: String.t()) ::
    {:ok, workspace_edit()} | {:error, String.t()} | :not_supported

  @doc "Return semantic token data for the whole document."
  @callback semantic_tokens(source(), [token()]) ::
    {:ok, [semantic_token()]} | :not_supported

  @doc "Return folding ranges (collapsible regions) for the document."
  @callback folding_ranges(ast()) :: {:ok, [folding_range()]} | :not_supported

  @doc "Return signature help for a function call at the given position."
  @callback signature_help(ast(), position()) ::
    {:ok, signature_help_result()} | {:none} | :not_supported

  @doc "Format the entire document."
  @callback format(source()) :: {:ok, [text_edit()]} | :not_supported
end
```

### Type definitions

```elixir
@type token :: %{
  type:   String.t(),       # e.g. "KEYWORD", "IDENTIFIER", "STRING_LIT"
  value:  String.t(),
  line:   pos_integer(),    # 1-based
  column: pos_integer()     # 1-based
}

@type ast :: CodingAdventures.Parser.ASTNode.t()  # the existing ASTNode struct

@type diagnostic :: %{
  range:    range(),
  severity: :error | :warning | :information | :hint,
  message:  String.t(),
  code:     String.t() | nil
}

@type document_symbol :: %{
  name:           String.t(),
  kind:           symbol_kind(),
  range:          range(),
  selection_range: range(),
  children:       [document_symbol()]
}

@type symbol_kind ::
  :file | :module | :namespace | :package | :class | :method | :property |
  :field | :constructor | :enum | :interface | :function | :variable |
  :constant | :string | :number | :boolean | :array | :object | :key |
  :null | :enum_member | :struct | :event | :operator | :type_parameter

@type hover_result :: %{
  contents: String.t(),   # markdown
  range:    range() | nil
}

@type location :: %{uri: uri(), range: range()}

@type completion_item :: %{
  label:            String.t(),
  kind:             completion_kind(),
  detail:           String.t() | nil,
  documentation:    String.t() | nil,
  insert_text:      String.t() | nil,
  insert_text_format: :plain | :snippet
}

@type semantic_token :: %{
  line:        non_neg_integer(),  # 0-based (LSP convention)
  character:   non_neg_integer(),
  length:      pos_integer(),
  token_type:  String.t(),
  modifiers:   [String.t()]
}
```

## Document Manager

The document manager is the most important non-obvious piece. When the user types a single character, the editor does not re-send the entire file — it sends an **incremental change** (what changed, where). The document manager applies changes and maintains the current text of each open file.

```
Editor opens file:   didOpen   → document_manager stores text at version 1
User types "X":     didChange  → document_manager applies delta → version 2
User saves:         didSave    → (optional: trigger format)
User closes:        didClose   → document_manager removes entry
```

```elixir
defmodule LanguageServer.DocumentManager do
  defstruct documents: %{}   # uri → %{text: String.t(), version: integer()}

  def open(manager, uri, text, version) do
    Map.put(manager.documents, uri, %{text: text, version: version})
  end

  def apply_changes(manager, uri, changes, new_version) do
    doc = manager.documents[uri]
    new_text = Enum.reduce(changes, doc.text, fn change, text ->
      apply_change(text, change)
    end)
    put_in(manager.documents[uri], %{text: new_text, version: new_version})
  end

  # Change can be full-document replacement or incremental (range + new text)
  defp apply_change(text, %{"text" => new_text}), do: new_text
  defp apply_change(text, %{"range" => range, "text" => new_text}) do
    # Convert LSP range (0-based line/char) to byte offsets, then splice
    {start_byte, end_byte} = range_to_bytes(text, range)
    String.slice(text, 0, start_byte) <> new_text <> String.slice(text, end_byte..-1)
  end
end
```

## Parse Cache

Re-parsing the file on every keystroke would be expensive for large files. The parse cache avoids redundant work:

```elixir
defmodule LanguageServer.ParseCache do
  # Keyed by {uri, version} — if the version matches, return cached result.
  # If not, re-parse and cache the new result, evicting the old one.
  defstruct cache: %{}

  def get_or_parse(cache, uri, version, source, bridge) do
    key = {uri, version}
    case Map.get(cache.cache, key) do
      nil ->
        # Cache miss: parse and store
        result = bridge.parse(source)
        new_cache = Map.put(cache.cache, key, result)
        {result, %{cache | cache: new_cache}}
      cached ->
        {cached, cache}
    end
  end
end
```

The cache key includes the version number, so a stale parse result is never returned after the document changes.

## Feature Handler: Diagnostics

Diagnostics are pushed *proactively* — the server does not wait for the editor to ask. Whenever a document changes, the server re-parses and publishes the new diagnostic list:

```elixir
def handle_did_change(server, uri, changes, version) do
  server = DocumentManager.apply_changes(server.doc_manager, uri, changes, version)
  text   = DocumentManager.get_text(server.doc_manager, uri)

  {parse_result, server} = ParseCache.get_or_parse(
    server.parse_cache, uri, version, text, server.bridge
  )

  diagnostics = case parse_result do
    {:ok, _ast, diags} -> diags
    {:error, msg}      -> [%{range: full_range(), severity: :error, message: msg}]
  end

  send_notification(server, "textDocument/publishDiagnostics", %{
    uri:         uri,
    version:     version,
    diagnostics: Enum.map(diagnostics, &to_lsp_diagnostic/1)
  })
end
```

## Feature Handler: Hover

```elixir
def handle_hover(server, uri, position) do
  text = DocumentManager.get_text(server.doc_manager, uri)
  {parse_result, _} = ParseCache.get_or_parse(...)

  case parse_result do
    {:ok, ast, _} ->
      case server.bridge.hover(ast, position) do
        {:ok, %{contents: md, range: range}} ->
          {:ok, %{"contents" => %{"kind" => "markdown", "value" => md},
                  "range"    => to_lsp_range(range)}}
        {:none}         -> {:ok, nil}
        :not_supported  -> {:ok, nil}
      end
    _ -> {:ok, nil}
  end
end
```

## Feature Handler: Semantic Tokens

Semantic tokens are the "second pass" of syntax highlighting. The editor's grammar-based highlighter (TextMate/tmLanguage) does a fast regex pass; semantic tokens layer on top with accurate, context-aware type information.

For example, a variable `string` might be highlighted as an identifier by the grammar pass, but the semantic token pass can label it `:variable` so it gets a different colour from the built-in keyword `string`.

The LSP semantic tokens protocol requires a compact binary encoding (a flat array of integers rather than JSON objects). The framework handles this encoding:

```elixir
# bridge returns:
# [%{line: 5, character: 4, length: 3, token_type: "variable", modifiers: ["readonly"]}]

def handle_semantic_tokens_full(server, uri) do
  text   = DocumentManager.get_text(server.doc_manager, uri)
  tokens = bridge.tokenize(text)  # reuse the lexer

  case server.bridge.semantic_tokens(text, tokens) do
    {:ok, sem_tokens} ->
      data = encode_semantic_tokens(sem_tokens)  # → flat integer array
      {:ok, %{"data" => data}}
    :not_supported ->
      {:ok, %{"data" => []}}
  end
end

# LSP encodes semantic tokens as a flat array of 5-tuples:
# [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask]
defp encode_semantic_tokens(tokens) do
  sorted = Enum.sort_by(tokens, &{&1.line, &1.character})
  {data, _prev} = Enum.reduce(sorted, {[], {0, 0}}, fn tok, {acc, {prev_line, prev_char}} ->
    delta_line = tok.line - prev_line
    delta_char = if delta_line == 0, do: tok.character - prev_char, else: tok.character
    entry = [delta_line, delta_char, tok.length,
             token_type_index(tok.token_type),
             token_modifier_mask(tok.modifiers)]
    {acc ++ entry, {tok.line, tok.character}}
  end)
  data
end
```

## Capabilities Advertisement

During the `initialize` handshake, the server tells the editor which features it supports. The framework builds this dynamically based on which bridge callbacks are implemented:

```elixir
def build_capabilities(bridge) do
  %{
    "textDocumentSync"          => 2,  # incremental
    "hoverProvider"             => implements?(bridge, :hover),
    "definitionProvider"        => implements?(bridge, :definition),
    "referencesProvider"        => implements?(bridge, :references),
    "documentSymbolProvider"    => implements?(bridge, :document_symbols),
    "completionProvider"        => if implements?(bridge, :completion) do
                                     %{"triggerCharacters" => [" ", "."]}
                                   end,
    "renameProvider"            => implements?(bridge, :rename),
    "foldingRangeProvider"      => implements?(bridge, :folding_ranges),
    "signatureHelpProvider"     => if implements?(bridge, :signature_help) do
                                     %{"triggerCharacters" => ["(", ","]}
                                   end,
    "documentFormattingProvider" => implements?(bridge, :format),
    "semanticTokensProvider"    => if implements?(bridge, :semantic_tokens) do
                                     %{"legend" => semantic_token_legend(),
                                       "full"   => true}
                                   end,
  }
  |> Enum.reject(fn {_, v} -> v == false or v == nil end)
  |> Map.new()
end

defp implements?(bridge, callback) do
  # Call the callback with dummy args; if it returns :not_supported, the
  # feature is not available.
  # In practice this is checked at startup via bridge module attributes.
  function_exported?(bridge, callback, expected_arity(callback))
end
```

## Server Lifecycle

```
Editor                         LSP Server
  │                                │
  ├──initialize──────────────────▶ │  server stores client capabilities
  │ ◀──────────────initialized────  │  server sends its capabilities
  │                                │
  ├──initialized (notification)──▶ │  handshake complete
  │                                │
  ├──textDocument/didOpen─────────▶ │  document manager stores file
  │                                ├──parse → send diagnostics
  │                                │
  ├──textDocument/didChange───────▶ │  doc manager applies change
  │                                ├──re-parse → send updated diagnostics
  │                                │
  ├──textDocument/hover───────────▶ │
  │ ◀──────────────hover result────  │
  │                                │
  ├──shutdown─────────────────────▶ │
  │ ◀──────────────ok──────────────  │
  │                                │
  ├──exit (notification)──────────▶ │  server exits
```

## VS Code Extension

Adding LSP support to a VS Code extension adds only a few lines on top of the debug adapter extension (`05e`):

```json
{
  "contributes": {
    "languages": [{ "id": "basic", "extensions": [".bas"] }],
    "grammars": [{
      "language": "basic",
      "scopeName": "source.basic",
      "path": "./syntaxes/basic.tmLanguage.json"
    }]
  }
}
```

```typescript
import * as lsp from 'vscode-languageclient/node';

export function activate(context: vscode.ExtensionContext) {
    const serverOptions: lsp.ServerOptions = {
        command: context.asAbsolutePath('./lsp/basic_language_server'),
        args: []
    };
    const clientOptions: lsp.LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'basic' }]
    };
    const client = new lsp.LanguageClient('basic-lsp', 'BASIC Language Server', serverOptions, clientOptions);
    client.start();
    context.subscriptions.push(client);
}
```

The `vscode-languageclient` npm package handles all the JSON-RPC wiring. The extension just says "here is the server binary, here are the file types" — nothing more.

## Reuse Across Languages

The pattern mirrors the debug adapter:

| What | Language-specific? | Defined where |
|---|---|---|
| JSON-RPC protocol handling | No — generic | This spec |
| Document manager | No — generic | This spec |
| Parse cache | No — generic | This spec |
| Feature handlers | No — generic | This spec |
| LanguageBridge impl | Yes — one per language | LS01 |
| VS Code extension | Yes — one per language | Per-language |

A new language author writes only the LanguageBridge module. The framework handles the rest.
