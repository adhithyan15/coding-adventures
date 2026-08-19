"""Unit tests for the SIR23 Tier A symbolic-expression + pattern/rewrite
Python runtime. Mirrors the published TypeScript sibling package's own
``tests/sir-runtime-symbolic.test.ts`` case-for-case (see that file's own
comments for the rationale behind each scenario), translated to pytest.
"""

from __future__ import annotations

import pytest
from cas_pattern_matching import Bindings
from symbolic_ir import ADD, IRApply, IRFloat, IRInteger, IRRational, IRString, IRSymbol

from coding_adventures_sir_runtime_symbolic import (
    MAX_TERM_DEPTH,
    DepthLimitError,
    RewriteCycleError,
    apply,
    apply_rule,
    blank,
    blank_typed,
    int_,
    is_blank,
    is_pattern,
    is_rule,
    match_pattern,
    named,
    number_node,
    rational,
    replace_all,
    replace_repeated,
    rule,
    rule_delayed,
    string_node,
    substitute,
    sym,
    unwrap,
)
from coding_adventures_sir_runtime_symbolic.runtime import _is_walk_error

# Reusable "x_ + 0 -> x_" identity-elimination rule, the running example in
# this package's own module docstring and the Rust/TS/Ruby sibling
# packages' own examples.
X_PAT = named("x", blank())
DROP_ADD_ZERO = rule(apply(sym("Add"), [X_PAT, int_(0)]), X_PAT)


class TestReexportedMatcherPrimitives:
    def test_match_pattern_apply_rule_substitute_all_work(self) -> None:
        bindings = match_pattern(named("a", blank()), int_(5), Bindings())
        assert bindings is not None
        assert bindings["a"] == int_(5)
        assert apply_rule(DROP_ADD_ZERO, apply(sym("Add"), [sym("z"), int_(0)])) == sym("z")
        assert substitute(X_PAT, Bindings().bind("x", int_(9))) == int_(9)

    def test_apply_is_a_thin_wrapper_matching_symbolic_ir(self) -> None:
        assert apply(sym("Add"), [int_(1), int_(2)]) == IRApply(
            IRSymbol("Add"), (IRInteger(1), IRInteger(2))
        )

    def test_is_blank_is_pattern_is_rule(self) -> None:
        assert is_blank(blank())
        assert not is_blank(sym("x"))
        assert is_pattern(named("a", blank()))
        assert not is_pattern(sym("x"))
        assert is_rule(DROP_ADD_ZERO)
        assert not is_rule(sym("x"))

    def test_apply_rule_raises_on_a_non_rule(self) -> None:
        with pytest.raises(ValueError, match="Rule/RuleDelayed"):
            apply_rule(sym("not-a-rule"), int_(1))


class TestReplaceAll:
    def test_fires_once_per_matching_subtree_everywhere(self) -> None:
        # Add(Add(z, 0), 0) -- both "+ 0"s should be eliminated in one call.
        expr = apply(sym("Add"), [apply(sym("Add"), [sym("z"), int_(0)]), int_(0)])
        assert replace_all(expr, [DROP_ADD_ZERO]) == sym("z")

    def test_does_not_retry_against_its_own_fresh_replacement(self) -> None:
        # A rule whose RHS is itself a fresh match candidate for the SAME
        # rule: f(a) -> f(f(a)). A single pass must fire exactly once at
        # the root (producing f(f(a))) and must NOT loop forever retrying
        # at that spot -- this is the one behavior that actually
        # distinguishes replace_all from replace_repeated.
        f = sym("f")
        self_wrap = rule(apply(f, [X_PAT]), apply(f, [apply(f, [X_PAT])]))
        result = replace_all(apply(f, [sym("a")]), [self_wrap])
        assert result == apply(f, [apply(f, [sym("a")])])

    def test_walks_bottom_up_child_rewrite_visible_to_parent(self) -> None:
        # g(Add(z, 0)) with DROP_ADD_ZERO -- if children are rewritten
        # first (bottom-up), the parent sees g(z). A rule targeting
        # exactly g(z) can then fire in the SAME pass, proving traversal
        # order.
        g = sym("g")
        g_of_z = rule(apply(g, [sym("z")]), sym("matched-g-of-z"))
        expr = apply(g, [apply(sym("Add"), [sym("z"), int_(0)])])
        assert replace_all(expr, [DROP_ADD_ZERO, g_of_z]) == sym("matched-g-of-z")

    def test_leaves_a_non_matching_tree_unchanged(self) -> None:
        expr = apply(sym("Add"), [sym("a"), sym("b")])
        assert replace_all(expr, [DROP_ADD_ZERO]) == expr

    def test_leaf_term_with_no_apply_structure_still_walked(self) -> None:
        # A bare symbol (no IRApply wrapper) still goes through the rule
        # loop at depth 0 -- exercises the "node is not an IRApply, skip
        # straight to the rule loop" path of `_walk_once`.
        assert replace_all(sym("a"), [rule(sym("a"), sym("b"))]) == sym("b")


