"""Tests for the Nib parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``nib.grammar``, correctly parses Nib source text into ASTs.

Nib Parsing Notes
------------------

The Nib grammar differs from ALGOL 60 in several important ways:

1. **No top-level block**: Nib programs are a flat sequence of top-level
   declarations (``const``, ``static``, ``fn``). There is no enclosing
   ``begin``/``end``. The root rule is ``program = { top_decl }``.

2. **Empty programs are valid**: Because ``program`` is ``{ top_decl }``
   (zero or more), an empty source string is valid and produces a
   ``program`` node with no children.

3. **Dangling-else resolved by braces**: Every branch of an ``if``/``else``
   must be a full brace-delimited block. There is no single-statement form.
   This eliminates the dangling-else ambiguity that plagues C-family grammars.

4. **8-level expression precedence**: Operator precedence is encoded entirely
   in the grammar hierarchy (logical < equality < relational < additive <
   bitwise < unary). No external precedence tables are needed.

5. **Explicit overflow operators**: Nib has ``+%`` (wrapping add) and ``+?``
   (saturating add) at the same precedence as ordinary ``+``/``-``. The
   programmer always chooses their overflow semantics explicitly.

6. **RANGE is a single token**: The ``..`` in ``for i: u8 in 0..10 { }``
   is a single RANGE token produced by the lexer, not two dot operators.
   This simplifies the parser rule for ``for_stmt``.
"""

from __future__ import annotations

import pytest

