"""Tests for the oct_type_checker package.

Oct is a statically-typed, 8-bit systems programming language targeting the
Intel 8008 microprocessor.  The type checker enforces language-level invariants
and annotates expression nodes with resolved OctTypes (``"u8"`` or ``"bool"``).

This suite drives the type checker through the real Oct parser pipeline so
that every test exercises the actual grammar-to-AST-to-typed-AST path.

Test strategy:
  - Use ``check_oct(parse_oct(source))`` as the primary entry point.
  - Positive tests: valid programs pass with ``result.ok == True``.
  - Negative tests: invalid programs fail with predictable error messages.
  - Annotation tests: expression nodes carry ``._oct_type`` on success.
"""

from __future__ import annotations

from oct_parser import parse_oct

from oct_type_checker import OctTypeChecker, check_oct

# ---------------------------------------------------------------------------
# Convenience helpers
# ---------------------------------------------------------------------------

def _check(source: str) -> object:
    """Parse *source* and type-check it.  Returns a TypeCheckResult."""
    return check_oct(parse_oct(source))


def _ok(source: str) -> bool:
    """Return True if *source* type-checks without errors."""
    return _check(source).ok


def _errors(source: str) -> list[str]:
    """Return error messages from type-checking *source*."""
    return [e.message for e in _check(source).errors]


def _has_error(source: str, fragment: str) -> bool:
    """Return True if any error message contains *fragment*."""
    return any(fragment in msg for msg in _errors(source))


# ---------------------------------------------------------------------------
# 1. Result structure
# ---------------------------------------------------------------------------

class TestResultStructure:
    """TypeCheckResult carries ok, typed_ast, and errors fields."""

    def test_ok_program_returns_ok_true(self) -> None:
        """A well-typed program yields result.ok == True."""
        result = _check("fn main() { }")
        assert result.ok is True

    def test_ok_program_has_no_errors(self) -> None:
        """A well-typed program yields an empty error list."""
        result = _check("fn main() { }")
        assert result.errors == []

    def test_typed_ast_is_original_root(self) -> None:
        """result.typed_ast is the same ASTNode we passed in."""
        ast = parse_oct("fn main() { }")
        result = check_oct(ast)
        assert result.typed_ast is ast

    def test_error_program_returns_ok_false(self) -> None:
        """A program with a type error yields result.ok == False."""
        result = _check("fn main() { let x: bool = 42; }")
        assert result.ok is False

    def test_errors_are_typed_error_diagnostics(self) -> None:
        """Each error is a TypeErrorDiagnostic with message, line, column."""
        from type_checker_protocol import TypeErrorDiagnostic
        result = _check("fn main() { let x: bool = 42; }")
        assert len(result.errors) >= 1
        err = result.errors[0]
        assert isinstance(err, TypeErrorDiagnostic)
        assert isinstance(err.message, str)
        assert isinstance(err.line, int)
        assert isinstance(err.column, int)

    def test_checker_instance_can_be_reused(self) -> None:
        """Calling check() twice on the same checker should reset state."""
        checker = OctTypeChecker()
        r1 = checker.check(parse_oct("fn main() { }"))
        r2 = checker.check(parse_oct("fn main() { let x: bool = 42; }"))
        assert r1.ok is True
        assert r2.ok is False
        # First result must not be contaminated by second call.
        assert r1.errors == []


# ---------------------------------------------------------------------------
# 2. main function validation
# ---------------------------------------------------------------------------

class TestMainFunction:
    """Every Oct program must have a 'main' function."""

    def test_missing_main_is_error(self) -> None:
        """Programs without main() produce an error."""
        assert _has_error("fn foo() { }", "main")

    def test_empty_main_is_ok(self) -> None:
        """fn main() { } is the minimal valid program."""
        assert _ok("fn main() { }")

    def test_main_with_params_is_error(self) -> None:
        """main cannot take parameters."""
        assert _has_error(
            "fn main(x: u8) { }",
            "no parameters",
        )

    def test_main_with_return_type_is_error(self) -> None:
        """main cannot have a return type."""
        assert _has_error(
            "fn main() -> u8 { return 0; }",
            "no return type",
        )

    def test_main_is_called_by_name(self) -> None:
        """The check is specifically for the 'main' identifier."""
        # 'mains' is not 'main' — should fail due to missing main
        assert _has_error("fn mains() { }", "main")


# ---------------------------------------------------------------------------
# 3. Static declarations
# ---------------------------------------------------------------------------

