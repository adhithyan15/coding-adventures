defmodule CodingAdventures.JavaLexerTest do
  use ExUnit.Case

  alias CodingAdventures.JavaLexer

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(JavaLexer)
  end

  # ---------------------------------------------------------------------------
  # tokenize/1 -- generic (no version, defaults to Java 21)
  # ---------------------------------------------------------------------------

  describe "tokenize/1 -- default grammar" do
    test "returns a list for empty string" do
      assert is_list(JavaLexer.tokenize(""))
    end

    test "returns a list for simple source" do
      assert is_list(JavaLexer.tokenize("int x = 1;"))
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize/2 -- version-specific
  # ---------------------------------------------------------------------------

  describe "tokenize/2 -- versioned grammar" do
    test "accepts nil version (default grammar)" do
      assert is_list(JavaLexer.tokenize("int x = 1;", nil))
    end

    test "accepts 1.0 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "1.0"))
    end

    test "accepts 1.1 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "1.1"))
    end

    test "accepts 1.4 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "1.4"))
    end

    test "accepts 5 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "5"))
    end

    test "accepts 7 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "7"))
    end

    test "accepts 8 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "8"))
    end

    test "accepts 10 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "10"))
    end

    test "accepts 14 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "14"))
    end

    test "accepts 17 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "17"))
    end

    test "accepts 21 version" do
      assert is_list(JavaLexer.tokenize("int x = 1;", "21"))
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version "99"/, fn ->
        JavaLexer.tokenize("int x = 1;", "99")
      end
    end

    test "raises ArgumentError for completely invalid version string" do
      assert_raise ArgumentError, ~r/Unknown Java version "latest"/, fn ->
        JavaLexer.tokenize("int x = 1;", "latest")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # create_lexer/1 and create_lexer/2
  # ---------------------------------------------------------------------------

  describe "create_lexer/2" do
    test "returns a map" do
      lexer = JavaLexer.create_lexer("int x = 1;")
      assert is_map(lexer)
    end

    test "stores source in returned map" do
      lexer = JavaLexer.create_lexer("int x = 1;")
      assert lexer.source == "int x = 1;"
    end

    test "stores nil version when not specified" do
      lexer = JavaLexer.create_lexer("int x = 1;")
      assert lexer.version == nil
    end

    test "stores version when 8 specified" do
      lexer = JavaLexer.create_lexer("int x = 1;", "8")
      assert lexer.version == "8"
    end

    test "stores language as java" do
      lexer = JavaLexer.create_lexer("int x = 1;")
      assert lexer.language == :java
    end

    test "raises ArgumentError for unknown version" do
      assert_raise ArgumentError, ~r/Unknown Java version/, fn ->
        JavaLexer.create_lexer("int x = 1;", "99")
      end
    end
  end
end
