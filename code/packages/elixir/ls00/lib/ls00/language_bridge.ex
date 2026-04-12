defmodule Ls00.LanguageBridge do
  @moduledoc """
  The behaviour that every language bridge must implement.

  ## Design Philosophy: Narrow Interfaces

  Elixir's `@behaviour` + `@optional_callbacks` mechanism serves the same purpose
  as Go's narrow interfaces. The LanguageBridge behaviour requires only two
  callbacks: `tokenize/1` and `parse/1`. All other features (hover,
  go-to-definition, etc.) are optional and declared as `@optional_callbacks`.

  At runtime, the server checks whether the bridge module exports the optional
  callbacks:

      if function_exported?(bridge_module, :hover, 2) do
        bridge_module.hover(ast, pos)
      end

  This matches the LSP spec's philosophy: capabilities are advertised, not
  assumed. An editor won't even try to ask for hover if the server didn't
  advertise it. The result is that a phase-1 bridge (lexer+parser only) works
  perfectly without stubs.

  ## Required Callbacks

  - `tokenize/1` -- lex the source string and return a token stream
  - `parse/1` -- parse the source string and return an AST + diagnostics

  ## Optional Callbacks

  Each optional callback corresponds to one LSP feature. A bridge implements
  only the features its language supports.

  | Callback            | LSP Feature                        |
  |---------------------|------------------------------------|
  | `hover/2`           | Hover tooltips                     |
  | `definition/3`      | Go to Definition (F12)             |
  | `references/4`      | Find All References                |
  | `completion/2`      | Autocomplete                       |
  | `rename/3`          | Symbol rename (F2)                 |
  | `semantic_tokens/2` | Semantic syntax highlighting       |
  | `document_symbols/1`| Document outline panel             |
  | `folding_ranges/1`  | Code folding                       |
  | `signature_help/2`  | Function signature hints           |
  | `format/1`          | Document formatting                |
  """

  alias Ls00.Types

  # ---------------------------------------------------------------------------
  # Required callbacks
  # ---------------------------------------------------------------------------

  @doc """
  Lex the source string and return the token stream.

  Each Token carries a type string (e.g. "KEYWORD", "IDENTIFIER"), its value,
  and its 1-based line and column position. The bridge is responsible for
  converting 1-based positions to 0-based before building SemanticToken values.
  """
  @callback tokenize(source :: String.t()) ::
              {:ok, [Types.Token.t()]} | {:error, String.t()}

  @doc """
  Parse the source string and return:
    - `ast` -- the parsed abstract syntax tree (may be partial on error)
    - `diagnostics` -- parse errors and warnings as Diagnostic structs
    - or an `{:error, reason}` tuple for fatal failures

  Even when there are syntax errors, `parse/1` should return a partial AST.
  This allows hover, folding, and symbol features to continue working on
  the valid portions of the file.
  """
  @callback parse(source :: String.t()) ::
              {:ok, any(), [Types.Diagnostic.t()]} | {:error, String.t()}

  # ---------------------------------------------------------------------------
  # Optional callbacks
  # ---------------------------------------------------------------------------

  @doc """
  Return hover information for the AST node at the given position.

  Returns `{:ok, hover_result}` with content to display, `{:ok, nil}` if there
  is no hover info at this position, or `{:error, reason}` if something went wrong.
  """
  @callback hover(ast :: any(), pos :: Types.Position.t()) ::
              {:ok, Types.HoverResult.t() | nil} | {:error, String.t()}

  @doc """
  Return the location where the symbol at `pos` was declared.

  Returns `{:ok, location}`, `{:ok, nil}` if the symbol is not found, or
  `{:error, reason}`.
  """
  @callback definition(ast :: any(), pos :: Types.Position.t(), uri :: String.t()) ::
              {:ok, Types.Location.t() | nil} | {:error, String.t()}

  @doc """
  Return all uses of the symbol at `pos`.

  `include_decl`: if true, include the declaration location in the results.
  """
  @callback references(
              ast :: any(),
              pos :: Types.Position.t(),
              uri :: String.t(),
              include_decl :: boolean()
            ) ::
              {:ok, [Types.Location.t()]} | {:error, String.t()}

  @doc """
  Return autocomplete suggestions valid at `pos`.
  """
  @callback completion(ast :: any(), pos :: Types.Position.t()) ::
              {:ok, [Types.CompletionItem.t()]} | {:error, String.t()}

  @doc """
  Return the set of text edits needed to rename the symbol at `pos` to `new_name`.
  """
  @callback rename(ast :: any(), pos :: Types.Position.t(), new_name :: String.t()) ::
              {:ok, Types.WorkspaceEdit.t() | nil} | {:error, String.t()}

  @doc """
  Return semantic token data for the whole document.

  `tokens` is the output of `tokenize/1` -- the bridge should use these rather
  than re-lexing. The returned SemanticTokens must be sorted by line, then
  by character (ascending), because the LSP encoding is delta-based.
  """
  @callback semantic_tokens(source :: String.t(), tokens :: [Types.Token.t()]) ::
              {:ok, [Types.SemanticToken.t()]} | {:error, String.t()}

  @doc """
  Return the outline tree for the given AST.
  """
  @callback document_symbols(ast :: any()) ::
              {:ok, [Types.DocumentSymbol.t()]} | {:error, String.t()}

  @doc """
  Return collapsible regions derived from the AST structure.
  """
  @callback folding_ranges(ast :: any()) ::
              {:ok, [Types.FoldingRange.t()]} | {:error, String.t()}

  @doc """
  Return signature hint information for the call at `pos`.

  Returns `{:ok, result}`, `{:ok, nil}` if not inside a call expression, or
  `{:error, reason}`.
  """
  @callback signature_help(ast :: any(), pos :: Types.Position.t()) ::
              {:ok, Types.SignatureHelpResult.t() | nil} | {:error, String.t()}

  @doc """
  Return the text edits needed to format the document.

  Typically this is a single edit replacing the entire file with the formatted
  content.
  """
  @callback format(source :: String.t()) ::
              {:ok, [Types.TextEdit.t()]} | {:error, String.t()}

  @optional_callbacks [
    hover: 2,
    definition: 3,
    references: 4,
    completion: 2,
    rename: 3,
    semantic_tokens: 2,
    document_symbols: 1,
    folding_ranges: 1,
    signature_help: 2,
    format: 1
  ]
end