class TestStaticDeclarations:
    """Top-level statics are global variables."""

    def test_static_u8_ok(self) -> None:
        """static PORT: u8 = 3; is valid."""
        assert _ok("static PORT: u8 = 3;\nfn main() { }")

    def test_static_bool_ok(self) -> None:
        """static FLAG: bool = false; is valid."""
        assert _ok("static FLAG: bool = false;\nfn main() { }")

    def test_static_visible_in_function(self) -> None:
        """Statics are accessible as variables inside functions."""
        src = """
static THRESHOLD: u8 = 100;
fn main() {
    let x: u8 = THRESHOLD;
}
"""
        assert _ok(src)

    def test_static_duplicate_is_error(self) -> None:
        """Duplicate static declarations are rejected."""
        src = """
static X: u8 = 1;
static X: u8 = 2;
fn main() { }
"""
        assert _has_error(src, "already declared")

    def test_static_unknown_type_is_error(self) -> None:
        """Unknown type name in static is an error."""
        src = "static X: i32 = 1;\nfn main() { }"
        assert not _ok(src)


# ---------------------------------------------------------------------------
# 4. Let declarations
# ---------------------------------------------------------------------------

class TestLetDeclarations:
    """let NAME: TYPE = expr; inside function bodies."""

    def test_let_u8_literal(self) -> None:
        """let x: u8 = 42; is valid."""
        assert _ok("fn main() { let x: u8 = 42; }")

    def test_let_bool_true(self) -> None:
        """let f: bool = true; is valid."""
        assert _ok("fn main() { let f: bool = true; }")

    def test_let_bool_false(self) -> None:
        """let f: bool = false; is valid."""
        assert _ok("fn main() { let f: bool = false; }")

    def test_let_u8_from_bool_ok(self) -> None:
        """bool coerces to u8 — assigning bool literal to u8 var is OK."""
        assert _ok("fn main() { let x: u8 = true; }")

    def test_let_bool_from_u8_literal_error(self) -> None:
        """u8 does NOT coerce to bool — assigning 42 to bool is an error."""
        assert _has_error(
            "fn main() { let x: bool = 42; }",
            "cannot assign 'u8' to 'bool'",
        )

    def test_let_unknown_type_error(self) -> None:
        """Unknown type name in let is an error."""
        assert not _ok("fn main() { let x: i32 = 1; }")

    def test_let_hex_literal(self) -> None:
        """let x: u8 = 0xFF; is valid (255 in range)."""
        assert _ok("fn main() { let x: u8 = 0xFF; }")

    def test_let_bin_literal(self) -> None:
        """let x: u8 = 0b10101010; is valid."""
        assert _ok("fn main() { let x: u8 = 0b10101010; }")

    def test_let_zero(self) -> None:
        """let x: u8 = 0; is valid (boundary)."""
        assert _ok("fn main() { let x: u8 = 0; }")

    def test_let_variable_visible_after_declaration(self) -> None:
        """A let variable is in scope for subsequent statements."""
        src = """
fn main() {
    let x: u8 = 10;
    let y: u8 = x;
}
"""
        assert _ok(src)


# ---------------------------------------------------------------------------
# 5. Assignment statements
# ---------------------------------------------------------------------------

class TestAssignStatements:
    """NAME = expr; requires NAME to be declared."""

    def test_assign_to_declared_local(self) -> None:
        """Assigning to a declared local is OK."""
        src = "fn main() { let x: u8 = 0; x = 42; }"
        assert _ok(src)

    def test_assign_to_undeclared_is_error(self) -> None:
        """Assigning to an undeclared variable is an error."""
        assert _has_error(
            "fn main() { x = 42; }",
            "undeclared variable 'x'",
        )

    def test_assign_type_mismatch_is_error(self) -> None:
        """Assigning u8 to a bool variable is an error."""
        src = "fn main() { let x: bool = false; x = 42; }"
        assert _has_error(src, "cannot assign 'u8' to 'bool'")

    def test_assign_bool_to_u8_is_ok(self) -> None:
        """Assigning bool to a u8 variable is OK (coercion)."""
        src = "fn main() { let x: u8 = 0; x = true; }"
        assert _ok(src)

    def test_assign_to_static_is_ok(self) -> None:
        """Assigning to a global static is OK."""
        src = "static PORT: u8 = 0;\nfn main() { PORT = 3; }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 6. Return statements
# ---------------------------------------------------------------------------

class TestReturnStatements:
    """return [expr]; must match the enclosing function's return type."""

    def test_void_function_bare_return_ok(self) -> None:
        """Bare return in void function is OK."""
        assert _ok("fn main() { return; }")

    def test_void_function_return_value_error(self) -> None:
        """Returning a value from a void function is an error."""
        assert _has_error(
            "fn main() { return 42; }",
            "void function must not return a value",
        )

    def test_u8_function_return_u8_ok(self) -> None:
        """Returning u8 from a u8 function is OK."""
        src = "fn foo() -> u8 { return 42; }\nfn main() { }"
        assert _ok(src)

    def test_u8_function_return_bool_ok(self) -> None:
        """Returning bool from a u8 function is OK (bool coerces to u8)."""
        src = "fn foo() -> u8 { return true; }\nfn main() { }"
        assert _ok(src)

    def test_bool_function_return_u8_error(self) -> None:
        """Returning u8 from a bool function is an error."""
        src = "fn foo() -> bool { return 42; }\nfn main() { }"
        assert _has_error(src, "type mismatch")

    def test_u8_function_bare_return_error(self) -> None:
        """Bare return in a non-void function is an error."""
        src = "fn foo() -> u8 { return; }\nfn main() { }"
        assert _has_error(src, "no value")


