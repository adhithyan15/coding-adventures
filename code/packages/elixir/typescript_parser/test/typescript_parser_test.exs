defmodule CodingAdventures.TypescriptParserTest do
  use ExUnit.Case

  alias CodingAdventures.TypescriptParser

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(TypescriptParser)
  end

  # ---------------------------------------------------------------------------
  # parse/1 — generic (no version)
  # ---------------------------------------------------------------------------

  describe "parse/1 — generic grammar" do
    test "returns a map for empty string" do
      assert is_map(TypescriptParser.parse(""))
    end

    test "returns a map with rule_name key" do
      ast = TypescriptParser.parse("let x = 1;")
      assert Map.has_key?(ast, :rule_name)
    end
  end

  # ---------------------------------------------------------------------------
  # parse/2 — version-specific
  # ---------------------------------------------------------------------------

  describe "parse/2 — versioned grammar" do
    test "accepts nil version (generic grammar)" do
      assert is_map(TypescriptParser.parse("let x = 1;", nil))
    end

    test "accepts ts1.0 version" do
      assert is_map(TypescriptParser.parse("var x = 0;", "ts1.0"))
    end

    test "accepts ts2.0 version" do
      assert is_map(TypescriptParser.parse("let x = 0;", "ts2.0"))
    end

    test "accepts ts3.0 version" do
      assert is_map(TypescriptParser.parse("let x = 0;", "ts3.0"))
    end

    test "accepts ts4.0 version" do
      assert is_map(TypescriptParser.parse("let x = 0;", "ts4.0"))
    end

    test "accepts ts5.0 version" do
      assert is_map(TypescriptParser.parse("let x = 0;", "ts5.0"))
    end

    test "accepts ts5.8 version" do
      assert is_map(TypescriptParser.parse("let x = 1;", "ts5.8"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown TypeScript version "ts99\.0"/, fn ->
        TypescriptParser.parse("let x = 1;", "ts99.0")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown TypeScript version "latest"/, fn ->
        TypescriptParser.parse("let x = 1;", "latest")
      end
    end
  end
end
