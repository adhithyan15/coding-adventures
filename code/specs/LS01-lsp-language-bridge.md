# LS01 — LSP Language Bridge

## Overview

The generic LSP framework (`LS00`) knows nothing about any specific language. The **language bridge** is the adapter that connects a language's existing lexer and parser packages to the framework's `LanguageBridge` behaviour.

This spec describes:
1. How each bridge callback maps to the existing lexer/parser APIs
2. What the bridge can deliver with just a lexer and parser (no extra infrastructure)
3. What requires a symbol table (a new, per-language layer)
4. A complete worked example: a BASIC language bridge

The existing lexer and parser APIs are established in specs `02-lexer.md` and `03-parser.md`. Every lexer exposes `tokenize(source) -> {:ok, [Token.t()]}`. Every parser exposes `parse(source) -> {:ok, ASTNode.t()}`. The bridge is a thin mapping from these outputs to LSP concepts.

## What You Get for Free (Lexer + Parser Only)

Before writing a single symbol table, a language bridge built on just the lexer and parser already provides:

| Feature | Powered by |
|---|---|
| Diagnostics (syntax errors) | Parser error list |
| Semantic tokens (accurate highlighting) | Lexer token types |
| Document symbols (outline panel) | AST walk |
| Code folding | AST structure |

These four features cover the most visible editor intelligence for a new language. They require zero knowledge of types, scopes, or variable resolution.

## What Requires a Symbol Table

| Feature | What's needed |
|---|---|
| Hover (type info) | Symbol resolution + type inference |
| Go to Definition | Symbol table: name → definition location |
| Find References | Symbol table: name → all use locations |
| Rename | All references, validated (no conflicts) |
| Completion | Scope-aware symbol enumeration |
| Signature Help | Function signature lookup |

A **symbol table** is built during semantic analysis — a pass over the AST that builds a map from each name to its declaration location, type, and scope. This is the only additional infrastructure needed beyond the existing lexer/parser stack.

## Mapping Lexer Tokens to Semantic Tokens

The lexer already assigns a type to every token (`"VAR"`, `"IDENTIFIER"`, `"STRING_LIT"`, `"NUMBER"`, etc.). The bridge maps these types to LSP's semantic token vocabulary.

LSP defines a standard set of token types and modifiers:

**Token types:** `namespace`, `type`, `class`, `enum`, `interface`, `struct`, `typeParameter`, `parameter`, `variable`, `property`, `enumMember`, `event`, `function`, `method`, `macro`, `keyword`, `modifier`, `comment`, `string`, `number`, `regexp`, `operator`, `decorator`

**Token modifiers:** `declaration`, `definition`, `readonly`, `static`, `deprecated`, `abstract`, `async`, `modification`, `documentation`, `defaultLibrary`

The bridge defines a mapping table:

```elixir
defmodule MyLanguage.LspBridge do
  # Map lexer token types to LSP semantic token types.
  # Tokens NOT in this map are omitted from semantic tokens
  # (the editor's grammar-based highlighter handles them).
  @semantic_token_map %{
    # Keywords
    "LET"       => {"keyword", []},
    "PRINT"     => {"keyword", []},
    "IF"        => {"keyword", []},
    "THEN"      => {"keyword", []},
    "ELSE"      => {"keyword", []},
    "GOTO"      => {"keyword", []},
    "GOSUB"     => {"keyword", []},
    "RETURN"    => {"keyword", []},
    "FOR"       => {"keyword", []},
    "TO"        => {"keyword", []},
    "STEP"      => {"keyword", []},
    "NEXT"      => {"keyword", []},
    "END"       => {"keyword", []},
    "DIM"       => {"keyword", []},
    "FUNCTION"  => {"keyword", []},
    "SUB"       => {"keyword", []},

    # Literals
    "STRING_LIT" => {"string", []},
    "NUMBER"     => {"number", []},
    "FLOAT"      => {"number", []},

    # Identifiers (enriched by symbol table if available)
    "NAME"       => {"variable", []},
    "FUNC_NAME"  => {"function", []},

    # Operators and punctuation are handled by grammar — not listed here
  }

  @impl LanguageServer.LanguageBridge
  def semantic_tokens(_source, tokens) do
    sem_tokens =
      tokens
      |> Enum.reject(&(&1.type == "EOF"))
      |> Enum.flat_map(fn tok ->
        case Map.get(@semantic_token_map, tok.type) do
          nil -> []   # not a semantic token, skip
          {type, modifiers} ->
            [%{
              line:       tok.line - 1,        # LSP is 0-based; lexer is 1-based
              character:  tok.column - 1,
              length:     String.length(tok.value),
              token_type: type,
              modifiers:  modifiers
            }]
        end
      end)
    {:ok, sem_tokens}
  end
end
```

