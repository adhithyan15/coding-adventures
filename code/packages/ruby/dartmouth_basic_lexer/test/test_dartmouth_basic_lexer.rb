# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Dartmouth BASIC 1964 Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, loaded with
# dartmouth_basic.tokens, correctly tokenizes Dartmouth BASIC source text.
#
# We test:
#   - All 20 keywords produce KEYWORD tokens
#   - The 11 built-in functions produce BUILTIN_FN tokens
#   - User-defined functions (FNA..FNZ) produce USER_FN tokens
#   - Variable names (one letter + optional digit) produce NAME tokens
#   - Numeric literals: integers, decimals, leading-dot, scientific notation
#   - String literals: the GrammarLexer strips the surrounding quotes
#   - All arithmetic and comparison operators
#   - LINE_NUM disambiguation: first NUMBER on each line → LINE_NUM
#   - REM comment suppression: tokens between REM and NEWLINE dropped
#   - Case insensitivity: "print" == "PRINT" == "Print"
#   - Multi-line programs
#   - Error recovery: UNKNOWN tokens for unrecognised characters
#   - Grammar file path resolves correctly
#   - Line/column tracking
#   - EOF token always present
#
# We are NOT testing the GrammarLexer engine itself — that is tested in
# the lexer gem. We are testing that:
#   1. The dartmouth_basic.tokens grammar file correctly describes the
#      lexical rules of Dartmouth BASIC 1964.
#   2. The post-tokenize hooks (LINE_NUM relabelling, REM suppression)
#      correctly transform the raw token stream.
# ================================================================