class TestReplaceRepeated:
    def test_matches_replace_all_on_a_single_firing_case(self) -> None:
        expr = apply(sym("Add"), [apply(sym("Add"), [sym("z"), int_(0)]), int_(0)])
        assert replace_repeated(expr, [DROP_ADD_ZERO], 100) == sym("z")

    def test_keeps_applying_until_a_true_fixed_point(self) -> None:
        # Add(Add(Add(z, 0), 0), 0) -- three nested "+0"s.
        expr = apply(
            sym("Add"),
            [apply(sym("Add"), [apply(sym("Add"), [sym("z"), int_(0)]), int_(0)]), int_(0)],
        )
        assert replace_repeated(expr, [DROP_ADD_ZERO], 100) == sym("z")

    def test_reports_rewrite_cycle_error_instead_of_hanging(self) -> None:
        # f(a) -> f(f(a)) retried forever would never reach a fixed point.
        f = sym("f")
        never_converges = rule(apply(f, [X_PAT]), apply(f, [apply(f, [X_PAT])]))
        result = replace_repeated(apply(f, [sym("a")]), [never_converges], 50)
        assert isinstance(result, RewriteCycleError)
        assert result.max_iterations == 50

    def test_leaves_a_non_matching_tree_unchanged(self) -> None:
        expr = apply(sym("Add"), [sym("a"), sym("b")])
        assert replace_repeated(expr, [DROP_ADD_ZERO], 100) == expr

    def test_survives_huge_max_iterations_without_a_stack_overflow(self) -> None:
        # Isolates the exact bug the loop-based retry design fixes: a ->
        # b, b -> a cycles forever WITHOUT ever building deeper tree
        # structure (both sides are bare symbols, never an Apply), so
        # MAX_TERM_DEPTH's tree-descent check never even triggers here --
        # if the retry step still recursed once per firing, 50,000
        # firings would mean 50,000 nested native stack frames and a real
        # stack overflow. The loop-based-retry implementation costs O(1)
        # stack per firing regardless, so this resolves cleanly to
        # RewriteCycleError instead.
        a, b = sym("a"), sym("b")
        a_to_b = rule(a, b)
        b_to_a = rule(b, a)
        result = replace_repeated(a, [a_to_b, b_to_a], 50_000)
        assert isinstance(result, RewriteCycleError)
        assert result.max_iterations == 50_000

    def test_leaf_term_with_no_apply_structure_still_walked(self) -> None:
        assert replace_repeated(sym("a"), [rule(sym("a"), sym("b"))], 100) == sym("b")