The key insight: **the lexer already does the hard work**. Token types are already categorised at the grammar level. The bridge is just a translation table.

## Mapping Parse Errors to Diagnostics

The parser returns errors with line and column information. The bridge converts these to LSP diagnostics:

```elixir
@impl LanguageServer.LanguageBridge
def parse(source) do
  # The lexer step: {:ok, tokens} | {:error, msg}
  with {:ok, tokens} <- MyLanguage.Lexer.tokenize(source) do
    case MyLanguage.Parser.parse_tokens(tokens) do
      {:ok, ast} ->
        {:ok, ast, []}    # no diagnostics

      {:ok, ast, errors} ->
        # Parser recovered from errors but still produced an AST
        diagnostics = Enum.map(errors, &to_diagnostic/1)
        {:ok, ast, diagnostics}

      {:error, %{message: msg, line: line, column: col}} ->
        diag = %{
          range:    %{start: %{line: line - 1, character: col - 1},
                      end:   %{line: line - 1, character: col}},
          severity: :error,
          message:  msg,
          code:     nil
        }
        # Return a partial/empty AST so other features still work
        {:ok, empty_ast(), [diag]}
    end
  end
end

defp to_diagnostic(%{message: msg, line: line, column: col, severity: sev}) do
  %{
    range:    %{start: %{line: line - 1, character: col - 1},
                end:   %{line: line - 1, character: col + 1}},
    severity: sev || :error,
    message:  msg
  }
end
```

**Error recovery matters**: if a parse error prevents the AST from being built, the bridge should still return a partial AST so that hover, folding, and symbol features continue working on the valid portions of the file. Good parser design (spec `03`) includes error recovery for this reason.

## Extracting Document Symbols from the AST

Document symbols power the **Outline** panel — the tree of functions, variables, and classes shown in the sidebar. They're extracted by walking the AST looking for declaration nodes.

```elixir
@impl LanguageServer.LanguageBridge
def document_symbols(ast) do
  symbols = extract_symbols(ast, [])
  {:ok, symbols}
end

defp extract_symbols(%ASTNode{rule_name: "function_def"} = node, _acc) do
  name_token = ASTNode.find_nodes(node, "function_name")
               |> List.first()
               |> ASTNode.token()
  body       = ASTNode.find_nodes(node, "function_body") |> List.first()

  children = if body, do: extract_symbols(body, []), else: []

  [%{
    name:             name_token.value,
    kind:             :function,
    range:            ast_to_range(node),
    selection_range:  token_to_range(name_token),
    children:         children
  }]
end

defp extract_symbols(%ASTNode{rule_name: "let_statement"} = node, _acc) do
  name_token = ASTNode.find_nodes(node, "variable") |> List.first() |> ASTNode.token()
  [%{
    name:             name_token.value,
    kind:             :variable,
    range:            ast_to_range(node),
    selection_range:  token_to_range(name_token),
    children:         []
  }]
end

defp extract_symbols(%ASTNode{children: children}, _acc) do
  Enum.flat_map(children, fn child ->
    case child do
      %ASTNode{} -> extract_symbols(child, [])
      _token     -> []
    end
  end)
end

# Convert ASTNode source span to LSP range (0-based)
defp ast_to_range(%ASTNode{start_line: sl, start_column: sc, end_line: el, end_column: ec}) do
  %{start: %{line: sl - 1, character: sc - 1},
    end:   %{line: el - 1, character: ec - 1}}
end
```

