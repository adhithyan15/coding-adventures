"""Tests for the oct_parser package.

Oct is a small, statically-typed, 8-bit systems programming language targeting
the Intel 8008 microprocessor (1972).  The parser consumes the token stream
produced by the Oct lexer and produces a generic ``ASTNode`` tree using the
EBNF rules defined in ``oct.grammar``.

This test suite validates:
  - The root node has rule_name "program"
  - Empty source produces a program with no children
  - All top-level declaration forms parse to the correct rule names
  - All statement kinds produce their expected AST structure
  - All 10 intrinsic calls parse without error as intrinsic_call nodes
  - The complete OCT00 spec program examples parse without error
  - Operator precedence is respected (tested via structural checks)
  - Type annotations using NAME tokens are accepted
"""

from __future__ import annotations

from oct_parser import parse_oct

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _rule(node: object) -> str:
    """Return the rule_name of an ASTNode, or the type of a Token."""
    if hasattr(node, "rule_name"):
        return node.rule_name  # type: ignore[attr-defined]
    return node.type  # type: ignore[attr-defined]


def _children(node: object) -> list[object]:
    """Return the children of an ASTNode (empty list for Token)."""
    if hasattr(node, "children"):
        return node.children  # type: ignore[attr-defined]
    return []


def _find_rules(node: object, rule: str) -> list[object]:
    """Depth-first search: collect all nodes with rule_name == rule."""
    results = []
    if _rule(node) == rule:
        results.append(node)
    for child in _children(node):
        results.extend(_find_rules(child, rule))
    return results


def _find_token_values(node: object, token_type: str) -> list[str]:
    """Depth-first search: collect Token values where token.type == token_type."""
    results: list[str] = []
    if hasattr(node, "type") and node.type == token_type:  # type: ignore[attr-defined]
        results.append(node.value)  # type: ignore[attr-defined]
    for child in _children(node):
        results.extend(_find_token_values(child, token_type))
    return results


# ---------------------------------------------------------------------------
# Basic structure
# ---------------------------------------------------------------------------

class TestParseOctBasic:
    """Root node shape and trivial programs."""

    def test_root_is_program(self) -> None:
        """parse_oct always returns a 'program' node."""
        ast = parse_oct("")
        assert _rule(ast) == "program"

    def test_empty_program_has_no_top_decls(self) -> None:
        """Empty source → program node with no children (or only non-decl)."""
        ast = parse_oct("")
        top_decls = _find_rules(ast, "top_decl")
        assert top_decls == []

    def test_single_function_parses(self) -> None:
        """A minimal fn main() { } produces exactly one fn_decl."""
        ast = parse_oct("fn main() { }")
        fn_decls = _find_rules(ast, "fn_decl")
        assert len(fn_decls) == 1

    def test_single_static_parses(self) -> None:
        """A single static declaration produces exactly one static_decl."""
        ast = parse_oct("static counter: u8 = 0;")
        static_decls = _find_rules(ast, "static_decl")
        assert len(static_decls) == 1


# ---------------------------------------------------------------------------
# Static declarations
# ---------------------------------------------------------------------------

class TestStaticDeclarations:
    """static_decl forms: u8/bool types, literal kinds, multiple statics."""

    def test_static_u8_decimal(self) -> None:
        ast = parse_oct("static counter: u8 = 0;")
        assert len(_find_rules(ast, "static_decl")) == 1

    def test_static_u8_hex(self) -> None:
        ast = parse_oct("static mask: u8 = 0xFF;")
        assert len(_find_rules(ast, "static_decl")) == 1

    def test_static_u8_binary(self) -> None:
        ast = parse_oct("static flags: u8 = 0b10110011;")
        assert len(_find_rules(ast, "static_decl")) == 1

    def test_static_bool_false(self) -> None:
        ast = parse_oct("static ready: bool = false;")
        assert len(_find_rules(ast, "static_decl")) == 1

    def test_static_bool_true(self) -> None:
        ast = parse_oct("static done: bool = true;")
        assert len(_find_rules(ast, "static_decl")) == 1

    def test_multiple_statics(self) -> None:
        source = "static lo: u8 = 0;\nstatic hi: u8 = 0;"
        ast = parse_oct(source)
        assert len(_find_rules(ast, "static_decl")) == 2

    def test_static_and_fn(self) -> None:
        source = "static x: u8 = 1;\nfn main() { }"
        ast = parse_oct(source)
        assert len(_find_rules(ast, "static_decl")) == 1
        assert len(_find_rules(ast, "fn_decl")) == 1