class TestDepthGuard:
    @staticmethod
    def _deep_chain(depth: int):  # noqa: ANN205
        # Build a right-leaning chain f(f(f(...f(leaf)...))) `depth`
        # levels deep.
        f = sym("f")
        node = sym("leaf")
        for _ in range(depth):
            node = apply(f, [node])
        return node

    def test_replace_all_succeeds_within_the_cap(self) -> None:
        shallow = self._deep_chain(MAX_TERM_DEPTH - 10)
        result = replace_all(shallow, [])
        assert not isinstance(result, DepthLimitError)

    def test_replace_all_reports_depth_limit_error_past_the_cap(self) -> None:
        too_deep = self._deep_chain(MAX_TERM_DEPTH * 4)
        result = replace_all(too_deep, [])
        assert isinstance(result, DepthLimitError)
        assert result.max_depth == MAX_TERM_DEPTH

    def test_replace_repeated_reports_depth_limit_error_past_the_cap(self) -> None:
        too_deep = self._deep_chain(MAX_TERM_DEPTH * 4)
        result = replace_repeated(too_deep, [], 100)
        assert isinstance(result, DepthLimitError)

    def test_replace_repeated_depth_limit_surfaces_from_args_not_just_head(self) -> None:
        # A deep chain nested as the SECOND arg of a shallow apply --
        # exercises the `for arg in current.args` loop's own error
        # propagation in `replace_repeated`'s `walk`, not just the `head`
        # path.
        too_deep = self._deep_chain(MAX_TERM_DEPTH * 4)
        wrapper = apply(sym("Pair"), [sym("shallow"), too_deep])
        result = replace_repeated(wrapper, [], 100)
        assert isinstance(result, DepthLimitError)

    def test_replace_all_depth_limit_surfaces_from_args_not_just_head(self) -> None:
        too_deep = self._deep_chain(MAX_TERM_DEPTH * 4)
        wrapper = apply(sym("Pair"), [sym("shallow"), too_deep])
        result = replace_all(wrapper, [])
        assert isinstance(result, DepthLimitError)

    def test_depth_limit_error_and_rewrite_cycle_error_do_not_cross_match(self) -> None:
        depth_err = DepthLimitError()
        cycle_err = RewriteCycleError()
        assert isinstance(depth_err, DepthLimitError)
        assert not isinstance(cycle_err, DepthLimitError)
        assert isinstance(cycle_err, RewriteCycleError)
        assert not isinstance(depth_err, RewriteCycleError)
        assert not isinstance(int_(1), (DepthLimitError, RewriteCycleError))

    def test_is_walk_error_type_guard(self) -> None:
        assert _is_walk_error(DepthLimitError())
        assert _is_walk_error(RewriteCycleError())
        assert not _is_walk_error(int_(1))


class TestRuleVsRuleDelayed:
    def test_currently_match_and_substitute_identically(self) -> None:
        eager = rule(apply(sym("Add"), [X_PAT, int_(0)]), X_PAT)
        delayed = rule_delayed(apply(sym("Add"), [X_PAT, int_(0)]), X_PAT)
        target = apply(sym("Add"), [sym("z"), int_(0)])

        assert apply_rule(eager, target) == apply_rule(delayed, target)
        assert replace_all(target, [eager]) == replace_all(target, [delayed])
        assert replace_repeated(target, [eager], 10) == replace_repeated(target, [delayed], 10)


class TestBlankAndBlankTyped:
    def test_typed_blank_matches_only_the_constrained_head(self) -> None:
        x_pat = named("x", blank_typed("Integer"))
        r = rule(apply(sym("f"), [x_pat]), x_pat)
        int_term = apply(sym("f"), [int_(5)])
        sym_term = apply(sym("f"), [sym("z")])
        assert replace_all(int_term, [r]) == int_(5)
        assert replace_all(sym_term, [r]) == sym_term  # no match, unchanged


class TestReexportedLeafTermConstructors:
    """These exist so a compiled `SymSymbol`/`SymRational` node -- or a
    bare `IntLit`/`FloatLit`/`StrLit` appearing as a child of a
    `SymApply`/`SymRule`/`SymReplaceAll` -- has somewhere to get wrapped
    into a real `IRNode` at the single import site a generated module
    uses; see the SIR23 codegen in `semantic-ir-to-python`'s
    `emit_sym_operand`."""

    def test_constructors_match_symbolic_ir_own_constructors(self) -> None:
        assert sym("z") == IRSymbol("z")
        assert int_(5) == IRInteger(5)
        assert rational(1, 3) == IRRational(1, 3)
        assert number_node(1.5) == IRFloat(1.5)
        assert string_node("hi") == IRString("hi")

    def test_rational_reduces_exactly_as_symbolic_ir_own_rational_does(self) -> None:
        # 2/4 should reduce to 1/2, proving this goes through
        # IRRational's own __post_init__ gcd reduction and is not a naive
        # pass-through.
        assert rational(2, 4) == rational(1, 2)
        assert rational(2, 4) == IRRational(1, 2)

    def test_a_leaf_constructor_round_trips_through_apply(self) -> None:
        expr = apply(sym("f"), [int_(2), rational(1, 3), string_node("x")])
        assert expr == IRApply(
            IRSymbol("f"), (IRInteger(2), IRRational(1, 3), IRString("x"))
        )

    def test_standard_head_symbol_add_is_reusable(self) -> None:
        # ADD is symbolic-ir's own shared IRSymbol("Add") singleton --
        # confirms `sym("Add")` round-trips through this package into the
        # SAME structural value the shared standard-vocabulary constant
        # produces.
        assert sym("Add") == ADD