## Extracting Folding Ranges from the AST

Code folding lets the user collapse blocks. Any AST node that spans multiple lines is a candidate:

```elixir
@impl LanguageServer.LanguageBridge
def folding_ranges(ast) do
  ranges = collect_folding_ranges(ast, [])
  {:ok, ranges}
end

defp collect_folding_ranges(%ASTNode{} = node, acc) do
  # Only fold nodes that span multiple lines
  new_acc = if spans_multiple_lines?(node) do
    kind = folding_kind(node.rule_name)
    [%{start_line: node.start_line - 1,
       end_line:   node.end_line - 1,
       kind:       kind} | acc]
  else
    acc
  end

  Enum.reduce(node.children, new_acc, fn child, a ->
    case child do
      %ASTNode{} -> collect_folding_ranges(child, a)
      _          -> a
    end
  end)
end

defp spans_multiple_lines?(%ASTNode{start_line: sl, end_line: el}), do: el > sl
defp spans_multiple_lines?(_), do: false

defp folding_kind("function_def"),  do: "region"
defp folding_kind("for_loop"),      do: "region"
defp folding_kind("if_statement"),  do: "region"
defp folding_kind("comment_block"), do: "comment"
defp folding_kind(_),               do: "region"
```

## The Symbol Table Layer

For hover, go-to-definition, references, and completion, the bridge needs a symbol table. This is a separate pass over the AST that the bridge runs after parsing.

```elixir
defmodule MyLanguage.SymbolTable do
  defstruct definitions: %{},   # name → %{location, type, kind}
            references:  %{},   # name → [location, ...]
            scopes:      []     # stack of scope frames during analysis

  def build(ast) do
    analyzer = %__MODULE__{}
    analyze(ast, analyzer)
  end
end
```

The analysis walk visits each AST node:
- **Declaration nodes** (`LET x = ...`, `FUNCTION foo(...)`) → add to `definitions`
- **Reference nodes** (`PRINT x`, `y = x + 1`) → add to `references`
- **Scope-introducing nodes** (`FUNCTION`, `FOR`, `IF`) → push/pop a scope frame

```elixir
defp analyze(%ASTNode{rule_name: "let_statement"} = node, table) do
  name_tok = get_name_token(node)
  type     = infer_type(node)          # optional: type inference
  table    = declare(table, name_tok.value, %{
    location: token_to_location(name_tok),
    type:     type,
    kind:     :variable
  })
  analyze_children(node.children, table)
end

defp analyze(%ASTNode{rule_name: "name_reference"} = node, table) do
  name_tok = get_name_token(node)
  record_reference(table, name_tok.value, token_to_location(name_tok))
end
```

## Hover Using the Symbol Table

```elixir
@impl LanguageServer.LanguageBridge
def hover(ast, position) do
  symbol_table = MyLanguage.SymbolTable.build(ast)
  name         = find_name_at(ast, position)   # walk AST to find identifier at cursor

  case Map.get(symbol_table.definitions, name) do
    nil ->
      {:none}
    %{type: type, kind: kind} ->
      markdown = "**#{kind}** `#{name}` : `#{type}`"
      range    = find_node_range_at(ast, position)
      {:ok, %{contents: markdown, range: range}}
  end
end

defp find_name_at(ast, position) do
  # Walk the AST to find the token at the cursor position
  ASTNode.walk_ast(ast, fn node ->
    case node do
      %ASTNode{rule_name: r} when r in ["name_reference", "let_statement"] ->
        tok = get_name_token(node)
        if token_contains_position?(tok, position), do: {:halt, tok.value}, else: {:cont}
      _ -> {:cont}
    end
  end)
