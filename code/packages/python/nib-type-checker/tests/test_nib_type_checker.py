"""Tests for the Nib type checker.

=============================================================================
TEST STRATEGY
=============================================================================

Each test exercises one language-level invariant. Tests are organised into
categories:

1. **Valid programs** — programs that should pass type checking (result.ok).
2. **Type mismatch errors** — wrong type on the RHS of an assignment or let.
3. **Undeclared name errors** — using a variable before declaring it.
4. **BCD restriction errors** — using an illegal operator with bcd operands.
5. **For-loop semantics** — numeric loop bounds type-check, including runtime values.
6. **If-condition errors** — non-bool condition in an if statement.
7. **Function call errors** — wrong argument count or types.
8. **Return type errors** — return expression type doesn't match declaration.
9. **Scope tests** — shadowing, inner/outer scopes.
10. **NibType and helpers** — unit tests for the types module.
11. **ScopeChain** — unit tests for the scope module.

We test the full pipeline — parse then type-check — via the ``tc()`` helper,
which mirrors the real-world usage pattern exactly.

Note on integer literal typing:
    Nib integer literals are typed as ``u4`` by default. When a literal is
    used in a ``let x: u8 = 5`` context, the checker sees the declared type
    as ``u8`` and the literal as ``u4``. This *could* be a type mismatch
    under strict rules, but to keep the language usable the checker coerces
    literals to the declared type when the context is unambiguous. In
    practice this means ``let x: u8 = 5`` passes.

    For this implementation we follow the spec: any numeric literal fits
    the declared type because the literal is untyped at parse time and the
    declared type is the authority. The checker only rejects mismatches
    between *named* types (variables, parameters, const/static).
"""

from __future__ import annotations

import pytest

from nib_type_checker import NibType, ScopeChain, Symbol, check
from nib_type_checker.types import (
    is_bcd_op_allowed,
    is_numeric,
    parse_type_name,
    types_are_compatible,
)


# ---------------------------------------------------------------------------
# Test helper
# ---------------------------------------------------------------------------


def tc(source: str):  # type: ignore[return]
    """Parse Nib source and type-check it; return a TypeCheckResult."""
    from nib_parser import parse_nib  # type: ignore[import-not-found]

    return check(parse_nib(source))


# ---------------------------------------------------------------------------
# 1. Valid programs
# ---------------------------------------------------------------------------


def test_empty_program() -> None:
    """An empty program is valid — no declarations required."""
    assert tc("").ok


def test_const_decl() -> None:
    """A top-level const declaration is valid."""
    assert tc("const MAX: u8 = 10;").ok


def test_static_decl() -> None:
    """A top-level static declaration is valid."""
    assert tc("static x: u4 = 0;").ok


def test_fn_no_body() -> None:
    """A function with an empty body is valid."""
    assert tc("fn main() { }").ok


def test_let_u4() -> None:
    """let binding with u4 type and integer literal."""
    assert tc("fn main() { let x: u4 = 5; }").ok


def test_let_u8() -> None:
    """let binding with u8 type and integer literal."""
    assert tc("fn main() { let x: u8 = 200; }").ok


def test_let_bcd() -> None:
    """let binding with bcd type and integer literal."""
    assert tc("fn main() { let d: bcd = 7; }").ok


def test_let_bool_true() -> None:
    """let binding with bool type and 'true' literal."""
    assert tc("fn main() { let b: bool = true; }").ok


def test_let_bool_false() -> None:
    """let binding with bool type and 'false' literal."""
    assert tc("fn main() { let b: bool = false; }").ok


def test_assign_same_type() -> None:
    """Assignment where RHS has same type as declared variable."""
    assert tc("fn main() { let x: u4 = 0; let y: u4 = 5; x = y; }").ok


def test_fn_with_return() -> None:
    """Function with a return statement matching declared return type."""
    assert tc("fn f() -> u4 { return 5; }").ok


def test_fn_call_no_args() -> None:
    """Call to a no-argument, no-return function."""
    assert tc("fn helper() { } fn main() { helper(); }").ok


def test_fn_call_with_return() -> None:
    """Call to a function that returns u4, binding result to u4."""
    assert tc("fn f() -> u4 { return 1; } fn main() { let x: u4 = f(); }").ok


