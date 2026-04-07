defmodule CodingAdventures.JavascriptParserTest do
  use ExUnit.Case

  alias CodingAdventures.JavascriptParser

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(JavascriptParser)
  end

  # ---------------------------------------------------------------------------
  # parse/1 — generic (no version)
  # ---------------------------------------------------------------------------

  describe "parse/1 — generic grammar" do
    test "returns a map for empty string" do
      assert is_map(JavascriptParser.parse(""))
    end

    test "returns a map with rule_name key" do
      ast = JavascriptParser.parse("let x = 1;")
      assert Map.has_key?(ast, :rule_name)
    end
  end

  # ---------------------------------------------------------------------------
  # parse/2 — version-specific
  # ---------------------------------------------------------------------------

  describe "parse/2 — versioned grammar" do
    test "accepts nil version (generic grammar)" do
      assert is_map(JavascriptParser.parse("let x = 1;", nil))
    end

    test "accepts es1 version" do
      assert is_map(JavascriptParser.parse("var x = 1;", "es1"))
    end

    test "accepts es3 version" do
      assert is_map(JavascriptParser.parse("var x = 1;", "es3"))
    end

    test "accepts es5 version" do
      assert is_map(JavascriptParser.parse("var x = 1;", "es5"))
    end

    test "accepts es2015 version" do
      assert is_map(JavascriptParser.parse("let x = 1;", "es2015"))
    end

    test "accepts es2020 version" do
      assert is_map(JavascriptParser.parse("const x = 1;", "es2020"))
    end

    test "accepts es2025 version" do
      assert is_map(JavascriptParser.parse("let x = 1;", "es2025"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown JavaScript\/ECMAScript version "es2099"/, fn ->
        JavascriptParser.parse("let x = 1;", "es2099")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown JavaScript\/ECMAScript version "latest"/, fn ->
        JavascriptParser.parse("let x = 1;", "latest")
      end
    end
  end
end