end
```

## Go to Definition

```elixir
@impl LanguageServer.LanguageBridge
def definition(ast, position, uri) do
  symbol_table = MyLanguage.SymbolTable.build(ast)
  name         = find_name_at(ast, position)

  case Map.get(symbol_table.definitions, name) do
    nil      -> {:none}
    %{location: loc} -> {:ok, %{uri: uri, range: loc.range}}
  end
end
```

For a multi-file project, the symbol table must be built across all files in scope. Single-file languages like classic BASIC can ignore this and always resolve within the current file.

## Find References

```elixir
@impl LanguageServer.LanguageBridge
def references(ast, position, uri, include_declaration) do
  symbol_table = MyLanguage.SymbolTable.build(ast)
  name         = find_name_at(ast, position)

  refs = Map.get(symbol_table.references, name, [])
  locs = if include_declaration do
    defn = Map.get(symbol_table.definitions, name)
    if defn, do: [%{uri: uri, range: defn.location.range} | refs], else: refs
  else
    refs
  end

  {:ok, Enum.map(locs, fn r -> %{uri: uri, range: r.range} end)}
end
```

## Completion

Completion suggests identifiers when the user is typing. The bridge walks the symbol table to enumerate all names in scope at the cursor position:

```elixir
@impl LanguageServer.LanguageBridge
def completion(ast, position) do
  symbol_table = MyLanguage.SymbolTable.build(ast)
  current_scope = find_scope_at(symbol_table, position)

  items = symbol_table.definitions
  |> Enum.filter(fn {_name, defn} -> visible_in_scope?(defn, current_scope) end)
  |> Enum.map(fn {name, %{kind: kind, type: type}} ->
    %{
      label:         name,
      kind:          kind_to_completion_kind(kind),
      detail:        type,
      documentation: nil,
      insert_text:   name
    }
  end)

  {:ok, items}
end