def test_bcd_wrap_add() -> None:
    """BCD wrapping addition (+%) is legal."""
    assert tc("fn main() { let d: bcd = 3 +% 4; }").ok


def test_bcd_sub() -> None:
    """BCD subtraction (-) is legal."""
    assert tc("fn main() { let d: bcd = 9 - 3; }").ok


def test_for_literal_bounds() -> None:
    """For-loop with integer literal bounds is valid."""
    assert tc("fn main() { for i: u8 in 0..10 { } }").ok


def test_for_const_bound() -> None:
    """For-loop whose upper bound is a const-declared name is valid."""
    assert tc("const N: u8 = 10; fn main() { for i: u8 in 0..N { } }").ok


def test_if_bool_literal() -> None:
    """If with a bool literal condition is valid."""
    assert tc("fn main() { if true { } }").ok


def test_if_else() -> None:
    """If/else with a bool literal condition is valid."""
    assert tc("fn main() { if true { } else { } }").ok


def test_wrap_add_u4() -> None:
    """Wrapping addition for u4 is legal."""
    assert tc("fn main() { let x: u4 = 1 +% 2; }").ok


def test_wrap_add_u8() -> None:
    """Wrapping addition for u8 is legal."""
    assert tc("fn main() { let x: u8 = 100 +% 200; }").ok


def test_fn_with_params() -> None:
    """Function with parameters; call with matching types."""
    assert tc("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() { let r: u4 = add(1, 2); }").ok


def test_const_used_in_expr() -> None:
    """A const name can be used in an expression."""
    assert tc("const C: u4 = 3; fn main() { let x: u4 = C; }").ok


def test_static_used_in_expr() -> None:
    """A static name can be used in an expression."""
    assert tc("static s: u4 = 0; fn main() { let x: u4 = s; }").ok


def test_hex_literal_u4() -> None:
    """Hex literal is valid for a u4 binding."""
    assert tc("fn main() { let x: u4 = 0xF; }").ok


def test_if_comparison_condition() -> None:
    """If with a comparison expression (produces bool) is valid."""
    assert tc("fn main() { let x: u4 = 5; if x == 5 { } }").ok


def test_multiple_let_stmts() -> None:
    """Multiple let statements in a function body."""
    assert tc("fn main() { let a: u4 = 1; let b: u4 = 2; let c: u4 = a +% b; }").ok


def test_nested_blocks() -> None:
    """Nested blocks (if inside if) are valid."""
    assert tc("fn main() { if true { if true { } } }").ok


def test_for_body_uses_loop_var() -> None:
    """The loop variable is available inside the for body."""
    assert tc("fn main() { for i: u4 in 0..8 { let x: u4 = i; } }").ok


def test_fn_void_no_return() -> None:
    """A void function with no return statement is valid."""
    assert tc("fn work() { let x: u4 = 1; }").ok


def test_fn_calls_another_fn() -> None:
    """Two functions where one calls the other — no recursion."""
    assert tc(
        "fn helper() -> u4 { return 1; } "
        "fn main() { let r: u4 = helper(); }"
    ).ok


def test_sat_add_u4() -> None:
    """Saturating addition for u4 is legal."""
    assert tc("fn main() { let x: u4 = 1 +? 2; }").ok


def test_bool_comparison_eq() -> None:
    """Equality comparison produces bool."""
    assert tc("fn main() { let b: bool = 1 == 1; }").ok


def test_if_comparison_lt() -> None:
    """Less-than comparison in if condition is valid."""
    assert tc("fn main() { let x: u4 = 3; if x == 3 { } }").ok


def test_multiple_fns_no_recursion() -> None:
    """Three functions calling a fourth — no cycles."""
    assert tc(
        "fn leaf() -> u4 { return 0; } "
        "fn a() -> u4 { return leaf(); } "
        "fn b() -> u4 { return leaf(); } "
        "fn main() { let x: u4 = a(); let y: u4 = b(); }"
    ).ok


# ---------------------------------------------------------------------------
# 2. Undeclared name errors
# ---------------------------------------------------------------------------


def test_undeclared_var_assign() -> None:
    """Assigning to an undeclared variable is an error."""
    result = tc("fn main() { x = 5; }")
    assert not result.ok
    assert any(
        "undeclared" in e.message.lower() or "not defined" in e.message.lower()
        for e in result.errors
    )