class TestDartmouthBasicLexer < Minitest::Test
  # Shorthand for the token type constants so tests are less verbose.
  TT = CodingAdventures::Lexer::TokenType

  # ------------------------------------------------------------------
  # Helper methods
  # ------------------------------------------------------------------

  # Tokenize source and return all tokens (including EOF).
  def tokenize(source)
    CodingAdventures::DartmouthBasicLexer.tokenize(source)
  end

  # Return token types for all non-EOF tokens.
  def types(source)
    tokenize(source).reject { |t| t.type == TT::EOF }.map(&:type)
  end

  # Return token values for all non-EOF tokens.
  def values(source)
    tokenize(source).reject { |t| t.type == TT::EOF }.map(&:value)
  end

  # Return all non-EOF tokens.
  def non_eof(source)
    tokenize(source).reject { |t| t.type == TT::EOF }
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------
  # The grammar file must be reachable from the tokenizer's computed path.
  # If this test fails, the relative path calculation is wrong.

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::DartmouthBasicLexer::DARTMOUTH_BASIC_TOKENS_PATH),
      "dartmouth_basic.tokens should exist at " \
      "#{CodingAdventures::DartmouthBasicLexer::DARTMOUTH_BASIC_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # EOF token
  # ------------------------------------------------------------------
  # Every token stream ends with EOF, even for empty input.

  def test_eof_always_present
    tokens = tokenize("10 END")
    assert_equal TT::EOF, tokens.last.type
  end

  def test_empty_input_produces_eof
    tokens = tokenize("")
    assert_equal 1, tokens.length
    assert_equal TT::EOF, tokens.last.type
  end

  def test_only_newline_produces_newline_and_eof
    tokens = tokenize("\n")
    non_eof_tokens = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof_tokens.length
    assert_equal "NEWLINE", non_eof_tokens[0].type
  end

  # ------------------------------------------------------------------
  # LINE_NUM disambiguation
  # ------------------------------------------------------------------
  #
  # The trickiest aspect of BASIC lexing. The first NUMBER on each
  # physical line must be relabelled LINE_NUM by the post-tokenize hook.
  # All other NUMBER tokens remain as NUMBER.

  def test_line_num_at_start_of_first_line
    tokens = non_eof("10 LET X = 5\n")
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "10",       tokens[0].value
  end

  def test_number_in_expression_not_relabelled
    # The "5" in "LET X = 5" is in an expression, not a line label.
    tokens = non_eof("10 LET X = 5\n")
    # tokens: LINE_NUM(10) KEYWORD(LET) NAME(X) EQ(=) NUMBER(5) NEWLINE
    eq_index = tokens.index { |t| t.type == "EQ" }
    number_token = tokens[eq_index + 1]
    assert_equal "NUMBER", number_token.type
    assert_equal "5",      number_token.value
  end

  def test_line_num_on_second_line_after_newline
    source = "10 END\n20 STOP\n"
    tokens = non_eof(source)
    # Expect: LINE_NUM(10) KEYWORD(END) NEWLINE LINE_NUM(20) KEYWORD(STOP) NEWLINE
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "10",       tokens[0].value
    assert_equal "NEWLINE",  tokens[2].type
    assert_equal "LINE_NUM", tokens[3].type
    assert_equal "20",       tokens[3].value
  end

  def test_line_num_with_three_digits
    tokens = non_eof("100 END\n")
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "100",      tokens[0].value
  end

  def test_line_num_with_four_digits
    tokens = non_eof("9999 END\n")
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "9999",     tokens[0].value
  end

  def test_goto_target_is_number_not_line_num
    # In "GOTO 30", the "30" is a branch target but not at line start.
    # It stays as NUMBER — the parser will validate it is an integer.
    source = "10 GOTO 30\n"
    tokens = non_eof(source)
    # tokens: LINE_NUM(10) KEYWORD(GOTO) NUMBER(30) NEWLINE
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "KEYWORD",  tokens[1].type
    assert_equal "GOTO",     tokens[1].value
    assert_equal "NUMBER",   tokens[2].type
    assert_equal "30",       tokens[2].value
  end

  def test_gosub_target_is_number_not_line_num
    source = "10 GOSUB 500\n"
    tokens = non_eof(source)
    assert_equal "NUMBER",   tokens[2].type
    assert_equal "500",      tokens[2].value
  end

  def test_if_then_target_is_number_not_line_num
    # "IF X > 0 THEN 100" — the 100 is a branch target (NUMBER)
    source = "10 IF X > 0 THEN 100\n"
    tokens = non_eof(source)
    last_number = tokens.reverse.find { |t| t.type == "NUMBER" }
    assert_equal "100", last_number.value
  end

  # ------------------------------------------------------------------
  # Keywords (all 20 Dartmouth BASIC 1964 reserved words)
  # ------------------------------------------------------------------
  #
  # Keywords are reclassified by the grammar engine from NAME tokens.
  # Because @case_insensitive true, all keywords are normalised to
  # uppercase in the emitted token value.

  def test_keyword_let
    tokens = non_eof("10 LET X = 1\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "LET", kw.value
  end

  def test_keyword_print
    tokens = non_eof("10 PRINT X\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "PRINT", kw.value
  end

  def test_keyword_input
    tokens = non_eof("10 INPUT X\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "INPUT", kw.value
  end

  def test_keyword_if
    tokens = non_eof("10 IF X > 0 THEN 20\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "IF", kw.value
  end

  def test_keyword_then
    tokens = non_eof("10 IF X > 0 THEN 20\n")
    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_includes keywords, "THEN"
  end

  def test_keyword_goto
    tokens = non_eof("10 GOTO 20\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "GOTO", kw.value
  end

  def test_keyword_gosub
    tokens = non_eof("10 GOSUB 100\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "GOSUB", kw.value
  end

  def test_keyword_return
    tokens = non_eof("10 RETURN\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "RETURN", kw.value
  end

  def test_keyword_for
    tokens = non_eof("10 FOR I = 1 TO 10\n")
    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_includes keywords, "FOR"
  end

  def test_keyword_to
    tokens = non_eof("10 FOR I = 1 TO 10\n")
    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_includes keywords, "TO"
  end

  def test_keyword_step
    tokens = non_eof("10 FOR I = 1 TO 10 STEP 2\n")
    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_includes keywords, "STEP"
  end

  def test_keyword_next
    tokens = non_eof("10 NEXT I\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "NEXT", kw.value
  end

  def test_keyword_end
    tokens = non_eof("10 END\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "END", kw.value
  end

  def test_keyword_stop
    tokens = non_eof("10 STOP\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "STOP", kw.value
  end

  def test_keyword_rem
    # REM itself is a KEYWORD; its content is suppressed by hook 2.
    tokens = non_eof("10 REM COMMENT\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "REM", kw.value
  end

  def test_keyword_read
    tokens = non_eof("10 READ X\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "READ", kw.value
  end

  def test_keyword_data
    tokens = non_eof("10 DATA 1, 2, 3\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "DATA", kw.value
  end

  def test_keyword_restore
    tokens = non_eof("10 RESTORE\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "RESTORE", kw.value
  end

  def test_keyword_dim
    tokens = non_eof("10 DIM A(10)\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "DIM", kw.value
  end

  def test_keyword_def
    tokens = non_eof("10 DEF FNA(X) = X * X\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "DEF", kw.value
  end

  # ------------------------------------------------------------------
  # Built-in functions (all 11 from the 1964 spec)
  # ------------------------------------------------------------------
  #
  # BUILTIN_FN tokens must appear BEFORE NAME in the grammar so that
  # "SIN" is not tokenised as NAME("S") NAME("I") NAME("N").

  def test_builtin_sin
    tokens = non_eof("10 LET X = SIN(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "SIN", fn_tok.value
  end

  def test_builtin_cos
    tokens = non_eof("10 LET X = COS(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "COS", fn_tok.value
  end

  def test_builtin_tan
    tokens = non_eof("10 LET X = TAN(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "TAN", fn_tok.value
  end

  def test_builtin_atn
    tokens = non_eof("10 LET X = ATN(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "ATN", fn_tok.value
  end

  def test_builtin_exp
    tokens = non_eof("10 LET X = EXP(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "EXP", fn_tok.value
  end

  def test_builtin_log
    tokens = non_eof("10 LET X = LOG(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "LOG", fn_tok.value
  end

  def test_builtin_abs
    tokens = non_eof("10 LET X = ABS(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "ABS", fn_tok.value
  end

  def test_builtin_sqr
    tokens = non_eof("10 LET X = SQR(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "SQR", fn_tok.value
  end

  def test_builtin_int
    tokens = non_eof("10 LET X = INT(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "INT", fn_tok.value
  end

  def test_builtin_rnd
    tokens = non_eof("10 LET X = RND(0)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "RND", fn_tok.value
  end

  def test_builtin_sgn
    tokens = non_eof("10 LET X = SGN(Y)\n")
    fn_tok = tokens.find { |t| t.type == "BUILTIN_FN" }
    assert_equal "SGN", fn_tok.value
  end

  # ------------------------------------------------------------------
  # User-defined functions (FNA..FNZ)
  # ------------------------------------------------------------------

  def test_user_fn_fna
    tokens = non_eof("10 DEF FNA(X) = X * X\n")
    fn_tok = tokens.find { |t| t.type == "USER_FN" }
    assert_equal "FNA", fn_tok.value
  end

  def test_user_fn_fnz
    tokens = non_eof("10 LET X = FNZ(Y)\n")
    fn_tok = tokens.find { |t| t.type == "USER_FN" }
    assert_equal "FNZ", fn_tok.value
  end

  def test_user_fn_fnb
    tokens = non_eof("10 LET X = FNB(Y)\n")
    fn_tok = tokens.find { |t| t.type == "USER_FN" }
    assert_equal "FNB", fn_tok.value
  end

  # ------------------------------------------------------------------
  # Variable names
  # ------------------------------------------------------------------
  #
  # Dartmouth BASIC 1964 supports exactly two variable name forms:
  #   - One uppercase letter: X, Y, Z, A..Z (26 names)
  #   - One uppercase letter + one digit: A0..A9, ..., Z0..Z9 (260 names)
  # Total: 286 variable names.

  def test_name_single_letter
    tokens = non_eof("10 LET X = 1\n")
    name_tok = tokens.find { |t| t.type == "NAME" }
    assert_equal "X", name_tok.value
  end

  def test_name_letter_plus_digit_a1
    tokens = non_eof("10 LET A1 = 2\n")
    name_tok = tokens.find { |t| t.type == "NAME" }
    assert_equal "A1", name_tok.value
  end

  def test_name_letter_plus_digit_z9
    tokens = non_eof("10 LET Z9 = 3\n")
    name_tok = tokens.find { |t| t.type == "NAME" }
    assert_equal "Z9", name_tok.value
  end

  def test_name_letter_plus_zero
    tokens = non_eof("10 LET B0 = 4\n")
    name_tok = tokens.find { |t| t.type == "NAME" }
    assert_equal "B0", name_tok.value
  end

  # ------------------------------------------------------------------
  # Numeric literals
  # ------------------------------------------------------------------
  #
  # BASIC stores all numbers as floats internally, so the grammar
  # matches a broad range of numeric literal formats.

  def test_number_integer
    tokens = non_eof("10 LET X = 42\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "42", num_tok.value
  end

  def test_number_decimal
    tokens = non_eof("10 LET X = 3.14\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "3.14", num_tok.value
  end

  def test_number_leading_dot
    # .5 is a valid number in BASIC (0.5)
    tokens = non_eof("10 LET X = .5\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal ".5", num_tok.value
  end

  def test_number_scientific_notation
    # 1.5E3 = 1500.0
    # Note: case_sensitive: false lowercases the source, so "E" becomes "e"
    # in the token value. The grammar regex matches [Ee] to handle both.
    tokens = non_eof("10 LET X = 1.5E3\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "1.5e3", num_tok.value
  end

  def test_number_scientific_notation_lowercase_input
    # Source already in lowercase — same result.
    tokens = non_eof("10 let x = 1.5e3\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "1.5e3", num_tok.value
  end

  def test_number_scientific_negative_exponent
    # 1.5E-3 = 0.0015; "E" → "e" after source lowercasing
    tokens = non_eof("10 LET X = 1.5E-3\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "1.5e-3", num_tok.value
  end

  def test_number_scientific_no_decimal
    # 1E10 = 10_000_000_000.0; "E" → "e" after source lowercasing
    tokens = non_eof("10 LET X = 1E10\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "1e10", num_tok.value
  end

  def test_number_zero
    tokens = non_eof("10 LET X = 0\n")
    num_tok = tokens.find { |t| t.type == "NUMBER" }
    assert_equal "0", num_tok.value
  end

  # ------------------------------------------------------------------
  # String literals
  # ------------------------------------------------------------------
  #
  # The GrammarLexer strips the surrounding double quotes from STRING
  # tokens (the alias `-> STRING` triggers the quote-stripping logic).
  # So "HELLO WORLD" produces value "HELLO WORLD" (no quotes).
  #
  # Note: because @case_insensitive normalises source to uppercase,
  # and the grammar uses @original_source for string extraction, the
  # case of string content is preserved from the original input.

  def test_string_simple
    tokens = non_eof("10 PRINT \"HELLO\"\n")
    str_tok = tokens.find { |t| t.type == "STRING" }
    refute_nil str_tok, "Expected a STRING token"
    assert_equal "HELLO", str_tok.value
  end

  def test_string_with_spaces
    tokens = non_eof("10 PRINT \"HELLO WORLD\"\n")
    str_tok = tokens.find { |t| t.type == "STRING" }
    assert_equal "HELLO WORLD", str_tok.value
  end

  def test_string_empty
    tokens = non_eof("10 PRINT \"\"\n")
    str_tok = tokens.find { |t| t.type == "STRING" }
    assert_equal "", str_tok.value
  end

  def test_string_with_numbers_inside
    tokens = non_eof("10 PRINT \"VALUE IS 42\"\n")
    str_tok = tokens.find { |t| t.type == "STRING" }
    assert_equal "VALUE IS 42", str_tok.value
  end

  # ------------------------------------------------------------------
  # Case insensitivity
  # ------------------------------------------------------------------
  #
  # All keywords and function names should be recognised regardless of
  # the case they appear in. The emitted KEYWORD and BUILTIN_FN tokens
  # always have uppercase values.

  def test_case_insensitive_keyword_print_lowercase
    tokens_upper = non_eof("10 PRINT X\n")
    tokens_lower = non_eof("10 print x\n")
    # Both should produce the same type sequence.
    assert_equal tokens_upper.map(&:type),  tokens_lower.map(&:type)
    assert_equal tokens_upper.map(&:value), tokens_lower.map(&:value)
  end

  def test_case_insensitive_keyword_let_mixed
    tokens_upper = non_eof("10 LET A = 1\n")
    tokens_mixed = non_eof("10 Let A = 1\n")
    assert_equal tokens_upper.map(&:type),  tokens_mixed.map(&:type)
    assert_equal tokens_upper.map(&:value), tokens_mixed.map(&:value)
  end

  def test_case_insensitive_keyword_goto_lowercase
    tokens_upper = non_eof("10 GOTO 20\n")
    tokens_lower = non_eof("10 goto 20\n")
    assert_equal tokens_upper.map(&:type),  tokens_lower.map(&:type)
    assert_equal tokens_upper.map(&:value), tokens_lower.map(&:value)
  end

  def test_case_insensitive_builtin_sin_lowercase
    tokens_upper = non_eof("10 LET X = SIN(Y)\n")
    tokens_lower = non_eof("10 let x = sin(y)\n")
    assert_equal tokens_upper.map(&:type), tokens_lower.map(&:type)
  end

  def test_case_insensitive_keyword_value_is_uppercase
    # Even if the source has lowercase "let", the emitted value is "LET".
    tokens = non_eof("10 let x = 1\n")
    kw = tokens.find { |t| t.type == "KEYWORD" }
    assert_equal "LET", kw.value
  end

  # ------------------------------------------------------------------
  # Multi-character comparison operators
  # ------------------------------------------------------------------
  #
  # These operators must be matched BEFORE their single-character
  # prefixes. Without priority ordering, "<=" would be LT + EQ.
  # The grammar places LE, GE, NE at the top to ensure they win.

  def test_le_operator_single_token
    tokens = non_eof("10 IF X <= Y THEN 50\n")
    le_tok = tokens.find { |t| t.type == "LE" }
    refute_nil le_tok, "Expected LE token for <="
    assert_equal "<=", le_tok.value
  end

  def test_le_is_not_lt_and_eq
    tokens = non_eof("10 IF X <= Y THEN 50\n")
    # Must be exactly ONE token for <=, not two.
    lt_count = tokens.count { |t| t.type == "LT" }
    eq_count = tokens.count { |t| t.type == "EQ" }
    le_count = tokens.count { |t| t.type == "LE" }
    assert_equal 1, le_count
    assert_equal 0, lt_count
    assert_equal 0, eq_count
  end

  def test_ge_operator_single_token
    tokens = non_eof("10 IF X >= Y THEN 50\n")
    ge_tok = tokens.find { |t| t.type == "GE" }
    refute_nil ge_tok
    assert_equal ">=", ge_tok.value
  end

  def test_ge_is_not_gt_and_eq
    tokens = non_eof("10 IF X >= Y THEN 50\n")
    assert_equal 1, tokens.count { |t| t.type == "GE" }
    assert_equal 0, tokens.count { |t| t.type == "GT" }
    assert_equal 0, tokens.count { |t| t.type == "EQ" }
  end

  def test_ne_operator_single_token
    tokens = non_eof("10 IF X <> Y THEN 50\n")
    ne_tok = tokens.find { |t| t.type == "NE" }
    refute_nil ne_tok
    assert_equal "<>", ne_tok.value
  end

  def test_ne_is_not_lt_and_gt
    tokens = non_eof("10 IF X <> Y THEN 50\n")
    assert_equal 1, tokens.count { |t| t.type == "NE" }
    assert_equal 0, tokens.count { |t| t.type == "LT" }
    assert_equal 0, tokens.count { |t| t.type == "GT" }
  end

  def test_lt_alone_is_still_lt
    # After LE/GE/NE are parsed, a standalone < is still LT.
    tokens = non_eof("10 IF X < Y THEN 50\n")
    assert_equal 1, tokens.count { |t| t.type == "LT" }
    assert_equal 0, tokens.count { |t| t.type == "LE" }
  end

  # ------------------------------------------------------------------
  # Single-character operators
  # ------------------------------------------------------------------

  def test_plus_operator
    tokens = non_eof("10 LET X = 1 + 2\n")
    assert_equal 1, tokens.count { |t| t.type == "PLUS" }
    assert_equal "+", tokens.find { |t| t.type == "PLUS" }.value
  end

  def test_minus_operator
    tokens = non_eof("10 LET X = 5 - 3\n")
    assert_equal 1, tokens.count { |t| t.type == "MINUS" }
    assert_equal "-", tokens.find { |t| t.type == "MINUS" }.value
  end

  def test_star_operator
    tokens = non_eof("10 LET X = 2 * 3\n")
    assert_equal 1, tokens.count { |t| t.type == "STAR" }
    assert_equal "*", tokens.find { |t| t.type == "STAR" }.value
  end

  def test_slash_operator
    tokens = non_eof("10 LET X = 6 / 2\n")
    assert_equal 1, tokens.count { |t| t.type == "SLASH" }
    assert_equal "/", tokens.find { |t| t.type == "SLASH" }.value
  end

  def test_caret_operator
    # ^ is exponentiation in BASIC: 2^3 = 8
    tokens = non_eof("10 LET X = 2 ^ 3\n")
    assert_equal 1, tokens.count { |t| t.type == "CARET" }
    assert_equal "^", tokens.find { |t| t.type == "CARET" }.value
  end

  def test_eq_operator
    tokens = non_eof("10 LET X = 5\n")
    assert_equal 1, tokens.count { |t| t.type == "EQ" }
    assert_equal "=", tokens.find { |t| t.type == "EQ" }.value
  end

  def test_lt_operator
    tokens = non_eof("10 IF X < Y THEN 50\n")
    assert_equal 1, tokens.count { |t| t.type == "LT" }
    assert_equal "<", tokens.find { |t| t.type == "LT" }.value
  end

  def test_gt_operator
    tokens = non_eof("10 IF X > Y THEN 50\n")
    assert_equal 1, tokens.count { |t| t.type == "GT" }
    assert_equal ">", tokens.find { |t| t.type == "GT" }.value
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_lparen
    tokens = non_eof("10 LET X = SIN(Y)\n")
    assert_equal 1, tokens.count { |t| t.type == "LPAREN" }
    assert_equal "(", tokens.find { |t| t.type == "LPAREN" }.value
  end

  def test_rparen
    tokens = non_eof("10 LET X = SIN(Y)\n")
    assert_equal 1, tokens.count { |t| t.type == "RPAREN" }
    assert_equal ")", tokens.find { |t| t.type == "RPAREN" }.value
  end

  def test_comma_in_print
    tokens = non_eof("10 PRINT X, Y\n")
    assert_equal 1, tokens.count { |t| t.type == "COMMA" }
    assert_equal ",", tokens.find { |t| t.type == "COMMA" }.value
  end

  def test_semicolon_in_print
    tokens = non_eof("10 PRINT X; Y\n")
    assert_equal 1, tokens.count { |t| t.type == "SEMICOLON" }
    assert_equal ";", tokens.find { |t| t.type == "SEMICOLON" }.value
  end

  def test_comma_vs_semicolon_both_present
    # PRINT X, Y; Z — uses both separators
    tokens = non_eof("10 PRINT X, Y; Z\n")
    assert_equal 1, tokens.count { |t| t.type == "COMMA" }
    assert_equal 1, tokens.count { |t| t.type == "SEMICOLON" }
  end

  # ------------------------------------------------------------------
  # NEWLINE tokens (significant in BASIC)
  # ------------------------------------------------------------------
  #
  # Unlike ALGOL 60 (where newlines are whitespace), Dartmouth BASIC
  # newlines are statement terminators and MUST appear in the token stream.
  # The grammar explicitly keeps NEWLINE tokens (they are NOT in the skip:
  # section of the grammar).

  def test_newline_present_in_stream
    tokens = non_eof("10 END\n")
    assert_equal 1, tokens.count { |t| t.type == "NEWLINE" }
  end

  def test_multiple_newlines
    source = "10 END\n20 STOP\n"
    tokens = non_eof(source)
    assert_equal 2, tokens.count { |t| t.type == "NEWLINE" }
  end

  def test_no_newline_at_end_of_source
    # If the source has no trailing newline, there is no NEWLINE token.
    tokens = non_eof("10 END")
    assert_equal 0, tokens.count { |t| t.type == "NEWLINE" }
  end

  def test_windows_style_newline_crlf
    # \r\n (Windows/teletype line endings) should produce a single NEWLINE.
    tokens = non_eof("10 END\r\n")
    assert_equal 1, tokens.count { |t| t.type == "NEWLINE" }
  end

  # ------------------------------------------------------------------
  # REM comment suppression
  # ------------------------------------------------------------------
  #
  # The suppress_rem_content_hook drops all tokens between a REM keyword
  # and the next NEWLINE. The REM token itself is kept; the NEWLINE is kept.

  def test_rem_comment_text_suppressed
    source = "10 REM THIS IS A COMMENT\n"
    tokens = non_eof(source)
    # Expected: LINE_NUM("10") KEYWORD("REM") NEWLINE
    # The comment text ("THIS IS A COMMENT") should be gone.
    assert_equal 3, tokens.length, "Expected LINE_NUM + REM + NEWLINE"
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "KEYWORD",  tokens[1].type
    assert_equal "REM",      tokens[1].value
    assert_equal "NEWLINE",  tokens[2].type
  end

  def test_rem_newline_is_kept
    # The NEWLINE after a REM line must be preserved (it's the terminator).
    source = "10 REM COMMENT\n20 END\n"
    tokens = non_eof(source)
    # Expect: LINE_NUM(10) REM NEWLINE LINE_NUM(20) END NEWLINE
    assert_equal 6, tokens.length
    assert_equal "NEWLINE",  tokens[2].type
    assert_equal "LINE_NUM", tokens[3].type
    assert_equal "20",       tokens[3].value
  end

  def test_rem_only_suppresses_own_line
    # REM on line 10 must not suppress tokens on line 20.
    source = "10 REM COMMENT\n20 LET X = 5\n"
    tokens = non_eof(source)
    let_tok = tokens.find { |t| t.type == "KEYWORD" && t.value == "LET" }
    refute_nil let_tok, "LET on line 20 should not be suppressed by REM on line 10"
  end

  def test_rem_without_text
    # REM at end of line with no comment text at all.
    source = "10 REM\n"
    tokens = non_eof(source)
    # Expected: LINE_NUM("10") KEYWORD("REM") NEWLINE
    assert_equal 3, tokens.length
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "KEYWORD",  tokens[1].type
    assert_equal "NEWLINE",  tokens[2].type
  end

  # ------------------------------------------------------------------
  # Complete statement sequences
  # ------------------------------------------------------------------
  #
  # These tests verify the full token stream for idiomatic BASIC statements.

  def test_let_statement_full_sequence
    # 10 LET X = 5
    tokens = non_eof("10 LET X = 5\n")
    token_types = tokens.map(&:type)
    token_values = tokens.map(&:value)
    assert_equal ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE"], token_types
    assert_equal ["10", "LET", "X", "=", "5", "\\n"], token_values
  end

  def test_print_statement
    # 20 PRINT X, Y
    tokens = non_eof("20 PRINT X, Y\n")
    assert_equal "LINE_NUM", tokens[0].type
    assert_equal "KEYWORD",  tokens[1].type; assert_equal "PRINT", tokens[1].value
    assert_equal "NAME",     tokens[2].type; assert_equal "X",     tokens[2].value
    assert_equal "COMMA",    tokens[3].type
    assert_equal "NAME",     tokens[4].type; assert_equal "Y",     tokens[4].value
    assert_equal "NEWLINE",  tokens[5].type
  end

  def test_goto_statement
    # 30 GOTO 10
    tokens = non_eof("30 GOTO 10\n")
    assert_equal ["LINE_NUM", "KEYWORD", "NUMBER", "NEWLINE"], tokens.map(&:type)
    assert_equal ["30", "GOTO", "10", "\\n"], tokens.map(&:value)
  end

  def test_if_then_statement
    # 40 IF X > 0 THEN 100
    tokens = non_eof("40 IF X > 0 THEN 100\n")
    token_types = tokens.map(&:type)
    assert_equal "LINE_NUM", token_types[0]
    assert_equal "KEYWORD",  token_types[1]; assert_equal "IF",   tokens[1].value
    assert_equal "NAME",     token_types[2]
    assert_equal "GT",       token_types[3]
    assert_equal "NUMBER",   token_types[4]
    assert_equal "KEYWORD",  token_types[5]; assert_equal "THEN", tokens[5].value
    assert_equal "NUMBER",   token_types[6]; assert_equal "100",  tokens[6].value
  end

  def test_for_to_step_statement
    # 50 FOR I = 1 TO 10 STEP 2
    tokens = non_eof("50 FOR I = 1 TO 10 STEP 2\n")
    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_equal ["FOR", "TO", "STEP"], keywords
  end

  def test_def_statement
    # 60 DEF FNA(X) = X * X
    tokens = non_eof("60 DEF FNA(X) = X * X\n")
    assert_equal "KEYWORD", tokens[1].type
    assert_equal "DEF",     tokens[1].value
    assert_equal "USER_FN", tokens[2].type
    assert_equal "FNA",     tokens[2].value
  end

  def test_let_with_builtin_function
    # 70 LET Y = SIN(X) + COS(X)
    tokens = non_eof("70 LET Y = SIN(X) + COS(X)\n")
    fn_tokens = tokens.select { |t| t.type == "BUILTIN_FN" }.map(&:value)
    assert_equal ["SIN", "COS"], fn_tokens
  end

  # ------------------------------------------------------------------
  # Multi-line programs
  # ------------------------------------------------------------------

  def test_multiline_program
    source = "10 LET X = 1\n20 PRINT X\n30 END\n"
    tokens = non_eof(source)

    line_nums = tokens.select { |t| t.type == "LINE_NUM" }.map(&:value)
    assert_equal ["10", "20", "30"], line_nums

    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_equal ["LET", "PRINT", "END"], keywords

    newlines = tokens.count { |t| t.type == "NEWLINE" }
    assert_equal 3, newlines
  end

  def test_multiline_with_rem_and_code
    source = "10 REM COMPUTE SQUARE\n20 LET X = 5\n30 LET Y = X ^ 2\n40 PRINT Y\n50 END\n"
    tokens = non_eof(source)

    # Line 10: LINE_NUM(10) KEYWORD(REM) NEWLINE  — comment text dropped
    first_three = tokens.first(3)
    assert_equal "LINE_NUM", first_three[0].type
    assert_equal "10",       first_three[0].value
    assert_equal "KEYWORD",  first_three[1].type
    assert_equal "REM",      first_three[1].value
    assert_equal "NEWLINE",  first_three[2].type

    # All 5 line numbers present.
    line_nums = tokens.select { |t| t.type == "LINE_NUM" }.map(&:value)
    assert_equal ["10", "20", "30", "40", "50"], line_nums
  end

  def test_program_without_trailing_newline
    # A program where the last line has no newline at the end.
    source = "10 LET X = 1\n20 PRINT X\n30 END"
    tokens = non_eof(source)
    line_nums = tokens.select { |t| t.type == "LINE_NUM" }.map(&:value)
    assert_equal ["10", "20", "30"], line_nums
  end

  # ------------------------------------------------------------------
  # FOR/NEXT loop structure
  # ------------------------------------------------------------------

  def test_for_next_loop
    source = "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n"
    tokens = non_eof(source)

    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_includes keywords, "FOR"
    assert_includes keywords, "TO"
    assert_includes keywords, "PRINT"
    assert_includes keywords, "NEXT"
  end

  # ------------------------------------------------------------------
  # GOSUB/RETURN subroutine structure
  # ------------------------------------------------------------------

  def test_gosub_return_structure
    source = "10 GOSUB 100\n20 END\n100 PRINT \"SUBROUTINE\"\n110 RETURN\n"
    tokens = non_eof(source)

    keywords = tokens.select { |t| t.type == "KEYWORD" }.map(&:value)
    assert_includes keywords, "GOSUB"
    assert_includes keywords, "END"
    assert_includes keywords, "PRINT"
    assert_includes keywords, "RETURN"
  end

  # ------------------------------------------------------------------
  # READ/DATA/RESTORE
  # ------------------------------------------------------------------

  def test_data_statement
    source = "10 DATA 1, 2, 3\n"
    tokens = non_eof(source)
    assert_equal "DATA", tokens.find { |t| t.type == "KEYWORD" }.value
    numbers = tokens.select { |t| t.type == "NUMBER" }.map(&:value)
    assert_equal ["1", "2", "3"], numbers
  end

  def test_read_statement
    source = "10 READ X\n"
    tokens = non_eof(source)
    assert_equal "READ", tokens.find { |t| t.type == "KEYWORD" }.value
  end

  def test_restore_statement
    source = "10 RESTORE\n"
    tokens = non_eof(source)
    assert_equal "RESTORE", tokens.find { |t| t.type == "KEYWORD" }.value
  end

  # ------------------------------------------------------------------
  # DIM statement
  # ------------------------------------------------------------------

  def test_dim_statement
    source = "10 DIM A(10)\n"
    tokens = non_eof(source)
    assert_equal "DIM", tokens.find { |t| t.type == "KEYWORD" }.value
    assert_equal 1, tokens.count { |t| t.type == "LPAREN" }
    assert_equal 1, tokens.count { |t| t.type == "RPAREN" }
  end

  # ------------------------------------------------------------------
  # Unrecognised character handling
  # ------------------------------------------------------------------
  #
  # The grammar's `errors:` section defines `UNKNOWN = /./` for error
  # recovery. However, the Ruby GrammarLexer does NOT implement the
  # `errors:` section — it stores it for compatibility but raises
  # CodingAdventures::Lexer::LexerError when no pattern matches.
  #
  # This means unrecognised characters cause a LexerError exception in
  # the Ruby implementation. The grammar file is still valid and the
  # `errors:` section will be used by other language implementations
  # (Python, TypeScript, Go) that DO support it.
  #
  # Dartmouth BASIC 1964 only uses printable ASCII characters that all
  # have valid tokens, so in practice this never occurs with valid BASIC.

  def test_unrecognised_at_sign_raises_error
    # @ is not a valid BASIC character. The Ruby lexer raises LexerError.
    assert_raises(CodingAdventures::Lexer::LexerError) do
      CodingAdventures::DartmouthBasicLexer.tokenize("10 LET @ = 1\n")
    end
  end

  def test_unrecognised_hash_raises_error
    # # is not a valid BASIC character either.
    assert_raises(CodingAdventures::Lexer::LexerError) do
      CodingAdventures::DartmouthBasicLexer.tokenize("10 LET # = 1\n")
    end
  end

  # ------------------------------------------------------------------
  # Line and column tracking
  # ------------------------------------------------------------------

  def test_line_tracking_line_one
    tokens = tokenize("10 LET X = 5\n")
    non_eof_toks = tokens.reject { |t| t.type == TT::EOF }
    assert_equal 1, non_eof_toks.first.line, "LINE_NUM token should be on line 1"
  end

  def test_line_tracking_across_newlines
    source = "10 END\n20 STOP\n"
    tokens = non_eof(source)
    line_num_20 = tokens.find { |t| t.type == "LINE_NUM" && t.value == "20" }
    assert_equal 2, line_num_20.line, "LINE_NUM 20 should be on source line 2"
  end

  def test_column_tracking_first_token
    tokens = tokenize("10 LET X = 5\n")
    first = tokens.first
    assert_equal 1, first.column, "First token should be at column 1"
  end

  # ------------------------------------------------------------------
  # Whitespace handling
  # ------------------------------------------------------------------
  #
  # Horizontal whitespace (spaces and tabs) is silently consumed.
  # Extra spaces between tokens do not produce any tokens.

  def test_extra_spaces_ignored
    tokens_normal = non_eof("10 LET X = 5\n")
    tokens_spaced = non_eof("10  LET  X  =  5\n")
    assert_equal tokens_normal.map(&:type),  tokens_spaced.map(&:type)
    assert_equal tokens_normal.map(&:value), tokens_spaced.map(&:value)
  end

  def test_tab_whitespace_ignored
    tokens_space = non_eof("10 LET X = 5\n")
    tokens_tab   = non_eof("10\tLET\tX\t=\t5\n")
    assert_equal tokens_space.map(&:type),  tokens_tab.map(&:type)
    assert_equal tokens_space.map(&:value), tokens_tab.map(&:value)
  end

  # ------------------------------------------------------------------
  # Complex expressions
  # ------------------------------------------------------------------

  def test_arithmetic_expression
    # LET X = 1 + 2 * 3 - 4 / 2
    tokens = non_eof("10 LET X = 1 + 2 * 3 - 4 / 2\n")
    token_types = tokens.reject { |t| t.type == "NEWLINE" }.map(&:type)
    expected = %w[LINE_NUM KEYWORD NAME EQ NUMBER PLUS NUMBER STAR NUMBER MINUS NUMBER SLASH NUMBER]
    assert_equal expected, token_types
  end

  def test_exponentiation_expression
    # LET Y = X ^ 2
    tokens = non_eof("10 LET Y = X ^ 2\n")
    assert_equal 1, tokens.count { |t| t.type == "CARET" }
  end

  def test_complex_builtin_expression
    # LET Y = SIN(X) + COS(X) - TAN(X)
    tokens = non_eof("10 LET Y = SIN(X) + COS(X) - TAN(X)\n")
    fn_values = tokens.select { |t| t.type == "BUILTIN_FN" }.map(&:value)
    assert_equal %w[SIN COS TAN], fn_values
  end

  def test_print_with_multiple_items_and_separators
    # PRINT A, B; C
    tokens = non_eof("10 PRINT A, B; C\n")
    assert_equal 3, tokens.count { |t| t.type == "NAME" }
    assert_equal 1, tokens.count { |t| t.type == "COMMA" }
    assert_equal 1, tokens.count { |t| t.type == "SEMICOLON" }
  end
end
