defmodule CodingAdventures.Brainfuck.LexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brainfuck.Lexer

  # =========================================================================
  # Helper: extract token types from a successful tokenization.
  # =========================================================================

  defp tokenize!(source) do
    {:ok, tokens} = Lexer.tokenize(source)
    tokens
  end

  defp command_types(source) do
    tokenize!(source)
    |> Enum.reject(&(&1.type == "EOF"))
    |> Enum.map(& &1.type)
  end

  defp command_values(source) do
    tokenize!(source)
    |> Enum.reject(&(&1.type == "EOF"))
    |> Enum.map(& &1.value)
  end

  # =========================================================================
  # Grammar path
  # =========================================================================

  describe "grammar path" do
    test "brainfuck.tokens grammar file exists" do
      # The Lexer module computes @grammars_dir at compile time via
      # Path.expand. If it computed the wrong path, create_lexer/0 would
      # have already crashed. We verify by calling create_lexer/0 directly.
      grammar = Lexer.create_lexer()
      # A successful return means the grammar file was found and parsed.
      refute is_nil(grammar)
    end
  end

  # =========================================================================
  # Individual command tokens
  # =========================================================================
  # Each of the 8 Brainfuck command characters must produce the correct
  # token type. The value field must contain the exact source character.

  describe "individual command tokens" do
    test "RIGHT token from '>'" do
      # ">" moves the data pointer one cell to the right.
      types = command_types(">")
      assert types == ["RIGHT"]
    end

    test "LEFT token from '<'" do
      # "<" moves the data pointer one cell to the left.
      types = command_types("<")
      assert types == ["LEFT"]
    end

    test "INC token from '+'" do
      # "+" increments the byte at the current cell (wraps at 255 -> 0).
      types = command_types("+")
      assert types == ["INC"]
    end

    test "DEC token from '-'" do
      # "-" decrements the byte at the current cell (wraps at 0 -> 255).
      types = command_types("-")
      assert types == ["DEC"]
    end

    test "OUTPUT token from '.'" do
      # "." writes the current cell's byte value to stdout as a character.
      types = command_types(".")
      assert types == ["OUTPUT"]
    end

    test "INPUT token from ','" do
      # "," reads one byte from stdin and stores it in the current cell.
      types = command_types(",")
      assert types == ["INPUT"]
    end

    test "LOOP_START token from '['" do
      # "[" starts a loop: jumps past matching "]" if current cell is zero.
      types = command_types("[")
      assert types == ["LOOP_START"]
    end

    test "LOOP_END token from ']'" do
      # "]" ends a loop: jumps back to matching "[" if current cell is nonzero.
      types = command_types("]")
      assert types == ["LOOP_END"]
    end

    test "token values are the exact source characters" do
      source = "><+-.,[]"
      values = command_values(source)
      assert values == [">", "<", "+", "-", ".", ",", "[", "]"]
    end

    test "all eight commands in order produce correct types" do
      source = "><+-.,[]"
      types = command_types(source)
      assert types == ["RIGHT", "LEFT", "INC", "DEC", "OUTPUT", "INPUT", "LOOP_START", "LOOP_END"]
    end
  end

  # =========================================================================
  # Comment skipping
  # =========================================================================
  # Brainfuck has no dedicated comment syntax: any character that is not one
  # of the 8 commands is a comment. The lexer's skip: mechanism consumes
  # comments silently. NO comment tokens should appear in the output.

  describe "comment skipping" do
    test "letters are comments and produce no tokens" do
      # Alphabetic text is the most common Brainfuck comment style.
      types = command_types("hello world")
      assert types == []
    end

    test "digits and punctuation are comments" do
      types = command_types("1234 cell 0 value = 42")
      assert types == []
    end

    test "comments between commands are stripped" do
      # A documented Brainfuck program: "++ add two" → only two INC tokens.
      source = "++ add two to cell 0"
      types = command_types(source)
      assert types == ["INC", "INC"]
    end

    test "whitespace between commands is ignored" do
      source = "+  >  -  <"
      types = command_types(source)
      assert types == ["INC", "RIGHT", "DEC", "LEFT"]
    end

    test "heavily annotated program produces correct commands" do
      # A fully commented Brainfuck idiom: copy cell 0 to cell 1.
      source = "++ set cell0=2 [ loop: > RIGHT + INC < LEFT - DEC ] end loop"
      types = command_types(source)
      assert types == ["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]
    end
  end

  # =========================================================================
  # Empty source
  # =========================================================================

  describe "empty source" do
    test "empty string returns ok tuple" do
      result = Lexer.tokenize("")
      assert {:ok, _tokens} = result
    end

    test "empty string produces only EOF token" do
      {:ok, tokens} = Lexer.tokenize("")
      assert length(tokens) == 1
      assert hd(tokens).type == "EOF"
    end

    test "source with only comments produces only EOF token" do
      {:ok, tokens} = Lexer.tokenize("This is a Brainfuck program that does nothing at all")
      non_eof = Enum.reject(tokens, &(&1.type == "EOF"))
      assert non_eof == []
    end
  end

  # =========================================================================
  # Line and column tracking
  # =========================================================================
  # The lexer tracks line (1-based) and column (1-based) for every token.
  # This is essential for meaningful parse error messages.

  describe "line and column tracking" do
    test "first command on line 1 column 1" do
      {:ok, tokens} = Lexer.tokenize("+")
      inc_token = Enum.find(tokens, &(&1.type == "INC"))
      refute is_nil(inc_token)
      assert inc_token.line == 1
    end

    test "command after newline is on next line" do
      # Two newlines of comment before ">", so ">" should be on line 3.
      source = "comment line 1\ncomment line 2\n>"
      {:ok, tokens} = Lexer.tokenize(source)
      right_token = Enum.find(tokens, &(&1.type == "RIGHT"))
      refute is_nil(right_token)
      assert right_token.line == 3
    end

    test "column advances with leading spaces" do
      # Four spaces then "+": should be at column 5.
      {:ok, tokens} = Lexer.tokenize("    +")
      inc_token = Enum.find(tokens, &(&1.type == "INC"))
      refute is_nil(inc_token)
      assert inc_token.column == 5
    end

    test "eof token is always present" do
      for source <- ["", "+", "++", "hello", "++[>+<-]"] do
        {:ok, tokens} = Lexer.tokenize(source)
        last = List.last(tokens)
        assert last.type == "EOF",
          "EOF must be last token for #{inspect(source)}"
      end
    end
  end

  # =========================================================================
  # Canonical Brainfuck example: ++[>+<-]
  # =========================================================================
  # This is the standard "copy cell" idiom. It adds cell 0's value to
  # cell 1, leaving cell 0 at zero.
  #
  #   ++      cell 0 = 2
  #   [       while cell 0 != 0
  #     >+    move right, increment cell 1
  #     <-    move left, decrement cell 0
  #   ]       end loop

  describe "canonical ++[>+<-] example" do
    test "produces correct token types in order" do
      types = command_types("++[>+<-]")
      expected = ["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]
      assert types == expected
    end

    test "produces correct token values in order" do
      values = command_values("++[>+<-]")
      assert values == ["+", "+", "[", ">", "+", "<", "-", "]"]
    end

    test "annotated version produces same tokens" do
      # Inline comments should not affect the output token stream.
      source = "++ set cell0=2\n[ loop while nonzero\n  >+ right and inc\n  <- left and dec\n] end"
      types = command_types(source)
      expected = ["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]
      assert types == expected
    end

    test "returns ok tuple" do
      result = Lexer.tokenize("++[>+<-]")
      assert {:ok, _} = result
    end
  end
end
