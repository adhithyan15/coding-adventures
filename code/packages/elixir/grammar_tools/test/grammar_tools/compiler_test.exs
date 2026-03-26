defmodule CodingAdventures.GrammarTools.CompilerTest do
  @moduledoc """
  Tests for the grammar compiler (compiler.ex).

  The compiler transforms in-memory `TokenGrammar` and `ParserGrammar` structs
  into Elixir source code.  Tests verify:

    1. The generated code contains the expected header / DO NOT EDIT comment.
    2. The generated code is valid Elixir (eval-able via `Code.eval_string/1`).
    3. Loading the generated code recreates an equivalent grammar struct.
    4. All grammar features round-trip: aliases, skip patterns, error patterns,
       groups, keywords, mode, escape_mode, case_sensitive, case_insensitive.
    5. Edge cases: empty grammars, special chars in patterns, nested elements.

  ## Round-trip strategy

      original = TokenGrammar.parse!(source)
      code     = Compiler.compile_token_grammar(original)
      {grammar, _} = Code.eval_string(code)
      assert grammar.definitions == original.definitions

  We use `Code.eval_string/1` to run the generated Elixir code and extract the
  returned struct.  Because the generated code only uses structs, maps, lists,
  and atoms — no I/O, no side effects — this is safe in tests.
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar, Compiler}

  # ---------------------------------------------------------------------------
  # Helper: eval generated Elixir code and return the grammar struct.
  #
  # The generated output is a `def token_grammar do ... end` or
  # `def parser_grammar do ... end` expression (not a full module).  We wrap
  # it in a module binding so `Code.eval_string` can see the aliases.
  # ---------------------------------------------------------------------------

  # Wrap the generated code in a fresh anonymous module so that `def` statements
  # are valid. Each call gets a unique module name to avoid conflicts.
  defp eval_token_grammar(code) do
    mod_name = :"TestEvalTokenGrammar#{System.unique_integer([:positive])}"

    full_code = """
    defmodule #{mod_name} do
      #{code}
    end
    #{mod_name}.token_grammar()
    """

    {grammar, _} = Code.eval_string(full_code)
    grammar
  end

  defp eval_parser_grammar(code) do
    mod_name = :"TestEvalParserGrammar#{System.unique_integer([:positive])}"

    full_code = """
    defmodule #{mod_name} do
      #{code}
    end
    #{mod_name}.parser_grammar()
    """

    {grammar, _} = Code.eval_string(full_code)
    grammar
  end

  # ===========================================================================
  # compile_token_grammar — output structure
  # ===========================================================================

  describe "compile_token_grammar/2 output structure" do
    test "includes DO NOT EDIT header" do
      code = Compiler.compile_token_grammar(%TokenGrammar{})
      assert String.contains?(code, "DO NOT EDIT")
    end

    test "includes source file when given" do
      code = Compiler.compile_token_grammar(%TokenGrammar{}, "json.tokens")
      assert String.contains?(code, "json.tokens")
    end

    test "omits source line when empty" do
      code = Compiler.compile_token_grammar(%TokenGrammar{}, "")
      refute String.contains?(code, "# Source:")
    end

    test "includes TokenGrammar alias" do
      code = Compiler.compile_token_grammar(%TokenGrammar{})
      assert String.contains?(code, "TokenGrammar")
    end

    test "includes def token_grammar" do
      code = Compiler.compile_token_grammar(%TokenGrammar{})
      assert String.contains?(code, "def token_grammar")
    end
  end

  # ===========================================================================
  # compile_token_grammar — round-trip tests
  # ===========================================================================

  describe "compile_token_grammar/2 round-trip" do
    test "empty grammar round-trips" do
      original = %TokenGrammar{}
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert loaded.definitions == original.definitions
      assert loaded.keywords == original.keywords
      assert loaded.version == original.version
      assert loaded.case_insensitive == original.case_insensitive
    end

    test "regex token round-trips" do
      {:ok, original} = TokenGrammar.parse("NUMBER = /[0-9]+/")
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert length(loaded.definitions) == 1
      defn = hd(loaded.definitions)
      assert defn.name == "NUMBER"
      assert defn.pattern == "[0-9]+"
      assert defn.is_regex == true
    end

    test "literal token round-trips" do
      {:ok, original} = TokenGrammar.parse(~s(PLUS = "+"))
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      defn = hd(loaded.definitions)
      assert defn.name == "PLUS"
      assert defn.pattern == "+"
      assert defn.is_regex == false
    end

    test "alias round-trips" do
      {:ok, original} = TokenGrammar.parse(~s(STRING_DQ = /"[^"]*"/ -> STRING))
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert hd(loaded.definitions).alias == "STRING"
    end

    test "keywords round-trip" do
      source = "NAME = /[a-z]+/\nkeywords:\n  if\n  else\n  while\n"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert loaded.keywords == ["if", "else", "while"]
    end

    test "skip definitions round-trip" do
      source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert length(loaded.skip_definitions) == 1
      assert hd(loaded.skip_definitions).name == "WHITESPACE"
    end

    test "error definitions round-trip" do
      source = "STRING = /\"[^\"]*\"/\nerrors:\n  BAD_STRING = /\"[^\"\\n]*/\n"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert length(loaded.error_definitions) == 1
      assert hd(loaded.error_definitions).name == "BAD_STRING"
    end

    test "mode round-trips" do
      source = "mode: indentation\nNAME = /[a-z]+/"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert loaded.mode == "indentation"
    end

    test "escape_mode round-trips" do
      source = "escapes: none\nSTRING = /\"[^\"]*\"/"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert loaded.escape_mode == "none"
    end

    test "version round-trips" do
      source = "# @version 3\nNAME = /[a-z]+/"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert loaded.version == 3
    end

    test "case_insensitive round-trips" do
      source = "# @case_insensitive true\nNAME = /[a-z]+/"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert loaded.case_insensitive == true
    end

    test "pattern groups round-trip" do
      source = "TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\n  EQ = \"=\"\n"
      {:ok, original} = TokenGrammar.parse(source)
      loaded = eval_token_grammar(Compiler.compile_token_grammar(original))
      assert Map.has_key?(loaded.groups, "tag")
      assert length(loaded.groups["tag"].definitions) == 2
    end
  end

  # ===========================================================================
  # compile_parser_grammar — output structure
  # ===========================================================================

  describe "compile_parser_grammar/2 output structure" do
    test "includes DO NOT EDIT header" do
      code = Compiler.compile_parser_grammar(%ParserGrammar{})
      assert String.contains?(code, "DO NOT EDIT")
    end

    test "includes def parser_grammar" do
      code = Compiler.compile_parser_grammar(%ParserGrammar{})
      assert String.contains?(code, "def parser_grammar")
    end

    test "includes ParserGrammar alias" do
      code = Compiler.compile_parser_grammar(%ParserGrammar{})
      assert String.contains?(code, "ParserGrammar")
    end
  end

  # ===========================================================================
  # compile_parser_grammar — round-trip tests
  # ===========================================================================

  describe "compile_parser_grammar/2 round-trip" do
    test "empty grammar round-trips" do
      original = %ParserGrammar{}
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      assert loaded.version == 0
      assert loaded.rules == []
    end

    test "rule reference round-trips" do
      {:ok, original} = ParserGrammar.parse("value = NUMBER ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      assert length(loaded.rules) == 1
      rule = hd(loaded.rules)
      assert rule.name == "value"
      assert rule.body == {:rule_reference, "NUMBER", true}
    end

    test "alternation round-trips" do
      {:ok, original} = ParserGrammar.parse("value = A | B | C ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      {:alternation, choices} = hd(loaded.rules).body
      assert length(choices) == 3
    end

    test "sequence round-trips" do
      {:ok, original} = ParserGrammar.parse("pair = KEY COLON VALUE ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      {type, _} = hd(loaded.rules).body
      assert type == :sequence
    end

    test "repetition round-trips" do
      {:ok, original} = ParserGrammar.parse("stmts = { stmt } ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      {type, _} = hd(loaded.rules).body
      assert type == :repetition
    end

    test "optional round-trips" do
      {:ok, original} = ParserGrammar.parse("expr = NUMBER [ PLUS NUMBER ] ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      {:sequence, [_, optional]} = hd(loaded.rules).body
      {type, _} = optional
      assert type == :optional
    end

    test "literal round-trips" do
      {:ok, original} = ParserGrammar.parse(~s(start = "hello" ;))
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      assert hd(loaded.rules).body == {:literal, "hello"}
    end

    test "group round-trips" do
      {:ok, original} = ParserGrammar.parse("expr = ( A | B ) ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      {type, _} = hd(loaded.rules).body
      assert type == :group
    end

    test "version round-trips" do
      {:ok, original} = ParserGrammar.parse("# @version 4\nvalue = NUMBER ;")
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original))
      assert loaded.version == 4
    end

    test "JSON grammar full round-trip" do
      source = """
      value    = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
      object   = LBRACE [ pair { COMMA pair } ] RBRACE ;
      pair     = STRING COLON value ;
      array    = LBRACKET [ value { COMMA value } ] RBRACKET ;
      """

      {:ok, original} = ParserGrammar.parse(source)
      loaded = eval_parser_grammar(Compiler.compile_parser_grammar(original, "json.grammar"))
      assert length(loaded.rules) == 4
      assert hd(loaded.rules).name == "value"
      assert List.last(loaded.rules).name == "array"
    end
  end
end
