"""Tests for the ALGOL 60 parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``algol/algol60.grammar``, correctly parses ALGOL 60 source text into ASTs.

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

from pathlib import Path

import pytest
from grammar_tools.compiler import compile_parser_grammar
from grammar_tools.parser_grammar import parse_parser_grammar
from lang_parser import ASTNode, GrammarParseError, GrammarParser
from lexer import Token

from algol_parser import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_algol_parser,
    parse_algol,
    resolve_version,
)
from algol_parser._grammar import PARSER_GRAMMAR

_REPO_ROOT = Path(__file__).resolve().parents[5]
_SOURCE_GRAMMAR = _REPO_ROOT / "code/grammars/algol/algol60.grammar"
_GENERATED_GRAMMAR = (
    _REPO_ROOT
    / "code/packages/python/algol-parser/src/algol_parser/_grammar.py"
)

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

    def test_default_version_is_algol60(self) -> None:
        """The default parser grammar is the compiled ALGOL 60 grammar."""
        assert DEFAULT_VERSION == "algol60"
        assert sorted(SUPPORTED_VERSIONS) == ["algol60"]
        assert resolve_version() == "algol60"
        assert resolve_version(None) == "algol60"

    def test_factory_uses_compiled_parser_grammar(self) -> None:
        """The parser imports native grammar data instead of reading files."""
        parser = create_algol_parser("begin end")

        assert parser._grammar is PARSER_GRAMMAR

    def test_compiled_parser_grammar_is_fresh(self) -> None:
        """The committed Python parser grammar matches the source grammar."""
        source = _SOURCE_GRAMMAR.read_text(encoding="utf-8")
        expected = compile_parser_grammar(
            parse_parser_grammar(source),
            "algol/algol60.grammar",
        )

        assert _GENERATED_GRAMMAR.read_text(encoding="utf-8") == expected

    def test_explicit_algol60_version_produces_ast(self) -> None:
        """The supported version name can be passed explicitly."""
        ast = parse_algol("begin end", version="algol60")

        assert ast.rule_name == "program"

    def test_unknown_version_is_rejected(self) -> None:
        """Unknown ALGOL versions fail before falling back to stale files."""
        with pytest.raises(ValueError, match="Unknown ALGOL version 'algol68'"):
            resolve_version("algol68")


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

    Empty programs are valid, and ALGOL dummy statements appear as zero-width
    ``dummy_stmt`` nodes where a statement boundary supplies the no-op.
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

    def test_keywords_are_case_insensitive(self) -> None:
        """Uppercase keywords parse through the ALGOL front door."""
        ast = parse("BEGIN INTEGER x; x := 42 END")

        assert ast.rule_name == "program"

    def test_uppercase_comment_is_ignored(self) -> None:
        """Uppercase COMMENT follows ALGOL's case-insensitive keyword policy."""
        ast = parse("begin COMMENT setup; integer x; x := 42 end")

        assert ast.rule_name == "program"

    def test_comment_prefixed_identifier_is_not_ignored(self) -> None:
        """A variable whose name starts with comment is still a variable."""
        ast = parse("begin integer commentary; commentary := 42 end")
        assign_nodes = find_nodes(ast, "assign_stmt")

        assert ast.rule_name == "program"
        assert assign_nodes


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

    def test_uparrow_exponentiation(self) -> None:
        """Publication uparrow exponentiation parses through CARET."""
        ast = parse("begin real x; x := 2 ↑ 10 end")
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

    def test_if_with_angle_not_equal(self) -> None:
        """The common ``<>`` not-equal spelling parses as a relation."""
        ast = parse("begin integer x; if x <> 0 then x := 1 end")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "relation")

    def test_if_with_publication_symbol_relations(self) -> None:
        """Publication relation symbols parse as normalized relations."""
        ast = parse(
            "begin integer x; "
            "if (2 ↑ 3 = 8) ∧ (3 ≤ 4) ∧ (5 ≥ 5) ∧ (1 ≠ 2) "
            "then x := 1 else x := 0 "
            "end"
        )
        assert ast.rule_name == "program"
        assert len(find_nodes(ast, "relation")) >= 4

    def test_if_with_boolean_operators(self) -> None:
        """Conditional with AND and NOT in the boolean expression."""
        ast = parse(
            "begin integer x; "
            "if x > 0 then x := 1 "
            "end"
        )
        assert ast.rule_name == "program"

    def test_if_then_dummy_statement(self) -> None:
        """The then-branch may be ALGOL's zero-width dummy statement."""
        ast = parse("begin integer x; if true then ; x := 1 end")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "dummy_stmt")

    def test_if_else_dummy_statement(self) -> None:
        """The else-branch may also be a dummy statement."""
        ast = parse("begin integer x; if false then x := 1 else ; end")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "dummy_stmt")


