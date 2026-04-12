defmodule CodingAdventures.JavaParserTest do
  use ExUnit.Case

  alias CodingAdventures.JavaParser

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(JavaParser)
  end

  # ---------------------------------------------------------------------------
  # parse/1 -- generic (no version, defaults to Java 21)
  # ---------------------------------------------------------------------------

  describe "parse/1 -- default grammar" do
    test "returns a map for empty string" do
      assert is_map(JavaParser.parse(""))
    end

    test "returns a map with rule_name key" do
      ast = JavaParser.parse("int x = 1;")
      assert Map.has_key?(ast, :rule_name)
    end
  end

  # ---------------------------------------------------------------------------
  # parse/2 -- version-specific
  # ---------------------------------------------------------------------------

  describe "parse/2 -- versioned grammar" do
    test "accepts nil version (default grammar)" do
      assert is_map(JavaParser.parse("int x = 1;", nil))
    end

    test "accepts 1.0 version" do
      assert is_map(JavaParser.parse("int x = 1;", "1.0"))
    end

    test "accepts 1.1 version" do
      assert is_map(JavaParser.parse("int x = 1;", "1.1"))
    end

    test "accepts 1.4 version" do
      assert is_map(JavaParser.parse("int x = 1;", "1.4"))
    end

    test "accepts 5 version" do
      assert is_map(JavaParser.parse("int x = 1;", "5"))
    end

    test "accepts 7 version" do
      assert is_map(JavaParser.parse("int x = 1;", "7"))
    end

    test "accepts 8 version" do
      assert is_map(JavaParser.parse("int x = 1;", "8"))
    end

    test "accepts 10 version" do
      assert is_map(JavaParser.parse("int x = 1;", "10"))
    end

    test "accepts 14 version" do
      assert is_map(JavaParser.parse("int x = 1;", "14"))
    end

    test "accepts 17 version" do
      assert is_map(JavaParser.parse("int x = 1;", "17"))
    end

    test "accepts 21 version" do
      assert is_map(JavaParser.parse("int x = 1;", "21"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version "99"/, fn ->
        JavaParser.parse("int x = 1;", "99")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown Java version "latest"/, fn ->
        JavaParser.parse("int x = 1;", "latest")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # create_parser/1 and create_parser/2
  # ---------------------------------------------------------------------------

  describe "create_parser/2" do
    test "returns a map" do
      parser = JavaParser.create_parser("int x = 1;")
      assert is_map(parser)
    end

    test "stores source in returned map" do
      parser = JavaParser.create_parser("int x = 1;")
      assert parser.source == "int x = 1;"
    end

    test "stores nil version when not specified" do
      parser = JavaParser.create_parser("int x = 1;")
      assert parser.version == nil
    end

    test "stores version when 8 specified" do
      parser = JavaParser.create_parser("int x = 1;", "8")
      assert parser.version == "8"
    end

    test "stores language as java" do
      parser = JavaParser.create_parser("int x = 1;")
      assert parser.language == :java
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version/, fn ->
        JavaParser.create_parser("int x = 1;", "99")
      end
    end
  end
end
