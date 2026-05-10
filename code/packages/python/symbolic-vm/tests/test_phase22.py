"""Phase 22 VM integration tests: matchdeclare, defrule, apply1, apply2, tellsimp.

Tests verify that the five new handlers are correctly wired into the VM,
that pattern variables declared via matchdeclare are respected by defrule,
and that tellsimp rules fire automatically during evaluation.

Helper pattern: _eval(src) compiles a MACSYMA string to IR and evaluates it.
"""

from __future__ import annotations

from symbolic_ir import (
    ADD,
    COS,
    EXP,
    LOG,
    MUL,
    POW,
    SIN,
    IRApply,
    IRInteger,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _eval_ir(vm: VM, node) -> object:
    return vm.eval(node)


# ---------------------------------------------------------------------------
# matchdeclare_handler
# ---------------------------------------------------------------------------


class TestMatchDeclareHandler:
    def test_single_arg_declares_any(self) -> None:
        vm = _make_vm()
        expr = IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),))
        result = vm.eval(expr)
        assert result == IRSymbol("done")
        assert vm.match_declarations.is_declared("x")
        assert vm.match_declarations.get_predicate("x") == "any"

    def test_two_args_declares_with_predicate(self) -> None:
        vm = _make_vm()
        expr = IRApply(IRSymbol("MatchDeclare"), (
            IRSymbol("n"), IRSymbol("integerp"),
        ))
        vm.eval(expr)
        assert vm.match_declarations.is_declared("n")
        assert vm.match_declarations.get_predicate("n") == "integerp"

    def test_symbolp_predicate(self) -> None:
        vm = _make_vm()
        expr = IRApply(IRSymbol("MatchDeclare"), (
            IRSymbol("a"), IRSymbol("symbolp"),
        ))
        vm.eval(expr)
        assert vm.match_declarations.get_predicate("a") == "symbolp"

    def test_true_predicate(self) -> None:
        vm = _make_vm()
        expr = IRApply(IRSymbol("MatchDeclare"), (
            IRSymbol("x"), IRSymbol("true"),
        ))
        vm.eval(expr)
        assert vm.match_declarations.is_declared("x")

    def test_malformed_returns_expr(self) -> None:
        vm = _make_vm()
        # No args
        expr = IRApply(IRSymbol("MatchDeclare"), ())
        result = vm.eval(expr)
        # Returns the expression unchanged (malformed)
        assert isinstance(result, IRApply)

    def test_non_symbol_first_arg_returns_expr(self) -> None:
        vm = _make_vm()
        expr = IRApply(IRSymbol("MatchDeclare"), (IRInteger(42),))
        result = vm.eval(expr)
        assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# defrule_handler
# ---------------------------------------------------------------------------


class TestDefruleHandler:
    def test_basic_defrule_stores_rule(self) -> None:
        vm = _make_vm()
        # matchdeclare(x, true)
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        # defrule(r1, sin(x)^2, 1 - cos(x)^2)
        x = IRSymbol("x")
        lhs = IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2)))
        cos_sq = IRApply(POW, (IRApply(COS, (x,)), IRInteger(2)))
        rhs = IRApply(ADD, (IRInteger(1), cos_sq))
        defrule_expr = IRApply(IRSymbol("Defrule"), (
            IRSymbol("r1"), lhs, rhs,
        ))
        result = vm.eval(defrule_expr)
        assert result == IRSymbol("r1")
        assert "r1" in vm.named_rules

    def test_defrule_returns_name_symbol(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        lhs = IRApply(SIN, (IRSymbol("x"),))
        rhs = IRApply(SIN, (IRSymbol("x"),))
        result = vm.eval(IRApply(IRSymbol("Defrule"), (IRSymbol("myRule"), lhs, rhs)))
        assert result == IRSymbol("myRule")

    def test_defrule_with_integerp_var(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (
            IRSymbol("n"), IRSymbol("integerp"),
        )))
        n = IRSymbol("n")
        # defrule(double, n+n, 2*n)
        lhs = IRApply(ADD, (n, n))
        rhs = IRApply(MUL, (IRInteger(2), n))
        vm.eval(IRApply(IRSymbol("Defrule"), (IRSymbol("double"), lhs, rhs)))
        assert "double" in vm.named_rules

    def test_defrule_malformed_wrong_arity(self) -> None:
        vm = _make_vm()
        # Only two args — malformed
        result = vm.eval(IRApply(IRSymbol("Defrule"), (
            IRSymbol("r"), IRSymbol("x"),
        )))
        assert isinstance(result, IRApply)

    def test_defrule_malformed_non_symbol_name(self) -> None:
        vm = _make_vm()
        result = vm.eval(IRApply(IRSymbol("Defrule"), (
            IRInteger(1), IRSymbol("x"), IRInteger(0),
        )))
        assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# apply1_handler
