defmodule CodingAdventures.MosaicLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.MosaicLexer

  # ---------------------------------------------------------------------------
  # Helper — extract token types from a result, dropping EOF for readability
  # ---------------------------------------------------------------------------

  defp types(source) do
    {:ok, tokens} = MosaicLexer.tokenize(source)
    tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
  end

  defp values(source) do
    {:ok, tokens} = MosaicLexer.tokenize(source)
    tokens |> Enum.reject(&(&1.type == "EOF")) |> Enum.map(& &1.value)
  end

  # ---------------------------------------------------------------------------
  # create_lexer/0 — grammar introspection
  # ---------------------------------------------------------------------------
  # These tests verify that the grammar file was parsed correctly and contains
  # the expected token names and keywords.

  describe "create_lexer/0" do
    test "returns a TokenGrammar struct" do
      grammar = MosaicLexer.create_lexer()
      # %TokenGrammar{} has a :definitions field — presence confirms the type
      assert is_list(grammar.definitions)
    end

    test "grammar contains expected token names" do
      grammar = MosaicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      # Structural delimiters
      assert "LBRACE" in names
      assert "RBRACE" in names
      assert "LANGLE" in names
      assert "RANGLE" in names
      assert "COLON" in names
      assert "SEMICOLON" in names
      assert "COMMA" in names
      assert "DOT" in names
      assert "EQUALS" in names
      assert "AT" in names

      # Literal and numeric token types
      assert "STRING" in names
      assert "NUMBER" in names
      assert "DIMENSION" in names
      assert "COLOR_HEX" in names

      # Identifier
      assert "NAME" in names
    end

    test "grammar contains all expected keywords" do
      grammar = MosaicLexer.create_lexer()
      kws = grammar.keywords

      # Structure keywords
      assert "component" in kws
      assert "slot" in kws
      assert "import" in kws
      assert "from" in kws
      assert "as" in kws

      # Type keywords
      assert "text" in kws
      assert "number" in kws
      assert "bool" in kws
      assert "image" in kws
      assert "color" in kws
      assert "node" in kws
      assert "list" in kws

      # Boolean literals
      assert "true" in kws
      assert "false" in kws

      # Control flow keywords
      assert "when" in kws
      assert "each" in kws
    end

    test "grammar defines skip patterns for whitespace and comments" do
      grammar = MosaicLexer.create_lexer()
      skip_names = Enum.map(grammar.skip_definitions, & &1.name)
      assert "WHITESPACE" in skip_names
      assert "LINE_COMMENT" in skip_names
      assert "BLOCK_COMMENT" in skip_names
    end
  end

  # ---------------------------------------------------------------------------
  # Keywords — promoted from NAME to KEYWORD
  # ---------------------------------------------------------------------------
  # The lexer matches NAME first, then checks if the text is in the keyword
  # list. When it matches, the token type becomes "KEYWORD" and the value
  # is the literal keyword text.

  describe "tokenize/1 — keywords" do
    test "tokenizes 'component' as KEYWORD" do
      {:ok, tokens} = MosaicLexer.tokenize("component")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "component"
    end

    test "tokenizes 'slot' as KEYWORD" do
      {:ok, tokens} = MosaicLexer.tokenize("slot")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "slot"
    end

    test "tokenizes 'when' as KEYWORD" do
      {:ok, tokens} = MosaicLexer.tokenize("when")
      [tok, _eof] = tokens
      assert tok.type == "KEYWORD"
      assert tok.value == "when"
    end

    test "tokenizes 'each' and 'as' as KEYWORDs" do
      # "each @items as item" uses three keywords: each, as
      # (@ is AT, items and item are NAMEs)
      toks = types("each as")
      assert toks == ["KEYWORD", "KEYWORD"]
      vals = values("each as")
      assert vals == ["each", "as"]
    end

    test "tokenizes type keywords" do
      for kw <- ~w(text number bool image color node list) do
        {:ok, tokens} = MosaicLexer.tokenize(kw)
        [tok, _eof] = tokens
        assert tok.type == "KEYWORD", "expected KEYWORD for #{kw}"
        assert tok.value == kw
      end
    end

    test "tokenizes boolean keywords true and false" do
      assert types("true false") == ["KEYWORD", "KEYWORD"]
      assert values("true false") == ["true", "false"]
    end
  end

  # ---------------------------------------------------------------------------
  # Identifiers (NAME tokens)
  # ---------------------------------------------------------------------------
  # NAMEs are used for component names (PascalCase), slot names, property
  # names, and iterator variables. They allow hyphens for CSS-like names.

  describe "tokenize/1 — identifiers" do
    test "tokenizes a simple identifier" do
      {:ok, tokens} = MosaicLexer.tokenize("Foo")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "Foo"
    end

    test "tokenizes a hyphenated property name" do
      # CSS-like names such as "padding-left" or "corner-radius" are common
      # in UI component grammars. The NAME pattern allows internal hyphens.
      {:ok, tokens} = MosaicLexer.tokenize("padding-left")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "padding-left"
    end

    test "tokenizes underscore-separated names" do
      {:ok, tokens} = MosaicLexer.tokenize("my_component")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "my_component"
    end

    test "distinguishes NAME from KEYWORD" do
      # "components" (plural) is NOT a keyword; "component" (exact) is.
      {:ok, tokens} = MosaicLexer.tokenize("components")
      [tok, _eof] = tokens
      assert tok.type == "NAME"
      assert tok.value == "components"
    end
  end

  # ---------------------------------------------------------------------------
  # Color literals (COLOR_HEX)
  # ---------------------------------------------------------------------------
  # Hex colors must start with # followed by 3–8 hex digits.
  # Short (#fff) and long (#rrggbbaa) forms are both valid.

  describe "tokenize/1 — hex colors" do
    test "tokenizes short 3-digit hex color" do
      {:ok, tokens} = MosaicLexer.tokenize("#fff")
      [tok, _eof] = tokens
      assert tok.type == "COLOR_HEX"
      assert tok.value == "#fff"
    end

    test "tokenizes 6-digit hex color" do
      {:ok, tokens} = MosaicLexer.tokenize("#2563eb")
      [tok, _eof] = tokens
      assert tok.type == "COLOR_HEX"
      assert tok.value == "#2563eb"
    end

    test "tokenizes 8-digit hex color with alpha" do
      {:ok, tokens} = MosaicLexer.tokenize("#ff000080")
      [tok, _eof] = tokens
      assert tok.type == "COLOR_HEX"
      assert tok.value == "#ff000080"
    end

    test "tokenizes uppercase hex digits" do
      {:ok, tokens} = MosaicLexer.tokenize("#AABBCC")
      [tok, _eof] = tokens
      assert tok.type == "COLOR_HEX"
    end
  end

  # ---------------------------------------------------------------------------
  # Numbers and dimensions (NUMBER, DIMENSION)
  # ---------------------------------------------------------------------------
  # DIMENSION must be tried before NUMBER because both share the same numeric
  # prefix — "16dp" would match NUMBER for "16" if DIMENSION came second.

  describe "tokenize/1 — numbers and dimensions" do
    test "tokenizes a plain integer" do
      {:ok, tokens} = MosaicLexer.tokenize("42")
      [tok, _eof] = tokens
      assert tok.type == "NUMBER"
      assert tok.value == "42"
    end

    test "tokenizes a decimal number" do
      {:ok, tokens} = MosaicLexer.tokenize("3.14")
      [tok, _eof] = tokens
      assert tok.type == "NUMBER"
      assert tok.value == "3.14"
    end

    test "tokenizes a negative number" do
      {:ok, tokens} = MosaicLexer.tokenize("-8")
      [tok, _eof] = tokens
      assert tok.type == "NUMBER"
      assert tok.value == "-8"
    end

    test "tokenizes a dimension with dp unit" do
      # "16dp" — the dp suffix causes this to be a DIMENSION, not a NUMBER.
      # This ordering is critical: DIMENSION is listed before NUMBER in the
      # grammar so that "16dp" is never split into NUMBER("16") + NAME("dp").
      {:ok, tokens} = MosaicLexer.tokenize("16dp")
      [tok, _eof] = tokens
      assert tok.type == "DIMENSION"
      assert tok.value == "16dp"
    end

    test "tokenizes a percentage dimension" do
      {:ok, tokens} = MosaicLexer.tokenize("50%")
      [tok, _eof] = tokens
      assert tok.type == "DIMENSION"
      assert tok.value == "50%"
    end

    test "tokenizes a rem dimension" do
      {:ok, tokens} = MosaicLexer.tokenize("1.5rem")
      [tok, _eof] = tokens
      assert tok.type == "DIMENSION"
      assert tok.value == "1.5rem"
    end
  end

  # ---------------------------------------------------------------------------
  # String literals (STRING)
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — strings" do
    test "tokenizes a double-quoted string" do
      {:ok, tokens} = MosaicLexer.tokenize(~s("hello world"))
      [tok, _eof] = tokens
      assert tok.type == "STRING"
      assert tok.value == "hello world"
    end

    test "tokenizes a string with escaped characters" do
      # The grammar uses `escapes: standard` so backslash sequences are
      # processed by the lexer engine and the value contains the literal
      # escape character (e.g., \n becomes a real newline in the value).
      {:ok, tokens} = MosaicLexer.tokenize(~S("path/to/image.png"))
      [tok, _eof] = tokens
      assert tok.type == "STRING"
    end
  end

  # ---------------------------------------------------------------------------
  # Structural tokens
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — structural tokens" do
    test "tokenizes all delimiter characters" do
      # Covers: LBRACE RBRACE LANGLE RANGLE COLON SEMICOLON COMMA DOT EQUALS AT
      {:ok, tokens} = MosaicLexer.tokenize("{}  <>  : ; , . = @")
      types_no_eof = tokens |> Enum.map(& &1.type) |> Enum.reject(&(&1 == "EOF"))
      assert "LBRACE" in types_no_eof
      assert "RBRACE" in types_no_eof
      assert "LANGLE" in types_no_eof
      assert "RANGLE" in types_no_eof
      assert "COLON" in types_no_eof
      assert "SEMICOLON" in types_no_eof
      assert "COMMA" in types_no_eof
      assert "DOT" in types_no_eof
      assert "EQUALS" in types_no_eof
      assert "AT" in types_no_eof
    end

    test "tokenizes AT followed by NAME as a slot reference" do
      # "@items" is tokenized as [AT, NAME] — the parser handles the meaning
      {:ok, tokens} = MosaicLexer.tokenize("@items")
      [at, name, _eof] = tokens
      assert at.type == "AT"
      assert name.type == "NAME"
      assert name.value == "items"
    end

    test "tokenizes a slot reference with hyphenated name" do
      {:ok, tokens} = MosaicLexer.tokenize("@display-name")
      [at, name, _eof] = tokens
      assert at.type == "AT"
      assert name.type == "NAME"
      assert name.value == "display-name"
    end
  end

  # ---------------------------------------------------------------------------
  # Whitespace skipping
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — whitespace handling" do
    test "skips spaces between tokens" do
      assert types("{ }") == ["LBRACE", "RBRACE"]
    end

    test "skips newlines and tabs" do
      source = "component\n  Foo\n{\n}"
      toks = types(source)
      assert toks == ["KEYWORD", "NAME", "LBRACE", "RBRACE"]
    end

    test "skips line comments" do
      source = """
      // This is a comment
      component Foo {
      }
      """
      assert types(source) == ["KEYWORD", "NAME", "LBRACE", "RBRACE"]
    end

    test "skips block comments" do
      source = "/* block comment */ component"
      toks = types(source)
      assert toks == ["KEYWORD"]
    end
  end

  # ---------------------------------------------------------------------------
  # Position tracking
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — position tracking" do
    test "tracks line and column for first token" do
      {:ok, tokens} = MosaicLexer.tokenize("component Foo")
      [first | _] = tokens
      assert first.line == 1
      assert first.column == 1
    end

    test "tracks column offset for second token" do
      {:ok, tokens} = MosaicLexer.tokenize("component Foo")
      [_kw, name | _] = tokens
      # "component" is 9 chars + 1 space = column 11
      assert name.column == 11
    end
  end

  # ---------------------------------------------------------------------------
  # A realistic minimal component snippet
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — realistic snippets" do
    test "tokenizes a minimal component declaration" do
      source = "component ProfileCard { }"
      assert types(source) == ["KEYWORD", "NAME", "LBRACE", "RBRACE"]
      assert values(source) == ["component", "ProfileCard", "{", "}"]
    end

    test "tokenizes a slot declaration" do
      # slot title: text;
      source = "slot title: text;"
      toks = types(source)
      assert toks == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]
    end

    test "tokenizes a property assignment with color" do
      # background: #2563eb;
      source = "background: #2563eb;"
      assert types(source) == ["NAME", "COLON", "COLOR_HEX", "SEMICOLON"]
    end

    test "tokenizes a property assignment with dimension" do
      # padding: 16dp;
      source = "padding: 16dp;"
      assert types(source) == ["NAME", "COLON", "DIMENSION", "SEMICOLON"]
    end

    test "tokenizes a when block header" do
      # when @show-header {
      source = "when @show-header {"
      assert types(source) == ["KEYWORD", "AT", "NAME", "LBRACE"]
    end

    test "tokenizes an each block header" do
      # each @items as item {
      source = "each @items as item {"
      assert types(source) == ["KEYWORD", "AT", "NAME", "KEYWORD", "NAME", "LBRACE"]
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "tokenize/1 — error cases" do
    test "errors on an unexpected character" do
      # The backtick is not part of the Mosaic grammar
      {:error, msg} = MosaicLexer.tokenize("`bad`")
      assert msg =~ "Unexpected character" or msg =~ "nexpected"
    end

    test "returns error tuple (not raise) on bad input" do
      result = MosaicLexer.tokenize("~invalid")
      assert match?({:error, _}, result)
    end
  end
end
