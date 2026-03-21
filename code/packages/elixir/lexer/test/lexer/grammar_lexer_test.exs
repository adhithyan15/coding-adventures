defmodule CodingAdventures.Lexer.GrammarLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.GrammarTools.TokenGrammar

  # Helper to create a simple grammar for testing
  defp simple_grammar do
    {:ok, g} =
      TokenGrammar.parse("""
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      PLUS   = "+"
      MINUS  = "-"
      """)

    g
  end

  defp json_grammar do
    grammar_dir =
      Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
      |> Path.expand()

    {:ok, g} = TokenGrammar.parse(File.read!(Path.join(grammar_dir, "json.tokens")))
    g
  end

  describe "tokenize/2 — basic tokens" do
    test "tokenizes a simple expression" do
      {:ok, tokens} = GrammarLexer.tokenize("x + 42", simple_grammar())

      types = Enum.map(tokens, & &1.type)
      values = Enum.map(tokens, & &1.value)

      assert types == ["NAME", "PLUS", "NUMBER", "EOF"]
      assert values == ["x", "+", "42", ""]
    end

    test "tokenizes identifiers" do
      {:ok, tokens} = GrammarLexer.tokenize("foo bar_baz", simple_grammar())
      types = Enum.map(tokens, & &1.type)
      assert types == ["NAME", "NAME", "EOF"]
    end

    test "tokenizes numbers" do
      {:ok, tokens} = GrammarLexer.tokenize("123 456", simple_grammar())
      values = Enum.map(tokens, & &1.value)
      assert values == ["123", "456", ""]
    end

    test "returns EOF for empty input" do
      {:ok, tokens} = GrammarLexer.tokenize("", simple_grammar())
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end

    test "returns EOF for whitespace-only input" do
      {:ok, tokens} = GrammarLexer.tokenize("   ", simple_grammar())
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end
  end

  describe "tokenize/2 — position tracking" do
    test "tracks line and column" do
      {:ok, tokens} = GrammarLexer.tokenize("x + 42", simple_grammar())

      [name, plus, number, _eof] = tokens
      assert name.line == 1
      assert name.column == 1
      assert plus.line == 1
      assert plus.column == 3
      assert number.line == 1
      assert number.column == 5
    end

    test "tracks lines across newlines" do
      {:ok, tokens} = GrammarLexer.tokenize("x\n42", simple_grammar())

      [name, _newline, number, _eof] = tokens
      assert name.line == 1
      assert number.line == 2
      assert number.column == 1
    end
  end

  describe "tokenize/2 — newlines" do
    test "emits NEWLINE tokens" do
      {:ok, tokens} = GrammarLexer.tokenize("x\ny", simple_grammar())
      types = Enum.map(tokens, & &1.type)
      assert types == ["NAME", "NEWLINE", "NAME", "EOF"]
    end
  end

  describe "tokenize/2 — skip patterns" do
    test "skip patterns consume whitespace silently" do
      {:ok, g} =
        TokenGrammar.parse("""
        NUMBER = /[0-9]+/
        PLUS = "+"

        skip:
          WHITESPACE = /[ \\t\\r\\n]+/
        """)

      {:ok, tokens} = GrammarLexer.tokenize("1 + 2\n", g)
      types = Enum.map(tokens, & &1.type)
      # With skip patterns consuming newlines, no NEWLINE token is emitted
      assert types == ["NUMBER", "PLUS", "NUMBER", "EOF"]
    end
  end

  describe "tokenize/2 — keywords" do
    test "reclassifies NAME as KEYWORD" do
      {:ok, g} =
        TokenGrammar.parse("""
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
        NUMBER = /[0-9]+/

        keywords:
          if
          else
        """)

      {:ok, tokens} = GrammarLexer.tokenize("if x else y", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["KEYWORD", "NAME", "KEYWORD", "NAME", "EOF"]
    end
  end

  describe "tokenize/2 — reserved keywords" do
    test "reserved keywords produce errors" do
      {:ok, g} =
        TokenGrammar.parse("""
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

        reserved:
          class
        """)

      {:error, msg} = GrammarLexer.tokenize("class", g)
      assert msg =~ "Reserved keyword 'class'"
    end
  end

  describe "tokenize/2 — aliases" do
    test "uses alias as token type" do
      {:ok, g} = TokenGrammar.parse(~s(STRING_DQ = /"[^"]*"/ -> STRING))
      {:ok, tokens} = GrammarLexer.tokenize(~s("hello"), g)
      [token, _eof] = tokens
      assert token.type == "STRING"
      assert token.value == "hello"
    end
  end

  describe "tokenize/2 — string escape processing" do
    test "processes standard escapes via JSON grammar" do
      g = json_grammar()
      source = ~S("hello\nworld")
      {:ok, tokens} = GrammarLexer.tokenize(source, g)
      [token, _eof] = tokens
      assert token.type == "STRING"
      assert token.value == "hello\nworld"
    end

    test "processes unicode escapes via JSON grammar" do
      g = json_grammar()
      source = ~S("caf\u00E9")
      {:ok, tokens} = GrammarLexer.tokenize(source, g)
      [token, _eof] = tokens
      assert token.value == "caf\u00E9"
    end
  end

  describe "tokenize/2 — error cases" do
    test "unexpected character" do
      {:error, msg} = GrammarLexer.tokenize("x @ y", simple_grammar())
      assert msg =~ "Unexpected character"
      assert msg =~ "@"
    end
  end

  describe "tokenize/2 — JSON grammar integration" do
    test "tokenizes JSON primitives" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("42", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["NUMBER", "EOF"]
    end

    test "tokenizes JSON boolean and null" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("true false null", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["TRUE", "FALSE", "NULL", "EOF"]
    end

    test "tokenizes JSON string" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize(~s("hello world"), g)
      [token, _eof] = tokens
      assert token.type == "STRING"
      assert token.value == "hello world"
    end

    test "tokenizes JSON structural tokens" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("{[]:,}", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["LBRACE", "LBRACKET", "RBRACKET", "COLON", "COMMA", "RBRACE", "EOF"]
    end

    test "tokenizes a JSON object" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize(~s({"key": 42}), g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE", "EOF"]
    end

    test "tokenizes JSON with whitespace" do
      g = json_grammar()

      {:ok, tokens} =
        GrammarLexer.tokenize("""
        {
          "name": "Alice",
          "age": 30
        }
        """, g)

      types = Enum.map(tokens, & &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "LBRACE",
               "STRING", "COLON", "STRING", "COMMA",
               "STRING", "COLON", "NUMBER",
               "RBRACE"
             ]
    end

    test "tokenizes negative numbers" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("-42", g)
      [token, _eof] = tokens
      assert token.type == "NUMBER"
      assert token.value == "-42"
    end

    test "tokenizes decimal and exponent numbers" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("3.14 1e10 2.5E-3", g)
      values = tokens |> Enum.reject(&(&1.type == "EOF")) |> Enum.map(& &1.value)
      assert values == ["3.14", "1e10", "2.5E-3"]
    end
  end

  describe "process_escapes/1" do
    test "handles \\n, \\t, \\r" do
      assert GrammarLexer.process_escapes("a\\nb\\tc\\rd") == "a\nb\tc\rd"
    end

    test "handles \\b, \\f" do
      assert GrammarLexer.process_escapes("a\\bb\\fc") == "a\bb\fc"
    end

    test "handles \\\\ and \\\"" do
      assert GrammarLexer.process_escapes("a\\\\b\\\"c") == "a\\b\"c"
    end

    test "handles \\/" do
      assert GrammarLexer.process_escapes("a\\/b") == "a/b"
    end

    test "handles \\uXXXX" do
      assert GrammarLexer.process_escapes("caf\\u00E9") == "caf\u00E9"
    end

    test "passes through unknown escapes" do
      assert GrammarLexer.process_escapes("\\x") == "x"
    end

    test "no escapes" do
      assert GrammarLexer.process_escapes("hello") == "hello"
    end
  end
end
