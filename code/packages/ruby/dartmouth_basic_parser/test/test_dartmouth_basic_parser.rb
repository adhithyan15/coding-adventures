# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the Dartmouth BASIC 1964 Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded with
# dartmouth_basic.grammar, correctly builds Abstract Syntax Trees from
# 1964 Dartmouth BASIC source code.
#
# What We're Testing
# -------------------
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "program", children: [...])
#
# Each node records which grammar rule produced it and its matched
# children (which can be tokens or other ASTNodes). The structure
# mirrors the EBNF grammar in dartmouth_basic.grammar.
#
# Test Organization
# -----------------
# - Factory: the parse() class method returns the correct type
# - Root: the root node is always "program"
# - Statements: all 17 statement types in the 1964 spec
# - Expressions: precedence, right-assoc ^, unary minus, parentheses
# - Built-in functions: all 11 (SIN, COS, TAN, ATN, EXP, LOG, ABS,
#                                SQR, INT, RND, SGN)
# - User functions: FNA-style calls
# - Arrays: subscript access and assignment
# - Multi-line programs: hello world, loops, conditionals, subroutines
# - Grammar path: the grammar file exists and is accessible
# - Error cases: missing =, missing THEN, missing TO
# ================================================================

class TestDartmouthBasicParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode

  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  # Parse a BASIC source string and return the root ASTNode.
  def parse(source)
    CodingAdventures::DartmouthBasicParser.parse(source)
  end

  # Recursively find all ASTNode descendants with a given rule_name.
  # Walks the tree depth-first. Returns an array of matching nodes.
  def find_nodes(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_nodes(child, rule_name)) if child.is_a?(ASTNode)
    end
    results
  end

  # Recursively collect all Token leaf nodes from an AST in order.
  def collect_tokens(node)
    tokens = []
    return tokens unless node.is_a?(ASTNode)

    node.children.each do |child|
      if child.is_a?(CodingAdventures::Lexer::Token)
        tokens << child
      elsif child.is_a?(ASTNode)
        tokens.concat(collect_tokens(child))
      end
    end
    tokens
  end

  # ------------------------------------------------------------------
  # Factory / parse() method
  # ------------------------------------------------------------------
  # The module-level parse() method is the primary API.

  def test_parse_returns_ast_node
    ast = parse("10 END\n")
    assert_instance_of ASTNode, ast
  end

  def test_parse_root_is_program
    ast = parse("10 END\n")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # Root and empty programs
  # ------------------------------------------------------------------

  def test_empty_program
    # An empty source string is a valid program with zero lines.
    ast = parse("")
    assert_equal "program", ast.rule_name
    lines = find_nodes(ast, "line")
    assert_equal 0, lines.length
  end

  def test_bare_line_number
    # "10\n" is valid: line = LINE_NUM [ statement ] NEWLINE
    # The [ statement ] is optional; a bare line number is a no-op statement.
    ast = parse("10\n")
    assert_equal "program", ast.rule_name
    lines = find_nodes(ast, "line")
    assert_equal 1, lines.length
  end

  # ------------------------------------------------------------------
  # LET statement
  # ------------------------------------------------------------------
  # Grammar: let_stmt = "LET" variable EQ expr ;
  #
  # LET is explicit assignment. The 1964 spec requires the LET keyword
  # (unlike BASIC dialects from the 1970s onward that made it optional).

  def test_let_simple
    ast = parse("10 LET X = 5\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  def test_let_expression
    # LET with a compound arithmetic expression
    ast = parse("10 LET X = 2 + 3 * 4\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  def test_let_variable_rhs
    ast = parse("10 LET Y = X\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  def test_let_array_subscript_lhs
    # Array element as the assignment target
    ast = parse("10 LET A(3) = 7\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # PRINT statement
  # ------------------------------------------------------------------
  # Grammar:
  #   print_stmt = "PRINT" [ print_list ] ;
  #   print_list = print_item { print_sep print_item } [ print_sep ] ;
  #   print_sep  = COMMA | SEMICOLON ;
  #   print_item = expr | STRING ;
  #
  # PRINT is the richest statement in 1964 BASIC:
  # - Comma separators put values in fixed 15-character columns.
  # - Semicolon separators suppress all extra spacing.
  # - A trailing separator suppresses the final newline.

  def test_print_bare
    # PRINT with no args outputs a blank line
    ast = parse("10 PRINT\n")
    stmts = find_nodes(ast, "print_stmt")
    assert_equal 1, stmts.length
  end

  def test_print_expression
    ast = parse("10 PRINT X + 1\n")
    stmts = find_nodes(ast, "print_stmt")
    assert_equal 1, stmts.length
  end

  def test_print_string_literal
    # String literals in 1964 BASIC are double-quoted.
    # There are no string variables — strings can only appear in PRINT.
    ast = parse("10 PRINT \"HELLO\"\n")
    stmts = find_nodes(ast, "print_stmt")
    assert_equal 1, stmts.length
  end

  def test_print_comma_separated
    # Comma: fixed-column spacing (every 15 characters)
    ast = parse("10 PRINT X, Y\n")
    stmts = find_nodes(ast, "print_stmt")
    assert_equal 1, stmts.length
  end

  def test_print_semicolon_separated
    # Semicolon: print values immediately adjacent
    ast = parse("10 PRINT X; Y\n")
    stmts = find_nodes(ast, "print_stmt")
    assert_equal 1, stmts.length
  end

  def test_print_trailing_comma
    # Trailing comma suppresses the final newline
    ast = parse("10 PRINT X,\n")
    stmts = find_nodes(ast, "print_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # INPUT statement
  # ------------------------------------------------------------------
  # Grammar: input_stmt = "INPUT" variable { COMMA variable } ;
  #
  # INPUT pauses execution, prints a ? prompt, and reads typed values.

  def test_input_single
    ast = parse("10 INPUT X\n")
    stmts = find_nodes(ast, "input_stmt")
    assert_equal 1, stmts.length
  end

  def test_input_multiple
    ast = parse("10 INPUT A, B, C\n")
    stmts = find_nodes(ast, "input_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # IF statement and all relational operators
  # ------------------------------------------------------------------
  # Grammar:
  #   if_stmt = "IF" expr relop expr "THEN" LINE_NUM ;
  #   relop   = EQ | LT | GT | LE | GE | NE ;
  #
  # There is no ELSE. The branch target must be a literal line number.
  # Six relational operators: =, <, >, <=, >=, <>

  def test_if_greater_than
    ast = parse("10 IF X > 0 THEN 100\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  def test_if_less_than
    ast = parse("10 IF X < 10 THEN 200\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  def test_if_equal
    ast = parse("10 IF X = Y THEN 50\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  def test_if_not_equal
    # <> is the not-equal operator in 1964 BASIC
    ast = parse("10 IF X <> Y THEN 30\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  def test_if_less_equal
    ast = parse("10 IF X <= 5 THEN 40\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  def test_if_greater_equal
    ast = parse("10 IF X >= 5 THEN 60\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  def test_if_with_expressions
    # Full arithmetic expressions on both sides of the relop
    ast = parse("10 IF X + 1 > Y * 2 THEN 99\n")
    stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # GOTO statement
  # ------------------------------------------------------------------
  # Grammar: goto_stmt = "GOTO" LINE_NUM ;
  #
  # Unconditional jump. Often considered harmful in modern programming,
  # but in 1964 BASIC it was one of the few flow-control mechanisms.

  def test_goto
    ast = parse("10 GOTO 50\n")
    stmts = find_nodes(ast, "goto_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # GOSUB / RETURN
  # ------------------------------------------------------------------
  # Grammar:
  #   gosub_stmt  = "GOSUB" LINE_NUM ;
  #   return_stmt = "RETURN" ;
  #
  # GOSUB pushes the return address and jumps to the subroutine.
  # RETURN pops the return address and resumes. This provides a
  # primitive form of parametrised code reuse.

  def test_gosub
    ast = parse("10 GOSUB 200\n")
    stmts = find_nodes(ast, "gosub_stmt")
    assert_equal 1, stmts.length
  end

  def test_return
    ast = parse("200 RETURN\n")
    stmts = find_nodes(ast, "return_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # FOR / NEXT loop
  # ------------------------------------------------------------------
  # Grammar:
  #   for_stmt  = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ] ;
  #   next_stmt = "NEXT" NAME ;
  #
  # FOR...NEXT is the only structured loop in 1964 BASIC. STEP defaults
  # to 1; negative STEP counts down.

  def test_for_without_step
    # FOR I = 1 TO 10 with implicit STEP 1
    ast = parse("10 FOR I = 1 TO 10\n20 NEXT I\n")
    for_stmts = find_nodes(ast, "for_stmt")
    next_stmts = find_nodes(ast, "next_stmt")
    assert_equal 1, for_stmts.length
    assert_equal 1, next_stmts.length
  end

  def test_for_with_step
    # Counting down: FOR I = 10 TO 1 STEP -1
    ast = parse("10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n")
    for_stmts = find_nodes(ast, "for_stmt")
    assert_equal 1, for_stmts.length
  end

  def test_for_with_positive_step
    ast = parse("10 FOR I = 0 TO 100 STEP 5\n20 NEXT I\n")
    for_stmts = find_nodes(ast, "for_stmt")
    assert_equal 1, for_stmts.length
  end

  # ------------------------------------------------------------------
  # END and STOP
  # ------------------------------------------------------------------
  # Grammar:
  #   end_stmt  = "END" ;
  #   stop_stmt = "STOP" ;
  #
  # END is the normal program terminus. STOP halts with a message and
  # can be resumed from the DTSS prompt.

  def test_end
    ast = parse("10 END\n")
    stmts = find_nodes(ast, "end_stmt")
    assert_equal 1, stmts.length
  end

  def test_stop
    ast = parse("10 STOP\n")
    stmts = find_nodes(ast, "stop_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # REM statement
  # ------------------------------------------------------------------
  # Grammar: rem_stmt = "REM" ;
  #
  # The lexer's post-tokenize hook strips everything between REM and
  # NEWLINE. By the time the parser sees the tokens, a REM line is just:
  # LINE_NUM KEYWORD("REM") NEWLINE. So rem_stmt matches only "REM".

  def test_rem_with_text
    ast = parse("10 REM THIS IS A COMMENT\n")
    stmts = find_nodes(ast, "rem_stmt")
    assert_equal 1, stmts.length
  end

  def test_rem_empty
    ast = parse("10 REM\n")
    stmts = find_nodes(ast, "rem_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # READ / DATA / RESTORE
  # ------------------------------------------------------------------
  # Grammar:
  #   read_stmt    = "READ" variable { COMMA variable } ;
  #   data_stmt    = "DATA" NUMBER { COMMA NUMBER } ;
  #   restore_stmt = "RESTORE" ;
  #
  # DATA embeds a pool of numeric constants. READ pops from the pool.
  # RESTORE resets the pool pointer. This allowed programs to embed
  # lookup tables without file I/O.

  def test_read_single
    ast = parse("10 READ X\n")
    stmts = find_nodes(ast, "read_stmt")
    assert_equal 1, stmts.length
  end

  def test_read_multiple
    ast = parse("10 READ A, B, C\n")
    stmts = find_nodes(ast, "read_stmt")
    assert_equal 1, stmts.length
  end

  def test_data_single
    ast = parse("10 DATA 42\n")
    stmts = find_nodes(ast, "data_stmt")
    assert_equal 1, stmts.length
  end

  def test_data_multiple
    ast = parse("10 DATA 1, 2, 3\n")
    stmts = find_nodes(ast, "data_stmt")
    assert_equal 1, stmts.length
  end

  def test_restore
    ast = parse("10 RESTORE\n")
    stmts = find_nodes(ast, "restore_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # DIM statement
  # ------------------------------------------------------------------
  # Grammar:
  #   dim_stmt = "DIM" dim_decl { COMMA dim_decl } ;
  #   dim_decl = NAME LPAREN NUMBER RPAREN ;
  #
  # Without DIM, arrays default to indices 0-10. DIM allows larger
  # arrays. The size must be a literal integer (not an expression).

  def test_dim_single
    ast = parse("10 DIM A(10)\n")
    stmts = find_nodes(ast, "dim_stmt")
    assert_equal 1, stmts.length
  end

  def test_dim_multiple
    ast = parse("10 DIM A(10), B(20)\n")
    stmts = find_nodes(ast, "dim_stmt")
    assert_equal 1, stmts.length
    decls = find_nodes(ast, "dim_decl")
    assert_equal 2, decls.length
  end

  # ------------------------------------------------------------------
  # DEF statement
  # ------------------------------------------------------------------
  # Grammar: def_stmt = "DEF" USER_FN LPAREN NAME RPAREN EQ expr ;
  #
  # DEF defines a user-named function (FNA through FNZ). Each takes
  # exactly one argument. The body can reference the parameter and
  # any global variables.

  def test_def_simple
    ast = parse("10 DEF FNA(X) = X * X\n")
    stmts = find_nodes(ast, "def_stmt")
    assert_equal 1, stmts.length
  end

  def test_def_with_builtin
    ast = parse("10 DEF FNB(T) = SIN(T) / COS(T)\n")
    stmts = find_nodes(ast, "def_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # Expression precedence
  # ------------------------------------------------------------------
  # Precedence (lowest to highest):
  #   + and -  (addition/subtraction, left-assoc)
  #   * and /  (multiplication/division, left-assoc)
  #   ^        (exponentiation, RIGHT-assoc)
  #   unary -
  #   primary  (literals, function calls, parenthesised expressions)
  #
  # Right-associativity of ^ is achieved by the grammar rule:
  #   power = unary [ CARET power ]
  # This makes 2^3^2 = 2^(3^2) = 512, matching standard mathematics.

  def test_addition_and_multiplication_precedence
    # 2 + 3 * 4 should be 2 + (3*4) = 14, not (2+3)*4 = 20
    ast = parse("10 LET X = 2 + 3 * 4\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
    exprs = find_nodes(ast, "expr")
    assert exprs.length >= 1
  end

  def test_right_assoc_exponentiation
    # 2 ^ 3 ^ 2 = 2 ^ (3^2) = 2^9 = 512 (not (2^3)^2 = 64)
    ast = parse("10 LET X = 2 ^ 3 ^ 2\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
    # Right-assoc creates nested power nodes
    power_nodes = find_nodes(ast, "power")
    assert power_nodes.length >= 2
  end

  def test_unary_minus
    ast = parse("10 LET X = -Y\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
    unary_nodes = find_nodes(ast, "unary")
    assert unary_nodes.length >= 1
  end

  def test_parentheses
    # (2 + 3) * 4 = 20 (parens override precedence)
    ast = parse("10 LET X = (2 + 3) * 4\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # Built-in functions (all 11)
  # ------------------------------------------------------------------
  # Grammar: primary = BUILTIN_FN LPAREN expr RPAREN | ...
  #
  # The 11 built-in functions in the 1964 spec:
  #   SIN, COS, TAN, ATN  — trigonometric (radians)
  #   EXP, LOG            — exponential and natural log
  #   ABS, SQR            — absolute value and square root
  #   INT                 — floor integer (rounds toward -infinity)
  #   RND                 — random number 0..1 (argument is a dummy)
  #   SGN                 — sign: -1, 0, or +1

  %w[SIN COS TAN ATN EXP LOG ABS SQR INT RND SGN].each do |fn|
    define_method("test_builtin_#{fn.downcase}") do
      ast = parse("10 LET Y = #{fn}(X)\n")
      stmts = find_nodes(ast, "let_stmt")
      assert_equal 1, stmts.length, "Expected let_stmt when using #{fn}"
      primary_nodes = find_nodes(ast, "primary")
      assert primary_nodes.length >= 1, "Expected primary node for #{fn} call"
    end
  end

  # ------------------------------------------------------------------
  # User-defined functions
  # ------------------------------------------------------------------
  # Grammar: primary = USER_FN LPAREN expr RPAREN | ...
  #
  # After DEF FNA(X) = expr, the function is called as FNA(value).
  # Names run from FNA through FNZ (26 possible user functions).

  def test_user_function_call
    ast = parse("10 LET Y = FNA(X)\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  def test_user_function_with_expr_arg
    ast = parse("10 LET Y = FNB(X + 1)\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # Array subscript access
  # ------------------------------------------------------------------
  # Grammar: variable = NAME LPAREN expr RPAREN | NAME ;
  #
  # Arrays are single-dimensional in 1964 BASIC. The subscript is an
  # arbitrary expression (unlike DIM sizes which must be literals).

  def test_array_read
    ast = parse("10 LET X = A(3)\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  def test_array_with_variable_index
    ast = parse("10 LET X = A(I)\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  def test_array_write
    ast = parse("10 LET A(5) = 42\n")
    stmts = find_nodes(ast, "let_stmt")
    assert_equal 1, stmts.length
  end

  # ------------------------------------------------------------------
  # Multi-line programs
  # ------------------------------------------------------------------
  # These tests exercise the program rule ({line}) and the interaction
  # between different statement types across multiple lines.

  def test_hello_world
    # The canonical first BASIC program — simpler than most modern hello worlds
    source = "10 PRINT \"HELLO WORLD\"\n20 END\n"
    ast = parse(source)
    assert_equal "program", ast.rule_name
    lines = find_nodes(ast, "line")
    assert_equal 2, lines.length
  end

  def test_counting_loop
    source = <<~BASIC
      10 FOR I = 1 TO 10
      20 PRINT I
      30 NEXT I
      40 END
    BASIC
    ast = parse(source)
    lines = find_nodes(ast, "line")
    assert_equal 4, lines.length
    for_stmts = find_nodes(ast, "for_stmt")
    assert_equal 1, for_stmts.length
    next_stmts = find_nodes(ast, "next_stmt")
    assert_equal 1, next_stmts.length
  end

  def test_conditional_program
    source = <<~BASIC
      10 LET X = 5
      20 IF X > 3 THEN 50
      30 PRINT 0
      40 GOTO 60
      50 PRINT 1
      60 END
    BASIC
    ast = parse(source)
    lines = find_nodes(ast, "line")
    assert_equal 6, lines.length
    if_stmts = find_nodes(ast, "if_stmt")
    assert_equal 1, if_stmts.length
  end

  def test_subroutine_program
    source = <<~BASIC
      10 GOSUB 100
      20 GOSUB 100
      30 END
      100 PRINT "IN SUBROUTINE"
      110 RETURN
    BASIC
    ast = parse(source)
    gosub_stmts = find_nodes(ast, "gosub_stmt")
    return_stmts = find_nodes(ast, "return_stmt")
    assert_equal 2, gosub_stmts.length
    assert_equal 1, return_stmts.length
  end

  def test_read_data_program
    source = <<~BASIC
      10 READ X
      20 READ Y
      30 LET Z = X + Y
      40 PRINT Z
      50 DATA 3, 4
      60 END
    BASIC
    ast = parse(source)
    read_stmts = find_nodes(ast, "read_stmt")
    data_stmts = find_nodes(ast, "data_stmt")
    assert_equal 2, read_stmts.length
    assert_equal 1, data_stmts.length
  end

  # ------------------------------------------------------------------
  # Grammar path
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::DartmouthBasicParser::DARTMOUTH_BASIC_GRAMMAR_PATH),
      "dartmouth_basic.grammar should exist at #{CodingAdventures::DartmouthBasicParser::DARTMOUTH_BASIC_GRAMMAR_PATH}"
  end

  def test_grammar_path_is_a_file
    assert File.file?(CodingAdventures::DartmouthBasicParser::DARTMOUTH_BASIC_GRAMMAR_PATH)
  end

  # ------------------------------------------------------------------
  # Error cases
  # ------------------------------------------------------------------
  # The parser must raise an error for invalid BASIC syntax.
  # We use StandardError because the exact class (GrammarParseError)
  # may be wrapped differently across parser versions.

  def test_error_missing_equals_in_let
    # "LET X 5" is missing the required = sign
    # Grammar: let_stmt = "LET" variable EQ expr
    assert_raises(StandardError) { parse("10 LET X 5\n") }
  end

  def test_error_missing_then_in_if
    # "IF X > 0 100" is missing the required THEN keyword
    # Grammar: if_stmt = "IF" expr relop expr "THEN" LINE_NUM
    assert_raises(StandardError) { parse("10 IF X > 0 100\n") }
  end

  def test_error_missing_to_in_for
    # "FOR I = 1 10" is missing the required TO keyword
    # Grammar: for_stmt = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ]
    assert_raises(StandardError) { parse("10 FOR I = 1 10\n") }
  end
end
