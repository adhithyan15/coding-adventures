"""Adapters that translate parsed Prolog builtin calls into runtime goals."""

from __future__ import annotations

from collections.abc import Callable

from logic_builtins import (
    abolisho,
    argo,
    assertao,
    assertzo,
    atomico,
    atomo,
    callableo,
    calltermo,
    clauseo,
    compare_termo,
    compoundo,
    current_predicateo,
    dynamico,
    functoro,
    groundo,
    nonvaro,
    noto,
    numbero,
    onceo,
    predicate_propertyo,
    retractallo,
    retracto,
    same_termo,
    stringo,
    termo_geqo,
    termo_gto,
    termo_leqo,
    termo_lto,
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
    relation,
    term,
)
from prolog_core import expand_dcg_phrase

_PREDICATE_INDICATOR = relation("/", 2)
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

    unary_term_builtins: dict[str, Callable[[object], GoalExpr]] = {
        "var": varo,
        "nonvar": nonvaro,
        "ground": groundo,
        "atom": atomo,
        "atomic": atomico,
        "number": numbero,
        "string": stringo,
        "compound": compoundo,
        "callable": callableo,
    }
    if goal.relation.arity == 1 and name in unary_term_builtins:
        return unary_term_builtins[name](args[0])

    if name == "call" and goal.relation.arity == 1:
        return calltermo(args[0])
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
        return onceo(calltermo(args[0]))
    if name in {"not", "\\+"} and goal.relation.arity == 1:
        return noto(calltermo(args[0]))
    if name == "functor" and goal.relation.arity == 3:
        return functoro(*args)
    if name == "arg" and goal.relation.arity == 3:
        return argo(*args)
    if name == "=.." and goal.relation.arity == 2:
        return univo(*args)
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
