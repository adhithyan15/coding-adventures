defmodule CodingAdventures.EcmascriptEs3Lexer do
  @moduledoc """
  Tokenizes ECMAScript 3 (1999) source code using the grammar-driven lexer.

  This module is a thin wrapper around `CodingAdventures.Lexer.GrammarLexer`.
  It loads the `es3.tokens` grammar file from `code/grammars/ecmascript/` and
  provides convenience functions for tokenizing ES3 JavaScript source code.

  ## What is ECMAScript 3?

  ES3 (ECMA-262 3rd Edition, December 1999) was the version that made JavaScript
  a real, complete language. It landed two years after ES1 and added features
  that developers today consider fundamental.

  ### What ES3 adds over ES1:

  - `===` and `!==` (strict equality -- no type coercion)
  - `try`/`catch`/`finally`/`throw` (structured error handling)
  - Regular expression literals (`/pattern/flags`)
  - `instanceof` operator
  - Expanded future-reserved words

  ### What ES3 does NOT have:

  - No getters/setters in object literals (ES5)
  - No strict mode (ES5)
  - No `let`/`const`/`class`/arrow functions (ES2015)
  - No template literals (ES2015)

  ## Regex vs Division Ambiguity

  The `/` character is ambiguous in JavaScript: it could start a regex literal
  or be the division operator. Context-sensitive lexing would be needed for a
  production lexer, but this grammar-driven approach handles the common cases.

  ## Usage

      {:ok, tokens} = CodingAdventures.EcmascriptEs3Lexer.tokenize("try { x === 1 } catch(e) {}")

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  alias CodingAdventures.GrammarTools
  alias CodingAdventures.Lexer

  @grammar_dir Path.expand("../../../../../grammars/ecmascript", __DIR__)

  @doc """
  Return the path to the ES3 tokens grammar file.
  """
  @spec grammar_path() :: String.t()
  def grammar_path do
    Path.join(@grammar_dir, "es3.tokens")
  end

  @doc """
  Load and parse the ES3 token grammar.

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
  Tokenize ECMAScript 3 source code.

  Takes a string of ES3 JavaScript source code and returns a list of
  `Token` structs. The list always ends with an EOF token.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.EcmascriptEs3Lexer.tokenize("x === 1")
      iex> Enum.any?(tokens, &(&1.type == "STRICT_EQUALS"))
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
