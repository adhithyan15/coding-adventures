"""Adapters that translate parsed Prolog builtin calls into runtime goals."""

from __future__ import annotations

from collections.abc import Callable

from logic_builtins import (
    abolisho,
    all_differento,
    argo,
    assertao,
    assertzo,
    atomico,
    atomo,
    bagofo,
    betweeno,
    callableo,
    calltermo,
    clauseo,
    compare_termo,
    compoundo,
    copytermo,
    current_predicateo,
    cuto,
    dynamico,
    failo,
    fd_addo,
    fd_eqo,
    fd_geqo,
    fd_gto,
    fd_ino,
    fd_leqo,
    fd_lto,
    fd_mulo,
    fd_neqo,
    fd_subo,
    fd_sumo,
    findallo,
    forallo,
    functoro,
    geqo,
    groundo,
    gto,
    ifthenelseo,
    iftheno,
    integero,
    iso,
    labeling_optionso,
    labelingo,
    leqo,
    lto,
    nonvaro,
    noto,
    numbero,
    numeqo,
    numneqo,
    onceo,
    predicate_propertyo,
    retractallo,
    retracto,
    same_termo,
    setofo,
    stringo,
    succo,
    termo_geqo,
    termo_gto,
    termo_leqo,
    termo_lto,
    trueo,
    univo,
    varo,
)
from logic_engine import (
    Atom,
    Compound,
    ConjExpr,
    DisjExpr,
    FreshExpr,
    GoalExpr,
    LogicVar,
    RelationCall,
    Term,
    atom,
    conj,
    disj,
    eq,
    fresh,
    goal_from_term,
    relation,
    term,
)
from logic_stdlib import (
    appendo,
    lasto,
    lengtho,
    listo,
    membero,
    msorto,
    nth0_resto,
    nth0o,
    nth1_resto,
    nth1o,
    permuteo,
    reverseo,
    selecto,
    sorto,
)
from prolog_core import expand_dcg_phrase

_PREDICATE_INDICATOR = relation("/", 2)
_IF_THEN = "->"
type IndicatorBuilder = Callable[[Term, Term], GoalExpr]


def adapt_prolog_goal(goal: GoalExpr) -> GoalExpr:
    """Adapt parsed Prolog builtin calls into executable runtime goals.

    Goals with no builtin mapping are returned unchanged, so ordinary predicate
    calls still flow through to the loaded program as-is.
    """

    if isinstance(goal, RelationCall):
        return _adapt_relation_call(goal)
    if isinstance(goal, ConjExpr):
        return conj(*(adapt_prolog_goal(child) for child in goal.goals))
    if isinstance(goal, DisjExpr):
        if_then_else = _adapt_if_then_else(goal)
        if if_then_else is not None:
            return if_then_else
        return disj(*(adapt_prolog_goal(child) for child in goal.goals))
    if isinstance(goal, FreshExpr):
        return FreshExpr(
            template_vars=goal.template_vars,
            body=adapt_prolog_goal(goal.body),
        )
    return goal