def test_undeclared_var_in_expr() -> None:
    """Using an undeclared variable in an expression is an error."""
    result = tc("fn main() { let y: u4 = x +% 1; }")
    assert not result.ok
    assert any("not defined" in e.message.lower() for e in result.errors)


def test_undeclared_fn_call() -> None:
    """Calling an undeclared function is an error."""
    result = tc("fn main() { let x: u4 = unknown(); }")
    assert not result.ok
    assert any("not defined" in e.message.lower() for e in result.errors)


def test_var_out_of_scope() -> None:
    """A variable declared inside an if block is not visible after it."""
    result = tc(
        "fn main() { "
        "  if true { let inner: u4 = 1; } "
        "  let y: u4 = inner; "
        "}"
    )
    assert not result.ok


# ---------------------------------------------------------------------------
# 3. Type mismatch errors
# ---------------------------------------------------------------------------


def test_type_mismatch_let() -> None:
    """let with declared type u4 but RHS is bool is an error."""
    result = tc("fn main() { let x: u4 = true; }")
    assert not result.ok


def test_type_mismatch_assign() -> None:
    """Assigning u8 variable to u4 declared variable is an error."""
    result = tc("fn main() { let x: u4 = 0; let y: u8 = 0; x = y; }")
    assert not result.ok


def test_type_mismatch_bool_to_u4() -> None:
    """Assigning bool to u4 in assignment is an error."""
    result = tc("fn main() { let x: u4 = 0; x = true; }")
    assert not result.ok


def test_type_mismatch_u4_to_bool() -> None:
    """Assigning u4 to bool in let is an error."""
    result = tc("fn main() { let b: bool = 5; }")
    assert not result.ok


# ---------------------------------------------------------------------------
# 4. BCD restriction errors
# ---------------------------------------------------------------------------


def test_bcd_bare_plus_error() -> None:
    """BCD type does not allow bare '+' — requires '+%' or '-'."""
    result = tc("fn main() { let d: bcd = 3 + 4; }")
    assert not result.ok
    assert any("bcd" in e.message.lower() for e in result.errors)


def test_bcd_sat_add_error() -> None:
    """BCD type does not allow '+?' (saturating add)."""
    result = tc("fn main() { let d: bcd = 3 +? 4; }")
    assert not result.ok
    assert any("bcd" in e.message.lower() for e in result.errors)


def test_bcd_plus_variable_error() -> None:
    """BCD variable used with bare '+' is an error."""
    result = tc("fn main() { let d: bcd = 5; let e: bcd = d + 1; }")
    assert not result.ok
    assert any("bcd" in e.message.lower() for e in result.errors)


# ---------------------------------------------------------------------------
# 5. For-loop semantics
# ---------------------------------------------------------------------------


def test_for_runtime_bound_variable() -> None:
    """A runtime numeric variable is a valid loop bound."""
    result = tc("fn main() { let n: u8 = 10; for i: u8 in 0..n { } }")
    assert result.ok


def test_for_bool_bound_error() -> None:
    """A boolean loop bound is still a type error."""
    result = tc("fn main() { let done: bool = false; for i: u8 in 0..done { } }")
    assert not result.ok
    assert any("numeric" in e.message.lower() for e in result.errors)


# ---------------------------------------------------------------------------
# Backend recursion checks now live in intel-4004-ir-validator tests.
# ---------------------------------------------------------------------------


    """A three-way cycle (a→b→c→a) is a recursion error."""


# ---------------------------------------------------------------------------
# 6. If-condition errors
# ---------------------------------------------------------------------------


def test_if_condition_u4() -> None:
    """u4 in if condition is an error — must be bool."""
    result = tc("fn main() { if 5 { } }")
    assert not result.ok
    assert any("bool" in e.message.lower() for e in result.errors)


def test_if_condition_u8() -> None:
    """u8 variable in if condition is an error."""
    result = tc("fn main() { let x: u8 = 1; if x { } }")
    assert not result.ok
    assert any("bool" in e.message.lower() for e in result.errors)


def test_if_condition_bcd() -> None:
    """bcd variable in if condition is an error."""
    result = tc("fn main() { let d: bcd = 3; if d { } }")
    assert not result.ok
    assert any("bool" in e.message.lower() for e in result.errors)


# ---------------------------------------------------------------------------
# 7. Function call errors
# ---------------------------------------------------------------------------