# ---------------------------------------------------------------------------


class TestApply1Handler:
    def test_apply1_root_match(self) -> None:
        vm = _make_vm()
        # Declare x and define rule: sin(x)^2 + cos(x)^2 → 1
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        lhs = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
        ))
        vm.eval(IRApply(IRSymbol("Defrule"), (IRSymbol("pyth"), lhs, IRInteger(1))))
        # apply1(pyth, sin(t)^2 + cos(t)^2) → 1
        t = IRSymbol("t")
        target = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (t,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (t,)), IRInteger(2))),
        ))
        result = vm.eval(IRApply(IRSymbol("Apply1"), (IRSymbol("pyth"), target)))
        assert result == IRInteger(1)

    def test_apply1_no_match_returns_target(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        lhs = IRApply(SIN, (x,))
        vm.eval(IRApply(IRSymbol("Defrule"), (
            IRSymbol("sinzero"), lhs, IRInteger(0),
        )))
        # cos(t) doesn't match sin(x) — no match
        t = IRSymbol("t")
        target = IRApply(COS, (t,))
        result = vm.eval(IRApply(IRSymbol("Apply1"), (IRSymbol("sinzero"), target)))
        assert result == target

    def test_apply1_unknown_rule_returns_target(self) -> None:
        vm = _make_vm()
        t = IRSymbol("t")
        target = IRApply(SIN, (t,))
        result = vm.eval(IRApply(IRSymbol("Apply1"), (IRSymbol("nosuchrule"), target)))
        assert result == target

    def test_apply1_root_only_not_recursive(self) -> None:
        """apply1 matches at root only; nested matches are not found."""
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        # sin(x)^2 + cos(x)^2 → 1
        lhs = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
        ))
        vm.eval(IRApply(IRSymbol("Defrule"), (IRSymbol("pyth"), lhs, IRInteger(1))))
        # Target is wrapped in another Add: 2 + (sin(t)^2 + cos(t)^2)
        # apply1 tries rule only at root — outer Add won't match sin^2+cos^2
        t = IRSymbol("t")
        inner = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (t,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (t,)), IRInteger(2))),
        ))
        outer = IRApply(ADD, (IRInteger(2), inner))
        result = vm.eval(IRApply(IRSymbol("Apply1"), (IRSymbol("pyth"), outer)))
        # Root doesn't match → target returned unchanged
        assert isinstance(result, IRApply)
        assert result.head == ADD

    def test_apply1_malformed_wrong_arity(self) -> None:
        vm = _make_vm()
        result = vm.eval(IRApply(IRSymbol("Apply1"), (IRSymbol("r"),)))
        assert isinstance(result, IRApply)

    def test_apply1_malformed_non_symbol_name(self) -> None:
        vm = _make_vm()
        result = vm.eval(IRApply(IRSymbol("Apply1"), (
            IRInteger(1), IRSymbol("x"),
        )))
        assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# apply2_handler
# ---------------------------------------------------------------------------


class TestApply2Handler:
    def test_apply2_recursive(self) -> None:
        """apply2 rewrites everywhere in the tree."""
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        # sin(x)^2 + cos(x)^2 → 1
        lhs = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
        ))
        vm.eval(IRApply(IRSymbol("Defrule"), (IRSymbol("pyth"), lhs, IRInteger(1))))
        t = IRSymbol("t")
        # 3 + (sin(t)^2 + cos(t)^2)
        inner = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (t,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (t,)), IRInteger(2))),
        ))
        outer = IRApply(ADD, (IRInteger(3), inner))
        result = vm.eval(IRApply(IRSymbol("Apply2"), (IRSymbol("pyth"), outer)))
        # inner fires → 1, then 3+1 = 4
        assert result == IRInteger(4)

    def test_apply2_no_match_returns_target(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        vm.eval(IRApply(IRSymbol("Defrule"), (
            IRSymbol("sinzero"),
            IRApply(SIN, (x,)),
            IRInteger(0),
        )))
        target = IRApply(COS, (IRSymbol("u"),))
        result = vm.eval(IRApply(IRSymbol("Apply2"), (IRSymbol("sinzero"), target)))
        assert result == target

    def test_apply2_unknown_rule_returns_target(self) -> None:
        vm = _make_vm()
        target = IRApply(SIN, (IRSymbol("z"),))
        result = vm.eval(IRApply(IRSymbol("Apply2"), (IRSymbol("ghost"), target)))
        assert result == target

    def test_apply2_malformed_wrong_arity(self) -> None:
        vm = _make_vm()
        result = vm.eval(IRApply(IRSymbol("Apply2"), (IRSymbol("r"),)))
        assert isinstance(result, IRApply)

    def test_apply2_integerp_rule(self) -> None:
        """integerp rule fires on integer but not on symbol."""
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (
            IRSymbol("n"), IRSymbol("integerp"),
        )))
        n = IRSymbol("n")
        # n^0 → 1  (for integers only)
        lhs = IRApply(POW, (n, IRInteger(0)))
        vm.eval(IRApply(IRSymbol("Defrule"), (
            IRSymbol("powzero"), lhs, IRInteger(1),
        )))
        # 5^0 should fire
        target = IRApply(POW, (IRInteger(5), IRInteger(0)))
        result = vm.eval(IRApply(IRSymbol("Apply2"), (IRSymbol("powzero"), target)))
        assert result == IRInteger(1)


