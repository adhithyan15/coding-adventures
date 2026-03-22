defmodule CodingAdventures.TomlLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.TomlLexer

  # ---------------------------------------------------------------------------
  # Grammar loading
  # ---------------------------------------------------------------------------

  describe "create_lexer/0" do
    test "returns a TokenGrammar with TOML token definitions" do
      grammar = TomlLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "BARE_KEY" in names
      assert "BASIC_STRING" in names
      assert "LITERAL_STRING" in names
      assert "INTEGER" in names
      assert "TRUE" in names
      assert "FALSE" in names
      assert "EQUALS" in names
      assert "LBRACKET" in names
      assert "RBRACKET" in names
      assert "LBRACE" in names
      assert "RBRACE" in names
      assert "DOT" in names
      assert "COMMA" in names
    end

    test "grammar has escape_mode set to none" do
      grammar = TomlLexer.create_lexer()
      assert grammar.escape_mode == "none"
    end

    test "grammar has skip definitions for comments and whitespace" do
      grammar = TomlLexer.create_lexer()
      skip_names = Enum.map(grammar.skip_definitions, & &1.name)
      assert "COMMENT" in skip_names
      assert "WHITESPACE" in skip_names
    end
  end

  # ---------------------------------------------------------------------------
  # Key-value pair basics
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — key-value pairs" do
    test "bare key with string value" do
      {:ok, tokens} = TomlLexer.tokenize(~s(title = "TOML Example"))
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["BARE_KEY", "EQUALS", "BASIC_STRING"]
    end

    test "bare key with integer value" do
      {:ok, tokens} = TomlLexer.tokenize("port = 8080")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["BARE_KEY", "EQUALS", "INTEGER"]
    end

    test "bare key with boolean value" do
      {:ok, tokens} = TomlLexer.tokenize("enabled = true")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["BARE_KEY", "EQUALS", "TRUE"]
    end

    test "dotted key" do
      {:ok, tokens} = TomlLexer.tokenize(~s(server.host = "localhost"))
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["BARE_KEY", "DOT", "BARE_KEY", "EQUALS", "BASIC_STRING"]
    end
  end

  # ---------------------------------------------------------------------------
  # String types
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — strings" do
    test "basic string" do
      {:ok, tokens} = TomlLexer.tokenize(~s("hello world"))
      [str, _eof] = tokens
      assert str.type == "BASIC_STRING"
      assert str.value == ~s("hello world")
    end

    test "literal string" do
      {:ok, tokens} = TomlLexer.tokenize("'C:\\\\Users\\\\foo'")
      [str, _eof] = tokens
      assert str.type == "LITERAL_STRING"
      assert str.value == "'C:\\\\Users\\\\foo'"
    end

    test "multi-line basic string" do
      source = ~s(\"\"\"hello\nworld\"\"\")
      {:ok, tokens} = TomlLexer.tokenize(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert "ML_BASIC_STRING" in types
    end

    test "multi-line literal string" do
      source = "'''hello\nworld'''"
      {:ok, tokens} = TomlLexer.tokenize(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert "ML_LITERAL_STRING" in types
    end

    test "escape sequences are preserved (escapes: none)" do
      # With escapes: none, the lexer should NOT process \n into a newline
      # The raw value should retain the backslash-n literally
      {:ok, tokens} = TomlLexer.tokenize(~S(key = "hello\nworld"))
      str_token = Enum.find(tokens, &(&1.type == "BASIC_STRING"))
      # The full match including quotes is kept since it's not a STRING alias
      assert str_token.value == ~S("hello\nworld")
    end
  end

  # ---------------------------------------------------------------------------
  # Number types
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — integers" do
    test "decimal integer" do
      {:ok, tokens} = TomlLexer.tokenize("42")
      [num, _eof] = tokens
      assert num.type == "INTEGER"
      assert num.value == "42"
    end

    test "negative integer" do
      {:ok, tokens} = TomlLexer.tokenize("-17")
      [num, _eof] = tokens
      assert num.type == "INTEGER"
      assert num.value == "-17"
    end

    test "hex integer" do
      {:ok, tokens} = TomlLexer.tokenize("0xDEADBEEF")
      [num, _eof] = tokens
      assert num.type == "INTEGER"
      assert num.value == "0xDEADBEEF"
    end

    test "octal integer" do
      {:ok, tokens} = TomlLexer.tokenize("0o755")
      [num, _eof] = tokens
      assert num.type == "INTEGER"
      assert num.value == "0o755"
    end

    test "binary integer" do
      {:ok, tokens} = TomlLexer.tokenize("0b11010110")
      [num, _eof] = tokens
      assert num.type == "INTEGER"
      assert num.value == "0b11010110"
    end

    test "integer with underscores" do
      {:ok, tokens} = TomlLexer.tokenize("1_000_000")
      [num, _eof] = tokens
      assert num.type == "INTEGER"
      assert num.value == "1_000_000"
    end
  end

  describe "tokenize/1 — floats" do
    test "decimal float" do
      {:ok, tokens} = TomlLexer.tokenize("3.14")
      [num, _eof] = tokens
      assert num.type == "FLOAT"
      assert num.value == "3.14"
    end

    test "scientific notation" do
      {:ok, tokens} = TomlLexer.tokenize("5e+22")
      [num, _eof] = tokens
      assert num.type == "FLOAT"
      assert num.value == "5e+22"
    end

    test "special float inf" do
      {:ok, tokens} = TomlLexer.tokenize("inf")
      [num, _eof] = tokens
      assert num.type == "FLOAT"
      assert num.value == "inf"
    end

    test "special float nan" do
      {:ok, tokens} = TomlLexer.tokenize("nan")
      [num, _eof] = tokens
      assert num.type == "FLOAT"
      assert num.value == "nan"
    end

    test "negative inf" do
      {:ok, tokens} = TomlLexer.tokenize("-inf")
      [num, _eof] = tokens
      assert num.type == "FLOAT"
      assert num.value == "-inf"
    end
  end

  # ---------------------------------------------------------------------------
  # Booleans
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — booleans" do
    test "true" do
      {:ok, tokens} = TomlLexer.tokenize("true")
      [tok, _eof] = tokens
      assert tok.type == "TRUE"
      assert tok.value == "true"
    end

    test "false" do
      {:ok, tokens} = TomlLexer.tokenize("false")
      [tok, _eof] = tokens
      assert tok.type == "FALSE"
      assert tok.value == "false"
    end
  end

  # ---------------------------------------------------------------------------
  # Date/time types
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — date/time" do
    test "offset datetime" do
      {:ok, tokens} = TomlLexer.tokenize("1979-05-27T07:32:00Z")
      [dt, _eof] = tokens
      assert dt.type == "OFFSET_DATETIME"
      assert dt.value == "1979-05-27T07:32:00Z"
    end

    test "offset datetime with offset" do
      {:ok, tokens} = TomlLexer.tokenize("1979-05-27T07:32:00+09:00")
      [dt, _eof] = tokens
      assert dt.type == "OFFSET_DATETIME"
    end

    test "local datetime" do
      {:ok, tokens} = TomlLexer.tokenize("1979-05-27T07:32:00")
      [dt, _eof] = tokens
      assert dt.type == "LOCAL_DATETIME"
    end

    test "local date" do
      {:ok, tokens} = TomlLexer.tokenize("1979-05-27")
      [dt, _eof] = tokens
      assert dt.type == "LOCAL_DATE"
      assert dt.value == "1979-05-27"
    end

    test "local time" do
      {:ok, tokens} = TomlLexer.tokenize("07:32:00")
      [dt, _eof] = tokens
      assert dt.type == "LOCAL_TIME"
      assert dt.value == "07:32:00"
    end

    test "local time with fractional seconds" do
      {:ok, tokens} = TomlLexer.tokenize("07:32:00.999")
      [dt, _eof] = tokens
      assert dt.type == "LOCAL_TIME"
    end
  end

  # ---------------------------------------------------------------------------
  # Structural tokens
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — delimiters" do
    test "all delimiters" do
      {:ok, tokens} = TomlLexer.tokenize("=.,[]{}")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["EQUALS", "DOT", "COMMA", "LBRACKET", "RBRACKET", "LBRACE", "RBRACE"]
    end
  end

  # ---------------------------------------------------------------------------
  # Table headers
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — table headers" do
    test "simple table header" do
      {:ok, tokens} = TomlLexer.tokenize("[server]")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["LBRACKET", "BARE_KEY", "RBRACKET"]
    end

    test "array-of-tables header" do
      {:ok, tokens} = TomlLexer.tokenize("[[products]]")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["LBRACKET", "LBRACKET", "BARE_KEY", "RBRACKET", "RBRACKET"]
    end
  end

  # ---------------------------------------------------------------------------
  # Comments and whitespace
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — comments and whitespace" do
    test "skips comments" do
      {:ok, tokens} = TomlLexer.tokenize("# This is a comment\nkey = 42")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 in ["EOF", "NEWLINE"]))
      assert types == ["BARE_KEY", "EQUALS", "INTEGER"]
    end

    test "skips inline comments" do
      {:ok, tokens} = TomlLexer.tokenize("key = 42 # inline comment")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["BARE_KEY", "EQUALS", "INTEGER"]
    end

    test "emits NEWLINE tokens" do
      {:ok, tokens} = TomlLexer.tokenize("a = 1\nb = 2")
      types = Enum.map(tokens, & &1.type)
      assert "NEWLINE" in types
    end
  end

  # ---------------------------------------------------------------------------
  # Multi-line TOML document
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — compound document" do
    test "tokenizes a realistic TOML document" do
      source = """
      [server]
      host = "localhost"
      port = 8080
      enabled = true
      """

      {:ok, tokens} = TomlLexer.tokenize(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 in ["EOF", "NEWLINE"]))

      assert types == [
               "LBRACKET", "BARE_KEY", "RBRACKET",
               "BARE_KEY", "EQUALS", "BASIC_STRING",
               "BARE_KEY", "EQUALS", "INTEGER",
               "BARE_KEY", "EQUALS", "TRUE"
             ]
    end

    test "tokenizes inline table" do
      {:ok, tokens} = TomlLexer.tokenize("point = { x = 1, y = 2 }")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "BARE_KEY", "EQUALS",
               "LBRACE", "BARE_KEY", "EQUALS", "INTEGER",
               "COMMA", "BARE_KEY", "EQUALS", "INTEGER", "RBRACE"
             ]
    end

    test "tokenizes array" do
      {:ok, tokens} = TomlLexer.tokenize(~s(colors = ["red", "green", "blue"]))
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "BARE_KEY", "EQUALS",
               "LBRACKET", "BASIC_STRING", "COMMA",
               "BASIC_STRING", "COMMA", "BASIC_STRING", "RBRACKET"
             ]
    end
  end

  # ---------------------------------------------------------------------------
  # Position tracking
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — position tracking" do
    test "tracks line and column" do
      {:ok, tokens} = TomlLexer.tokenize("[server]")
      [lbracket | _] = tokens
      assert lbracket.line == 1
      assert lbracket.column == 1
    end

    test "tracks position across lines" do
      {:ok, tokens} = TomlLexer.tokenize("a = 1\nb = 2")
      b_token = Enum.find(tokens, &(&1.value == "b"))
      assert b_token.line == 2
      assert b_token.column == 1
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — error cases" do
    test "errors on unexpected character" do
      {:error, msg} = TomlLexer.tokenize("@")
      assert msg =~ "Unexpected character"
    end
  end
end