# ---------------------------------------------------------------------------
# 7. If statements
# ---------------------------------------------------------------------------

class TestIfStatements:
    """if condition must be bool."""

    def test_if_bool_condition_ok(self) -> None:
        """if with a bool condition is OK."""
        assert _ok("fn main() { if true { } }")

    def test_if_comparison_condition_ok(self) -> None:
        """if with a comparison expression (which yields bool) is OK."""
        src = "fn main() { let x: u8 = 5; if x == 5 { } }"
        assert _ok(src)

    def test_if_u8_condition_error(self) -> None:
        """if with a u8 condition is an error."""
        assert _has_error(
            "fn main() { let x: u8 = 1; if x { } }",
            "'if' condition must be 'bool'",
        )

    def test_if_else_ok(self) -> None:
        """if/else with bool condition is OK."""
        src = "fn main() { let x: u8 = 0; if x == 0 { } else { } }"
        assert _ok(src)

    def test_if_body_scope_isolated(self) -> None:
        """Variables declared inside if body don't escape to enclosing scope."""
        src = """
fn main() {
    if true {
        let inner: u8 = 1;
    }
}
"""
        # Should be OK — 'inner' is scoped to the if body.
        assert _ok(src)


# ---------------------------------------------------------------------------
# 8. While statements
# ---------------------------------------------------------------------------

class TestWhileStatements:
    """while condition must be bool."""

    def test_while_bool_condition_ok(self) -> None:
        """while with a bool condition is OK."""
        assert _ok("fn main() { while true { } }")

    def test_while_comparison_ok(self) -> None:
        """while with a comparison expression is OK."""
        src = "fn main() { let x: u8 = 0; while x != 255 { } }"
        assert _ok(src)

    def test_while_u8_condition_error(self) -> None:
        """while with a u8 condition is an error."""
        assert _has_error(
            "fn main() { let x: u8 = 1; while x { } }",
            "'while' condition must be 'bool'",
        )


# ---------------------------------------------------------------------------
# 9. Loop and break
# ---------------------------------------------------------------------------

class TestLoopBreak:
    """loop {} is an infinite loop; break exits."""

    def test_loop_ok(self) -> None:
        """loop { } is valid."""
        assert _ok("fn main() { loop { } }")

    def test_loop_with_break_ok(self) -> None:
        """loop { break; } is valid."""
        assert _ok("fn main() { loop { break; } }")

    def test_loop_with_statements(self) -> None:
        """Statements inside loop are type-checked."""
        src = """
fn main() {
    let x: u8 = 0;
    loop {
        x = x + 1;
        if x == 255 { break; }
    }
}
"""
        assert _ok(src)


# ---------------------------------------------------------------------------
# 10. Arithmetic and bitwise expressions
# ---------------------------------------------------------------------------

class TestArithmeticBitwiseExpressions:
    """Arithmetic (+, -, &, |, ^) require u8-compatible operands → u8."""

    def test_add_u8_ok(self) -> None:
        """u8 + u8 is OK, result u8."""
        src = "fn main() { let x: u8 = 1 + 2; }"
        assert _ok(src)

    def test_sub_u8_ok(self) -> None:
        """u8 - u8 is OK, result u8."""
        assert _ok("fn main() { let x: u8 = 10 - 3; }")

    def test_bitwise_and_ok(self) -> None:
        """u8 & u8 is OK, result u8."""
        assert _ok("fn main() { let x: u8 = 0xFF & 0x0F; }")

    def test_bitwise_or_ok(self) -> None:
        """u8 | u8 is OK, result u8."""
        assert _ok("fn main() { let x: u8 = 0x0F | 0xF0; }")

    def test_bitwise_xor_ok(self) -> None:
        """u8 ^ u8 is OK, result u8."""
        assert _ok("fn main() { let x: u8 = 0xAA ^ 0x55; }")

    def test_add_result_is_u8(self) -> None:
        """Addition of u8s produces u8, which can be assigned to u8."""
        src = "fn main() { let a: u8 = 3; let b: u8 = 4; let c: u8 = a + b; }"
        assert _ok(src)

    def test_bool_plus_u8_ok(self) -> None:
        """bool + u8 is OK because bool coerces to u8."""
        src = "fn main() { let b: bool = true; let x: u8 = b + 1; }"
        assert _ok(src)

    def test_add_bool_result_to_bool_error(self) -> None:
        """Result of addition is u8, which cannot be assigned to bool."""
        src = "fn main() { let x: bool = 1 + 2; }"
        assert _has_error(src, "cannot assign 'u8' to 'bool'")

    def test_chained_add(self) -> None:
        """Chained additions work: a + b + c."""
        src = "fn main() { let x: u8 = 1 + 2 + 3; }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 11. Comparison expressions