def _adapt_relation_call(goal: RelationCall) -> GoalExpr:
    name = goal.relation.symbol.name
    args = goal.args

    nullary_builtins: dict[str, Callable[[], GoalExpr]] = {
        "true": trueo,
        "fail": failo,
        "!": cuto,
    }
    if goal.relation.arity == 0 and name in nullary_builtins:
        return nullary_builtins[name]()

    unary_term_builtins: dict[str, Callable[[object], GoalExpr]] = {
        "var": varo,
        "nonvar": nonvaro,
        "ground": groundo,
        "atom": atomo,
        "atomic": atomico,
        "integer": integero,
        "number": numbero,
        "string": stringo,
        "compound": compoundo,
        "callable": callableo,
    }
    if goal.relation.arity == 1 and name in unary_term_builtins:
        return unary_term_builtins[name](args[0])

    unary_list_builtins: dict[str, Callable[[object], GoalExpr]] = {
        "all_different": all_differento,
        "all_distinct": all_differento,
        "is_list": listo,
    }
    if goal.relation.arity == 1 and name in unary_list_builtins:
        return unary_list_builtins[name](args[0])

    binary_arithmetic_builtins: dict[str, Callable[[object, object], GoalExpr]] = {
        "is": iso,
        "succ": succo,
        "=:=": numeqo,
        "=\\=": numneqo,
        "<": lto,
        "=<": leqo,
        ">": gto,
        ">=": geqo,
    }
    if goal.relation.arity == 2 and name in binary_arithmetic_builtins:
        return binary_arithmetic_builtins[name](args[0], args[1])

    binary_fd_builtins: dict[str, Callable[[object, object], GoalExpr]] = {
        "#\\=": fd_neqo,
        "#<": fd_lto,
        "#=<": fd_leqo,
        "#>": fd_gto,
        "#>=": fd_geqo,
        "in": fd_ino,
    }
    if goal.relation.arity == 2 and name == "#=":
        return _adapt_fd_equality(args[0], args[1])
    if goal.relation.arity == 2 and name == "ins":
        ins_goal = _adapt_fd_ins(args[0], args[1])
        return goal if ins_goal is None else ins_goal
    if goal.relation.arity == 2 and name in binary_fd_builtins:
        return binary_fd_builtins[name](args[0], args[1])
    if goal.relation.arity == 3 and name == "sum":
        sum_goal = _adapt_fd_sum(args[0], args[1], args[2])
        return goal if sum_goal is None else sum_goal

    ternary_arithmetic_builtins: dict[
        str,
        Callable[[object, object, object], GoalExpr],
    ] = {
        "between": betweeno,
    }
    if goal.relation.arity == 3 and name in ternary_arithmetic_builtins:
        return ternary_arithmetic_builtins[name](*args)

    binary_list_builtins: dict[str, Callable[[object, object], GoalExpr]] = {
        "last": lasto,
        "length": lengtho,
        "member": membero,
        "msort": msorto,
        "permutation": permuteo,
        "reverse": reverseo,
        "sort": sorto,
    }
    if goal.relation.arity == 2 and name in binary_list_builtins:
        return binary_list_builtins[name](args[0], args[1])

    ternary_list_builtins: dict[str, Callable[[object, object, object], GoalExpr]] = {
        "append": appendo,
        "nth0": nth0o,
        "nth1": nth1o,
        "select": selecto,
    }
    if goal.relation.arity == 3 and name in ternary_list_builtins:
        return ternary_list_builtins[name](*args)

    quaternary_list_builtins: dict[
        str,
        Callable[[object, object, object, object], GoalExpr],
    ] = {
        "nth0": nth0_resto,
        "nth1": nth1_resto,
    }
    if goal.relation.arity == 4 and name in quaternary_list_builtins:
        return quaternary_list_builtins[name](*args)

    if name == "call" and goal.relation.arity == 1:
        return _adapt_callable_goal(args[0])
    if name == "phrase" and goal.relation.arity == 2:
        try:
            return calltermo(expand_dcg_phrase(args[0], args[1]))
        except TypeError:
            return goal
    if name == "phrase" and goal.relation.arity == 3:
        try:
            return calltermo(expand_dcg_phrase(args[0], args[1], args[2]))
        except TypeError:
            return goal
    if name == "once" and goal.relation.arity == 1:
        return onceo(_adapt_callable_goal(args[0]))
    if name == _IF_THEN and goal.relation.arity == 2:
        return iftheno(_adapt_callable_goal(args[0]), _adapt_callable_goal(args[1]))
    if name in {"not", "\\+"} and goal.relation.arity == 1:
        return noto(_adapt_callable_goal(args[0]))
    if name == "findall" and goal.relation.arity == 3:
        return findallo(args[0], _adapt_callable_goal(args[1]), args[2])
    if name == "bagof" and goal.relation.arity == 3:
        return bagofo(args[0], _adapt_callable_goal(args[1]), args[2])
    if name == "setof" and goal.relation.arity == 3:
        return setofo(args[0], _adapt_callable_goal(args[1]), args[2])
    if name == "forall" and goal.relation.arity == 2:
        return forallo(_adapt_callable_goal(args[0]), _adapt_callable_goal(args[1]))
    if name == "labeling" and goal.relation.arity == 2:
        return labeling_optionso(args[0], args[1])
    if name == "label" and goal.relation.arity == 1:
        return labelingo(args[0])
    if name == "functor" and goal.relation.arity == 3:
        return functoro(*args)
    if name == "arg" and goal.relation.arity == 3:
        return argo(*args)
    if name == "=.." and goal.relation.arity == 2:
        return univo(*args)
    if name == "copy_term" and goal.relation.arity == 2:
        return copytermo(*args)
    if name == "==" and goal.relation.arity == 2:
        return same_termo(*args)
    if name == "compare" and goal.relation.arity == 3:
        return compare_termo(*args)
    if name == "@<" and goal.relation.arity == 2:
        return termo_lto(*args)
    if name == "@=<" and goal.relation.arity == 2:
        return termo_leqo(*args)
    if name == "@>" and goal.relation.arity == 2:
        return termo_gto(*args)
    if name == "@>=" and goal.relation.arity == 2:
        return termo_geqo(*args)
    if name == "asserta" and goal.relation.arity == 1:
        return assertao(args[0])
    if name == "assertz" and goal.relation.arity == 1:
        return assertzo(args[0])
    if name == "retract" and goal.relation.arity == 1:
        return retracto(args[0])
    if name == "retractall" and goal.relation.arity == 1:
        return retractallo(args[0])
    if name == "clause" and goal.relation.arity == 2:
        return clauseo(*args)
    if name == "dynamic" and goal.relation.arity == 1:
        dynamic_goal = _adapt_indicator_declaration(args[0], dynamico)
        return goal if dynamic_goal is None else dynamic_goal
    if name == "abolish" and goal.relation.arity == 1:
        abolish_goal = _adapt_indicator_declaration(args[0], abolisho)
        return goal if abolish_goal is None else abolish_goal
    if name == "current_predicate" and goal.relation.arity == 1:
        current_goal = _adapt_current_predicate(args[0])
        return goal if current_goal is None else current_goal
    if name == "predicate_property" and goal.relation.arity == 2:
        property_goal = _adapt_predicate_property(args[0], args[1])
        return goal if property_goal is None else property_goal

    return goal


