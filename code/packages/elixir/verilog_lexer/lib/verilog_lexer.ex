defmodule CodingAdventures.VerilogLexer do
  @moduledoc """
  Verilog Lexer — Tokenizes Verilog HDL source code with optional preprocessing.

  This module reads `verilog.tokens` from the shared grammars directory and
  uses `GrammarLexer.tokenize/2` to tokenize Verilog source code. It also
  provides a preprocessor that handles compiler directives before tokenization.

  ## What is Verilog?

  Verilog is a Hardware Description Language (HDL) used to describe digital
  circuits. Unlike software languages that describe sequential computations,
  Verilog describes physical structures — gates, wires, flip-flops — that
  exist simultaneously and operate in parallel.

  ## Preprocessor

  Verilog source files often contain compiler directives (lines starting with
  a backtick `` ` ``). These directives control conditional compilation, macro
  expansion, and file inclusion — similar to the C preprocessor. The
  `VerilogLexer.Preprocessor` module handles these before the source reaches
  the lexer.

  Supported directives:
  - `` `define `` / `` `undef `` — macro definition and removal
  - `` `ifdef `` / `` `ifndef `` / `` `else `` / `` `endif `` — conditional compilation
  - `` `include `` — file inclusion (stubbed: emits a comment placeholder)
  - `` `timescale `` — time unit specification (stripped entirely)
  - Parameterized macros: `` `define ADD(a, b) (a + b) ``

  ## Usage

      # Basic tokenization (no preprocessing)
      {:ok, tokens} = CodingAdventures.VerilogLexer.tokenize("module foo; endmodule")

      # With preprocessing enabled
      source = ~s(`define WIDTH 8\\nwire [`WIDTH-1:0] bus;)
      {:ok, tokens} = CodingAdventures.VerilogLexer.tokenize(source, preprocess: true)

  ## How It Works

  1. If `preprocess: true` is passed, the source is first run through
     `VerilogLexer.Preprocessor.process/1` to expand macros, evaluate
     conditionals, and strip directives.

  2. The (possibly preprocessed) source is passed to `GrammarLexer.tokenize/2`
     with the `verilog.tokens` grammar.

  3. The grammar is cached in a `persistent_term` for fast repeated access.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.VerilogLexer.Preprocessor

  # ---------------------------------------------------------------------------
  # Grammar File Location
  # ---------------------------------------------------------------------------
  #
  # The verilog.tokens file lives in code/grammars/ at the repository root.
  # From this file's location (lib/verilog_lexer.ex), we navigate:
  #   lib/ -> verilog_lexer/ -> elixir/ -> packages/ -> code/ -> grammars/

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Tokenize Verilog source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.

  ## Options

  - `:preprocess` — when `true`, runs the source through the Verilog
    preprocessor before tokenizing. This expands macros, evaluates
    conditional compilation blocks, and strips directives like `timescale.
    Defaults to `false`.

  ## Examples

      # Simple module declaration
      {:ok, tokens} = CodingAdventures.VerilogLexer.tokenize("module top; endmodule")

      # With preprocessing
      source = ~s(`define WIDTH 8\\nreg [`WIDTH-1:0] data;)
      {:ok, tokens} = CodingAdventures.VerilogLexer.tokenize(source, preprocess: true)
  """
  @spec tokenize(String.t(), keyword()) ::
          {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source, opts \\ []) do
    # ---------------------------------------------------------------------------
    # Step 1: Optional Preprocessing
    # ---------------------------------------------------------------------------
    #
    # If the caller passes `preprocess: true`, we run the source through
    # the preprocessor first. This handles `define, `ifdef, `include, etc.
    # The preprocessor returns plain Verilog source with all directives
    # resolved and removed.

    processed_source =
      if Keyword.get(opts, :preprocess, false) do
        Preprocessor.process(source)
      else
        source
      end

    # ---------------------------------------------------------------------------
    # Step 2: Grammar-Driven Tokenization
    # ---------------------------------------------------------------------------
    #
    # The verilog.tokens grammar handles all the lexical rules: keywords,
    # operators, number literals (sized, real, plain), identifiers, system
    # tasks ($display), compiler directives (`timescale), and escaped
    # identifiers (\special.name).

    grammar = get_grammar()
    GrammarLexer.tokenize(processed_source, grammar)
  end

  @doc """
  Parse the verilog.tokens grammar file and return the TokenGrammar.

  This is useful if you want to inspect the grammar or reuse it directly.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "verilog.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # ---------------------------------------------------------------------------
  # Grammar Caching
  # ---------------------------------------------------------------------------
  #
  # Parsing the .tokens file involves reading from disk and building regex
  # patterns. We cache the result in a persistent_term so that the first call
  # pays the cost, but subsequent calls are essentially free (a single ETS
  # lookup).

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