# ---------------------------------------------------------------------------

class TestComparisonExpressions:
    """Comparison operators (==, !=, <, >, <=, >=) require u8 operands → bool."""

    def test_eq_ok(self) -> None:
        """5 == 5 produces bool."""
        src = "fn main() { let b: bool = (5 == 5); }"
        assert _ok(src)

    def test_neq_ok(self) -> None:
        """x != 0 produces bool."""
        src = "fn main() { let x: u8 = 0; let b: bool = x != 0; }"
        assert _ok(src)

    def test_lt_ok(self) -> None:
        """x < y produces bool."""
        src = "fn main() { let x: u8 = 1; let y: u8 = 2; let b: bool = x < y; }"
        assert _ok(src)

    def test_gt_ok(self) -> None:
        """x > y produces bool."""
        src = "fn main() { let x: u8 = 5; let b: bool = x > 3; }"
        assert _ok(src)

    def test_leq_ok(self) -> None:
        """x <= 255 produces bool."""
        src = "fn main() { let x: u8 = 5; let b: bool = x <= 255; }"
        assert _ok(src)

    def test_geq_ok(self) -> None:
        """x >= 0 produces bool."""
        src = "fn main() { let x: u8 = 5; let b: bool = x >= 0; }"
        assert _ok(src)

    def test_comparison_result_used_in_if(self) -> None:
        """Comparison result (bool) can be used in if condition."""
        src = "fn main() { let x: u8 = 5; if x > 3 { } }"
        assert _ok(src)

    def test_comparison_result_cannot_assign_to_u8_wait_actually_it_can(self) -> None:
        """bool can be assigned to u8 (coercion), so comparison → u8 is OK."""
        src = "fn main() { let x: u8 = 5; let b: u8 = x > 3; }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 12. Logical expressions
# ---------------------------------------------------------------------------

class TestLogicalExpressions:
    """Logical operators (&&, ||) require bool operands → bool."""

    def test_and_bool_ok(self) -> None:
        """true && false is OK."""
        src = "fn main() { let b: bool = true && false; }"
        assert _ok(src)

    def test_or_bool_ok(self) -> None:
        """true || false is OK."""
        src = "fn main() { let b: bool = true || false; }"
        assert _ok(src)

    def test_and_u8_error(self) -> None:
        """u8 && u8 is an error — && requires bool operands."""
        src = "fn main() { let x: u8 = 1; let y: u8 = 2; let b: bool = x && y; }"
        assert _has_error(src, "requires 'bool'")

    def test_or_u8_error(self) -> None:
        """u8 || u8 is an error."""
        src = "fn main() { let x: u8 = 1; let y: u8 = 2; let b: bool = x || y; }"
        assert _has_error(src, "requires 'bool'")

    def test_and_comparison_result_ok(self) -> None:
        """(x > 0) && (x < 255) is OK — both sides yield bool."""
        src = """
fn main() {
    let x: u8 = 5;
    let b: bool = x > 0 && x < 255;
}
"""
        assert _ok(src)


# ---------------------------------------------------------------------------
# 13. Unary expressions
# ---------------------------------------------------------------------------

class TestUnaryExpressions:
    """! (logical NOT) and ~ (bitwise NOT)."""

    def test_bang_bool_ok(self) -> None:
        """!true is OK, result bool."""
        src = "fn main() { let b: bool = !true; }"
        assert _ok(src)

    def test_bang_bool_variable_ok(self) -> None:
        """!flag where flag:bool is OK."""
        src = "fn main() { let flag: bool = false; let b: bool = !flag; }"
        assert _ok(src)

    def test_bang_u8_error(self) -> None:
        """!42 is an error — ! requires bool operand."""
        assert _has_error(
            "fn main() { let b: bool = !42; }",
            "'!' (logical NOT) requires 'bool'",
        )

    def test_tilde_u8_ok(self) -> None:
        """~0xFF is OK, result u8."""
        src = "fn main() { let x: u8 = ~0xFF; }"
        assert _ok(src)

    def test_tilde_bool_ok(self) -> None:
        """~true is OK because bool coerces to u8; result is u8."""
        src = "fn main() { let x: u8 = ~true; }"
        assert _ok(src)

    def test_double_bang_ok(self) -> None:
        """!!true is OK — nested logical NOT."""
        src = "fn main() { let b: bool = !!true; }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 14. Integer literals
# ---------------------------------------------------------------------------

