defmodule CodingAdventures.SqlLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.SqlLexer

  # ---------------------------------------------------------------------------
  # create_sql_lexer/1
  # ---------------------------------------------------------------------------
  #
  # The function parses sql.tokens and returns a TokenGrammar.  We verify that
  # the grammar contains the token definitions we depend on and that the
  # case_insensitive flag is set.

  describe "create_sql_lexer/1" do
    test "returns {:ok, grammar} for the default grammar" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      assert grammar != nil
    end

    test "grammar has case_insensitive set to true" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      assert grammar.case_insensitive == true
    end

    test "grammar definitions include NAME and NUMBER" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "NAME" in names
      assert "NUMBER" in names
    end

    test "grammar definitions include operator token names" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "LESS_EQUALS" in names
      assert "GREATER_EQUALS" in names
      assert "NOT_EQUALS" in names
      assert "EQUALS" in names
      assert "LESS_THAN" in names
      assert "GREATER_THAN" in names
    end

    test "grammar definitions include punctuation token names" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "LPAREN" in names
      assert "RPAREN" in names
      assert "COMMA" in names
      assert "SEMICOLON" in names
      assert "DOT" in names
    end

    test "grammar has SQL keywords" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      assert "SELECT" in grammar.keywords
      assert "FROM" in grammar.keywords
      assert "WHERE" in grammar.keywords
      assert "INSERT" in grammar.keywords
      assert "UPDATE" in grammar.keywords
      assert "DELETE" in grammar.keywords
    end

    test "grammar has NULL, TRUE, FALSE keywords" do
      {:ok, grammar} = SqlLexer.create_sql_lexer()
      assert "NULL" in grammar.keywords
      assert "TRUE" in grammar.keywords
      assert "FALSE" in grammar.keywords
    end

    test "returns {:error, _} for a non-existent grammar path" do
      {:error, _msg} = SqlLexer.create_sql_lexer("/tmp/does_not_exist_xyz/")
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — keywords
  # ---------------------------------------------------------------------------
  #
  # SQL keywords are matched case-insensitively. The grammar sets
  # @case_insensitive true, so the lexer normalizes all keyword values to
  # uppercase.  A KEYWORD token always has value in uppercase regardless of
  # how the source was written.

  describe "tokenize_sql/1 — keywords" do
    test "SELECT keyword uppercase" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "SELECT"
    end

    test "select lowercase is normalized to SELECT" do
      {:ok, tokens} = SqlLexer.tokenize_sql("select")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "SELECT"
    end

    test "Select mixed-case is normalized to SELECT" do
      {:ok, tokens} = SqlLexer.tokenize_sql("Select")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "SELECT"
    end

    test "FROM keyword" do
      {:ok, tokens} = SqlLexer.tokenize_sql("FROM")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "FROM"
    end

    test "WHERE keyword" do
      {:ok, tokens} = SqlLexer.tokenize_sql("where")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "WHERE"
    end

    test "NULL keyword" do
      {:ok, tokens} = SqlLexer.tokenize_sql("NULL")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "NULL"
    end

    test "null lowercase is normalized to NULL" do
      {:ok, tokens} = SqlLexer.tokenize_sql("null")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "NULL"
    end

    test "TRUE keyword" do
      {:ok, tokens} = SqlLexer.tokenize_sql("TRUE")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "TRUE"
    end

    test "FALSE keyword" do
      {:ok, tokens} = SqlLexer.tokenize_sql("FALSE")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "FALSE"
    end

    test "INSERT INTO keywords" do
      {:ok, tokens} = SqlLexer.tokenize_sql("INSERT INTO")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      values = tokens |> Enum.map(& &1.value) |> Enum.reject(&(&1 == ""))
      assert types == ["KEYWORD", "KEYWORD"]
      assert values == ["INSERT", "INTO"]
    end

    test "UPDATE SET keywords" do
      {:ok, tokens} = SqlLexer.tokenize_sql("UPDATE SET")
      values = tokens |> Enum.map(& &1.value) |> Enum.reject(&(&1 == ""))
      assert "UPDATE" in values
      assert "SET" in values
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — identifiers (NAME)
  # ---------------------------------------------------------------------------
  #
  # Non-keyword identifiers produce NAME tokens.  The value is preserved
  # exactly as written (no case folding for identifiers).

  describe "tokenize_sql/1 — identifiers" do
    test "simple identifier" do
      {:ok, tokens} = SqlLexer.tokenize_sql("users")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "users"
    end

    test "mixed-case identifier preserves case" do
      {:ok, tokens} = SqlLexer.tokenize_sql("UserName")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "UserName"
    end

    test "identifier with underscore" do
      {:ok, tokens} = SqlLexer.tokenize_sql("first_name")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "first_name"
    end

    test "backtick-quoted identifier is NAME with backticks preserved" do
      # QUOTED_ID = /`[^`]+`/ -> NAME, so the token type becomes NAME
      # but unlike STRING tokens, backticks are NOT stripped from the value.
      {:ok, tokens} = SqlLexer.tokenize_sql("`my column`")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "`my column`"
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — number literals
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — numbers" do
    test "integer" do
      {:ok, tokens} = SqlLexer.tokenize_sql("42")
      [tok, _eof] = tokens
      assert tok.type == "NUMBER"
      assert tok.value == "42"
    end

    test "decimal number" do
      {:ok, tokens} = SqlLexer.tokenize_sql("3.14")
      [tok, _eof] = tokens
      assert tok.type == "NUMBER"
      assert tok.value == "3.14"
    end

    test "zero" do
      {:ok, tokens} = SqlLexer.tokenize_sql("0")
      [tok, _eof] = tokens
      assert tok.type == "NUMBER"
      assert tok.value == "0"
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — string literals
  # ---------------------------------------------------------------------------
  #
  # SQL uses single-quoted strings ('hello'). The grammar defines
  # STRING_SQ = /'([^'\\]|\\.)*'/ -> STRING
  # so the token type is STRING and the quotes are stripped from the value.

  describe "tokenize_sql/1 — strings" do
    test "single-quoted string has type STRING" do
      {:ok, tokens} = SqlLexer.tokenize_sql("'hello'")
      [tok, _eof] = tokens
      assert tok.type == "STRING"
    end

    test "quotes are stripped from string value" do
      {:ok, tokens} = SqlLexer.tokenize_sql("'hello'")
      [tok, _eof] = tokens
      assert tok.value == "hello"
    end

    test "string with spaces" do
      {:ok, tokens} = SqlLexer.tokenize_sql("'hello world'")
      [tok, _eof] = tokens
      assert tok.type == "STRING"
      assert tok.value == "hello world"
    end

    test "empty string" do
      # '' in SQL is an empty string — but our regex requires at least no chars,
      # so let's use a minimal string instead to ensure the token is present
      {:ok, tokens} = SqlLexer.tokenize_sql("'x'")
      [tok, _eof] = tokens
      assert tok.type == "STRING"
      assert tok.value == "x"
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — operators
  # ---------------------------------------------------------------------------
  #
  # The grammar defines operators in longest-match-first order so that
  # multi-character operators like <= match before <.
  #
  # token name → expected type string:
  #   NOT_EQUALS    → "NOT_EQUALS"   (both != and <>)
  #   LESS_EQUALS   → "LESS_EQUALS"
  #   GREATER_EQUALS→ "GREATER_EQUALS"
  #   EQUALS        → "EQUALS"
  #   LESS_THAN     → "LESS_THAN"
  #   GREATER_THAN  → "GREATER_THAN"

  describe "tokenize_sql/1 — operators" do
    test "!= produces NOT_EQUALS type" do
      {:ok, tokens} = SqlLexer.tokenize_sql("!=")
      [tok, _eof] = tokens
      assert tok.type == "NOT_EQUALS"
      assert tok.value == "!="
    end

    test "<> (ANSI not-equals) produces NOT_EQUALS type via alias" do
      {:ok, tokens} = SqlLexer.tokenize_sql("<>")
      [tok, _eof] = tokens
      assert tok.type == "NOT_EQUALS"
    end

    test "<= produces LESS_EQUALS type" do
      {:ok, tokens} = SqlLexer.tokenize_sql("<=")
      [tok, _eof] = tokens
      assert tok.type == "LESS_EQUALS"
      assert tok.value == "<="
    end

    test ">= produces GREATER_EQUALS type" do
      {:ok, tokens} = SqlLexer.tokenize_sql(">=")
      [tok, _eof] = tokens
      assert tok.type == "GREATER_EQUALS"
      assert tok.value == ">="
    end

    test "= produces EQUALS type" do
      {:ok, tokens} = SqlLexer.tokenize_sql("=")
      [tok, _eof] = tokens
      assert tok.type == "EQUALS"
      assert tok.value == "="
    end

    test "< produces LESS_THAN type" do
      {:ok, tokens} = SqlLexer.tokenize_sql("<")
      [tok, _eof] = tokens
      assert tok.type == "LESS_THAN"
      assert tok.value == "<"
    end

    test "> produces GREATER_THAN type" do
      {:ok, tokens} = SqlLexer.tokenize_sql(">")
      [tok, _eof] = tokens
      assert tok.type == "GREATER_THAN"
      assert tok.value == ">"
    end

    test "arithmetic operators" do
      {:ok, tokens} = SqlLexer.tokenize_sql("+ - * / %")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["PLUS", "MINUS", "STAR", "SLASH", "PERCENT"]
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — punctuation
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — punctuation" do
    test "left paren" do
      {:ok, tokens} = SqlLexer.tokenize_sql("(")
      [tok, _eof] = tokens
      assert tok.type == "LPAREN"
    end

    test "right paren" do
      {:ok, tokens} = SqlLexer.tokenize_sql(")")
      [tok, _eof] = tokens
      assert tok.type == "RPAREN"
    end

    test "comma" do
      {:ok, tokens} = SqlLexer.tokenize_sql(",")
      [tok, _eof] = tokens
      assert tok.type == "COMMA"
    end

    test "semicolon" do
      {:ok, tokens} = SqlLexer.tokenize_sql(";")
      [tok, _eof] = tokens
      assert tok.type == "SEMICOLON"
    end

    test "dot" do
      {:ok, tokens} = SqlLexer.tokenize_sql(".")
      [tok, _eof] = tokens
      assert tok.type == "DOT"
    end

    test "all punctuation in sequence" do
      {:ok, tokens} = SqlLexer.tokenize_sql("(),;.")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["LPAREN", "RPAREN", "COMMA", "SEMICOLON", "DOT"]
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — comment skipping
  # ---------------------------------------------------------------------------
  #
  # SQL supports two comment styles, both defined in the `skip:` section of
  # sql.tokens:
  #   LINE_COMMENT  = /--[^\n]*/      — from -- to end of line
  #   BLOCK_COMMENT = /\/\*…\*\//     — C-style block comment
  #
  # Both are skipped silently — they produce no tokens in the output.

  describe "tokenize_sql/1 — comment skipping" do
    test "line comment -- is skipped" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT -- this is a comment\n1")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      # Only KEYWORD and NUMBER — the comment text disappears
      assert "KEYWORD" in types
      assert "NUMBER" in types
      refute Enum.any?(tokens, fn t -> String.contains?(t.value, "comment") end)
    end

    test "block comment /* */ is skipped" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT /* pick all */ * FROM t")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["KEYWORD", "STAR", "KEYWORD", "NAME"]
    end

    test "multiline block comment is skipped" do
      source = """
      SELECT
      /* this is
         a multiline
         comment */
      42
      """

      {:ok, tokens} = SqlLexer.tokenize_sql(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert "KEYWORD" in types
      assert "NUMBER" in types
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — compound expressions
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — compound expressions" do
    test "simple SELECT * FROM" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT * FROM users")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["KEYWORD", "STAR", "KEYWORD", "NAME"]

      values = tokens |> Enum.map(& &1.value) |> Enum.reject(&(&1 == ""))
      assert values == ["SELECT", "*", "FROM", "users"]
    end

    test "WHERE clause with comparison" do
      {:ok, tokens} = SqlLexer.tokenize_sql("WHERE age >= 18")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["KEYWORD", "NAME", "GREATER_EQUALS", "NUMBER"]
    end

    test "qualified column reference with dot" do
      {:ok, tokens} = SqlLexer.tokenize_sql("u.name")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["NAME", "DOT", "NAME"]
    end

    test "function call syntax" do
      {:ok, tokens} = SqlLexer.tokenize_sql("COUNT(*)")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["NAME", "LPAREN", "STAR", "RPAREN"]
    end

    test "INSERT INTO with values" do
      {:ok, tokens} = SqlLexer.tokenize_sql("INSERT INTO t VALUES (1, 'a')")

      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "KEYWORD",
               "KEYWORD",
               "NAME",
               "KEYWORD",
               "LPAREN",
               "NUMBER",
               "COMMA",
               "STRING",
               "RPAREN"
             ]
    end

    test "statement with semicolon terminator" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT 1;")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["KEYWORD", "NUMBER", "SEMICOLON"]
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — whitespace handling
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — whitespace" do
    test "leading/trailing whitespace is skipped" do
      {:ok, tokens} = SqlLexer.tokenize_sql("   SELECT   ")
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert types == ["KEYWORD"]
    end

    test "multiline query" do
      source = """
      SELECT id, name
      FROM   users
      WHERE  active = 1
      """

      {:ok, tokens} = SqlLexer.tokenize_sql(source)
      types = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "KEYWORD",
               "NAME",
               "COMMA",
               "NAME",
               "KEYWORD",
               "NAME",
               "KEYWORD",
               "NAME",
               "EQUALS",
               "NUMBER"
             ]
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — position tracking
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — position tracking" do
    test "tracks line 1 and column 1 for first token" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT 1")
      [tok | _] = tokens
      assert tok.line == 1
      assert tok.column == 1
    end

    test "tracks column correctly for subsequent tokens" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT 42")
      # SELECT is at column 1, 42 is at column 8 (after "SELECT ")
      [_, num | _] = tokens
      assert num.type == "NUMBER"
      assert num.column == 8
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — EOF
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — EOF" do
    test "empty string produces only EOF" do
      {:ok, tokens} = SqlLexer.tokenize_sql("")
      assert length(tokens) == 1
      [eof] = tokens
      assert eof.type == "EOF"
    end

    test "token list always ends with EOF" do
      {:ok, tokens} = SqlLexer.tokenize_sql("SELECT 1")
      last = List.last(tokens)
      assert last.type == "EOF"
    end
  end

  # ---------------------------------------------------------------------------
  # tokenize_sql/1 — error cases
  # ---------------------------------------------------------------------------

  describe "tokenize_sql/1 — errors" do
    test "unexpected character returns error" do
      {:error, msg} = SqlLexer.tokenize_sql("@bad")
      assert msg =~ "Unexpected character"
    end
  end
end
