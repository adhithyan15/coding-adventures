defmodule CodingAdventures.EcmascriptEs1Lexer do
  @moduledoc """
  Tokenizes ECMAScript 1 (1997) source code using the grammar-driven lexer.

  This module is a thin wrapper around `CodingAdventures.Lexer.GrammarLexer`.
  It loads the `es1.tokens` grammar file from `code/grammars/ecmascript/` and
  provides convenience functions for tokenizing ES1 JavaScript source code.

  ## What is ECMAScript 1?

  ES1 (ECMA-262 1st Edition, June 1997) is the very first standardized version
  of JavaScript. Brendan Eich created the language for Netscape Navigator in
  1995; two years later, ECMA International published this specification.

  ### What ES1 has:

  - 23 keywords: `break`, `case`, `continue`, `default`, `delete`, `do`,
    `else`, `for`, `function`, `if`, `in`, `new`, `return`, `switch`, `this`,
    `typeof`, `var`, `void`, `while`, `with`, `true`, `false`, `null`
  - Basic operators: arithmetic, bitwise, logical, comparison, assignment
  - String literals (single and double quoted)
  - Numeric literals (decimal integers, floats, hex with 0x prefix)
  - The `$` character is valid in identifiers

  ### What ES1 does NOT have:

  - No `===` or `!==` (strict equality -- that is ES3)
  - No `try`/`catch`/`finally`/`throw` (error handling -- that is ES3)
  - No regex literals (formalized in ES3)
  - No template literals (ES2015)
  - No arrow functions (ES2015)
  - No `let`/`const` (ES2015)

  ## Usage

      {:ok, tokens} = CodingAdventures.EcmascriptEs1Lexer.tokenize("var x = 1 + 2;")

  ## Pipeline

  Source code --> es1.tokens grammar --> GrammarLexer --> Token list

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  alias CodingAdventures.GrammarTools
  alias CodingAdventures.Lexer

  # ---------------------------------------------------------------------------
  # Grammar File Location
  # ---------------------------------------------------------------------------
  #
  # The es1.tokens file lives in code/grammars/ecmascript/ at the repository
  # root. We locate it by walking up from this module's compiled location.
  #
  # Directory structure:
  #   ecmascript_es1_lexer/lib/coding_adventures/ecmascript_es1_lexer.ex (here)
  #   -> lib/                   (parent x1)
  #   -> ecmascript_es1_lexer/  (parent x2)
  #   -> elixir/                (parent x3)
  #   -> packages/              (parent x4)
  #   -> code/                  (parent x5)
  #   -> code/grammars/ecmascript/es1.tokens

  @grammar_dir Path.expand("../../../../../grammars/ecmascript", __DIR__)

  @doc """
  Return the path to the ES1 tokens grammar file.

  This is useful for debugging and for tools that need to inspect the grammar.
  """
  @spec grammar_path() :: String.t()
  def grammar_path do
    Path.join(@grammar_dir, "es1.tokens")
  end

  @doc """
  Load and parse the ES1 token grammar.

  Returns `{:ok, token_grammar}` on success, `{:error, message}` on failure.
  The returned `TokenGrammar` struct can be passed to `CodingAdventures.Lexer.tokenize/2`
  directly if you need more control over the tokenization process.
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
  Tokenize ECMAScript 1 source code.

  Takes a string of ES1 JavaScript source code and returns a list of
  `Token` structs. The list always ends with an EOF token.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.

  ## Examples

      iex> {:ok, tokens} = CodingAdventures.EcmascriptEs1Lexer.tokenize("var x = 42;")
      iex> length(tokens) > 0
      true

      iex> {:ok, tokens} = CodingAdventures.EcmascriptEs1Lexer.tokenize("1 + 2")
      iex> hd(tokens).type
      "NUMBER"
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    case load_grammar() do
      {:ok, grammar} -> Lexer.tokenize(source, grammar)
      {:error, _} = err -> err
    end
  end
end