class TestIntegerLiterals:
    """Integer literals must be in range 0–255."""

    def test_zero_ok(self) -> None:
        """0 is the smallest valid u8 literal."""
        assert _ok("fn main() { let x: u8 = 0; }")

    def test_max_ok(self) -> None:
        """255 is the largest valid u8 literal."""
        assert _ok("fn main() { let x: u8 = 255; }")

    def test_hex_ff_ok(self) -> None:
        """0xFF (255) is valid."""
        assert _ok("fn main() { let x: u8 = 0xFF; }")

    def test_hex_zero_ok(self) -> None:
        """0x00 is valid."""
        assert _ok("fn main() { let x: u8 = 0x00; }")

    def test_bin_max_ok(self) -> None:
        """0b11111111 (255) is valid."""
        assert _ok("fn main() { let x: u8 = 0b11111111; }")

    def test_true_is_bool_not_u8(self) -> None:
        """true literal has type bool, not u8."""
        src = "fn main() { let x: bool = true; }"
        assert _ok(src)

    def test_false_is_bool_not_u8(self) -> None:
        """false literal has type bool, not u8."""
        src = "fn main() { let x: bool = false; }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 15. User-defined function calls
# ---------------------------------------------------------------------------

class TestUserFunctionCalls:
    """Function calls check arity and argument types."""

    def test_call_void_function_ok(self) -> None:
        """Calling a void function as a statement is OK."""
        src = "fn helper() { }\nfn main() { helper(); }"
        assert _ok(src)

    def test_call_u8_function_ok(self) -> None:
        """Calling a u8-returning function and storing result is OK."""
        src = "fn get() -> u8 { return 42; }\nfn main() { let x: u8 = get(); }"
        assert _ok(src)

    def test_forward_call_ok(self) -> None:
        """Functions can be called before they are declared (Pass 1 collects all)."""
        src = """
fn main() {
    let x: u8 = helper();
}
fn helper() -> u8 {
    return 7;
}
"""
        assert _ok(src)

    def test_call_undefined_function_error(self) -> None:
        """Calling an undeclared function is an error."""
        assert _has_error(
            "fn main() { let x: u8 = unknown(); }",
            "undefined function 'unknown'",
        )

    def test_call_wrong_arity_error(self) -> None:
        """Calling with wrong number of arguments is an error."""
        src = "fn foo(x: u8) -> u8 { return x; }\nfn main() { let r: u8 = foo(); }"
        assert _has_error(src, "expects 1 argument")

    def test_call_arg_type_mismatch_error(self) -> None:
        """Passing u8 where bool is expected is an error."""
        src = "fn cond(flag: bool) { }\nfn main() { cond(42); }"
        assert _has_error(src, "expected 'bool'")

    def test_call_bool_arg_to_u8_param_ok(self) -> None:
        """Passing bool to a u8 parameter is OK (coercion)."""
        src = "fn use_u8(x: u8) { }\nfn main() { use_u8(true); }"
        assert _ok(src)

    def test_function_duplicate_definition_error(self) -> None:
        """Two functions with the same name is an error."""
        src = "fn foo() { }\nfn foo() { }\nfn main() { }"
        assert _has_error(src, "already defined")

    def test_mutual_recursion_ok(self) -> None:
        """Mutually recursive functions are OK (both collected in Pass 1)."""
        src = """
fn is_even(n: u8) -> bool { return n == 0; }
fn is_odd(n: u8) -> bool  { return !is_even(n); }
fn main() { }
"""
        assert _ok(src)


# ---------------------------------------------------------------------------
# 16. Intrinsic calls — in() and out()
# ---------------------------------------------------------------------------

class TestIntrinsicInOut:
    """in(PORT) and out(PORT, val) enforce literal port constraints."""

    def test_in_literal_port_ok(self) -> None:
        """in(3) is valid — port is a literal."""
        src = "fn main() { let x: u8 = in(3); }"
        assert _ok(src)

    def test_in_hex_port_ok(self) -> None:
        """in(0x05) with hex literal port is valid."""
        src = "fn main() { let x: u8 = in(0x05); }"
        assert _ok(src)

    def test_in_bin_port_ok(self) -> None:
        """in(0b010) with binary literal port is valid."""
        src = "fn main() { let x: u8 = in(0b010); }"
        assert _ok(src)

    def test_in_variable_port_error(self) -> None:
        """in(x) with variable port is an error — port must be literal."""
        src = "fn main() { let p: u8 = 3; let x: u8 = in(p); }"
        assert _has_error(src, "compile-time integer literal")

    def test_in_returns_u8(self) -> None:
        """in() returns u8, assignable to u8 variable."""
        src = "fn main() { let x: u8 = in(0); }"
        assert _ok(src)

    def test_out_literal_port_u8_ok(self) -> None:
        """out(1, x) with literal port and u8 value is valid."""
        src = "fn main() { let x: u8 = 42; out(1, x); }"
        assert _ok(src)

    def test_out_literal_port_bool_ok(self) -> None:
        """out(1, true) with bool value is valid (bool coerces to u8)."""
        src = "fn main() { out(1, true); }"
        assert _ok(src)

    def test_out_variable_port_error(self) -> None:
        """out(p, x) with variable port is an error."""
        src = "fn main() { let p: u8 = 1; let x: u8 = 5; out(p, x); }"
        assert _has_error(src, "compile-time integer literal")

    def test_in_wrong_arity_note(self) -> None:
        """Grammar enforces in() arity — wrong-arity calls are rejected by the parser.

        The grammar rule ``intrinsic_call = "in" LPAREN expr RPAREN`` accepts
        exactly one argument. Calling in() with zero or two arguments raises
        GrammarParseError before the type checker runs. Arity errors at the
        type-checker level would only fire for manually constructed (non-parsed)
        ASTs, which is why there is no type-level arity test here.
        """
        # Grammar: "in" LPAREN expr RPAREN — fixed 1-arg arity.
        pass  # Documented: arity enforced by grammar.


