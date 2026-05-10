"""Tests for Phase 22: MatchDeclareContext and RuleStore.

These tests cover:
  - MatchDeclareContext: declare / forget / compile_pattern
  - Predicate → Blank constraint mapping
  - Pattern compilation: simple, nested, multiple variables
  - RuleStore: store / get / remove / clear / names / len / contains
  - End-to-end: compile + match + apply_rule round-trips
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    MUL,
    POW,
    SIN,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRString,
    IRSymbol,
)

from cas_pattern_matching.defrule_engine import RuleStore
from cas_pattern_matching.matchdeclare import MatchDeclareContext
from cas_pattern_matching.nodes import (
    Rule,
    is_blank,
    is_pattern,
)
from cas_pattern_matching.rewriter import apply_rule, rewrite

# ---------------------------------------------------------------------------
# MatchDeclareContext — declare / forget / query
# ---------------------------------------------------------------------------


class TestMatchDeclareContextMutation:
    def test_declare_is_declared(self) -> None:
        ctx = MatchDeclareContext()
        assert not ctx.is_declared("x")
        ctx.declare("x", "any")
        assert ctx.is_declared("x")

    def test_declare_stores_lower(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("n", "IntegerP")
        assert ctx.get_predicate("n") == "integerp"

    def test_declare_overwrites(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "symbolp")
        ctx.declare("x", "floatp")
        assert ctx.get_predicate("x") == "floatp"

    def test_forget_single(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        ctx.forget("x")
        assert not ctx.is_declared("x")

    def test_forget_nonexistent_noop(self) -> None:
        ctx = MatchDeclareContext()
        ctx.forget("z")  # should not raise

    def test_forget_all(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        ctx.declare("n", "integerp")
        ctx.forget_all()
        assert not ctx.is_declared("x")
        assert not ctx.is_declared("n")

    def test_get_predicate_unknown(self) -> None:
        ctx = MatchDeclareContext()
        assert ctx.get_predicate("y") is None


# ---------------------------------------------------------------------------
# MatchDeclareContext — compile_pattern: atoms
# ---------------------------------------------------------------------------


class TestCompilePatternAtoms:
    def test_undeclared_symbol_unchanged(self) -> None:
        ctx = MatchDeclareContext()
        x = IRSymbol("x")
        assert ctx.compile_pattern(x) is x

    def test_declared_symbol_any_becomes_blank(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        compiled = ctx.compile_pattern(IRSymbol("x"))
        assert is_pattern(compiled)
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert is_blank(inner)

    def test_declared_symbol_integerp_blank_integer(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("n", "integerp")
        compiled = ctx.compile_pattern(IRSymbol("n"))
        assert is_pattern(compiled)
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert is_blank(inner)
        # Blank("Integer") has one arg: IRSymbol("Integer")
        assert isinstance(inner, IRApply)
        assert len(inner.args) == 1
        assert isinstance(inner.args[0], IRSymbol)
        assert inner.args[0].name == "Integer"

    def test_declared_symbol_symbolp_blank_symbol(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("a", "symbolp")
        compiled = ctx.compile_pattern(IRSymbol("a"))
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert isinstance(inner, IRApply)
        assert inner.args[0].name == "Symbol"

    def test_declared_symbol_floatp_blank_float(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("f", "floatp")
        compiled = ctx.compile_pattern(IRSymbol("f"))
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert inner.args[0].name == "Float"

    def test_declared_symbol_rationalp_blank_rational(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("r", "rationalp")
        compiled = ctx.compile_pattern(IRSymbol("r"))
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert inner.args[0].name == "Rational"

    def test_declared_symbol_listp_blank_list(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("lst", "listp")
        compiled = ctx.compile_pattern(IRSymbol("lst"))
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert inner.args[0].name == "List"

    def test_declared_symbol_numberp_unconstrained(self) -> None:
        # numberp → unconstrained Blank() because it's a union type
        ctx = MatchDeclareContext()
        ctx.declare("n", "numberp")
        compiled = ctx.compile_pattern(IRSymbol("n"))
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert len(inner.args) == 0  # Blank() has no args

    def test_declared_symbol_true_unconstrained(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "true")
        compiled = ctx.compile_pattern(IRSymbol("x"))
        assert isinstance(compiled, IRApply)
        inner = compiled.args[1]
        assert len(inner.args) == 0

    def test_integer_literal_unchanged(self) -> None:
        ctx = MatchDeclareContext()
        n = IRInteger(42)
        assert ctx.compile_pattern(n) is n

    def test_rational_literal_unchanged(self) -> None:
        ctx = MatchDeclareContext()
        r = IRRational(1, 3)
        assert ctx.compile_pattern(r) is r

    def test_float_literal_unchanged(self) -> None:
        ctx = MatchDeclareContext()
        f = IRFloat(3.14)
        assert ctx.compile_pattern(f) is f

    def test_string_literal_unchanged(self) -> None:
        ctx = MatchDeclareContext()
        s = IRString("hello")
        assert ctx.compile_pattern(s) is s


# ---------------------------------------------------------------------------
# MatchDeclareContext — compile_pattern: compound IR
# ---------------------------------------------------------------------------


class TestCompilePatternCompound:
    def test_no_declarations_identity(self) -> None:
        ctx = MatchDeclareContext()
        expr = IRApply(ADD, (IRSymbol("x"), IRInteger(1)))
        result = ctx.compile_pattern(expr)
        assert result == expr

    def test_single_var_in_apply(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        expr = IRApply(SIN, (IRSymbol("x"),))
        compiled = ctx.compile_pattern(expr)
        assert isinstance(compiled, IRApply)
        assert compiled.head == SIN
        assert is_pattern(compiled.args[0])

    def test_two_vars_sin_cos_pythagoras(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        # sin(x)^2 + cos(x)^2
        x = IRSymbol("x")
        pattern = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
        ))
        compiled = ctx.compile_pattern(pattern)
        # Top-level ADD unchanged
        assert isinstance(compiled, IRApply)
        assert compiled.head == ADD
        # Both POW branches have SIN/COS of Pattern("x", ...)
        sin_branch = compiled.args[0]
        cos_branch = compiled.args[1]
        assert sin_branch.head == POW
        assert cos_branch.head == POW
        assert is_pattern(sin_branch.args[0].args[0])
        assert is_pattern(cos_branch.args[0].args[0])

    def test_mixed_declared_undeclared(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        # x + y — only x is declared
        expr = IRApply(ADD, (IRSymbol("x"), IRSymbol("y")))
        compiled = ctx.compile_pattern(expr)
        assert isinstance(compiled, IRApply)
        assert is_pattern(compiled.args[0])   # x → Pattern
        assert isinstance(compiled.args[1], IRSymbol)  # y unchanged
        assert compiled.args[1].name == "y"

    def test_same_var_twice_both_replaced(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        # x + x
        expr = IRApply(ADD, (IRSymbol("x"), IRSymbol("x")))
        compiled = ctx.compile_pattern(expr)
        assert is_pattern(compiled.args[0])
        assert is_pattern(compiled.args[1])


# ---------------------------------------------------------------------------
# End-to-end: compile_pattern → match → apply_rule
# ---------------------------------------------------------------------------


class TestMatchDeclareEndToEnd:
    def test_pythagorean_rule_fires(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        x = IRSymbol("x")
        lhs = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
        ))
        compiled_lhs = ctx.compile_pattern(lhs)
        rule = Rule(compiled_lhs, IRInteger(1))
        # Target: sin(t)^2 + cos(t)^2
        t = IRSymbol("t")
        target = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (t,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (t,)), IRInteger(2))),
        ))
        result = apply_rule(rule, target)
        assert result == IRInteger(1)

    def test_integerp_rule_fires_on_integer(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("n", "integerp")
        n = IRSymbol("n")
        lhs = IRApply(ADD, (n, n))
        compiled_lhs = ctx.compile_pattern(lhs)
        rule = Rule(compiled_lhs, IRApply(MUL, (IRInteger(2), n)))
        # Target: 3 + 3
        target = IRApply(ADD, (IRInteger(3), IRInteger(3)))
        result = apply_rule(rule, target)
        assert result is not None

    def test_integerp_rule_no_match_on_symbol(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("n", "integerp")
        n = IRSymbol("n")
        lhs = IRApply(ADD, (n, n))
        compiled_lhs = ctx.compile_pattern(lhs)
        rule = Rule(compiled_lhs, IRApply(MUL, (IRInteger(2), n)))
        # Target: x + x — x is a Symbol, not Integer
        target = IRApply(ADD, (IRSymbol("x"), IRSymbol("x")))
        result = apply_rule(rule, target)
        assert result is None

    def test_recursive_rewrite_apply2_style(self) -> None:
        ctx = MatchDeclareContext()
        ctx.declare("x", "any")
        x = IRSymbol("x")
        # x^1 → x  (both LHS and RHS compiled so _substitute replaces Pattern nodes)
        lhs = IRApply(POW, (x, IRInteger(1)))
        compiled_lhs = ctx.compile_pattern(lhs)
        compiled_rhs = ctx.compile_pattern(x)  # Pattern("x", Blank())
        rule = Rule(compiled_lhs, compiled_rhs)
        # Target: (a^1) + (b^1)
        a, b = IRSymbol("a"), IRSymbol("b")
        target = IRApply(ADD, (
            IRApply(POW, (a, IRInteger(1))),
            IRApply(POW, (b, IRInteger(1))),
        ))
        result = rewrite(target, [rule])
        assert result == IRApply(ADD, (a, b))


# ---------------------------------------------------------------------------
# RuleStore
# ---------------------------------------------------------------------------


class TestRuleStore:
    def test_empty_store(self) -> None:
        store = RuleStore()
        assert len(store) == 0
        assert store.names() == []

    def test_store_and_get(self) -> None:
        store = RuleStore()
        rule = Rule(IRSymbol("x"), IRInteger(1))
        store.store("r1", rule)
        assert store.get("r1") is rule

    def test_get_missing_returns_none(self) -> None:
        store = RuleStore()
        assert store.get("nonexistent") is None

    def test_contains(self) -> None:
        store = RuleStore()
        rule = Rule(IRSymbol("x"), IRInteger(0))
        store.store("r", rule)
        assert "r" in store
        assert "q" not in store

    def test_overwrite(self) -> None:
        store = RuleStore()
        rule1 = Rule(IRSymbol("x"), IRInteger(1))
        rule2 = Rule(IRSymbol("y"), IRInteger(2))
        store.store("r", rule1)
        store.store("r", rule2)
        assert store.get("r") is rule2
        assert len(store) == 1

    def test_remove(self) -> None:
        store = RuleStore()
        rule = Rule(IRSymbol("x"), IRInteger(1))
        store.store("r", rule)
        store.remove("r")
        assert store.get("r") is None
        assert len(store) == 0

    def test_remove_nonexistent_noop(self) -> None:
        store = RuleStore()
        store.remove("ghost")  # should not raise

    def test_clear(self) -> None:
        store = RuleStore()
        store.store("r1", Rule(IRSymbol("a"), IRInteger(1)))
        store.store("r2", Rule(IRSymbol("b"), IRInteger(2)))
        store.clear()
        assert len(store) == 0
        assert store.names() == []

    def test_names_sorted(self) -> None:
        store = RuleStore()
        store.store("zebra", Rule(IRSymbol("z"), IRInteger(0)))
        store.store("alpha", Rule(IRSymbol("a"), IRInteger(1)))
        store.store("middle", Rule(IRSymbol("m"), IRInteger(2)))
        assert store.names() == ["alpha", "middle", "zebra"]

    def test_repr(self) -> None:
        store = RuleStore()
        assert "RuleStore" in repr(store)
