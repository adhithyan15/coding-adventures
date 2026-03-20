defmodule CodingAdventures.GrammarTools.TokenGrammarTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.GrammarTools.TokenGrammar

  describe "parse/1 — basic definitions" do
    test "parses regex pattern" do
      {:ok, grammar} = TokenGrammar.parse(~s(NAME = /[a-zA-Z]+/))
      assert length(grammar.definitions) == 1
      [defn] = grammar.definitions
      assert defn.name == "NAME"
      assert defn.pattern == "[a-zA-Z]+"
      assert defn.is_regex == true
    end

    test "parses literal pattern" do
      {:ok, grammar} = TokenGrammar.parse(~s(PLUS = "+"))
      [defn] = grammar.definitions
      assert defn.name == "PLUS"
      assert defn.pattern == "+"
      assert defn.is_regex == false
    end

    test "parses multiple definitions" do
      source = """
      NAME = /[a-zA-Z]+/
      NUMBER = /[0-9]+/
      PLUS = "+"
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.definitions) == 3
    end

    test "records line numbers" do
      source = """
      NAME = /[a-zA-Z]+/

      NUMBER = /[0-9]+/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      [name_defn, number_defn] = grammar.definitions
      assert name_defn.line_number == 1
      assert number_defn.line_number == 3
    end
  end

  describe "parse/1 — comments and blank lines" do
    test "skips comments" do
      source = """
      # This is a comment
      NAME = /[a-zA-Z]+/
      # Another comment
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.definitions) == 1
    end

    test "skips blank lines" do
      source = """
      NAME = /[a-zA-Z]+/

      NUMBER = /[0-9]+/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.definitions) == 2
    end
  end

  describe "parse/1 — keywords section" do
    test "parses keywords" do
      source = """
      NAME = /[a-zA-Z]+/

      keywords:
        if
        else
        while
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert grammar.keywords == ["if", "else", "while"]
    end
  end

  describe "parse/1 — skip section" do
    test "parses skip patterns" do
      source = """
      NAME = /[a-zA-Z]+/

      skip:
        WHITESPACE = /[ \\t]+/
        COMMENT = /#[^\\n]*/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.skip_definitions) == 2
      [ws, comment] = grammar.skip_definitions
      assert ws.name == "WHITESPACE"
      assert comment.name == "COMMENT"
    end
  end

  describe "parse/1 — aliases" do
    test "parses alias syntax" do
      source = ~s(STRING_DQ = /"[^"]*"/ -> STRING)
      {:ok, grammar} = TokenGrammar.parse(source)
      [defn] = grammar.definitions
      assert defn.name == "STRING_DQ"
      assert defn.alias == "STRING"
    end
  end

  describe "parse/1 — mode directive" do
    test "parses mode" do
      source = """
      mode: indentation
      NAME = /[a-zA-Z]+/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert grammar.mode == "indentation"
    end
  end

  describe "parse/1 — reserved keywords" do
    test "parses reserved keywords" do
      source = """
      NAME = /[a-zA-Z]+/

      reserved:
        class
        import
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert grammar.reserved_keywords == ["class", "import"]
    end
  end

  describe "parse/1 — error cases" do
    test "error on missing pattern" do
      {:error, msg} = TokenGrammar.parse("NAME")
      assert msg =~ "Expected token definition"
    end

    test "error on empty regex" do
      {:error, msg} = TokenGrammar.parse("NAME = //")
      assert msg =~ "Empty regex"
    end

    test "error on empty literal" do
      {:error, msg} = TokenGrammar.parse(~s(NAME = ""))
      assert msg =~ "Empty literal"
    end
  end

  describe "parse/1 — json.tokens integration" do
    test "parses the json.tokens file" do
      grammar_dir =
        Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
        |> Path.expand()

      json_tokens = File.read!(Path.join(grammar_dir, "json.tokens"))
      {:ok, grammar} = TokenGrammar.parse(json_tokens)

      names = Enum.map(grammar.definitions, & &1.name)
      assert "STRING" in names
      assert "NUMBER" in names
      assert "TRUE" in names
      assert "FALSE" in names
      assert "NULL" in names
      assert "LBRACE" in names
      assert "RBRACE" in names
      assert "COLON" in names
      assert "COMMA" in names

      assert length(grammar.skip_definitions) == 1
      assert grammar.keywords == []
    end
  end

  describe "token_names/1" do
    test "returns set of defined names" do
      source = """
      NAME = /[a-zA-Z]+/
      NUMBER = /[0-9]+/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      names = TokenGrammar.token_names(grammar)
      assert MapSet.member?(names, "NAME")
      assert MapSet.member?(names, "NUMBER")
    end
  end
end