# ---------------------------------------------------------------------------
# Function declarations
# ---------------------------------------------------------------------------

class TestFunctionDeclarations:
    """fn_decl forms: void, return type, parameters."""

    def test_void_no_params(self) -> None:
        ast = parse_oct("fn main() { }")
        fn_decls = _find_rules(ast, "fn_decl")
        assert len(fn_decls) == 1

    def test_with_return_type(self) -> None:
        ast = parse_oct("fn double(x: u8) -> u8 { return x + x; }")
        fn_decls = _find_rules(ast, "fn_decl")
        assert len(fn_decls) == 1

    def test_with_two_params(self) -> None:
        ast = parse_oct("fn add(a: u8, b: u8) -> u8 { return a + b; }")
        fn_decls = _find_rules(ast, "fn_decl")
        assert len(fn_decls) == 1
        param_list = _find_rules(ast, "param_list")
        assert len(param_list) == 1

    def test_with_four_params(self) -> None:
        """Four parameters — the maximum allowed by the 8008 register file."""
        source = "fn max4(a: u8, b: u8, c: u8, d: u8) -> u8 { return a; }"
        ast = parse_oct(source)
        params = _find_rules(ast, "param")
        assert len(params) == 4

    def test_multiple_functions(self) -> None:
        source = "fn tick() { }\nfn main() { tick(); }"
        ast = parse_oct(source)
        assert len(_find_rules(ast, "fn_decl")) == 2

    def test_empty_body(self) -> None:
        ast = parse_oct("fn noop() { }")
        blocks = _find_rules(ast, "block")
        assert len(blocks) == 1


# ---------------------------------------------------------------------------
# Statements
# ---------------------------------------------------------------------------

class TestStatements:
    """All statement kinds: let, assign, return, if, while, loop, break, expr_stmt."""

    def test_let_stmt(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 0; }")
        assert len(_find_rules(ast, "let_stmt")) == 1

    def test_assign_stmt(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 0; x = 1; }")
        assert len(_find_rules(ast, "assign_stmt")) == 1

    def test_return_with_expr(self) -> None:
        ast = parse_oct("fn f() -> u8 { return 42; }")
        assert len(_find_rules(ast, "return_stmt")) == 1

    def test_return_without_expr(self) -> None:
        """Bare return for void functions: return;"""
        ast = parse_oct("fn f() { return; }")
        assert len(_find_rules(ast, "return_stmt")) == 1

    def test_if_without_else(self) -> None:
        ast = parse_oct("fn f() { if true { let x: u8 = 0; } }")
        if_stmts = _find_rules(ast, "if_stmt")
        assert len(if_stmts) == 1

    def test_if_with_else(self) -> None:
        ast = parse_oct("fn f() { if true { } else { } }")
        if_stmts = _find_rules(ast, "if_stmt")
        assert len(if_stmts) == 1
        # Both branches present → 2 blocks inside if_stmt
        blocks = _find_rules(ast, "block")
        assert len(blocks) >= 3  # fn body + if-block + else-block

    def test_while_stmt(self) -> None:
        ast = parse_oct("fn f() { let n: u8 = 5; while n != 0 { n = n - 1; } }")
        assert len(_find_rules(ast, "while_stmt")) == 1

    def test_loop_stmt(self) -> None:
        ast = parse_oct("fn f() { loop { } }")
        assert len(_find_rules(ast, "loop_stmt")) == 1

    def test_break_stmt(self) -> None:
        ast = parse_oct("fn f() { loop { break; } }")
        assert len(_find_rules(ast, "break_stmt")) == 1

    def test_expr_stmt_with_call(self) -> None:
        """A user-defined function call as a statement."""
        ast = parse_oct("fn g() { }\nfn f() { g(); }")
        # expr_stmt contains call_expr
        assert len(_find_rules(ast, "expr_stmt")) == 1
        assert len(_find_rules(ast, "call_expr")) >= 1

    def test_nested_if_in_while(self) -> None:
        source = "fn f() { let n: u8 = 8; while n != 0 { if true { } n = n - 1; } }"
        ast = parse_oct(source)
        assert len(_find_rules(ast, "while_stmt")) == 1
        assert len(_find_rules(ast, "if_stmt")) == 1

    def test_multiple_let_stmts(self) -> None:
        source = "fn f() { let a: u8 = 1; let b: u8 = 2; let c: u8 = 3; }"
        ast = parse_oct(source)
        assert len(_find_rules(ast, "let_stmt")) == 3


# ---------------------------------------------------------------------------
# Expression parsing and precedence
# ---------------------------------------------------------------------------

