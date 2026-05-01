"""Adapters that translate parsed Prolog builtin calls into runtime goals."""

from __future__ import annotations

from collections.abc import Callable

from logic_builtins import (
    abolisho,
    all_differento,
    argo,
    assertao,
    assertzo,
    atom_charso,
    atom_codeso,
    atomico,
    atomo,
    bagofo,
    betweeno,
    callableo,
    calltermo,
    catcho,
    char_codeo,
    clauseo,
    compare_termo,
    compoundo,
    convlisto,
    copytermo,
    current_predicateo,
    current_prolog_flago,
    cuto,
    difo,
    dynamico,
    excludeo,
    failo,
    fd_addo,
    fd_bool_ando,
    fd_bool_equivo,
    fd_bool_implieso,
    fd_bool_noto,
    fd_bool_oro,
    fd_elemento,
    fd_eqo,
    fd_geqo,
    fd_gto,
    fd_ino,
    fd_leqo,
    fd_lto,
    fd_mulo,
    fd_neqo,
    fd_reify_relationo,
    fd_scalar_product_relationo,
    fd_subo,
    fd_sum_relationo,
    fd_sumo,
    findallo,
    foldlo,
    forallo,
    functoro,
    groundo,
    ifthenelseo,
    iftheno,
    includeo,
    integero,
    labeling_optionso,
    labelingo,
    maplisto,
    nonvaro,
    not_same_termo,
    not_variant_termo,
    noto,
    number_charso,
    number_codeso,
    numbero,
    onceo,
    partitiono,
    predicate_propertyo,
    prolog_geqo,
    prolog_gto,
    prolog_iso,
    prolog_leqo,
    prolog_lto,
    prolog_numeqo,
    prolog_numneqo,
    retractallo,
    retracto,
    same_termo,
    scanlo,
    set_prolog_flago,
    setofo,
    string_charso,
    string_codeso,
    stringo,
    subsumes_termo,
    succo,
    term_variableso,
    termo_geqo,
    termo_gto,
    termo_leqo,
    termo_lto,
    throwo,
    trueo,
    univo,
    variant_termo,
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
    NeqExpr,
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
_FD_REIFIABLE_RELATIONS = frozenset({"#=", "#\\=", "#<", "#=<", "#>", "#>="})
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
    if isinstance(goal, NeqExpr):
        # Prolog \=/2 is immediate non-unifiability, unlike the engine's
        # delayed disequality constraint.
        return noto(eq(goal.left, goal.right))
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
        "is": prolog_iso,
        "succ": succo,
        "=:=": prolog_numeqo,
        "=\\=": prolog_numneqo,
        "<": prolog_lto,
        "=<": prolog_leqo,
        ">": prolog_gto,
        ">=": prolog_geqo,
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
    if name in {"#<==>", "#==>", "#/\\", "#\\/"} and goal.relation.arity == 2:
        boolean_goal = _adapt_fd_boolean_goal(name, args)
        return goal if boolean_goal is None else boolean_goal
    if name == "#\\" and goal.relation.arity == 1:
        return _adapt_fd_boolean_expr(args[0], 0)
    if goal.relation.arity == 3 and name == "sum":
        sum_goal = _adapt_fd_sum(args[0], args[1], args[2])
        return goal if sum_goal is None else sum_goal
    if goal.relation.arity == 3 and name == "element":
        return fd_elemento(args[0], args[1], args[2])
    if goal.relation.arity == 4 and name == "scalar_product":
        scalar_goal = _adapt_fd_scalar_product(args[0], args[1], args[2], args[3])
        return goal if scalar_goal is None else scalar_goal

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

    if name == "maplist" and 2 <= goal.relation.arity <= 5:
        return maplisto(args[0], *args[1:])
    if name == "convlist" and goal.relation.arity == 3:
        return convlisto(*args)
    if name == "include" and goal.relation.arity == 3:
        return includeo(*args)
    if name == "exclude" and goal.relation.arity == 3:
        return excludeo(*args)
    if name == "partition" and goal.relation.arity == 4:
        return partitiono(*args)
    if name == "foldl" and 4 <= goal.relation.arity <= 7:
        return foldlo(*args)
    if name == "scanl" and 4 <= goal.relation.arity <= 7:
        return scanlo(*args)

    if name == "call" and 1 <= goal.relation.arity <= 8:
        if goal.relation.arity > 1:
            extended_call = _extend_callable_term(args[0], args[1:])
            if extended_call is None:
                return calltermo(args[0], *args[1:])
            return _adapt_callable_goal(extended_call)
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
    if name == "throw" and goal.relation.arity == 1:
        return throwo(args[0])
    if name == "catch" and goal.relation.arity == 3:
        return catcho(
            _adapt_callable_goal(args[0]),
            args[1],
            _adapt_callable_goal(args[2]),
        )
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
    if name == "term_variables" and goal.relation.arity == 2:
        return term_variableso(*args)
    if name == "atom_chars" and goal.relation.arity == 2:
        return atom_charso(*args)
    if name == "atom_codes" and goal.relation.arity == 2:
        return atom_codeso(*args)
    if name == "number_chars" and goal.relation.arity == 2:
        return number_charso(*args)
    if name == "number_codes" and goal.relation.arity == 2:
        return number_codeso(*args)
    if name == "char_code" and goal.relation.arity == 2:
        return char_codeo(*args)
    if name == "string_chars" and goal.relation.arity == 2:
        return string_charso(*args)
    if name == "string_codes" and goal.relation.arity == 2:
        return string_codeso(*args)
    if name == "current_prolog_flag" and goal.relation.arity == 2:
        return current_prolog_flago(*args)
    if name == "set_prolog_flag" and goal.relation.arity == 2:
        return set_prolog_flago(*args)
    if name == "=" and goal.relation.arity == 2:
        return eq(*args)
    if name == "\\=" and goal.relation.arity == 2:
        return noto(eq(*args))
    if name == "dif" and goal.relation.arity == 2:
        return difo(*args)
    if name == "==" and goal.relation.arity == 2:
        return same_termo(*args)
    if name == "\\==" and goal.relation.arity == 2:
        return not_same_termo(*args)
    if name == "=@=" and goal.relation.arity == 2:
        return variant_termo(*args)
    if name == "\\=@=" and goal.relation.arity == 2:
        return not_variant_termo(*args)
    if name == "subsumes_term" and goal.relation.arity == 2:
        return subsumes_termo(*args)
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
    if _fd_relation_name(operator_value) is None:
        return None
    return fd_sum_relationo(terms_value, operator_value, result)


def _adapt_fd_scalar_product(
    coeffs_value: Term,
    terms_value: Term,
    operator_value: Term,
    result: Term,
) -> GoalExpr | None:
    if _fd_relation_name(operator_value) is None:
        return None
    return fd_scalar_product_relationo(
        coeffs_value,
        terms_value,
        operator_value,
        result,
    )


def _adapt_fd_boolean_goal(name: str, args: tuple[Term, ...]) -> GoalExpr | None:
    if name == "#<==>":
        left, right = args
        left_expression = _is_fd_boolean_expression(left)
        right_expression = _is_fd_boolean_expression(right)
        if left_expression and not right_expression:
            return _adapt_fd_boolean_expr(left, right)
        if right_expression and not left_expression:
            return _adapt_fd_boolean_expr(right, left)
        return fresh(
            2,
            lambda left_truth, right_truth: conj(
                _adapt_fd_boolean_expr(left, left_truth),
                _adapt_fd_boolean_expr(right, right_truth),
                fd_bool_equivo(left_truth, right_truth, 1),
            ),
        )

    left, right = args
    connective = {
        "#/\\": fd_bool_ando,
        "#\\/": fd_bool_oro,
        "#==>": fd_bool_implieso,
    }.get(name)
    if connective is None:
        return None
    return fresh(
        2,
        lambda left_truth, right_truth: conj(
            _adapt_fd_boolean_expr(left, left_truth),
            _adapt_fd_boolean_expr(right, right_truth),
            connective(left_truth, right_truth, 1),
        ),
    )


def _adapt_fd_boolean_expr(expression: Term, truth: Term) -> GoalExpr:
    parts = _callable_term_parts(expression)
    if parts is None:
        return fd_bool_equivo(expression, truth, 1)

    name, args = parts
    if name in _FD_REIFIABLE_RELATIONS and len(args) == 2:
        return fd_reify_relationo(args[0], name, args[1], truth)
    if name == "#\\" and len(args) == 1:
        return fresh(
            1,
            lambda child_truth: conj(
                _adapt_fd_boolean_expr(args[0], child_truth),
                fd_bool_noto(child_truth, truth),
            ),
        )

    connective = {
        "#/\\": fd_bool_ando,
        "#\\/": fd_bool_oro,
        "#==>": fd_bool_implieso,
        "#<==>": fd_bool_equivo,
    }.get(name)
    if connective is not None and len(args) == 2:
        return fresh(
            2,
            lambda left_truth, right_truth: conj(
                _adapt_fd_boolean_expr(args[0], left_truth),
                _adapt_fd_boolean_expr(args[1], right_truth),
                connective(left_truth, right_truth, truth),
            ),
        )

    return fd_bool_equivo(expression, truth, 1)


def _is_fd_boolean_expression(term_value: Term) -> bool:
    parts = _callable_term_parts(term_value)
    if parts is None:
        return False
    name, args = parts
    if name in _FD_REIFIABLE_RELATIONS and len(args) == 2:
        return True
    if name == "#\\" and len(args) == 1:
        return True
    return name in {"#/\\", "#\\/", "#==>", "#<==>"} and len(args) == 2


def _callable_term_parts(term_value: Term) -> tuple[str, tuple[Term, ...]] | None:
    if isinstance(term_value, RelationCall):
        symbol = term_value.relation.symbol
        if symbol.namespace is None:
            return (symbol.name, term_value.args)
        return None
    if isinstance(term_value, Compound) and term_value.functor.namespace is None:
        return (term_value.functor.name, term_value.args)
    return None


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


def _extend_callable_term(
    callable_term: Term,
    extra_args: tuple[Term, ...],
) -> Term | None:
    if isinstance(callable_term, RelationCall):
        callable_term = callable_term.as_term()
    if isinstance(callable_term, LogicVar):
        return None
    if isinstance(callable_term, Atom) and callable_term.symbol.namespace is None:
        return term(callable_term.symbol, *extra_args)
    if isinstance(callable_term, Compound):
        if (
            callable_term.functor.namespace is None
            and callable_term.functor.name == ":"
            and len(callable_term.args) == 2
        ):
            qualified_goal = _extend_callable_term(callable_term.args[1], extra_args)
            if qualified_goal is None:
                return None
            return term(":", callable_term.args[0], qualified_goal)
        return term(callable_term.functor, *callable_term.args, *extra_args)
    return None


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


def _fd_relation_name(term_value: Term) -> str | None:
    name = _atom_symbol_name(term_value)
    if name in {"#=", "#\\=", "#<", "#=<", "#>", "#>="}:
        return name
    return None