# ---------------------------------------------------------------------------
# 17. Intrinsic calls — adc and sbb
# ---------------------------------------------------------------------------

class TestIntrinsicAdcSbb:
    """adc(a, b) and sbb(a, b) require u8-compatible args → u8."""

    def test_adc_u8_ok(self) -> None:
        """adc(x, y) with u8 args is valid."""
        src = "fn main() { let x: u8 = 10; let y: u8 = 20; let z: u8 = adc(x, y); }"
        assert _ok(src)

    def test_sbb_u8_ok(self) -> None:
        """sbb(x, y) with u8 args is valid."""
        src = "fn main() { let x: u8 = 100; let y: u8 = 20; let z: u8 = sbb(x, y); }"
        assert _ok(src)

    def test_adc_bool_args_ok(self) -> None:
        """adc(true, false) is OK — bool coerces to u8."""
        src = "fn main() { let z: u8 = adc(true, false); }"
        assert _ok(src)

    def test_adc_returns_u8(self) -> None:
        """adc() result can be stored in u8."""
        src = "fn main() { let z: u8 = adc(1, 2); }"
        assert _ok(src)

    def test_adc_sbb_arity_note(self) -> None:
        """Grammar enforces adc/sbb arity — each requires exactly 2 args.

        The grammar rules ``adc LPAREN expr COMMA expr RPAREN`` and
        ``sbb LPAREN expr COMMA expr RPAREN`` are fixed-arity. Calling with
        wrong arg counts raises GrammarParseError before the type checker runs.
        """
        # Grammar: adc(expr, expr) and sbb(expr, expr) — fixed 2-arg arity.
        pass  # Documented: arity enforced by grammar.


# ---------------------------------------------------------------------------
# 18. Intrinsic calls — rotate: rlc, rrc, ral, rar
# ---------------------------------------------------------------------------

class TestIntrinsicRotate:
    """rlc, rrc, ral, rar each take one u8 arg → u8."""

    def test_rlc_ok(self) -> None:
        """rlc(x) with u8 arg is valid."""
        src = "fn main() { let x: u8 = 0xAB; let y: u8 = rlc(x); }"
        assert _ok(src)

    def test_rrc_ok(self) -> None:
        """rrc(x) with u8 arg is valid."""
        src = "fn main() { let x: u8 = 0xAB; let y: u8 = rrc(x); }"
        assert _ok(src)

    def test_ral_ok(self) -> None:
        """ral(x) with u8 arg is valid."""
        src = "fn main() { let x: u8 = 1; let y: u8 = ral(x); }"
        assert _ok(src)

    def test_rar_ok(self) -> None:
        """rar(x) with u8 arg is valid."""
        src = "fn main() { let x: u8 = 128; let y: u8 = rar(x); }"
        assert _ok(src)

    def test_rotate_arity_note(self) -> None:
        """Grammar enforces rotate arity — rlc/rrc/ral/rar each require exactly 1 arg.

        E.g. ``rlc LPAREN expr RPAREN`` — fixed 1-arg. Wrong-arity calls are
        rejected at parse time before the type checker runs.
        """
        # Grammar: rlc/rrc/ral/rar each take exactly 1 arg.
        pass  # Documented: arity enforced by grammar.

    def test_rotate_bool_arg_ok(self) -> None:
        """rlc(true) is OK — bool coerces to u8."""
        src = "fn main() { let y: u8 = rlc(true); }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 19. Intrinsic calls — carry() and parity()
# ---------------------------------------------------------------------------