class TestUnwrap:
    def test_returns_the_ir_node_unchanged_on_success(self) -> None:
        expr = apply(sym("Add"), [sym("z"), int_(0)])
        assert unwrap(replace_all(expr, [DROP_ADD_ZERO])) == sym("z")
        assert unwrap(replace_repeated(expr, [DROP_ADD_ZERO], 10)) == sym("z")

    def test_raises_on_depth_limit_error_instead_of_the_sentinel(self) -> None:
        f = sym("f")
        node = sym("leaf")
        for _ in range(MAX_TERM_DEPTH * 4):
            node = apply(f, [node])
        result = replace_all(node, [])
        assert isinstance(result, DepthLimitError)
        with pytest.raises(ValueError, match="depth-limit"):
            unwrap(result)

    def test_raises_on_rewrite_cycle_error_instead_of_the_sentinel(self) -> None:
        f = sym("f")
        never_converges = rule(apply(f, [X_PAT]), apply(f, [apply(f, [X_PAT])]))
        result = replace_repeated(apply(f, [sym("a")]), [never_converges], 50)
        assert isinstance(result, RewriteCycleError)
        with pytest.raises(ValueError, match="rewrite-cycle"):
            unwrap(result)

    def test_never_mistakes_an_ordinary_ir_node_for_an_error_sentinel(self) -> None:
        # int_(5) is a plain data object; unwrap must not misfire on it
        # just because it happens to be an object.
        assert unwrap(int_(5)) == int_(5)
        assert unwrap(sym("z")) == sym("z")


class TestSubstitute:
    def test_unbound_pattern_reference_in_template_left_as_is(self) -> None:
        # An unbound `named(...)` reference in a RHS template is left
        # unchanged rather than raising -- matches `apply_rule`'s own
        # forgiving convention (half-finished rules stay usable during
        # exploration).
        unbound = named("y", blank())
        assert substitute(unbound, Bindings()) == unbound

    def test_recurses_through_compound_structure(self) -> None:
        template = apply(sym("Add"), [X_PAT, sym("z")])
        result = substitute(template, Bindings().bind("x", int_(7)))
        assert result == apply(sym("Add"), [int_(7), sym("z")])

    def test_leaf_with_no_pattern_reference_passes_through_unchanged(self) -> None:
        assert substitute(sym("z"), Bindings()) == sym("z")


class TestMatchPattern:
    def test_returns_none_on_a_failed_match(self) -> None:
        assert match_pattern(sym("a"), sym("b")) is None

    def test_extends_an_existing_bindings_object(self) -> None:
        first = match_pattern(named("a", blank()), int_(1))
        assert first is not None
        second = match_pattern(named("b", blank()), int_(2), first)
        assert second is not None
        assert second["a"] == int_(1)
        assert second["b"] == int_(2)


# Sanity: the re-exported constants/type carry through the package's own
# top-level namespace, not just `runtime.py` internally.
def test_pattern_vocabulary_constants_are_reexported() -> None:
    from coding_adventures_sir_runtime_symbolic import BLANK, PATTERN, RULE, RULE_DELAYED

    assert isinstance(BLANK, IRSymbol)
    assert isinstance(PATTERN, IRSymbol)
    assert isinstance(RULE, IRSymbol)
    assert isinstance(RULE_DELAYED, IRSymbol)


def test_int_alias_matches_int_underscore() -> None:
    import coding_adventures_sir_runtime_symbolic as pkg

    assert pkg.int(3) == pkg.int_(3) == IRInteger(3)


def test_ir_apply_and_ir_node_types_reexported() -> None:
    from coding_adventures_sir_runtime_symbolic import IRApply as ReexportedIRApply
    from coding_adventures_sir_runtime_symbolic import IRNode as ReexportedIRNode

    assert ReexportedIRApply is IRApply
    assert isinstance(apply(sym("f"), []), ReexportedIRNode)
