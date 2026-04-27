defmodule CodingAdventures.CSharpParserTest do
  use ExUnit.Case

  alias CodingAdventures.CSharpParser

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(CSharpParser)
  end

  # ---------------------------------------------------------------------------
  # parse_csharp/1 -- generic (no version, defaults to C# 12.0)
  # ---------------------------------------------------------------------------

  describe "parse_csharp/1 -- default grammar" do
    test "returns a map for empty string" do
      assert is_map(CSharpParser.parse_csharp(""))
    end

    test "returns a map with rule_name key" do
      ast = CSharpParser.parse_csharp("int x = 1;")
      assert Map.has_key?(ast, :rule_name)
    end
  end

  # ---------------------------------------------------------------------------
  # parse_csharp/2 -- version-specific
  # ---------------------------------------------------------------------------

  describe "parse_csharp/2 -- versioned grammar" do
    test "accepts nil version (default grammar)" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", nil))
    end

    test "accepts 1.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "1.0"))
    end

    test "accepts 2.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "2.0"))
    end

    test "accepts 3.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "3.0"))
    end

    test "accepts 4.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "4.0"))
    end

    test "accepts 5.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "5.0"))
    end

    test "accepts 6.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "6.0"))
    end

    test "accepts 7.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "7.0"))
    end

    test "accepts 8.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "8.0"))
    end

    test "accepts 9.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "9.0"))
    end

    test "accepts 10.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "10.0"))
    end

    test "accepts 11.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "11.0"))
    end

    test "accepts 12.0 version" do
      assert is_map(CSharpParser.parse_csharp("int x = 1;", "12.0"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown C# version "99"/, fn ->
        CSharpParser.parse_csharp("int x = 1;", "99")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown C# version "latest"/, fn ->
        CSharpParser.parse_csharp("int x = 1;", "latest")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # create_csharp_parser/1 and create_csharp_parser/2
  # ---------------------------------------------------------------------------

  describe "create_csharp_parser/2" do
    test "returns a map" do
      parser = CSharpParser.create_csharp_parser("int x = 1;")
      assert is_map(parser)
    end

    test "stores source in returned map" do
      parser = CSharpParser.create_csharp_parser("int x = 1;")
      assert parser.source == "int x = 1;"
    end

    test "stores nil version when not specified" do
      parser = CSharpParser.create_csharp_parser("int x = 1;")
      assert parser.version == nil
    end

    test "stores version when 8.0 specified" do
      parser = CSharpParser.create_csharp_parser("int x = 1;", "8.0")
      assert parser.version == "8.0"
    end

    test "stores language as csharp" do
      parser = CSharpParser.create_csharp_parser("int x = 1;")
      assert parser.language == :csharp
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown C# version/, fn ->
        CSharpParser.create_csharp_parser("int x = 1;", "99")
      end
    end
  end
end
