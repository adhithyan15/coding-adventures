"""The ``ir.tuple`` level (VER04r section 4.1, Appendix A.1, slice S10e), beyond the kit.

``ir.tuple`` is the tuple types ``(T1,..,Tn)``, the literal, ``project``, structural
``eq``/``ne``, stream tag ``64``, and **``match`` on a ``Result``**.  The kit pins 78
``v2-ir`` trees, 44 ``v2-domain`` and 15 ``v2-chunk`` cases; these tests pin the rules
behind them: the typing of every node, the six patterns and the usefulness check that
refuses a fall-through or an unreachable arm, the type-directed *level* of ``match``
(R76), the tuple descriptor's enumeration order, and the fault relation that lets a
fault change a tuple's component (R94, R96 to R98).
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.v2 import STANDARD, Program, Semantics
from coding_adventures_warrant_corpus.v2 import patterns as pat
from coding_adventures_warrant_corpus.v2.domains import build, descriptor_for
from coding_adventures_warrant_corpus.v2.errors import IrOverflow, Refusal
from coding_adventures_warrant_corpus.v2.faults import apply_fault, compile_fault
from coding_adventures_warrant_corpus.v2.ir import (
    IMPLEMENTED_LEVELS,
    check_declared_levels,
    evaluate,
)
from coding_adventures_warrant_corpus.v2.kit import outcome_ir
from coding_adventures_warrant_corpus.v2.values import (
    decode_value,
    encode_value,
    value_to_json,
)
from coding_adventures_warrant_corpus.v2.wtypes import (
    NAT,
    Ty,
    fault_unify,
    parse_type,
)

CORE = frozenset({"ir.core"})
TUPLE = frozenset({"ir.core", "ir.tuple"})
FIXED = frozenset({"ir.core", "ir.fixed"})
ALL = frozenset({"ir.core", "ir.fixed", "ir.tuple"})

ERRORS = {
    "E": {"kind": "error", "cases": ["BAD", "WORSE"]},
    "F": {"kind": "error", "cases": ["ONE", "TWO", "THREE"]},
}


def lit(type_name, value):
    return ["lit", {"type": type_name, "value": str(value)}]


def var(name):
    return ["var", name]


def pair(a, b):
    return ["tuple", [a, b]]


def scope_of(**names):
    return {name: parse_type(text, ERRORS) for name, text in names.items()}


def run(ir, env=None, scope=None, levels=TUPLE):
    program = Program(types=ERRORS, levels=levels)
    compiled = program.compile(ir, scope or {})
    return str(compiled.ty), evaluate(compiled, env or {})


def code(ir, scope=None, levels=TUPLE):
    with pytest.raises(Refusal) as caught:
        Program(types=ERRORS, levels=levels).compile(ir, scope or {})
    return caught.value.code


# ---------------------------------------------------------------------------
# The level lattice a lane declares (section 4.1, R1)
# ---------------------------------------------------------------------------
def test_this_lane_declares_ir_core_ir_fixed_and_ir_tuple():
    assert {"ir.core", "ir.fixed", "ir.tuple"} == IMPLEMENTED_LEVELS


@pytest.mark.parametrize(
    "levels",
    [
        set(),
        {"ir.core"},
        {"ir.core", "ir.fixed"},
        {"ir.core", "ir.tuple"},  # siblings: neither needs the other
        {"ir.core", "ir.fixed", "ir.tuple", "ir.data"},
        {"ir.core", "ir.tuple", "ir.data"},  # ir.data does not imply ir.fixed
        {"ir.core", "ir.tuple", "ir.data", "ir.list", "ir.seq"},
    ],
)
def test_a_declaration_that_respects_the_lattice_is_accepted(levels):
    assert check_declared_levels(levels) == frozenset(levels)


@pytest.mark.parametrize(
    "levels",
    [
        {"ir.fixed"},  # needs ir.core
        {"ir.tuple"},
        {"ir.core", "ir.data"},  # ir.data implies ir.tuple
        {"ir.core", "ir.fixed", "ir.data"},
        {"ir.core", "ir.tuple", "ir.list"},  # ir.list implies ir.data
        {"ir.core", "ir.nonsense"},
        {"ir.core", "ir.inout.records"},
    ],
)
def test_a_declaration_that_breaks_the_lattice_is_refused(levels):
    with pytest.raises(Refusal):
        check_declared_levels(levels)


# ---------------------------------------------------------------------------
# The literal and project
# ---------------------------------------------------------------------------
def test_a_tuple_literal_of_every_arity_from_two_to_eight():
    for arity in range(2, 9):
        parts = [lit("U8", i) for i in range(arity)]
        type_text, value = run(["tuple", parts])
        assert type_text == "(" + ",".join(["U8"] * arity) + ")"
        assert value == tuple(range(arity))


@pytest.mark.parametrize("arity", [0, 1, 9, 12])
def test_a_tuple_literal_of_another_arity_is_ill_typed(arity):
    assert code(["tuple", [lit("U8", 1)] * arity]) == "ill-typed"


@pytest.mark.parametrize(
    "ir",
    [
        ["tuple"],
        ["tuple", 3],
        ["tuple", [lit("U8", 1), lit("U8", 2)], 3],
        ["project", pair(lit("U8", 1), lit("U8", 2))],
        ["project", pair(lit("U8", 1), lit("U8", 2)), "0"],
        ["project", pair(lit("U8", 1), lit("U8", 2)), True],  # a bool is not an index
        ["project", pair(lit("U8", 1), lit("U8", 2)), -1],
        ["project", pair(lit("U8", 1), lit("U8", 2)), 2],
        ["project", lit("U8", 1), 0],
    ],
)
def test_a_malformed_tuple_or_project_is_ill_typed(ir):
    assert code(ir) == "ill-typed"


def test_the_component_types_are_the_operands_static_types_and_nothing_lifts():
    # Nat arithmetic is an Int, a bytes literal keeps its width: R51, nothing lifts
    type_text, value = run(
        pair(["add", lit("Nat", 1), lit("Nat", 2)], lit("Bytes", "0a0b"))
    )
    assert (type_text, value) == ("(Int,Bytes<2>)", (3, b"\n\x0b"))


def test_project_reads_a_component_and_keeps_its_declared_type():
    scope = scope_of(t="((U8,U8),Bool)")
    env = {"t": ((1, 2), True)}
    assert run(["project", var("t"), 0], env, scope) == ("(U8,U8)", (1, 2))
    assert run(["project", ["project", var("t"), 0], 1], env, scope) == ("U8", 2)
    assert run(["project", var("t"), 1], env, scope) == ("Bool", True)


def test_a_tuple_is_equal_component_wise_and_never_ordered():
    scope = scope_of(a="(U8,Bool)", b="(U8,Bool)", c="(U8,Nat)")
    env = {"a": (1, True), "b": (1, True), "c": (1, 5)}
    assert run(["eq", var("a"), var("b")], env, scope)[1] is True
    assert (
        run(["ne", var("a"), pair(lit("U8", 1), ["lit", False])], env, scope)[1] is True
    )
    # the types differ: invariant, so ill-typed even though components widen
    assert code(["eq", var("a"), var("c")], scope) == "ill-typed"
    assert code(["lt", var("a"), var("b")], scope) == "ill-typed"
    nested = pair(pair(lit("Nat", 1), lit("Nat", 2)), lit("Nat", 3))
    wide = pair(pair(lit("Int", 1), lit("Int", 2)), lit("Int", 3))
    assert (
        code(["eq", nested, wide]) == "ill-typed"
    )  # a tuple of Nat is not a tuple of Int


def test_an_if_selects_a_tuple_and_two_widths_unify_to_plain_bytes_only_outside_a_tuple():
    ir = [
        "if",
        ["lit", True],
        pair(lit("U8", 1), lit("U8", 2)),
        pair(lit("U8", 3), lit("U8", 4)),
    ]
    assert run(ir) == ("(U8,U8)", (1, 2))
    mixed = [
        "if",
        ["lit", True],
        pair(lit("Bytes", "00"), lit("U8", 1)),
        pair(lit("Bytes", "0000"), lit("U8", 1)),
    ]
    assert code(mixed) == "ill-typed"  # nothing lifts under a tuple


def test_a_runner_without_ir_tuple_refuses_the_tags_and_the_type_as_level_missing():
    for ir in (
        pair(lit("U8", 1), lit("U8", 2)),
        ["project", var("t"), 0],
        ["match", var("r"), [[["wild"], lit("U8", 0)]]],
    ):
        assert (
            code(ir, scope_of(t="(U8,U8)", r="Result<U8,E>"), levels=CORE)
            == "level-missing"
        )
    # a tuple type reached through a variable is a missing level with no tuple tag in the tree
    assert code(["lit", True], scope_of(t="(U8,U8)"), levels=CORE) == "level-missing"
    # ir.fixed does not give ir.tuple: the levels are siblings
    assert code(pair(lit("U8", 1), lit("U8", 2)), levels=FIXED) == "level-missing"
    # and the other way round
    assert code(["bit_not", lit("U8", 1)], levels=TUPLE) == "level-missing"


# ---------------------------------------------------------------------------
# match on a Result: the six patterns
# ---------------------------------------------------------------------------
R = scope_of(r="Result<U8,E>")


def arms(*pairs):
    return [list(p) for p in pairs]


def test_every_pattern_kind_matches_what_it_says():
    ir = [
        "match",
        var("r"),
        arms(
            (["err", "BAD"], lit("Nat", 1)),
            (["err", "WORSE"], lit("Nat", 2)),
            (["ok", ["wild"]], lit("Nat", 3)),
        ),
    ]
    for value, want in ((("err", "BAD"), 1), (("err", "WORSE"), 2), (("ok", 9), 3)):
        assert run(ir, {"r": value}, R) == ("Nat", want)
    any_ = [
        "match",
        var("r"),
        arms(
            (["ok", ["bind", "x"]], ["to_nat", var("x")]), (["err_any"], lit("Nat", 0))
        ),
    ]
    assert run(any_, {"r": ("ok", 9)}, R, levels=ALL) == ("Nat", 9)
    assert run(any_, {"r": ("err", "BAD")}, R, levels=ALL) == ("Nat", 0)
    whole = [
        "match",
        var("r"),
        arms(
            (["ok", ["wild"]], ["lit", True]),
            (["bind", "w"], ["eq", var("w"), var("r")]),
        ),
    ]
    assert run(whole, {"r": ("err", "BAD")}, R) == ("Bool", True)


def test_a_tuple_pattern_binds_components_and_nests():
    scope = scope_of(r="Result<((U8,U8),U8),E>")
    ir = [
        "match",
        var("r"),
        arms(
            (
                [
                    "ok",
                    ["tuple", [["tuple", [["bind", "a"], ["wild"]]], ["bind", "c"]]],
                ],
                ["add", var("a"), var("c")],
            ),
            (["err_any"], lit("U8", 0)),
        ),
    ]
    assert run(ir, {"r": ("ok", ((1, 2), 3))}, scope) == ("U8", 4)


def test_a_result_inside_a_result_is_matched_by_nested_ok_and_err_patterns():
    scope = scope_of(r="Result<Result<U8,E>,E>")
    ir = [
        "match",
        var("r"),
        arms(
            (["ok", ["ok", ["wild"]]], lit("Nat", 0)),
            (["ok", ["err", "BAD"]], lit("Nat", 1)),
            (["ok", ["err", "WORSE"]], lit("Nat", 2)),
            (["err_any"], lit("Nat", 3)),
        ),
    ]
    for value, want in (
        (("ok", ("ok", 1)), 0),
        (("ok", ("err", "BAD")), 1),
        (("ok", ("err", "WORSE")), 2),
        (("err", "BAD"), 3),
    ):
        assert run(ir, {"r": value}, scope) == ("Nat", want)


def test_the_first_matching_arm_runs_and_only_that_arm():
    ir = [
        "match",
        var("r"),
        arms(
            (["err", "BAD"], lit("Nat", 1)),
            (["err_any"], lit("Nat", 2)),
            (["ok", ["wild"]], lit("Nat", 0)),
        ),
    ]
    assert run(ir, {"r": ("err", "BAD")}, R)[1] == 1

    class LastWins(Semantics):
        def choose_arm(self, matching):  # type: ignore[no-untyped-def]
            return matching[-1]

    program = Program(types=ERRORS, levels=TUPLE, sem=LastWins())
    compiled = program.compile(ir, R)
    assert evaluate(compiled, {"r": ("err", "BAD")}) == 2  # the seam is the rule


def test_the_arms_share_a_type_and_a_mismatch_is_ill_typed():
    ir = [
        "match",
        var("r"),
        arms((["ok", ["wild"]], lit("U8", 1)), (["err_any"], ["lit", True])),
    ]
    assert code(ir, R) == "ill-typed"
    widening = [
        "match",
        var("r"),
        arms((["ok", ["wild"]], lit("Nat", 1)), (["err_any"], lit("Int", -1))),
    ]
    assert run(widening, {"r": ("ok", 1)}, R)[0] == "Int"


# -- patterns that are refused -------------------------------------------------------------
@pytest.mark.parametrize(
    ("why", "ir"),
    [
        ("no arms", ["match", var("r"), []]),
        ("arms not a list", ["match", var("r"), 5]),
        ("an arm that is not a pair", ["match", var("r"), [[["wild"]]]]),
        ("a pattern that is not a list", ["match", var("r"), [[7, lit("U8", 1)]]]),
        ("a pattern with no tag", ["match", var("r"), [[[], lit("U8", 1)]]]),
        ("an unknown pattern", ["match", var("r"), [[["wibble"], lit("U8", 1)]]]),
        ("wild with an operand", ["match", var("r"), [[["wild", 1], lit("U8", 1)]]]),
        ("bind without a name", ["match", var("r"), [[["bind"], lit("U8", 1)]]]),
        ("bind with a bad name", ["match", var("r"), [[["bind", "1x"], lit("U8", 1)]]]),
        (
            "err of an undeclared case",
            [
                "match",
                var("r"),
                [[["err", "GHOST"], lit("U8", 1)], [["bind", "z"], lit("U8", 2)]],
            ],
        ),
        (
            "err with no case",
            [
                "match",
                var("r"),
                [[["err"], lit("U8", 1)], [["bind", "z"], lit("U8", 2)]],
            ],
        ),
        (
            "err_any with an operand",
            [
                "match",
                var("r"),
                [[["err_any", 1], lit("U8", 1)], [["bind", "z"], lit("U8", 2)]],
            ],
        ),
        (
            "ok with two operands",
            [
                "match",
                var("r"),
                [
                    [["ok", ["wild"], ["wild"]], lit("U8", 1)],
                    [["bind", "z"], lit("U8", 2)],
                ],
            ],
        ),
        (
            "a tuple pattern on a scalar payload",
            [
                "match",
                var("r"),
                [
                    [["ok", ["tuple", [["wild"], ["wild"]]]], lit("U8", 1)],
                    [["bind", "z"], lit("U8", 2)],
                ],
            ],
        ),
        (
            "a tuple pattern that is not a list",
            [
                "match",
                var("r"),
                [[["ok", ["tuple", 1]], lit("U8", 1)], [["bind", "z"], lit("U8", 2)]],
            ],
        ),
        (
            "an ok pattern on a scalar",
            [
                "match",
                var("r"),
                [
                    [["ok", ["ok", ["wild"]]], lit("U8", 1)],
                    [["bind", "z"], lit("U8", 2)],
                ],
            ],
        ),
        (
            "the same bind twice in one pattern",
            [
                "match",
                var("p"),
                [
                    [["ok", ["tuple", [["bind", "a"], ["bind", "a"]]]], lit("U8", 1)],
                    [["err_any"], lit("U8", 2)],
                ],
            ],
        ),
        (
            "a bind that shadows a variable",
            [
                "match",
                var("r"),
                [[["ok", ["bind", "r"]], lit("U8", 1)], [["err_any"], lit("U8", 2)]],
            ],
        ),
        (
            "a bind that shadows an outer bind",
            [
                "match",
                var("r"),
                [
                    [
                        ["ok", ["bind", "x"]],
                        [
                            "match",
                            var("r"),
                            [
                                [["ok", ["bind", "x"]], lit("U8", 1)],
                                [["err_any"], lit("U8", 2)],
                            ],
                        ],
                    ],
                    [["err_any"], lit("U8", 3)],
                ],
            ],
        ),
        (
            "a match that falls through (no ok)",
            ["match", var("r"), [[["err_any"], lit("U8", 1)]]],
        ),
        (
            "a match that falls through (one case missing)",
            [
                "match",
                var("r"),
                [[["err", "BAD"], lit("U8", 1)], [["ok", ["wild"]], lit("U8", 2)]],
            ],
        ),
        (
            "an arm after a bind",
            [
                "match",
                var("r"),
                [[["bind", "x"], lit("U8", 1)], [["ok", ["wild"]], lit("U8", 2)]],
            ],
        ),
        (
            "an arm that repeats an earlier one",
            [
                "match",
                var("r"),
                [
                    [["err", "BAD"], lit("U8", 1)],
                    [["err", "BAD"], lit("U8", 2)],
                    [["bind", "z"], lit("U8", 3)],
                ],
            ],
        ),
        (
            "err_any after every case",
            [
                "match",
                var("r"),
                [
                    [["err", "BAD"], lit("U8", 1)],
                    [["err", "WORSE"], lit("U8", 1)],
                    [["err_any"], lit("U8", 2)],
                    [["ok", ["wild"]], lit("U8", 3)],
                ],
            ],
        ),
    ],
)
def test_a_bad_pattern_or_arm_is_refused(why, ir):
    scope = {**R, **scope_of(p="Result<(U8,U8),E>")}
    assert code(ir, scope) == "ill-typed", why


ONE = lit("U8", 1)
TWO = lit("U8", 2)
# an error type that is undetermined: a bare ok, or a case two error types declare
TWO_TYPES = {
    "E": {"kind": "error", "cases": ["BAD", "WORSE"]},
    "F": {"kind": "error", "cases": ["BAD", "ONE"]},
}


def code_two(ir, scope=None):
    with pytest.raises(Refusal) as caught:
        Program(types=TWO_TYPES, levels=TUPLE).compile(ir, scope or {})
    return caught.value.code


def value_two(ir, env=None, scope=None):
    compiled = Program(types=TWO_TYPES, levels=TUPLE).compile(ir, scope or {})
    return evaluate(compiled, env or {})


def test_an_ok_only_match_on_an_undetermined_error_type_is_not_exhaustive():
    # P2/P7: this used to be accepted and then crash with a KitError at evaluation
    joined = ["if", var("c"), ["ok", ONE], ["err", "BAD"]]
    scope = {"c": parse_type("Bool", {})}
    ir = ["match", joined, [[["ok", ["bind", "x"]], var("x")]]]
    assert code_two(ir, scope) == "ill-typed"
    assert (
        code(["match", ["ok", ONE], [[["ok", ["bind", "x"]], var("x")]]]) == "ill-typed"
    )


@pytest.mark.parametrize(
    "arms",
    [
        [[["ok", ["bind", "x"]], ONE], [["err_any"], TWO]],
        [[["ok", ["wild"]], ONE], [["wild"], TWO]],
        [[["ok", ["wild"]], ONE], [["bind", "w"], TWO]],
        [[["ok", ["wild"]], ONE], [["err", "BAD"], TWO], [["wild"], ONE]],
        [[["err", "BAD"], ONE], [["wild"], TWO]],
        [[["ok", ["wild"]], ONE], [["err", "BAD"], TWO], [["err_any"], ONE]],
    ],
)
def test_forms_that_cover_an_undetermined_error_type_are_accepted(arms):
    scope = {"c": parse_type("Bool", {})}
    for scrutinee in (
        ["if", var("c"), ["ok", ONE], ["err", "BAD"]],
        ["ok", ONE],
        ["err", "BAD"],  # BAD belongs to two error types
    ):
        ir = ["match", scrutinee, arms]
        for c in (True, False):
            if scrutinee[0] == "if" or c:
                value_two(ir, {"c": c}, scope)


def test_an_undetermined_error_type_still_refuses_a_case_nobody_declares():
    ir = ["match", ["ok", ONE], [[["err", "GHOST"], ONE], [["wild"], TWO]]]
    assert code_two(ir) == "ill-typed"


def test_a_bind_named_result_is_refused_in_any_body():
    ir = [
        "match",
        var("r"),
        arms((["ok", ["bind", "result"]], ONE), (["err_any"], TWO)),
    ]
    assert code(ir, R) == "ill-typed"


def test_a_match_of_more_than_256_arms_is_refused():
    many = [[["err", "BAD"], ONE]] * (pat.MAX_ARMS + 1)
    assert pat.MAX_ARMS == 256 and code(["match", var("r"), many], R) == "ill-typed"


def nested_ok(levels):
    node = ["wild"]
    for _ in range(levels - 1):
        node = ["ok", node]
    return node


def test_a_pattern_nests_at_most_eight_levels():
    scope = scope_of(r="Result<" * 9 + "U8,E>" + ",E>" * 8)
    for levels, want in ((8, "ok"), (9, "depth")):
        ir = ["match", var("r"), [[nested_ok(levels), ONE], [["wild"], TWO]]]
        if want == "ok":
            Program(types=ERRORS, levels=TUPLE).compile(ir, scope)
        else:
            assert code(ir, scope) == "depth"
    assert pat.pattern_depth(nested_ok(8)) == 8 and pat.pattern_depth("x") == 0


def test_an_intricate_match_is_refused_within_its_budget_not_ground_through():
    # 18 Result columns, each with an ok row and an err_any row over wildcards: the
    # exhaustiveness check branches three ways per column (3^18 without a budget)
    import time

    columns = 18
    column = Ty("Result", (Ty("U8"), Ty("E")))
    ty = Ty("()", (column,) * columns)
    rows = []
    for i in range(columns):
        for kind in (pat.Pattern("ok", (pat.WILD,)), pat.Pattern("err_any")):
            parts = [pat.WILD] * columns
            parts[i] = kind
            rows.append([pat.Pattern("tuple", tuple(parts))])
    start = time.monotonic()
    with pytest.raises(Refusal) as caught:
        pat.useful(rows, [pat.WILD], [ty], {"E": ("A", "B")})
    assert caught.value.code == "ill-typed" and "intricate" in caught.value.message
    assert time.monotonic() - start < 1.0


def test_check_arms_shares_one_budget_across_the_whole_match(monkeypatch):
    monkeypatch.setattr(pat, "MAX_STEPS", 5)
    ir = [
        "match",
        var("r"),
        arms((["err", "BAD"], ONE), (["err", "WORSE"], TWO), (["ok", ["wild"]], ONE)),
    ]
    assert code(ir, R) == "ill-typed"


def test_an_error_type_of_three_cases_needs_all_three():
    scope = scope_of(r="Result<U8,F>")
    full = [
        "match",
        var("r"),
        arms(
            (["err", "ONE"], lit("U8", 1)),
            (["err", "TWO"], lit("U8", 2)),
            (["err", "THREE"], lit("U8", 3)),
            (["ok", ["wild"]], lit("U8", 0)),
        ),
    ]
    assert run(full, {"r": ("err", "THREE")}, scope)[1] == 3
    short = ["match", var("r"), full[2][:2] + [full[2][3]]]  # THREE is missing
    assert code(short, scope) == "ill-typed"
    # err_any covers the two that are missing
    covered = ["match", var("r"), [full[2][0], [["err_any"], lit("U8", 9)], full[2][3]]]
    assert run(covered, {"r": ("err", "TWO")}, scope)[1] == 9


# -- the level of match is type-directed (R76) ---------------------------------------------------
@pytest.mark.parametrize(
    "ir",
    [
        ["match", var("t"), [[["tuple", [["bind", "a"], ["bind", "b"]]], var("a")]]],
        ["match", var("b"), [[["bind", "x"], lit("U8", 1)]]],
        ["match", pair(lit("U8", 1), lit("U8", 2)), [[["wild"], lit("U8", 1)]]],
        [
            "match",
            var("r"),
            [[["some", ["wild"]], lit("U8", 1)], [["bind", "x"], lit("U8", 2)]],
        ],
        ["match", var("r"), [[["none"], lit("U8", 1)], [["bind", "x"], lit("U8", 2)]]],
        [
            "match",
            var("r"),
            [[["ctor", "C", []], lit("U8", 1)], [["bind", "x"], lit("U8", 2)]],
        ],
        [
            "match",
            var("r"),
            [[["bool", True], lit("U8", 1)], [["bind", "x"], lit("U8", 2)]],
        ],
        # a data pattern hidden under ok and under a tuple pattern
        [
            "match",
            var("r"),
            [[["ok", ["some", ["wild"]]], lit("U8", 1)], [["bind", "x"], lit("U8", 2)]],
        ],
        [
            "match",
            var("p"),
            [
                [["ok", ["tuple", [["wild"], ["none"]]]], lit("U8", 1)],
                [["bind", "x"], lit("U8", 2)],
            ],
        ],
    ],
)
def test_a_match_that_is_not_on_a_result_with_result_patterns_is_ir_data(ir):
    scope = scope_of(t="(U8,U8)", b="Bool", r="Result<U8,E>", p="Result<(U8,U8),E>")
    assert code(ir, scope) == "level-missing"


def test_holds_data_pattern_reads_every_depth_without_recursing():
    deep = ["wild"]
    for _ in range(5000):  # a hostile pattern nested far past the recursion limit
        deep = ["ok", deep]
    assert pat.holds_data_pattern(deep) is False
    deep = ["some", ["wild"]]
    for _ in range(5000):
        deep = ["tuple", [deep, ["wild"]]]
    assert pat.holds_data_pattern(deep) is True
    assert pat.holds_data_pattern("not a pattern") is False


# -- calls, cost, overflow -------------------------------------------------------------------------
BUMP = {"bump": {"params": [["a", "U8"]], "requires": ["ir.core"], "ret": "U8"}}
INC = {"inc": {"params": [["a", "U8"]], "body": ["add", var("a"), lit("U8", 1)]}}


def match_case(ir, r_value, **fields):
    base = {
        "ir": ir,
        "env": [["r", {"type": "Result<U8,E>", "value": r_value}]],
        "levels": ["ir.core", "ir.tuple"],
        "types": {"E": ERRORS["E"]},
        "ops": BUMP,
        "models": INC,
        "model_order": ["inc"],
        "subjects": {"bump": "inc"},
    }
    return base | fields


def test_only_the_taken_arm_is_counted_and_the_trace_agrees():
    ir = [
        "match",
        var("r"),
        arms(
            (["ok", ["bind", "x"]], var("x")),
            (["err_any"], ["subject_op", "bump", [lit("U8", 5)]]),
        ),
    ]
    assert outcome_ir(match_case(ir, {"ok": "7"}), STANDARD) == {
        "type": "U8",
        "value": "7",
        "calls": 0,
    }
    assert outcome_ir(match_case(ir, {"err": "BAD"}), STANDARD) == {
        "type": "U8",
        "value": "6",
        "calls": 1,
    }


def test_calls_depend_on_a_binding_the_pure_scrutinee_made():
    # the arm's own guard reads the bound variable: the dynamic count sees it
    arm = [
        "if",
        ["eq", var("x"), lit("U8", 7)],
        ["subject_op", "bump", [var("x")]],
        var("x"),
    ]
    ir = [
        "match",
        var("r"),
        arms((["ok", ["bind", "x"]], arm), (["err_any"], lit("U8", 0))),
    ]
    assert outcome_ir(match_case(ir, {"ok": "7"}), STANDARD)["calls"] == 1
    assert outcome_ir(match_case(ir, {"ok": "8"}), STANDARD)["calls"] == 0


def test_a_subject_call_in_the_scrutinee_is_counted_and_may_not_guard_a_call():
    res = {
        "res": {
            "params": [["r", "Result<U8,E>"]],
            "requires": ["ir.core"],
            "ret": "Result<U8,E>",
        }
    }
    idr = {"idr": {"params": [["r", "Result<U8,E>"]], "body": var("r")}}
    ir = [
        "match",
        ["subject_op", "res", [var("r")]],
        arms((["ok", ["bind", "x"]], var("x")), (["err_any"], lit("U8", 0))),
    ]
    case = match_case(
        ir,
        {"ok": "7"},
        ops=res,
        models=idr,
        model_order=["idr"],
        subjects={"res": "idr"},
    )
    assert outcome_ir(case, STANDARD) == {"type": "U8", "value": "7", "calls": 1}
    guarded = [
        "match",
        ["subject_op", "res", [var("r")]],
        arms(
            (["ok", ["bind", "x"]], ["subject_op", "bump", [var("x")]]),
            (["err_any"], lit("U8", 0)),
        ),
    ]
    case = match_case(
        guarded,
        {"ok": "7"},
        ops=res | BUMP,
        models=idr | INC,
        model_order=["idr", "inc"],
        subjects={"res": "idr", "bump": "inc"},
    )
    assert outcome_ir(case, STANDARD) == {"error": "impure-guard"}


def test_the_cost_of_a_match_is_the_scrutinee_plus_the_larger_arm():
    ir = [
        "match",
        var("r"),
        arms(
            (["ok", ["bind", "x"]], ["add", var("x"), ["add", var("x"), var("x")]]),
            (["err_any"], lit("U8", 0)),
        ),
    ]
    compiled = Program(types=ERRORS, levels=TUPLE).compile(ir, R)
    assert (
        compiled.cost == 1 + 1 + 5
    )  # match + var r + the larger arm (add, x, add, x, x)


def test_an_overflow_in_an_arm_that_is_not_taken_is_not_an_error():
    huge = ["mul", lit("Nat", 2**64 - 1), lit("Nat", 2**64 - 1)]
    ir = [
        "match",
        var("r"),
        arms(
            (["ok", ["wild"]], lit("Int", 0)),
            (["err_any"], ["add", huge, lit("Int", 0)]),
        ),
    ]
    assert run(ir, {"r": ("ok", 1)}, R) == ("Int", 0)
    program = Program(types=ERRORS, levels=TUPLE)
    compiled = program.compile(ir, R)
    with pytest.raises(IrOverflow) as caught:
        evaluate(compiled, {"r": ("err", "BAD")})
    assert caught.value.klass == "corpus"


def test_a_match_with_too_many_arms_is_refused_not_ground_through():
    many = [[["err", "BAD"], lit("U8", 1)]] * (pat.MAX_ARMS + 1)
    assert code(["match", var("r"), many], R) == "ill-typed"


# -- the usefulness check on its own -----------------------------------------------------------------
def test_usefulness_over_a_pair_of_results():
    ty = parse_type("(Result<U8,E>,Result<U8,E>)", ERRORS)
    ok, bad, worse = (
        pat.Pattern("ok", (pat.WILD,)),
        pat.Pattern("err", name="BAD"),
        pat.Pattern("err", name="WORSE"),
    )

    def tup(a, b):
        return pat.Pattern("tuple", (a, b))

    errors = {"E": ("BAD", "WORSE")}
    full = [
        [tup(ok, pat.WILD)],
        [tup(bad, pat.WILD)],
        [tup(worse, ok)],
        [tup(worse, bad)],
    ]
    assert pat.useful(full, [pat.WILD], [ty], errors)  # (WORSE, WORSE) is missing
    full.append([tup(worse, worse)])
    assert not pat.useful(full, [pat.WILD], [ty], errors)
    # an arm is unreachable when the rows above it cover it
    assert not pat.useful([[tup(ok, pat.WILD)]], [tup(ok, bad)], [ty], errors)
    assert pat.useful([[tup(ok, bad)]], [tup(ok, pat.WILD)], [ty], errors)
    # err_any as a query is every err case: useful iff one of them is
    one = pat.Pattern("err_any")
    assert pat.useful([[bad]], [one], [parse_type("Result<U8,E>", ERRORS)], errors)
    assert not pat.useful(
        [[bad], [worse]], [one], [parse_type("Result<U8,E>", ERRORS)], errors
    )


# ---------------------------------------------------------------------------
# Values: JSON, stream tag 64
# ---------------------------------------------------------------------------
def test_a_tuple_value_decodes_encodes_and_reports_by_declared_type():
    ty = parse_type("((U8,Bool),I8)", {})
    raw = [["1", True], "-1"]
    value = decode_value(raw, ty, {})
    assert value == ((1, True), -1)
    assert value_to_json(value, ty, {}) == raw
    stream = encode_value(value, ty, {})
    head = b"\x64" + (2).to_bytes(4, "little")
    inner = (
        b"\x64"
        + (2).to_bytes(4, "little")
        + b"\x10"
        + (1).to_bytes(8, "little")
        + b"\x01"
    )
    assert stream == head + inner + b"\x11" + b"\xff" * 8
    for bad in (
        [["1", True]],
        [["1", True], "-1", "0"],
        {"a": 1},
        [["1", 1], "-1"],
        "x",
    ):
        with pytest.raises(Refusal) as caught:
            decode_value(bad, ty, {})
        assert caught.value.code == "value"


# ---------------------------------------------------------------------------
# Domains: the tuple descriptor is a product, last component fastest
# ---------------------------------------------------------------------------
def test_a_tuple_descriptor_enumerates_last_component_fastest():
    node = {
        "d": "tuple",
        "of": [{"d": "bool", "size": 2}, {"d": "nat_count", "k": 2, "size": 3}],
        "size": 6,
    }
    desc = build(node)
    assert [desc.nth(i) for i in range(6)] == [
        (False, 0),
        (False, 1),
        (False, 2),
        (True, 0),
        (True, 1),
        (True, 2),
    ]

    class FirstFastest(Semantics):
        def component_digits(self, index, sizes):  # type: ignore[no-untyped-def]
            digits = []
            for size in sizes:
                index, digit = divmod(index, size)
                digits.append(digit)
            return digits

    reordered = build(node, FirstFastest())
    assert reordered.nth(1) == (True, 0)  # the seam is the rule


def test_a_tuple_descriptor_is_checked_against_its_type():
    node = {
        "d": "tuple",
        "of": [{"d": "bool", "size": 2}, {"d": "u8_all", "size": 256}],
        "size": 512,
    }
    desc, ty = descriptor_for(node, "(Bool,U8)")
    assert str(ty) == "(Bool,U8)" and desc.size == 512
    for wrong in ("(U8,Bool)", "(Bool,U8,U8)", "Bool"):
        with pytest.raises(Refusal) as caught:
            descriptor_for(node, wrong)
        assert caught.value.code == "descriptor"
    for size in (511, 513):
        with pytest.raises(Refusal):
            build(node | {"size": size})
    for parts in ([], [node["of"][0]], [node["of"][0]] * 9):
        with pytest.raises(Refusal):
            build({"d": "tuple", "of": parts, "size": 1})


def test_a_nat_shift_binder_is_a_nat_leaf():
    desc, ty = descriptor_for({"d": "nat_shift", "size": 73}, "Nat")
    assert ty == NAT and desc.size == 73
    assert desc.nth(72) == 65536 and desc.nth(64) == 64 and desc.nth(65) == 65


# ---------------------------------------------------------------------------
# Faults over tuples: R94, R96 to R98, R67
# ---------------------------------------------------------------------------
def op_row(ret, params=(("a", "U8"),)):
    return {"params": [list(p) for p in params], "ret": ret, "requires": ["ir.core"]}


def post(expr, requires=("ir.core", "ir.tuple")):
    return {
        "fault": "f",
        "op": "op",
        "kind": "post",
        "expr": expr,
        "requires": list(requires),
    }


def apply(row, op, result, args=("5",), levels=ALL, types=None):
    return apply_fault(
        row, op, types if types is not None else ERRORS, levels, list(args), result
    )


def test_fault_unify_is_r51_plus_tuples_with_a_narrow_relation_below_them():
    b1, b2, bytes_ = Ty("Bytes", (1,)), Ty("Bytes", (2,)), Ty("Bytes")
    assert fault_unify(b1, b2) == bytes_  # at the top two widths unify to plain Bytes
    pair_of = lambda x, y: Ty("()", (x, y))  # noqa: E731
    assert fault_unify(pair_of(b1, NAT), pair_of(b1, Ty("Int"))) == pair_of(
        b1, Ty("Int")
    )
    assert fault_unify(pair_of(b1, NAT), pair_of(b2, NAT)) is None  # narrow: no lifting
    assert fault_unify(Ty("Option", (b1,)), Ty("Option", (b2,))) == Ty(
        "Option", (bytes_,)
    )
    assert fault_unify(Ty("Option", (b1,)), Ty("Option", (b2,)), narrow=True) is None
    nested = pair_of(pair_of(NAT, NAT), NAT)
    assert fault_unify(nested, pair_of(pair_of(Ty("Int"), NAT), NAT)) is not None
    assert fault_unify(pair_of(NAT, NAT), Ty("()", (NAT, NAT, NAT))) is None
    assert (
        fault_unify(Ty("Result", (NAT, Ty("E"))), Ty("Result", (Ty("?"), Ty("E"))))
        is not None
    )
    assert (
        fault_unify(Ty("Result", (NAT, Ty("E"))), Ty("Result", (NAT, Ty("F")))) is None
    )


def test_a_fault_may_widen_one_component_of_a_tuple_result():
    # off-by-one on the second component: (Bytes,Nat) -> (Bytes,Int) has a common type (R96)
    second = ["project", var("result"), 1]
    expr = ["tuple", [["project", var("result"), 0], ["add", second, lit("Nat", 1)]]]
    out = apply(post(expr), op_row("(Bytes,Nat)"), ["0a", "4"])
    assert out == {"result": ["0a", "5"]}


def test_a_bytes_literal_where_plain_bytes_is_expected_is_a_plain_bytes():
    # R67: the zero literal of a plain Bytes component; a one-byte literal in a Bytes<2> slot is refused
    ok = [
        "if",
        ["is_err", var("result")],
        ["ok", pair(["lit", {"type": "Bytes", "value": ""}], ["lit", False])],
        var("result"),
    ]
    assert apply(post(ok), op_row("Result<(Bytes,Bool),E>"), {"err": "BAD"}) == {
        "result": {"ok": ["", False]}
    }
    wrong = ["tuple", [["lit", {"type": "Bytes", "value": "01"}], ["lit", False]]]
    with pytest.raises(Refusal) as caught:
        compile_fault(post(wrong), op_row("(Bytes<2>,Bool)"), ERRORS, ALL)
    assert caught.value.code == "ill-typed"


def test_every_branch_of_a_fault_if_or_match_is_compared_with_the_declared_type():
    bad_if = ["if", ["is_err", var("result")], var("result"), ["ok", ["lit", True]]]
    with pytest.raises(Refusal) as caught:
        compile_fault(post(bad_if, ("ir.core",)), op_row("Result<U8,E>"), ERRORS, ALL)
    assert caught.value.code == "ill-typed"
    bad_arm = [
        "match",
        var("result"),
        [
            [["ok", ["bind", "p"]], ["ok", var("p")]],
            [["err_any"], ["ok", ["lit", True]]],
        ],
    ]
    with pytest.raises(Refusal) as caught:
        compile_fault(post(bad_arm), op_row("Result<U8,E>"), ERRORS, ALL)
    assert caught.value.code == "ill-typed"
    in_component = [
        "tuple",
        [["if", ["lit", True], var("x"), ["ok", ["lit", True]]], ["lit", True]],
    ]
    with pytest.raises(Refusal) as caught:
        compile_fault(
            post(in_component),
            op_row("(Result<U8,E>,Bool)", (("x", "Result<U8,E>"),)),
            ERRORS,
            ALL,
        )
    assert caught.value.code == "ill-typed"


def test_a_pre_rewrite_may_build_a_tuple_parameter():
    row = {
        "fault": "f",
        "op": "op",
        "kind": "pre",
        "requires": ["ir.core", "ir.tuple"],
        "rewrites": [
            {
                "param": "p",
                "expr": pair(["project", var("p"), 1], ["project", var("p"), 0]),
            }
        ],
    }
    out = apply(row, op_row("U8", (("p", "(U8,U8)"),)), "0", args=[["1", "2"]])
    assert out == {"args": [["2", "1"]]}


def test_a_fault_row_that_needs_a_level_the_runner_lacks_is_level_missing():
    for levels in (CORE, FIXED):
        with pytest.raises(Refusal) as caught:
            compile_fault(
                post(pair(var("result"), var("result"))), op_row("U8"), ERRORS, levels
            )
        assert caught.value.code == "level-missing"


def test_a_fault_that_reads_a_missing_bind_is_ill_typed():
    expr = [
        "match",
        var("result"),
        [[["ok", ["bind", "result"]], var("result")], [["err_any"], ["lit", False]]],
    ]
    with pytest.raises(Refusal) as caught:
        compile_fault(post(expr), op_row("Result<Bool,E>"), ERRORS, ALL)
    assert caught.value.code == "ill-typed"
