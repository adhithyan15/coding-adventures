defmodule CodingAdventures.JsonParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.JsonParser
  alias CodingAdventures.Parser.ASTNode

  describe "create_parser/0" do
    test "returns a ParserGrammar with 4 rules" do
      grammar = JsonParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "value" in rule_names
      assert "object" in rule_names
      assert "pair" in rule_names
      assert "array" in rule_names
    end
  end

  describe "parse/1 — primitives" do
    test "parses number" do
      {:ok, node} = JsonParser.parse("42")
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "NUMBER"
      assert child.value == "42"
    end

    test "parses negative number" do
      {:ok, node} = JsonParser.parse("-3.14")
      assert node.rule_name == "value"
      [child] = node.children
      assert child.value == "-3.14"
    end

    test "parses string" do
      {:ok, node} = JsonParser.parse(~s("hello world"))
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "STRING"
      assert child.value == "hello world"
    end

    test "parses true" do
      {:ok, node} = JsonParser.parse("true")
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "TRUE"
    end

    test "parses false" do
      {:ok, node} = JsonParser.parse("false")
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "FALSE"
    end

    test "parses null" do
      {:ok, node} = JsonParser.parse("null")
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "NULL"
    end
  end

  describe "parse/1 — objects" do
    test "parses empty object" do
      {:ok, node} = JsonParser.parse("{}")
      assert node.rule_name == "value"
      [object] = node.children
      assert object.rule_name == "object"
    end

    test "parses object with one pair" do
      {:ok, node} = JsonParser.parse(~s({"key": 42}))
      assert node.rule_name == "value"
      [object] = node.children
      assert object.rule_name == "object"
    end

    test "parses object with multiple pairs" do
      {:ok, node} = JsonParser.parse(~s({"a": 1, "b": 2, "c": 3}))
      assert node.rule_name == "value"
    end
  end

  describe "parse/1 — arrays" do
    test "parses empty array" do
      {:ok, node} = JsonParser.parse("[]")
      assert node.rule_name == "value"
      [array] = node.children
      assert array.rule_name == "array"
    end

    test "parses array with elements" do
      {:ok, node} = JsonParser.parse("[1, 2, 3]")
      assert node.rule_name == "value"
    end

    test "parses array with mixed types" do
      {:ok, node} = JsonParser.parse(~s([1, "two", true, null]))
      assert node.rule_name == "value"
    end
  end

  describe "parse/1 — nested structures" do
    test "parses nested object in array" do
      {:ok, node} = JsonParser.parse(~s([{"a": 1}, {"b": 2}]))
      assert node.rule_name == "value"
    end

    test "parses deeply nested structure" do
      {:ok, node} = JsonParser.parse(~s({"users": [{"name": "Alice", "age": 30}]}))
      assert node.rule_name == "value"
    end

    test "parses RFC 8259 example" do
      source = ~S({
        "Image": {
          "Width": 800,
          "Height": 600,
          "Title": "View from 15th Floor",
          "Thumbnail": {
            "Url": "http://www.example.com/image/481989943",
            "Height": 125,
            "Width": 100
          },
          "Animated": false,
          "IDs": [116, 943, 234, 38793]
        }
      })

      {:ok, node} = JsonParser.parse(source)
      assert node.rule_name == "value"
    end
  end

  describe "parse/1 — whitespace" do
    test "handles significant whitespace" do
      source = """
      {
        "name" : "Alice" ,
        "age"  : 30
      }
      """

      {:ok, node} = JsonParser.parse(source)
      assert node.rule_name == "value"
    end
  end

  describe "parse/1 — error cases" do
    test "error on invalid JSON" do
      {:error, msg} = JsonParser.parse("{")
      assert msg =~ "Parse error" or msg =~ "Unexpected"
    end

    test "error on trailing comma" do
      {:error, _msg} = JsonParser.parse("[1, 2,]")
    end

    test "error on unexpected token" do
      {:error, msg} = JsonParser.parse("@")
      assert msg =~ "Unexpected"
    end
  end

  describe "ASTNode helpers" do
    test "leaf node detection works through parse" do
      {:ok, node} = JsonParser.parse("42")
      assert ASTNode.leaf?(node)
      assert ASTNode.token(node).value == "42"
    end
  end
end