def _adapt_fd_equality(left: Term, right: Term) -> GoalExpr:
    right_expression = _adapt_fd_arithmetic_expression(right, left)
    if right_expression is not None:
        return right_expression

    left_expression = _adapt_fd_arithmetic_expression(left, right)
    if left_expression is not None:
        return left_expression

    return fd_eqo(left, right)


def _adapt_fd_sum(
    terms_value: Term,
    operator_value: Term,
    result: Term,
) -> GoalExpr | None:
    if _atom_symbol_name(operator_value) != "#=":
        return None
    return fd_sumo(terms_value, result)


def _adapt_fd_arithmetic_expression(expression: Term, result: Term) -> GoalExpr | None:
    if not isinstance(expression, Compound) or len(expression.args) != 2:
        return None
    if expression.functor.namespace is not None:
        return None

    sum_terms = _fd_sum_terms(expression)
    if sum_terms is not None and len(sum_terms) > 2:
        return fd_sumo(sum_terms, result)

    left, right = expression.args
    if expression.functor.name == "+":
        return fd_addo(left, right, result)
    if expression.functor.name == "-":
        return fd_subo(left, right, result)
    if expression.functor.name == "*":
        return fd_mulo(left, right, result)
    return None


def _fd_sum_terms(expression: Term) -> tuple[Term, ...] | None:
    if not isinstance(expression, Compound) or len(expression.args) != 2:
        return (expression,)
    if expression.functor.namespace is not None:
        return None

    left, right = expression.args
    if expression.functor.name == "+":
        left_terms = _fd_sum_terms(left)
        right_terms = _fd_sum_terms(right)
        if left_terms is None or right_terms is None:
            return None
        return (*left_terms, *right_terms)

    if expression.functor.name == "-":
        left_terms = _fd_sum_terms(left)
        if left_terms is None:
            return None
        if isinstance(right, int):
            return (*left_terms, -right)

    return None


