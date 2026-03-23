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

  # ---------------------------------------------------------------------------
  # Pattern Groups
  # ---------------------------------------------------------------------------
  #
  # Pattern groups enable context-sensitive lexing by defining named sets
  # of token patterns. The lexer maintains a stack of active groups and
  # only tries patterns from the group on top of the stack.

  describe "parse/1 — pattern groups" do
    test "basic group is parsed into a map with name and definitions" do
      source =
        "TEXT = /[^<]+/\n" <>
          "TAG_OPEN = \"<\"\n" <>
          "\n" <>
          "group tag:\n" <>
          "  TAG_NAME = /[a-zA-Z]+/\n" <>
          "  TAG_CLOSE = \">\"\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      # Default group patterns
      assert length(grammar.definitions) == 2
      assert Enum.at(grammar.definitions, 0).name == "TEXT"
      assert Enum.at(grammar.definitions, 1).name == "TAG_OPEN"

      # Named group
      assert Map.has_key?(grammar.groups, "tag")
      group = grammar.groups["tag"]
      assert group.name == "tag"
      assert length(group.definitions) == 2
      assert Enum.at(group.definitions, 0).name == "TAG_NAME"
      assert Enum.at(group.definitions, 1).name == "TAG_CLOSE"
    end

    test "multiple groups can be defined in the same file" do
      source =
        "TEXT = /[^<]+/\n" <>
          "\n" <>
          "group tag:\n" <>
          "  TAG_NAME = /[a-zA-Z]+/\n" <>
          "\n" <>
          "group cdata:\n" <>
          "  CDATA_TEXT = /[^]]+/\n" <>
          "  CDATA_END = \"]]>\"\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      assert map_size(grammar.groups) == 2
      assert Map.has_key?(grammar.groups, "tag")
      assert Map.has_key?(grammar.groups, "cdata")
      assert length(grammar.groups["tag"].definitions) == 1
      assert length(grammar.groups["cdata"].definitions) == 2
    end

    test "definitions inside groups support -> ALIAS" do
      source =
        "TEXT = /[^<]+/\n" <>
          "\n" <>
          "group tag:\n" <>
          "  ATTR_VALUE_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n" <>
          "  ATTR_VALUE_SQ = /'[^']*'/ -> ATTR_VALUE\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      group = grammar.groups["tag"]
      assert Enum.at(group.definitions, 0).name == "ATTR_VALUE_DQ"
      assert Enum.at(group.definitions, 0).alias == "ATTR_VALUE"
      assert Enum.at(group.definitions, 1).name == "ATTR_VALUE_SQ"
      assert Enum.at(group.definitions, 1).alias == "ATTR_VALUE"
    end

    test "groups support both regex and literal patterns" do
      source =
        "TEXT = /[^<]+/\n" <>
          "\n" <>
          "group tag:\n" <>
          "  EQUALS = \"=\"\n" <>
          "  TAG_NAME = /[a-zA-Z]+/\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      group = grammar.groups["tag"]
      assert Enum.at(group.definitions, 0).is_regex == false
      assert Enum.at(group.definitions, 0).pattern == "="
      assert Enum.at(group.definitions, 1).is_regex == true
    end

    test "files without groups have an empty groups map" do
      source = "NUMBER = /[0-9]+/\nPLUS = \"+\"\n"
      {:ok, grammar} = TokenGrammar.parse(source)

      assert grammar.groups == %{}
      assert length(grammar.definitions) == 2
    end

    test "skip: and group: sections coexist correctly" do
      source =
        "skip:\n" <>
          "  WS = /[ \\t]+/\n" <>
          "\n" <>
          "TEXT = /[^<]+/\n" <>
          "\n" <>
          "group tag:\n" <>
          "  TAG_NAME = /[a-zA-Z]+/\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      assert length(grammar.skip_definitions) == 1
      assert length(grammar.definitions) == 1
      assert map_size(grammar.groups) == 1
    end
  end

  describe "token_names/1 — with groups" do
    test "includes names from all groups" do
      source =
        "TEXT = /[^<]+/\n" <>
          "\n" <>
          "group tag:\n" <>
          "  TAG_NAME = /[a-zA-Z]+/\n" <>
          "  ATTR_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      names = TokenGrammar.token_names(grammar)
      assert MapSet.member?(names, "TEXT")
      assert MapSet.member?(names, "TAG_NAME")
      assert MapSet.member?(names, "ATTR_DQ")
      assert MapSet.member?(names, "ATTR_VALUE")
    end
  end

  # ---------------------------------------------------------------------------
  # Error Definitions Section
  # ---------------------------------------------------------------------------
  #
  # The errors: section defines patterns for error recovery. When the lexer
  # fails to match any normal token or skip pattern, it tries these patterns
  # before raising a LexError. This allows graceful degradation for malformed
  # inputs — e.g., CSS emits BAD_STRING for unclosed strings instead of
  # crashing.

  describe "parse/1 — errors section" do
    test "parses error patterns into error_definitions" do
      source = """
      STRING = /"[^"]*"/

      errors:
        BAD_STRING = /"[^"\\n]*/
        BAD_URI = /url\\([^)\\n]*/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.error_definitions) == 2
      [bad_str, bad_uri] = grammar.error_definitions
      assert bad_str.name == "BAD_STRING"
      assert bad_uri.name == "BAD_URI"
    end

    test "empty error_definitions when no errors: section" do
      {:ok, grammar} = TokenGrammar.parse("NAME = /[a-z]+/")
      assert grammar.error_definitions == []
    end

    test "error definitions support alias syntax" do
      source = "NAME = /[a-z]+/\n\nerrors:\n  BAD_STR_DQ = /\"[^\"\\n]*/ -> BAD_STRING\n"
      {:ok, grammar} = TokenGrammar.parse(source)
      [defn] = grammar.error_definitions
      assert defn.alias == "BAD_STRING"
    end

    test "errors: section and skip: section coexist" do
      source = """
      STRING = /"[^"]*"/

      skip:
        WS = /[ \\t]+/

      errors:
        BAD_STRING = /"[^"\\n]*/
      """

      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.skip_definitions) == 1
      assert length(grammar.error_definitions) == 1
    end

    test "errors: with a space before colon is recognized" do
      source = "NAME = /[a-z]+/\n\nerrors :\n  BAD = /x/\n"
      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.error_definitions) == 1
    end

    test "non-indented line after errors: section exits back to definitions" do
      source = "NAME = /[a-z]+/\n\nerrors:\n  BAD = /x/\n\nNUMBER = /[0-9]+/\n"
      {:ok, grammar} = TokenGrammar.parse(source)
      assert length(grammar.definitions) == 2
      assert length(grammar.error_definitions) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # validate_token_grammar/1
  # ---------------------------------------------------------------------------
  #
  # The validator runs a lint pass over a parsed grammar, catching semantic
  # issues that would cause problems downstream without failing the parser.

  describe "validate_token_grammar/1 — valid grammar" do
    test "returns empty list for a valid grammar" do
      {:ok, grammar} = TokenGrammar.parse("NAME = /[a-zA-Z]+/\nNUMBER = /[0-9]+/")
      assert TokenGrammar.validate_token_grammar(grammar) == []
    end
  end

  describe "validate_token_grammar/1 — duplicate names" do
    test "reports duplicate token name" do
      # We have to manually construct a grammar with duplicate names since the
      # parser would accept both (it doesn't validate semantic uniqueness).
      defn1 = %{name: "NAME", pattern: "[a-z]+", is_regex: true, line_number: 1, alias: nil}
      defn2 = %{name: "NAME", pattern: "[A-Z]+", is_regex: true, line_number: 3, alias: nil}
      grammar = %TokenGrammar{definitions: [defn1, defn2]}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Duplicate token name 'NAME'"))
      assert Enum.any?(issues, &(&1 =~ "first defined on line 1"))
    end
  end

  describe "validate_token_grammar/1 — invalid regex" do
    test "reports invalid regex pattern" do
      defn = %{name: "BAD", pattern: "[unclosed", is_regex: true, line_number: 1, alias: nil}
      grammar = %TokenGrammar{definitions: [defn]}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Invalid regex for token 'BAD'"))
    end
  end

  describe "validate_token_grammar/1 — naming conventions" do
    test "reports non-UPPER_CASE token name" do
      defn = %{name: "lowercase_name", pattern: "x", is_regex: false, line_number: 1, alias: nil}
      grammar = %TokenGrammar{definitions: [defn]}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "should be UPPER_CASE"))
    end

    test "reports non-UPPER_CASE alias" do
      defn = %{name: "FOO", pattern: "x", is_regex: false, line_number: 1, alias: "lowercase_alias"}
      grammar = %TokenGrammar{definitions: [defn]}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Alias 'lowercase_alias'"))
      assert Enum.any?(issues, &(&1 =~ "should be UPPER_CASE"))
    end
  end

  describe "validate_token_grammar/1 — mode checks" do
    test "reports unknown mode" do
      grammar = %TokenGrammar{mode: "unknown_mode"}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Unknown lexer mode 'unknown_mode'"))
    end

    test "accepts 'indentation' mode" do
      grammar = %TokenGrammar{mode: "indentation"}
      issues = TokenGrammar.validate_token_grammar(grammar)
      refute Enum.any?(issues, &(&1 =~ "Unknown lexer mode"))
    end
  end

  describe "validate_token_grammar/1 — escape_mode checks" do
    test "reports unknown escape_mode" do
      grammar = %TokenGrammar{escape_mode: "strict"}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "Unknown escape mode 'strict'"))
    end

    test "accepts 'none' escape_mode" do
      grammar = %TokenGrammar{escape_mode: "none"}
      issues = TokenGrammar.validate_token_grammar(grammar)
      refute Enum.any?(issues, &(&1 =~ "Unknown escape mode"))
    end
  end

  describe "validate_token_grammar/1 — skip and error definitions" do
    test "validates skip_definitions with same checks" do
      skip_defn = %{name: "ws", pattern: "[ ]+", is_regex: true, line_number: 5, alias: nil}
      grammar = %TokenGrammar{skip_definitions: [skip_defn]}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "should be UPPER_CASE"))
    end

    test "validates error_definitions with same checks" do
      err_defn = %{name: "bad", pattern: "x", is_regex: false, line_number: 7, alias: nil}
      grammar = %TokenGrammar{error_definitions: [err_defn]}
      issues = TokenGrammar.validate_token_grammar(grammar)
      assert Enum.any?(issues, &(&1 =~ "should be UPPER_CASE"))
    end
  end

  describe "effective_token_names/1 — with groups" do
    test "includes aliased names from groups" do
      source =
        "TEXT = /[^<]+/\n" <>
          "\n" <>
          "group tag:\n" <>
          "  ATTR_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n"

      {:ok, grammar} = TokenGrammar.parse(source)

      names = TokenGrammar.effective_token_names(grammar)
      assert MapSet.member?(names, "TEXT")
      assert MapSet.member?(names, "ATTR_VALUE")
      # alias replaces name in effective set
      refute MapSet.member?(names, "ATTR_DQ")
    end
  end

  # ---------------------------------------------------------------------------
  # Pattern Group Error Cases
  # ---------------------------------------------------------------------------

  describe "parse/1 — pattern group errors" do
    test "'group :' with no name raises an error" do
      source = "TEXT = /abc/\ngroup :\n  FOO = /x/\n"
      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Missing group name"
    end

    test "uppercase group names are rejected" do
      source = "TEXT = /abc/\ngroup Tag:\n  FOO = /x/\n"
      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Invalid group name"
    end

    test "group names starting with a digit are rejected" do
      source = "TEXT = /abc/\ngroup 1tag:\n  FOO = /x/\n"
      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Invalid group name"
    end

    test "'group default:' is rejected as reserved" do
      source = "TEXT = /abc/\ngroup default:\n  FOO = /x/\n"
      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Reserved group name"
    end

    test "'group skip:' is rejected as reserved" do
      source = "TEXT = /abc/\ngroup skip:\n  FOO = /x/\n"
      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Reserved group name"
    end

    test "'group keywords:' is rejected as reserved" do
      source = "TEXT = /abc/\ngroup keywords:\n  FOO = /x/\n"
      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Reserved group name"
    end

    test "duplicate group names are rejected" do
      source =
        "TEXT = /abc/\n" <>
          "group tag:\n" <>
          "  FOO = /x/\n" <>
          "group tag:\n" <>
          "  BAR = /y/\n"

      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Duplicate group name"
    end

    test "invalid definition inside a group raises an error" do
      source =
        "TEXT = /abc/\n" <>
          "group tag:\n" <>
          "  not a definition\n"

      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Expected token definition"
    end

    test "missing pattern in group definition raises an error" do
      source =
        "TEXT = /abc/\n" <>
          "group tag:\n" <>
          "  FOO = \n"

      {:error, msg} = TokenGrammar.parse(source)
      assert msg =~ "Incomplete definition"
    end
  end
end
