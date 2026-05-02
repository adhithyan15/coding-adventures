"""Adapters that translate parsed Prolog builtin calls into runtime goals."""

from __future__ import annotations

import re
from collections.abc import Callable, Iterator

from logic_builtins import (
    abolisho,
    acyclic_termo,
    all_differento,
    argo,
    assertao,
    assertzo,
    atom_charso,
    atom_codeso,
    atom_concato,
    atom_lengtho,
    atomic_list_concato,
    atomic_list_concato_with_separator,
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
    compound_name_argumentso,
    compound_name_arityo,
    compoundo,
    convlisto,
    copytermo,
    current_atomo,
    current_functoro,
    current_predicateo,
    current_prolog_flago,
    cuto,
    cyclic_termo,
    difo,
    dynamico,
    excludeo,
    failo,
    falseo,
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
    ignoreo,
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
    number_stringo,
    numbero,
    numbervarso,
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
    repeato,
    retractallo,
    retracto,
    same_termo,
    scanlo,
    set_prolog_flago,
    setofo,
    string_charso,
    string_codeso,
    string_lengtho,
    stringo,
    sub_atomo,
    sub_stringo,
    subsumes_termo,
    succo,
    term_hash_boundedo,
    term_hasho,
    term_variableso,
    termo_geqo,
    termo_gto,
    termo_leqo,
    termo_lto,
    throwo,
    trueo,
    unifiableo,
    unify_with_occurs_checko,
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
    Number,
    Program,
    RelationCall,
    State,
    String,
    Term,
    atom,
    conj,
    disj,
    eq,
    fresh,
    goal_from_term,
    logic_list,
    native_goal,
    num,
    reify,
    relation,
    solve_from,
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
from prolog_core import OperatorTable, expand_dcg_phrase
from prolog_parser import PrologParseError
from swi_prolog_parser import parse_swi_term

_PREDICATE_INDICATOR = relation("/", 2)
_IF_THEN = "->"
_FD_REIFIABLE_RELATIONS = frozenset({"#=", "#\\=", "#<", "#=<", "#>", "#>="})
_PLAIN_ATOM_RE = re.compile(r"^[a-z][A-Za-z0-9_]*$")
_VARIABLE_RE = re.compile(r"^(?:[A-Z_][A-Za-z0-9_]*)$")
type IndicatorBuilder = Callable[[Term, Term], GoalExpr]


def adapt_prolog_goal(
    goal: GoalExpr,
    *,
    operator_table: OperatorTable | None = None,
) -> GoalExpr:
    """Adapt parsed Prolog builtin calls into executable runtime goals.

    Goals with no builtin mapping are returned unchanged, so ordinary predicate
    calls still flow through to the loaded program as-is.
    """

    if isinstance(goal, RelationCall):
        return _adapt_relation_call(goal, operator_table=operator_table)
    if isinstance(goal, ConjExpr):
        return conj(
            *(
                adapt_prolog_goal(child, operator_table=operator_table)
                for child in goal.goals
            ),
        )
    if isinstance(goal, DisjExpr):
        if_then_else = _adapt_if_then_else(goal, operator_table=operator_table)
        if if_then_else is not None:
            return if_then_else
        return disj(
            *(
                adapt_prolog_goal(child, operator_table=operator_table)
                for child in goal.goals
            ),
        )
    if isinstance(goal, NeqExpr):
        # Prolog \=/2 is immediate non-unifiability, unlike the engine's
        # delayed disequality constraint.
        return noto(eq(goal.left, goal.right))
    if isinstance(goal, FreshExpr):
        return FreshExpr(
            template_vars=goal.template_vars,
            body=adapt_prolog_goal(goal.body, operator_table=operator_table),
        )
    return goal


def _adapt_relation_call(
    goal: RelationCall,
    *,
    operator_table: OperatorTable | None,
) -> GoalExpr:
    name = goal.relation.symbol.name
    args = goal.args

    nullary_builtins: dict[str, Callable[[], GoalExpr]] = {
        "true": trueo,
        "fail": failo,
        "false": falseo,
        "!": cuto,
    }
    if goal.relation.arity == 0 and name in nullary_builtins:
        return nullary_builtins[name]()

    unary_term_builtins: dict[str, Callable[[object], GoalExpr]] = {
        "var": varo,
        "nonvar": nonvaro,
        "ground": groundo,
        "acyclic_term": acyclic_termo,
        "cyclic_term": cyclic_termo,
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
            return _adapt_callable_goal(extended_call, operator_table=operator_table)
        return _adapt_callable_goal(args[0], operator_table=operator_table)
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
        return onceo(_adapt_callable_goal(args[0], operator_table=operator_table))
    if name == "repeat" and goal.relation.arity == 0:
        return repeato()
    if name == "ignore" and goal.relation.arity == 1:
        return ignoreo(_adapt_callable_goal(args[0], operator_table=operator_table))
    if name == "throw" and goal.relation.arity == 1:
        return throwo(args[0])
    if name == "catch" and goal.relation.arity == 3:
        return catcho(
            _adapt_callable_goal(args[0], operator_table=operator_table),
            args[1],
            _adapt_callable_goal(args[2], operator_table=operator_table),
        )
    if name == _IF_THEN and goal.relation.arity == 2:
        return iftheno(
            _adapt_callable_goal(args[0], operator_table=operator_table),
            _adapt_callable_goal(args[1], operator_table=operator_table),
        )
    if name in {"not", "\\+"} and goal.relation.arity == 1:
        return noto(_adapt_callable_goal(args[0], operator_table=operator_table))
    if name == "findall" and goal.relation.arity == 3:
        return findallo(
            args[0],
            _adapt_collection_goal(args[1], operator_table=operator_table),
            args[2],
            scope=args[1],
        )
    if name == "bagof" and goal.relation.arity == 3:
        return bagofo(
            args[0],
            _adapt_collection_goal(args[1], operator_table=operator_table),
            args[2],
            scope=args[1],
        )
    if name == "setof" and goal.relation.arity == 3:
        return setofo(
            args[0],
            _adapt_collection_goal(args[1], operator_table=operator_table),
            args[2],
            scope=args[1],
        )
    if name == "forall" and goal.relation.arity == 2:
        return forallo(
            _adapt_callable_goal(args[0], operator_table=operator_table),
            _adapt_callable_goal(args[1], operator_table=operator_table),
        )
    if name == "current_op" and goal.relation.arity == 3:
        if operator_table is None:
            return goal
        return _current_op_goal(operator_table, *args)
    if name == "labeling" and goal.relation.arity == 2:
        return labeling_optionso(args[0], args[1])
    if name == "label" and goal.relation.arity == 1:
        return labelingo(args[0])
    if name == "functor" and goal.relation.arity == 3:
        return functoro(*args)
    if name == "compound_name_arguments" and goal.relation.arity == 3:
        return compound_name_argumentso(*args)
    if name == "compound_name_arity" and goal.relation.arity == 3:
        return compound_name_arityo(*args)
    if name == "arg" and goal.relation.arity == 3:
        return argo(*args)
    if name == "=.." and goal.relation.arity == 2:
        return univo(*args)
    if name == "unifiable" and goal.relation.arity == 3:
        return unifiableo(*args)
    if name == "unify_with_occurs_check" and goal.relation.arity == 2:
        return unify_with_occurs_checko(*args)
    if name == "copy_term" and goal.relation.arity == 2:
        return copytermo(*args)
    if name == "term_variables" and goal.relation.arity == 2:
        return term_variableso(*args)
    if name == "numbervars" and goal.relation.arity == 3:
        return numbervarso(*args)
    if name == "term_hash" and goal.relation.arity == 2:
        return term_hasho(*args)
    if name == "term_hash" and goal.relation.arity == 4:
        return term_hash_boundedo(*args)
    if name == "term_to_atom" and goal.relation.arity == 2:
        return _term_to_atom_goal(*args)
    if name == "atom_to_term" and goal.relation.arity == 3:
        return _atom_to_term_goal(*args)
    if name == "read_term_from_atom" and goal.relation.arity == 3:
        return _read_term_from_atom_goal(*args)
    if name == "write_term_to_atom" and goal.relation.arity == 3:
        return _write_term_to_atom_goal(*args)
    if name == "atom_concat" and goal.relation.arity == 3:
        return atom_concato(*args)
    if name == "atom_length" and goal.relation.arity == 2:
        return atom_lengtho(*args)
    if name == "atomic_list_concat" and goal.relation.arity == 2:
        return atomic_list_concato(*args)
    if name == "atomic_list_concat" and goal.relation.arity == 3:
        return atomic_list_concato_with_separator(*args)
    if name == "atom_chars" and goal.relation.arity == 2:
        return atom_charso(*args)
    if name == "atom_codes" and goal.relation.arity == 2:
        return atom_codeso(*args)
    if name == "number_chars" and goal.relation.arity == 2:
        return number_charso(*args)
    if name == "number_codes" and goal.relation.arity == 2:
        return number_codeso(*args)
    if name == "number_string" and goal.relation.arity == 2:
        return number_stringo(*args)
    if name == "char_code" and goal.relation.arity == 2:
        return char_codeo(*args)
    if name == "string_chars" and goal.relation.arity == 2:
        return string_charso(*args)
    if name == "string_codes" and goal.relation.arity == 2:
        return string_codeso(*args)
    if name == "string_length" and goal.relation.arity == 2:
        return string_lengtho(*args)
    if name == "sub_atom" and goal.relation.arity == 5:
        return sub_atomo(*args)
    if name == "sub_string" and goal.relation.arity == 5:
        return sub_stringo(*args)
    if name == "current_atom" and goal.relation.arity == 1:
        return current_atomo(*args)
    if name == "current_functor" and goal.relation.arity == 2:
        return current_functoro(*args)
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


def _current_op_goal(
    operator_table: OperatorTable,
    precedence: object,
    associativity: object,
    name: object,
) -> GoalExpr:
    def run(
        program_value: Program,
        state: State,
        args: tuple[Term, ...],
    ) -> Iterator[State]:
        precedence_target, associativity_target, name_target = args
        for spec in operator_table.operators:
            yield from solve_from(
                program_value,
                conj(
                    eq(precedence_target, num(spec.precedence)),
                    eq(associativity_target, atom(spec.associativity)),
                    eq(name_target, atom(spec.symbol)),
                ),
                state,
            )

    return native_goal(run, precedence, associativity, name)


def _term_to_atom_goal(term_value: object, atom_value: object) -> GoalExpr:
    def run(
        program_value: Program,
        state: State,
        args: tuple[Term, ...],
    ) -> Iterator[State]:
        term_arg, atom_arg = args
        reified_term = reify(term_arg, state.substitution)
        reified_atom = reify(atom_arg, state.substitution)

        if not isinstance(reified_term, LogicVar):
            rendered = _render_prolog_term(reified_term)
            yield from solve_from(program_value, eq(atom_arg, atom(rendered)), state)
            return

        if not isinstance(reified_atom, Atom):
            return
        parsed = _parse_atom_text(reified_atom)
        if parsed is None:
            return
        yield from solve_from(program_value, eq(term_arg, parsed.term), state)

    return native_goal(run, term_value, atom_value)


def _atom_to_term_goal(
    atom_value: object,
    term_value: object,
    bindings: object,
) -> GoalExpr:
    def run(
        program_value: Program,
        state: State,
        args: tuple[Term, ...],
    ) -> Iterator[State]:
        atom_arg, term_arg, bindings_arg = args
        reified_atom = reify(atom_arg, state.substitution)
        if not isinstance(reified_atom, Atom):
            return

        parsed = _parse_atom_text(reified_atom)
        if parsed is None:
            return
        yield from solve_from(
            program_value,
            conj(
                eq(term_arg, parsed.term),
                eq(bindings_arg, _variable_bindings(parsed.variables)),
            ),
            state,
        )

    return native_goal(run, atom_value, term_value, bindings)


def _read_term_from_atom_goal(
    atom_value: object,
    term_value: object,
    options: object,
) -> GoalExpr:
    def run(
        program_value: Program,
        state: State,
        args: tuple[Term, ...],
    ) -> Iterator[State]:
        atom_arg, term_arg, options_arg = args
        reified_atom = reify(atom_arg, state.substitution)
        if not isinstance(reified_atom, Atom):
            return

        parsed = _parse_atom_text(reified_atom)
        if parsed is None:
            return
        options_goal = _read_term_options_goal(options_arg, parsed.variables)
        if options_goal is None:
            return
        yield from solve_from(
            program_value,
            conj(eq(term_arg, parsed.term), options_goal),
            state,
        )

    return native_goal(run, atom_value, term_value, options)


def _write_term_to_atom_goal(
    term_value: object,
    atom_value: object,
    options: object,
) -> GoalExpr:
    def run(
        program_value: Program,
        state: State,
        args: tuple[Term, ...],
    ) -> Iterator[State]:
        term_arg, atom_arg, options_arg = args
        reified_term = reify(term_arg, state.substitution)
        if isinstance(reified_term, LogicVar):
            return
        write_options = _write_term_options(reify(options_arg, state.substitution))
        if write_options is None:
            return
        rendered = _render_prolog_term(
            reified_term,
            numbervars=write_options["numbervars"],
        )
        yield from solve_from(program_value, eq(atom_arg, atom(rendered)), state)

    return native_goal(run, term_value, atom_value, options)


def _parse_atom_text(atom_value: Atom) -> object | None:
    try:
        return parse_swi_term(atom_value.symbol.name)
    except PrologParseError:
        return None


def _variable_bindings(variables: dict[str, LogicVar]) -> Term:
    return logic_list(
        [term("=", atom(name), variable) for name, variable in variables.items()],
    )


def _variable_values(variables: dict[str, LogicVar]) -> Term:
    return logic_list(list(variables.values()))


def _read_term_options_goal(
    options: Term,
    variables: dict[str, LogicVar],
) -> GoalExpr | None:
    items = _logic_list_items(options)
    if items is None:
        return None

    goals: list[GoalExpr] = []
    for item in items:
        if not isinstance(item, Compound) or len(item.args) != 1:
            return None
        if item.functor.name == "variable_names":
            goals.append(eq(item.args[0], _variable_bindings(variables)))
        elif item.functor.name == "variables":
            goals.append(eq(item.args[0], _variable_values(variables)))
        else:
            return None
    return conj(*goals)


def _write_term_options(options: Term) -> dict[str, bool] | None:
    items = _logic_list_items(options)
    if items is None:
        return None
    parsed = {
        "quoted": False,
        "ignore_ops": False,
        "numbervars": False,
    }
    for item in items:
        if not isinstance(item, Compound) or len(item.args) != 1:
            return None
        option_name = item.functor.name
        value = item.args[0]
        if option_name in {"quoted", "ignore_ops", "numbervars"}:
            if (
                not isinstance(value, Atom)
                or value.symbol.name not in {"true", "false"}
            ):
                return None
            parsed[option_name] = value.symbol.name == "true"
            continue
        return None
    return parsed


def _render_prolog_term(term_value: Term, *, numbervars: bool = False) -> str:
    if numbervars:
        numbered_name = _render_numbered_variable(term_value)
        if numbered_name is not None:
            return numbered_name
    list_text = _render_list(term_value, numbervars=numbervars)
    if list_text is not None:
        return list_text
    if isinstance(term_value, Atom):
        return _render_atom(term_value.symbol.name)
    if isinstance(term_value, Number):
        return str(term_value.value)
    if isinstance(term_value, String):
        return '"' + _escape_string(term_value.value) + '"'
    if isinstance(term_value, LogicVar):
        return _render_variable(term_value)
    if isinstance(term_value, Compound):
        args = ", ".join(
            _render_prolog_term(argument, numbervars=numbervars)
            for argument in term_value.args
        )
        return f"{_render_functor(term_value.functor.name)}({args})"
    raise TypeError(f"cannot render {type(term_value).__name__} as Prolog term")


def _render_list(term_value: Term, *, numbervars: bool = False) -> str | None:
    items: list[str] = []
    current = term_value
    while (
        isinstance(current, Compound)
        and current.functor.name == "."
        and len(current.args) == 2
    ):
        items.append(_render_prolog_term(current.args[0], numbervars=numbervars))
        current = current.args[1]
    if isinstance(current, Atom) and current.symbol.name == "[]":
        return "[" + ", ".join(items) + "]"
    if items:
        tail = _render_prolog_term(current, numbervars=numbervars)
        return "[" + ", ".join(items) + " | " + tail + "]"
    return None


def _render_numbered_variable(term_value: Term) -> str | None:
    if (
        not isinstance(term_value, Compound)
        or term_value.functor.name != "$VAR"
        or len(term_value.args) != 1
    ):
        return None
    [index_term] = term_value.args
    if not isinstance(index_term, Number):
        return None
    index = index_term.value
    if isinstance(index, bool) or not isinstance(index, int) or index < 0:
        return None
    suffix = "" if index < 26 else str(index // 26)
    return chr(ord("A") + (index % 26)) + suffix


def _render_functor(name: str) -> str:
    if _PLAIN_ATOM_RE.fullmatch(name) or name in {"!", "[]"}:
        return name
    return "'" + _escape_atom(name) + "'"


def _render_atom(name: str) -> str:
    if _PLAIN_ATOM_RE.fullmatch(name) or name in {"!", "[]"}:
        return name
    return "'" + _escape_atom(name) + "'"


def _render_variable(variable: LogicVar) -> str:
    name = None if variable.display_name is None else variable.display_name.name
    if name is not None and _VARIABLE_RE.fullmatch(name):
        return name
    return f"_G{abs(variable.id)}"


def _escape_atom(value: str) -> str:
    return value.replace("\\", "\\\\").replace("'", "\\'")


def _escape_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


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


def _adapt_if_then_else(
    goal: DisjExpr,
    *,
    operator_table: OperatorTable | None,
) -> GoalExpr | None:
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
        _adapt_callable_goal(condition, operator_table=operator_table),
        _adapt_callable_goal(then_goal, operator_table=operator_table),
        adapt_prolog_goal(else_goal, operator_table=operator_table),
    )


def _adapt_callable_goal(
    term_value: Term,
    *,
    operator_table: OperatorTable | None = None,
) -> GoalExpr:
    try:
        return adapt_prolog_goal(
            goal_from_term(term_value),
            operator_table=operator_table,
        )
    except TypeError:
        return calltermo(term_value)


def _adapt_collection_goal(
    term_value: Term,
    *,
    operator_table: OperatorTable | None = None,
) -> GoalExpr:
    """Adapt the executable side of a collector goal, stripping ``^/2`` scopes."""

    return _adapt_callable_goal(
        _strip_collection_existentials(term_value),
        operator_table=operator_table,
    )


def _strip_collection_existentials(term_value: Term) -> Term:
    current = term_value
    while (
        isinstance(current, Compound)
        and current.functor.namespace is None
        and current.functor.name == "^"
        and len(current.args) == 2
    ):
        current = current.args[1]
    return current


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