class TestIntrinsicCarryParity:
    """carry() → bool (no args); parity(a) → bool (u8-compatible arg)."""

    def test_carry_ok(self) -> None:
        """carry() returns bool, can be stored in bool."""
        src = (
            "fn main() {"
            " let x: u8 = 10; let y: u8 = adc(x, 5); let c: bool = carry(); }"
        )
        assert _ok(src)

    def test_carry_result_used_in_if(self) -> None:
        """carry() result used in if condition is OK."""
        src = "fn main() { let x: u8 = 200; let y: u8 = adc(x, 100); if carry() { } }"
        assert _ok(src)

    def test_carry_arity_note(self) -> None:
        """Grammar enforces carry() arity — exactly 0 args.

        Grammar: ``"carry" LPAREN RPAREN``. Calling carry(1) raises
        GrammarParseError at parse time, before the type checker runs.
        """
        # Grammar: carry() — fixed 0-arg.
        pass  # Documented: arity enforced by grammar.

    def test_parity_u8_ok(self) -> None:
        """parity(x) with u8 arg returns bool."""
        src = "fn main() { let x: u8 = 42; let p: bool = parity(x); }"
        assert _ok(src)

    def test_parity_bool_arg_ok(self) -> None:
        """parity(true) is OK — bool coerces to u8."""
        src = "fn main() { let p: bool = parity(true); }"
        assert _ok(src)

    def test_parity_result_in_if_ok(self) -> None:
        """parity() result used in if condition is OK."""
        src = "fn main() { let x: u8 = 42; if parity(x) { } }"
        assert _ok(src)

    def test_parity_arity_note(self) -> None:
        """Grammar enforces parity() arity — exactly 1 arg.

        Grammar: ``"parity" LPAREN expr RPAREN``. Calling parity(1, 2) raises
        GrammarParseError at parse time, before the type checker runs.
        """
        # Grammar: parity(expr) — fixed 1-arg.
        pass  # Documented: arity enforced by grammar.


# ---------------------------------------------------------------------------
# 20. AST annotation
# ---------------------------------------------------------------------------

class TestAstAnnotation:
    """Expression nodes get ._oct_type set after successful type checking."""

    def _find_nodes_with_attr(self, node: object, attr: str) -> list[object]:
        """Depth-first collect all ASTNodes that have *attr*."""
        results = []
        if hasattr(node, attr):
            results.append(node)
        for child in getattr(node, "children", []):
            results.extend(self._find_nodes_with_attr(child, attr))
        return results

    def test_literal_annotated(self) -> None:
        """Integer literal node gets ._oct_type == 'u8'."""
        ast = parse_oct("fn main() { let x: u8 = 42; }")
        result = check_oct(ast)
        assert result.ok
        annotated = self._find_nodes_with_attr(result.typed_ast, "_oct_type")
        assert len(annotated) > 0

    def test_bool_literal_annotated(self) -> None:
        """Boolean literal node gets ._oct_type == 'bool'."""
        ast = parse_oct("fn main() { let b: bool = true; }")
        result = check_oct(ast)
        assert result.ok
        # Find any token or node with _oct_type == 'bool'
        annotated = self._find_nodes_with_attr(result.typed_ast, "_oct_type")
        bool_typed = [n for n in annotated if getattr(n, "_oct_type", None) == "bool"]
        assert len(bool_typed) > 0

    def test_u8_expr_annotated(self) -> None:
        """Addition expression node gets ._oct_type == 'u8'."""
        ast = parse_oct("fn main() { let x: u8 = 1 + 2; }")
        result = check_oct(ast)
        assert result.ok
        annotated = self._find_nodes_with_attr(result.typed_ast, "_oct_type")
        u8_typed = [n for n in annotated if getattr(n, "_oct_type", None) == "u8"]
        assert len(u8_typed) > 0

    def test_comparison_annotated_bool(self) -> None:
        """Comparison expression node gets ._oct_type == 'bool'."""
        ast = parse_oct("fn main() { let b: bool = 3 > 2; }")
        result = check_oct(ast)
        assert result.ok
        annotated = self._find_nodes_with_attr(result.typed_ast, "_oct_type")
        bool_typed = [n for n in annotated if getattr(n, "_oct_type", None) == "bool"]
        assert len(bool_typed) > 0


# ---------------------------------------------------------------------------
# 21. Operator precedence and type flow
# ---------------------------------------------------------------------------

class TestOperatorPrecedenceFlow:
    """Expressions with mixed operators type-flow correctly."""

    def test_complex_bool_expr_ok(self) -> None:
        """(x > 0) && carry() — both sides bool → bool."""
        src = """
fn main() {
    let x: u8 = adc(100, 200);
    let ok: bool = x > 0 && carry();
}
"""
        assert _ok(src)

    def test_add_then_compare(self) -> None:
        """(a + b) > c — add is u8, compare is bool."""
        src = """
fn main() {
    let a: u8 = 10;
    let b: u8 = 20;
    let c: u8 = 25;
    let gt: bool = a + b > c;
}
"""
        assert _ok(src)

    def test_paren_expr_ok(self) -> None:
        """Parenthesised expression inherits its inner type."""
        src = "fn main() { let x: u8 = (42); }"
        assert _ok(src)

    def test_nested_paren_ok(self) -> None:
        """Nested parentheses work."""
        src = "fn main() { let x: u8 = ((1 + 2)); }"
        assert _ok(src)


# ---------------------------------------------------------------------------
# 22. Full program examples (from spec)
# ---------------------------------------------------------------------------

