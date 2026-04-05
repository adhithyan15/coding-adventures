defmodule CodingAdventures.EcmascriptEs5Lexer do
  @moduledoc """
  Tokenizes ECMAScript 5 (2009) source code using the grammar-driven lexer.

  This module is a thin wrapper around `CodingAdventures.Lexer.GrammarLexer`.
  It loads the `es5.tokens` grammar file from `code/grammars/ecmascript/` and
  provides convenience functions for tokenizing ES5 JavaScript source code.

  ## What is ECMAScript 5?

  ES5 (ECMA-262 5th Edition, December 2009) landed a full decade after ES3.
  ES4 was abandoned after years of debate. The syntactic changes in ES5 are
  modest -- the real innovations were strict mode semantics, native JSON
  support, and property descriptors.

  ### What ES5 adds over ES3:

  - `debugger` keyword (moved from future-reserved to keyword)
  - Getter/setter syntax in object literals: `{ get x() {}, set x(v) {} }`
  - String line continuation (backslash before newline)
  - Trailing commas in object literals

  ### What ES5 does NOT have:

  - No `let`/`const` (ES2015)
  - No class syntax (ES2015)
  - No arrow functions (ES2015)
  - No template literals (ES2015)
  - No modules (ES2015)
  - No destructuring (ES2015)

  ## Usage

      {:ok, tokens} = CodingAdventures.EcmascriptEs5Lexer.tokenize("debugger;")

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  alias CodingAdventures.GrammarTools
  alias CodingAdventures.Lexer

  @grammar_dir Path.expand("../../../../../grammars/ecmascript", __DIR__)

  @doc """
  Return the path to the ES5 tokens grammar file.
  """
  @spec grammar_path() :: String.t()
  def grammar_path do
    Path.join(@grammar_dir, "es5.tokens")
  end

  @doc """
  Load and parse the ES5 token grammar.

  Returns `{:ok, token_grammar}` on success, `{:error, message}` on failure.
  """
  @spec load_grammar() :: {:ok, CodingAdventures.GrammarTools.TokenGrammar.t()} | {:error, String.t()}
  def load_grammar do
    path = grammar_path()

    case File.read(path) do
      {:ok, source} -> GrammarTools.parse_token_grammar(source)
      {:error, reason} -> {:error, "Failed to read #{path}: #{inspect(reason)}"}
    end
  end

  @doc """
  Tokenize ECMAScript 5 source code.

  Takes a string of ES5 JavaScript source code and returns a list of
  `Token` structs. The list always ends with an EOF token.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.EcmascriptEs5Lexer.tokenize("debugger;")
      iex> Enum.any?(tokens, fn t -> t.type == "KEYWORD" and t.value == "debugger" end)
      true
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    case load_grammar() do
      {:ok, grammar} -> Lexer.tokenize(source, grammar)
      {:error, _} = err -> err
    end
  end
end
