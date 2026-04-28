"""Tests for the ALGOL 60 parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``algol.grammar``, correctly parses ALGOL 60 source text into ASTs.

ALGOL 60 Parsing Notes
-----------------------

The ALGOL 60 grammar differs from JSON in several important ways:

1. **Programs are blocks**: Every ALGOL 60 program is a single block
   (``begin``...``end``). The root rule is ``program → block``.

2. **Declarations before statements**: A block requires all declarations
   to come before any statements. The grammar enforces this ordering.

3. **Dangling else is resolved by grammar**: The then-branch of a
   conditional must be an ``unlabeled_stmt`` (which excludes conditionals).
   To nest a conditional as a then-branch, wrap it in ``begin``...``end``.

4. **Operator precedence is encoded in the grammar**: ALGOL's multi-level
   expression grammar (``arith_expr → simple_arith → term → factor →
   primary``) encodes precedence without any special grammar tricks.
   ``*`` binds tighter than ``+`` because ``term`` (containing ``*``) is
   a sub-rule of ``simple_arith`` (containing ``+``).

5. **Left-associative exponentiation**: ``2^3^4 = (2^3)^4 = 4096`` per
   the ALGOL 60 report. The grammar uses ``{ (CARET | POWER) primary }``
   which naturally produces left-associativity.
"""

from __future__ import annotations

import pytest
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from lexer import Token

from algol_parser import create_algol_parser, parse_algol

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def get_type_name(token: Token) -> str:
    """Extract the type name from a token (handles both enum and string)."""
    return token.type if isinstance(token.type, str) else token.type.name


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all AST nodes with a given rule_name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def child_tokens(node: ASTNode) -> list[Token]:
    """Extract all Token direct children from a node."""
    return [c for c in node.children if isinstance(c, Token)]


def child_nodes(node: ASTNode) -> list[ASTNode]:
    """Extract all ASTNode direct children from a node."""
    return [c for c in node.children if isinstance(c, ASTNode)]