def test_call_arg_count_too_many() -> None:
    """Passing too many arguments to a function is an error."""
    result = tc("fn f(x: u4) { } fn main() { f(1, 2); }")
    assert not result.ok


def test_call_arg_count_too_few() -> None:
    """Passing too few arguments to a function is an error."""
    result = tc("fn f(x: u4, y: u4) { } fn main() { f(1); }")
    assert not result.ok


def test_call_arg_type_mismatch() -> None:
    """Wrong argument type (bool instead of u4) is an error."""
    result = tc("fn f(x: u4) { } fn main() { f(true); }")
    assert not result.ok


def test_call_arg_type_u8_for_u4() -> None:
    """Passing u8 where u4 is expected is an error."""
    result = tc(
        "fn f(x: u4) { } "
        "fn main() { let y: u8 = 5; f(y); }"
    )
    assert not result.ok


# ---------------------------------------------------------------------------
# 8. Return type errors
# ---------------------------------------------------------------------------


def test_return_type_mismatch_bool_for_u4() -> None:
    """Returning bool from a -> u4 function is an error."""
    result = tc("fn f() -> u4 { return true; }")
    assert not result.ok


def test_return_type_mismatch_u8_for_u4() -> None:
    """Returning u8 from a -> u4 function is an error."""
    result = tc(
        "fn f() -> u4 { "
        "  let x: u8 = 5; "
        "  return x; "
        "}"
    )
    assert not result.ok


def test_return_type_mismatch_u4_for_bool() -> None:
    """Returning a u4 from a -> bool function is an error."""
    result = tc("fn f() -> bool { return 5; }")
    assert not result.ok


# ---------------------------------------------------------------------------
# 9. Scope tests
# ---------------------------------------------------------------------------


def test_inner_scope_variable_not_visible_outside() -> None:
    """Variable declared in inner scope is not visible in outer scope."""
    result = tc(
        "fn main() { "
        "  if true { let inner: u4 = 1; } "
        "  let y: u4 = inner; "
        "}"
    )
    assert not result.ok


def test_outer_variable_visible_in_inner_scope() -> None:
    """Variable declared in outer scope is visible in inner scope."""
    assert tc(
        "fn main() { "
        "  let outer: u4 = 3; "
        "  if true { let x: u4 = outer; } "
        "}"
    ).ok


def test_fn_param_visible_in_body() -> None:
    """Function parameters are visible inside the function body."""
    assert tc(
        "fn double(x: u4) -> u4 { return x +% x; }"
    ).ok


def test_global_const_visible_in_fn() -> None:
    """A global const is visible inside any function."""
    assert tc(
        "const LIMIT: u4 = 10; "
        "fn main() { let x: u4 = LIMIT; }"
    ).ok


def test_global_static_visible_in_fn() -> None:
    """A global static is visible inside any function."""
    assert tc(
        "static counter: u8 = 0; "
        "fn main() { let x: u8 = counter; }"
    ).ok


def test_for_loop_var_visible_in_body() -> None:
    """The for-loop variable is visible inside the loop body."""
    assert tc(
        "fn main() { for i: u4 in 0..8 { let x: u4 = i; } }"
    ).ok


# ---------------------------------------------------------------------------
# 10. NibType and helpers unit tests
# ---------------------------------------------------------------------------


def test_nibtype_values() -> None:
    """NibType enum values match the source-level strings."""
    assert NibType.U4.value == "u4"
    assert NibType.U8.value == "u8"
    assert NibType.BCD.value == "bcd"
    assert NibType.BOOL.value == "bool"


def test_nibtype_size_bytes() -> None:
    """u8 takes 2 bytes; all others take 1 byte."""
    assert NibType.U8.size_bytes == 2
    assert NibType.U4.size_bytes == 1
    assert NibType.BCD.size_bytes == 1
    assert NibType.BOOL.size_bytes == 1


def test_parse_type_name_valid() -> None:
    """parse_type_name converts valid type strings correctly."""
    assert parse_type_name("u4") == NibType.U4
    assert parse_type_name("u8") == NibType.U8
    assert parse_type_name("bcd") == NibType.BCD
    assert parse_type_name("bool") == NibType.BOOL


def test_parse_type_name_invalid() -> None:
    """parse_type_name returns None for unknown type names."""
    assert parse_type_name("int") is None
    assert parse_type_name("float") is None
    assert parse_type_name("") is None