defp kind_to_completion_kind(:function), do: :function
defp kind_to_completion_kind(:variable), do: :variable
defp kind_to_completion_kind(:constant), do: :constant
defp kind_to_completion_kind(_),         do: :text
```

## Worked Example: BASIC Language Bridge

A complete, minimal BASIC bridge using only the lexer and parser (no symbol table) for the first iteration:

```elixir
defmodule Basic.LspBridge do
  @behaviour LanguageServer.LanguageBridge

  alias CodingAdventures.BasicLexer
  alias CodingAdventures.BasicParser
  alias CodingAdventures.Parser.ASTNode

  # ── Required callbacks ──────────────────────────────────────────────────

  @impl true
  def tokenize(source) do
    BasicLexer.tokenize(source)
  end

  @impl true
  def parse(source) do
    with {:ok, tokens} <- BasicLexer.tokenize(source),
         {:ok, ast}    <- BasicParser.parse_tokens(tokens) do
      {:ok, ast, []}
    else
      {:error, %{message: msg, line: l, col: c}} ->
        {:ok, ASTNode.empty(), [error_diagnostic(msg, l, c)]}
      {:error, msg} ->
        {:ok, ASTNode.empty(), [error_diagnostic(msg, 1, 1)]}
    end
  end

  # ── Optional callbacks ──────────────────────────────────────────────────

  @impl true
  def semantic_tokens(_source, tokens) do
    sem = tokens
    |> Enum.reject(&(&1.type == "EOF"))
    |> Enum.flat_map(&to_semantic_token/1)
    {:ok, sem}
  end

  @impl true
  def document_symbols(ast) do
    {:ok, extract_symbols(ast)}
  end

  @impl true
  def folding_ranges(ast) do
    {:ok, collect_folds(ast, [])}
  end

  # No symbol table yet — these return :not_supported so the framework
  # omits them from the capabilities advertisement
  @impl true; def hover(_ast, _pos),                    do: :not_supported
  @impl true; def definition(_ast, _pos, _uri),         do: :not_supported
  @impl true; def references(_ast, _pos, _uri, _decl),  do: :not_supported
  @impl true; def completion(_ast, _pos),               do: :not_supported
  @impl true; def rename(_ast, _pos, _name),            do: :not_supported
  @impl true; def signature_help(_ast, _pos),           do: :not_supported
  @impl true; def format(_source),                      do: :not_supported

  # ── Private helpers ─────────────────────────────────────────────────────

  @keyword_types ~w[LET PRINT IF THEN ELSE GOTO GOSUB RETURN FOR TO STEP NEXT END DIM]

  defp to_semantic_token(%{type: t, line: l, column: c, value: v}) when t in @keyword_types do
    [%{line: l - 1, character: c - 1, length: String.length(v),
       token_type: "keyword", modifiers: []}]
  end
  defp to_semantic_token(%{type: "STRING_LIT", line: l, column: c, value: v}) do
    [%{line: l - 1, character: c - 1, length: String.length(v),
       token_type: "string", modifiers: []}]
  end
  defp to_semantic_token(%{type: "NUMBER", line: l, column: c, value: v}) do
    [%{line: l - 1, character: c - 1, length: String.length(v),
       token_type: "number", modifiers: []}]
  end
  defp to_semantic_token(_), do: []

  defp extract_symbols(%ASTNode{rule_name: "sub_def"} = n) do
    name = get_name_token(n)
    [%{name: name.value, kind: :function,
       range: ast_range(n), selection_range: token_range(name), children: []}]
  end
  defp extract_symbols(%ASTNode{children: kids}) do
    Enum.flat_map(kids, fn
      %ASTNode{} = child -> extract_symbols(child)
      _                  -> []
    end)
  end
  defp extract_symbols(_), do: []

  defp collect_folds(%ASTNode{start_line: sl, end_line: el} = n, acc) when el > sl do
    acc = [%{start_line: sl - 1, end_line: el - 1, kind: "region"} | acc]
    Enum.reduce(n.children, acc, fn
      %ASTNode{} = child, a -> collect_folds(child, a)
      _, a                  -> a
    end)
  end
  defp collect_folds(%ASTNode{children: kids}, acc) do
    Enum.reduce(kids, acc, fn
      %ASTNode{} = child, a -> collect_folds(child, a)
      _, a                  -> a
    end)
  end
  defp collect_folds(_, acc), do: acc

  defp error_diagnostic(msg, line, col) do
    %{range:    %{start: %{line: line - 1, character: col - 1},
                  end:   %{line: line - 1, character: col}},
      severity: :error,
      message:  msg}
  end

  defp ast_range(%ASTNode{start_line: sl, start_column: sc, end_line: el, end_column: ec}) do
    %{start: %{line: sl - 1, character: sc - 1}, end: %{line: el - 1, character: ec - 1}}
  end
  defp token_range(%{line: l, column: c, value: v}) do
    %{start: %{line: l - 1, character: c - 1},
      end:   %{line: l - 1, character: c - 1 + String.length(v)}}
  end
  defp get_name_token(node) do
    ASTNode.find_nodes(node, "name") |> List.first() |> ASTNode.token()
  end
end
```

## Evolution Path

A language bridge starts minimal and grows. The recommended sequence:

```
Phase 1 — Syntax only (just lexer + parser)
  ✓ Diagnostics (parse errors → squiggles)
  ✓ Semantic tokens (accurate highlighting)
  ✓ Document symbols (outline panel)
  ✓ Code folding

Phase 2 — Add symbol table (semantic analysis pass)
  ✓ Hover (type info on mouse-over)
  ✓ Go to Definition
  ✓ Find References
  ✓ Rename

Phase 3 — Add scope-aware completions
  ✓ Autocomplete (names in scope at cursor)
  ✓ Signature help (function parameter hints)

Phase 4 — Add formatter
  ✓ Format on save
```

Each phase is independently shippable. A language does not need a symbol table to get Phase 1 working. And the framework advertises exactly the features that are implemented — no stubs, no unimplemented features pretending to work.
