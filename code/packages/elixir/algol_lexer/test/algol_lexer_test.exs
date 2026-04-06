defmodule CodingAdventures.AlgolLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.AlgolLexer

  # Convenience: extract just the token type list, dropping EOF.
  defp types(source) do
    {:ok, tokens} = AlgolLexer.tokenize(source)
    tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
  end

  # Convenience: first token from a source string.
  defp first(source) do
    {:ok, [tok | _]} = AlgolLexer.tokenize(source)
    tok
  end

  # ---------------------------------------------------------------------------
  # Grammar inspection
  # ---------------------------------------------------------------------------

  describe "create_lexer/0" do
    # The TokenGrammar should contain every token kind defined in algol.tokens.
    test "returns a TokenGrammar with value token kinds" do
      grammar = AlgolLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "REAL_LIT" in names
      assert "INTEGER_LIT" in names
      assert "STRING_LIT" in names
      assert "NAME" in names
    end

    test "returns a TokenGrammar with operator kinds" do
      grammar = AlgolLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "ASSIGN" in names
      assert "POWER" in names
      assert "LEQ" in names
      assert "GEQ" in names
      assert "NEQ" in names
      assert "PLUS" in names
      assert "MINUS" in names
      assert "STAR" in names
      assert "SLASH" in names
      assert "CARET" in names
      assert "EQ" in names
      assert "LT" in names
      assert "GT" in names
    end

    test "returns a TokenGrammar with delimiter kinds" do
      grammar = AlgolLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "LPAREN" in names
      assert "RPAREN" in names
      assert "LBRACKET" in names
      assert "RBRACKET" in names
      assert "SEMICOLON" in names
      assert "COMMA" in names
      assert "COLON" in names
    end
  end

  # ---------------------------------------------------------------------------
  # Keywords
  # ---------------------------------------------------------------------------
  #
  # ALGOL 60 keywords are reserved. They are reclassified from IDENT after a
  # full-token match, so "beginning" is IDENT (not BEGIN), and "integer1" is
  # IDENT (not INTEGER followed by a digit).

  describe "tokenize/1 — keywords" do
    test "begin and end" do
      assert types("begin end") == ["begin", "end"]
    end

    test "if then else" do
      assert types("if then else") == ["if", "then", "else"]
    end

    test "for do step until while" do
      assert types("for do step until while") == ["for", "do", "step", "until", "while"]
    end

    test "goto" do
      assert types("goto") == ["goto"]
    end

    test "switch procedure" do
      assert types("switch procedure") == ["switch", "procedure"]
    end

    test "own array label value" do
      assert types("own array label value") == ["own", "array", "label", "value"]
    end

    test "integer real boolean string" do
      assert types("integer real boolean string") == ["integer", "real", "boolean", "string"]
    end

    test "true false" do
      assert types("true false") == ["true", "false"]
    end

    test "not and or impl eqv" do
      assert types("not and or impl eqv") == ["not", "and", "or", "impl", "eqv"]
    end

    test "div mod" do
      assert types("div mod") == ["div", "mod"]
    end
  end

  # ---------------------------------------------------------------------------
  # Keyword boundary: partial matches are IDENT, not keywords
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — keyword boundary" do
    # "beginning" starts with "begin" but is a full IDENT — not BEGIN.
    test "beginning is IDENT not begin" do
      assert types("beginning") == ["NAME"]
      assert first("beginning").value == "beginning"
    end

    # "integer1" is a single IDENT — not INTEGER followed by something.
    test "integer1 is IDENT" do
      assert types("integer1") == ["NAME"]
      assert first("integer1").value == "integer1"
    end

    # "endgame" — starts with "end" but not followed by a delimiter.
    test "endgame is IDENT" do
      assert types("endgame") == ["NAME"]
      assert first("endgame").value == "endgame"
    end

    # "trueblood" — starts with "true".
    test "trueblood is IDENT" do
      assert types("trueblood") == ["NAME"]
    end

    # "foreach" — starts with "for".
    test "foreach is IDENT" do
      assert types("foreach") == ["NAME"]
      assert first("foreach").value == "foreach"
    end
  end

  # ---------------------------------------------------------------------------
  # Literals
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — INTEGER_LIT" do
    test "plain integer" do
      tok = first("42")
      assert tok.type == "INTEGER_LIT"
      assert tok.value == "42"
    end

    test "single digit" do
      tok = first("0")
      assert tok.type == "INTEGER_LIT"
      assert tok.value == "0"
    end

    test "large integer" do
      tok = first("1000000")
      assert tok.type == "INTEGER_LIT"
      assert tok.value == "1000000"
    end
  end

  describe "tokenize/1 — REAL_LIT" do
    # REAL_LIT must be matched before INTEGER_LIT; the grammar ordering ensures this.

    test "decimal form 3.14" do
      tok = first("3.14")
      assert tok.type == "REAL_LIT"
      assert tok.value == "3.14"
    end

    test "scientific notation 1.5E3" do
      tok = first("1.5E3")
      assert tok.type == "REAL_LIT"
      assert tok.value == "1.5E3"
    end

    test "scientific notation with negative exponent 1.5E-3" do
      tok = first("1.5E-3")
      assert tok.type == "REAL_LIT"
      assert tok.value == "1.5E-3"
    end

    test "exponent without decimal point 100E2" do
      tok = first("100E2")
      assert tok.type == "REAL_LIT"
      assert tok.value == "100E2"
    end

    test "lowercase e exponent" do
      tok = first("2.0e10")
      assert tok.type == "REAL_LIT"
      assert tok.value == "2.0e10"
    end
  end

  describe "tokenize/1 — STRING_LIT" do
    # ALGOL 60 string literals are single-quoted. There are no escape sequences —
    # a literal single-quote cannot appear inside a string. The value returned
    # by the lexer strips the surrounding quotes.

    test "simple string" do
      tok = first("'hello'")
      assert tok.type == "STRING_LIT"
      # The grammar engine strips surrounding quotes and stores the content.
      assert tok.value == "hello"
    end

    test "empty string" do
      tok = first("''")
      assert tok.type == "STRING_LIT"
      assert tok.value == ""
    end

    test "string with spaces" do
      tok = first("'hello world'")
      assert tok.type == "STRING_LIT"
      assert tok.value == "hello world"
    end
  end

  # ---------------------------------------------------------------------------
  # Operators
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — operators" do
    # := is the ALGOL assignment operator. The design was deliberate: using a
    # two-character sequence for assignment makes it impossible to confuse with
    # equality testing (=), a bug that C inherited when it chose = for assignment.
    test "ASSIGN :=" do
      assert types(":=") == ["ASSIGN"]
    end

    # Exponentiation: ** (Fortran style) — must be matched before * * (two STARs).
    test "POWER **" do
      assert types("**") == ["POWER"]
    end

    # CARET ^ is the alternative exponentiation token.
    test "CARET ^" do
      assert types("^") == ["CARET"]
    end

    # Multi-char relational operators must be matched before their single-char prefixes.
    test "LEQ <=" do
      assert types("<=") == ["LEQ"]
    end

    test "GEQ >=" do
      assert types(">=") == ["GEQ"]
    end

    test "NEQ !=" do
      assert types("!=") == ["NEQ"]
    end

    # EQ = means equality test (not assignment).
    test "EQ =" do
      assert types("=") == ["EQ"]
    end

    test "LT <" do
      assert types("<") == ["LT"]
    end

    test "GT >" do
      assert types(">") == ["GT"]
    end

    test "PLUS +" do
      assert types("+") == ["PLUS"]
    end

    test "MINUS -" do
      assert types("-") == ["MINUS"]
    end

    test "STAR *" do
      assert types("*") == ["STAR"]
    end

    test "SLASH /" do
      assert types("/") == ["SLASH"]
    end
  end

  # ---------------------------------------------------------------------------
  # Delimiters
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — delimiters" do
    test "all delimiters in sequence" do
      assert types("()[]:;,") == ["LPAREN", "RPAREN", "LBRACKET", "RBRACKET", "COLON", "SEMICOLON", "COMMA"]
    end

    # := must be matched before : so x := 5 is IDENT ASSIGN INTEGER_LIT.
    test "colon does not consume := prefix" do
      assert types(":=") == ["ASSIGN"]
      assert types(":") == ["COLON"]
    end
  end

  # ---------------------------------------------------------------------------
  # Comment skipping
  # ---------------------------------------------------------------------------
  #
  # ALGOL 60 comments have a distinctive form: the keyword `comment` followed
  # by any text, terminated by a semicolon. The comment keyword and everything
  # up to (and including) the semicolon is consumed silently.
  #
  # This means a comment is just another statement form, terminated like any
  # other ALGOL statement. Comments cannot be nested.

  describe "tokenize/1 — comment skipping" do
    test "comment before statement is skipped entirely" do
      # Only the assignment should appear; the comment (including its semicolon)
      # is consumed by the COMMENT skip rule.
      result = types("comment this is ignored; x := 1")
      assert result == ["NAME", "ASSIGN", "INTEGER_LIT"]
    end

    test "comment with multiple words is skipped" do
      result = types("comment this has many words and symbols 123; y := 2")
      assert result == ["NAME", "ASSIGN", "INTEGER_LIT"]
    end

    test "code after comment is tokenized normally" do
      {:ok, tokens} = AlgolLexer.tokenize("comment skip me; z := 99")
      non_eof = Enum.reject(tokens, &(&1.type == "EOF"))
      [ident, assign, num] = non_eof
      assert ident.value == "z"
      assert assign.type == "ASSIGN"
      assert num.value == "99"
    end
  end

  # ---------------------------------------------------------------------------
  # Whitespace insignificance
  # ---------------------------------------------------------------------------
  #
  # One of ALGOL 60's key improvements over Fortran: whitespace is completely
  # insignificant. "x:=1" and "x  :=   1" produce identical token streams.

  describe "tokenize/1 — whitespace insignificance" do
    test "no spaces produces same types as with spaces" do
      assert types("x:=1") == types("x := 1")
    end

    test "extra spaces and tabs are ignored" do
      assert types("x   :=\t42") == ["NAME", "ASSIGN", "INTEGER_LIT"]
    end

    test "newlines between tokens are ignored" do
      source = """
      begin
        integer x;
        x := 42
      end
      """

      result = types(source)
      assert "begin" in result
      assert "integer" in result
      assert "NAME" in result
      assert "ASSIGN" in result
      assert "INTEGER_LIT" in result
      assert "end" in result
    end
  end

  # ---------------------------------------------------------------------------
  # Full expression tokenization
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — expressions" do
    test "simple assignment x := 42" do
      assert types("x := 42") == ["NAME", "ASSIGN", "INTEGER_LIT"]
    end

    test "arithmetic expression x := 1 + 2 * 3" do
      assert types("x := 1 + 2 * 3") == [
               "NAME", "ASSIGN",
               "INTEGER_LIT", "PLUS",
               "INTEGER_LIT", "STAR",
               "INTEGER_LIT"
             ]
    end

    test "relational expression pi > 3.0" do
      assert types("pi > 3.0") == ["NAME", "GT", "REAL_LIT"]
    end

    test "exponentiation x ** 2" do
      assert types("x ** 2") == ["NAME", "POWER", "INTEGER_LIT"]
    end

    test "exponentiation x ^ 2 (caret style)" do
      assert types("x ^ 2") == ["NAME", "CARET", "INTEGER_LIT"]
    end

    test "boolean expression a and b or c" do
      assert types("a and b or c") == ["NAME", "and", "NAME", "or", "NAME"]
    end

    test "not operator" do
      assert types("not flag") == ["not", "NAME"]
    end
  end

  # ---------------------------------------------------------------------------
  # Full programs
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — full programs" do
    test "minimal program: begin integer x; x := 42 end" do
      result = types("begin integer x; x := 42 end")
      assert result == [
               "begin", "integer", "NAME", "SEMICOLON",
               "NAME", "ASSIGN", "INTEGER_LIT",
               "end"
             ]
    end

    test "real variable and assignment" do
      result = types("begin real pi; pi := 3.14159 end")
      assert result == [
               "begin", "real", "NAME", "SEMICOLON",
               "NAME", "ASSIGN", "REAL_LIT",
               "end"
             ]
    end

    test "for loop header" do
      result = types("for x := 1 step 1 until 10 do")
      assert result == [
               "for", "NAME", "ASSIGN",
               "INTEGER_LIT", "step", "INTEGER_LIT", "until", "INTEGER_LIT",
               "do"
             ]
    end

    test "if statement" do
      result = types("if x > 0 then x := 1")
      assert result == [
               "if", "NAME", "GT", "INTEGER_LIT",
               "then", "NAME", "ASSIGN", "INTEGER_LIT"
             ]
    end

    test "procedure declaration header" do
      result = types("procedure foo(a, b);")
      assert result == [
               "procedure", "NAME",
               "LPAREN", "NAME", "COMMA", "NAME", "RPAREN",
               "SEMICOLON"
             ]
    end
  end

  # ---------------------------------------------------------------------------
  # Position tracking
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — position tracking" do
    test "first token on line 1 column 1" do
      tok = first("begin")
      assert tok.line == 1
      assert tok.column == 1
    end

    test "column advances across tokens" do
      {:ok, tokens} = AlgolLexer.tokenize("x := 1")
      [ident, assign, num | _] = tokens
      assert ident.column == 1
      # := is at column 3 (after "x ")
      assert assign.column == 3
      # 1 is at column 6 (after "x := ")
      assert num.column == 6
    end

    test "line advances across newlines" do
      {:ok, tokens} = AlgolLexer.tokenize("x\ny")
      [first_tok, second_tok | _] = tokens
      assert first_tok.line == 1
      assert second_tok.line == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — error cases" do
    test "unexpected character returns error" do
      {:error, msg} = AlgolLexer.tokenize("@")
      assert msg =~ "Unexpected character"
    end

    test "hash character is not valid ALGOL" do
      {:error, _msg} = AlgolLexer.tokenize("#")
    end
  end

  # ---------------------------------------------------------------------------
  # EOF sentinel
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — EOF token" do
    test "tokenization always ends with EOF" do
      {:ok, tokens} = AlgolLexer.tokenize("x")
      last = List.last(tokens)
      assert last.type == "EOF"
    end

    test "empty source produces only EOF" do
      {:ok, tokens} = AlgolLexer.tokenize("")
      assert length(tokens) == 1
      [eof] = tokens
      assert eof.type == "EOF"
    end
  end
end