def _adapt_fd_ins(targets: Term, domain: Term) -> GoalExpr | None:
    items = _logic_list_items(targets)
    if items is None:
        return None
    return conj(*(fd_ino(item, domain) for item in items))


def _adapt_if_then_else(goal: DisjExpr) -> GoalExpr | None:
    if len(goal.goals) != 2:
        return None
    condition_then, else_goal = goal.goals
    if (
        not isinstance(condition_then, RelationCall)
        or condition_then.relation.symbol.name != _IF_THEN
        or condition_then.relation.arity != 2
    ):
        return None
    condition, then_goal = condition_then.args
    return ifthenelseo(
        _adapt_callable_goal(condition),
        _adapt_callable_goal(then_goal),
        adapt_prolog_goal(else_goal),
    )


def _adapt_callable_goal(term_value: Term) -> GoalExpr:
    try:
        return adapt_prolog_goal(goal_from_term(term_value))
    except TypeError:
        return calltermo(term_value)


def _adapt_indicator_declaration(
    indicator_term: Term,
    builder: IndicatorBuilder,
) -> GoalExpr | None:
    if isinstance(indicator_term, LogicVar):
        return None

    indicator = _indicator_parts(indicator_term)
    if indicator is not None:
        return builder(*indicator)

    items = _logic_list_items(indicator_term)
    if items is None:
        return None

    goals: list[GoalExpr] = []
    for item in items:
        parts = _indicator_parts(item)
        if parts is None:
            return None
        goals.append(builder(*parts))
    return conj(*goals)


def _adapt_current_predicate(indicator_term: Term) -> GoalExpr | None:
    parts = _indicator_parts(indicator_term)
    if parts is not None:
        return current_predicateo(*parts)
    if isinstance(indicator_term, LogicVar):
        return fresh(
            2,
            lambda name_term, arity_term: conj(
                current_predicateo(name_term, arity_term),
                eq(indicator_term, term("/", name_term, arity_term)),
            ),
        )
    return None


def _adapt_predicate_property(
    indicator_or_head: Term,
    property_term: Term,
) -> GoalExpr | None:
    parts = _indicator_parts(indicator_or_head)
    if parts is not None:
        return predicate_propertyo(parts[0], parts[1], property_term)

    if isinstance(indicator_or_head, LogicVar):
        return fresh(
            2,
            lambda name_term, arity_term: conj(
                predicate_propertyo(name_term, arity_term, property_term),
                eq(indicator_or_head, term("/", name_term, arity_term)),
            ),
        )

    if isinstance(indicator_or_head, Atom):
        return predicate_propertyo(atom(indicator_or_head.symbol), 0, property_term)
    if isinstance(indicator_or_head, Compound):
        return predicate_propertyo(
            atom(indicator_or_head.functor),
            len(indicator_or_head.args),
            property_term,
        )
    return None


def _indicator_parts(term_value: Term) -> tuple[Term, Term] | None:
    if (
        isinstance(term_value, Compound)
        and term_value.functor == _PREDICATE_INDICATOR.symbol
        and len(term_value.args) == 2
    ):
        return (term_value.args[0], term_value.args[1])
    return None


def _logic_list_items(term_value: Term) -> list[Term] | None:
    items: list[Term] = []
    current = term_value
    while True:
        if isinstance(current, Atom) and current.symbol.name == "[]":
            return items
        if (
            isinstance(current, Compound)
            and current.functor.name == "."
            and len(current.args) == 2
        ):
            items.append(current.args[0])
            current = current.args[1]
            continue
        return None


def _atom_symbol_name(term_value: Term) -> str | None:
    if isinstance(term_value, Atom) and term_value.symbol.namespace is None:
        return term_value.symbol.name
    if isinstance(term_value, str):
        return term_value
    return None
