# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the ALGOL 60 Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with algol.grammar, correctly builds Abstract Syntax Trees from
# ALGOL 60 source text.
#
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "program", children: [...])
#
# Each node records which grammar rule produced it and its matched
# children (which can be tokens or other ASTNodes). This differs
# from a hand-written parser's typed nodes (Block, AssignStmt) but
# captures the same structural information.
#
# ALGOL 60's grammar (algol.grammar) has these top-level rules:
#   - program:     = block
#   - block:       = BEGIN { declaration ; } statement { ; statement } END
#   - declaration: type_decl | array_decl | switch_decl | procedure_decl
#   - statement:   [ label : ] unlabeled_stmt | cond_stmt
#
# Key ALGOL 60 grammar features tested:
#   - Keyword token detection (begin/end as block delimiters)
#   - Type declarations (integer, real, boolean, string)
#   - Assignment statements
#   - If/then/else conditionals (dangling-else avoidance)
#   - For loops with step/until
#   - Procedure declarations and calls
#   - Arithmetic expressions with operator precedence
#   - Boolean expressions with operator precedence
# ================================================================

class TestAlgolParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # ALGOL-specific token type strings
  INTEGER_LIT_TYPE = "INTEGER_LIT"
  REAL_LIT_TYPE    = "REAL_LIT"
  STRING_LIT_TYPE  = "STRING_LIT"
  IDENT_TYPE       = "NAME"
  ASSIGN_TYPE      = "ASSIGN"

  # ------------------------------------------------------------------
  # Helper
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::AlgolParser.parse(source)
  end

  # Recursively collect all Token objects from an AST.
  # Walks the entire tree depth-first, gathering every leaf token.
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

  # Find all ASTNode descendants with a given rule_name.
  # Useful for asserting that specific grammar rules were matched.
  def find_nodes_by_rule(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_nodes_by_rule(child, rule_name)) if child.is_a?(ASTNode)
    end
    results
  end

  # ------------------------------------------------------------------
  # Minimal program
  # ------------------------------------------------------------------
  # The simplest valid ALGOL 60 program is a block with a single
  # statement. The grammar requires at least one statement in a block.
  #
  #   begin integer x; x := 42 end

  def test_minimal_program
    ast = parse("begin integer x; x := 42 end")
    assert_equal "program", ast.rule_name

    # Should have a block node as the direct child
    block_nodes = find_nodes_by_rule(ast, "block")
    assert block_nodes.length >= 1, "Expected a block node"
  end

  def test_program_root_rule
    ast = parse("begin integer x; x := 42 end")
    assert_equal "program", ast.rule_name
  end

  def test_block_has_begin_and_end_tokens
    ast = parse("begin integer x; x := 42 end")
    all_tokens = collect_tokens(ast)
    values = all_tokens.map(&:value)
    assert_includes values, "begin"
    assert_includes values, "end"
  end

  # ------------------------------------------------------------------
  # Type declarations
  # ------------------------------------------------------------------
  # ALGOL 60 blocks may contain zero or more declarations before
  # the first statement. Type declarations introduce typed variables.

  def test_integer_declaration
    ast = parse("begin integer x; x := 1 end")
    type_decls = find_nodes_by_rule(ast, "type_decl")
    assert type_decls.length >= 1, "Expected a type_decl node"

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "integer" }, "Expected 'integer' keyword"
  end

  def test_real_declaration
    ast = parse("begin real r; r := 3.14 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "real" }, "Expected 'real' keyword"
  end

  def test_boolean_declaration
    ast = parse("begin boolean flag; flag := true end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "boolean" }, "Expected 'boolean' keyword"
  end

  def test_multiple_variable_declaration
    # integer x, y, z declares three variables in one declaration
    ast = parse("begin integer x, y, z; x := 1 end")
    type_decls = find_nodes_by_rule(ast, "type_decl")
    assert type_decls.length >= 1, "Expected a type_decl node"

    all_tokens = collect_tokens(ast)
    ident_tokens = all_tokens.select { |t| t.type == IDENT_TYPE }
    ident_values = ident_tokens.map(&:value)
    assert_includes ident_values, "x"
    assert_includes ident_values, "y"
    assert_includes ident_values, "z"
  end

  def test_multiple_declarations
    source = "begin integer x; real r; x := 1 end"
    ast = parse(source)
    type_decls = find_nodes_by_rule(ast, "type_decl")
    assert_equal 2, type_decls.length, "Expected 2 type_decl nodes"
  end

  # ------------------------------------------------------------------
  # Assignment statement
  # ------------------------------------------------------------------
  # x := 42  produces an assign_stmt node with a left_part and expression.

  def test_simple_assignment
    ast = parse("begin integer x; x := 42 end")
    assign_nodes = find_nodes_by_rule(ast, "assign_stmt")
    assert assign_nodes.length >= 1, "Expected an assign_stmt node"

    all_tokens = collect_tokens(ast)
    assign_token = all_tokens.find { |t| t.type == ASSIGN_TYPE }
    refute_nil assign_token, "Expected ASSIGN token"
    assert_equal ":=", assign_token.value
  end

  def test_assignment_value
    ast = parse("begin integer x; x := 42 end")
    all_tokens = collect_tokens(ast)
    int_token = all_tokens.find { |t| t.type == INTEGER_LIT_TYPE && t.value == "42" }
    refute_nil int_token, "Expected INTEGER_LIT '42'"
  end

  def test_assignment_real_value
    ast = parse("begin real r; r := 3.14 end")
    all_tokens = collect_tokens(ast)
    real_token = all_tokens.find { |t| t.type == REAL_LIT_TYPE }
    refute_nil real_token, "Expected REAL_LIT token"
    assert_equal "3.14", real_token.value
  end

  # ------------------------------------------------------------------
  # Arithmetic expressions
  # ------------------------------------------------------------------
  # ALGOL 60 arithmetic follows standard precedence:
  #   Exponentiation (** / ^) > Multiplication (* / div mod) > Addition (+ -)
  # This means x := 1 + 2 * 3 assigns 7 (not 9).

  def test_addition_expression
    ast = parse("begin integer x; x := 1 + 2 end")
    assert_equal "program", ast.rule_name

    all_tokens = collect_tokens(ast)
    plus_token = all_tokens.find { |t| t.value == "+" }
    refute_nil plus_token, "Expected PLUS token"
  end

  def test_multiplication_expression
    ast = parse("begin integer x; x := 2 * 3 end")
    all_tokens = collect_tokens(ast)
    star_token = all_tokens.find { |t| t.value == "*" }
    refute_nil star_token, "Expected STAR token"
  end

  def test_complex_arithmetic
    # x := 1 + 2 * 3  — precedence: 1 + (2*3) = 7
    ast = parse("begin integer x; x := 1 + 2 * 3 end")
    all_tokens = collect_tokens(ast)
    values = all_tokens.map(&:value)
    assert_includes values, "+"
    assert_includes values, "*"
    assert_includes values, "1"
    assert_includes values, "2"
    assert_includes values, "3"
  end

  def test_parenthesized_expression
    ast = parse("begin integer x; x := (1 + 2) * 3 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "(" }, "Expected LPAREN"
    assert all_tokens.any? { |t| t.value == ")" }, "Expected RPAREN"
  end

  def test_exponentiation_expression
    # x := 2 ** 3   (left-associative per ALGOL 60 report)
    ast = parse("begin integer x; x := 2 ** 3 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "**" }, "Expected POWER token"
  end

  def test_caret_exponentiation
    ast = parse("begin integer x; x := 2 ^ 3 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "^" }, "Expected CARET token"
  end

  def test_div_expression
    ast = parse("begin integer x; x := 10 div 3 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "div" }, "Expected 'div' keyword"
  end

  def test_mod_expression
    ast = parse("begin integer x; x := 10 mod 3 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "mod" }, "Expected 'mod' keyword"
  end

  def test_unary_minus
    ast = parse("begin integer x; x := -42 end")
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "-" }, "Expected MINUS token"
    assert all_tokens.any? { |t| t.value == "42" }, "Expected INTEGER_LIT '42'"
  end

  # ------------------------------------------------------------------
  # If/then/else conditional
  # ------------------------------------------------------------------
  # ALGOL 60 resolves the dangling-else ambiguity at the grammar level:
  # the then-branch is unlabeled_stmt (excludes conditionals), so
  # nesting requires begin...end.

  def test_if_then
    source = "begin integer x; if x > 0 then x := 1 end"
    ast = parse(source)
    cond_nodes = find_nodes_by_rule(ast, "cond_stmt")
    assert cond_nodes.length >= 1, "Expected a cond_stmt node"

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "if" }, "Expected 'if'"
    assert all_tokens.any? { |t| t.value == "then" }, "Expected 'then'"
  end

  def test_if_then_else
    source = "begin integer x; if x > 0 then x := 1 else x := 0 end"
    ast = parse(source)
    cond_nodes = find_nodes_by_rule(ast, "cond_stmt")
    assert cond_nodes.length >= 1, "Expected a cond_stmt node"

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "else" }, "Expected 'else'"
  end

  def test_if_then_else_values
    source = "begin integer x; if x > 0 then x := 1 else x := 0 end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    int_values = all_tokens.select { |t| t.type == INTEGER_LIT_TYPE }.map(&:value)
    assert_includes int_values, "1"
    assert_includes int_values, "0"
  end

  def test_conditional_with_leq
    source = "begin integer x; if x <= 10 then x := x + 1 end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "<=" }, "Expected LEQ token"
  end

  def test_conditional_with_boolean_literal
    source = "begin boolean flag; if true then flag := false end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "true" }, "Expected 'true'"
    assert all_tokens.any? { |t| t.value == "false" }, "Expected 'false'"
  end

  # ------------------------------------------------------------------
  # For loop
  # ------------------------------------------------------------------
  # ALGOL 60's for loop uses step/until for range iteration:
  #   for i := 1 step 1 until 10 do statement

  def test_for_loop_step_until
    source = "begin integer i; integer s; s := 0; for i := 1 step 1 until 10 do s := s + i end"
    ast = parse(source)
    for_nodes = find_nodes_by_rule(ast, "for_stmt")
    assert for_nodes.length >= 1, "Expected a for_stmt node"

    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "for" },   "Expected 'for'"
    assert all_tokens.any? { |t| t.value == "step" },  "Expected 'step'"
    assert all_tokens.any? { |t| t.value == "until" }, "Expected 'until'"
    assert all_tokens.any? { |t| t.value == "do" },    "Expected 'do'"
  end

  def test_for_loop_loop_variable
    source = "begin integer i; integer s; s := 0; for i := 1 step 1 until 5 do s := s + i end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    ident_tokens = all_tokens.select { |t| t.type == IDENT_TYPE }.map(&:value)
    assert_includes ident_tokens, "i"
  end

  def test_for_loop_with_do_body
    source = "begin integer i; for i := 1 step 1 until 3 do i := i + 1 end"
    ast = parse(source)
    for_nodes = find_nodes_by_rule(ast, "for_stmt")
    assert_equal 1, for_nodes.length, "Expected exactly one for_stmt"
  end

  # ------------------------------------------------------------------
  # Procedure call
  # ------------------------------------------------------------------
  # A procedure call as a statement: proc_stmt = IDENT [ ( params ) ]

  def test_procedure_call_no_args
    # In a valid ALGOL 60 program, a no-argument procedure call is just the name.
    source = "begin integer x; x := 0; print end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    ident_tokens = all_tokens.select { |t| t.type == IDENT_TYPE }.map(&:value)
    assert_includes ident_tokens, "print"
  end

  def test_procedure_call_with_args
    source = "begin integer x; x := 0; write(x) end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    ident_tokens = all_tokens.select { |t| t.type == IDENT_TYPE }.map(&:value)
    assert_includes ident_tokens, "write"
    assert_includes ident_tokens, "x"
  end

  def test_procedure_call_multiple_args
    source = "begin integer x; integer y; x := 1; y := 2; max(x, y) end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    ident_tokens = all_tokens.select { |t| t.type == IDENT_TYPE }.map(&:value)
    assert_includes ident_tokens, "max"
  end

  # ------------------------------------------------------------------
  # Boolean expressions
  # ------------------------------------------------------------------
  # ALGOL 60 uses word operators (not symbols): not, and, or, impl, eqv.
  # Precedence (lowest to highest): eqv, impl, or, and, not.

  def test_boolean_and
    source = "begin boolean flag; flag := true and false end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "and" }, "Expected 'and'"
  end

  def test_boolean_or
    source = "begin boolean flag; flag := true or false end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "or" }, "Expected 'or'"
  end

  def test_boolean_not
    source = "begin boolean flag; flag := not true end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "not" }, "Expected 'not'"
  end

  def test_boolean_impl
    source = "begin boolean a; boolean b; boolean c; c := a impl b end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "impl" }, "Expected 'impl'"
  end

  def test_boolean_eqv
    source = "begin boolean a; boolean b; boolean c; c := a eqv b end"
    ast = parse(source)
    all_tokens = collect_tokens(ast)
    assert all_tokens.any? { |t| t.value == "eqv" }, "Expected 'eqv'"
  end

  # ------------------------------------------------------------------
  # Compound statement (begin...end without declarations)
  # ------------------------------------------------------------------
  # A compound_stmt is BEGIN statement { ; statement } END with no declarations.
  # Used for grouping multiple statements where one is syntactically expected.

  def test_compound_statement
    source = "begin integer x; if true then begin x := 1 end end"
    ast = parse(source)
    compound_nodes = find_nodes_by_rule(ast, "compound_stmt")
    assert compound_nodes.length >= 1, "Expected a compound_stmt node"
  end

  # ------------------------------------------------------------------
  # Multiple statements in a block
  # ------------------------------------------------------------------

  def test_multiple_statements
    source = "begin integer x; integer y; x := 1; y := 2 end"
    ast = parse(source)
    assign_nodes = find_nodes_by_rule(ast, "assign_stmt")
    assert_equal 2, assign_nodes.length, "Expected 2 assign_stmt nodes"
  end

  def test_three_statements
    source = "begin integer x; integer y; integer z; x := 1; y := 2; z := x + y end"
    ast = parse(source)
    assign_nodes = find_nodes_by_rule(ast, "assign_stmt")
    assert_equal 3, assign_nodes.length, "Expected 3 assign_stmt nodes"
  end

  # ------------------------------------------------------------------
  # Realistic ALGOL 60 programs
  # ------------------------------------------------------------------
  # These tests exercise the full grammar with realistic code patterns.

  def test_factorial_style_loop
    # A classic iterative program: compute n! using a for loop.
    source = <<~ALGOL
      begin
        integer n;
        integer fact;
        n := 5;
        fact := 1;
        for n := 1 step 1 until 5 do
          fact := fact * n
      end
    ALGOL
    ast = parse(source)
    assert_equal "program", ast.rule_name

    for_nodes = find_nodes_by_rule(ast, "for_stmt")
    assert_equal 1, for_nodes.length, "Expected one for_stmt"

    assign_nodes = find_nodes_by_rule(ast, "assign_stmt")
    assert assign_nodes.length >= 2, "Expected assignment statements"
  end

  def test_conditional_chain
    # if/then/else if chain — the else branch can be a full statement
    # (including another conditional).
    source = "begin integer x; integer y; if x > 0 then y := 1 else if x < 0 then y := -1 else y := 0 end"
    ast = parse(source)
    cond_nodes = find_nodes_by_rule(ast, "cond_stmt")
    assert cond_nodes.length >= 2, "Expected at least 2 cond_stmt nodes (chained if/else)"
  end

  def test_nested_block
    # A nested block introduces its own declarations
    source = "begin integer x; x := 1; begin integer y; y := 2 end end"
    ast = parse(source)
    block_nodes = find_nodes_by_rule(ast, "block")
    assert block_nodes.length >= 2, "Expected at least 2 block nodes (outer + inner)"
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::AlgolParser::ALGOL_GRAMMAR_PATH),
      "algol.grammar file should exist at #{CodingAdventures::AlgolParser::ALGOL_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Root node rule
  # ------------------------------------------------------------------

  def test_root_is_program
    ast = parse("begin integer x; x := 1 end")
    assert_equal "program", ast.rule_name
  end
end
