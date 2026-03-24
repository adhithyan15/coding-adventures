defmodule CodingAdventures.Lexer.GrammarLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.Lexer.GrammarLexer
  alias CodingAdventures.Lexer.GrammarLexer.LexerContext
  alias CodingAdventures.GrammarTools.TokenGrammar

  # Helper to create a simple grammar for testing
  defp simple_grammar do
    {:ok, g} =
      TokenGrammar.parse("""
      NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
      NUMBER = /[0-9]+/
      PLUS   = "+"
      MINUS  = "-"
      """)

    g
  end

  defp json_grammar do
    grammar_dir =
      Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
      |> Path.expand()

    {:ok, g} = TokenGrammar.parse(File.read!(Path.join(grammar_dir, "json.tokens")))
    g
  end

  # Helper to create a grammar with pattern groups for testing.
  #
  # This simulates a simplified XML-like grammar:
  # - Default group: TEXT and OPEN_TAG
  # - tag group: TAG_NAME, EQUALS, VALUE, TAG_CLOSE
  #
  # The grammar uses skip patterns for whitespace and no keywords.
  defp group_grammar do
    {:ok, g} =
      TokenGrammar.parse("""
      escapes: none

      skip:
        WS = /[ \\t\\r\\n]+/

      TEXT      = /[^<]+/
      OPEN_TAG  = "<"

      group tag:
        TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
        EQUALS    = "="
        VALUE     = /"[^"]*"/
        TAG_CLOSE = ">"
      """)

    g
  end

  describe "tokenize/2 — basic tokens" do
    test "tokenizes a simple expression" do
      {:ok, tokens} = GrammarLexer.tokenize("x + 42", simple_grammar())

      types = Enum.map(tokens, & &1.type)
      values = Enum.map(tokens, & &1.value)

      assert types == ["NAME", "PLUS", "NUMBER", "EOF"]
      assert values == ["x", "+", "42", ""]
    end

    test "tokenizes identifiers" do
      {:ok, tokens} = GrammarLexer.tokenize("foo bar_baz", simple_grammar())
      types = Enum.map(tokens, & &1.type)
      assert types == ["NAME", "NAME", "EOF"]
    end

    test "tokenizes numbers" do
      {:ok, tokens} = GrammarLexer.tokenize("123 456", simple_grammar())
      values = Enum.map(tokens, & &1.value)
      assert values == ["123", "456", ""]
    end

    test "returns EOF for empty input" do
      {:ok, tokens} = GrammarLexer.tokenize("", simple_grammar())
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end

    test "returns EOF for whitespace-only input" do
      {:ok, tokens} = GrammarLexer.tokenize("   ", simple_grammar())
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end
  end

  describe "tokenize/2 — position tracking" do
    test "tracks line and column" do
      {:ok, tokens} = GrammarLexer.tokenize("x + 42", simple_grammar())

      [name, plus, number, _eof] = tokens
      assert name.line == 1
      assert name.column == 1
      assert plus.line == 1
      assert plus.column == 3
      assert number.line == 1
      assert number.column == 5
    end

    test "tracks lines across newlines" do
      {:ok, tokens} = GrammarLexer.tokenize("x\n42", simple_grammar())

      [name, _newline, number, _eof] = tokens
      assert name.line == 1
      assert number.line == 2
      assert number.column == 1
    end
  end

  describe "tokenize/2 — newlines" do
    test "emits NEWLINE tokens" do
      {:ok, tokens} = GrammarLexer.tokenize("x\ny", simple_grammar())
      types = Enum.map(tokens, & &1.type)
      assert types == ["NAME", "NEWLINE", "NAME", "EOF"]
    end
  end

  describe "tokenize/2 — skip patterns" do
    test "skip patterns consume whitespace silently" do
      {:ok, g} =
        TokenGrammar.parse("""
        NUMBER = /[0-9]+/
        PLUS = "+"

        skip:
          WHITESPACE = /[ \\t\\r\\n]+/
        """)

      {:ok, tokens} = GrammarLexer.tokenize("1 + 2\n", g)
      types = Enum.map(tokens, & &1.type)
      # With skip patterns consuming newlines, no NEWLINE token is emitted
      assert types == ["NUMBER", "PLUS", "NUMBER", "EOF"]
    end
  end

  describe "tokenize/2 — keywords" do
    test "reclassifies NAME as KEYWORD" do
      {:ok, g} =
        TokenGrammar.parse("""
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
        NUMBER = /[0-9]+/

        keywords:
          if
          else
        """)

      {:ok, tokens} = GrammarLexer.tokenize("if x else y", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["KEYWORD", "NAME", "KEYWORD", "NAME", "EOF"]
    end
  end

  # ---------------------------------------------------------------------------
  # Case-Insensitive Keyword Matching
  # ---------------------------------------------------------------------------
  #
  # When a grammar declares `case_insensitive: true`, keyword matching ignores
  # case and the emitted KEYWORD value is normalized to uppercase. This is the
  # "normalize on both sides" strategy:
  #
  # - The keyword set stores keywords as uppercase (done at grammar init time).
  # - At match time, the matched value is uppercased before the set lookup.
  # - When a KEYWORD token is emitted, its value is String.upcase(original).
  #
  # This means "select", "SELECT", and "Select" all produce:
  #   %Token{type: "KEYWORD", value: "SELECT", ...}
  #
  # Non-keyword identifiers (NAMEs) are NOT affected — their value is left as-is.

  describe "tokenize/2 — case-insensitive keywords" do
    # Helper: a grammar with `case_insensitive: true` and keyword `select`.
    defp ci_grammar do
      {:ok, g} =
        TokenGrammar.parse("""
        # @case_insensitive true

        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

        keywords:
          select
        """)

      g
    end

    test "lowercase keyword → KEYWORD with uppercase value" do
      # "select" matches the keyword set (stored as "SELECT") because
      # String.upcase("select") == "SELECT". The emitted value is "SELECT".
      {:ok, tokens} = GrammarLexer.tokenize("select", ci_grammar())
      [token, _eof] = tokens
      assert token.type == "KEYWORD"
      assert token.value == "SELECT"
    end

    test "uppercase keyword → KEYWORD with uppercase value" do
      # "SELECT" also matches. The normalized lookup and normalized emit
      # both produce "SELECT", so the output is identical to the lowercase case.
      {:ok, tokens} = GrammarLexer.tokenize("SELECT", ci_grammar())
      [token, _eof] = tokens
      assert token.type == "KEYWORD"
      assert token.value == "SELECT"
    end

    test "mixed-case keyword → KEYWORD with uppercase value" do
      # "Select" is neither lowercase nor uppercase but still matches,
      # because String.upcase("Select") == "SELECT" is in the keyword set.
      {:ok, tokens} = GrammarLexer.tokenize("Select", ci_grammar())
      [token, _eof] = tokens
      assert token.type == "KEYWORD"
      assert token.value == "SELECT"
    end

    test "non-keyword identifier → NAME with original case preserved" do
      # Non-keyword identifiers are not uppercased. The NAME value is
      # emitted exactly as it appeared in the source — case insensitivity
      # only applies to the keyword set membership check.
      {:ok, tokens} = GrammarLexer.tokenize("myVar", ci_grammar())
      [token, _eof] = tokens
      assert token.type == "NAME"
      assert token.value == "myVar"
    end

    test "keyword and non-keyword in the same input" do
      # Verify that keyword normalization and NAME passthrough work together
      # in a single tokenization pass.
      {:ok, tokens} = GrammarLexer.tokenize("select myTable", ci_grammar())
      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"KEYWORD", "SELECT"},
               {"NAME", "myTable"}
             ]
    end

    test "case_insensitive: false (default) preserves case-sensitive behavior" do
      # The default grammar is case-sensitive. "Select" is not in the keyword
      # set (which stores "select" as-is), so it becomes a NAME token.
      {:ok, g} =
        TokenGrammar.parse("""
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

        keywords:
          select
        """)

      {:ok, tokens} = GrammarLexer.tokenize("Select", g)
      [token, _eof] = tokens
      # "Select" != "select" so it is treated as a plain identifier
      assert token.type == "NAME"
      assert token.value == "Select"
    end

    test "case_insensitive: false (default) still matches exact keyword" do
      # In case-sensitive mode, "select" (exact match) is still a KEYWORD.
      {:ok, g} =
        TokenGrammar.parse("""
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

        keywords:
          select
        """)

      {:ok, tokens} = GrammarLexer.tokenize("select", g)
      [token, _eof] = tokens
      assert token.type == "KEYWORD"
      # Value is NOT uppercased — case_insensitive is false
      assert token.value == "select"
    end
  end

  describe "tokenize/2 — reserved keywords" do
    test "reserved keywords produce errors" do
      {:ok, g} =
        TokenGrammar.parse("""
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

        reserved:
          class
        """)

      {:error, msg} = GrammarLexer.tokenize("class", g)
      assert msg =~ "Reserved keyword 'class'"
    end
  end

  describe "tokenize/2 — aliases" do
    test "uses alias as token type" do
      {:ok, g} = TokenGrammar.parse(~s(STRING_DQ = /"[^"]*"/ -> STRING))
      {:ok, tokens} = GrammarLexer.tokenize(~s("hello"), g)
      [token, _eof] = tokens
      assert token.type == "STRING"
      assert token.value == "hello"
    end
  end

  describe "tokenize/2 — string escape processing" do
    test "processes standard escapes via JSON grammar" do
      g = json_grammar()
      source = ~S("hello\nworld")
      {:ok, tokens} = GrammarLexer.tokenize(source, g)
      [token, _eof] = tokens
      assert token.type == "STRING"
      assert token.value == "hello\nworld"
    end

    test "processes unicode escapes via JSON grammar" do
      g = json_grammar()
      source = ~S("caf\u00E9")
      {:ok, tokens} = GrammarLexer.tokenize(source, g)
      [token, _eof] = tokens
      assert token.value == "caf\u00E9"
    end
  end

  describe "tokenize/2 — error cases" do
    test "unexpected character" do
      {:error, msg} = GrammarLexer.tokenize("x @ y", simple_grammar())
      assert msg =~ "Unexpected character"
      assert msg =~ "@"
    end
  end

  describe "tokenize/2 — JSON grammar integration" do
    test "tokenizes JSON primitives" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("42", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["NUMBER", "EOF"]
    end

    test "tokenizes JSON boolean and null" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("true false null", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["TRUE", "FALSE", "NULL", "EOF"]
    end

    test "tokenizes JSON string" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize(~s("hello world"), g)
      [token, _eof] = tokens
      assert token.type == "STRING"
      assert token.value == "hello world"
    end

    test "tokenizes JSON structural tokens" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("{[]:,}", g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["LBRACE", "LBRACKET", "RBRACKET", "COLON", "COMMA", "RBRACE", "EOF"]
    end

    test "tokenizes a JSON object" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize(~s({"key": 42}), g)
      types = Enum.map(tokens, & &1.type)
      assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE", "EOF"]
    end

    test "tokenizes JSON with whitespace" do
      g = json_grammar()

      {:ok, tokens} =
        GrammarLexer.tokenize("""
        {
          "name": "Alice",
          "age": 30
        }
        """, g)

      types = Enum.map(tokens, & &1.type) |> Enum.reject(&(&1 == "EOF"))

      assert types == [
               "LBRACE",
               "STRING", "COLON", "STRING", "COMMA",
               "STRING", "COLON", "NUMBER",
               "RBRACE"
             ]
    end

    test "tokenizes negative numbers" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("-42", g)
      [token, _eof] = tokens
      assert token.type == "NUMBER"
      assert token.value == "-42"
    end

    test "tokenizes decimal and exponent numbers" do
      g = json_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("3.14 1e10 2.5E-3", g)
      values = tokens |> Enum.reject(&(&1.type == "EOF")) |> Enum.map(& &1.value)
      assert values == ["3.14", "1e10", "2.5E-3"]
    end
  end

  describe "process_escapes/1" do
    test "handles \\n, \\t, \\r" do
      assert GrammarLexer.process_escapes("a\\nb\\tc\\rd") == "a\nb\tc\rd"
    end

    test "handles \\b, \\f" do
      assert GrammarLexer.process_escapes("a\\bb\\fc") == "a\bb\fc"
    end

    test "handles \\\\ and \\\"" do
      assert GrammarLexer.process_escapes("a\\\\b\\\"c") == "a\\b\"c"
    end

    test "handles \\/" do
      assert GrammarLexer.process_escapes("a\\/b") == "a/b"
    end

    test "handles \\uXXXX" do
      assert GrammarLexer.process_escapes("caf\\u00E9") == "caf\u00E9"
    end

    test "passes through unknown escapes" do
      assert GrammarLexer.process_escapes("\\x") == "x"
    end

    test "no escapes" do
      assert GrammarLexer.process_escapes("hello") == "hello"
    end
  end

  # ---------------------------------------------------------------------------
  # LexerContext — Unit Tests
  # ---------------------------------------------------------------------------
  #
  # These tests verify the LexerContext struct and its peek/peek_str functions.
  # Unlike the Python tests which test mutating methods, these test the
  # read-only struct and its helper functions.

  describe "LexerContext — peek" do
    test "peek reads characters from the source after the token" do
      ctx = %LexerContext{
        active_group: "default",
        group_stack_depth: 1,
        source: "hello",
        pos_after_token: 3,
        available_groups: ["default"]
      }

      # Position 3 = after "hel", so peek(1) = "l", peek(2) = "o"
      assert LexerContext.peek(ctx, 1) == "l"
      assert LexerContext.peek(ctx, 2) == "o"
      assert LexerContext.peek(ctx, 3) == ""  # past EOF
    end

    test "peek_str reads a substring after the token" do
      ctx = %LexerContext{
        active_group: "default",
        group_stack_depth: 1,
        source: "hello world",
        pos_after_token: 5,
        available_groups: ["default"]
      }

      assert LexerContext.peek_str(ctx, 6) == " world"
    end

    test "peek_str truncates at end of source" do
      ctx = %LexerContext{
        active_group: "default",
        group_stack_depth: 1,
        source: "hi",
        pos_after_token: 1,
        available_groups: ["default"]
      }

      assert LexerContext.peek_str(ctx, 100) == "i"
    end
  end

  # ---------------------------------------------------------------------------
  # Pattern Group Tokenization — Integration Tests
  # ---------------------------------------------------------------------------
  #
  # These tests verify that the lexer correctly switches between pattern
  # groups based on callback actions, producing the right tokens in the
  # right order. They are ported from the Python TestPatternGroupTokenization
  # class, adapted for Elixir's functional callback style.

  describe "pattern groups — no callback" do
    test "without a callback, only default group patterns are used" do
      grammar = group_grammar()
      {:ok, tokens} = GrammarLexer.tokenize("hello", grammar)
      # TEXT pattern matches in default group
      assert hd(tokens).type == "TEXT"
      assert hd(tokens).value == "hello"
    end
  end

  describe "pattern groups — push/pop" do
    test "callback can push/pop groups to switch pattern sets" do
      # Simulates: <div> where < triggers push("tag"), > triggers pop().
      grammar = group_grammar()

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" -> [{:push_group, "tag"}]
          "TAG_CLOSE" -> [:pop_group]
          _ -> []
        end
      end

      {:ok, tokens} = GrammarLexer.tokenize("<div>hello", grammar, on_token: callback)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"OPEN_TAG", "<"},
               {"TAG_NAME", "div"},
               {"TAG_CLOSE", ">"},
               {"TEXT", "hello"}
             ]
    end

    test "callback handles tag with attributes" do
      # Simulates: <div class="main"> where the tag group lexes
      # TAG_NAME, EQUALS, and VALUE tokens.
      grammar = group_grammar()

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" -> [{:push_group, "tag"}]
          "TAG_CLOSE" -> [:pop_group]
          _ -> []
        end
      end

      {:ok, tokens} =
        GrammarLexer.tokenize(~s(<div class="main">), grammar, on_token: callback)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"OPEN_TAG", "<"},
               {"TAG_NAME", "div"},
               {"TAG_NAME", "class"},
               {"EQUALS", "="},
               {"VALUE", ~s("main")},
               {"TAG_CLOSE", ">"}
             ]
    end
  end

  describe "pattern groups — nested tags" do
    test "group stack handles nested structures" do
      # Simulates: <a>text<b>inner</b></a> with push/pop on < and >.
      # Need a grammar with CLOSE_TAG_START for </
      {:ok, grammar} =
        TokenGrammar.parse("""
        escapes: none

        skip:
          WS = /[ \\t\\r\\n]+/

        TEXT             = /[^<]+/
        CLOSE_TAG_START  = "</"
        OPEN_TAG         = "<"

        group tag:
          TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
          TAG_CLOSE = ">"
          SLASH     = "/"
        """)

      callback = fn token, _ctx ->
        case token.type do
          type when type in ["OPEN_TAG", "CLOSE_TAG_START"] ->
            [{:push_group, "tag"}]

          "TAG_CLOSE" ->
            [:pop_group]

          _ ->
            []
        end
      end

      {:ok, tokens} =
        GrammarLexer.tokenize("<a>text<b>inner</b></a>", grammar, on_token: callback)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"OPEN_TAG", "<"},
               {"TAG_NAME", "a"},
               {"TAG_CLOSE", ">"},
               {"TEXT", "text"},
               {"OPEN_TAG", "<"},
               {"TAG_NAME", "b"},
               {"TAG_CLOSE", ">"},
               {"TEXT", "inner"},
               {"CLOSE_TAG_START", "</"},
               {"TAG_NAME", "b"},
               {"TAG_CLOSE", ">"},
               {"CLOSE_TAG_START", "</"},
               {"TAG_NAME", "a"},
               {"TAG_CLOSE", ">"}
             ]
    end
  end

  describe "pattern groups — suppress" do
    test "callback can suppress tokens (remove from output)" do
      grammar = group_grammar()

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" -> [:suppress]
          _ -> []
        end
      end

      {:ok, tokens} = GrammarLexer.tokenize("<hello", grammar, on_token: callback)

      types =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(& &1.type)

      # OPEN_TAG was suppressed, only TEXT remains
      assert types == ["TEXT"]
    end
  end

  describe "pattern groups — emit synthetic tokens" do
    test "callback can emit synthetic tokens after the current one" do
      grammar = group_grammar()

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" ->
            [{:emit, %Token{type: "MARKER", value: "[start]", line: token.line, column: token.column}}]

          _ ->
            []
        end
      end

      {:ok, tokens} = GrammarLexer.tokenize("<hello", grammar, on_token: callback)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"OPEN_TAG", "<"},
               {"MARKER", "[start]"},
               {"TEXT", "hello"}
             ]
    end
  end

  describe "pattern groups — suppress + emit (token replacement)" do
    test "suppress + emit = token replacement" do
      # The current token is swallowed, but emitted tokens still output.
      # This enables token rewriting (e.g., replacing OPEN_TAG with a
      # different token type).
      grammar = group_grammar()

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" ->
            [:suppress, {:emit, %Token{type: "REPLACED", value: "<", line: token.line, column: token.column}}]

          _ ->
            []
        end
      end

      {:ok, tokens} = GrammarLexer.tokenize("<hello", grammar, on_token: callback)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"REPLACED", "<"},
               {"TEXT", "hello"}
             ]
    end
  end

  describe "pattern groups — pop at bottom" do
    test "popping when only default remains is a no-op (no crash)" do
      grammar = group_grammar()

      callback = fn _token, _ctx ->
        [:pop_group]
      end

      {:ok, tokens} = GrammarLexer.tokenize("hello", grammar, on_token: callback)

      # Should still produce TEXT token without crashing
      assert hd(tokens).type == "TEXT"
    end
  end

  describe "pattern groups — set_skip_enabled" do
    test "callback can disable skip patterns for significant whitespace" do
      # Grammar with a group that captures whitespace as a token.
      # When skip is disabled, whitespace that would normally be consumed
      # silently instead remains for the active group's patterns to match.
      {:ok, grammar} =
        TokenGrammar.parse("""
        escapes: none

        skip:
          WS = /[ \\t]+/

        TEXT      = /[^<]+/
        START     = "<!"

        group raw:
          RAW_TEXT = /[^>]+/
          END      = ">"
        """)

      callback = fn token, _ctx ->
        case token.type do
          "START" ->
            [{:push_group, "raw"}, {:set_skip_enabled, false}]

          "END" ->
            [:pop_group, {:set_skip_enabled, true}]

          _ ->
            []
        end
      end

      # The space in "hello world" should be preserved (not skipped)
      # because skip is disabled while in the raw group.
      {:ok, tokens} =
        GrammarLexer.tokenize("<! hello world >after", grammar, on_token: callback)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"START", "<!"},
               {"RAW_TEXT", " hello world "},
               {"END", ">"},
               {"TEXT", "after"}
             ]
    end
  end

  describe "pattern groups — backward compatibility" do
    test "a grammar with no groups behaves identically to before" do
      # This verifies backward compatibility: no groups + no callback
      # = same behavior as the original GrammarLexer.
      {:ok, grammar} =
        TokenGrammar.parse("""
        NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
        NUMBER = /[0-9]+/
        PLUS   = "+"
        """)

      {:ok, tokens} = GrammarLexer.tokenize("x + 1", grammar)

      type_value_pairs =
        tokens
        |> Enum.reject(&(&1.type in ["NEWLINE", "EOF"]))
        |> Enum.map(&{&1.type, &1.value})

      assert type_value_pairs == [
               {"NAME", "x"},
               {"PLUS", "+"},
               {"NUMBER", "1"}
             ]
    end
  end

  describe "pattern groups — callback lifecycle" do
    test "callback receives correct context fields" do
      grammar = group_grammar()

      # We'll collect context info during tokenization to verify it
      test_pid = self()

      callback = fn token, ctx ->
        if token.type == "OPEN_TAG" do
          send(test_pid, {:context, ctx})
          [{:push_group, "tag"}]
        else
          []
        end
      end

      {:ok, _tokens} = GrammarLexer.tokenize("<hello", grammar, on_token: callback)

      assert_received {:context, ctx}
      assert ctx.active_group == "default"
      assert ctx.group_stack_depth == 1
      assert is_binary(ctx.source)
      assert is_integer(ctx.pos_after_token)
      assert "default" in ctx.available_groups
      assert "tag" in ctx.available_groups
    end

    test "callback is not invoked for EOF" do
      grammar = group_grammar()
      test_pid = self()

      callback = fn token, _ctx ->
        send(test_pid, {:token, token.type})
        []
      end

      {:ok, _tokens} = GrammarLexer.tokenize("hello", grammar, on_token: callback)

      # Should receive TEXT but not EOF
      assert_received {:token, "TEXT"}
      refute_received {:token, "EOF"}
    end

    test "push_group with unknown group raises error" do
      grammar = group_grammar()

      callback = fn _token, _ctx ->
        [{:push_group, "nonexistent"}]
      end

      assert_raise ArgumentError, ~r/Unknown pattern group/, fn ->
        GrammarLexer.tokenize("hello", grammar, on_token: callback)
      end
    end

    test "multiple push/pop in one callback are applied in order" do
      grammar = group_grammar()

      callback = fn token, _ctx ->
        case token.type do
          "OPEN_TAG" ->
            # Push tag twice — stacking
            [{:push_group, "tag"}, {:push_group, "tag"}]

          _ ->
            []
        end
      end

      # Should not crash with double-push
      {:ok, tokens} = GrammarLexer.tokenize("<div", grammar, on_token: callback)
      assert Enum.any?(tokens, &(&1.type == "TAG_NAME"))
    end
  end
end