# ---------------------------------------------------------------------------
# tellsimp_handler — automatic application
# ---------------------------------------------------------------------------


class TestTellSimpHandler:
    def test_tellsimp_fires_automatically(self) -> None:
        """After tellsimp, a matching expression evaluates to the RHS."""
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        # tellsimp(sin(x)^2 + cos(x)^2, 1)
        lhs = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
        ))
        vm.eval(IRApply(IRSymbol("TellSimp"), (lhs, IRInteger(1))))
        # Now evaluate sin(t)^2 + cos(t)^2 directly — should → 1
        t = IRSymbol("t")
        target = IRApply(ADD, (
            IRApply(POW, (IRApply(SIN, (t,)), IRInteger(2))),
            IRApply(POW, (IRApply(COS, (t,)), IRInteger(2))),
        ))
        result = vm.eval(target)
        assert result == IRInteger(1)

    def test_tellsimp_returns_done(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        lhs = IRApply(SIN, (x,))
        result = vm.eval(IRApply(IRSymbol("TellSimp"), (lhs, IRInteger(0))))
        assert result == IRSymbol("done")

    def test_tellsimp_multiple_rules_first_match_wins(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        # First rule: sin(x)^2 → 1
        lhs1 = IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2)))
        vm.eval(IRApply(IRSymbol("TellSimp"), (lhs1, IRInteger(1))))
        # Second rule: sin(x)^2 → 2 (would override, but first rule fires)
        vm.eval(IRApply(IRSymbol("TellSimp"), (lhs1, IRInteger(2))))
        # sin(t)^2 — first tellsimp rule fires first
        t = IRSymbol("t")
        result = vm.eval(IRApply(POW, (IRApply(SIN, (t,)), IRInteger(2))))
        assert result == IRInteger(1)

    def test_tellsimp_accumulates_rules(self) -> None:
        vm = _make_vm()
        assert len(vm.tellsimp_rules) == 0
        vm.eval(IRApply(IRSymbol("MatchDeclare"), (IRSymbol("x"),)))
        x = IRSymbol("x")
        vm.eval(IRApply(IRSymbol("TellSimp"), (
            IRApply(SIN, (x,)), IRInteger(0),
        )))
        assert len(vm.tellsimp_rules) == 1
        vm.eval(IRApply(IRSymbol("TellSimp"), (
            IRApply(COS, (x,)), IRInteger(1),
        )))
        assert len(vm.tellsimp_rules) == 2

    def test_tellsimp_malformed_wrong_arity(self) -> None:
        vm = _make_vm()
        result = vm.eval(IRApply(IRSymbol("TellSimp"), (IRSymbol("x"),)))
        assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# Regression: prior phases unaffected
# ---------------------------------------------------------------------------


class TestPhase22Regressions:
    def test_assume_still_works(self) -> None:
        vm = _make_vm()
        vm.eval(IRApply(IRSymbol("Assume"), (
            IRApply(IRSymbol("Greater"), (IRSymbol("x"), IRInteger(0))),
        )))
        assert vm.assumptions.is_positive("x") is True

    def test_radcan_still_works(self) -> None:
        # Use Exp(Log(t)) → t which stays symbolic (no pre-eval to float).
        vm = _make_vm()
        t = IRSymbol("t")
        result = vm.eval(IRApply(IRSymbol("Radcan"), (
            IRApply(EXP, (IRApply(LOG, (t,)),)),
        )))
        assert result == t

    def test_basic_arithmetic_unchanged(self) -> None:
        vm = _make_vm()
        result = vm.eval(IRApply(ADD, (IRInteger(2), IRInteger(3))))
        assert result == IRInteger(5)
