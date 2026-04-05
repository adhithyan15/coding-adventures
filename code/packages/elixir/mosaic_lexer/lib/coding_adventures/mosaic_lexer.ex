defmodule CodingAdventures.MosaicLexer do
  @moduledoc """
  Mosaic Lexer — Thin wrapper around the grammar-driven lexer engine.

  Mosaic is a Component Description Language (CDL) for declaring UI component
  structure with named typed slots. A `.mosaic` file compiles to native code
  per target platform (Web Components, React, SwiftUI, Compose, Rust/paint-vm).

  This module reads `mosaic.tokens` from the shared grammars directory and
  delegates to `GrammarLexer.tokenize/2` to tokenize Mosaic source code.

  ## Usage

      {:ok, tokens} = CodingAdventures.MosaicLexer.tokenize(~S(component Foo { Box { } }))

  ## Token Types

  The mosaic grammar emits these token types:

  | Type        | Examples                              |
  |-------------|---------------------------------------|
  | `KEYWORD`   | `component`, `slot`, `when`, `each`,  |
  |             | `import`, `from`, `as`, `text`,       |
  |             | `number`, `bool`, `image`, `color`,   |
  |             | `node`, `list`, `true`, `false`       |
  | `NAME`      | `Foo`, `padding-left`, `corner_radius`|
  | `DIMENSION` | `16dp`, `50%`, `1.5rem`               |
  | `NUMBER`    | `42`, `-3.14`, `0.5`                  |
  | `COLOR_HEX` | `#fff`, `#2563eb`, `#rrggbbaa`        |
  | `STRING`    | `"hello"`, `"path/to/image"`          |
  | `LBRACE`    | `{`                                   |
  | `RBRACE`    | `}`                                   |
  | `LANGLE`    | `<`                                   |
  | `RANGLE`    | `>`                                   |
  | `COLON`     | `:`                                   |
  | `SEMICOLON` | `;`                                   |
  | `COMMA`     | `,`                                   |
  | `DOT`       | `.`                                   |
  | `EQUALS`    | `=`                                   |
  | `AT`        | `@`                                   |
  | `EOF`       | (end of input)                        |

  ## How Keywords Work

  The `keywords:` section in `mosaic.tokens` causes the lexer to promote any
  `NAME` token whose text exactly matches a keyword into a `KEYWORD` token.
  So `component Foo` produces `[{KEYWORD, "component"}, {NAME, "Foo"}]` —
  there is no separate `COMPONENT` token type.

  ## Whitespace and Comments

  The grammar skips `WHITESPACE`, `LINE_COMMENT` (`// …`), and
  `BLOCK_COMMENT` (`/* … */`) automatically. They never appear in the output.

  ## How It Works

  1. On first call, `create_lexer/0` parses `mosaic.tokens` into a
     `TokenGrammar` struct. This is cached in a persistent term for
     fast repeated access.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/2`,
     which does all the real work.

  The entire module is about 25 lines of actual logic — the grammar file
  does the heavy lifting.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # The shared grammars directory lives five levels above __DIR__:
  #   __DIR__ = .../code/packages/elixir/mosaic_lexer/lib/coding_adventures/
  #   ..       = .../code/packages/elixir/mosaic_lexer/lib/
  #   ../..    = .../code/packages/elixir/mosaic_lexer/
  #   ../../.. = .../code/packages/elixir/
  #   ../../../.. = .../code/packages/
  #   ../../../../.. = .../code/
  #   + /grammars = .../code/grammars/
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Tokenize Mosaic source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.

  Whitespace and comments are automatically skipped; they do not appear
  in the returned list.

  The last token is always `%Token{type: "EOF"}`.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.MosaicLexer.tokenize("component Foo { Box { } }")
      iex> hd(tokens).type
      "KEYWORD"
      iex> hd(tokens).value
      "component"
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar)
  end

  @doc """
  Parse the `mosaic.tokens` grammar file and return the `TokenGrammar`.

  This is useful if you want to inspect the grammar — for example, to check
  which token names and keywords are defined — or to reuse it directly with
  `GrammarLexer.tokenize/2`.

  The result is NOT cached; call `tokenize/1` for cached operation.

  ## Example

      grammar = CodingAdventures.MosaicLexer.create_lexer()
      Enum.map(grammar.definitions, & &1.name)
      # => ["STRING", "DIMENSION", "NUMBER", "COLOR_HEX", "NAME",
      #     "LBRACE", "RBRACE", "LANGLE", "RANGLE", "COLON",
      #     "SEMICOLON", "COMMA", "DOT", "EQUALS", "AT"]

      grammar.keywords
      # => ["component", "slot", "import", "from", "as", "text", "number",
      #     "bool", "image", "color", "node", "list", "true", "false",
      #     "when", "each"]
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "mosaic.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # ---------------------------------------------------------------------------
  # Private: cache the grammar in a persistent_term for fast repeated access.
  #
  # :persistent_term is a read-mostly global store — reads are extremely cheap
  # (O(1) with no copying) while writes invalidate all processes' instruction
  # cache. Since the grammar never changes at runtime, we write once and read
  # many times.
  # ---------------------------------------------------------------------------
  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
