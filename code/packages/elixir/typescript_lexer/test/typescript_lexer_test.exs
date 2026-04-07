defmodule CodingAdventures.TypescriptLexerTest do
  use ExUnit.Case

  alias CodingAdventures.TypescriptLexer

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(TypescriptLexer)
  end

  # ---------------------------------------------------------------------------
  # tokenize/1 — generic (no version)
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — generic grammar" do
    test "returns a list for empty string" do
      assert is_list(TypescriptLexer.tokenize(""))
    end

    test "returns a list for simple source" do
      assert is_list(TypescriptLexer.tokenize("let x = 1;"))
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize/2 — version-specific
  # ---------------------------------------------------------------------------

  describe "tokenize/2 — versioned grammar" do
    test "accepts nil version (generic grammar)" do
      assert is_list(TypescriptLexer.tokenize("let x = 1;", nil))
    end

    test "accepts ts1.0 version" do
      assert is_list(TypescriptLexer.tokenize("var x = 0;", "ts1.0"))
    end

    test "accepts ts2.0 version" do
      assert is_list(TypescriptLexer.tokenize("let x = 0;", "ts2.0"))
    end

    test "accepts ts3.0 version" do
      assert is_list(TypescriptLexer.tokenize("let x = 0;", "ts3.0"))
    end

    test "accepts ts4.0 version" do
      assert is_list(TypescriptLexer.tokenize("let x = 0;", "ts4.0"))
    end

    test "accepts ts5.0 version" do
      assert is_list(TypescriptLexer.tokenize("let x = 0;", "ts5.0"))
    end

    test "accepts ts5.8 version" do
      assert is_list(TypescriptLexer.tokenize("let x = 1;", "ts5.8"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown TypeScript version "ts99\.0"/, fn ->
        TypescriptLexer.tokenize("let x = 1;", "ts99.0")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown TypeScript version "latest"/, fn ->
        TypescriptLexer.tokenize("let x = 1;", "latest")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # create_lexer/1 and create_lexer/2
  # ---------------------------------------------------------------------------

  describe "create_lexer/2" do
    test "returns a map" do
      lexer = TypescriptLexer.create_lexer("let x = 1;")
      assert is_map(lexer)
    end

    test "stores source in returned map" do
      lexer = TypescriptLexer.create_lexer("let x = 1;")
      assert lexer.source == "let x = 1;"
    end

    test "stores nil version when not specified" do
      lexer = TypescriptLexer.create_lexer("let x = 1;")
      assert lexer.version == nil
    end

    test "stores version when ts5.8 specified" do
      lexer = TypescriptLexer.create_lexer("let x = 1;", "ts5.8")
      assert lexer.version == "ts5.8"
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown TypeScript version/, fn ->
        TypescriptLexer.create_lexer("let x = 1;", "ts0.1")
      end
    end
  end
end