def test_types_are_compatible_same() -> None:
    """Same types are compatible."""
    assert types_are_compatible(NibType.U4, NibType.U4)
    assert types_are_compatible(NibType.U8, NibType.U8)
    assert types_are_compatible(NibType.BCD, NibType.BCD)
    assert types_are_compatible(NibType.BOOL, NibType.BOOL)


def test_types_are_compatible_different() -> None:
    """Different types are not compatible (no implicit widening)."""
    assert not types_are_compatible(NibType.U4, NibType.U8)
    assert not types_are_compatible(NibType.U8, NibType.U4)
    assert not types_are_compatible(NibType.BOOL, NibType.U4)
    assert not types_are_compatible(NibType.BCD, NibType.BOOL)


def test_is_bcd_op_allowed_permitted() -> None:
    """+% and - are the only permitted BCD operators."""
    assert is_bcd_op_allowed("+%")
    assert is_bcd_op_allowed("-")


def test_is_bcd_op_allowed_forbidden() -> None:
    """+, +?, *, / are forbidden BCD operators."""
    assert not is_bcd_op_allowed("+")
    assert not is_bcd_op_allowed("+?")
    assert not is_bcd_op_allowed("*")
    assert not is_bcd_op_allowed("/")


def test_is_numeric() -> None:
    """is_numeric returns True for u4/u8/bcd, False for bool."""
    assert is_numeric(NibType.U4)
    assert is_numeric(NibType.U8)
    assert is_numeric(NibType.BCD)
    assert not is_numeric(NibType.BOOL)


# ---------------------------------------------------------------------------
# 11. ScopeChain unit tests
# ---------------------------------------------------------------------------


def test_scope_chain_define_lookup() -> None:
    """Basic define and lookup in a ScopeChain."""
    sc = ScopeChain()
    sc.define("x", Symbol("x", NibType.U4))
    sym = sc.lookup("x")
    assert sym is not None
    assert sym.nib_type == NibType.U4


def test_scope_chain_lookup_missing() -> None:
    """Lookup of undefined name returns None."""
    sc = ScopeChain()
    assert sc.lookup("missing") is None


def test_scope_chain_push_pop() -> None:
    """Push creates inner scope; pop destroys it."""
    sc = ScopeChain()
    sc.define("outer", Symbol("outer", NibType.U4))
    sc.push()
    sc.define("inner", Symbol("inner", NibType.U8))
    assert sc.lookup("inner") is not None
    assert sc.lookup("outer") is not None
    sc.pop()
    assert sc.lookup("inner") is None
    assert sc.lookup("outer") is not None


def test_scope_chain_pop_global_raises() -> None:
    """Popping the global scope is a programming error."""
    sc = ScopeChain()
    with pytest.raises(RuntimeError):
        sc.pop()


def test_scope_chain_shadowing() -> None:
    """Inner scope can shadow outer scope name."""
    sc = ScopeChain()
    sc.define("x", Symbol("x", NibType.U4))
    sc.push()
    sc.define("x", Symbol("x", NibType.U8))
    inner_sym = sc.lookup("x")
    assert inner_sym is not None
    assert inner_sym.nib_type == NibType.U8  # inner shadow wins
    sc.pop()
    outer_sym = sc.lookup("x")
    assert outer_sym is not None
    assert outer_sym.nib_type == NibType.U4  # outer restored


def test_scope_chain_define_global() -> None:
    """define_global always inserts into the outermost scope."""
    sc = ScopeChain()
    sc.push()
    sc.push()
    sc.define_global("g", Symbol("g", NibType.U8, is_static=True))
    sc.pop()
    sc.pop()
    sym = sc.lookup("g")
    assert sym is not None
    assert sym.nib_type == NibType.U8


def test_symbol_flags() -> None:
    """Symbol is_const, is_static, is_fn flags work as expected."""
    s_const = Symbol("C", NibType.U4, is_const=True)
    assert s_const.is_const
    assert not s_const.is_static
    assert not s_const.is_fn

    s_static = Symbol("s", NibType.U8, is_static=True)
    assert s_static.is_static
    assert not s_static.is_const

    s_fn = Symbol("f", NibType.U4, is_fn=True, fn_params=[("a", NibType.U4)])
    assert s_fn.is_fn
    assert s_fn.fn_params == [("a", NibType.U4)]