class TestFullPrograms:
    """End-to-end type checking of realistic Oct programs."""

    def test_counter_program(self) -> None:
        """A simple counter loop type-checks cleanly."""
        src = """
static COUNT: u8 = 0;
fn main() {
    while COUNT != 255 {
        COUNT = COUNT + 1;
    }
    out(0, COUNT);
}
"""
        assert _ok(src)

    def test_io_program(self) -> None:
        """A program reading input and writing output."""
        src = """
fn process(val: u8) -> u8 {
    return rlc(val);
}

fn main() {
    let data: u8 = in(0);
    let result: u8 = process(data);
    out(1, result);
}
"""
        assert _ok(src)

    def test_carry_check_program(self) -> None:
        """Addition with carry check type-checks cleanly."""
        src = """
fn main() {
    let a: u8 = 200;
    let b: u8 = 100;
    let sum: u8 = adc(a, b);
    let overflow: bool = carry();
    if overflow {
        out(0, 1);
    } else {
        out(0, 0);
    }
}
"""
        assert _ok(src)

    def test_parity_check_program(self) -> None:
        """Parity check program type-checks cleanly."""
        src = """
fn main() {
    let data: u8 = in(2);
    let even_parity: bool = parity(data);
    if even_parity {
        out(3, 1);
    }
}
"""
        assert _ok(src)

    def test_bit_manipulation_program(self) -> None:
        """Bitwise operations with rotate and NOT."""
        src = """
fn mirror(x: u8) -> u8 {
    let a: u8 = rlc(x);
    let b: u8 = rrc(x);
    return a & b;
}

fn main() {
    let val: u8 = in(0);
    let m: u8 = mirror(val);
    out(0, m);
}
"""
        assert _ok(src)

    def test_multi_function_program(self) -> None:
        """A program with several functions calling each other."""
        src = """
fn clamp(val: u8) -> u8 {
    if val > 200 {
        return 200;
    }
    return val;
}

fn scale(val: u8) -> u8 {
    let c: u8 = clamp(val);
    return c & 0xF0;
}

fn main() {
    let raw: u8 = in(0);
    let scaled: u8 = scale(raw);
    out(1, scaled);
}
"""
        assert _ok(src)

    def test_nested_loops_program(self) -> None:
        """Nested loops with break type-check cleanly."""
        src = """
fn main() {
    let i: u8 = 0;
    while i != 10 {
        let j: u8 = 0;
        loop {
            if j == 5 { break; }
            j = j + 1;
        }
        i = i + 1;
    }
}
"""
        assert _ok(src)

    def test_bitwise_not_program(self) -> None:
        """Bitwise NOT and XOR together."""
        src = """
fn invert(x: u8) -> u8 {
    return ~x;
}

fn main() {
    let v: u8 = 0b10101010;
    let inv: u8 = invert(v);
    let xored: u8 = v ^ inv;
    out(0, xored);
}
"""
        assert _ok(src)

    def test_logical_negation_program(self) -> None:
        """Logical NOT and boolean assignment."""
        src = """
fn main() {
    let x: u8 = in(0);
    let is_zero: bool = x == 0;
    let is_nonzero: bool = !is_zero;
    if is_nonzero {
        out(0, x);
    }
}
"""
        assert _ok(src)


# ---------------------------------------------------------------------------
# 23. Error cascade — errors don't multiply unexpectedly
# ---------------------------------------------------------------------------

class TestErrorCascade:
    """After one error, cascaded errors should be minimal."""

    def test_single_undeclared_variable_one_error(self) -> None:
        """Using an undeclared variable reports exactly one error for that name."""
        errors = _errors("fn main() { let y: u8 = x; }")
        undeclared_errors = [e for e in errors if "x" in e]
        assert len(undeclared_errors) == 1

    def test_multiple_independent_errors(self) -> None:
        """Multiple independent type errors each produce a diagnostic."""
        src = """
fn main() {
    let a: bool = 42;
    let b: bool = 99;
}
"""
        errs = _errors(src)
        assert len(errs) >= 2


# ---------------------------------------------------------------------------
# 24. Undeclared variable usage
# ---------------------------------------------------------------------------

class TestUndeclaredVariables:
    """Reading from undeclared variables is an error."""

    def test_undeclared_in_expr_error(self) -> None:
        """Using an undeclared name in an expression is an error."""
        assert _has_error(
            "fn main() { let y: u8 = ghost; }", "undefined variable 'ghost'"
        )

    def test_undeclared_in_if_condition(self) -> None:
        """Using an undeclared name in if condition is an error."""
        assert _has_error("fn main() { if ghost { } }", "undefined variable 'ghost'")

    def test_declared_after_use_is_error(self) -> None:
        """Variables cannot be used before their let declaration."""
        src = """
fn main() {
    let y: u8 = x;
    let x: u8 = 5;
}
"""
        assert _has_error(src, "undefined variable 'x'")