from lang_parser import ASTNode, GrammarParser, GrammarParseError
from lexer import Token
from nib_parser import create_nib_parser, parse_nib


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
    """Convenience wrapper: parse Nib source and return the AST root."""
    return parse_nib(source)


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_nib_parser factory function."""

    def test_returns_grammar_parser(self) -> None:
        """create_nib_parser should return a GrammarParser instance."""
        parser = create_nib_parser("fn main() { }")
        assert isinstance(parser, GrammarParser)

    def test_factory_produces_ast(self) -> None:
        """The factory-created parser should produce a valid AST."""
        parser = create_nib_parser("fn main() { }")
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "program"

    def test_factory_empty_source(self) -> None:
        """Factory should handle empty source and produce a program node."""
        parser = create_nib_parser("")
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# Basic top-level construct tests
# ---------------------------------------------------------------------------


class TestTopLevel:
    """Tests for the three top-level declaration forms.

    Nib programs are flat sequences of ``const``, ``static``, and ``fn``
    declarations. There is no enclosing block — the program is just a list
    of these three forms.

    This keeps the compilation model simple: declarations are collected in
    one pass, then compiled. The compiler can process a Nib file in O(n) time
    without needing to maintain a complex scope stack at the top level.
    """

    def test_parse_empty_program(self) -> None:
        """An empty source string is a valid program with no declarations."""
        ast = parse("")
        assert ast.rule_name == "program"

    def test_parse_const_decl(self) -> None:
        """const MAX: u8 = 10; produces a const_decl node."""
        ast = parse("const MAX: u8 = 10;")
        assert ast.rule_name == "program"
        const_nodes = find_nodes(ast, "const_decl")
        assert len(const_nodes) >= 1

    def test_parse_static_decl(self) -> None:
        """static x: u4 = 0; produces a static_decl node."""
        ast = parse("static x: u4 = 0;")
        assert ast.rule_name == "program"
        static_nodes = find_nodes(ast, "static_decl")
        assert len(static_nodes) >= 1

    def test_parse_multiple_top_decls(self) -> None:
        """Multiple top-level declarations in sequence."""
        ast = parse("const A: u4 = 1;\nstatic b: u8 = 0;\nfn main() { }")
        assert ast.rule_name == "program"
        const_nodes = find_nodes(ast, "const_decl")
        static_nodes = find_nodes(ast, "static_decl")
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(const_nodes) >= 1
        assert len(static_nodes) >= 1
        assert len(fn_nodes) >= 1


# ---------------------------------------------------------------------------
# Function declaration tests
# ---------------------------------------------------------------------------


class TestFunctionDeclaration:
    """Tests for Nib function declarations.

    A function declaration has the form::

        fn NAME ( [param_list] ) [-> type] block

    The return type is optional: omitting ``-> type`` declares a void function.
    The parameter list is also optional: ``fn main()`` has zero parameters.

    WHY OPTIONAL RETURN TYPE:
    The Intel 4004 calling convention uses the accumulator (A) for a single
    return nibble. Void functions simply don't write to A before returning.
    Making the return type optional at the grammar level matches this reality.
    """

    def test_parse_fn_no_params(self) -> None:
        """fn main() { } — void function with no parameters."""
        ast = parse("fn main() { }")
        assert ast.rule_name == "program"
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 1

    def test_parse_fn_with_params(self) -> None:
        """fn add(a: u4, b: u4) -> u4 { return a; } — two parameters."""
        ast = parse("fn add(a: u4, b: u4) -> u4 { return a; }")
        assert ast.rule_name == "program"
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 1

    def test_parse_fn_return_type(self) -> None:
        """Function with explicit return type produces fn_decl."""
        ast = parse("fn get_max() -> u8 { return 255; }")
        assert ast.rule_name == "program"
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 1

    def test_parse_fn_single_param(self) -> None:
        """Function with a single parameter."""
        ast = parse("fn double(x: u4) -> u4 { return x; }")
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 1

    def test_parse_fn_bool_return(self) -> None:
        """Function returning bool."""
        ast = parse("fn is_zero(x: u4) -> bool { return x == 0; }")
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 1

    def test_parse_multiple_functions(self) -> None:
        """Multiple function declarations produce multiple fn_decl nodes."""
        ast = parse("fn a() { }\nfn b() { }\nfn main() { }")
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 3


# ---------------------------------------------------------------------------
# Statement tests
# ---------------------------------------------------------------------------


class TestStatements:
    """Tests for Nib statement forms.

    Nib has six statement types:

    1. ``let_stmt``    — introduces a new variable into the current block scope
    2. ``assign_stmt`` — mutates an existing variable (let, static, or loop var)
    3. ``return_stmt`` — exits the current function, optionally with a value
    4. ``for_stmt``    — iterates over an integer range with const bounds
    5. ``if_stmt``     — conditionally executes a brace-delimited block
    6. ``expr_stmt``   — calls a function in statement position (side effects)

    All statements within a function body appear inside a ``block``
    (``{ stmt... }``). Blocks create lexical scope for ``let`` declarations.
    """

    def test_parse_let_stmt(self) -> None:
        """let x: u4 = 5; produces a let_stmt node."""
        ast = parse("fn main() { let x: u4 = 5; }")
        let_nodes = find_nodes(ast, "let_stmt")
        assert len(let_nodes) >= 1

    def test_parse_assign_stmt(self) -> None:
        """x = 5; (without let) produces an assign_stmt node."""
        ast = parse("static x: u4 = 0;\nfn main() { x = 5; }")
        assign_nodes = find_nodes(ast, "assign_stmt")
        assert len(assign_nodes) >= 1

    def test_parse_return_stmt(self) -> None:
        """return 1; produces a return_stmt node."""
        ast = parse("fn f() -> u4 { return 1; }")
        return_nodes = find_nodes(ast, "return_stmt")
        assert len(return_nodes) >= 1

    def test_parse_for_stmt(self) -> None:
        """for i: u8 in 0..10 { } produces a for_stmt node."""
        ast = parse("fn main() { for i: u8 in 0..10 { } }")
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_parse_if_stmt(self) -> None:
        """if true { } produces an if_stmt node."""
        ast = parse("fn main() { if true { } }")
        if_nodes = find_nodes(ast, "if_stmt")
        assert len(if_nodes) >= 1

    def test_parse_if_else_stmt(self) -> None:
        """if true { } else { } produces an if_stmt node with else branch."""
        ast = parse("fn main() { if true { } else { } }")
        if_nodes = find_nodes(ast, "if_stmt")
        assert len(if_nodes) >= 1

    def test_parse_multiple_stmts(self) -> None:
        """Multiple statements in a single function body."""
        ast = parse(
            "fn main() { let x: u4 = 0; let y: u4 = 1; return x; }"
        )
        let_nodes = find_nodes(ast, "let_stmt")
        assert len(let_nodes) >= 2
        return_nodes = find_nodes(ast, "return_stmt")
        assert len(return_nodes) >= 1


# ---------------------------------------------------------------------------
# Expression and operator tests
# ---------------------------------------------------------------------------


class TestExpressions:
    """Tests for Nib expression forms and operator precedence.

    The Nib expression hierarchy (lowest to highest precedence):

        or_expr  → and_expr  → eq_expr  → cmp_expr
                → add_expr  → bitwise_expr → unary_expr → primary

    Each level is left-associative: ``a +% b +% c = (a +% b) +% c``.

    WHY THIS ORDERING MATTERS:
    Without the correct ordering, ``1 +% 2 == 3 && true`` could be parsed as
    ``1 +% (2 == 3) && true`` (wrong: eq_expr tighter than add_expr) or as
    ``(1 +% 2 == 3 && true)`` (wrong: && tighter than ==). The grammar
    ensures the correct reading: ``((1 +% 2) == 3) && true``.
    """

    def test_parse_wrap_add(self) -> None:
        """let x: u4 = 1 +% 2; — wrapping addition."""
        ast = parse("fn main() { let x: u4 = 1 +% 2; }")
        assert ast.rule_name == "program"

    def test_parse_sat_add(self) -> None:
        """let x: u4 = 1 +? 2; — saturating addition."""
        ast = parse("fn main() { let x: u4 = 1 +? 2; }")
        assert ast.rule_name == "program"

    def test_parse_hex_literal(self) -> None:
        """let x: u4 = 0xF; — hexadecimal literal."""
        ast = parse("fn main() { let x: u4 = 0xF; }")
        assert ast.rule_name == "program"

    def test_parse_bool_literal_true(self) -> None:
        """let b: bool = true; — boolean literal true."""
        ast = parse("fn main() { let b: bool = true; }")
        assert ast.rule_name == "program"

    def test_parse_bool_literal_false(self) -> None:
        """let b: bool = false; — boolean literal false."""
        ast = parse("fn main() { let b: bool = false; }")
        assert ast.rule_name == "program"

    def test_parse_call_expr(self) -> None:
        """let x: u4 = f(); — function call as expression."""
        ast = parse("fn f() -> u4 { return 1; }\nfn main() { let x: u4 = f(); }")
        assert ast.rule_name == "program"

    def test_parse_nested_expr(self) -> None:
        """let x: u4 = (1 +% 2) +% 3; — parenthesized sub-expression."""
        ast = parse("fn main() { let x: u4 = (1 +% 2) +% 3; }")
        assert ast.rule_name == "program"

    def test_parse_comparison(self) -> None:
        """let b: bool = 1 == 1; — equality comparison."""
        ast = parse("fn main() { let b: bool = 1 == 1; }")
        assert ast.rule_name == "program"

    def test_parse_not_equal(self) -> None:
        """let b: bool = 1 != 2; — not-equal comparison."""
        ast = parse("fn main() { let b: bool = 1 != 2; }")
        assert ast.rule_name == "program"

    def test_parse_logical_and(self) -> None:
        """let b: bool = true && false; — logical AND."""
        ast = parse("fn main() { let b: bool = true && false; }")
        assert ast.rule_name == "program"

    def test_parse_logical_or(self) -> None:
        """let b: bool = true || false; — logical OR."""
        ast = parse("fn main() { let b: bool = true || false; }")
        assert ast.rule_name == "program"

    def test_parse_unary_bang(self) -> None:
        """let b: bool = !true; — logical NOT."""
        ast = parse("fn main() { let b: bool = !true; }")
        assert ast.rule_name == "program"

    def test_parse_unary_tilde(self) -> None:
        """let x: u4 = ~0; — bitwise NOT."""
        ast = parse("fn main() { let x: u4 = ~0; }")
        assert ast.rule_name == "program"

    def test_parse_bitwise_and(self) -> None:
        """let x: u4 = 0xF & 0xA; — bitwise AND."""
        ast = parse("fn main() { let x: u4 = 0xF & 0xA; }")
        assert ast.rule_name == "program"

    def test_parse_bitwise_or(self) -> None:
        """let x: u4 = 1 | 2; — bitwise OR."""
        ast = parse("fn main() { let x: u4 = 1 | 2; }")
        assert ast.rule_name == "program"

    def test_parse_bitwise_xor(self) -> None:
        """let x: u4 = 5 ^ 3; — bitwise XOR."""
        ast = parse("fn main() { let x: u4 = 5 ^ 3; }")
        assert ast.rule_name == "program"

    def test_parse_relational_lt(self) -> None:
        """let b: bool = 1 < 2; — less-than comparison."""
        ast = parse("fn main() { let b: bool = 1 < 2; }")
        assert ast.rule_name == "program"

    def test_parse_relational_geq(self) -> None:
        """let b: bool = 5 >= 3; — greater-or-equal comparison."""
        ast = parse("fn main() { let b: bool = 5 >= 3; }")
        assert ast.rule_name == "program"

    def test_parse_subtraction(self) -> None:
        """let x: u4 = 5 - 3; — ordinary subtraction."""
        ast = parse("fn main() { let x: u4 = 5 - 3; }")
        assert ast.rule_name == "program"

    def test_parse_addition(self) -> None:
        """let x: u4 = 1 + 2; — ordinary addition."""
        ast = parse("fn main() { let x: u4 = 1 + 2; }")
        assert ast.rule_name == "program"


# ---------------------------------------------------------------------------
# For loop tests
# ---------------------------------------------------------------------------


class TestForLoop:
    """Tests for Nib for-loop statements.

    Nib's for loop has the form::

        for NAME: type in lower_expr RANGE upper_expr block

    The RANGE token ``..`` is produced atomically by the lexer. The upper
    bound is EXCLUSIVE: ``0..8`` gives i = 0, 1, 2, 3, 4, 5, 6, 7 (8 steps).

    WHY EXCLUSIVE UPPER BOUND:
    Exclusive ranges make the length of the range equal to ``upper - lower``,
    which matches the DJNZ (Decrement and Jump if Not Zero) pattern on the 4004:
    load a register with the count, decrement after each iteration, jump when
    non-zero. The count register is initialized to ``upper - lower``.

    WHY CONST BOUNDS:
    The 4004 has no indirect jump instruction for runtime-computed targets.
    Loop unrolling and DJNZ codegen both require a statically known trip count.
    The compiler enforces this during semantic analysis (both bound expressions
    must reduce to constant values). The parser just verifies syntax.
    """

    def test_parse_range_in_for(self) -> None:
        """for i: u8 in 0..10 { } — basic range loop."""
        ast = parse("fn main() { for i: u8 in 0..10 { } }")
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_parse_for_u4_var(self) -> None:
        """for i: u4 in 0..8 { } — loop variable typed u4."""
        ast = parse("fn main() { for i: u4 in 0..8 { } }")
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_parse_for_with_body(self) -> None:
        """For loop with a non-empty body."""
        ast = parse(
            "static c: u8 = 0;\n"
            "fn main() { for i: u8 in 0..5 { c = c +% 1; } }"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_parse_for_with_const_upper(self) -> None:
        """For loop where upper bound is a const reference."""
        ast = parse(
            "const MAX: u8 = 10;\n"
            "fn main() { for i: u8 in 0..MAX { } }"
        )
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1

    def test_parse_for_with_hex_bounds(self) -> None:
        """For loop with hexadecimal bounds."""
        ast = parse("fn main() { for i: u4 in 0..0xF { } }")
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(for_nodes) >= 1


# ---------------------------------------------------------------------------
# Static in function tests
# ---------------------------------------------------------------------------


class TestStaticAndConst:
    """Tests for static and const declarations interacting with functions."""

    def test_parse_static_in_fn(self) -> None:
        """Static variable used in function body."""
        ast = parse("static c: u8 = 0;\nfn main() { c = 1; }")
        assert ast.rule_name == "program"
        static_nodes = find_nodes(ast, "static_decl")
        assert len(static_nodes) >= 1

    def test_parse_const_in_fn(self) -> None:
        """Const referenced in function body."""
        ast = parse(
            "const MAX: u4 = 9;\n"
            "fn main() { let x: u4 = MAX; }"
        )
        const_nodes = find_nodes(ast, "const_decl")
        assert len(const_nodes) >= 1

    def test_parse_const_hex_value(self) -> None:
        """Const with a hexadecimal initial value."""
        ast = parse("const MASK: u4 = 0xF;")
        const_nodes = find_nodes(ast, "const_decl")
        assert len(const_nodes) >= 1

    def test_parse_static_bool(self) -> None:
        """Static boolean variable."""
        ast = parse("static flag: bool = false;")
        static_nodes = find_nodes(ast, "static_decl")
        assert len(static_nodes) >= 1


# ---------------------------------------------------------------------------
# Error case tests
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for Nib parse errors.

    These tests verify that the parser raises an exception for programs that
    violate the Nib grammar. The parser delegates error detection to
    ``GrammarParser``, which raises ``GrammarParseError`` on syntax errors.
    """

    def test_parse_error_missing_semicolon(self) -> None:
        """A let statement without a trailing semicolon should raise."""
        with pytest.raises(Exception):
            parse("fn main() { let x: u4 = 5 }")

    def test_parse_error_missing_brace(self) -> None:
        """A function body without a closing brace should raise."""
        with pytest.raises(Exception):
            parse("fn main() { let x: u4 = 5;")

    def test_parse_error_fn_without_body(self) -> None:
        """A function declaration without a body should raise."""
        with pytest.raises(Exception):
            parse("fn main()")

    def test_parse_error_bare_statement(self) -> None:
        """A statement outside any function should raise."""
        with pytest.raises(Exception):
            parse("let x: u4 = 5;")


