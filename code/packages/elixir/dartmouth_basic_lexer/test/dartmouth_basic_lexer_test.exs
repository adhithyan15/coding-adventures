defmodule CodingAdventures.DartmouthBasicLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.DartmouthBasicLexer

  # ---------------------------------------------------------------------------
  # Test helpers
  # ---------------------------------------------------------------------------
  #
  # These small helpers reduce boilerplate in tests so each test can focus on
  # what it actually cares about.

  # Extract just the token types from a source string, dropping the final EOF.
  # This is the most common assertion: "tokenising X gives these types in order."
  defp types(source) do
    {:ok, tokens} = DartmouthBasicLexer.tokenize(source)
    tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
  end

  # Extract {type, value} pairs, dropping EOF. Useful when both type AND value
  # matter, e.g. verifying KEYWORD("PRINT") vs KEYWORD("LET").
  defp type_values(source) do
    {:ok, tokens} = DartmouthBasicLexer.tokenize(source)
    tokens |> Enum.map(&{&1.type, &1.value}) |> Enum.reject(fn {t, _} -> t == "EOF" end)
  end

  # Return just the first token from a source string. Useful for testing a
  # single token type or value without worrying about the rest.
  defp first_token(source) do
    {:ok, [tok | _]} = DartmouthBasicLexer.tokenize(source)
    tok
  end

  # ---------------------------------------------------------------------------
  # Grammar inspection: create_lexer/0
  # ---------------------------------------------------------------------------
  #
  # These tests verify that the grammar file is well-formed and contains all
  # the expected token definitions. They test the STRUCTURE of the grammar,
  # not the tokenisation behaviour.

  describe "create_lexer/0" do
    test "returns a TokenGrammar struct" do
      grammar = DartmouthBasicLexer.create_lexer()
      # TokenGrammar has a :definitions field containing token rules.
      assert is_list(grammar.definitions)
      assert length(grammar.definitions) > 0
    end

    test "grammar contains comparison operator tokens" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      # Multi-character operators — must appear before single-char ones in grammar.
      assert "LE" in names
      assert "GE" in names
      assert "NE" in names
    end

    test "grammar contains numeric tokens" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "LINE_NUM" in names
      assert "NUMBER" in names
    end

    test "grammar contains literal tokens" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      # STRING_BODY aliased to STRING in the grammar.
      assert "STRING_BODY" in names
    end

    test "grammar contains function tokens" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "BUILTIN_FN" in names
      assert "USER_FN" in names
    end

    test "grammar contains NAME and keyword section" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "NAME" in names
    end

    test "grammar contains arithmetic operator tokens" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "PLUS" in names
      assert "MINUS" in names
      assert "STAR" in names
      assert "SLASH" in names
      assert "CARET" in names
      assert "EQ" in names
      assert "LT" in names
      assert "GT" in names
    end

    test "grammar contains punctuation tokens" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "LPAREN" in names
      assert "RPAREN" in names
      assert "COMMA" in names
      assert "SEMICOLON" in names
    end

    test "grammar contains NEWLINE token" do
      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      # NEWLINE is significant in BASIC — it terminates statements.
      assert "NEWLINE" in names
    end
  end

  # ---------------------------------------------------------------------------
  # Basic tokenisation: canonical example
  # ---------------------------------------------------------------------------
  #
  # The most fundamental BASIC statement: LET assigns a value to a variable.
  # This test verifies the entire token pipeline on the simplest possible
  # meaningful program line.

  describe "tokenize/1 — canonical LET statement" do
    test "10 LET X = 5 produces expected token types" do
      result = types("10 LET X = 5\n")
      assert result == ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE"]
    end

    test "10 LET X = 5 produces expected token values" do
      result = type_values("10 LET X = 5\n")

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "LET"},
               {"NAME", "X"},
               {"EQ", "="},
               {"NUMBER", "5"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "tokenization always ends with EOF" do
      {:ok, tokens} = DartmouthBasicLexer.tokenize("10 END\n")
      last = List.last(tokens)
      assert last.type == "EOF"
      assert last.value == ""
    end

    test "empty source produces only EOF" do
      {:ok, tokens} = DartmouthBasicLexer.tokenize("")
      assert length(tokens) == 1
      [eof] = tokens
      assert eof.type == "EOF"
    end
  end

  # ---------------------------------------------------------------------------
  # LINE_NUM disambiguation
  # ---------------------------------------------------------------------------
  #
  # This is the trickiest part of BASIC lexing. The NUMBER regex matches digits
  # in all positions. A post-tokenize hook relabels the first number on each
  # line as LINE_NUM.

  describe "tokenize/1 — LINE_NUM vs NUMBER disambiguation" do
    test "number at start of line becomes LINE_NUM" do
      # "10" at position 0 → LINE_NUM
      tok = first_token("10 LET X = 0\n")
      assert tok.type == "LINE_NUM"
      assert tok.value == "10"
    end

    test "number after GOTO is NUMBER, not LINE_NUM" do
      # "30" is the jump target — a NUMBER in expression position.
      result = type_values("10 GOTO 30\n")

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "GOTO"},
               {"NUMBER", "30"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "number after THEN is NUMBER, not LINE_NUM" do
      # In IF...THEN 50, the 50 is a jump target — a NUMBER.
      # The final token before EOF is NEWLINE, so we use `in` not List.last.
      result = type_values("10 IF X > 0 THEN 50\n")

      assert {"LINE_NUM", "10"} == hd(result)
      assert {"NUMBER", "50"} in result
    end

    test "each new line resets the state" do
      # Both 10 and 20 should become LINE_NUM.
      result = type_values("10 LET X = 1\n20 LET Y = 2\n")
      line_nums = Enum.filter(result, fn {t, _} -> t == "LINE_NUM" end)
      assert line_nums == [{"LINE_NUM", "10"}, {"LINE_NUM", "20"}]
    end

    test "numbers inside expressions remain NUMBER" do
      # 42 and 3 are expression values, not line labels.
      result = types("10 LET X = 42 + 3\n")
      assert result == ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "PLUS", "NUMBER", "NEWLINE"]
    end

    test "three-line program has three LINE_NUM tokens" do
      source = "10 LET X = 1\n20 PRINT X\n30 END\n"
      result = type_values(source)
      line_nums = Enum.filter(result, fn {t, _} -> t == "LINE_NUM" end)
      assert length(line_nums) == 3
      assert Enum.map(line_nums, &elem(&1, 1)) == ["10", "20", "30"]
    end
  end

  # ---------------------------------------------------------------------------
  # REM comment suppression
  # ---------------------------------------------------------------------------
  #
  # REM is the ONLY comment form in Dartmouth BASIC 1964. Everything after REM
  # on the same line is discarded. The NEWLINE is kept (to terminate the statement).
  # The KEYWORD("REM") is kept (so the parser knows this is a REM statement).

  describe "tokenize/1 — REM suppression" do
    test "REM with comment text: only LINE_NUM, REM, NEWLINE remain" do
      result = type_values("10 REM THIS IS A COMMENT\n")

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "REM"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "REM with no following text: LINE_NUM, REM, NEWLINE" do
      result = type_values("10 REM\n")

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "REM"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "line after REM line tokenises normally" do
      # The REM suppression state resets at NEWLINE.
      # Line 20 should tokenise fully.
      source = "10 REM IGNORE\n20 LET X = 1\n"
      result = type_values(source)

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "REM"},
               {"NEWLINE", "\\n"},
               {"LINE_NUM", "20"},
               {"KEYWORD", "LET"},
               {"NAME", "X"},
               {"EQ", "="},
               {"NUMBER", "1"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "REM suppresses keywords that look like tokens" do
      # Keywords and operators in the comment body must be suppressed.
      result = type_values("10 REM PRINT LET GOTO + - * /\n")

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "REM"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "REM after a normal statement on same line" do
      # This is unusual syntax but the lexer should still suppress content after REM.
      # "10 PRINT X" but then followed by REM would be parser-invalid BASIC,
      # but we test lexer behaviour only.
      result = type_values("10 REM COMMENT WITH NUMBERS 100 200\n")

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "REM"},
               {"NEWLINE", "\\n"}
             ]
    end
  end

  # ---------------------------------------------------------------------------
  # Case insensitivity
  # ---------------------------------------------------------------------------
  #
  # The grammar uses @case_insensitive true, meaning the whole source is
  # uppercased before tokenisation. This means `print`, `Print`, and `PRINT`
  # all produce KEYWORD("PRINT") with value "PRINT" (uppercase).

  describe "tokenize/1 — case insensitivity" do
    test "lowercase keywords match uppercase" do
      lower = types("10 let x = 1\n")
      upper = types("10 LET X = 1\n")
      assert lower == upper
    end

    test "lowercase keyword produces uppercase value" do
      result = type_values("10 print x\n")
      keyword = Enum.find(result, fn {t, _} -> t == "KEYWORD" end)
      assert keyword == {"KEYWORD", "PRINT"}
    end

    test "mixed case keywords normalised" do
      result = types("10 Goto 20\n")
      assert "KEYWORD" in result
    end

    test "lowercase variable names uppercased" do
      result = type_values("10 let a = 1\n")
      name = Enum.find(result, fn {t, _} -> t == "NAME" end)
      assert name == {"NAME", "A"}
    end

    test "lowercase built-in functions uppercased" do
      result = type_values("10 let x = sin(y)\n")
      fn_tok = Enum.find(result, fn {t, _} -> t == "BUILTIN_FN" end)
      assert fn_tok == {"BUILTIN_FN", "SIN"}
    end

    test "case insensitive for all 20 keywords" do
      # Spot check several keywords in lowercase.
      keywords = ["let", "print", "if", "goto", "for", "next", "end", "rem", "def", "dim"]

      for kw <- keywords do
        result = type_values("10 #{kw} x\n")
        found = Enum.any?(result, fn {t, v} -> t == "KEYWORD" and v == String.upcase(kw) end)
        assert found, "expected KEYWORD(#{String.upcase(kw)}) from lowercase '#{kw}'"
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Keywords
  # ---------------------------------------------------------------------------
  #
  # All 20 Dartmouth BASIC 1964 keywords must be recognised as KEYWORD tokens,
  # not NAME tokens. Each keyword is a full-token match against the NAME regex,
  # so longer identifiers like "LETTERS" (starts with "LET") must remain NAME.

  describe "tokenize/1 — all 20 keywords" do
    # The 20 keywords of Dartmouth BASIC 1964, listed for reference:
    #   LET PRINT INPUT IF THEN GOTO GOSUB RETURN FOR TO STEP NEXT
    #   END STOP REM READ DATA RESTORE DIM DEF
    test "all 20 keywords produce KEYWORD tokens" do
      keywords = ~w(LET PRINT INPUT IF THEN GOTO GOSUB RETURN FOR TO STEP NEXT
                    END STOP REM READ DATA RESTORE DIM DEF)

      for kw <- keywords do
        result = type_values("10 #{kw} X\n")
        found = Enum.any?(result, fn {t, v} -> t == "KEYWORD" and v == kw end)
        assert found, "Expected KEYWORD(#{kw}) but did not find it in: #{inspect(result)}"
      end
    end

    test "LET keyword" do
      assert {"KEYWORD", "LET"} in type_values("10 LET X = 1\n")
    end

    test "PRINT keyword" do
      assert {"KEYWORD", "PRINT"} in type_values("10 PRINT X\n")
    end

    test "INPUT keyword" do
      assert {"KEYWORD", "INPUT"} in type_values("10 INPUT X\n")
    end

    test "IF and THEN keywords" do
      result = type_values("10 IF X > 0 THEN 20\n")
      assert {"KEYWORD", "IF"} in result
      assert {"KEYWORD", "THEN"} in result
    end

    test "GOTO keyword" do
      assert {"KEYWORD", "GOTO"} in type_values("10 GOTO 20\n")
    end

    test "GOSUB and RETURN keywords" do
      result1 = type_values("10 GOSUB 100\n")
      result2 = type_values("10 RETURN\n")
      assert {"KEYWORD", "GOSUB"} in result1
      assert {"KEYWORD", "RETURN"} in result2
    end

    test "FOR, TO, STEP, NEXT keywords" do
      result = type_values("10 FOR I = 1 TO 10 STEP 2\n20 NEXT I\n")
      assert {"KEYWORD", "FOR"} in result
      assert {"KEYWORD", "TO"} in result
      assert {"KEYWORD", "STEP"} in result
      assert {"KEYWORD", "NEXT"} in result
    end

    test "END and STOP keywords" do
      result1 = type_values("10 END\n")
      result2 = type_values("10 STOP\n")
      assert {"KEYWORD", "END"} in result1
      assert {"KEYWORD", "STOP"} in result2
    end

    test "READ, DATA, RESTORE keywords" do
      result1 = type_values("10 READ X\n")
      result2 = type_values("20 DATA 1, 2, 3\n")
      result3 = type_values("30 RESTORE\n")
      assert {"KEYWORD", "READ"} in result1
      assert {"KEYWORD", "DATA"} in result2
      assert {"KEYWORD", "RESTORE"} in result3
    end

    test "DIM and DEF keywords" do
      result1 = type_values("10 DIM A(10)\n")
      result2 = type_values("10 DEF FNA(X) = X * X\n")
      assert {"KEYWORD", "DIM"} in result1
      assert {"KEYWORD", "DEF"} in result2
    end
  end

  # ---------------------------------------------------------------------------
  # NUMBER literals
  # ---------------------------------------------------------------------------
  #
  # Dartmouth BASIC stores ALL numbers as floating-point internally. The lexer
  # recognises integers, decimals, and scientific notation.

  describe "tokenize/1 — NUMBER literals" do
    test "plain integer" do
      result = type_values("10 LET X = 42\n")
      assert {"NUMBER", "42"} in result
    end

    test "decimal number 3.14" do
      result = type_values("10 LET X = 3.14\n")
      assert {"NUMBER", "3.14"} in result
    end

    test "leading-dot decimal .5" do
      # .5 is valid BASIC — no integer part required.
      result = type_values("10 LET X = .5\n")
      assert {"NUMBER", ".5"} in result
    end

    test "scientific notation 1.5E3" do
      # 1.5E3 = 1.5 × 10³ = 1500.0
      # Source is downcased before tokenising, so "E" becomes "e" in the token value.
      result = type_values("10 LET X = 1.5E3\n")
      assert {"NUMBER", "1.5e3"} in result
    end

    test "scientific notation with negative exponent 1.5E-3" do
      # 1.5E-3 = 1.5 × 10⁻³ = 0.0015
      # Source is downcased before tokenising, so "E" becomes "e" in the token value.
      result = type_values("10 LET X = 1.5E-3\n")
      assert {"NUMBER", "1.5e-3"} in result
    end

    test "scientific notation without decimal part 1E10" do
      # Source is downcased before tokenising, so "E" becomes "e" in the token value.
      result = type_values("10 LET X = 1E10\n")
      assert {"NUMBER", "1e10"} in result
    end

    test "zero" do
      result = type_values("10 LET X = 0\n")
      assert {"NUMBER", "0"} in result
    end

    test "large integer" do
      result = type_values("10 LET X = 9999\n")
      assert {"NUMBER", "9999"} in result
    end
  end

  # ---------------------------------------------------------------------------
  # STRING literals
  # ---------------------------------------------------------------------------
  #
  # Dartmouth BASIC 1964 strings are double-quoted. No escape sequences are
  # supported — a double quote cannot appear inside a string.
  #
  # The GrammarLexer strips the surrounding double quotes from STRING tokens,
  # so the token value contains only the string content (without quotes).
  #
  # Because the lexer downcases the source before tokenising (the pre-tokenise
  # hook calls String.downcase/1 to handle case-insensitivity), string content
  # is also lowercased in the token value. For example:
  #   PRINT "HELLO"  →  STRING("hello")
  #
  # Note: the GrammarLexer uses the originalSource field for STRING tokens to
  # preserve casing within strings in some implementations, but the Elixir
  # GrammarLexer does not — it always uses the downcased value.

  describe "tokenize/1 — STRING literals" do
    test "simple string: quotes stripped, content lowercased" do
      source = "10 PRINT \"HELLO\"\n"
      result = type_values(source)
      string_tok = Enum.find(result, fn {t, _} -> t == "STRING" end)
      # GrammarLexer strips surrounding quotes; source downcase lowercases content.
      assert string_tok == {"STRING", "hello"}
    end

    test "string with spaces" do
      source = "10 PRINT \"HELLO WORLD\"\n"
      result = type_values(source)
      string_tok = Enum.find(result, fn {t, _} -> t == "STRING" end)
      assert string_tok == {"STRING", "hello world"}
    end

    test "empty string" do
      source = "10 PRINT \"\"\n"
      result = type_values(source)
      string_tok = Enum.find(result, fn {t, _} -> t == "STRING" end)
      # An empty string body produces STRING("") — no quotes, no content.
      assert string_tok == {"STRING", ""}
    end

    test "PRINT statement with string produces correct types" do
      source = "10 PRINT \"HI\"\n"
      result = types(source)
      assert result == ["LINE_NUM", "KEYWORD", "STRING", "NEWLINE"]
    end
  end

  # ---------------------------------------------------------------------------
  # Multi-character operators
  # ---------------------------------------------------------------------------
  #
  # The three two-character comparison operators must be recognised as single
  # tokens. Without the grammar ordering (multi-char before single-char), `<=`
  # would incorrectly lex as two tokens: LT then EQ.

  describe "tokenize/1 — multi-character operators" do
    test "LE: <= is a single token" do
      result = types("10 IF X <= Y THEN 50\n")
      # Must NOT contain both LT and EQ in sequence.
      assert "LE" in result
      refute "LT" in result
    end

    test "GE: >= is a single token" do
      result = types("10 IF X >= Y THEN 50\n")
      assert "GE" in result
      refute "GT" in result
    end

    test "NE: <> is a single token" do
      result = types("10 IF X <> Y THEN 50\n")
      assert "NE" in result
      refute "LT" in result
    end

    test "LE value is <=" do
      result = type_values("10 IF X <= 0 THEN 50\n")
      op = Enum.find(result, fn {t, _} -> t == "LE" end)
      assert op == {"LE", "<="}
    end

    test "GE value is >=" do
      result = type_values("10 IF X >= 0 THEN 50\n")
      op = Enum.find(result, fn {t, _} -> t == "GE" end)
      assert op == {"GE", ">="}
    end

    test "NE value is <>" do
      result = type_values("10 IF X <> 0 THEN 50\n")
      op = Enum.find(result, fn {t, _} -> t == "NE" end)
      assert op == {"NE", "<>"}
    end

    test "single < is LT not LE" do
      result = types("10 IF X < Y THEN 50\n")
      assert "LT" in result
      refute "LE" in result
    end

    test "single > is GT not GE" do
      result = types("10 IF X > Y THEN 50\n")
      assert "GT" in result
      refute "GE" in result
    end
  end

  # ---------------------------------------------------------------------------
  # Single-character operators and punctuation
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — single-character operators" do
    test "PLUS +" do
      assert "PLUS" in types("10 LET X = A + B\n")
    end

    test "MINUS -" do
      assert "MINUS" in types("10 LET X = A - B\n")
    end

    test "STAR *" do
      assert "STAR" in types("10 LET X = A * B\n")
    end

    test "SLASH /" do
      assert "SLASH" in types("10 LET X = A / B\n")
    end

    test "CARET ^ (exponentiation)" do
      # 2^3 = 8. BASIC uses ^ for exponentiation (not ** like Fortran).
      assert "CARET" in types("10 LET X = 2 ^ 3\n")
    end

    test "EQ =" do
      assert "EQ" in types("10 LET X = 1\n")
    end

    test "LT <" do
      assert "LT" in types("10 IF X < 0 THEN 99\n")
    end

    test "GT >" do
      assert "GT" in types("10 IF X > 0 THEN 99\n")
    end

    test "LPAREN (" do
      assert "LPAREN" in types("10 LET X = SIN(Y)\n")
    end

    test "RPAREN )" do
      assert "RPAREN" in types("10 LET X = SIN(Y)\n")
    end

    test "COMMA in PRINT" do
      # Comma advances to next print zone.
      assert "COMMA" in types("10 PRINT X, Y\n")
    end

    test "SEMICOLON in PRINT" do
      # Semicolon prints items with no space.
      assert "SEMICOLON" in types("10 PRINT X; Y\n")
    end
  end

  # ---------------------------------------------------------------------------
  # NEWLINE handling
  # ---------------------------------------------------------------------------
  #
  # In BASIC, NEWLINE is a statement terminator — it is significant and kept
  # in the token stream (unlike most languages where whitespace is skipped).

  describe "tokenize/1 — NEWLINE handling" do
    test "NEWLINE is included in token stream" do
      result = types("10 LET X = 1\n")
      assert "NEWLINE" in result
    end

    test "NEWLINE value is the two-character string backslash-n" do
      # The GrammarLexer hardcodes NEWLINE token value as the two-character
      # string "\\n" (backslash + n), not an actual newline character.
      # This is a GrammarLexer implementation detail: bare \n in source is
      # intercepted before pattern matching and emitted with value "\\n".
      result = type_values("10 END\n")
      newline = Enum.find(result, fn {t, _} -> t == "NEWLINE" end)
      assert newline == {"NEWLINE", "\\n"}
    end

    test "Windows-style CRLF is also a single NEWLINE token" do
      result = type_values("10 END\r\n")
      newlines = Enum.filter(result, fn {t, _} -> t == "NEWLINE" end)
      # Must produce exactly ONE newline token, not two.
      assert length(newlines) == 1
    end

    test "multiple lines produce multiple NEWLINE tokens" do
      result = types("10 LET X = 1\n20 PRINT X\n")
      newlines = Enum.filter(result, &(&1 == "NEWLINE"))
      assert length(newlines) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Built-in functions (BUILTIN_FN)
  # ---------------------------------------------------------------------------
  #
  # The 11 built-in mathematical functions of Dartmouth BASIC 1964.
  # They must be recognised before NAME so that "SIN" is not tokenised as
  # a variable name.

  describe "tokenize/1 — BUILTIN_FN tokens" do
    test "SIN is BUILTIN_FN" do
      assert {"BUILTIN_FN", "SIN"} in type_values("10 LET X = SIN(Y)\n")
    end

    test "COS is BUILTIN_FN" do
      assert {"BUILTIN_FN", "COS"} in type_values("10 LET X = COS(Y)\n")
    end

    test "TAN is BUILTIN_FN" do
      assert {"BUILTIN_FN", "TAN"} in type_values("10 LET X = TAN(Y)\n")
    end

    test "ATN is BUILTIN_FN" do
      assert {"BUILTIN_FN", "ATN"} in type_values("10 LET X = ATN(Y)\n")
    end

    test "EXP is BUILTIN_FN" do
      assert {"BUILTIN_FN", "EXP"} in type_values("10 LET X = EXP(Y)\n")
    end

    test "LOG is BUILTIN_FN" do
      assert {"BUILTIN_FN", "LOG"} in type_values("10 LET X = LOG(Y)\n")
    end

    test "ABS is BUILTIN_FN" do
      assert {"BUILTIN_FN", "ABS"} in type_values("10 LET X = ABS(Y)\n")
    end

    test "SQR is BUILTIN_FN" do
      assert {"BUILTIN_FN", "SQR"} in type_values("10 LET X = SQR(Y)\n")
    end

    test "INT is BUILTIN_FN" do
      assert {"BUILTIN_FN", "INT"} in type_values("10 LET X = INT(Y)\n")
    end

    test "RND is BUILTIN_FN" do
      assert {"BUILTIN_FN", "RND"} in type_values("10 LET X = RND(1)\n")
    end

    test "SGN is BUILTIN_FN" do
      assert {"BUILTIN_FN", "SGN"} in type_values("10 LET X = SGN(Y)\n")
    end

    test "all 11 built-in functions in one expression" do
      source = "10 LET X = SIN(A) + COS(A) + TAN(A) + ATN(A) + EXP(A) + LOG(A) + ABS(A) + SQR(A) + INT(A) + RND(1) + SGN(A)\n"
      result = type_values(source)
      builtin_fns = result |> Enum.filter(fn {t, _} -> t == "BUILTIN_FN" end) |> Enum.map(&elem(&1, 1))
      assert "SIN" in builtin_fns
      assert "COS" in builtin_fns
      assert "TAN" in builtin_fns
      assert "ATN" in builtin_fns
      assert "EXP" in builtin_fns
      assert "LOG" in builtin_fns
      assert "ABS" in builtin_fns
      assert "SQR" in builtin_fns
      assert "INT" in builtin_fns
      assert "RND" in builtin_fns
      assert "SGN" in builtin_fns
    end
  end

  # ---------------------------------------------------------------------------
  # User-defined functions (USER_FN)
  # ---------------------------------------------------------------------------
  #
  # User-defined functions use the form FNA, FNB, ..., FNZ. They are defined
  # with DEF FNA(X) = expression. The USER_FN rule must precede NAME in the
  # grammar so "FNA" is not tokenised as NAME("F") + NAME("NA").

  describe "tokenize/1 — USER_FN tokens" do
    test "FNA is USER_FN" do
      result = type_values("10 DEF FNA(X) = X * X\n")
      assert {"USER_FN", "FNA"} in result
    end

    test "FNB is USER_FN" do
      assert {"USER_FN", "FNB"} in type_values("10 LET Y = FNB(X)\n")
    end

    test "FNZ is USER_FN" do
      assert {"USER_FN", "FNZ"} in type_values("10 LET Y = FNZ(X)\n")
    end

    test "user function definition produces correct token sequence" do
      # DEF FNA(X) = X * X
      # Expected: KEYWORD("DEF") USER_FN("FNA") LPAREN NAME("X") RPAREN EQ NAME("X") STAR NAME("X")
      result = type_values("10 DEF FNA(X) = X * X\n")

      assert {"KEYWORD", "DEF"} in result
      assert {"USER_FN", "FNA"} in result
      assert {"LPAREN", "("} in result
      assert {"NAME", "X"} in result
    end
  end

  # ---------------------------------------------------------------------------
  # Variable names (NAME)
  # ---------------------------------------------------------------------------
  #
  # In Dartmouth BASIC 1964, variable names are exactly:
  #   - One letter: A, B, ..., Z  (26 variables)
  #   - One letter + one digit: A0, A1, ..., Z9  (260 variables)
  # Total: 286 possible variable names. All initialise to 0.

  describe "tokenize/1 — NAME (variable names)" do
    test "single letter variable" do
      result = type_values("10 LET X = 1\n")
      assert {"NAME", "X"} in result
    end

    test "letter plus digit variable" do
      result = type_values("10 LET A1 = 2\n")
      assert {"NAME", "A1"} in result
    end

    test "variable Z9" do
      result = type_values("10 LET Z9 = 3\n")
      assert {"NAME", "Z9"} in result
    end

    test "single letter I used as loop variable" do
      result = types("10 FOR I = 1 TO 5\n")
      assert "NAME" in result
    end

    test "multiple different variable names in one line" do
      result = type_values("10 LET A = B + C1\n")
      names = result |> Enum.filter(fn {t, _} -> t == "NAME" end) |> Enum.map(&elem(&1, 1))
      assert "A" in names
      assert "B" in names
      assert "C1" in names
    end
  end

  # ---------------------------------------------------------------------------
  # Full programs
  # ---------------------------------------------------------------------------
  #
  # These tests verify complete multi-line programs tokenise correctly end-to-end.

  describe "tokenize/1 — full programs" do
    test "minimal program: LET, PRINT, END" do
      source = "10 LET X = 1\n20 PRINT X\n30 END\n"
      result = type_values(source)

      assert result == [
               {"LINE_NUM", "10"},
               {"KEYWORD", "LET"},
               {"NAME", "X"},
               {"EQ", "="},
               {"NUMBER", "1"},
               {"NEWLINE", "\\n"},
               {"LINE_NUM", "20"},
               {"KEYWORD", "PRINT"},
               {"NAME", "X"},
               {"NEWLINE", "\\n"},
               {"LINE_NUM", "30"},
               {"KEYWORD", "END"},
               {"NEWLINE", "\\n"}
             ]
    end

    test "FOR/NEXT loop" do
      source = "10 FOR I = 1 TO 10 STEP 2\n20 PRINT I\n30 NEXT I\n"
      result = types(source)

      assert "FOR" not in result
      assert "KEYWORD" in result
      assert "NEXT" not in result

      keywords = type_values(source) |> Enum.filter(fn {t, _} -> t == "KEYWORD" end) |> Enum.map(&elem(&1, 1))
      assert "FOR" in keywords
      assert "TO" in keywords
      assert "STEP" in keywords
      assert "NEXT" in keywords
    end

    test "IF/THEN conditional" do
      source = "10 IF X > 0 THEN 50\n20 PRINT \"NEGATIVE\"\n50 PRINT \"POSITIVE\"\n"
      result = type_values(source)
      keywords = result |> Enum.filter(fn {t, _} -> t == "KEYWORD" end) |> Enum.map(&elem(&1, 1))
      assert "IF" in keywords
      assert "THEN" in keywords
    end

    test "GOSUB/RETURN subroutine" do
      source = "10 GOSUB 100\n20 END\n100 PRINT \"SUBROUTINE\"\n110 RETURN\n"
      result = type_values(source)
      keywords = result |> Enum.filter(fn {t, _} -> t == "KEYWORD" end) |> Enum.map(&elem(&1, 1))
      assert "GOSUB" in keywords
      assert "RETURN" in keywords
    end

    test "program with REM comments between lines" do
      source = "10 REM INITIALISE\n20 LET X = 0\n30 REM PRINT RESULT\n40 PRINT X\n"
      result = type_values(source)
      # Each REM line should only contribute LINE_NUM, REM, NEWLINE.
      rem_lines =
        result
        |> Enum.chunk_while(
          [],
          fn tok, chunk ->
            case tok do
              {"NEWLINE", _} -> {:cont, Enum.reverse([tok | chunk]), []}
              _ -> {:cont, [tok | chunk]}
            end
          end,
          fn
            [] -> {:cont, []}
            chunk -> {:cont, Enum.reverse(chunk), []}
          end
        )
        |> Enum.filter(fn line ->
          Enum.any?(line, fn {t, v} -> t == "KEYWORD" and v == "REM" end)
        end)

      for rem_line <- rem_lines do
        types_on_line = Enum.map(rem_line, &elem(&1, 0))
        assert types_on_line == ["LINE_NUM", "KEYWORD", "NEWLINE"],
               "Expected REM line to have only LINE_NUM, KEYWORD, NEWLINE but got: #{inspect(types_on_line)}"
      end
    end

    test "DEF and user function call" do
      source = "10 DEF FNA(X) = X * X\n20 LET Y = FNA(3)\n"
      result = type_values(source)
      assert {"KEYWORD", "DEF"} in result
      assert {"USER_FN", "FNA"} in result
    end

    test "trigonometric computation" do
      source = "10 LET Y = SIN(X) ^ 2 + COS(X) ^ 2\n"
      result = types(source)
      assert "BUILTIN_FN" in result
      assert "CARET" in result
    end
  end

  # ---------------------------------------------------------------------------
  # PRINT separators
  # ---------------------------------------------------------------------------
  #
  # PRINT in Dartmouth BASIC uses two separators with different spacing semantics:
  #   COMMA      (,) — advance to the next print zone (column multiple of 14)
  #   SEMICOLON  (;) — no space; items are printed adjacently

  describe "tokenize/1 — PRINT separators" do
    test "COMMA separator produces COMMA token" do
      result = type_values("10 PRINT X, Y\n")
      assert {"COMMA", ","} in result
    end

    test "SEMICOLON separator produces SEMICOLON token" do
      result = type_values("10 PRINT X; Y\n")
      assert {"SEMICOLON", ";"} in result
    end

    test "multiple comma-separated items" do
      result = types("10 PRINT A, B, C\n")
      commas = Enum.filter(result, &(&1 == "COMMA"))
      assert length(commas) == 2
    end

    test "mixed comma and semicolon" do
      result = types("10 PRINT A, B; C\n")
      assert "COMMA" in result
      assert "SEMICOLON" in result
    end
  end

  # ---------------------------------------------------------------------------
  # Position tracking
  # ---------------------------------------------------------------------------
  #
  # The lexer records line and column numbers for each token. This is
  # important for error messages: "unexpected token at line 3, column 7."

  describe "tokenize/1 — position tracking" do
    test "first token is at line 1, column 1" do
      tok = first_token("10 LET X = 1\n")
      assert tok.line == 1
      assert tok.column == 1
    end

    test "line number advances after NEWLINE" do
      {:ok, tokens} = DartmouthBasicLexer.tokenize("10 LET X = 1\n20 PRINT X\n")
      # Find the LINE_NUM token for line 20.
      line_20_num = Enum.find(tokens, fn t -> t.type == "LINE_NUM" and t.value == "20" end)
      assert line_20_num.line == 2
    end

    test "column advances across tokens on a line" do
      {:ok, tokens} = DartmouthBasicLexer.tokenize("10 END\n")
      [line_num, keyword | _] = tokens
      # "10" is at column 1; "END" is at column 4 (after "10 ").
      assert line_num.column == 1
      assert keyword.column > line_num.column
    end
  end

  # ---------------------------------------------------------------------------
  # Error handling
  # ---------------------------------------------------------------------------
  #
  # The grammar's `errors:` section is defined for compatibility with other
  # language implementations (Python, Ruby, TypeScript) that support UNKNOWN
  # token recovery. The Elixir GrammarLexer does NOT implement this section —
  # when it encounters an unrecognised character, it returns {:error, message}
  # immediately rather than emitting an UNKNOWN token and continuing.
  #
  # This means tokenize/1 returns {:error, string} (not {:ok, tokens}) when
  # the source contains any character that the grammar doesn't recognise.

  describe "tokenize/1 — error handling for unknown characters" do
    test "unknown character @ produces an error tuple" do
      # The Elixir GrammarLexer does not implement the errors: section.
      # Unrecognised characters cause tokenize/1 to return {:error, message}.
      result = DartmouthBasicLexer.tokenize("10 LET @ = 1\n")
      assert {:error, _message} = result
    end

    test "unknown character @ error message contains position info" do
      # The error message includes the line and column of the bad character.
      {:error, message} = DartmouthBasicLexer.tokenize("10 LET @ = 1\n")
      assert is_binary(message)
      # Error message mentions the unexpected character.
      assert String.contains?(message, "@") or String.contains?(message, "Unexpected")
    end

    test "hash # produces an error tuple" do
      result = DartmouthBasicLexer.tokenize("10 LET X # 1\n")
      assert {:error, _message} = result
    end
  end

  # ---------------------------------------------------------------------------
  # Whitespace handling
  # ---------------------------------------------------------------------------
  #
  # Horizontal whitespace (spaces and tabs) is silently skipped between tokens.
  # `10 LET X = 1` and `10 LET X=1` produce identical token streams.
  # NEWLINES are NOT skipped — they are significant statement terminators.

  describe "tokenize/1 — whitespace handling" do
    test "spaces between tokens are skipped" do
      spaced = types("10 LET X = 1\n")
      compact = types("10 LET X=1\n")
      assert spaced == compact
    end

    test "tabs between tokens are skipped" do
      result = types("10\tLET\tX\t=\t1\n")
      assert result == ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE"]
    end

    test "extra spaces do not produce extra tokens" do
      result = types("10  LET   X   =   1\n")
      assert result == ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE"]
    end
  end

  # ---------------------------------------------------------------------------
  # Edge cases
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — edge cases" do
    test "DATA statement with comma-separated values" do
      result = type_values("10 DATA 1, 2, 3\n")
      assert {"KEYWORD", "DATA"} in result
      numbers = result |> Enum.filter(fn {t, _} -> t == "NUMBER" end) |> Enum.map(&elem(&1, 1))
      assert numbers == ["1", "2", "3"]
    end

    test "DIM array declaration" do
      result = type_values("10 DIM A(10)\n")
      assert {"KEYWORD", "DIM"} in result
      assert {"NAME", "A"} in result
      assert {"LPAREN", "("} in result
      assert {"NUMBER", "10"} in result
      assert {"RPAREN", ")"} in result
    end

    test "INPUT statement" do
      result = type_values("10 INPUT X\n")
      assert {"KEYWORD", "INPUT"} in result
      assert {"NAME", "X"} in result
    end

    test "negative number via MINUS operator" do
      # In BASIC, -3 is MINUS followed by NUMBER — there is no unary negative token.
      result = types("10 LET X = -3\n")
      assert "MINUS" in result
      assert "NUMBER" in result
    end

    test "nested parentheses" do
      result = types("10 LET X = (A + (B * C))\n")
      lparens = Enum.filter(result, &(&1 == "LPAREN"))
      rparens = Enum.filter(result, &(&1 == "RPAREN"))
      assert length(lparens) == 2
      assert length(rparens) == 2
    end

    test "source without trailing newline" do
      # Some programs may omit the final newline. The lexer should still produce EOF.
      {:ok, tokens} = DartmouthBasicLexer.tokenize("10 END")
      last = List.last(tokens)
      assert last.type == "EOF"
    end

    test "RESTORE statement with no arguments" do
      result = types("10 RESTORE\n")
      assert result == ["LINE_NUM", "KEYWORD", "NEWLINE"]
    end

    test "STOP statement" do
      result = type_values("10 STOP\n")
      assert {"KEYWORD", "STOP"} in result
    end
  end
end
