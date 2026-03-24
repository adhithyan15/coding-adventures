defmodule CodingAdventures.LatticeLexerTest do
  use ExUnit.Case

  alias CodingAdventures.LatticeLexer

  # ---------------------------------------------------------------------------
  # Helper: tokenize and extract types
  # ---------------------------------------------------------------------------

  # Tokenize and return a list of {type, value} tuples (excluding EOF)
  defp tokenize!(source) do
    {:ok, tokens} = LatticeLexer.tokenize(source)
    tokens
    |> Enum.reject(fn t -> t.type == "EOF" end)
    |> Enum.map(fn t -> {t.type, t.value} end)
  end

  # Return just the token types (excluding EOF)
  defp types!(source) do
    {:ok, tokens} = LatticeLexer.tokenize(source)
    tokens
    |> Enum.reject(fn t -> t.type == "EOF" end)
    |> Enum.map(fn t -> t.type end)
  end

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  describe "module loading" do
    test "module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeLexer)
    end

    test "create_lexer/0 returns a TokenGrammar" do
      grammar = LatticeLexer.create_lexer()
      # Grammar must have definitions (token patterns)
      assert is_list(grammar.definitions)
      assert length(grammar.definitions) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # EOF
  # ---------------------------------------------------------------------------

  describe "EOF token" do
    test "empty source produces only EOF" do
      {:ok, tokens} = LatticeLexer.tokenize("")
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end

    test "whitespace-only source produces only EOF" do
      {:ok, tokens} = LatticeLexer.tokenize("   \t\n  ")
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice-specific tokens (the 5 new ones)
  # ---------------------------------------------------------------------------

  describe "VARIABLE token (Lattice extension)" do
    test "simple variable" do
      assert tokenize!("$color") == [{"VARIABLE", "$color"}]
    end

    test "variable with hyphens" do
      assert tokenize!("$font-size") == [{"VARIABLE", "$font-size"}]
    end

    test "variable with underscore" do
      assert tokenize!("$base_margin") == [{"VARIABLE", "$base_margin"}]
    end

    test "variable with numbers in name" do
      assert tokenize!("$h1-size") == [{"VARIABLE", "$h1-size"}]
    end

    test "multiple variables" do
      result = tokenize!("$a $b $c")
      types = Enum.map(result, fn {type, _} -> type end)
      assert types == ["VARIABLE", "VARIABLE", "VARIABLE"]
    end

    test "variable in declaration context" do
      result = tokenize!("$color: red;")
      assert result == [
        {"VARIABLE", "$color"},
        {"COLON", ":"},
        {"IDENT", "red"},
        {"SEMICOLON", ";"}
      ]
    end
  end

  describe "EQUALS_EQUALS token (Lattice extension)" do
    test "double equals" do
      assert tokenize!("==") == [{"EQUALS_EQUALS", "=="}]
    end

    test "double equals in @if condition" do
      result = tokenize!("$theme == dark")
      assert {"EQUALS_EQUALS", "=="} in result
    end

    test "double equals not confused with single equals" do
      result = tokenize!("= ==")
      types = Enum.map(result, fn {type, _} -> type end)
      assert "EQUALS" in types
      assert "EQUALS_EQUALS" in types
    end
  end

  describe "NOT_EQUALS token (Lattice extension)" do
    test "not equals operator" do
      assert tokenize!("!=") == [{"NOT_EQUALS", "!="}]
    end

    test "not equals in condition" do
      result = tokenize!("$x != 0")
      assert {"NOT_EQUALS", "!="} in result
    end
  end

  describe "GREATER_EQUALS token (Lattice extension)" do
    test "greater equals operator" do
      assert tokenize!(">=") == [{"GREATER_EQUALS", ">="}]
    end

    test "greater equals distinct from greater" do
      result = tokenize!("> >=")
      types = Enum.map(result, fn {type, _} -> type end)
      assert "GREATER" in types
      assert "GREATER_EQUALS" in types
    end
  end

  describe "LESS_EQUALS token (Lattice extension)" do
    test "less equals operator" do
      assert tokenize!("<=") == [{"LESS_EQUALS", "<="}]
    end
  end

  # ---------------------------------------------------------------------------
  # CSS token types (must all work in Lattice context)
  # ---------------------------------------------------------------------------

  describe "STRING tokens" do
    test "double-quoted string" do
      # The lexer strips quotes — value is the content without quotes
      assert tokenize!(~s("hello")) == [{"STRING", "hello"}]
    end

    test "single-quoted string" do
      assert tokenize!("'world'") == [{"STRING", "world"}]
    end

    test "string in @use directive" do
      result = tokenize!(~s(@use "colors";))
      assert {"STRING", "colors"} in result
    end
  end

  describe "numeric tokens" do
    test "integer NUMBER" do
      assert tokenize!("42") == [{"NUMBER", "42"}]
    end

    test "float NUMBER" do
      assert tokenize!("3.14") == [{"NUMBER", "3.14"}]
    end

    test "negative NUMBER" do
      assert tokenize!("-5") == [{"NUMBER", "-5"}]
    end

    test "DIMENSION (number + unit)" do
      assert tokenize!("16px") == [{"DIMENSION", "16px"}]
    end

    test "DIMENSION with em" do
      assert tokenize!("2em") == [{"DIMENSION", "2em"}]
    end

    test "DIMENSION with rem" do
      assert tokenize!("1.5rem") == [{"DIMENSION", "1.5rem"}]
    end

    test "PERCENTAGE" do
      assert tokenize!("50%") == [{"PERCENTAGE", "50%"}]
    end

    test "PERCENTAGE float" do
      assert tokenize!("33.33%") == [{"PERCENTAGE", "33.33%"}]
    end

    test "DIMENSION before PERCENTAGE before NUMBER ordering" do
      # The grammar must match DIMENSION first — "16px" is DIMENSION not NUMBER+IDENT
      result = tokenize!("16px 50% 42")
      types = Enum.map(result, fn {type, _} -> type end)
      assert types == ["DIMENSION", "PERCENTAGE", "NUMBER"]
    end
  end

  describe "HASH (color) token" do
    test "hex color short" do
      assert tokenize!("#fff") == [{"HASH", "#fff"}]
    end

    test "hex color full" do
      assert tokenize!("#4a90d9") == [{"HASH", "#4a90d9"}]
    end

    test "id selector hash" do
      assert tokenize!("#main") == [{"HASH", "#main"}]
    end
  end

  describe "AT_KEYWORD token" do
    test "@media" do
      assert tokenize!("@media") == [{"AT_KEYWORD", "@media"}]
    end

    test "@import" do
      assert tokenize!("@import") == [{"AT_KEYWORD", "@import"}]
    end

    test "@mixin (Lattice)" do
      assert tokenize!("@mixin") == [{"AT_KEYWORD", "@mixin"}]
    end

    test "@include (Lattice)" do
      assert tokenize!("@include") == [{"AT_KEYWORD", "@include"}]
    end

    test "@if (Lattice)" do
      assert tokenize!("@if") == [{"AT_KEYWORD", "@if"}]
    end

    test "@else (Lattice)" do
      assert tokenize!("@else") == [{"AT_KEYWORD", "@else"}]
    end

    test "@for (Lattice)" do
      assert tokenize!("@for") == [{"AT_KEYWORD", "@for"}]
    end

    test "@each (Lattice)" do
      assert tokenize!("@each") == [{"AT_KEYWORD", "@each"}]
    end

    test "@function (Lattice)" do
      assert tokenize!("@function") == [{"AT_KEYWORD", "@function"}]
    end

    test "@return (Lattice)" do
      assert tokenize!("@return") == [{"AT_KEYWORD", "@return"}]
    end

    test "@use (Lattice)" do
      assert tokenize!("@use") == [{"AT_KEYWORD", "@use"}]
    end
  end

  describe "IDENT token" do
    test "simple identifier" do
      assert tokenize!("red") == [{"IDENT", "red"}]
    end

    test "hyphenated identifier" do
      assert tokenize!("sans-serif") == [{"IDENT", "sans-serif"}]
    end

    test "identifier with leading hyphen (valid CSS)" do
      assert tokenize!("-webkit-transform") == [{"IDENT", "-webkit-transform"}]
    end
  end

  describe "FUNCTION token" do
    test "function name with open paren" do
      # FUNCTION token includes the opening paren
      assert tokenize!("rgb(") == [{"FUNCTION", "rgb("}]
    end

    test "calc function" do
      assert tokenize!("calc(") == [{"FUNCTION", "calc("}]
    end
  end

  describe "CUSTOM_PROPERTY token" do
    test "CSS custom property" do
      assert tokenize!("--primary-color") == [{"CUSTOM_PROPERTY", "--primary-color"}]
    end
  end

  describe "delimiter tokens" do
    test "LBRACE" do
      assert tokenize!("{") == [{"LBRACE", "{"}]
    end

    test "RBRACE" do
      assert tokenize!("}") == [{"RBRACE", "}"}]
    end

    test "LPAREN" do
      assert tokenize!("(") == [{"LPAREN", "("}]
    end

    test "RPAREN" do
      assert tokenize!(")") == [{"RPAREN", ")"}]
    end

    test "SEMICOLON" do
      assert tokenize!(";") == [{"SEMICOLON", ";"}]
    end

    test "COLON" do
      assert tokenize!(":") == [{"COLON", ":"}]
    end

    test "COMMA" do
      assert tokenize!(",") == [{"COMMA", ","}]
    end

    test "DOT" do
      assert tokenize!(".") == [{"DOT", "."}]
    end

    test "PLUS" do
      assert tokenize!("+") == [{"PLUS", "+"}]
    end

    test "GREATER" do
      assert tokenize!(">") == [{"GREATER", ">"}]
    end

    test "STAR" do
      assert tokenize!("*") == [{"STAR", "*"}]
    end

    test "SLASH" do
      assert tokenize!("/") == [{"SLASH", "/"}]
    end

    test "EQUALS" do
      assert tokenize!("=") == [{"EQUALS", "="}]
    end

    test "AMPERSAND" do
      assert tokenize!("&") == [{"AMPERSAND", "&"}]
    end

    test "BANG" do
      assert tokenize!("!") == [{"BANG", "!"}]
    end

    test "COLON_COLON for pseudo-elements" do
      assert tokenize!("::") == [{"COLON_COLON", "::"}]
    end
  end

  describe "multi-character CSS operators" do
    test "TILDE_EQUALS" do
      assert tokenize!("~=") == [{"TILDE_EQUALS", "~="}]
    end

    test "PIPE_EQUALS" do
      assert tokenize!("|=") == [{"PIPE_EQUALS", "|="}]
    end

    test "CARET_EQUALS" do
      assert tokenize!("^=") == [{"CARET_EQUALS", "^="}]
    end

    test "DOLLAR_EQUALS" do
      assert tokenize!("$=") == [{"DOLLAR_EQUALS", "$="}]
    end

    test "STAR_EQUALS" do
      assert tokenize!("*=") == [{"STAR_EQUALS", "*="}]
    end
  end

  # ---------------------------------------------------------------------------
  # Skip patterns (comments and whitespace)
  # ---------------------------------------------------------------------------

  describe "skip patterns" do
    test "single-line comment is skipped" do
      result = tokenize!("$x: 1; // this is a comment\n$y: 2;")
      types = Enum.map(result, fn {type, _} -> type end)
      refute "LINE_COMMENT" in types
      # Should have both variable declarations
      vars = Enum.filter(result, fn {type, _} -> type == "VARIABLE" end)
      assert length(vars) == 2
    end

    test "block comment is skipped" do
      result = tokenize!("$x: /* this is a block comment */ 1;")
      types = Enum.map(result, fn {type, _} -> type end)
      refute "COMMENT" in types
      assert "VARIABLE" in types
      assert "NUMBER" in types
    end

    test "whitespace is skipped" do
      result = tokenize!("  $color  :  red  ;  ")
      assert result == [
        {"VARIABLE", "$color"},
        {"COLON", ":"},
        {"IDENT", "red"},
        {"SEMICOLON", ";"}
      ]
    end

    test "newlines are skipped" do
      result = tokenize!("$a: 1;\n$b: 2;")
      vars = Enum.filter(result, fn {type, _} -> type == "VARIABLE" end)
      assert length(vars) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Token position tracking
  # ---------------------------------------------------------------------------

  describe "token position tracking" do
    test "line 1 column 1 for first token" do
      {:ok, tokens} = LatticeLexer.tokenize("$x")
      first = hd(tokens)
      assert first.line == 1
      assert first.column == 1
    end

    test "column advances correctly" do
      {:ok, tokens} = LatticeLexer.tokenize("$x: red;")
      # $x is at col 1, : at col 3, red at col 5, ; at col 8
      types = tokens |> Enum.map(fn t -> {t.type, t.column} end)
      variable = Enum.find(types, fn {t, _} -> t == "VARIABLE" end)
      assert elem(variable, 1) == 1
    end

    test "line advances after newline" do
      {:ok, tokens} = LatticeLexer.tokenize("$a: 1;\n$b: 2;")
      second_var =
        tokens
        |> Enum.filter(fn t -> t.type == "VARIABLE" end)
        |> Enum.at(1)

      assert second_var.line == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Real Lattice constructs
  # ---------------------------------------------------------------------------

  describe "variable declaration" do
    test "simple variable declaration" do
      result = types!("$primary: #4a90d9;")
      assert result == ["VARIABLE", "COLON", "HASH", "SEMICOLON"]
    end

    test "variable with dimension value" do
      result = types!("$base-size: 16px;")
      assert result == ["VARIABLE", "COLON", "DIMENSION", "SEMICOLON"]
    end

    test "variable with multiple values" do
      result = types!("$font-stack: Helvetica, sans-serif;")
      assert "VARIABLE" in result
      assert "COMMA" in result
    end
  end

  describe "@mixin definition" do
    test "mixin keyword and function token" do
      result = tokenize!("@mixin button(")
      assert result == [
        {"AT_KEYWORD", "@mixin"},
        {"FUNCTION", "button("}
      ]
    end
  end

  describe "@if / @else control flow" do
    test "if condition with equality" do
      result = tokenize!("@if $theme == dark")
      assert result == [
        {"AT_KEYWORD", "@if"},
        {"VARIABLE", "$theme"},
        {"EQUALS_EQUALS", "=="},
        {"IDENT", "dark"}
      ]
    end

    test "else keyword" do
      result = tokenize!("@else")
      assert result == [{"AT_KEYWORD", "@else"}]
    end
  end

  describe "@for loop" do
    test "for loop header" do
      result = tokenize!("@for $i from 1 through 12")
      assert result == [
        {"AT_KEYWORD", "@for"},
        {"VARIABLE", "$i"},
        {"IDENT", "from"},
        {"NUMBER", "1"},
        {"IDENT", "through"},
        {"NUMBER", "12"}
      ]
    end
  end

  describe "@function definition" do
    test "function definition header" do
      result = tokenize!("@function spacing($n)")
      assert result == [
        {"AT_KEYWORD", "@function"},
        {"FUNCTION", "spacing("},
        {"VARIABLE", "$n"},
        {"RPAREN", ")"}
      ]
    end

    test "@return directive" do
      result = tokenize!("@return $n * 8px;")
      assert result == [
        {"AT_KEYWORD", "@return"},
        {"VARIABLE", "$n"},
        {"STAR", "*"},
        {"DIMENSION", "8px"},
        {"SEMICOLON", ";"}
      ]
    end
  end

  describe "@use directive" do
    test "use with string" do
      result = tokenize!(~s(@use "colors";))
      assert result == [
        {"AT_KEYWORD", "@use"},
        {"STRING", "colors"},
        {"SEMICOLON", ";"}
      ]
    end
  end

  describe "CSS qualified rule (passes through)" do
    test "selector and block opener" do
      result = tokenize!("h1 {")
      assert result == [
        {"IDENT", "h1"},
        {"LBRACE", "{"}
      ]
    end

    test "class selector" do
      result = tokenize!(".button {")
      assert result == [
        {"DOT", "."},
        {"IDENT", "button"},
        {"LBRACE", "{"}
      ]
    end

    test "declaration" do
      result = tokenize!("color: red;")
      assert result == [
        {"IDENT", "color"},
        {"COLON", ":"},
        {"IDENT", "red"},
        {"SEMICOLON", ";"}
      ]
    end
  end

  describe "full Lattice snippet" do
    test "variable + qualified rule" do
      source = """
      $primary: #4a90d9;
      h1 { color: $primary; }
      """

      {:ok, tokens} = LatticeLexer.tokenize(source)
      types = tokens |> Enum.map(fn t -> t.type end) |> Enum.reject(&(&1 == "EOF"))

      assert "VARIABLE" in types
      assert "HASH" in types
      assert "IDENT" in types
      assert "LBRACE" in types
      assert "RBRACE" in types
    end

    test "mixin definition and include" do
      source = """
      @mixin flex-center() {
        display: flex;
        align-items: center;
      }
      .box { @include flex-center(); }
      """

      {:ok, tokens} = LatticeLexer.tokenize(source)
      at_keywords = tokens |> Enum.filter(fn t -> t.type == "AT_KEYWORD" end) |> Enum.map(fn t -> t.value end)

      assert "@mixin" in at_keywords
      assert "@include" in at_keywords
    end
  end
end
