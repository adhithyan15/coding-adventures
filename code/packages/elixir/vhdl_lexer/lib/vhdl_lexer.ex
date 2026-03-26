defmodule CodingAdventures.VhdlLexer do
  @moduledoc """
  VHDL Lexer — Tokenizes VHDL source code with case normalization.

  This module reads `vhdl.tokens` from the shared grammars directory and
  uses `GrammarLexer.tokenize/2` to tokenize VHDL source code. After
  tokenization, NAME and KEYWORD token values are lowercased to implement
  VHDL's case-insensitive semantics.

  ## What is VHDL?

  VHDL (VHSIC Hardware Description Language) was designed by the US
  Department of Defense for documenting and simulating digital systems.
  Where Verilog is terse and C-like, VHDL is verbose and Ada-like, with
  strong typing, explicit declarations, and case-insensitive identifiers.

  ## Key Differences from Verilog

  VHDL has no preprocessor. There are no `define, `ifdef, or `include
  directives. Configuration and conditional logic are handled through
  VHDL's own language constructs (generics, generate statements,
  configurations).

  VHDL is case-insensitive: `ENTITY`, `Entity`, and `entity` all mean the
  same thing. This lexer normalizes all NAME and KEYWORD tokens to lowercase
  after tokenization, so downstream tools see a consistent representation.

  ## How Case Normalization Works

  The `.tokens` grammar file lists all keywords in lowercase. After the
  grammar lexer produces tokens, we post-process the list:

  1. Any token with type "NAME" has its value lowercased.
  2. After lowercasing, if the value matches a keyword, the token type
     is changed to "KEYWORD".

  This two-step process means `ENTITY`, `Entity`, and `entity` all become
  `%Token{type: "KEYWORD", value: "entity"}`.

  ## Usage

      # Basic tokenization
      {:ok, tokens} = CodingAdventures.VhdlLexer.tokenize("ENTITY counter IS END counter;")

      # All identifiers and keywords are lowercased
      Enum.map(tokens, & &1.value)
      # => ["entity", "counter", "is", "end", "counter", ";", ""]

  ## How It Works

  1. The source is passed to `GrammarLexer.tokenize/2` with the
     `vhdl.tokens` grammar.

  2. The resulting tokens are post-processed to normalize case:
     NAME and KEYWORD values are downcased, and NAME tokens whose
     downcased value matches a keyword are promoted to KEYWORD.

  3. The grammar is cached in a `persistent_term` for fast repeated access.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # ---------------------------------------------------------------------------
  # Grammar File Location
  # ---------------------------------------------------------------------------
  #
  # The vhdl.tokens file lives in code/grammars/ at the repository root.
  # From this file's location (lib/vhdl_lexer.ex), we navigate:
  #   lib/ -> vhdl_lexer/ -> elixir/ -> packages/ -> code/ -> grammars/
  #
  # Using Path.join and Path.expand ensures this works regardless of
  # the current working directory when the code is compiled.

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Tokenize VHDL source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.

  All NAME and KEYWORD token values are normalized to lowercase, because
  VHDL is a case-insensitive language. Extended identifiers (\\name\\)
  preserve their original case, as required by the VHDL standard.

  ## Examples

      # Simple entity declaration
      {:ok, tokens} = CodingAdventures.VhdlLexer.tokenize("entity counter is end counter;")

      # Case insensitivity — these produce identical token streams
      {:ok, t1} = CodingAdventures.VhdlLexer.tokenize("ENTITY Foo IS END Foo;")
      {:ok, t2} = CodingAdventures.VhdlLexer.tokenize("entity foo is end foo;")
      # t1 and t2 have the same token values (all lowercase)
  """
  @spec tokenize(String.t()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    # ---------------------------------------------------------------------------
    # Step 1: Grammar-Driven Tokenization
    # ---------------------------------------------------------------------------
    #
    # The vhdl.tokens grammar handles all the lexical rules: keywords,
    # operators, number literals (based, real, plain), identifiers,
    # character literals ('0', '1', 'X'), bit string literals (X"FF"),
    # and extended identifiers (\special name\).

    grammar = get_grammar()

    case GrammarLexer.tokenize(source, grammar) do
      {:ok, tokens} ->
        # -----------------------------------------------------------------------
        # Step 2: Case Normalization (Post-Tokenization)
        # -----------------------------------------------------------------------
        #
        # VHDL is case-insensitive for identifiers and keywords. The grammar
        # file lists keywords in lowercase. After tokenization, we:
        #
        # 1. Lowercase all NAME token values
        # 2. Re-check if the lowercased value is a keyword
        # 3. Lowercase all KEYWORD token values (in case the grammar engine
        #    matched a mixed-case keyword)
        #
        # Extended identifiers (EXTENDED_IDENT) are NOT normalized — the VHDL
        # standard says \MyName\ and \myname\ are different identifiers.
        #
        # Truth table for case normalization:
        #
        #   Input Token          | After Normalization
        #   ---------------------|--------------------
        #   NAME "Counter"       | NAME "counter"
        #   NAME "ENTITY"        | KEYWORD "entity"
        #   KEYWORD "Entity"     | KEYWORD "entity"
        #   EXTENDED_IDENT "\X\" | EXTENDED_IDENT "\X\"  (unchanged)
        #   STRING "Hello"       | STRING "Hello"        (unchanged)
        #   NUMBER "42"          | NUMBER "42"           (unchanged)

        keyword_set = MapSet.new(grammar.keywords)
        normalized = Enum.map(tokens, fn token -> normalize_case(token, keyword_set) end)
        {:ok, normalized}

      {:error, _msg} = err ->
        err
    end
  end

  @doc """
  Parse the vhdl.tokens grammar file and return the TokenGrammar.

  This is useful if you want to inspect the grammar or reuse it directly.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "vhdl.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # ---------------------------------------------------------------------------
  # Case Normalization
  # ---------------------------------------------------------------------------
  #
  # VHDL's case insensitivity is one of its defining characteristics. It
  # comes from VHDL's Ada heritage — Ada is also case-insensitive.
  #
  # The normalization function handles three cases:
  #
  # 1. NAME tokens: lowercase the value, then check if it's a keyword.
  #    If so, promote to KEYWORD type. This handles input like "ENTITY"
  #    which the regex matches as NAME but should be KEYWORD "entity".
  #
  # 2. KEYWORD tokens: lowercase the value. This handles cases where the
  #    grammar engine already identified a keyword but the value might
  #    have mixed case (shouldn't happen with our grammar, but defensive).
  #
  # 3. All other tokens: leave unchanged. Strings, numbers, operators,
  #    and extended identifiers keep their original values.

  defp normalize_case(%{type: "NAME"} = token, keyword_set) do
    downcased = String.downcase(token.value)

    if MapSet.member?(keyword_set, downcased) do
      %{token | type: "KEYWORD", value: downcased}
    else
      %{token | value: downcased}
    end
  end

  defp normalize_case(%{type: "KEYWORD"} = token, _keyword_set) do
    %{token | value: String.downcase(token.value)}
  end

  defp normalize_case(token, _keyword_set), do: token

  # ---------------------------------------------------------------------------
  # Grammar Caching
  # ---------------------------------------------------------------------------
  #
  # Parsing the .tokens file involves reading from disk and building regex
  # patterns. We cache the result in a persistent_term so that the first
  # call pays the cost, but subsequent calls are essentially free (a single
  # ETS lookup).
  #
  # persistent_term is ideal for data that is written once and read many
  # times — exactly the pattern for grammar definitions.

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
