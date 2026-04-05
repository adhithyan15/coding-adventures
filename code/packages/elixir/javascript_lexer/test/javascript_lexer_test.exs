defmodule CodingAdventures.JavascriptLexerTest do
  use ExUnit.Case

  alias CodingAdventures.JavascriptLexer

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(JavascriptLexer)
  end

  # ---------------------------------------------------------------------------
  # tokenize/1 — generic (no version)
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — generic grammar" do
    test "returns a list for empty string" do
      assert is_list(JavascriptLexer.tokenize(""))
    end

    test "returns a list for simple source" do
      assert is_list(JavascriptLexer.tokenize("let x = 1;"))
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize/2 — version-specific
  # ---------------------------------------------------------------------------

  describe "tokenize/2 — versioned grammar" do
    test "accepts nil version (generic grammar)" do
      assert is_list(JavascriptLexer.tokenize("let x = 1;", nil))
    end

    test "accepts es1 version" do
      assert is_list(JavascriptLexer.tokenize("var x = 1;", "es1"))
    end

    test "accepts es3 version" do
      assert is_list(JavascriptLexer.tokenize("var x = 1;", "es3"))
    end

    test "accepts es5 version" do
      assert is_list(JavascriptLexer.tokenize("var x = 1;", "es5"))
    end

    test "accepts es2015 version" do
      assert is_list(JavascriptLexer.tokenize("let x = 1;", "es2015"))
    end

    test "accepts es2020 version" do
      assert is_list(JavascriptLexer.tokenize("const x = 1;", "es2020"))
    end

    test "accepts es2025 version" do
      assert is_list(JavascriptLexer.tokenize("let x = 1;", "es2025"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown JavaScript\/ECMAScript version "es2099"/, fn ->
        JavascriptLexer.tokenize("let x = 1;", "es2099")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown JavaScript\/ECMAScript version "latest"/, fn ->
        JavascriptLexer.tokenize("let x = 1;", "latest")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # create_lexer/1 and create_lexer/2
  # ---------------------------------------------------------------------------

  describe "create_lexer/2" do
    test "returns a map" do
      lexer = JavascriptLexer.create_lexer("let x = 1;")
      assert is_map(lexer)
    end

    test "stores source in returned map" do
      lexer = JavascriptLexer.create_lexer("let x = 1;")
      assert lexer.source == "let x = 1;"
    end

    test "stores nil version when not specified" do
      lexer = JavascriptLexer.create_lexer("let x = 1;")
      assert lexer.version == nil
    end

    test "stores version when es5 specified" do
      lexer = JavascriptLexer.create_lexer("var x = 1;", "es5")
      assert lexer.version == "es5"
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown JavaScript\/ECMAScript version/, fn ->
        JavascriptLexer.create_lexer("let x = 1;", "es99")
      end
    end
  end
end