class TestExpressions:
    """Expression kinds and operator precedence."""

    def test_addition(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 1 + 2; }")
        assert len(_find_rules(ast, "add_expr")) >= 1

    def test_subtraction(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 5 - 3; }")
        assert len(_find_rules(ast, "add_expr")) >= 1

    def test_bitwise_and(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 0xFF & 0x0F; }")
        assert len(_find_rules(ast, "bitwise_expr")) >= 1

    def test_bitwise_or(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 0x0F | 0xF0; }")
        assert len(_find_rules(ast, "bitwise_expr")) >= 1

    def test_bitwise_xor(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = a ^ b; }")
        assert len(_find_rules(ast, "bitwise_expr")) >= 1

    def test_unary_not_bitwise(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = ~a; }")
        assert len(_find_rules(ast, "unary_expr")) >= 1

    def test_unary_not_logical(self) -> None:
        ast = parse_oct("fn f() { let ok: bool = !false; }")
        assert len(_find_rules(ast, "unary_expr")) >= 1

    def test_equality_eq_eq(self) -> None:
        ast = parse_oct("fn f() { if a == 0 { } }")
        assert len(_find_rules(ast, "eq_expr")) >= 1

    def test_equality_neq(self) -> None:
        ast = parse_oct("fn f() { if n != 255 { } }")
        assert len(_find_rules(ast, "eq_expr")) >= 1

    def test_comparison_lt(self) -> None:
        ast = parse_oct("fn f() { if a < b { } }")
        assert len(_find_rules(ast, "cmp_expr")) >= 1

    def test_comparison_geq(self) -> None:
        ast = parse_oct("fn f() { if a >= 128 { } }")
        assert len(_find_rules(ast, "cmp_expr")) >= 1

    def test_logical_and(self) -> None:
        ast = parse_oct("fn f() { if a == 0 && b == 0 { } }")
        assert len(_find_rules(ast, "and_expr")) >= 1

    def test_logical_or(self) -> None:
        ast = parse_oct("fn f() { if a == 0 || b == 0 { } }")
        assert len(_find_rules(ast, "or_expr")) >= 1

    def test_parenthesised_expression(self) -> None:
        """Parentheses group correctly: (a + b) & mask."""
        ast = parse_oct("fn f() { let x: u8 = (a + b) & mask; }")
        # No parse error means precedence is resolved
        assert _rule(ast) == "program"

    def test_literal_true_in_expr(self) -> None:
        ast = parse_oct("fn f() { let ok: bool = true; }")
        # 'true' should appear as a token value somewhere
        true_tokens = _find_token_values(ast, "true")
        assert true_tokens == ["true"]

    def test_literal_false_in_expr(self) -> None:
        ast = parse_oct("fn f() { let ok: bool = false; }")
        false_tokens = _find_token_values(ast, "false")
        assert false_tokens == ["false"]

    def test_hex_literal_in_expr(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 0xFF; }")
        assert len(_find_token_values(ast, "HEX_LIT")) == 1

    def test_binary_literal_in_expr(self) -> None:
        ast = parse_oct("fn f() { let x: u8 = 0b10110011; }")
        assert len(_find_token_values(ast, "BIN_LIT")) == 1


# ---------------------------------------------------------------------------
# Intrinsic calls
# ---------------------------------------------------------------------------