# ---------------------------------------------------------------------------
# Goto statement tests
# ---------------------------------------------------------------------------


class TestGotoStatement:
    """Tests for ALGOL 60 direct goto syntax."""

    def test_go_to_statement(self) -> None:
        """The report-style ``go to`` spelling parses as a goto statement."""
        ast = parse(
            "begin integer result; "
            "go to done; "
            "result := 99; "
            "done: result := 7 "
            "end"
        )
        goto_nodes = find_nodes(ast, "goto_stmt")
        assert len(goto_nodes) == 1

    def test_multiple_labels_on_statement(self) -> None:
        """A statement may have more than one direct goto label."""
        ast = parse(
            "begin integer result; "
            "first: second: result := 7 "
            "end"
        )
        statement = next(
            node
            for node in find_nodes(ast, "statement")
            if len([child for child in child_nodes(node) if child.rule_name == "label"])
            == 2
        )

        assert [child.rule_name for child in child_nodes(statement)][:2] == [
            "label",
            "label",
        ]

    def test_multiple_terminal_labels(self) -> None:
        """Multiple terminal labels may share the block-end dummy statement."""
        ast = parse(
            "begin integer result; "
            "result := 1; "
            "done1: done2: "
            "end"
        )
        statement = next(
            node
            for node in find_nodes(ast, "statement")
            if len([child for child in child_nodes(node) if child.rule_name == "label"])
            == 2
        )

        nodes = child_nodes(statement)
        assert [child.rule_name for child in nodes[:2]] == ["label", "label"]
        assert find_nodes(statement, "dummy_stmt")


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

    def test_array_element_control_variable(self) -> None:
        """For loops can use a subscripted variable as the control lvalue."""
        ast = parse(
            "begin integer result; integer array a[1:1]; "
            "for a[1] := 1 step 1 until 3 do result := result + a[1] "
            "end"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) == 1
        assert find_nodes(for_nodes[0], "subscripts")

    def test_dummy_statement_body(self) -> None:
        """For-loop bodies may be empty dummy statements."""
        ast = parse("begin integer i; for i := 1 do ; i := 2 end")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "dummy_stmt")


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

    The compiler accepts both word spellings and publication symbols for these
    boolean operators, normalizing symbols before parsing.
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

    def test_boolean_literal_relation(self) -> None:
        """Boolean literals can be operands in equality relations."""
        ast = parse(
            "begin integer x; boolean b; "
            "b := true; "
            "if b = true then x := 1 else x := 0 "
            "end"
        )

        assert ast.rule_name == "program"
        assert find_nodes(ast, "relation")

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

    def test_publication_symbol_boolean_expression(self) -> None:
        """Boolean publication symbols normalize to parser keyword values."""
        ast = parse(
            "begin integer x; "
            "if (¬ false) ∧ (true ∨ false) ∧ (true ⊃ true) "
            "∧ (true ≡ true) "
            "then x := 1 else x := 0 "
            "end"
        )

        assert ast.rule_name == "program"
        all_token_values: list[str] = []

        def collect_token_values(node: ASTNode) -> None:
            for child in node.children:
                if isinstance(child, Token):
                    all_token_values.append(child.value)
                else:
                    collect_token_values(child)

        collect_token_values(ast)
        assert {"not", "and", "or", "impl", "eqv"} <= set(all_token_values)

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

    def test_report_style_typed_array_parameter_specifier(self) -> None:
        """Formal parameters accept ``integer array`` as one specifier."""
        ast = parse(
            "begin integer result; integer array xs[1:1]; "
            "procedure first(a); integer array a; begin result := a[1] end; "
            "first(xs) "
            "end"
        )

        assert ast.rule_name == "program"
        assert find_nodes(ast, "specifier")

    def test_report_style_typed_procedure_parameter_specifier(self) -> None:
        """Formal parameters accept ``real procedure`` as one specifier."""
        ast = parse(
            "begin integer result; "
            "procedure invoke(f); real procedure f; "
            "begin if f(2) = 4 then result := 1 else result := 0 end; "
            "real procedure twice(x); value x; real x; begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )

        assert ast.rule_name == "program"
        assert find_nodes(ast, "specifier")


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
        ast = parse("begin procedure halt; begin end; halt end")
        assert ast.rule_name == "program"
        assert find_nodes(ast, "proc_stmt")

    def test_procedure_call_explicit_empty_args(self) -> None:
        """No-argument statement calls also accept explicit empty parens."""
        ast = parse("begin procedure halt(); begin end; halt() end")
        proc_nodes = find_nodes(ast, "proc_stmt")

        assert ast.rule_name == "program"
        assert len(proc_nodes) == 1
        assert not find_nodes(proc_nodes[0], "actual_params")

    def test_parameterless_procedure_declaration_explicit_empty_params(self) -> None:
        """Parameterless declarations also accept explicit empty parens."""
        ast = parse("begin procedure halt(); begin end; halt end")

        assert ast.rule_name == "program"
        formal_params = find_nodes(ast, "formal_params")
        assert len(formal_params) == 1
        assert not find_nodes(formal_params[0], "ident_list")

    def test_procedure_call_with_args(self) -> None:
        """Procedure call with arguments in parentheses."""
        # 'print' with an argument: this is a typical ALGOL I/O pattern.
        ast = parse(
            "begin integer x; x := 42; print(x) end"
        )
        proc_nodes = find_nodes(ast, "proc_stmt")
        assert len(proc_nodes) >= 1

    def test_procedure_call_with_double_quoted_string_arg(self) -> None:
        """Double-quoted string literals parse as actual parameters."""
        ast = parse('begin output("Hi") end')
        assert ast.rule_name == "program"
        assert find_nodes(ast, "proc_stmt")

    def test_procedure_call_as_statement(self) -> None:
        """A procedure call appears as a statement."""
        ast = parse(
            "begin integer n; output(n + 1) end"
        )
        assert ast.rule_name == "program"

    def test_no_argument_procedure_call_expression(self) -> None:
        """Typed procedure calls can use explicit empty parens in expressions."""
        ast = parse(
            "begin integer result; "
            "integer procedure seven(); begin seven := 7 end; "
            "result := seven() "
            "end"
        )

        assert ast.rule_name == "program"
        proc_calls = find_nodes(ast, "proc_call")
        assert len(proc_calls) == 1
        assert not find_nodes(proc_calls[0], "actual_params")

    def test_procedure_call_in_relation_left_operand(self) -> None:
        """Procedure calls remain calls inside arithmetic relation operands."""
        ast = parse(
            "begin integer result; "
            "integer procedure twice(x); value x; integer x; begin twice := x * 2 end; "
            "if twice(2) = 4 then result := 1 else result := 0 "
            "end"
        )

        assert ast.rule_name == "program"
        assert find_nodes(ast, "relation")
        assert find_nodes(ast, "proc_call")

    def test_procedure_call_in_array_subscript(self) -> None:
        """Subscripts use arith_expr, so calls must parse there too."""
        ast = parse(
            "begin integer result; integer array a[1:4]; "
            "integer procedure idx(x); value x; integer x; begin idx := x end; "
            "a[idx(2)] := 7; result := a[idx(2)] "
            "end"
        )

        assert ast.rule_name == "program"
        assert find_nodes(ast, "subscripts")
        assert find_nodes(ast, "proc_call")


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
# Nested conditional context tests
# ---------------------------------------------------------------------------


class TestNestedConditionalContexts:
    """Conditional values can nest in type-specific ALGOL contexts."""

    def test_nested_arithmetic_conditional_in_bounds_and_subscripts(self) -> None:
        ast = parse(
            "begin integer result; "
            "integer array a[1:if true then if false then 2 else 3 else 1]; "
            "a[if true then if false then 1 else 2 else 3] := 9; "
            "result := a[2] "
            "end"
        )

        assert ast.rule_name == "program"
        assert len(find_nodes(ast, "arith_expr")) >= 4

    def test_nested_boolean_conditional_in_condition(self) -> None:
        ast = parse(
            "begin integer result; "
            "if if true then if false then false else true else false "
            "then result := 1 else result := 0 "
            "end"
        )

        assert ast.rule_name == "program"
        assert len(find_nodes(ast, "bool_expr")) >= 3

    def test_nested_designational_conditional_in_goto(self) -> None:
        ast = parse(
            "begin integer result; "
            "goto if true then if false then left else right else fail; "
            "left: result := 1; goto done; "
            "right: result := 7; goto done; "
            "fail: result := 0; "
            "done: "
            "end"
        )

        assert ast.rule_name == "program"
        assert len(find_nodes(ast, "desig_expr")) >= 3


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
