defmodule CodingAdventures.VerilogLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.VerilogLexer

  # ===========================================================================
  # Grammar Loading
  # ===========================================================================

  describe "create_lexer/0" do
    test "returns a TokenGrammar with expected token definitions" do
      grammar = VerilogLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      # Core token types from verilog.tokens
      assert "NAME" in names
      assert "NUMBER" in names
      assert "SIZED_NUMBER" in names
      assert "REAL_NUMBER" in names
      assert "STRING" in names
      assert "SYSTEM_ID" in names
      assert "DIRECTIVE" in names
      assert "SEMICOLON" in names
      assert "LPAREN" in names
      assert "RPAREN" in names
    end

    test "includes keyword definitions" do
      grammar = VerilogLexer.create_lexer()
      assert length(grammar.keywords) > 0
      assert "module" in grammar.keywords
      assert "endmodule" in grammar.keywords
      assert "wire" in grammar.keywords
      assert "reg" in grammar.keywords
    end

    test "supports selecting an explicit language edition" do
      default_names =
        VerilogLexer.create_lexer()
        |> Map.fetch!(:definitions)
        |> Enum.map(& &1.name)

      versioned_names =
        VerilogLexer.create_lexer("2005")
        |> Map.fetch!(:definitions)
        |> Enum.map(& &1.name)

      assert default_names == versioned_names
    end

    test "raises for an unknown language edition" do
      assert_raise ArgumentError, ~r/Unknown Verilog version/, fn ->
        VerilogLexer.create_lexer("2099")
      end
    end
  end

  # ===========================================================================
  # Keywords
  # ===========================================================================

  describe "tokenize/1 — keywords" do
    test "recognizes module and endmodule as KEYWORD tokens" do
      {:ok, tokens} = VerilogLexer.tokenize("module endmodule")
      [mod, endmod, _eof] = tokens
      assert mod.type == "KEYWORD"
      assert mod.value == "module"
      assert endmod.type == "KEYWORD"
      assert endmod.value == "endmodule"
    end

    test "recognizes data type keywords" do
      {:ok, tokens} = VerilogLexer.tokenize("wire reg integer")
      types_and_values =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert {"KEYWORD", "wire"} in types_and_values
      assert {"KEYWORD", "reg"} in types_and_values
      assert {"KEYWORD", "integer"} in types_and_values
    end

    test "recognizes procedural keywords" do
      {:ok, tokens} = VerilogLexer.tokenize("always begin end if else case endcase default for")
      keyword_values =
        tokens
        |> Enum.filter(&(&1.type == "KEYWORD"))
        |> Enum.map(& &1.value)

      assert "always" in keyword_values
      assert "begin" in keyword_values
      assert "end" in keyword_values
      assert "if" in keyword_values
      assert "else" in keyword_values
      assert "case" in keyword_values
    end

    test "recognizes sensitivity keywords" do
      {:ok, tokens} = VerilogLexer.tokenize("posedge negedge")
      [pos, neg, _eof] = tokens
      assert pos.type == "KEYWORD"
      assert pos.value == "posedge"
      assert neg.type == "KEYWORD"
      assert neg.value == "negedge"
    end
  end

  # ===========================================================================
  # Number Literals
  # ===========================================================================

  describe "tokenize/1 — number literals" do
    test "tokenizes plain integers" do
      {:ok, tokens} = VerilogLexer.tokenize("42")
      [num, _eof] = tokens
      assert num.type == "NUMBER"
      assert num.value == "42"
    end

    test "tokenizes integers with underscores" do
      {:ok, tokens} = VerilogLexer.tokenize("1_000_000")
      [num, _eof] = tokens
      assert num.type == "NUMBER"
      assert num.value == "1_000_000"
    end

    test "tokenizes sized binary numbers" do
      {:ok, tokens} = VerilogLexer.tokenize("4'b1010")
      [num, _eof] = tokens
      assert num.type == "SIZED_NUMBER"
      assert num.value == "4'b1010"
    end

    test "tokenizes sized hex numbers" do
      {:ok, tokens} = VerilogLexer.tokenize("8'hFF")
      [num, _eof] = tokens
      assert num.type == "SIZED_NUMBER"
      assert num.value == "8'hFF"
    end

    test "tokenizes sized decimal numbers" do
      {:ok, tokens} = VerilogLexer.tokenize("32'd42")
      [num, _eof] = tokens
      assert num.type == "SIZED_NUMBER"
      assert num.value == "32'd42"
    end

    test "tokenizes unsized based numbers" do
      {:ok, tokens} = VerilogLexer.tokenize("'o77")
      [num, _eof] = tokens
      assert num.type == "SIZED_NUMBER"
      assert num.value == "'o77"
    end

    test "tokenizes sized numbers with x and z" do
      {:ok, tokens} = VerilogLexer.tokenize("4'bxxzz")
      [num, _eof] = tokens
      assert num.type == "SIZED_NUMBER"
      assert num.value == "4'bxxzz"
    end

    test "tokenizes real numbers" do
      {:ok, tokens} = VerilogLexer.tokenize("3.14")
      [num, _eof] = tokens
      assert num.type == "REAL_NUMBER"
      assert num.value == "3.14"
    end

    test "tokenizes real numbers with exponent" do
      {:ok, tokens} = VerilogLexer.tokenize("1.5e10")
      [num, _eof] = tokens
      assert num.type == "REAL_NUMBER"
      assert num.value == "1.5e10"
    end
  end

  # ===========================================================================
  # String Literals
  # ===========================================================================

  describe "tokenize/1 — strings" do
    test "tokenizes a string" do
      {:ok, tokens} = VerilogLexer.tokenize(~s("hello"))
      [str, _eof] = tokens
      assert str.type == "STRING"
    end

    test "tokenizes a string with escapes" do
      {:ok, tokens} = VerilogLexer.tokenize(~S("hello\nworld"))
      [str, _eof] = tokens
      assert str.type == "STRING"
    end
  end

  # ===========================================================================
  # Special Identifiers
  # ===========================================================================

  describe "tokenize/1 — special identifiers" do
    test "tokenizes system task identifiers" do
      {:ok, tokens} = VerilogLexer.tokenize("$display")
      [sys, _eof] = tokens
      assert sys.type == "SYSTEM_ID"
      assert sys.value == "$display"
    end

    test "tokenizes compiler directives" do
      {:ok, tokens} = VerilogLexer.tokenize("`timescale")
      [dir, _eof] = tokens
      assert dir.type == "DIRECTIVE"
      assert dir.value == "`timescale"
    end

    test "tokenizes escaped identifiers" do
      {:ok, tokens} = VerilogLexer.tokenize("\\my.odd.name ")
      escaped =
        tokens
        |> Enum.find(&(&1.type == "ESCAPED_IDENT"))

      assert escaped != nil
      assert escaped.value == "\\my.odd.name"
    end

    test "tokenizes regular identifiers" do
      {:ok, tokens} = VerilogLexer.tokenize("my_signal")
      [name, _eof] = tokens
      assert name.type == "NAME"
      assert name.value == "my_signal"
    end
  end

  # ===========================================================================
  # Operators
  # ===========================================================================

  describe "tokenize/1 — operators" do
    test "tokenizes three-character operators" do
      {:ok, tokens} = VerilogLexer.tokenize("<<< >>> === !==")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert types == ["ARITH_LEFT_SHIFT", "ARITH_RIGHT_SHIFT", "CASE_EQ", "CASE_NEQ"]
    end

    test "tokenizes two-character operators" do
      {:ok, tokens} = VerilogLexer.tokenize("<< >> == != <= >= ** && || ->")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert "LEFT_SHIFT" in types
      assert "RIGHT_SHIFT" in types
      assert "EQUALS_EQUALS" in types
      assert "NOT_EQUALS" in types
      assert "LESS_EQUALS" in types
      assert "GREATER_EQUALS" in types
      assert "POWER" in types
      assert "LOGIC_AND" in types
      assert "LOGIC_OR" in types
      assert "TRIGGER" in types
    end

    test "tokenizes single-character operators" do
      {:ok, tokens} = VerilogLexer.tokenize("+ - * / % & | ^ ~ ! < > = ? :")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert "PLUS" in types
      assert "MINUS" in types
      assert "STAR" in types
      assert "SLASH" in types
      assert "PERCENT" in types
      assert "AMP" in types
      assert "PIPE" in types
      assert "CARET" in types
      assert "TILDE" in types
      assert "BANG" in types
      assert "LESS_THAN" in types
      assert "GREATER_THAN" in types
      assert "EQUALS" in types
      assert "QUESTION" in types
      assert "COLON" in types
    end
  end

  # ===========================================================================
  # Delimiters
  # ===========================================================================

  describe "tokenize/1 — delimiters" do
    test "tokenizes all delimiters" do
      {:ok, tokens} = VerilogLexer.tokenize("( ) [ ] { } ; , . # @")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert types == [
               "LPAREN", "RPAREN", "LBRACKET", "RBRACKET",
               "LBRACE", "RBRACE", "SEMICOLON", "COMMA",
               "DOT", "HASH", "AT"
             ]
    end
  end

  # ===========================================================================
  # Whitespace and Comments
  # ===========================================================================

  describe "tokenize/1 — whitespace and comments" do
    test "skips whitespace" do
      {:ok, tokens} = VerilogLexer.tokenize("  module   foo  ;  ")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert types == ["KEYWORD", "NAME", "SEMICOLON"]
    end

    test "skips line comments" do
      {:ok, tokens} = VerilogLexer.tokenize("module // this is a comment\nfoo")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert "KEYWORD" in types
      assert "NAME" in types
    end

    test "skips block comments" do
      {:ok, tokens} = VerilogLexer.tokenize("module /* block comment */ foo")
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert types == ["KEYWORD", "NAME"]
    end
  end

  # ===========================================================================
  # Compound Structures
  # ===========================================================================

  describe "tokenize/1 — compound structures" do
    test "tokenizes a simple module declaration" do
      source = "module counter(input clk, output reg [7:0] count); endmodule"
      {:ok, tokens} = VerilogLexer.tokenize(source)
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      # module counter ( input clk , output reg [ 7 : 0 ] count ) ; endmodule
      assert List.first(types) == "KEYWORD"
      assert List.last(types) == "KEYWORD"
      assert "LPAREN" in types
      assert "RPAREN" in types
      assert "LBRACKET" in types
      assert "RBRACKET" in types
      assert "SEMICOLON" in types
    end

    test "tokenizes an always block with sensitivity list" do
      source = "always @(posedge clk) begin q <= d; end"
      {:ok, tokens} = VerilogLexer.tokenize(source)
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert List.first(types) == "KEYWORD"
      assert "AT" in types
      assert "LESS_EQUALS" in types
    end

    test "tokenizes an assign statement" do
      source = "assign out = a & b | c ^ d;"
      {:ok, tokens} = VerilogLexer.tokenize(source)
      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      assert List.first(types) == "KEYWORD"
      assert "EQUALS" in types
      assert "AMP" in types
      assert "PIPE" in types
      assert "CARET" in types
    end
  end

  # ===========================================================================
  # Position Tracking
  # ===========================================================================

  describe "tokenize/1 — position tracking" do
    test "tracks line and column of first token" do
      {:ok, tokens} = VerilogLexer.tokenize("module foo;")
      [first | _] = tokens
      assert first.line == 1
      assert first.column == 1
    end
  end

  # ===========================================================================
  # Error Cases
  # ===========================================================================

  describe "tokenize/1 — errors" do
    test "errors on unexpected character" do
      # The backtick alone (without a following identifier) may cause an error
      # Use a truly unexpected character
      {:error, msg} = VerilogLexer.tokenize("\x01")
      assert msg =~ "Unexpected character"
    end
  end

  # ===========================================================================
  # Preprocessing Integration
  # ===========================================================================

  describe "tokenize/2 — with preprocess option" do
    test "expands simple macros when preprocess: true" do
      source = "`define WIDTH 8\nwire [`WIDTH-1:0] bus;"
      {:ok, tokens} = VerilogLexer.tokenize(source, preprocess: true)

      values =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.value)

      # After preprocessing: "wire [8-1:0] bus;"
      # The `define line becomes empty, `WIDTH is replaced with 8
      assert "8" in values
      assert "bus" in values
    end

    test "does not preprocess when option is false (default)" do
      source = "`define WIDTH 8"
      {:ok, tokens} = VerilogLexer.tokenize(source)

      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      # Without preprocessing, `define is a DIRECTIVE token
      assert "DIRECTIVE" in types
    end

    test "handles ifdef conditional compilation" do
      source = """
      `define DEBUG
      `ifdef DEBUG
      wire debug_wire;
      `else
      wire release_wire;
      `endif
      """

      {:ok, tokens} = VerilogLexer.tokenize(source, preprocess: true)
      values =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.value)

      assert "debug_wire" in values
      refute "release_wire" in values
    end

    test "handles timescale stripping" do
      source = "`timescale 1ns/1ps\nmodule top; endmodule"
      {:ok, tokens} = VerilogLexer.tokenize(source, preprocess: true)

      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      # timescale is stripped, only module tokens remain
      refute "DIRECTIVE" in types
      assert "KEYWORD" in types
    end
  end
end