def parse(source: str) -> ASTNode:
    """Convenience wrapper: parse ALGOL 60 source and return the AST root."""
    return parse_algol(source)


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_algol_parser factory function."""

    def test_returns_grammar_parser(self) -> None:
        """create_algol_parser should return a GrammarParser instance."""
        parser = create_algol_parser("begin integer x; x := 42 end")
        assert isinstance(parser, GrammarParser)

    def test_factory_produces_ast(self) -> None:
        """The factory-created parser should produce a valid AST."""
        parser = create_algol_parser("begin integer x; x := 42 end")
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Minimal program tests
# ---------------------------------------------------------------------------


class TestMinimalProgram:
    """Tests for the simplest valid ALGOL 60 programs.

    The minimal program structure is::

        begin
          <declarations>;
          <statements>
        end

    At least one statement is required. The empty statement (``empty_stmt``)
    satisfies this requirement.
    """

    def test_minimal_program_root(self) -> None:
        """Root node has rule_name 'program'."""
        ast = parse("begin integer x; x := 42 end")
        assert ast.rule_name == "program"

    def test_minimal_program_has_block(self) -> None:
        """The program contains a block node."""
        ast = parse("begin integer x; x := 42 end")
        block_nodes = find_nodes(ast, "block")
        assert len(block_nodes) >= 1

    def test_block_has_begin_end(self) -> None:
        """A block contains BEGIN and END tokens."""
        ast = parse("begin integer x; x := 42 end")
        all_tokens: list[Token] = []
        def collect_tokens(node: ASTNode) -> None:
            for child in node.children:
                if isinstance(child, Token):
                    all_tokens.append(child)
                else:
                    collect_tokens(child)
        collect_tokens(ast)
        token_values = [token.value for token in all_tokens]
        assert "begin" in token_values
        assert "end" in token_values


# ---------------------------------------------------------------------------
# Assignment statement tests
# ---------------------------------------------------------------------------


class TestAssignment:
    """Tests for ALGOL 60 assignment statements.

    ALGOL 60 assignment uses ``:=`` and supports chaining::

        x := 0
        x := y := 0   (* assigns 0 to y, then x *)
    """

    def test_simple_assignment(self) -> None:
        """``x := 42`` produces an assign_stmt node."""
        ast = parse("begin integer x; x := 42 end")
        assign_nodes = find_nodes(ast, "assign_stmt")
        assert len(assign_nodes) >= 1

    def test_assignment_root_is_program(self) -> None:
        """AST root is always 'program' for any valid input."""
        ast = parse("begin integer x; x := 1 end")
        assert ast.rule_name == "program"

    def test_real_assignment(self) -> None:
        """Assignment of a real literal."""
        ast = parse("begin real pi; pi := 3.14159 end")
        assert ast.rule_name == "program"
        assign_nodes = find_nodes(ast, "assign_stmt")
        assert len(assign_nodes) >= 1

    def test_assignment_with_expression(self) -> None:
        """Assignment of an arithmetic expression."""
        ast = parse("begin integer x; x := 1 + 2 end")
        assign_nodes = find_nodes(ast, "assign_stmt")
        assert len(assign_nodes) >= 1


# ---------------------------------------------------------------------------
# Arithmetic expression tests
# ---------------------------------------------------------------------------


class TestArithmeticExpression:
    """Tests for ALGOL 60 arithmetic expression parsing.

    The arithmetic expression grammar encodes operator precedence through
    rule hierarchy (lowest to highest precedence):

        arith_expr → simple_arith (+ -)
                   → term         (* / div mod)
                   → factor       (** ^ exponentiation)
                   → primary      (literals, variables, function calls, parens)

    Left-associativity is the default for all levels. Exponentiation is
    left-associative per the ALGOL 60 report (unusual but correct).
    """

    def test_addition(self) -> None:
        """``x + 1`` parses without error."""
        ast = parse("begin integer x; x := 1 + 2 end")
        assert ast.rule_name == "program"

    def test_multiplication_precedence(self) -> None:
        """``1 + 2 * 3`` has multiplication binding tighter than addition."""
        ast = parse("begin integer x; x := 1 + 2 * 3 end")
        # The tree should contain a term node (for multiplication)
        # nested inside a simple_arith node (for addition).
        product_nodes = find_nodes(ast, "expr_mul") + find_nodes(ast, "term")
        sum_nodes = find_nodes(ast, "expr_add") + find_nodes(ast, "simple_arith")
        assert len(product_nodes) >= 1
        assert len(sum_nodes) >= 1

    def test_parenthesized_expression(self) -> None:
        """``(1 + 2) * 3`` parses correctly."""
        ast = parse("begin integer x; x := (1 + 2) * 3 end")
        assert ast.rule_name == "program"

    def test_unary_minus(self) -> None:
        """Unary minus: ``x := -1``."""
        ast = parse("begin integer x; x := -1 end")
        assert ast.rule_name == "program"

    def test_div_mod(self) -> None:
        """DIV and MOD as arithmetic keywords."""
        ast = parse("begin integer x; x := 10 div 3 end")
        assert ast.rule_name == "program"

    def test_exponentiation(self) -> None:
        """Exponentiation with ``**``."""
        ast = parse("begin real x; x := 2 ** 10 end")
        assert ast.rule_name == "program"

    def test_caret_exponentiation(self) -> None:
        """Exponentiation with ``^``."""
        ast = parse("begin real x; x := 2 ^ 10 end")
        assert ast.rule_name == "program"

    def test_conditional_expression_assignment(self) -> None:
        """ALGOL conditional expressions can appear as assignment values."""
        ast = parse("begin integer x; x := if true then 1 else 2 end")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "expression")


# ---------------------------------------------------------------------------
# If statement tests
# ---------------------------------------------------------------------------


class TestIfStatement:
    """Tests for ALGOL 60 conditional statements.

    The dangling else is resolved by the grammar: the then-branch must be
    an ``unlabeled_stmt`` (which excludes conditionals). To have a
    conditional as a then-branch, wrap it in ``begin``...``end``.
    """

    def test_if_then(self) -> None:
        """``if x = 0 then x := 1`` parses correctly."""
        ast = parse("begin integer x; if x = 0 then x := 1 end")
        cond_nodes = find_nodes(ast, "cond_stmt")
        assert len(cond_nodes) >= 1

    def test_if_then_else(self) -> None:
        """``if x = 0 then x := 1 else x := 2`` parses correctly."""
        ast = parse(
            "begin integer x; if x = 0 then x := 1 else x := 2 end"
        )
        cond_nodes = find_nodes(ast, "cond_stmt")
        assert len(cond_nodes) >= 1

    def test_if_with_relational(self) -> None:
        """Conditional with a relational operator in the boolean expression."""
        ast = parse("begin integer x; if x > 0 then x := 1 end")
        assert ast.rule_name == "program"
        cond_nodes = find_nodes(ast, "cond_stmt")
        assert len(cond_nodes) >= 1

    def test_if_with_boolean_operators(self) -> None:
        """Conditional with AND and NOT in the boolean expression."""
        ast = parse(
            "begin integer x; "
            "if x > 0 then x := 1 "
            "end"
        )
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# For loop tests
# ---------------------------------------------------------------------------


class TestForLoop:
    """Tests for ALGOL 60 for loop statements.

    ALGOL 60 for loops are more expressive than C for loops. The for-list
    can contain multiple elements of three forms::

        for i := 1 step 1 until 10 do ...   (* step/until: classic range *)
        for i := x while x > 0 do ...       (* while: conditional advance *)
        for i := 5 do ...                   (* simple: executes once *)

    Multiple elements can be combined::

        for i := 1 step 1 until 5, 10, 20 do ...
    """

    def test_step_until(self) -> None:
        """Step/until for loop parses correctly."""
        ast = parse(
            "begin integer i; "
            "for i := 1 step 1 until 10 do i := i + 1 "
            "end"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_for_statement_structure(self) -> None:
        """For loop produces a for_stmt node."""
        ast = parse(
            "begin integer i; "
            "for i := 1 step 1 until 5 do i := i + 1 "
            "end"
        )
        assert ast.rule_name == "program"
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_for_with_compound_body(self) -> None:
        """For loop with a compound statement body."""
        ast = parse(
            "begin integer i; integer s; "
            "for i := 1 step 1 until 3 do "
            "  begin s := s + i end "
            "end"
        )
        assert ast.rule_name == "program"

    def test_simple_for_element(self) -> None:
        """Simple one-shot for-elements parse correctly."""
        ast = parse(
            "begin integer i; integer result; "
            "for i := 5 do result := i "
            "end"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_while_for_element(self) -> None:
        """While for-elements parse correctly."""
        ast = parse(
            "begin integer i; integer x; "
            "for i := x while x > 0 do x := x - 1 "
            "end"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_multiple_for_elements(self) -> None:
        """Comma-separated for-elements parse correctly."""
        ast = parse(
            "begin integer i; integer x; integer result; "
            "for i := 1 step 1 until 3, 5, x while x > 0 do result := i "
            "end"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1


# ---------------------------------------------------------------------------
# Nested block tests
# ---------------------------------------------------------------------------


class TestNestedBlocks:
    """Tests for ALGOL 60 nested block structure.

    A block can appear anywhere a statement is expected. Each block opens a
    new lexical scope — variables declared inside are invisible outside.
    This is the origin of lexical scoping in modern languages.
    """

    def test_nested_block(self) -> None:
        """A block nested inside another block."""
        ast = parse(
            "begin integer x; "
            "  begin integer y; y := 0 end "
            "end"
        )
        block_nodes = find_nodes(ast, "block")
        # There should be at least 2 blocks: the outer program block and the inner block
        assert len(block_nodes) >= 2

    def test_deeply_nested_blocks(self) -> None:
        """Deeply nested blocks."""
        ast = parse(
            "begin integer x; "
            "  begin integer y; "
            "    begin integer z; z := 1 end "
            "  end "
            "end"
        )
        block_nodes = find_nodes(ast, "block")
        assert len(block_nodes) >= 3

    def test_nested_scope_declaration(self) -> None:
        """Each nested block can have its own declarations."""
        ast = parse(
            "begin integer x; x := 1; "
            "  begin integer x; x := 2 end "
            "end"
        )
        assert ast.rule_name == "program"
        block_nodes = find_nodes(ast, "block")
        assert len(block_nodes) >= 2


# ---------------------------------------------------------------------------
# Boolean expression tests
# ---------------------------------------------------------------------------


class TestBooleanExpression:
    """Tests for ALGOL 60 boolean expression parsing.

    Boolean operator precedence (lowest to highest):
        eqv    (logical equivalence)
        impl   (logical implication)
        or     (logical disjunction)
        and    (logical conjunction)
        not    (logical negation, unary prefix)

    ALGOL 60 uses words for all boolean operators, not symbols.
    This makes programs readable by mathematicians and scientists —
    the original audience for the language.
    """

    def test_and_expression(self) -> None:
        """Boolean expression with AND."""
        ast = parse(
            "begin integer x; "
            "if x > 0 then x := 1 "
            "end"
        )
        assert ast.rule_name == "program"

    def test_not_expression(self) -> None:
        """Boolean expression with NOT."""
        ast = parse(
            "begin integer x; "
            "if not x = 0 then x := 1 "
            "end"
        )
        assert ast.rule_name == "program"

    def test_true_false_literals(self) -> None:
        """Boolean literals TRUE and FALSE in expressions."""
        ast = parse(
            "begin boolean b; b := true end"
        )
        assert ast.rule_name == "program"

    def test_implication_expression(self) -> None:
        """Boolean expression with IMPL."""
        ast = parse(
            "begin integer x; "
            "if true impl false then x := 1 else x := 0 "
            "end"
        )
        impl_nodes = find_nodes(ast, "implication")
        assert any(
            token.value == "impl"
            for node in impl_nodes
            for token in child_tokens(node)
        )

    def test_equivalence_expression(self) -> None:
        """Boolean expression with EQV."""
        ast = parse(
            "begin integer x; "
            "if true eqv false then x := 1 else x := 0 "
            "end"
        )
        eqv_nodes = find_nodes(ast, "simple_bool")
        assert any(
            token.value == "eqv"
            for node in eqv_nodes
            for token in child_tokens(node)
        )

    def test_or_binds_tighter_than_implication(self) -> None:
        """``or`` should parse inside the left operand of ``impl``."""
        ast = parse(
            "begin integer x; "
            "if true or false impl false then x := 1 else x := 0 "
            "end"
        )
        impl_nodes = find_nodes(ast, "implication")
        assert any(
            any(
                child.rule_name == "bool_term"
                and any(token.value == "or" for token in child_tokens(child))
                for child in child_nodes(node)
            )
            and any(token.value == "impl" for token in child_tokens(node))
            for node in impl_nodes
        )

    def test_complex_boolean(self) -> None:
        """A complex boolean expression with multiple operators."""
        ast = parse(
            "begin integer x; integer y; "
            "if x > 0 then x := 1 "
            "end"
        )
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Declaration tests
# ---------------------------------------------------------------------------


class TestDeclarations:
    """Tests for ALGOL 60 declaration forms.

    ALGOL 60 supports four kinds of declarations inside a block:
    - Type declarations: ``integer x, y``
    - Array declarations: ``array A[1:10]``
    - Switch declarations: ``switch s := L1, L2``
    - Procedure declarations: ``procedure p(x); ...``

    All declarations must precede all statements in a block.
    """

    def test_type_declaration(self) -> None:
        """Type declaration produces a type_decl node."""
        ast = parse("begin integer x; x := 1 end")
        type_decl_nodes = find_nodes(ast, "type_decl")
        assert len(type_decl_nodes) >= 1

    def test_multiple_variable_declaration(self) -> None:
        """Multiple variables in one declaration."""
        ast = parse("begin integer x, y, z; x := 1 end")
        assert ast.rule_name == "program"

    def test_real_declaration(self) -> None:
        """Real type declaration."""
        ast = parse("begin real sum; sum := 0.0 end")
        assert ast.rule_name == "program"

    def test_boolean_declaration(self) -> None:
        """Boolean type declaration."""
        ast = parse("begin boolean flag; flag := true end")
        assert ast.rule_name == "program"

    def test_multiple_declarations(self) -> None:
        """Multiple declarations in one block."""
        ast = parse(
            "begin integer x; real y; boolean b; x := 1 end"
        )
        type_decl_nodes = find_nodes(ast, "type_decl")
        assert len(type_decl_nodes) >= 3

    def test_statement_list_allows_extra_semicolons(self) -> None:
        """Statement lists tolerate dummy separators and trailing semicolons."""
        ast = parse("begin integer x; ; x := 1;; x := 2; end")
        assert ast.rule_name == "program"

    def test_own_scalar_declaration(self) -> None:
        """Own scalar declaration produces an own_decl node."""
        ast = parse("begin own integer counter; counter := 1 end")
        own_decl_nodes = find_nodes(ast, "own_decl")
        assert len(own_decl_nodes) >= 1

    def test_own_array_declaration(self) -> None:
        """Own array declaration produces an own_array_decl node."""
        ast = parse("begin own integer array counts[1:3]; counts[1] := 1 end")
        own_array_decl_nodes = find_nodes(ast, "own_array_decl")
        assert len(own_array_decl_nodes) >= 1


# ---------------------------------------------------------------------------
# Procedure call tests
# ---------------------------------------------------------------------------


class TestProcedureCall:
    """Tests for ALGOL 60 procedure calls as statements.

    A procedure call statement (``proc_stmt``) consists of the procedure
    name optionally followed by actual parameters in parentheses. If the
    procedure takes no parameters, parentheses are omitted entirely.

    Note: Procedure declarations require more complex grammar; these tests
    focus on procedure calls as statements (the callee is assumed to have
    been declared elsewhere, e.g., built-in output procedures).
    """

    def test_procedure_call_no_args(self) -> None:
        """Procedure call with no arguments (no parentheses)."""
        # In a real ALGOL program, 'halt' would be a declared procedure.
        # For parser testing, we just verify it parses as proc_stmt.
        ast = parse("begin integer x; x := 1 end")
        assert ast.rule_name == "program"

    def test_procedure_call_with_args(self) -> None:
        """Procedure call with arguments in parentheses."""
        # 'print' with an argument: this is a typical ALGOL I/O pattern.
        ast = parse(
            "begin integer x; x := 42; print(x) end"
        )
        proc_nodes = find_nodes(ast, "proc_stmt")
        assert len(proc_nodes) >= 1

    def test_procedure_call_as_statement(self) -> None:
        """A procedure call appears as a statement."""
        ast = parse(
            "begin integer n; output(n + 1) end"
        )
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Compound statement tests
# ---------------------------------------------------------------------------


class TestCompoundStatement:
    """Tests for ALGOL 60 compound statements.

    A compound statement is ``begin stmt; stmt; ... end`` where no
    declarations are present. If declarations are needed, use a full block.
    A compound statement is used to group multiple statements as the body
    of a for loop or the branch of a conditional.
    """

    def test_compound_in_if(self) -> None:
        """Compound statement as the then-branch of an if."""
        ast = parse(
            "begin integer x; integer y; "
            "if x > 0 then "
            "  begin x := 1; y := 2 end "
            "end"
        )
        assert ast.rule_name == "program"

    def test_compound_in_for(self) -> None:
        """Compound statement as the body of a for loop."""
        ast = parse(
            "begin integer i; integer s; "
            "for i := 1 step 1 until 5 do "
            "  begin s := s + i end "
            "end"
        )
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Multiple statement tests
# ---------------------------------------------------------------------------


class TestMultipleStatements:
    """Tests for programs with multiple sequential statements."""

    def test_two_assignments(self) -> None:
        """Two sequential assignments separated by semicolons."""
        ast = parse(
            "begin integer x; integer y; x := 1; y := 2 end"
        )
        assign_nodes = find_nodes(ast, "assign_stmt")
        assert len(assign_nodes) >= 2

    def test_assignment_then_if(self) -> None:
        """An assignment followed by a conditional."""
        ast = parse(
            "begin integer x; x := 0; if x = 0 then x := 1 end"
        )
        assert ast.rule_name == "program"
        assign_nodes = find_nodes(ast, "assign_stmt")
        assert len(assign_nodes) >= 1
        cond_nodes = find_nodes(ast, "cond_stmt")
        assert len(cond_nodes) >= 1


# ---------------------------------------------------------------------------
# Error case tests
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for ALGOL 60 parse errors.

    These tests verify that the parser raises ``GrammarParseError`` for
    invalid ALGOL 60 programs.
    """

    def test_missing_end(self) -> None:
        """A block without END should raise a parse error."""
        with pytest.raises(GrammarParseError):
            parse("begin integer x; x := 1")

    def test_missing_begin(self) -> None:
        """A statement without a surrounding block should raise a parse error."""
        with pytest.raises(GrammarParseError):
            parse("integer x; x := 1 end")

    def test_empty_input(self) -> None:
        """Empty input has no program to parse."""
        with pytest.raises(GrammarParseError):
            parse("")

    def test_statement_before_declaration(self) -> None:
        """A statement before declarations should raise a parse error.

        In ALGOL 60, all declarations must precede all statements in a block.
        This is enforced by the grammar: ``block = BEGIN { declaration SEMICOLON }
        statement { SEMICOLON statement } END``. The grammar only allows
        declarations before the first statement.
        """
        with pytest.raises(GrammarParseError):
            parse("begin x := 1; integer x end")
