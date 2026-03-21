defmodule CodingAdventures.Parser.GrammarParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Parser.{GrammarParser, ASTNode}
  alias CodingAdventures.Lexer.{GrammarLexer, Token}
  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar}

  # -- Helpers ----------------------------------------------------------------

  defp simple_token_grammar do
    {:ok, g} =
      TokenGrammar.parse("""
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      PLUS   = "+"
      MINUS  = "-"
      STAR   = "*"
      """)

    g
  end

  defp tokenize(source, token_grammar) do
    {:ok, tokens} = GrammarLexer.tokenize(source, token_grammar)
    tokens
  end

  defp json_token_grammar do
    grammar_dir =
      Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
      |> Path.expand()

    {:ok, g} = TokenGrammar.parse(File.read!(Path.join(grammar_dir, "json.tokens")))
    g
  end

  defp json_parser_grammar do
    grammar_dir =
      Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
      |> Path.expand()

    {:ok, g} = ParserGrammar.parse(File.read!(Path.join(grammar_dir, "json.grammar")))
    g
  end

  # -- Tests ------------------------------------------------------------------

  describe "parse/2 — simple rules" do
    test "parses a single token" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("value = NUMBER ;")
      tokens = tokenize("42", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
      assert length(node.children) == 1
      [child] = node.children
      assert child.type == "NUMBER"
      assert child.value == "42"
    end

    test "parses alternation" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("value = NUMBER | NAME ;")
      tokens = tokenize("hello", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "NAME"
      assert child.value == "hello"
    end

    test "parses sequence" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("expr = NUMBER PLUS NUMBER ;")
      tokens = tokenize("1 + 2", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "expr"
      assert length(node.children) == 3
      [left, op, right] = node.children
      assert left.value == "1"
      assert op.value == "+"
      assert right.value == "2"
    end
  end

  describe "parse/2 — repetition and optional" do
    test "parses zero-or-more repetition" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("nums = NUMBER { NUMBER } ;")
      tokens = tokenize("1 2 3", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert length(node.children) == 3
    end

    test "parses empty repetition" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("nums = NUMBER { PLUS } ;")
      tokens = tokenize("42", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert length(node.children) == 1
    end

    test "parses optional present" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("maybe = NUMBER [ PLUS ] ;")
      tokens = tokenize("42 +", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert length(node.children) == 2
    end

    test "parses optional absent" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("maybe = NUMBER [ PLUS ] ;")
      tokens = tokenize("42", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert length(node.children) == 1
    end
  end

  describe "parse/2 — recursive rules" do
    test "parses recursive grammar" do
      tg = simple_token_grammar()

      {:ok, pg} =
        ParserGrammar.parse("""
        expr = term { PLUS term } ;
        term = NUMBER ;
        """)

      tokens = tokenize("1 + 2 + 3", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "expr"
      # Should have: term, PLUS, term, PLUS, term
      assert length(node.children) == 5
    end
  end

  describe "parse/2 — error cases" do
    test "error on empty grammar" do
      {:error, msg} = GrammarParser.parse([], %ParserGrammar{rules: []})
      assert msg =~ "no rules"
    end

    test "error on unexpected token" do
      tg = simple_token_grammar()
      {:ok, pg} = ParserGrammar.parse("value = NUMBER ;")
      tokens = tokenize("hello", tg)

      {:error, msg} = GrammarParser.parse(tokens, pg)
      assert msg =~ "Parse error"
    end
  end

  describe "parse/2 — JSON grammar integration" do
    test "parses JSON number" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize("42", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end

    test "parses JSON string" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize(~s("hello"), tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
      [child] = node.children
      assert child.type == "STRING"
      assert child.value == "hello"
    end

    test "parses JSON boolean" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize("true", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end

    test "parses JSON null" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize("null", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end

    test "parses empty JSON object" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize("{}", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
      [object] = node.children
      assert object.rule_name == "object"
    end

    test "parses JSON object with one pair" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize(~s({"key": 42}), tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
      [object] = node.children
      assert object.rule_name == "object"
    end

    test "parses JSON object with multiple pairs" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize(~s({"a": 1, "b": 2, "c": 3}), tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end

    test "parses empty JSON array" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize("[]", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
      [array] = node.children
      assert array.rule_name == "array"
    end

    test "parses JSON array with elements" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      tokens = tokenize("[1, 2, 3]", tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end

    test "parses nested JSON" do
      tg = json_token_grammar()
      pg = json_parser_grammar()
      source = ~s({"users": [{"name": "Alice", "age": 30}]})
      tokens = tokenize(source, tg)

      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end

    test "parses JSON with whitespace" do
      tg = json_token_grammar()
      pg = json_parser_grammar()

      source = """
      {
        "name": "Alice",
        "age": 30,
        "active": true,
        "address": null
      }
      """

      tokens = tokenize(source, tg)
      {:ok, node} = GrammarParser.parse(tokens, pg)
      assert node.rule_name == "value"
    end
  end

  describe "ASTNode" do
    test "leaf? returns true for single-token node" do
      token = %Token{type: "NUMBER", value: "42", line: 1, column: 1}
      node = %ASTNode{rule_name: "value", children: [token]}
      assert ASTNode.leaf?(node)
    end

    test "leaf? returns false for multi-child node" do
      t1 = %Token{type: "NUMBER", value: "1", line: 1, column: 1}
      t2 = %Token{type: "PLUS", value: "+", line: 1, column: 3}
      node = %ASTNode{rule_name: "expr", children: [t1, t2]}
      refute ASTNode.leaf?(node)
    end

    test "token/1 returns token for leaf" do
      token = %Token{type: "NUMBER", value: "42", line: 1, column: 1}
      node = %ASTNode{rule_name: "value", children: [token]}
      assert ASTNode.token(node) == token
    end

    test "token/1 returns nil for non-leaf" do
      sub = %ASTNode{rule_name: "inner", children: []}
      node = %ASTNode{rule_name: "outer", children: [sub]}
      assert ASTNode.token(node) == nil
    end
  end
end