# ---------------------------------------------------------------------------
# Complete program tests
# ---------------------------------------------------------------------------


class TestCompletePrograms:
    """Tests for complete, realistic Nib programs.

    These tests exercise multiple grammar rules together and verify
    that the parser produces a well-formed AST for programs that
    resemble what a real Nib programmer might write.

    Each program is based on the Intel 4004's actual capabilities:
    counter programs, BCD digit manipulation, nibble masking.
    """

    def test_parse_complete_program(self) -> None:
        """A program with const, static, two functions, and a for loop."""
        src = """
        const MAX: u8 = 10;
        static counter: u8 = 0;
        fn inc() { counter = counter +% 1; }
        fn main() { for i: u8 in 0..MAX { inc(); } }
        """
        ast = parse_nib(src)
        assert ast.rule_name == "program"
        const_nodes = find_nodes(ast, "const_decl")
        static_nodes = find_nodes(ast, "static_decl")
        fn_nodes = find_nodes(ast, "fn_decl")
        for_nodes = find_nodes(ast, "for_stmt")
        assert len(const_nodes) >= 1
        assert len(static_nodes) >= 1
        assert len(fn_nodes) >= 2
        assert len(for_nodes) >= 1

    def test_parse_counter_with_if(self) -> None:
        """Counter program using if/else for carry detection."""
        src = """
        static digit: u4 = 0;
        static carry: bool = false;
        fn step() {
            if digit == 9 {
                digit = 0;
                carry = true;
            } else {
                digit = digit +% 1;
                carry = false;
            }
        }
        fn main() {
            for i: u8 in 0..20 { step(); }
        }
        """
        ast = parse_nib(src)
        assert ast.rule_name == "program"
        if_nodes = find_nodes(ast, "if_stmt")
        assert len(if_nodes) >= 1

    def test_parse_bcd_nibble_program(self) -> None:
        """Program using bcd and u4 types with bitwise masks."""
        src = """
        const NIBBLE_MASK: u4 = 0xF;
        static acc: u8 = 0;
        fn get_high(x: u8) -> u4 { return x & NIBBLE_MASK; }
        fn main() {
            let hi: u4 = get_high(acc);
            let lo: u4 = acc & NIBBLE_MASK;
            acc = hi | lo;
        }
        """
        ast = parse_nib(src)
        assert ast.rule_name == "program"

    def test_parse_function_call_chain(self) -> None:
        """Two functions, one calling the other, plus main."""
        src = """
        fn helper() -> u4 { return 7; }
        fn compute() -> u4 { return helper(); }
        fn main() {
            let result: u4 = compute();
        }
        """
        ast = parse_nib(src)
        fn_nodes = find_nodes(ast, "fn_decl")
        assert len(fn_nodes) >= 3

    def test_parse_complex_expression_program(self) -> None:
        """Program with complex mixed expressions and precedence."""
        src = """
        fn check(a: u4, b: u4) -> bool {
            return a +% b == 0xF && !false;
        }
        fn main() {
            let ok: bool = check(7, 8);
        }
        """
        ast = parse_nib(src)
        assert ast.rule_name == "program"
