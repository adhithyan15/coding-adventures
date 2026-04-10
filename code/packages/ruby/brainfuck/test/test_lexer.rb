# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Brainfuck Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with brainfuck.tokens, correctly tokenizes Brainfuck source.
#
# Brainfuck tokenization has two important properties that we test:
#
#   1. COMMAND recognition — the 8 command characters (> < + - . , [ ])
#      must each produce the correct token type and value.
#
#   2. COMMENT skipping — every non-command character is silently
#      consumed. Letters, digits, spaces, punctuation are all comments.
#      They MUST NOT appear in the token stream.
#
# We are not testing the GrammarLexer engine itself (tested in the
# lexer gem) — we are testing that brainfuck.tokens correctly
# describes Brainfuck's lexical rules.
# ================================================================

class TestBrainfuckLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  # Brainfuck token types are plain strings (not in TokenType::ALL)
  RIGHT_TYPE      = "RIGHT"
  LEFT_TYPE       = "LEFT"
  INC_TYPE        = "INC"
  DEC_TYPE        = "DEC"
  OUTPUT_TYPE     = "OUTPUT"
  INPUT_TYPE      = "INPUT"
  LOOP_START_TYPE = "LOOP_START"
  LOOP_END_TYPE   = "LOOP_END"

  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  def tokenize(source)
    CodingAdventures::Brainfuck::Lexer.tokenize(source)
  end

  def command_tokens(source)
    # Filter out EOF so tests can just check command tokens
    tokenize(source).reject { |t| t.type == TT::EOF }
  end

  def token_types(source)
    command_tokens(source).map(&:type)
  end

  # ------------------------------------------------------------------
  # Grammar path sanity check
  # ------------------------------------------------------------------
  # The lexer must be able to find brainfuck.tokens at the path
  # computed by the module constant. If this test fails, the directory
  # navigation arithmetic in GRAMMAR_DIR is wrong.

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::Brainfuck::Lexer::BF_TOKENS_PATH),
      "brainfuck.tokens should exist at #{CodingAdventures::Brainfuck::Lexer::BF_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Individual command tokens
  # ------------------------------------------------------------------
  # Each of the 8 Brainfuck commands must be recognized as its own
  # token type. The value field must contain the exact source character.

  def test_right_token
    # ">" moves the data pointer one cell to the right.
    tokens = command_tokens(">")
    assert_equal 1, tokens.length
    assert_equal RIGHT_TYPE, tokens[0].type
    assert_equal ">", tokens[0].value
  end

  def test_left_token
    # "<" moves the data pointer one cell to the left.
    tokens = command_tokens("<")
    assert_equal 1, tokens.length
    assert_equal LEFT_TYPE, tokens[0].type
    assert_equal "<", tokens[0].value
  end

  def test_inc_token
    # "+" increments the byte at the current cell.
    tokens = command_tokens("+")
    assert_equal 1, tokens.length
    assert_equal INC_TYPE, tokens[0].type
    assert_equal "+", tokens[0].value
  end

  def test_dec_token
    # "-" decrements the byte at the current cell.
    tokens = command_tokens("-")
    assert_equal 1, tokens.length
    assert_equal DEC_TYPE, tokens[0].type
    assert_equal "-", tokens[0].value
  end

  def test_output_token
    # "." writes the current cell's byte value to stdout.
    tokens = command_tokens(".")
    assert_equal 1, tokens.length
    assert_equal OUTPUT_TYPE, tokens[0].type
    assert_equal ".", tokens[0].value
  end

  def test_input_token
    # "," reads one byte from stdin into the current cell.
    tokens = command_tokens(",")
    assert_equal 1, tokens.length
    assert_equal INPUT_TYPE, tokens[0].type
    assert_equal ",", tokens[0].value
  end

  def test_loop_start_token
    # "[" jumps forward past matching "]" if current cell is zero.
    tokens = command_tokens("[")
    assert_equal 1, tokens.length
    assert_equal LOOP_START_TYPE, tokens[0].type
    assert_equal "[", tokens[0].value
  end

  def test_loop_end_token
    # "]" jumps back to matching "[" if current cell is nonzero.
    tokens = command_tokens("]")
    assert_equal 1, tokens.length
    assert_equal LOOP_END_TYPE, tokens[0].type
    assert_equal "]", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Comment skipping
  # ------------------------------------------------------------------
  # All non-command characters are comments. The lexer's skip: section
  # consumes them silently. No comment tokens should appear in output.

  def test_letters_are_comments
    # Alphabetic characters often appear as prose comments in Brainfuck.
    # "hello" produces zero command tokens.
    types = token_types("hello world")
    assert_empty types, "Letter comments should not produce any tokens"
  end

  def test_digits_are_comments
    # Numeric annotations (like "cell 0") are comments too.
    types = token_types("123 cell value")
    assert_empty types, "Digit comments should not produce any tokens"
  end

  def test_comments_between_commands
    # A common Brainfuck convention is to annotate commands inline:
    #   +  increment cell 0
    #   >  move right to cell 1
    # The lexer should strip the annotation and keep only + and >.
    types = token_types("+ increment cell 0 > move right")
    assert_equal [INC_TYPE, RIGHT_TYPE], types
  end

  def test_mixed_whitespace_and_commands
    # Whitespace between commands (spaces, tabs, newlines) is skipped.
    # A well-formatted Brainfuck program uses indentation and newlines.
    source = "  +  \n  >  \n  -  "
    types = token_types(source)
    assert_equal [INC_TYPE, RIGHT_TYPE, DEC_TYPE], types
  end

  def test_all_commands_in_sequence
    # All eight commands together, no comments. Order must be preserved.
    source = "><+-.,[]"
    types = token_types(source)
    expected = [
      RIGHT_TYPE, LEFT_TYPE, INC_TYPE, DEC_TYPE,
      OUTPUT_TYPE, INPUT_TYPE, LOOP_START_TYPE, LOOP_END_TYPE
    ]
    assert_equal expected, types
  end

  # ------------------------------------------------------------------
  # Empty source
  # ------------------------------------------------------------------
  # An empty Brainfuck program is valid. The only token should be EOF.

  def test_empty_source_returns_eof
    tokens = tokenize("")
    assert_equal 1, tokens.length, "Empty source should produce exactly one token (EOF)"
    assert_equal TT::EOF, tokens[0].type
  end

  def test_only_comments_returns_eof
    # A source with nothing but comments produces an empty command stream.
    tokens = tokenize("This is a Brainfuck program that does nothing.")
    assert_equal TT::EOF, tokens.last.type
    non_eof = tokens.reject { |t| t.type == TT::EOF }
    assert_empty non_eof, "Only-comment source should produce no command tokens"
  end

  # ------------------------------------------------------------------
  # Line and column tracking
  # ------------------------------------------------------------------
  # The lexer must track line and column numbers accurately so that
  # error messages from the parser can report meaningful locations.
  # Line numbers start at 1, column numbers start at 1.

  def test_line_tracking_across_newlines
    # A newline in a comment should advance the line counter.
    # The ">" on line 3 (after two lines of comments) should report line 3.
    source = "comment line 1\ncomment line 2\n>"
    tokens = command_tokens(source)
    assert_equal 1, tokens.length
    assert_equal RIGHT_TYPE, tokens[0].type
    assert_equal 3, tokens[0].line, "Token after two newlines should be on line 3"
  end

  def test_column_tracking_on_first_line
    # Spaces before the first command push the column to the right.
    # Four spaces before "+" means column 5.
    tokens = command_tokens("    +")
    assert_equal 1, tokens.length
    assert_equal 5, tokens[0].col, "Token after 4 spaces should be at column 5"
  end

  def test_eof_always_present
    # The token stream always ends with EOF regardless of source content.
    ["", "+", "++", "hello", "++[>+<-]"].each do |src|
      tokens = tokenize(src)
      assert_equal TT::EOF, tokens.last.type, "EOF must be last token for #{src.inspect}"
    end
  end

  # ------------------------------------------------------------------
  # Canonical Brainfuck example: ++[>+<-]
  # ------------------------------------------------------------------
  # This program is the simplest non-trivial Brainfuck idiom.
  # It adds the value of cell 0 to cell 1 (2 + 0 = 2 in cell 1),
  # leaving cell 0 at 0. It exercises all the pointer, arithmetic,
  # and loop control tokens together.
  #
  #   ++      set cell 0 to 2
  #   [       loop while cell 0 != 0
  #     >+    move right, increment cell 1
  #     <-    move left, decrement cell 0
  #   ]       end loop (exits when cell 0 = 0)

  def test_canonical_plus_plus_loop
    source = "++[>+<-]"
    types = token_types(source)
    expected = [
      INC_TYPE, INC_TYPE,           # ++
      LOOP_START_TYPE,              # [
        RIGHT_TYPE, INC_TYPE,       # >+
        LEFT_TYPE, DEC_TYPE,        # <-
      LOOP_END_TYPE                 # ]
    ]
    assert_equal expected, types
  end

  def test_canonical_plus_plus_loop_values
    # Verify that the value field contains the exact source character.
    source = "++[>+<-]"
    tokens = command_tokens(source)
    values = tokens.map(&:value)
    assert_equal ["+", "+", "[", ">", "+", "<", "-", "]"], values
  end

  def test_canonical_with_inline_comments
    # The same program annotated with comments should produce identical tokens.
    # This is how Brainfuck programs are documented in the wild.
    source = "++ add two [>+ move+inc <-  decrement  ] end loop"
    types = token_types(source)
    expected = [
      INC_TYPE, INC_TYPE,
      LOOP_START_TYPE,
        RIGHT_TYPE, INC_TYPE,
        LEFT_TYPE, DEC_TYPE,
      LOOP_END_TYPE
    ]
    assert_equal expected, types
  end
end