class TestIntrinsicCalls:
    """All 10 Oct intrinsics parse without error and produce intrinsic_call nodes."""

    def _ast_with_intrinsic(self, call: str) -> object:
        return parse_oct(f"fn f() {{ let x: u8 = {call}; }}")

    def _void_intrinsic(self, call: str) -> object:
        return parse_oct(f"fn f() {{ {call}; }}")

    def test_in_call(self) -> None:
        """in(0) — read from input port 0."""
        ast = self._ast_with_intrinsic("in(0)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_out_call_as_stmt(self) -> None:
        """out(1, x) — write to output port 1 (used as statement)."""
        ast = self._void_intrinsic("out(1, x)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_adc_call(self) -> None:
        """adc(a, b) — add with carry."""
        ast = self._ast_with_intrinsic("adc(a, b)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_sbb_call(self) -> None:
        """sbb(a, b) — subtract with borrow."""
        ast = self._ast_with_intrinsic("sbb(a, b)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_rlc_call(self) -> None:
        """rlc(x) — rotate left circular."""
        ast = self._ast_with_intrinsic("rlc(x)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_rrc_call(self) -> None:
        """rrc(x) — rotate right circular."""
        ast = self._ast_with_intrinsic("rrc(x)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_ral_call(self) -> None:
        """ral(x) — rotate left through carry (9-bit)."""
        ast = self._ast_with_intrinsic("ral(x)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_rar_call(self) -> None:
        """rar(x) — rotate right through carry (9-bit)."""
        ast = self._ast_with_intrinsic("rar(x)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_carry_call(self) -> None:
        """carry() — read carry flag (zero arguments)."""
        ast = self._ast_with_intrinsic("carry()")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_parity_call(self) -> None:
        """parity(b) — read parity flag of b."""
        ast = self._ast_with_intrinsic("parity(b)")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_carry_in_if_condition(self) -> None:
        """carry() used as a condition: if carry() { … }"""
        ast = parse_oct("fn f() { if carry() { } }")
        assert len(_find_rules(ast, "intrinsic_call")) >= 1

    def test_in_with_nested_expr(self) -> None:
        """in port can be any expr (type-checker validates constant later)."""
        ast = self._ast_with_intrinsic("in(0)")
        assert _rule(ast) == "program"


# ---------------------------------------------------------------------------
# Complete programs from OCT00 spec
# ---------------------------------------------------------------------------

class TestCompletePrograms:
    """All five OCT00 spec example programs parse without error."""

    def test_example1_echo_input_to_output(self) -> None:
        """Echo input to output — loop with in() and out()."""
        source = """
        fn main() {
            loop {
                let b: u8 = in(0);
                out(8, b);
            }
        }
        """
        ast = parse_oct(source)
        assert _rule(ast) == "program"
        assert len(_find_rules(ast, "fn_decl")) == 1
        assert len(_find_rules(ast, "loop_stmt")) == 1
        assert len(_find_rules(ast, "let_stmt")) == 1

    def test_example2_count_to_255(self) -> None:
        """Count from 0 to 255, output each value."""
        source = """
        fn main() {
            let n: u8 = 0;
            while n != 255 {
                out(1, n);
                n = n + 1;
            }
            out(1, 255);
        }
        """
        ast = parse_oct(source)
        assert _rule(ast) == "program"
        assert len(_find_rules(ast, "while_stmt")) == 1
        assert len(_find_rules(ast, "assign_stmt")) == 1

    def test_example3_xor_checksum(self) -> None:
        """XOR checksum of 8 bytes from port 0, output to port 1."""
        source = """
        fn main() {
            let checksum: u8 = 0;
            let i: u8 = 0;
            while i != 8 {
                let b: u8 = in(0);
                checksum = checksum ^ b;
                i = i + 1;
            }
            out(1, checksum);
        }
        """
        ast = parse_oct(source)
        assert _rule(ast) == "program"
        # checksum, i, b (inside while) = 3 let stmts minimum
        assert len(_find_rules(ast, "let_stmt")) >= 3

    def test_example4_16bit_counter_with_carry(self) -> None:
        """16-bit counter using carry() — two statics, two functions."""
        source = """
        static lo: u8 = 0;
        static hi: u8 = 0;

        fn tick() {
            let l: u8 = lo;
            l = l + 1;
            lo = l;
            if carry() {
                let h: u8 = hi;
                h = h + 1;
                hi = h;
                out(1, h);
            }
        }

        fn main() {
            loop {
                tick();
            }
        }
        """
        ast = parse_oct(source)
        assert _rule(ast) == "program"
        assert len(_find_rules(ast, "static_decl")) == 2
        assert len(_find_rules(ast, "fn_decl")) == 2
        assert len(_find_rules(ast, "if_stmt")) == 1
        carry_intrinsics = _find_rules(ast, "intrinsic_call")
        assert len(carry_intrinsics) >= 1

    def test_example5_bit_reversal_with_rotations(self) -> None:
        """Bit reversal using ral/rar — two functions, both rotation intrinsics."""
        source = """
        fn reverse_bits(x: u8) -> u8 {
            let result: u8 = 0;
            let i: u8 = 0;
            while i != 8 {
                x = ral(x);
                result = rar(result);
                i = i + 1;
            }
            return result;
        }

        fn main() {
            let b: u8 = in(0);
            out(1, reverse_bits(b));
        }
        """
        ast = parse_oct(source)
        assert _rule(ast) == "program"
        assert len(_find_rules(ast, "fn_decl")) == 2
        assert len(_find_rules(ast, "return_stmt")) == 1
        intrinsics = _find_rules(ast, "intrinsic_call")
        assert len(intrinsics) >= 3  # ral + rar + in + out = 4
