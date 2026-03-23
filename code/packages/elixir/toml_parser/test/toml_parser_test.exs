defmodule CodingAdventures.TomlParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.TomlParser
  alias CodingAdventures.Parser.ASTNode

  # ---------------------------------------------------------------------------
  # Grammar loading
  # ---------------------------------------------------------------------------

  describe "create_parser/0" do
    test "returns a ParserGrammar with TOML rules" do
      grammar = TomlParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "document" in rule_names
      assert "expression" in rule_names
      assert "keyval" in rule_names
      assert "key" in rule_names
      assert "simple_key" in rule_names
      assert "table_header" in rule_names
      assert "array_table_header" in rule_names
      assert "value" in rule_names
      assert "array" in rule_names
      assert "array_values" in rule_names
      assert "inline_table" in rule_names
    end

    test "document is the first (entry) rule" do
      grammar = TomlParser.create_parser()
      assert hd(grammar.rules).name == "document"
    end
  end

  # ---------------------------------------------------------------------------
  # Key-value pairs — primitives
  # ---------------------------------------------------------------------------

  describe "parse/1 — key-value primitives" do
    test "parses bare key with string value" do
      {:ok, node} = TomlParser.parse(~s(title = "TOML Example"))
      assert node.rule_name == "document"
      # document -> expression -> keyval
      [expr] = node.children
      assert expr.rule_name == "expression"
      [keyval] = expr.children
      assert keyval.rule_name == "keyval"
    end

    test "parses bare key with integer value" do
      {:ok, node} = TomlParser.parse("port = 8080")
      assert node.rule_name == "document"
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "INTEGER"
      assert token.value == "8080"
    end

    test "parses bare key with float value" do
      {:ok, node} = TomlParser.parse("pi = 3.14")
      assert node.rule_name == "document"
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "FLOAT"
    end

    test "parses bare key with boolean true" do
      {:ok, node} = TomlParser.parse("enabled = true")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "TRUE"
    end

    test "parses bare key with boolean false" do
      {:ok, node} = TomlParser.parse("debug = false")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "FALSE"
    end

    test "parses bare key with offset datetime value" do
      {:ok, node} = TomlParser.parse("created = 1979-05-27T07:32:00Z")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "OFFSET_DATETIME"
    end

    test "parses bare key with local date value" do
      {:ok, node} = TomlParser.parse("birthday = 1979-05-27")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "LOCAL_DATE"
    end

    test "parses bare key with local time value" do
      {:ok, node} = TomlParser.parse("alarm = 07:32:00")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "LOCAL_TIME"
    end

    test "parses literal string value" do
      {:ok, node} = TomlParser.parse("path = 'C:\\Users\\foo'")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [token] = value_node.children
      assert token.type == "LITERAL_STRING"
    end
  end

  # ---------------------------------------------------------------------------
  # Keys — dotted and quoted
  # ---------------------------------------------------------------------------

  describe "parse/1 — key types" do
    test "parses dotted key" do
      {:ok, node} = TomlParser.parse("a.b.c = 1")
      keyval = get_keyval(node)
      key_node = get_key(keyval)
      # key -> simple_key DOT simple_key DOT simple_key
      simple_keys =
        key_node.children
        |> Enum.filter(&match?(%ASTNode{rule_name: "simple_key"}, &1))

      assert length(simple_keys) == 3
    end

    test "parses quoted key" do
      {:ok, node} = TomlParser.parse(~s("quoted key" = 1))
      keyval = get_keyval(node)
      key_node = get_key(keyval)
      [simple_key] = Enum.filter(key_node.children, &match?(%ASTNode{}, &1))
      [token] = simple_key.children
      assert token.type == "BASIC_STRING"
    end

    test "parses literal quoted key" do
      {:ok, node} = TomlParser.parse("'literal key' = 1")
      keyval = get_keyval(node)
      key_node = get_key(keyval)
      [simple_key] = Enum.filter(key_node.children, &match?(%ASTNode{}, &1))
      [token] = simple_key.children
      assert token.type == "LITERAL_STRING"
    end

    test "parses integer as key (TOML allows this)" do
      {:ok, _node} = TomlParser.parse("42 = true")
    end

    test "parses true as key (TOML allows this)" do
      {:ok, _node} = TomlParser.parse("true = 1")
    end
  end

  # ---------------------------------------------------------------------------
  # Table headers
  # ---------------------------------------------------------------------------

  describe "parse/1 — table headers" do
    test "parses simple table header" do
      {:ok, node} = TomlParser.parse("[server]")
      assert node.rule_name == "document"
      [expr] = node.children
      [header] = expr.children
      assert header.rule_name == "table_header"
    end

    test "parses table header with dotted key" do
      {:ok, node} = TomlParser.parse("[a.b.c]")
      assert node.rule_name == "document"
      [expr] = node.children
      [header] = expr.children
      assert header.rule_name == "table_header"
    end

    test "parses table header followed by key-value pairs" do
      source = "[server]\nhost = \"localhost\"\nport = 8080"
      {:ok, node} = TomlParser.parse(source)
      assert node.rule_name == "document"
      # Should have 3 expressions: table_header, keyval, keyval
      expressions =
        node.children
        |> Enum.filter(&match?(%ASTNode{rule_name: "expression"}, &1))

      assert length(expressions) == 3
    end
  end

  # ---------------------------------------------------------------------------
  # Array-of-tables headers
  # ---------------------------------------------------------------------------

  describe "parse/1 — array-of-tables headers" do
    test "parses array-of-tables header" do
      {:ok, node} = TomlParser.parse("[[products]]")
      [expr] = node.children
      [header] = expr.children
      assert header.rule_name == "array_table_header"
    end

    test "parses array-of-tables with entries" do
      source = "[[products]]\nname = \"Hammer\"\n[[products]]\nname = \"Nail\""
      {:ok, node} = TomlParser.parse(source)
      assert node.rule_name == "document"
      expressions =
        node.children
        |> Enum.filter(&match?(%ASTNode{rule_name: "expression"}, &1))

      # 2 array-table headers + 2 key-value pairs = 4 expressions
      assert length(expressions) == 4
    end
  end

  # ---------------------------------------------------------------------------
  # Arrays
  # ---------------------------------------------------------------------------

  describe "parse/1 — arrays" do
    test "parses empty array" do
      {:ok, node} = TomlParser.parse("a = []")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [array_node] = value_node.children
      assert array_node.rule_name == "array"
    end

    test "parses array with elements" do
      {:ok, node} = TomlParser.parse(~s(colors = ["red", "green", "blue"]))
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [array_node] = value_node.children
      assert array_node.rule_name == "array"
    end

    test "parses array with trailing comma" do
      {:ok, _node} = TomlParser.parse(~s(a = [1, 2, 3,]))
    end

    test "parses multi-line array" do
      source = "a = [\n  1,\n  2,\n  3,\n]"
      {:ok, _node} = TomlParser.parse(source)
    end

    test "parses nested arrays" do
      {:ok, _node} = TomlParser.parse("a = [[1, 2], [3, 4]]")
    end

    test "parses array with mixed types" do
      {:ok, _node} = TomlParser.parse(~s(a = [1, "two", true, 3.14]))
    end
  end

  # ---------------------------------------------------------------------------
  # Inline tables
  # ---------------------------------------------------------------------------

  describe "parse/1 — inline tables" do
    test "parses empty inline table" do
      {:ok, node} = TomlParser.parse("a = {}")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [inline_table] = value_node.children
      assert inline_table.rule_name == "inline_table"
    end

    test "parses inline table with key-value pairs" do
      {:ok, node} = TomlParser.parse("point = { x = 1, y = 2 }")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      [inline_table] = value_node.children
      assert inline_table.rule_name == "inline_table"
    end

    test "parses inline table with string values" do
      {:ok, _node} = TomlParser.parse(~s(name = { first = "Tom", last = "Preston-Werner" }))
    end

    test "parses nested inline table" do
      {:ok, _node} = TomlParser.parse("a = { b = { c = 1 } }")
    end
  end

  # ---------------------------------------------------------------------------
  # Document structure
  # ---------------------------------------------------------------------------

  describe "parse/1 — document structure" do
    test "parses empty document" do
      {:ok, node} = TomlParser.parse("")
      assert node.rule_name == "document"
      assert node.children == []
    end

    test "parses comment-only document" do
      {:ok, node} = TomlParser.parse("# just a comment")
      assert node.rule_name == "document"
    end

    test "parses multiple key-value pairs" do
      source = "a = 1\nb = 2\nc = 3"
      {:ok, node} = TomlParser.parse(source)
      expressions =
        node.children
        |> Enum.filter(&match?(%ASTNode{rule_name: "expression"}, &1))

      assert length(expressions) == 3
    end

    test "parses blank lines between expressions" do
      source = "a = 1\n\n\nb = 2"
      {:ok, node} = TomlParser.parse(source)
      expressions =
        node.children
        |> Enum.filter(&match?(%ASTNode{rule_name: "expression"}, &1))

      assert length(expressions) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Realistic TOML documents
  # ---------------------------------------------------------------------------

  describe "parse/1 — realistic documents" do
    test "parses a typical server config" do
      source = """
      # Server configuration
      [server]
      host = "0.0.0.0"
      port = 8080
      enabled = true

      [database]
      server = "192.168.1.1"
      ports = [8001, 8001, 8002]
      connection_max = 5000
      """

      {:ok, node} = TomlParser.parse(source)
      assert node.rule_name == "document"
    end

    test "parses document with inline tables and arrays" do
      source = """
      [package]
      name = "toml_parser"
      version = "0.1.0"
      authors = ["Alice", "Bob"]
      metadata = { license = "MIT", homepage = "https://example.com" }
      """

      {:ok, node} = TomlParser.parse(source)
      assert node.rule_name == "document"
    end

    test "parses document with array-of-tables" do
      source = """
      [[fruits]]
      name = "apple"

      [[fruits]]
      name = "banana"

      [[fruits]]
      name = "cherry"
      """

      {:ok, node} = TomlParser.parse(source)
      assert node.rule_name == "document"
    end

    test "parses document with all value types" do
      source = """
      string = "hello"
      integer = 42
      float = 3.14
      bool = true
      date = 1979-05-27
      time = 07:32:00
      datetime = 1979-05-27T07:32:00Z
      array = [1, 2, 3]
      inline = { a = 1 }
      """

      {:ok, node} = TomlParser.parse(source)
      assert node.rule_name == "document"
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "parse/1 — error cases" do
    test "error on missing value" do
      {:error, _msg} = TomlParser.parse("key =")
    end

    test "error on invalid character" do
      {:error, msg} = TomlParser.parse("@")
      assert msg =~ "Unexpected"
    end

    test "error on missing equals in key-value" do
      {:error, _msg} = TomlParser.parse("key value")
    end
  end

  # ---------------------------------------------------------------------------
  # ASTNode helpers
  # ---------------------------------------------------------------------------

  describe "ASTNode helpers" do
    test "leaf? works through parse" do
      {:ok, node} = TomlParser.parse("a = 1")
      keyval = get_keyval(node)
      value_node = get_value(keyval)
      assert ASTNode.leaf?(value_node)
      assert ASTNode.token(value_node).value == "1"
    end
  end

  # ---------------------------------------------------------------------------
  # Test helpers
  # ---------------------------------------------------------------------------

  # Navigate from document -> first expression -> keyval
  defp get_keyval(document) do
    [expr | _] = document.children
    [child | _] = expr.children
    assert child.rule_name == "keyval"
    child
  end

  # Get the key node from a keyval
  defp get_key(keyval) do
    Enum.find(keyval.children, &match?(%ASTNode{rule_name: "key"}, &1))
  end

  # Get the value node from a keyval
  defp get_value(keyval) do
    Enum.find(keyval.children, &match?(%ASTNode{rule_name: "value"}, &1))
  end
end
