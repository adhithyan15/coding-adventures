"""Prolog-inspired builtins for the Python logic library.

Most relational helpers can be written with unification, conjunction,
disjunction, and relations. Prolog's everyday builtins also need to ask
questions about the *current* search state:

- is this variable still unbound?
- is this term ground after the bindings so far?
- does this goal have at least one proof?

`logic-engine` provides a small native-goal hook for that purpose. This module
keeps the user-facing predicates in a separate library layer.
"""

from __future__ import annotations

from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass
from hashlib import blake2b
from itertools import count, product
from pathlib import Path

from logic_engine import (
    Atom,
    Clause,
    Compound,
    ConjExpr,
    CutExpr,
    DeferredExpr,
    DisjExpr,
    EqExpr,
    FailExpr,
    FreshExpr,
    GoalExpr,
    LogicVar,
    NativeGoalExpr,
    NeqExpr,
    Number,
    Program,
    Relation,
    RelationCall,
    State,
    String,
    SucceedExpr,
    Term,
    atom,
    clause_body,
    clause_from_term,
    conj,
    cut,
    eq,
    fresh,
    freshen_clause,
    goal_as_term,
    goal_from_term,
    is_dynamic_relation,
    logic_list,
    native_goal,
    neq,
    num,
    reify,
    relation,
    runtime_abolish,
    runtime_asserta,
    runtime_assertz,
    runtime_declare_dynamic,
    runtime_retract_all,
    runtime_retract_first,
    solve_from,
    succeed,
    term,
    visible_clause_count,
    visible_clauses,
    visible_predicate_keys,
)
from logic_engine import (
    fail as engine_fail,
)

__all__ = [
    "add",
    "acyclic_termo",
    "all_differento",
    "argo",
    "atom_concato",
    "atom_codeso",
    "atom_charso",
    "atom_lengtho",
    "atom_numbero",
    "atomico",
    "atomo",
    "at_end_of_streamo",
    "atomic_list_concato",
    "atomic_list_concato_with_separator",
    "betweeno",
    "callo",
    "callableo",
    "call_cleanupo",
    "calltermo",
    "catcho",
    "closeo",
    "compound_name_argumentso",
    "compound_name_arityo",
    "compoundo",
    "compare_termo",
    "copytermo",
    "convlisto",
    "current_atomo",
    "current_functoro",
    "current_prolog_flago",
    "current_predicateo",
    "current_streamo",
    "cuto",
    "cyclic_termo",
    "difo",
    "dynamico",
    "div",
    "exists_fileo",
    "fd_eqo",
    "fd_elemento",
    "fd_geqo",
    "fd_gto",
    "fd_ino",
    "fd_leqo",
    "fd_lto",
    "fd_addo",
    "fd_bool_ando",
    "fd_bool_equivo",
    "fd_bool_implieso",
    "fd_bool_noto",
    "fd_bool_oro",
    "fd_mulo",
    "fd_neqo",
    "fd_reify_relationo",
    "fd_subo",
    "fd_scalar_producto",
    "fd_scalar_product_relationo",
    "fd_sumo",
    "fd_sum_relationo",
    "failo",
    "falseo",
    "excludeo",
    "FiniteDomainConstraint",
    "FiniteDomainStore",
    "findallo",
    "floordiv",
    "foldlo",
    "forallo",
    "flush_outputo",
    "functoro",
    "geqo",
    "get_charo",
    "gto",
    "groundo",
    "ifthenelseo",
    "iftheno",
    "ignoreo",
    "integero",
    "includeo",
    "iso",
    "leqo",
    "labelingo",
    "lto",
    "maplisto",
    "mod",
    "mul",
    "neg",
    "nonvaro",
    "not_same_termo",
    "noto",
    "numbervarso",
    "numeqo",
    "numneqo",
    "number_codeso",
    "number_charso",
    "number_stringo",
    "numbero",
    "nlo",
    "onceo",
    "openo",
    "open_optionso",
    "bagofo",
    "partitiono",
    "predicate_propertyo",
    "read_file_to_codeso",
    "read_file_to_stringo",
    "read_line_to_stringo",
    "read_stringo",
    "repeato",
    "PrologEvaluationError",
    "PrologFlagStore",
    "PrologInstantiationError",
    "PrologRuntimeError",
    "PrologThrown",
    "PrologTypeError",
    "prolog_geqo",
    "prolog_gto",
    "prolog_iso",
    "prolog_leqo",
    "prolog_lto",
    "prolog_numeqo",
    "prolog_numneqo",
    "assertao",
    "assertzo",
    "abolisho",
    "retractallo",
    "retracto",
    "same_termo",
    "scanlo",
    "setofo",
    "set_prolog_flago",
    "setup_call_cleanupo",
    "char_codeo",
    "clauseo",
    "string_codeso",
    "string_charso",
    "string_lengtho",
    "stringo",
    "stream_propertyo",
    "sub_atomo",
    "sub_stringo",
    "sub",
    "succo",
    "termo_geqo",
    "termo_gto",
    "termo_leqo",
    "termo_lto",
    "term_variableso",
    "term_hash_boundedo",
    "term_hasho",
    "throwo",
    "trueo",
    "univo",
    "unifiableo",
    "unify_with_occurs_checko",
    "varo",
    "writeo",
    "not_variant_termo",
    "subsumes_termo",
    "variant_termo",
]


type NativeArgs = tuple[Term, ...]
type NativeRunner = Callable[[Program, State, NativeArgs], Iterator[State]]
type NumericValue = int | float
type TermSortKey = tuple[object, ...]
type TermHashKey = tuple[object, ...]
type FdOperator = str


_MAX_FD_DOMAIN_SIZE = 10_000
_DEFAULT_TERM_HASH_RANGE = 2_147_483_647


@dataclass(slots=True)
class _TextStream:
    """Host-side UTF-8 text stream used by the bounded Prolog facade."""

    mode: str
    path: Path
    contents: str = ""
    cursor: int = 0
    alias: str | None = None


_STREAM_IDS = count(1)
_STREAMS: dict[str, _TextStream] = {}
_STREAM_ALIASES: dict[str, str] = {}


@dataclass(frozen=True, slots=True)
class FiniteDomainConstraint:
    """A residual finite-domain relation waiting for enough information."""

    operator: FdOperator
    terms: tuple[Term, ...]


@dataclass(frozen=True, slots=True)
class FiniteDomainStore:
    """Branch-local finite domains and residual integer constraints."""

    domains: dict[LogicVar, frozenset[int]]
    constraints: tuple[FiniteDomainConstraint, ...] = ()


@dataclass(frozen=True, slots=True)
class PrologFlagStore:
    """Branch-local Prolog flag overrides."""

    values: dict[Atom, Term]


@dataclass(frozen=True, slots=True)
class LabelingOptions:
    """Small, deterministic subset of Prolog CLP(FD) labeling options."""

    variable_order: str = "ff"
    value_order: str = "up"


class PrologRuntimeError(RuntimeError):
    """Base class for source-level Prolog runtime errors."""

    kind: str
    culprit: object | None

    def __init__(
        self,
        kind: str,
        message: str,
        *,
        culprit: object | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.culprit = culprit


class PrologInstantiationError(PrologRuntimeError):
    """Raised when a Prolog builtin needs a term to be instantiated."""

    def __init__(self, message: str, *, culprit: object | None = None) -> None:
        super().__init__("instantiation_error", message, culprit=culprit)


class PrologTypeError(PrologRuntimeError):
    """Raised when a Prolog builtin receives a term of the wrong type."""

    expected: str

    def __init__(
        self,
        expected: str,
        message: str,
        *,
        culprit: object | None = None,
    ) -> None:
        super().__init__("type_error", message, culprit=culprit)
        self.expected = expected


class PrologEvaluationError(PrologRuntimeError):
    """Raised when arithmetic evaluation itself fails."""

    evaluation_error: str

    def __init__(
        self,
        evaluation_error: str,
        message: str,
        *,
        culprit: object | None = None,
    ) -> None:
        super().__init__("evaluation_error", message, culprit=culprit)
        self.evaluation_error = evaluation_error


class PrologThrown(PrologRuntimeError):
    """Raised by ``throwo/1`` to unwind to the nearest ``catcho/3``."""

    term: Term
    state: State

    def __init__(self, term_value: Term, state: State) -> None:
        super().__init__("throw", f"uncaught Prolog exception: {term_value}")
        self.term = term_value
        self.state = state


_DEFAULT_LABELING_OPTIONS = LabelingOptions()


_BUILTIN_PREDICATES: tuple[tuple[str, int], ...] = (
    ("abolisho", 2),
    ("acyclic_termo", 1),
    ("all_differento", 1),
    ("argo", 3),
    ("assertao", 1),
    ("assertzo", 1),
    ("atom_concato", 3),
    ("atom_codeso", 2),
    ("atom_charso", 2),
    ("atom_lengtho", 2),
    ("atom_numbero", 2),
    ("atomico", 1),
    ("atomo", 1),
    ("atomic_list_concato", 2),
    ("atomic_list_concato_with_separator", 3),
    ("bagofo", 3),
    ("betweeno", 3),
    ("callableo", 1),
    ("callo", 1),
    ("call_cleanupo", 2),
    ("calltermo", 1),
    ("calltermo", 2),
    ("calltermo", 3),
    ("calltermo", 4),
    ("calltermo", 5),
    ("calltermo", 6),
    ("calltermo", 7),
    ("calltermo", 8),
    ("catcho", 3),
    ("clauseo", 2),
    ("compare_termo", 3),
    ("compound_name_argumentso", 3),
    ("compound_name_arityo", 3),
    ("compoundo", 1),
    ("copytermo", 2),
    ("convlisto", 3),
    ("current_atomo", 1),
    ("current_functoro", 2),
    ("current_prolog_flago", 2),
    ("current_predicateo", 2),
    ("cuto", 0),
    ("cyclic_termo", 1),
    ("difo", 2),
    ("dynamico", 2),
    ("fd_eqo", 2),
    ("fd_elemento", 3),
    ("fd_geqo", 2),
    ("fd_gto", 2),
    ("fd_ino", 2),
    ("fd_leqo", 2),
    ("fd_lto", 2),
    ("fd_addo", 3),
    ("fd_bool_ando", 3),
    ("fd_bool_equivo", 3),
    ("fd_bool_implieso", 3),
    ("fd_bool_noto", 2),
    ("fd_bool_oro", 3),
    ("fd_mulo", 3),
    ("fd_neqo", 2),
    ("fd_reify_relationo", 4),
    ("fd_subo", 3),
    ("fd_scalar_producto", 3),
    ("fd_scalar_product_relationo", 4),
    ("fd_sumo", 2),
    ("fd_sum_relationo", 3),
    ("failo", 0),
    ("falseo", 0),
    ("excludeo", 3),
    ("findallo", 3),
    ("foldlo", 4),
    ("foldlo", 5),
    ("foldlo", 6),
    ("foldlo", 7),
    ("forallo", 2),
    ("functoro", 3),
    ("geqo", 2),
    ("groundo", 1),
    ("gto", 2),
    ("ifthenelseo", 3),
    ("iftheno", 2),
    ("ignoreo", 1),
    ("integero", 1),
    ("includeo", 3),
    ("iso", 2),
    ("labeling_optionso", 2),
    ("labelingo", 1),
    ("leqo", 2),
    ("lto", 2),
    ("maplisto", 2),
    ("maplisto", 3),
    ("maplisto", 4),
    ("maplisto", 5),
    ("nonvaro", 1),
    ("not_same_termo", 2),
    ("noto", 1),
    ("numeqo", 2),
    ("numneqo", 2),
    ("number_codeso", 2),
    ("number_charso", 2),
    ("number_stringo", 2),
    ("numbero", 1),
    ("onceo", 1),
    ("partitiono", 4),
    ("predicate_propertyo", 3),
    ("repeato", 0),
    ("retractallo", 1),
    ("retracto", 1),
    ("same_termo", 2),
    ("scanlo", 4),
    ("scanlo", 5),
    ("scanlo", 6),
    ("scanlo", 7),
    ("setofo", 3),
    ("set_prolog_flago", 2),
    ("setup_call_cleanupo", 3),
    ("char_codeo", 2),
    ("string_codeso", 2),
    ("string_charso", 2),
    ("string_lengtho", 2),
    ("stringo", 1),
    ("sub_atomo", 5),
    ("sub_stringo", 5),
    ("succo", 2),
    ("term_hash_boundedo", 4),
    ("term_hasho", 2),
    ("termo_geqo", 2),
    ("termo_gto", 2),
    ("termo_leqo", 2),
    ("termo_lto", 2),
    ("term_variableso", 2),
    ("throwo", 1),
    ("trueo", 0),
    ("univo", 2),
    ("unifiableo", 3),
    ("unify_with_occurs_checko", 2),
    ("varo", 1),
    ("not_variant_termo", 2),
    ("subsumes_termo", 2),
    ("variant_termo", 2),
)


_PROLOG_FLAGS: tuple[tuple[Atom, Term], ...] = (
    (atom("bounded"), atom("false")),
    (atom("char_conversion"), atom("false")),
    (atom("debug"), atom("false")),
    (atom("double_quotes"), atom("string")),
    (atom("integer_rounding_function"), atom("floor")),
    (atom("occurs_check"), atom("false")),
    (atom("unknown"), atom("fail")),
)
_PROLOG_DEFAULT_FLAGS: dict[Atom, Term] = dict(_PROLOG_FLAGS)
_PROLOG_WRITABLE_FLAG_VALUES: dict[Atom, tuple[Term, ...]] = {
    atom("char_conversion"): (atom("false"), atom("true")),
    atom("debug"): (atom("false"), atom("true")),
    atom("double_quotes"): (
        atom("atom"),
        atom("chars"),
        atom("codes"),
        atom("string"),
    ),
    atom("occurs_check"): (atom("false"), atom("true"), atom("error")),
    atom("unknown"): (atom("error"), atom("fail"), atom("warning")),
}


def _as_goal(goal: object) -> GoalExpr:
    """Validate and normalize one goal-like object using the engine API."""

    return conj(goal)


def _as_callable_term(term_value: object) -> object:
    """Allow relation calls where a Prolog callable term is expected."""

    if isinstance(term_value, RelationCall):
        return term_value.as_term()
    return term_value


def _reified(term_value: Term, state: State) -> Term:
    """Read the current value of a term under the active substitution."""

    return reify(term_value, state.substitution)


def _is_ground(term_value: Term) -> bool:
    """Return True when a reified term contains no logic variables."""

    if isinstance(term_value, LogicVar):
        return False
    if isinstance(term_value, Compound):
        return all(_is_ground(argument) for argument in term_value.args)
    return True


def _is_acyclic_term(term_value: Term, visiting: set[int] | None = None) -> bool:
    """Return True when a term graph contains no recursive compound path."""

    if not isinstance(term_value, Compound):
        return True

    active = visiting if visiting is not None else set()
    identity = id(term_value)
    if identity in active:
        return False

    active.add(identity)
    try:
        return all(_is_acyclic_term(argument, active) for argument in term_value.args)
    finally:
        active.remove(identity)


def _is_empty_list(term_value: Term) -> bool:
    """Return True when a term is the canonical empty logic list."""

    return (
        isinstance(term_value, Atom)
        and term_value.symbol.namespace is None
        and term_value.symbol.name == "[]"
    )


def _is_cons(term_value: Term) -> bool:
    """Return True when a term is a canonical logic-list cons cell."""

    return (
        isinstance(term_value, Compound)
        and term_value.functor.namespace is None
        and term_value.functor.name == "."
        and len(term_value.args) == 2
    )


def _proper_list_items(term_value: Term) -> list[Term] | None:
    """Decode a reified proper logic list into host-language items."""

    items: list[Term] = []
    current = term_value
    while not _is_empty_list(current):
        if not _is_cons(current):
            return None
        head, tail = current.args
        items.append(head)
        current = tail
    return items


def _fresh_copy(
    term_value: Term,
    mapping: dict[LogicVar, LogicVar],
    next_var_id: int,
) -> tuple[Term, int]:
    """Copy one term, replacing every source variable with a fresh variable."""

    if isinstance(term_value, LogicVar):
        existing = mapping.get(term_value)
        if existing is not None:
            return existing, next_var_id

        copied = LogicVar(
            id=next_var_id,
            display_name=term_value.display_name,
        )
        mapping[term_value] = copied
        return copied, next_var_id + 1

    if isinstance(term_value, Compound):
        copied_args: list[Term] = []
        running_id = next_var_id
        for argument in term_value.args:
            copied_argument, running_id = _fresh_copy(
                argument,
                mapping,
                running_id,
            )
            copied_args.append(copied_argument)
        return Compound(functor=term_value.functor, args=tuple(copied_args)), running_id

    return term_value, next_var_id


def _state_with_next_var_id(state: State, next_var_id: int) -> State:
    """Preserve the logical state while reserving freshly allocated variables."""

    return State(
        substitution=state.substitution,
        constraints=state.constraints,
        next_var_id=next_var_id,
        database=state.database,
        fd_store=state.fd_store,
        prolog_flags=state.prolog_flags,
    )


def _empty_fd_store() -> FiniteDomainStore:
    """Return an empty finite-domain overlay."""

    return FiniteDomainStore(domains={})


def _fd_store(state: State) -> FiniteDomainStore:
    """Read the branch-local finite-domain store from ``state``."""

    if state.fd_store is None:
        return _empty_fd_store()
    if isinstance(state.fd_store, FiniteDomainStore):
        return state.fd_store
    msg = "State.fd_store contains an unsupported finite-domain store"
    raise TypeError(msg)


def _state_with_fd_store(state: State, store: FiniteDomainStore) -> State:
    """Return ``state`` with a normalized finite-domain store attached."""

    return State(
        substitution=state.substitution,
        constraints=state.constraints,
        next_var_id=state.next_var_id,
        database=state.database,
        fd_store=store,
        prolog_flags=state.prolog_flags,
    )


def _empty_prolog_flag_store() -> PrologFlagStore:
    """Return an empty branch-local Prolog flag overlay."""

    return PrologFlagStore(values={})


def _prolog_flag_store(state: State) -> PrologFlagStore:
    """Read the branch-local Prolog flag overlay from ``state``."""

    if state.prolog_flags is None:
        return _empty_prolog_flag_store()
    if isinstance(state.prolog_flags, PrologFlagStore):
        return state.prolog_flags
    msg = "State.prolog_flags contains an unsupported Prolog flag store"
    raise TypeError(msg)


def _state_with_prolog_flags(state: State, store: PrologFlagStore) -> State:
    """Return ``state`` with branch-local Prolog flag overrides attached."""

    return State(
        substitution=state.substitution,
        constraints=state.constraints,
        next_var_id=state.next_var_id,
        database=state.database,
        fd_store=state.fd_store,
        prolog_flags=store,
    )


def _integer_value(term_value: object) -> int | None:
    """Return a concrete finite-domain integer, rejecting floats and bools."""

    if isinstance(term_value, Number):
        raw_value = term_value.value
        if isinstance(raw_value, bool) or not isinstance(raw_value, int):
            return None
        return raw_value
    if isinstance(term_value, bool):
        return None
    if isinstance(term_value, int):
        return term_value
    return None


def _reified_integer(term_value: Term, state: State) -> int | None:
    """Read one reified term as a concrete non-bool integer."""

    return _integer_value(_reified(term_value, state))


def _domain_from_items(items: Iterable[object]) -> frozenset[int]:
    """Normalize a finite collection of integers into a checked domain."""

    values: set[int] = set()
    for item in items:
        integer = _integer_value(item)
        if integer is None:
            msg = "finite domains may contain only integer values"
            raise TypeError(msg)
        values.add(integer)
        if len(values) > _MAX_FD_DOMAIN_SIZE:
            msg = "finite domain exceeds the maximum supported size"
            raise ValueError(msg)
    return frozenset(values)


def _range_domain(low: int, high: int) -> frozenset[int]:
    """Build an inclusive finite integer range domain."""

    if high < low:
        return frozenset()
    if high - low + 1 > _MAX_FD_DOMAIN_SIZE:
        msg = "finite domain range exceeds the maximum supported size"
        raise ValueError(msg)
    return frozenset(range(low, high + 1))


def _domain_values(domain: object) -> frozenset[int]:
    """Normalize a Python or Prolog-shaped finite-domain description."""

    integer = _integer_value(domain)
    if integer is not None:
        return frozenset({integer})
    if isinstance(domain, range):
        if len(domain) > _MAX_FD_DOMAIN_SIZE:
            msg = "finite domain range exceeds the maximum supported size"
            raise ValueError(msg)
        return frozenset(domain)
    if isinstance(domain, list | tuple | set | frozenset):
        return _domain_from_items(domain)
    if isinstance(domain, Atom | Compound):
        items = _proper_list_items(domain)
        if items is not None:
            return _domain_from_items(items)
    if (
        isinstance(domain, Compound)
        and domain.functor.namespace is None
        and domain.functor.name == ".."
        and len(domain.args) == 2
    ):
        low = _integer_value(domain.args[0])
        high = _integer_value(domain.args[1])
        if low is None or high is None:
            msg = "finite range bounds must be integer values"
            raise TypeError(msg)
        return _range_domain(low, high)

    msg = f"cannot use {type(domain).__name__} as a finite domain"
    raise TypeError(msg)


def _fd_compare(operator: FdOperator, left: int, right: int) -> bool:
    """Evaluate one finite-domain binary relation over concrete integers."""

    if operator == "eq":
        return left == right
    if operator == "neq":
        return left != right
    if operator == "lt":
        return left < right
    if operator == "le":
        return left <= right
    if operator == "gt":
        return left > right
    if operator == "ge":
        return left >= right
    msg = f"unknown finite-domain operator {operator}"
    raise ValueError(msg)


_FD_RELATION_NEGATIONS: dict[FdOperator, FdOperator] = {
    "eq": "neq",
    "neq": "eq",
    "lt": "ge",
    "le": "gt",
    "gt": "le",
    "ge": "lt",
}


def _fd_boolean_tuple_satisfies(
    operator: FdOperator,
    values: tuple[int, ...],
) -> bool:
    """Evaluate one concrete truth-table constraint over CLP(FD) booleans."""

    if any(value not in {0, 1} for value in values):
        return False
    if operator == "bool_not":
        value, result = values
        return result == 1 - value
    left, right, result = values
    if operator == "bool_and":
        return result == int(left == 1 and right == 1)
    if operator == "bool_or":
        return result == int(left == 1 or right == 1)
    if operator == "bool_implies":
        return result == int(left == 0 or right == 1)
    if operator == "bool_equiv":
        return result == int(left == right)
    msg = f"unknown finite-domain boolean operator {operator}"
    raise ValueError(msg)


def _fd_tuple_satisfies(
    constraint: FiniteDomainConstraint,
    values: tuple[int, ...],
) -> bool:
    """Evaluate one concrete residual finite-domain constraint.

    The store keeps all pending finite-domain facts in one shape: an operator
    name and the terms it talks about. Concrete evaluation is deliberately
    small and obvious so each public predicate has a crisp mathematical meaning.
    """

    if constraint.operator in {"neq", "lt", "le", "gt", "ge"}:
        left, right = values
        return _fd_compare(constraint.operator, left, right)
    if constraint.operator.startswith("reify_"):
        left, right, truth = values
        relation = constraint.operator.removeprefix("reify_")
        return truth in {0, 1} and _fd_compare(relation, left, right) == (truth == 1)
    if constraint.operator in {
        "bool_and",
        "bool_or",
        "bool_not",
        "bool_implies",
        "bool_equiv",
    }:
        return _fd_boolean_tuple_satisfies(constraint.operator, values)
    if constraint.operator == "add":
        left, right, result = values
        return left + right == result
    if constraint.operator == "sub":
        left, right, result = values
        return left - right == result
    if constraint.operator == "mul":
        left, right, result = values
        return left * right == result
    if constraint.operator == "sum":
        *terms_value, result = values
        return sum(terms_value) == result
    if constraint.operator.startswith("sum_"):
        *terms_value, result = values
        return _fd_compare(
            constraint.operator.removeprefix("sum_"),
            sum(terms_value),
            result,
        )
    if constraint.operator == "scalar_product":
        pair_count = (len(values) - 1) // 2
        if len(values) != (pair_count * 2) + 1:
            return False
        coeffs = values[:pair_count]
        terms_value = values[pair_count:-1]
        result = values[-1]
        return sum(
            coeff * term_value
            for coeff, term_value in zip(coeffs, terms_value, strict=True)
        ) == result
    if constraint.operator.startswith("scalar_product_"):
        pair_count = (len(values) - 1) // 2
        if len(values) != (pair_count * 2) + 1:
            return False
        coeffs = values[:pair_count]
        terms_value = values[pair_count:-1]
        result = values[-1]
        weighted_total = sum(
            coeff * term_value
            for coeff, term_value in zip(coeffs, terms_value, strict=True)
        )
        return _fd_compare(
            constraint.operator.removeprefix("scalar_product_"),
            weighted_total,
            result,
        )
    if constraint.operator == "element":
        index, *items, value = values
        return 1 <= index <= len(items) and items[index - 1] == value
    if constraint.operator == "all_different":
        return len(set(values)) == len(values)
    msg = f"unknown finite-domain constraint {constraint.operator}"
    raise ValueError(msg)


def _reified_fd_value(term_value: Term, state: State) -> LogicVar | int | None:
    """Read a term as either an open variable or a concrete integer."""

    reified_value = _reified(term_value, state)
    if isinstance(reified_value, LogicVar):
        return reified_value
    return _integer_value(reified_value)


def _normalize_fd_domains(
    store: FiniteDomainStore,
    state: State,
) -> dict[LogicVar, frozenset[int]] | None:
    """Merge domains for aliased variables and reject incompatible bindings."""

    domains: dict[LogicVar, frozenset[int]] = {}
    for variable, domain in store.domains.items():
        reified_value = _reified(variable, state)
        if isinstance(reified_value, LogicVar):
            existing = domains.get(reified_value)
            domains[reified_value] = domain if existing is None else existing & domain
            if not domains[reified_value]:
                return None
            continue

        integer = _integer_value(reified_value)
        if integer is None or integer not in domain:
            return None

    return domains


def _domain_for_fd_value(
    value: LogicVar | int,
    domains: dict[LogicVar, frozenset[int]],
) -> frozenset[int] | None:
    """Return a finite domain for a concrete value or known FD variable."""

    if isinstance(value, LogicVar):
        return domains.get(value)
    return frozenset({value})


def _constraint_values(
    constraint: FiniteDomainConstraint,
    state: State,
) -> tuple[LogicVar | int, ...] | None:
    """Read all constraint terms as open FD variables or concrete integers."""

    values: list[LogicVar | int] = []
    for term_value in constraint.terms:
        value = _reified_fd_value(term_value, state)
        if value is None:
            return None
        values.append(value)
    return tuple(values)


def _constraint_domains(
    values: tuple[LogicVar | int, ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[frozenset[int] | None, ...]:
    """Return concrete singleton domains or known finite variable domains."""

    return tuple(_domain_for_fd_value(value, domains) for value in values)


def _revise_binary_constraint_domains(
    constraint: FiniteDomainConstraint,
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Apply arc-consistency pruning for a binary comparison constraint."""

    left_value, right_value = values
    left_domain, right_domain = value_domains

    allowed_left = frozenset(
        left
        for left in left_domain
        if any(_fd_compare(constraint.operator, left, right) for right in right_domain)
    )
    allowed_right = frozenset(
        right
        for right in right_domain
        if any(_fd_compare(constraint.operator, left, right) for left in left_domain)
    )
    if not allowed_left or not allowed_right:
        return None, False

    changed = False
    updated = dict(domains)
    if isinstance(left_value, LogicVar) and allowed_left != left_domain:
        updated[left_value] = allowed_left
        changed = True
    if isinstance(right_value, LogicVar) and allowed_right != right_domain:
        updated[right_value] = allowed_right
        changed = True

    return updated, changed


def _revise_reified_relation_domains(
    constraint: FiniteDomainConstraint,
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Prune domains for ``Truth #<==> (Left Relation Right)``."""

    left_value, right_value, truth_value = values
    left_domain, right_domain, truth_domain = value_domains
    relation = constraint.operator.removeprefix("reify_")

    supports_true = any(
        _fd_compare(relation, left, right)
        for left in left_domain
        for right in right_domain
    )
    supports_false = any(
        not _fd_compare(relation, left, right)
        for left in left_domain
        for right in right_domain
    )
    allowed_truth = truth_domain & frozenset(
        value
        for value, supported in ((1, supports_true), (0, supports_false))
        if supported
    )
    if not allowed_truth:
        return None, False

    updated = dict(domains)
    changed = False
    if isinstance(truth_value, LogicVar) and allowed_truth != truth_domain:
        updated[truth_value] = allowed_truth
        changed = True

    if len(allowed_truth) != 1:
        return updated, changed

    (truth,) = allowed_truth
    relation_to_enforce = (
        relation if truth == 1 else _FD_RELATION_NEGATIONS[relation]
    )
    revised, revised_changed = _revise_binary_constraint_domains(
        FiniteDomainConstraint(relation_to_enforce, constraint.terms[:2]),
        (left_value, right_value),
        (left_domain, right_domain),
        updated,
    )
    if revised is None:
        return None, False
    return revised, changed or revised_changed


def _revise_truth_table_constraint_domains(
    constraint: FiniteDomainConstraint,
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Prune boolean FD domains by enumerating their tiny truth table."""

    supported: list[set[int]] = [set() for _ in values]
    for candidate_tuple in product(*value_domains):
        if not _fd_boolean_tuple_satisfies(constraint.operator, candidate_tuple):
            continue
        for index, candidate in enumerate(candidate_tuple):
            supported[index].add(candidate)

    if any(not candidates for candidates in supported):
        return None, False

    updated = dict(domains)
    changed = False
    for value, current_domain, candidates in zip(
        values,
        value_domains,
        supported,
        strict=True,
    ):
        allowed = current_domain & frozenset(candidates)
        if not allowed:
            return None, False
        if isinstance(value, LogicVar) and allowed != current_domain:
            updated[value] = allowed
            changed = True

    return updated, changed


def _fd_arithmetic_holds(
    operator: FdOperator,
    left: int,
    right: int,
    result: int,
) -> bool:
    """Evaluate one ternary finite-domain arithmetic relation."""

    if operator == "add":
        return left + right == result
    if operator == "sub":
        return left - right == result
    if operator == "mul":
        return left * right == result
    msg = f"unknown finite-domain arithmetic operator {operator}"
    raise ValueError(msg)


def _has_arithmetic_support(
    operator: FdOperator,
    index: int,
    candidate: int,
    value_domains: tuple[frozenset[int], frozenset[int], frozenset[int]],
) -> bool:
    """Return True when ``candidate`` can appear in a satisfying triple."""

    left_domain, right_domain, result_domain = value_domains
    if operator == "add":
        if index == 0:
            return any(candidate + right in result_domain for right in right_domain)
        if index == 1:
            return any(left + candidate in result_domain for left in left_domain)
        return any(candidate - left in right_domain for left in left_domain)

    if operator == "sub":
        if index == 0:
            return any(candidate - right in result_domain for right in right_domain)
        if index == 1:
            return any(left - candidate in result_domain for left in left_domain)
        return any(left - candidate in right_domain for left in left_domain)

    if operator == "mul":
        if index == 0:
            if candidate == 0:
                return 0 in result_domain and bool(right_domain)
            return any(
                result % candidate == 0 and result // candidate in right_domain
                for result in result_domain
            )
        if index == 1:
            if candidate == 0:
                return 0 in result_domain and bool(left_domain)
            return any(
                result % candidate == 0 and result // candidate in left_domain
                for result in result_domain
            )
        return any(
            candidate % left == 0 and candidate // left in right_domain
            for left in left_domain
            if left != 0
        ) or (candidate == 0 and (0 in left_domain or 0 in right_domain))

    msg = f"unknown finite-domain arithmetic operator {operator}"
    raise ValueError(msg)


def _revise_arithmetic_constraint_domains(
    constraint: FiniteDomainConstraint,
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Prune ternary arithmetic domains to values with tuple support."""

    arithmetic_domains = (
        value_domains[0],
        value_domains[1],
        value_domains[2],
    )
    updated = dict(domains)
    changed = False
    for index, value in enumerate(values):
        if not isinstance(value, LogicVar):
            continue
        current_domain = arithmetic_domains[index]
        allowed = frozenset(
            candidate
            for candidate in current_domain
            if _has_arithmetic_support(
                constraint.operator,
                index,
                candidate,
                arithmetic_domains,
            )
        )
        if not allowed:
            return None, False
        if allowed != current_domain:
            updated[value] = allowed
            changed = True

    return updated, changed


def _revise_sum_constraint_domains(
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
    relation: FdOperator = "eq",
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Prune sum domains by checking each candidate against interval bounds."""

    term_domains = value_domains[:-1]
    result_domain = value_domains[-1]
    updated = dict(domains)
    changed = False
    term_mins = tuple(min(domain) for domain in term_domains)
    term_maxes = tuple(max(domain) for domain in term_domains)
    min_total = sum(term_mins)
    max_total = sum(term_maxes)

    for index, value in enumerate(values):
        if not isinstance(value, LogicVar):
            continue
        current_domain = value_domains[index]
        if index == len(values) - 1:
            allowed = frozenset(
                candidate
                for candidate in current_domain
                if _range_has_relation_support(
                    min_total,
                    max_total,
                    relation,
                    frozenset({candidate}),
                )
            )
        else:
            other_min = min_total - term_mins[index]
            other_max = max_total - term_maxes[index]
            allowed = frozenset(
                candidate
                for candidate in current_domain
                if _range_has_relation_support(
                    other_min + candidate,
                    other_max + candidate,
                    relation,
                    result_domain,
                )
            )
        if not allowed:
            return None, False
        if allowed != current_domain:
            updated[value] = allowed
            changed = True

    return updated, changed


def _range_has_relation_support(
    low: int,
    high: int,
    relation: FdOperator,
    target_domain: frozenset[int],
) -> bool:
    """Return True if any integer in an interval can relate to a target."""

    if relation == "eq":
        return any(low <= target <= high for target in target_domain)
    if relation == "neq":
        return bool(target_domain) and (
            low < high
            or len(target_domain) > 1
            or low not in target_domain
        )
    if relation == "lt":
        return low < max(target_domain)
    if relation == "le":
        return low <= max(target_domain)
    if relation == "gt":
        return high > min(target_domain)
    if relation == "ge":
        return high >= min(target_domain)
    msg = f"unknown finite-domain relation {relation}"
    raise ValueError(msg)


def _linear_bounds(coefficient: int, domain: frozenset[int]) -> tuple[int, int]:
    """Return min/max contribution for one weighted finite-domain term."""

    low = coefficient * min(domain)
    high = coefficient * max(domain)
    return (min(low, high), max(low, high))


def _revise_scalar_product_constraint_domains(
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
    relation: FdOperator = "eq",
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Prune weighted-sum domains with interval bounds."""

    pair_count = (len(values) - 1) // 2
    if len(values) != (pair_count * 2) + 1:
        return None, False

    coeff_values = values[:pair_count]
    term_values = values[pair_count:-1]
    result_value = values[-1]
    term_domains = value_domains[pair_count:-1]
    result_domain = value_domains[-1]
    if any(not isinstance(coefficient, int) for coefficient in coeff_values):
        return domains, False

    coeffs = tuple(
        coefficient
        for coefficient in coeff_values
        if isinstance(coefficient, int)
    )
    contribution_bounds = tuple(
        _linear_bounds(coefficient, domain)
        for coefficient, domain in zip(coeffs, term_domains, strict=True)
    )
    min_total = sum(low for low, _ in contribution_bounds)
    max_total = sum(high for _, high in contribution_bounds)

    updated = dict(domains)
    changed = False
    if isinstance(result_value, LogicVar):
        allowed_result = frozenset(
            candidate
            for candidate in result_domain
            if _range_has_relation_support(
                min_total,
                max_total,
                relation,
                frozenset({candidate}),
            )
        )
        if not allowed_result:
            return None, False
        if allowed_result != result_domain:
            updated[result_value] = allowed_result
            changed = True

    for index, (value, coefficient, current_domain) in enumerate(
        zip(term_values, coeffs, term_domains, strict=True),
    ):
        if not isinstance(value, LogicVar):
            continue
        contribution_low, contribution_high = contribution_bounds[index]
        other_min = min_total - contribution_low
        other_max = max_total - contribution_high
        allowed = frozenset(
            candidate
            for candidate in current_domain
            if _range_has_relation_support(
                other_min + (coefficient * candidate),
                other_max + (coefficient * candidate),
                relation,
                result_domain,
            )
        )
        if not allowed:
            return None, False
        if allowed != current_domain:
            updated[value] = allowed
            changed = True

    return updated, changed


def _revise_element_constraint_domains(
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int], ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Prune domains for the 1-based CLP(FD) element global constraint."""

    if len(values) < 3:
        return None, False
    index_value, *item_values, target_value = values
    index_domain, *item_domains, target_domain = value_domains
    indexed_items = tuple(enumerate(item_domains, start=1))
    allowed_index = frozenset(
        index
        for index in index_domain
        if 1 <= index <= len(item_domains)
        and bool(item_domains[index - 1] & target_domain)
    )
    if not allowed_index:
        return None, False

    allowed_target = frozenset(
        candidate
        for candidate in target_domain
        if any(
            candidate in item_domain
            for index, item_domain in indexed_items
            if index in allowed_index
        )
    )
    if not allowed_target:
        return None, False

    updated = dict(domains)
    changed = False
    if isinstance(index_value, LogicVar) and allowed_index != index_domain:
        updated[index_value] = allowed_index
        changed = True
    if isinstance(target_value, LogicVar) and allowed_target != target_domain:
        updated[target_value] = allowed_target
        changed = True

    if len(allowed_index) == 1:
        (selected_index,) = allowed_index
        item_value = item_values[selected_index - 1]
        item_domain = item_domains[selected_index - 1]
        allowed_item = item_domain & allowed_target
        if not allowed_item:
            return None, False
        if isinstance(item_value, LogicVar) and allowed_item != item_domain:
            updated[item_value] = allowed_item
            changed = True

    return updated, changed


def _revise_all_different_domains(
    values: tuple[LogicVar | int, ...],
    value_domains: tuple[frozenset[int] | None, ...],
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Propagate simple singleton pruning for ``all_differento``.

    This is intentionally the small, trustworthy version of all-different. It
    rejects duplicate concrete assignments and removes values that are already
    forced elsewhere. More advanced Hall-set pruning can layer on later.
    """

    concrete_seen: set[int] = set()
    singleton_by_owner: dict[LogicVar | int, int] = {}
    for value, domain in zip(values, value_domains, strict=True):
        if isinstance(value, int):
            if value in concrete_seen:
                return None, False
            concrete_seen.add(value)
            singleton_by_owner[value] = value
            continue
        if domain is None:
            continue
        if len(domain) == 1:
            (singleton,) = domain
            if singleton in singleton_by_owner.values():
                return None, False
            singleton_by_owner[value] = singleton

    forced_values = set(singleton_by_owner.values())
    updated = dict(domains)
    changed = False
    for value, domain in zip(values, value_domains, strict=True):
        if not isinstance(value, LogicVar) or domain is None or len(domain) == 1:
            continue
        allowed = frozenset(
            candidate for candidate in domain if candidate not in forced_values
        )
        if not allowed:
            return None, False
        if allowed != domain:
            updated[value] = allowed
            changed = True

    return updated, changed


def _revise_constraint_domains(
    constraint: FiniteDomainConstraint,
    state: State,
    domains: dict[LogicVar, frozenset[int]],
) -> tuple[dict[LogicVar, frozenset[int]] | None, bool]:
    """Apply one round of pruning for a residual finite-domain constraint."""

    values = _constraint_values(constraint, state)
    if values is None:
        return None, False

    value_domains = _constraint_domains(values, domains)
    if constraint.operator == "all_different":
        return _revise_all_different_domains(values, value_domains, domains)

    if any(domain is None for domain in value_domains):
        return domains, False

    known_domains = tuple(
        domain for domain in value_domains if domain is not None
    )
    if constraint.operator in {"neq", "lt", "le", "gt", "ge"}:
        return _revise_binary_constraint_domains(
            constraint,
            values,
            known_domains,
            domains,
        )
    if constraint.operator.startswith("reify_"):
        return _revise_reified_relation_domains(
            constraint,
            values,
            known_domains,
            domains,
        )
    if constraint.operator in {
        "bool_and",
        "bool_or",
        "bool_not",
        "bool_implies",
        "bool_equiv",
    }:
        return _revise_truth_table_constraint_domains(
            constraint,
            values,
            known_domains,
            domains,
        )
    if constraint.operator in {"add", "sub", "mul"}:
        return _revise_arithmetic_constraint_domains(
            constraint,
            values,
            known_domains,
            domains,
        )
    if constraint.operator == "sum":
        return _revise_sum_constraint_domains(values, known_domains, domains)
    if constraint.operator.startswith("sum_"):
        return _revise_sum_constraint_domains(
            values,
            known_domains,
            domains,
            constraint.operator.removeprefix("sum_"),
        )
    if constraint.operator == "scalar_product":
        return _revise_scalar_product_constraint_domains(
            values,
            known_domains,
            domains,
        )
    if constraint.operator.startswith("scalar_product_"):
        return _revise_scalar_product_constraint_domains(
            values,
            known_domains,
            domains,
            constraint.operator.removeprefix("scalar_product_"),
        )
    if constraint.operator == "element":
        return _revise_element_constraint_domains(values, known_domains, domains)

    msg = f"unknown finite-domain constraint {constraint.operator}"
    raise ValueError(msg)


def _concrete_constraint_values(
    values: tuple[LogicVar | int, ...],
) -> tuple[int, ...] | None:
    """Return concrete integer values only when every constraint term is bound."""

    if all(isinstance(value, int) for value in values):
        return tuple(value for value in values if isinstance(value, int))
    return None


def _normalize_fd_store(
    store: FiniteDomainStore,
    state: State,
) -> FiniteDomainStore | None:
    """Re-check FD domains and residual constraints after state changes."""

    domains = _normalize_fd_domains(store, state)
    if domains is None:
        return None

    kept_constraints: list[FiniteDomainConstraint] = []
    for constraint in store.constraints:
        values = _constraint_values(constraint, state)
        if values is None:
            return None
        concrete_values = _concrete_constraint_values(values)
        if concrete_values is not None:
            if not _fd_tuple_satisfies(constraint, concrete_values):
                return None
            continue
        kept_constraints.append(constraint)

    changed = True
    while changed:
        changed = False
        for constraint in kept_constraints:
            revised, revised_changed = _revise_constraint_domains(
                constraint,
                state,
                domains,
            )
            if revised is None:
                return None
            domains = revised
            changed = changed or revised_changed

    return FiniteDomainStore(
        domains=dict(domains),
        constraints=tuple(kept_constraints),
    )


def _normalize_fd_state(state: State) -> State | None:
    """Return ``state`` with its FD store reconciled against substitutions."""

    store = _normalize_fd_store(_fd_store(state), state)
    if store is None:
        return None
    return _state_with_fd_store(state, store)


def _succeed_if(condition: bool, state: State) -> Iterator[State]:
    """Yield the current state exactly when a builtin predicate succeeds."""

    if condition:
        yield state


def _term_sort_key(term_value: Term) -> TermSortKey:
    """Return the documented Prolog-inspired standard term ordering key."""

    if isinstance(term_value, LogicVar):
        return (0, term_value.id, str(term_value.display_name or ""))
    if isinstance(term_value, Number):
        return (1, term_value.value)
    if isinstance(term_value, Atom):
        return (2, term_value.symbol.namespace or "", term_value.symbol.name)
    if isinstance(term_value, String):
        return (3, term_value.value)
    return (
        4,
        len(term_value.args),
        term_value.functor.namespace or "",
        term_value.functor.name,
        tuple(_term_sort_key(argument) for argument in term_value.args),
    )


def _compare_terms(left: Term, right: Term) -> int:
    """Compare two already reified terms using the standard term order."""

    left_key = _term_sort_key(left)
    right_key = _term_sort_key(right)
    if left_key < right_key:
        return -1
    if left_key > right_key:
        return 1
    return 0


def _unique_sorted_terms(values: list[Term]) -> list[Term]:
    """Remove duplicate terms and return them in deterministic set order."""

    unique: dict[Term, Term] = {}
    for value in values:
        unique.setdefault(value, value)
    return sorted(unique.values(), key=_term_sort_key)


def _collect_template_values(
    program_value: Program,
    state: State,
    template: Term,
    goal: GoalExpr,
) -> list[Term]:
    """Collect reified template values for every proof of a nested goal."""

    return [
        reify(template, inner_state.substitution)
        for inner_state in solve_from(program_value, goal, state)
    ]


def _strip_existential_quantifiers(scope: Term) -> tuple[tuple[LogicVar, ...], Term]:
    """Return variables marked existential by Prolog ``^/2`` collection syntax."""

    variables: list[LogicVar] = []
    seen: set[LogicVar] = set()
    current = scope
    while (
        isinstance(current, Compound)
        and current.functor.namespace is None
        and current.functor.name == "^"
        and len(current.args) == 2
    ):
        quantified, current = current.args
        for variable in _term_variables_in_order(quantified):
            if variable not in seen:
                seen.add(variable)
                variables.append(variable)
    return tuple(variables), current


def _collection_free_variables(
    template: Term,
    scope: Term,
    state: State,
) -> tuple[LogicVar, ...]:
    """Return unquantified goal variables that group ``bagof``/``setof``."""

    reified_template = _reified(template, state)
    reified_scope = _reified(scope, state)
    existential_vars, scoped_goal = _strip_existential_quantifiers(reified_scope)
    template_vars = set(_term_variables_in_order(reified_template))
    existential_set = set(existential_vars)
    return tuple(
        variable
        for variable in _term_variables_in_order(scoped_goal)
        if variable not in template_vars and variable not in existential_set
    )


def _group_sort_key(key: tuple[Term, ...]) -> tuple[object, ...]:
    """Return a deterministic standard-term-order-ish key for one group."""

    return tuple(_term_sort_key(item) for item in key)


def _collect_template_groups(
    program_value: Program,
    state: State,
    template: Term,
    goal: GoalExpr,
    scope: Term | None,
) -> list[tuple[tuple[Term, ...], list[Term]]]:
    """Collect template values, grouped by free variables when a scope exists."""

    if scope is None:
        values = _collect_template_values(program_value, state, template, goal)
        return [((), values)] if values else []

    free_vars = _collection_free_variables(template, scope, state)
    groups: dict[tuple[Term, ...], list[Term]] = {}
    for inner_state in solve_from(program_value, goal, state):
        key = tuple(reify(variable, inner_state.substitution) for variable in free_vars)
        value = reify(template, inner_state.substitution)
        groups.setdefault(key, []).append(value)
    return sorted(groups.items(), key=lambda item: _group_sort_key(item[0]))


def _unify_collection(
    program_value: Program,
    state: State,
    results: Term,
    values: list[Term],
) -> Iterator[State]:
    """Unify a result term with a canonical logic list from the outer state."""

    yield from solve_from(program_value, eq(results, logic_list(values)), state)


def _unify_collection_group(
    program_value: Program,
    state: State,
    results: Term,
    values: list[Term],
    free_vars: tuple[LogicVar, ...],
    key: tuple[Term, ...],
) -> Iterator[State]:
    """Unify one grouped bag/set answer from the outer collection state."""

    bindings = [
        eq(variable, value)
        for variable, value in zip(free_vars, key, strict=True)
    ]
    yield from solve_from(
        program_value,
        conj(*bindings, eq(results, logic_list(values))),
        state,
    )


def _unify_grouped_collection(
    program_value: Program,
    state: State,
    template: Term,
    results: Term,
    groups: list[tuple[tuple[Term, ...], list[Term]]],
    scope: Term | None,
    *,
    unique: bool,
) -> Iterator[State]:
    """Unify bag/set groups, binding grouping variables per outer answer."""

    if not groups:
        return
    free_vars = (
        ()
        if scope is None
        else _collection_free_variables(template, scope, state)
    )
    for key, values in groups:
        group_values = _unique_sorted_terms(values) if unique else values
        yield from _unify_collection_group(
            program_value,
            state,
            results,
            group_values,
            free_vars,
            key,
        )


def findallo(
    template: object,
    goal: object,
    results: object,
    *,
    scope: Term | None = None,
) -> GoalExpr:
    """Collect every solution of `goal` into `results`, preserving proof order."""

    called_goal = _as_goal(goal)
    goal_term = _goal_term_or_none(called_goal)

    if goal_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            template_term, called_goal_term, results_term = args
            try:
                _, scoped_goal = _strip_existential_quantifiers(
                    _reified(called_goal_term, state),
                )
                reified_goal = goal_from_term(scoped_goal)
            except TypeError:
                return
            values = _collect_template_values(
                program_value,
                state,
                template_term,
                reified_goal,
            )
            yield from _unify_collection(program_value, state, results_term, values)

        return native_goal(run_terms, template, goal_term, results)

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        template_term, results_term = args
        values = _collect_template_values(
            program_value,
            state,
            template_term,
            called_goal,
        )
        yield from _unify_collection(program_value, state, results_term, values)

    return native_goal(run, template, results)


def bagofo(
    template: object,
    goal: object,
    results: object,
    *,
    scope: Term | None = None,
) -> GoalExpr:
    """Collect a non-empty proof-order bag, grouped by free variables."""

    called_goal = _as_goal(goal)
    goal_term = _goal_term_or_none(called_goal)
    goal_scope = goal_term if scope is None else scope

    if goal_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            template_term, called_goal_term, results_term = args
            try:
                _, scoped_goal = _strip_existential_quantifiers(
                    _reified(called_goal_term, state),
                )
                reified_goal = goal_from_term(scoped_goal)
            except TypeError:
                return
            groups = _collect_template_groups(
                program_value,
                state,
                template_term,
                reified_goal,
                goal_scope,
            )
            yield from _unify_grouped_collection(
                program_value,
                state,
                template_term,
                results_term,
                groups,
                goal_scope,
                unique=False,
            )

        return native_goal(run_terms, template, goal_term, results)

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        template_term, results_term = args
        groups = _collect_template_groups(
            program_value,
            state,
            template_term,
            called_goal,
            goal_scope,
        )
        yield from _unify_grouped_collection(
            program_value,
            state,
            template_term,
            results_term,
            groups,
            goal_scope,
            unique=False,
        )

    return native_goal(run, template, results)


def setofo(
    template: object,
    goal: object,
    results: object,
    *,
    scope: Term | None = None,
) -> GoalExpr:
    """Collect a non-empty sorted set, grouped by free variables."""

    called_goal = _as_goal(goal)
    goal_term = _goal_term_or_none(called_goal)
    goal_scope = goal_term if scope is None else scope

    if goal_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            template_term, called_goal_term, results_term = args
            try:
                _, scoped_goal = _strip_existential_quantifiers(
                    _reified(called_goal_term, state),
                )
                reified_goal = goal_from_term(scoped_goal)
            except TypeError:
                return
            groups = _collect_template_groups(
                program_value,
                state,
                template_term,
                reified_goal,
                goal_scope,
            )
            yield from _unify_grouped_collection(
                program_value,
                state,
                template_term,
                results_term,
                groups,
                goal_scope,
                unique=True,
            )

        return native_goal(run_terms, template, goal_term, results)

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        template_term, results_term = args
        groups = _collect_template_groups(
            program_value,
            state,
            template_term,
            called_goal,
            goal_scope,
        )
        yield from _unify_grouped_collection(
            program_value,
            state,
            template_term,
            results_term,
            groups,
            goal_scope,
            unique=True,
        )

    return native_goal(run, template, results)


def trueo() -> GoalExpr:
    """Succeed once without changing the current logic state."""

    return succeed()


def failo() -> GoalExpr:
    """Fail without yielding any successor states."""

    return engine_fail()


def falseo() -> GoalExpr:
    """Alias for logical failure, matching Prolog's standard `false/0`."""

    return failo()


def cuto() -> GoalExpr:
    """Commit to choices made so far in the current search-control frame."""

    return cut()


def repeato() -> GoalExpr:
    """Succeed repeatedly, leaving callers to bound search with cut or limits."""

    def run(_program: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        while True:
            yield state

    return native_goal(run)


def fd_ino(target: object, domain: object) -> GoalExpr:
    """Constrain ``target`` to a finite integer domain.

    Domains are intentionally concrete in the foundation layer: callers can use
    Python ranges/iterables, a single integer, a proper logic list, or a
    ``..(Low, High)`` compound term. The store narrows immediately, and normal
    backtracking restores older domain snapshots.
    """

    domain_values = _domain_values(domain)

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target_term,) = args
        target_value = _reified_fd_value(target_term, state)
        if target_value is None:
            return
        if isinstance(target_value, int):
            if target_value in domain_values:
                normalized_state = _normalize_fd_state(state)
                if normalized_state is not None:
                    yield normalized_state
            return

        store = _fd_store(state)
        domains = dict(store.domains)
        existing_domain = domains.get(target_value)
        narrowed_domain = (
            domain_values
            if existing_domain is None
            else existing_domain & domain_values
        )
        if not narrowed_domain:
            return

        updated_store = FiniteDomainStore(
            domains={**domains, target_value: narrowed_domain},
            constraints=store.constraints,
        )
        normalized_store = _normalize_fd_store(updated_store, state)
        if normalized_store is not None:
            yield _state_with_fd_store(state, normalized_store)

    return native_goal(run, target)


def fd_eqo(left: object, right: object) -> GoalExpr:
    """Constrain two finite-domain terms to equal integer values."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        for unified_state in solve_from(
            program_value,
            eq(left_term, right_term),
            state,
        ):
            normalized_state = _normalize_fd_state(unified_state)
            if normalized_state is not None:
                yield normalized_state

    return native_goal(run, left, right)


def _fd_constrainto(operator: FdOperator, *terms_value: object) -> GoalExpr:
    """Create a residual finite-domain constraint goal."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        store = _fd_store(state)
        updated_store = FiniteDomainStore(
            domains=dict(store.domains),
            constraints=(
                *store.constraints,
                FiniteDomainConstraint(operator, args),
            ),
        )
        normalized_store = _normalize_fd_store(updated_store, state)
        if normalized_store is not None:
            yield _state_with_fd_store(state, normalized_store)

    return native_goal(run, *terms_value)


def fd_neqo(left: object, right: object) -> GoalExpr:
    """Constrain two finite-domain terms to different integer values."""

    return _fd_constrainto("neq", left, right)


def fd_lto(left: object, right: object) -> GoalExpr:
    """Constrain ``left`` to be less than ``right``."""

    return _fd_constrainto("lt", left, right)


def fd_leqo(left: object, right: object) -> GoalExpr:
    """Constrain ``left`` to be less than or equal to ``right``."""

    return _fd_constrainto("le", left, right)


def fd_gto(left: object, right: object) -> GoalExpr:
    """Constrain ``left`` to be greater than ``right``."""

    return _fd_constrainto("gt", left, right)


def fd_geqo(left: object, right: object) -> GoalExpr:
    """Constrain ``left`` to be greater than or equal to ``right``."""

    return _fd_constrainto("ge", left, right)


def fd_addo(left: object, right: object, result: object) -> GoalExpr:
    """Constrain three finite-domain terms so ``left + right == result``."""

    return _fd_constrainto("add", left, right, result)


def fd_subo(left: object, right: object, result: object) -> GoalExpr:
    """Constrain three finite-domain terms so ``left - right == result``."""

    return _fd_constrainto("sub", left, right, result)


def fd_mulo(left: object, right: object, result: object) -> GoalExpr:
    """Constrain three finite-domain terms so ``left * right == result``."""

    return _fd_constrainto("mul", left, right, result)


def _fd_boolean_domaino(value: object) -> GoalExpr:
    """Constrain one finite-domain term to the CLP(FD) boolean domain."""

    return fd_ino(value, (0, 1))


def fd_bool_noto(value: object, result: object) -> GoalExpr:
    """Constrain ``result`` to the boolean negation of ``value``."""

    return conj(
        _fd_boolean_domaino(value),
        _fd_boolean_domaino(result),
        _fd_constrainto("bool_not", value, result),
    )


def fd_bool_ando(left: object, right: object, result: object) -> GoalExpr:
    """Constrain ``result`` to the boolean conjunction of two FD booleans."""

    return conj(
        _fd_boolean_domaino(left),
        _fd_boolean_domaino(right),
        _fd_boolean_domaino(result),
        _fd_constrainto("bool_and", left, right, result),
    )


def fd_bool_oro(left: object, right: object, result: object) -> GoalExpr:
    """Constrain ``result`` to the boolean disjunction of two FD booleans."""

    return conj(
        _fd_boolean_domaino(left),
        _fd_boolean_domaino(right),
        _fd_boolean_domaino(result),
        _fd_constrainto("bool_or", left, right, result),
    )


def fd_bool_implieso(left: object, right: object, result: object) -> GoalExpr:
    """Constrain ``result`` to the material implication of two FD booleans."""

    return conj(
        _fd_boolean_domaino(left),
        _fd_boolean_domaino(right),
        _fd_boolean_domaino(result),
        _fd_constrainto("bool_implies", left, right, result),
    )


def fd_bool_equivo(left: object, right: object, result: object) -> GoalExpr:
    """Constrain ``result`` to true when two FD booleans are equal."""

    return conj(
        _fd_boolean_domaino(left),
        _fd_boolean_domaino(right),
        _fd_boolean_domaino(result),
        _fd_constrainto("bool_equiv", left, right, result),
    )


_FD_RELATION_NAMES: dict[str, FdOperator] = {
    "#=": "eq",
    "=": "eq",
    "eq": "eq",
    "#\\=": "neq",
    "neq": "neq",
    "#<": "lt",
    "lt": "lt",
    "#=<": "le",
    "le": "le",
    "#>": "gt",
    "gt": "gt",
    "#>=": "ge",
    "ge": "ge",
}


def _fd_relation_name(operator_value: object) -> FdOperator | None:
    """Normalize a Prolog-style CLP(FD) relation operator."""

    if isinstance(operator_value, str):
        return _FD_RELATION_NAMES.get(operator_value)
    if isinstance(operator_value, Atom) and operator_value.symbol.namespace is None:
        return _FD_RELATION_NAMES.get(operator_value.symbol.name)
    return None


def fd_reify_relationo(
    left: object,
    operator_value: object,
    right: object,
    truth: object,
) -> GoalExpr:
    """Constrain ``truth`` to 1 iff ``left`` relates to ``right``."""

    operator = _fd_relation_name(operator_value)
    if operator is None:
        return failo()
    return conj(
        _fd_boolean_domaino(truth),
        _fd_constrainto(f"reify_{operator}", left, right, truth),
    )


def _sum_terms_goal(
    terms_value: tuple[Term, ...],
    operator: FdOperator,
    result: object,
) -> GoalExpr:
    """Create a sum goal for already materialized finite-domain terms."""

    constraint_operator = "sum" if operator == "eq" else f"sum_{operator}"
    return _fd_constrainto(constraint_operator, *terms_value, result)


def fd_sum_relationo(
    terms_value: object,
    operator_value: object,
    result: object,
) -> GoalExpr:
    """Constrain a finite-domain sum against ``result`` with a relation."""

    operator = _fd_relation_name(operator_value)
    if operator is None:
        return failo()

    if isinstance(terms_value, list | tuple):
        return _sum_terms_goal(tuple(terms_value), operator, result)

    def run_logic_list(
        program_value: Program,
        state: State,
        args: NativeArgs,
    ) -> Iterator[State]:
        terms_term, _operator_term, result_term = args
        items = _proper_list_items(_reified(terms_term, state))
        if items is None:
            return
        yield from solve_from(
            program_value,
            _sum_terms_goal(tuple(items), operator, result_term),
            state,
        )

    return native_goal(run_logic_list, terms_value, operator_value, result)


def fd_sumo(terms_value: object, result: object) -> GoalExpr:
    """Constrain finite-domain terms so their sum equals ``result``."""

    return fd_sum_relationo(terms_value, "eq", result)


def _scalar_product_terms_goal(
    coeffs: tuple[Term, ...],
    terms_value: tuple[Term, ...],
    operator: FdOperator,
    result: object,
) -> GoalExpr:
    """Create a weighted-sum goal for already materialized terms."""

    if len(coeffs) != len(terms_value):
        return failo()
    constraint_operator = (
        "scalar_product" if operator == "eq" else f"scalar_product_{operator}"
    )
    return _fd_constrainto(constraint_operator, *coeffs, *terms_value, result)


def fd_scalar_product_relationo(
    coeffs_value: object,
    terms_value: object,
    operator_value: object,
    result: object,
) -> GoalExpr:
    """Constrain a weighted finite-domain sum with a relation."""

    operator = _fd_relation_name(operator_value)
    if operator is None:
        return failo()

    if isinstance(coeffs_value, list | tuple) and isinstance(terms_value, list | tuple):
        return _scalar_product_terms_goal(
            tuple(coeffs_value),
            tuple(terms_value),
            operator,
            result,
        )

    def run_logic_lists(
        program_value: Program,
        state: State,
        args: NativeArgs,
    ) -> Iterator[State]:
        coeffs_term, terms_term, _operator_term, result_term = args
        coeff_items = _proper_list_items(_reified(coeffs_term, state))
        term_items = _proper_list_items(_reified(terms_term, state))
        if coeff_items is None or term_items is None:
            return
        yield from solve_from(
            program_value,
            _scalar_product_terms_goal(
                tuple(coeff_items),
                tuple(term_items),
                operator,
                result_term,
            ),
            state,
        )

    return native_goal(
        run_logic_lists,
        coeffs_value,
        terms_value,
        operator_value,
        result,
    )


def fd_scalar_producto(
    coeffs_value: object,
    terms_value: object,
    result: object,
) -> GoalExpr:
    """Constrain finite-domain terms so ``sum(Coeff * Term) == result``."""

    return fd_scalar_product_relationo(coeffs_value, terms_value, "eq", result)


def _element_terms_goal(
    index: object,
    items: tuple[Term, ...],
    value: object,
) -> GoalExpr:
    """Create an element goal for already materialized finite-domain terms."""

    if not items:
        return failo()
    return _fd_constrainto("element", index, *items, value)


def fd_elemento(index: object, items_value: object, value: object) -> GoalExpr:
    """Constrain ``value`` to equal the 1-based element at ``index``."""

    if isinstance(items_value, list | tuple):
        return _element_terms_goal(index, tuple(items_value), value)

    def run_logic_list(
        program_value: Program,
        state: State,
        args: NativeArgs,
    ) -> Iterator[State]:
        index_term, items_term, value_term = args
        items = _proper_list_items(_reified(items_term, state))
        if items is None:
            return
        yield from solve_from(
            program_value,
            _element_terms_goal(index_term, tuple(items), value_term),
            state,
        )

    return native_goal(run_logic_list, index, items_value, value)


def _all_different_terms_goal(terms_value: tuple[Term, ...]) -> GoalExpr:
    """Create an all-different goal for already materialized terms."""

    return _fd_constrainto("all_different", *terms_value)


def all_differento(vars_value: object) -> GoalExpr:
    """Constrain every finite-domain term in ``vars_value`` to be distinct."""

    if isinstance(vars_value, list | tuple):
        return _all_different_terms_goal(tuple(vars_value))

    def run_logic_list(
        program_value: Program,
        state: State,
        args: NativeArgs,
    ) -> Iterator[State]:
        (vars_term,) = args
        items = _proper_list_items(_reified(vars_term, state))
        if items is None:
            return
        yield from solve_from(
            program_value,
            _all_different_terms_goal(tuple(items)),
            state,
        )

    return native_goal(run_logic_list, vars_value)


def _label_fd_terms(
    program_value: Program,
    state: State,
    terms: tuple[Term, ...],
    options: LabelingOptions = _DEFAULT_LABELING_OPTIONS,
) -> Iterator[State]:
    """Enumerate concrete values for requested FD variables.

    Labeling is the point where the constraint store becomes concrete search.
    By default, choose the smallest currently-known domain first to reduce
    branching while preserving the caller's variable order as a deterministic
    tie-breaker. Options can switch to Prolog's leftmost selection or descending
    value order.
    """

    normalized_state = _normalize_fd_state(state)
    if normalized_state is None:
        return

    candidates: list[tuple[int, int, LogicVar, frozenset[int]]] = []
    store = _fd_store(normalized_state)
    for index, term_value in enumerate(terms):
        value = _reified_fd_value(term_value, normalized_state)
        if value is None:
            return
        if isinstance(value, int):
            continue
        domain = store.domains.get(value)
        if domain is None:
            return
        candidates.append((len(domain), index, value, domain))

    if not candidates:
        yield normalized_state
        return

    if options.variable_order == "leftmost":
        _, _, value, domain = min(candidates, key=lambda candidate: candidate[1])
    else:
        _, _, value, domain = min(candidates, key=lambda candidate: candidate[:2])

    for choice in sorted(domain, reverse=options.value_order == "down"):
        for assigned_state in solve_from(
            program_value,
            eq(value, num(choice)),
            normalized_state,
        ):
            yield from _label_fd_terms(program_value, assigned_state, terms, options)


def _sequence_items(term_value: object) -> list[Term] | None:
    if isinstance(term_value, list | tuple):
        return list(term_value)
    if isinstance(term_value, Atom | Compound):
        return _proper_list_items(term_value)
    return None


def _labeling_option_name(option: Term) -> str | None:
    if isinstance(option, str):
        return option
    if (
        isinstance(option, Atom)
        and option.symbol.namespace is None
    ):
        return option.symbol.name
    return None


def _labeling_options_from_items(items: list[Term]) -> LabelingOptions | None:
    options = LabelingOptions()
    for item in items:
        name = _labeling_option_name(item)
        if name in {"ff", "leftmost"}:
            options = LabelingOptions(
                variable_order=name,
                value_order=options.value_order,
            )
            continue
        if name in {"up", "down"}:
            options = LabelingOptions(
                variable_order=options.variable_order,
                value_order=name,
            )
            continue
        if name in {"enum", "step"}:
            continue
        return None
    return options


def labelingo(vars_value: object) -> GoalExpr:
    """Enumerate concrete assignments for finite-domain variables."""

    if isinstance(vars_value, list | tuple):
        def run_sequence(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            yield from _label_fd_terms(program_value, state, args)

        return native_goal(run_sequence, *vars_value)

    def run_logic_list(
        program_value: Program,
        state: State,
        args: NativeArgs,
    ) -> Iterator[State]:
        (vars_term,) = args
        items = _proper_list_items(_reified(vars_term, state))
        if items is None:
            return
        yield from _label_fd_terms(program_value, state, tuple(items))

    return native_goal(run_logic_list, vars_value)


def labeling_optionso(options_value: object, vars_value: object) -> GoalExpr:
    """Enumerate finite-domain variables with a small labeling option subset."""

    if isinstance(options_value, list | tuple):
        options = _labeling_options_from_items(list(options_value))
        if options is None:
            return failo()

        if isinstance(vars_value, list | tuple):
            def run_sequence(
                program_value: Program,
                state: State,
                args: NativeArgs,
            ) -> Iterator[State]:
                yield from _label_fd_terms(program_value, state, args, options)

            return native_goal(run_sequence, *vars_value)

        def run_logic_vars(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            (vars_term,) = args
            var_items = _sequence_items(_reified(vars_term, state))
            if var_items is None:
                return
            yield from _label_fd_terms(program_value, state, tuple(var_items), options)

        return native_goal(run_logic_vars, vars_value)

    if isinstance(vars_value, list | tuple):
        def run_options_with_sequence(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            options_term, *var_terms = args
            option_items = _sequence_items(_reified(options_term, state))
            if option_items is None:
                return
            options = _labeling_options_from_items(option_items)
            if options is None:
                return
            yield from _label_fd_terms(
                program_value,
                state,
                tuple(var_terms),
                options,
            )

        return native_goal(run_options_with_sequence, options_value, *vars_value)

    def run(
        program_value: Program,
        state: State,
        args: NativeArgs,
    ) -> Iterator[State]:
        options_term, vars_term = args
        option_items = _sequence_items(_reified(options_term, state))
        var_items = _sequence_items(_reified(vars_term, state))
        if option_items is None or var_items is None:
            return
        options = _labeling_options_from_items(option_items)
        if options is None:
            return
        yield from _label_fd_terms(program_value, state, tuple(var_items), options)

    return native_goal(run, options_value, vars_value)


def iftheno(condition: object, then_goal: object) -> GoalExpr:
    """Run `then_goal` from the first proof of `condition`, or fail."""

    condition_goal = _as_goal(condition)
    called_then = _as_goal(then_goal)
    condition_term = _goal_term_or_none(condition_goal)
    then_term = _goal_term_or_none(called_then)

    if condition_term is not None and then_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            condition_goal_term, then_goal_term = args
            try:
                called_condition = goal_from_term(
                    _reified(condition_goal_term, state),
                )
            except TypeError:
                return

            condition_proofs = solve_from(program_value, called_condition, state)
            first_condition_state = next(condition_proofs, None)
            if first_condition_state is None:
                return

            try:
                called_then_goal = goal_from_term(
                    _reified(then_goal_term, first_condition_state),
                )
            except TypeError:
                return
            yield from solve_from(
                program_value,
                called_then_goal,
                first_condition_state,
            )

        return native_goal(run_terms, condition_term, then_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        condition_proofs = solve_from(program_value, condition_goal, state)
        first_condition_state = next(condition_proofs, None)
        if first_condition_state is None:
            return
        yield from solve_from(program_value, called_then, first_condition_state)

    return native_goal(run)


def ifthenelseo(condition: object, then_goal: object, else_goal: object) -> GoalExpr:
    """Choose a committed then branch or an else branch from the original state."""

    condition_goal = _as_goal(condition)
    called_then = _as_goal(then_goal)
    called_else = _as_goal(else_goal)
    condition_term = _goal_term_or_none(condition_goal)
    then_term = _goal_term_or_none(called_then)
    else_term = _goal_term_or_none(called_else)

    if (
        condition_term is not None
        and then_term is not None
        and else_term is not None
    ):

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            condition_goal_term, then_goal_term, else_goal_term = args
            try:
                called_condition = goal_from_term(
                    _reified(condition_goal_term, state),
                )
            except TypeError:
                return

            condition_proofs = solve_from(program_value, called_condition, state)
            first_condition_state = next(condition_proofs, None)
            if first_condition_state is None:
                try:
                    called_else_goal = goal_from_term(
                        _reified(else_goal_term, state),
                    )
                except TypeError:
                    return
                yield from solve_from(program_value, called_else_goal, state)
                return

            try:
                called_then_goal = goal_from_term(
                    _reified(then_goal_term, first_condition_state),
                )
            except TypeError:
                return
            yield from solve_from(
                program_value,
                called_then_goal,
                first_condition_state,
            )

        return native_goal(run_terms, condition_term, then_term, else_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        condition_proofs = solve_from(program_value, condition_goal, state)
        first_condition_state = next(condition_proofs, None)
        if first_condition_state is None:
            yield from solve_from(program_value, called_else, state)
            return
        yield from solve_from(program_value, called_then, first_condition_state)

    return native_goal(run)


def _goal_term_or_none(goal: GoalExpr) -> Term | None:
    try:
        return goal_as_term(goal)
    except TypeError:
        return None


def throwo(ball: object) -> GoalExpr:
    """Throw a Prolog exception term to the nearest enclosing ``catcho/3``."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (ball_term,) = args
        raise PrologThrown(_reified(ball_term, state), state)
        yield state

    return native_goal(run, ball)


def _runtime_error_ball(error: PrologRuntimeError) -> Term:
    """Represent structured runtime errors as catchable Prolog error terms."""

    if isinstance(error, PrologTypeError):
        formal = term(
            "type_error",
            atom(error.expected),
            error.culprit
            if isinstance(error.culprit, Atom | Compound | LogicVar | Number | String)
            else atom("unknown"),
        )
    elif isinstance(error, PrologEvaluationError):
        formal = term("evaluation_error", atom(error.evaluation_error))
    elif isinstance(error, PrologInstantiationError):
        formal = atom("instantiation_error")
    else:
        formal = atom(error.kind)
    return term("error", formal, atom("logic_runtime"))


def _catch_exception(
    program_value: Program,
    state: State,
    catcher: Term,
    recovery_goal: GoalExpr,
    thrown: PrologThrown,
) -> Iterator[State]:
    """Run catch recovery when the thrown ball unifies with the catcher."""

    matched = False
    for matched_state in solve_from(
        program_value,
        eq(catcher, thrown.term),
        thrown.state,
    ):
        matched = True
        yield from solve_from(program_value, recovery_goal, matched_state)
    if not matched:
        raise thrown


def catcho(goal: object, catcher: object, recovery: object) -> GoalExpr:
    """Run ``goal`` and recover from matching Prolog exceptions."""

    called_goal = _as_goal(goal)
    recovery_goal = _as_goal(recovery)
    goal_term = _goal_term_or_none(called_goal)
    recovery_term = _goal_term_or_none(recovery_goal)

    if goal_term is not None and recovery_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            called_goal_term, catcher_term, recovery_goal_term = args
            try:
                reified_goal = goal_from_term(_reified(called_goal_term, state))
                reified_recovery = goal_from_term(_reified(recovery_goal_term, state))
                yield from solve_from(program_value, reified_goal, state)
            except PrologThrown as thrown:
                yield from _catch_exception(
                    program_value,
                    state,
                    catcher_term,
                    reified_recovery,
                    thrown,
                )
            except PrologRuntimeError as error:
                yield from _catch_exception(
                    program_value,
                    state,
                    catcher_term,
                    reified_recovery,
                    PrologThrown(_runtime_error_ball(error), state),
                )

        return native_goal(run_terms, goal_term, catcher, recovery_term)

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (catcher_term,) = args
        try:
            yield from solve_from(program_value, called_goal, state)
        except PrologThrown as thrown:
            yield from _catch_exception(
                program_value,
                state,
                catcher_term,
                recovery_goal,
                thrown,
            )
        except PrologRuntimeError as error:
            yield from _catch_exception(
                program_value,
                state,
                catcher_term,
                recovery_goal,
                PrologThrown(_runtime_error_ball(error), state),
            )

    return native_goal(run, catcher)


def forallo(generator: object, test: object) -> GoalExpr:
    """Succeed once when every generated proof satisfies `test` at least once."""

    generator_goal = _as_goal(generator)
    test_goal = _as_goal(test)
    generator_term = _goal_term_or_none(generator_goal)
    test_term = _goal_term_or_none(test_goal)

    if generator_term is not None and test_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            generator_goal_term, test_goal_term = args
            try:
                called_generator = goal_from_term(
                    _reified(generator_goal_term, state),
                )
            except TypeError:
                return

            for generated_state in solve_from(program_value, called_generator, state):
                try:
                    called_test = goal_from_term(
                        _reified(test_goal_term, generated_state),
                    )
                except TypeError:
                    return
                test_proofs = solve_from(program_value, called_test, generated_state)
                if next(test_proofs, None) is None:
                    return
            yield state

        return native_goal(run_terms, generator_term, test_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        for generated_state in solve_from(program_value, generator_goal, state):
            test_proofs = solve_from(program_value, test_goal, generated_state)
            if next(test_proofs, None) is None:
                return
        yield state

    return native_goal(run)


def add(left: object, right: object) -> Compound:
    """Build a symbolic arithmetic addition expression."""

    return term("+", left, right)


def sub(left: object, right: object) -> Compound:
    """Build a symbolic arithmetic subtraction expression."""

    return term("-", left, right)


def mul(left: object, right: object) -> Compound:
    """Build a symbolic arithmetic multiplication expression."""

    return term("*", left, right)


def div(left: object, right: object) -> Compound:
    """Build a symbolic arithmetic true-division expression."""

    return term("/", left, right)


def floordiv(left: object, right: object) -> Compound:
    """Build a symbolic arithmetic floor-division expression."""

    return term("//", left, right)


def mod(left: object, right: object) -> Compound:
    """Build a symbolic arithmetic modulo expression."""

    return term("mod", left, right)


def neg(value: object) -> Compound:
    """Build a symbolic arithmetic unary-negation expression."""

    return term("-", value)


def _numeric_value(term_value: Term, state: State) -> NumericValue | None:
    """Evaluate a reified arithmetic expression into a host numeric value."""

    reified_value = _reified(term_value, state)
    if isinstance(reified_value, Number):
        return reified_value.value
    if not isinstance(reified_value, Compound):
        return None
    if reified_value.functor.namespace is not None:
        return None

    operator = reified_value.functor.name
    arguments = reified_value.args

    if operator == "-" and len(arguments) == 1:
        value = _numeric_value(arguments[0], state)
        if value is None:
            return None
        return -value

    if len(arguments) != 2:
        return None

    left = _numeric_value(arguments[0], state)
    right = _numeric_value(arguments[1], state)
    if left is None or right is None:
        return None

    if operator == "+":
        return left + right
    if operator == "-":
        return left - right
    if operator == "*":
        return left * right
    if operator == "/":
        if right == 0:
            return None
        return left / right
    if operator == "//":
        if right == 0:
            return None
        return left // right
    if operator == "mod":
        if right == 0:
            return None
        return left % right

    return None


def _prolog_numeric_value(term_value: Term, state: State) -> NumericValue:
    """Evaluate an arithmetic expression using Prolog runtime error semantics."""

    reified_value = _reified(term_value, state)
    if isinstance(reified_value, LogicVar):
        msg = "arithmetic expression is not sufficiently instantiated"
        raise PrologInstantiationError(msg, culprit=reified_value)
    if isinstance(reified_value, Number):
        return reified_value.value
    if not isinstance(reified_value, Compound) or reified_value.functor.namespace:
        msg = "expected an evaluable arithmetic expression"
        raise PrologTypeError("evaluable", msg, culprit=reified_value)

    operator = reified_value.functor.name
    arguments = reified_value.args

    if operator == "-" and len(arguments) == 1:
        return -_prolog_numeric_value(arguments[0], state)

    if len(arguments) != 2:
        msg = "expected an evaluable arithmetic expression"
        raise PrologTypeError("evaluable", msg, culprit=reified_value)

    left = _prolog_numeric_value(arguments[0], state)
    right = _prolog_numeric_value(arguments[1], state)

    if operator == "+":
        return left + right
    if operator == "-":
        return left - right
    if operator == "*":
        return left * right
    if operator == "/":
        if right == 0:
            msg = "division by zero"
            raise PrologEvaluationError("zero_divisor", msg, culprit=reified_value)
        return left / right
    if operator == "//":
        if right == 0:
            msg = "integer division by zero"
            raise PrologEvaluationError("zero_divisor", msg, culprit=reified_value)
        return left // right
    if operator == "mod":
        if right == 0:
            msg = "modulo by zero"
            raise PrologEvaluationError("zero_divisor", msg, culprit=reified_value)
        return left % right

    msg = "expected an evaluable arithmetic expression"
    raise PrologTypeError("evaluable", msg, culprit=reified_value)


def iso(result: object, expression: object) -> GoalExpr:
    """Evaluate an arithmetic expression and unify the numeric result."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        result_term, expression_term = args
        value = _numeric_value(expression_term, state)
        if value is None:
            return
        yield from solve_from(program_value, eq(result_term, num(value)), state)

    return native_goal(run, result, expression)


def prolog_iso(result: object, expression: object) -> GoalExpr:
    """Evaluate `is/2` with source-level Prolog error semantics."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        result_term, expression_term = args
        value = _prolog_numeric_value(expression_term, state)
        yield from solve_from(program_value, eq(result_term, num(value)), state)

    return native_goal(run, result, expression)


def _numeric_compareo(
    left: object,
    right: object,
    predicate: Callable[[NumericValue, NumericValue], bool],
) -> GoalExpr:
    """Build an arithmetic comparison goal over two evaluated expressions."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        left_value = _numeric_value(left_term, state)
        right_value = _numeric_value(right_term, state)
        if left_value is None or right_value is None:
            return
        yield from _succeed_if(predicate(left_value, right_value), state)

    return native_goal(run, left, right)


def _prolog_numeric_compareo(
    left: object,
    right: object,
    predicate: Callable[[NumericValue, NumericValue], bool],
) -> GoalExpr:
    """Build a source-level Prolog arithmetic comparison goal."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        left_value = _prolog_numeric_value(left_term, state)
        right_value = _prolog_numeric_value(right_term, state)
        yield from _succeed_if(predicate(left_value, right_value), state)

    return native_goal(run, left, right)


def numeqo(left: object, right: object) -> GoalExpr:
    """Succeed when two arithmetic expressions evaluate to equal numbers."""

    return _numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value == right_value,
    )


def prolog_numeqo(left: object, right: object) -> GoalExpr:
    """Evaluate `=:=/2` with source-level Prolog error semantics."""

    return _prolog_numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value == right_value,
    )


def numneqo(left: object, right: object) -> GoalExpr:
    """Succeed when two arithmetic expressions evaluate to different numbers."""

    return _numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value != right_value,
    )


def prolog_numneqo(left: object, right: object) -> GoalExpr:
    """Evaluate `=\\=/2` with source-level Prolog error semantics."""

    return _prolog_numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value != right_value,
    )


def lto(left: object, right: object) -> GoalExpr:
    """Succeed when the left arithmetic expression is less than the right."""

    return _numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value < right_value,
    )


def prolog_lto(left: object, right: object) -> GoalExpr:
    """Evaluate `</2` with source-level Prolog error semantics."""

    return _prolog_numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value < right_value,
    )


def leqo(left: object, right: object) -> GoalExpr:
    """Succeed when the left arithmetic expression is less than or equal to right."""

    return _numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value <= right_value,
    )


def prolog_leqo(left: object, right: object) -> GoalExpr:
    """Evaluate `=</2` with source-level Prolog error semantics."""

    return _prolog_numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value <= right_value,
    )


def gto(left: object, right: object) -> GoalExpr:
    """Succeed when the left arithmetic expression is greater than the right."""

    return _numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value > right_value,
    )


def prolog_gto(left: object, right: object) -> GoalExpr:
    """Evaluate `>/2` with source-level Prolog error semantics."""

    return _prolog_numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value > right_value,
    )


def geqo(left: object, right: object) -> GoalExpr:
    """Succeed when the left arithmetic expression is greater than or equal to right."""

    return _numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value >= right_value,
    )


def prolog_geqo(left: object, right: object) -> GoalExpr:
    """Evaluate `>=/2` with source-level Prolog error semantics."""

    return _prolog_numeric_compareo(
        left,
        right,
        lambda left_value, right_value: left_value >= right_value,
    )


def betweeno(low: object, high: object, value: object) -> GoalExpr:
    """Generate or validate an integer between two finite inclusive bounds."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        low_term, high_term, value_term = args
        low_value = _reified_integer(low_term, state)
        high_value = _reified_integer(high_term, state)
        if low_value is None or high_value is None or high_value < low_value:
            return

        value_reified = _reified(value_term, state)
        value_integer = _integer_value(value_reified)
        if value_integer is not None:
            if low_value <= value_integer <= high_value:
                yield state
            return

        if not isinstance(value_reified, LogicVar):
            return

        for candidate in range(low_value, high_value + 1):
            yield from solve_from(program_value, eq(value_term, num(candidate)), state)

    return native_goal(run, low, high, value)


def succo(predecessor: object, successor: object) -> GoalExpr:
    """Relate a non-negative integer to its successor."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        predecessor_term, successor_term = args
        predecessor_reified = _reified(predecessor_term, state)
        successor_reified = _reified(successor_term, state)
        predecessor_value = _integer_value(predecessor_reified)
        successor_value = _integer_value(successor_reified)

        if predecessor_value is not None and successor_value is not None:
            if predecessor_value >= 0 and successor_value == predecessor_value + 1:
                yield state
            return

        if predecessor_value is not None:
            if predecessor_value < 0 or not isinstance(successor_reified, LogicVar):
                return
            yield from solve_from(
                program_value,
                eq(successor_term, num(predecessor_value + 1)),
                state,
            )
            return

        if successor_value is not None:
            if successor_value <= 0 or not isinstance(predecessor_reified, LogicVar):
                return
            yield from solve_from(
                program_value,
                eq(predecessor_term, num(successor_value - 1)),
                state,
            )

    return native_goal(run, predecessor, successor)


def callo(goal: object) -> GoalExpr:
    """Run a goal supplied as data.

    Python callers already hold goal objects directly, so this is mostly a
    composability adapter and a stepping stone toward Prolog's `call/1`.
    """

    called_goal = _as_goal(goal)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        yield from solve_from(program_value, called_goal, state)

    return native_goal(run)


def _append_callable_arguments(
    callable_term: Term,
    extra_args: tuple[Term, ...],
) -> Term | None:
    """Return ``callable_term`` with extra arguments appended, if callable."""

    if not extra_args:
        return callable_term
    if isinstance(callable_term, RelationCall):
        callable_term = callable_term.as_term()
    if isinstance(callable_term, Atom) and callable_term.symbol.namespace is None:
        return Compound(functor=callable_term.symbol, args=extra_args)
    if isinstance(callable_term, Compound):
        if (
            callable_term.functor.namespace is None
            and callable_term.functor.name == ":"
            and len(callable_term.args) == 2
        ):
            qualified_goal = _append_callable_arguments(
                callable_term.args[1],
                extra_args,
            )
            if qualified_goal is None:
                return None
            return Compound(
                functor=callable_term.functor,
                args=(callable_term.args[0], qualified_goal),
            )
        return Compound(
            functor=callable_term.functor,
            args=(*callable_term.args, *extra_args),
        )
    return None


def calltermo(term_goal: object, *extra_args: object) -> GoalExpr:
    """Execute a reified callable term, optionally appending arguments."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        goal_term, *argument_terms = args
        reified_args = tuple(
            _reified(argument_term, state)
            for argument_term in argument_terms
        )
        callable_term = _append_callable_arguments(
            _reified(goal_term, state),
            reified_args,
        )
        if callable_term is None:
            return
        try:
            called_goal = goal_from_term(callable_term)
        except TypeError:
            return
        yield from solve_from(program_value, called_goal, state)

    return native_goal(run, _as_callable_term(term_goal), *extra_args)


def _runtime_goal(goal_term: Term, state: State) -> GoalExpr | None:
    """Convert a possibly-bound callable term into an executable goal."""

    try:
        return goal_from_term(_reified(goal_term, state))
    except TypeError:
        return None


def _run_cleanup_once(
    program_value: Program,
    state: State,
    cleanup_goal: GoalExpr,
) -> State:
    """Run cleanup for its first effectful proof, ignoring ordinary failure."""

    return next(solve_from(program_value, cleanup_goal, state), state)


def call_cleanupo(goal: object, cleanup: object) -> GoalExpr:
    """Run ``cleanup`` after each deterministic ``goal`` proof."""

    called_goal = _as_goal(goal)
    cleanup_goal = _as_goal(cleanup)
    goal_term = _goal_term_or_none(called_goal)
    cleanup_term = _goal_term_or_none(cleanup_goal)

    if goal_term is not None and cleanup_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            called_goal_term, cleanup_goal_term = args
            reified_goal = _runtime_goal(called_goal_term, state)
            reified_cleanup = _runtime_goal(cleanup_goal_term, state)
            if reified_goal is None or reified_cleanup is None:
                return
            try:
                for goal_state in solve_from(program_value, reified_goal, state):
                    yield _run_cleanup_once(
                        program_value,
                        goal_state,
                        reified_cleanup,
                    )
            except PrologThrown as thrown:
                cleanup_state = _run_cleanup_once(
                    program_value,
                    thrown.state,
                    reified_cleanup,
                )
                raise PrologThrown(thrown.term, cleanup_state) from thrown

        return native_goal(run_terms, goal_term, cleanup_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        try:
            for goal_state in solve_from(program_value, called_goal, state):
                yield _run_cleanup_once(program_value, goal_state, cleanup_goal)
        except PrologThrown as thrown:
            cleanup_state = _run_cleanup_once(
                program_value,
                thrown.state,
                cleanup_goal,
            )
            raise PrologThrown(thrown.term, cleanup_state) from thrown

    return native_goal(run)


def setup_call_cleanupo(setup: object, goal: object, cleanup: object) -> GoalExpr:
    """Run setup once, then run a goal with cleanup semantics."""

    return conj(setup, call_cleanupo(goal, cleanup))


def _fresh_logic_vars(
    count: int,
    next_var_id: int,
) -> tuple[tuple[LogicVar, ...], int]:
    vars_value = tuple(LogicVar(id=next_var_id + index) for index in range(count))
    return vars_value, next_var_id + count


def _materialize_proper_list_args(
    program_value: Program,
    state: State,
    list_terms: tuple[Term, ...],
) -> Iterator[tuple[State, tuple[tuple[Term, ...], ...]]]:
    """Resolve proper list arguments, creating same-length open lists when safe."""

    entries: list[list[Term] | LogicVar] = []
    known_lengths: set[int] = set()
    for list_term in list_terms:
        reified_list = _reified(list_term, state)
        items = _proper_list_items(reified_list)
        if items is not None:
            entries.append(items)
            known_lengths.add(len(items))
            continue
        if isinstance(reified_list, LogicVar):
            entries.append(reified_list)
            continue
        return

    if len(known_lengths) != 1:
        return

    (list_length,) = known_lengths
    next_state = state
    materialized: list[tuple[Term, ...]] = []
    constraints: list[tuple[LogicVar, tuple[LogicVar, ...]]] = []
    next_var_id = state.next_var_id
    for entry in entries:
        if isinstance(entry, LogicVar):
            vars_value, next_var_id = _fresh_logic_vars(list_length, next_var_id)
            materialized.append(vars_value)
            constraints.append((entry, vars_value))
            continue
        materialized.append(tuple(entry))

    if constraints:
        next_state = _state_with_next_var_id(state, next_var_id)
    states = [next_state]
    for list_var, items in constraints:
        states = [
            unified_state
            for running_state in states
            for unified_state in solve_from(
                program_value,
                eq(list_var, logic_list(items)),
                running_state,
            )
        ]

    for materialized_state in states:
        yield materialized_state, tuple(materialized)


def _run_maplist_rows(
    program_value: Program,
    state: State,
    closure: Term,
    rows: tuple[tuple[Term, ...], ...],
) -> Iterator[State]:
    if not rows:
        yield state
        return
    row, *tail = rows
    for called_state in solve_from(program_value, calltermo(closure, *row), state):
        yield from _run_maplist_rows(program_value, called_state, closure, tuple(tail))


def maplisto(closure: object, *lists_value: object) -> GoalExpr:
    """Apply a callable closure across one to four same-length lists."""

    if not 1 <= len(lists_value) <= 4:
        return failo()

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        closure_term, *list_terms = args
        for materialized_state, lists in _materialize_proper_list_args(
            program_value,
            state,
            tuple(list_terms),
        ):
            rows = tuple(zip(*lists, strict=True))
            yield from _run_maplist_rows(
                program_value,
                materialized_state,
                closure_term,
                rows,
            )

    return native_goal(run, _as_callable_term(closure), *lists_value)


def _first_success_state(
    program_value: Program,
    goal: GoalExpr,
    state: State,
) -> State | None:
    return next(solve_from(program_value, goal, state), None)


def _partition_items(
    program_value: Program,
    state: State,
    closure: Term,
    items: tuple[Term, ...],
) -> tuple[list[Term], list[Term], State] | None:
    included: list[Term] = []
    excluded: list[Term] = []
    running_state = state
    for item in items:
        called_state = _first_success_state(
            program_value,
            calltermo(closure, item),
            running_state,
        )
        if called_state is None:
            excluded.append(item)
            continue
        included.append(item)
        running_state = called_state
    return included, excluded, running_state


def partitiono(
    closure: object,
    items_value: object,
    included_value: object,
    excluded_value: object,
) -> GoalExpr:
    """Partition a proper list by a unary callable closure."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        closure_term, items_term, included_term, excluded_term = args
        items = _proper_list_items(_reified(items_term, state))
        if items is None:
            return
        partitioned = _partition_items(
            program_value,
            state,
            closure_term,
            tuple(items),
        )
        if partitioned is None:
            return
        included, excluded, partition_state = partitioned
        yield from solve_from(
            program_value,
            conj(
                eq(included_term, logic_list(included)),
                eq(excluded_term, logic_list(excluded)),
            ),
            partition_state,
        )

    return native_goal(
        run,
        _as_callable_term(closure),
        items_value,
        included_value,
        excluded_value,
    )


def includeo(closure: object, items_value: object, included_value: object) -> GoalExpr:
    """Keep the items for which a unary callable closure succeeds."""

    return fresh(
        1,
        lambda excluded: partitiono(closure, items_value, included_value, excluded),
    )


def excludeo(closure: object, items_value: object, excluded_value: object) -> GoalExpr:
    """Keep the items for which a unary callable closure fails."""

    return fresh(
        1,
        lambda included: partitiono(closure, items_value, included, excluded_value),
    )


def _run_convlist_items(
    program_value: Program,
    state: State,
    closure: Term,
    items: tuple[Term, ...],
    converted: tuple[Term, ...],
    results: Term,
) -> Iterator[State]:
    if not items:
        yield from solve_from(program_value, eq(results, logic_list(converted)), state)
        return

    item, *tail = items
    (converted_item,), next_var_id = _fresh_logic_vars(1, state.next_var_id)
    reserved_state = _state_with_next_var_id(state, next_var_id)
    called_state = _first_success_state(
        program_value,
        calltermo(closure, item, converted_item),
        reserved_state,
    )
    if called_state is None:
        yield from _run_convlist_items(
            program_value,
            state,
            closure,
            tuple(tail),
            converted,
            results,
        )
        return
    yield from _run_convlist_items(
        program_value,
        called_state,
        closure,
        tuple(tail),
        (*converted, converted_item),
        results,
    )


def convlisto(closure: object, items_value: object, results_value: object) -> GoalExpr:
    """Map and filter a proper list through a binary callable closure."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        closure_term, items_term, results_term = args
        items = _proper_list_items(_reified(items_term, state))
        if items is None:
            return
        yield from _run_convlist_items(
            program_value,
            state,
            closure_term,
            tuple(items),
            (),
            results_term,
        )

    return native_goal(run, _as_callable_term(closure), items_value, results_value)


def _run_foldl_rows(
    program_value: Program,
    state: State,
    closure: Term,
    rows: tuple[tuple[Term, ...], ...],
    accumulator: Term,
    result: Term,
) -> Iterator[State]:
    if not rows:
        yield from solve_from(program_value, eq(accumulator, result), state)
        return

    row, *tail = rows
    (next_accumulator,), next_var_id = _fresh_logic_vars(1, state.next_var_id)
    reserved_state = _state_with_next_var_id(state, next_var_id)
    for called_state in solve_from(
        program_value,
        calltermo(closure, *row, accumulator, next_accumulator),
        reserved_state,
    ):
        yield from _run_foldl_rows(
            program_value,
            called_state,
            closure,
            tuple(tail),
            next_accumulator,
            result,
        )


def foldlo(
    closure: object,
    *values: object,
) -> GoalExpr:
    """Fold one to four same-length lists through a callable closure."""

    if not 3 <= len(values) <= 6:
        return failo()

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        closure_term, *terms = args
        *list_terms, accumulator_term, result_term = terms
        for materialized_state, lists in _materialize_proper_list_args(
            program_value,
            state,
            tuple(list_terms),
        ):
            rows = tuple(zip(*lists, strict=True))
            yield from _run_foldl_rows(
                program_value,
                materialized_state,
                closure_term,
                rows,
                accumulator_term,
                result_term,
            )

    return native_goal(run, _as_callable_term(closure), *values)


def _run_scanl_rows(
    program_value: Program,
    state: State,
    closure: Term,
    rows: tuple[tuple[Term, ...], ...],
    accumulator: Term,
    results: Term,
    produced: tuple[Term, ...],
) -> Iterator[State]:
    if not rows:
        yield from solve_from(program_value, eq(results, logic_list(produced)), state)
        return

    row, *tail = rows
    (next_accumulator,), next_var_id = _fresh_logic_vars(1, state.next_var_id)
    reserved_state = _state_with_next_var_id(state, next_var_id)
    for called_state in solve_from(
        program_value,
        calltermo(closure, *row, accumulator, next_accumulator),
        reserved_state,
    ):
        yield from _run_scanl_rows(
            program_value,
            called_state,
            closure,
            tuple(tail),
            next_accumulator,
            results,
            (*produced, next_accumulator),
        )


def scanlo(closure: object, *values: object) -> GoalExpr:
    """Fold one to four same-length lists and collect intermediate accumulators."""

    if not 3 <= len(values) <= 6:
        return failo()

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        closure_term, *terms = args
        *list_terms, accumulator_term, results_term = terms
        for materialized_state, lists in _materialize_proper_list_args(
            program_value,
            state,
            tuple(list_terms),
        ):
            rows = tuple(zip(*lists, strict=True))
            yield from _run_scanl_rows(
                program_value,
                materialized_state,
                closure_term,
                rows,
                accumulator_term,
                results_term,
                (),
            )

    return native_goal(run, _as_callable_term(closure), *values)


def onceo(goal: object) -> GoalExpr:
    """Run `goal` and keep at most its first solution."""

    called_goal = _as_goal(goal)
    goal_term = _goal_term_or_none(called_goal)

    if goal_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            (called_goal_term,) = args
            try:
                reified_goal = goal_from_term(_reified(called_goal_term, state))
            except TypeError:
                return
            iterator = solve_from(program_value, reified_goal, state)
            first = next(iterator, None)
            if first is not None:
                yield first

        return native_goal(run_terms, goal_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        iterator = solve_from(program_value, called_goal, state)
        first = next(iterator, None)
        if first is not None:
            yield first

    return native_goal(run)


def ignoreo(goal: object) -> GoalExpr:
    """Run `goal` once, or succeed unchanged when `goal` cannot be proven."""

    called_goal = _as_goal(goal)
    goal_term = _goal_term_or_none(called_goal)

    if goal_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            (called_goal_term,) = args
            try:
                reified_goal = goal_from_term(_reified(called_goal_term, state))
            except TypeError:
                yield state
                return
            iterator = solve_from(program_value, reified_goal, state)
            yield next(iterator, state)

        return native_goal(run_terms, goal_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        iterator = solve_from(program_value, called_goal, state)
        yield next(iterator, state)

    return native_goal(run)


def noto(goal: object) -> GoalExpr:
    """Negation as failure.

    This succeeds once if `goal` has no solutions from the current state and
    fails if `goal` can be proven. It is operational negation, not classical
    logical negation.
    """

    called_goal = _as_goal(goal)
    goal_term = _goal_term_or_none(called_goal)

    if goal_term is not None:

        def run_terms(
            program_value: Program,
            state: State,
            args: NativeArgs,
        ) -> Iterator[State]:
            (called_goal_term,) = args
            try:
                reified_goal = goal_from_term(_reified(called_goal_term, state))
            except TypeError:
                return
            iterator = solve_from(program_value, reified_goal, state)
            if next(iterator, None) is None:
                yield state

        return native_goal(run_terms, goal_term)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        iterator = solve_from(program_value, called_goal, state)
        if next(iterator, None) is None:
            yield state

    return native_goal(run)


def groundo(term_value: object) -> GoalExpr:
    """Succeed when the current value of `term_value` is ground."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        yield from _succeed_if(_is_ground(_reified(target, state)), state)

    return native_goal(run, term_value)


def acyclic_termo(term_value: object) -> GoalExpr:
    """Succeed when the current value of ``term_value`` has no cycles."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        yield from _succeed_if(_is_acyclic_term(_reified(target, state)), state)

    return native_goal(run, term_value)


def cyclic_termo(term_value: object) -> GoalExpr:
    """Succeed when the current value of ``term_value`` contains a cycle."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        yield from _succeed_if(not _is_acyclic_term(_reified(target, state)), state)

    return native_goal(run, term_value)


def varo(term_value: object) -> GoalExpr:
    """Succeed when `term_value` is currently an unbound logic variable."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        yield from _succeed_if(isinstance(_reified(target, state), LogicVar), state)

    return native_goal(run, term_value)


def nonvaro(term_value: object) -> GoalExpr:
    """Succeed when `term_value` is not currently an unbound logic variable."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        yield from _succeed_if(not isinstance(_reified(target, state), LogicVar), state)

    return native_goal(run, term_value)


def _type_checko(term_value: object, expected_type: type[Term]) -> GoalExpr:
    """Build a state-aware type test for one reified term."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        reified_target = _reified(target, state)
        yield from _succeed_if(isinstance(reified_target, expected_type), state)

    return native_goal(run, term_value)


def atomo(term_value: object) -> GoalExpr:
    """Succeed when the current value is an atom."""

    return _type_checko(term_value, Atom)


def numbero(term_value: object) -> GoalExpr:
    """Succeed when the current value is a number."""

    return _type_checko(term_value, Number)


def integero(term_value: object) -> GoalExpr:
    """Succeed when the current value is a non-bool integer."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        yield from _succeed_if(_reified_integer(target, state) is not None, state)

    return native_goal(run, term_value)


def stringo(term_value: object) -> GoalExpr:
    """Succeed when the current value is a string term."""

    return _type_checko(term_value, String)


def _one_char_atoms_from_text(text: str) -> list[Atom]:
    """Return a Prolog char list representation for text."""

    return [atom(character) for character in text]


def _code_numbers_from_text(text: str) -> list[Number]:
    """Return a Prolog code list representation for text."""

    return [num(ord(character)) for character in text]


def _text_from_char_items(items: list[Term]) -> str | None:
    characters: list[str] = []
    for item in items:
        if (
            not isinstance(item, Atom)
            or item.symbol.namespace is not None
            or len(item.symbol.name) != 1
        ):
            return None
        characters.append(item.symbol.name)
    return "".join(characters)


def _text_from_code_items(items: list[Term]) -> str | None:
    characters: list[str] = []
    for item in items:
        if not isinstance(item, Number):
            return None
        value = item.value
        if not isinstance(value, int):
            return None
        try:
            characters.append(chr(value))
        except ValueError:
            return None
    return "".join(characters)


def _number_text(number_value: Number) -> str:
    """Render a number in a syntax that can be parsed back by Python."""

    return str(number_value.value)


def _parse_number_text(text: str) -> Number | None:
    if text == "":
        return None
    try:
        if any(marker in text for marker in (".", "e", "E")):
            return num(float(text))
        return num(int(text))
    except ValueError:
        return None


def _texto(
    scalar: object,
    pieces: object,
    *,
    scalar_type: type[Atom] | type[String],
    from_text: Callable[[str], list[Term]],
    to_text: Callable[[list[Term]], str | None],
) -> GoalExpr:
    """Relate a text-like scalar to either chars or character codes."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        scalar_term, pieces_term = args
        reified_scalar = _reified(scalar_term, state)
        if isinstance(reified_scalar, scalar_type):
            text = (
                reified_scalar.symbol.name
                if isinstance(reified_scalar, Atom)
                else reified_scalar.value
            )
            yield from solve_from(
                program_value,
                eq(pieces_term, logic_list(from_text(text))),
                state,
            )
            return

        if not isinstance(reified_scalar, LogicVar):
            return

        items = _proper_list_items(_reified(pieces_term, state))
        if items is None:
            return
        text = to_text(items)
        if text is None:
            return
        constructed = atom(text) if scalar_type is Atom else String(text)
        yield from solve_from(program_value, eq(scalar_term, constructed), state)

    return native_goal(run, scalar, pieces)


def atom_charso(atom_value: object, chars: object) -> GoalExpr:
    """Relate an atom to a proper list of one-character atoms."""

    return _texto(
        atom_value,
        chars,
        scalar_type=Atom,
        from_text=_one_char_atoms_from_text,
        to_text=_text_from_char_items,
    )


def atom_codeso(atom_value: object, codes: object) -> GoalExpr:
    """Relate an atom to a proper list of Unicode code numbers."""

    return _texto(
        atom_value,
        codes,
        scalar_type=Atom,
        from_text=_code_numbers_from_text,
        to_text=_text_from_code_items,
    )


def string_charso(string_value: object, chars: object) -> GoalExpr:
    """Relate a string term to a proper list of one-character atoms."""

    return _texto(
        string_value,
        chars,
        scalar_type=String,
        from_text=_one_char_atoms_from_text,
        to_text=_text_from_char_items,
    )


def string_codeso(string_value: object, codes: object) -> GoalExpr:
    """Relate a string term to a proper list of Unicode code numbers."""

    return _texto(
        string_value,
        codes,
        scalar_type=String,
        from_text=_code_numbers_from_text,
        to_text=_text_from_code_items,
    )


def _number_texto(
    number_value: object,
    pieces: object,
    *,
    from_text: Callable[[str], list[Term]],
    to_text: Callable[[list[Term]], str | None],
) -> GoalExpr:
    """Relate a number to chars or codes using finite, non-enumerating modes."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        number_term, pieces_term = args
        reified_number = _reified(number_term, state)
        if isinstance(reified_number, Number):
            yield from solve_from(
                program_value,
                eq(pieces_term, logic_list(from_text(_number_text(reified_number)))),
                state,
            )
            return

        if not isinstance(reified_number, LogicVar):
            return

        items = _proper_list_items(_reified(pieces_term, state))
        if items is None:
            return
        text = to_text(items)
        if text is None:
            return
        parsed = _parse_number_text(text)
        if parsed is None:
            return
        yield from solve_from(program_value, eq(number_term, parsed), state)

    return native_goal(run, number_value, pieces)


def number_charso(number_value: object, chars: object) -> GoalExpr:
    """Relate a number to a proper list of one-character atoms."""

    return _number_texto(
        number_value,
        chars,
        from_text=_one_char_atoms_from_text,
        to_text=_text_from_char_items,
    )


def number_codeso(number_value: object, codes: object) -> GoalExpr:
    """Relate a number to a proper list of Unicode code numbers."""

    return _number_texto(
        number_value,
        codes,
        from_text=_code_numbers_from_text,
        to_text=_text_from_code_items,
    )


def atom_numbero(atom_value: object, number_value: object) -> GoalExpr:
    """Relate an atom to a number parsed from or rendered as text."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        atom_term, number_term = args
        reified_atom = _reified(atom_term, state)
        reified_number = _reified(number_term, state)

        if isinstance(reified_number, Number):
            yield from solve_from(
                program_value,
                eq(atom_term, atom(_number_text(reified_number))),
                state,
            )
            return

        if not isinstance(reified_number, LogicVar):
            return
        text = _plain_atom_text(reified_atom)
        if text is None:
            return
        parsed = _parse_number_text(text)
        if parsed is None:
            return
        yield from solve_from(program_value, eq(number_term, parsed), state)

    return native_goal(run, atom_value, number_value)


def char_codeo(char_value: object, code_value: object) -> GoalExpr:
    """Relate a one-character atom to its Unicode code number."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        char_term, code_term = args
        reified_char = _reified(char_term, state)
        reified_code = _reified(code_term, state)

        if isinstance(reified_char, Atom):
            if reified_char.symbol.namespace is not None:
                return
            text = reified_char.symbol.name
            if len(text) != 1:
                return
            yield from solve_from(program_value, eq(code_term, num(ord(text))), state)
            return

        if not isinstance(reified_char, LogicVar):
            return
        if not isinstance(reified_code, Number):
            return
        raw_code = reified_code.value
        if not isinstance(raw_code, int):
            return
        try:
            character = chr(raw_code)
        except ValueError:
            return
        yield from solve_from(program_value, eq(char_term, atom(character)), state)

    return native_goal(run, char_value, code_value)


def _plain_atom_text(term_value: Term) -> str | None:
    if not isinstance(term_value, Atom) or term_value.symbol.namespace is not None:
        return None
    return term_value.symbol.name


def _atom_from_text(text: str) -> Atom | None:
    """Construct an atom unless the prototype cannot represent that text yet."""

    if text == "":
        return None
    return atom(text)


def _atomic_text(term_value: Term) -> str | None:
    if isinstance(term_value, Atom):
        return _plain_atom_text(term_value)
    if isinstance(term_value, String):
        return term_value.value
    if isinstance(term_value, Number):
        return _number_text(term_value)
    return None


def _non_negative_integer_term(term_value: Term, state: State) -> int | None:
    value = _reified_integer(term_value, state)
    if value is None or value < 0:
        return None
    return value


def _text_lengtho(
    scalar: object,
    length: object,
    *,
    scalar_type: type[Atom] | type[String],
) -> GoalExpr:
    """Relate a text-like scalar to its character length."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        scalar_term, length_term = args
        reified_scalar = _reified(scalar_term, state)
        if not isinstance(reified_scalar, scalar_type):
            return
        text = (
            reified_scalar.symbol.name
            if isinstance(reified_scalar, Atom)
            else reified_scalar.value
        )
        yield from solve_from(program_value, eq(length_term, num(len(text))), state)

    return native_goal(run, scalar, length)


def atom_lengtho(atom_value: object, length: object) -> GoalExpr:
    """Relate an atom to its character length."""

    return _text_lengtho(atom_value, length, scalar_type=Atom)


def string_lengtho(string_value: object, length: object) -> GoalExpr:
    """Relate a string term to its character length."""

    return _text_lengtho(string_value, length, scalar_type=String)


def _text_sliceo(
    text_value: object,
    before: object,
    length: object,
    after: object,
    sub_text: object,
    *,
    scalar_type: type[Atom] | type[String],
) -> GoalExpr:
    """Finite relation backing ``sub_atom/5`` and ``sub_string/5``."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        text_term, before_term, length_term, after_term, sub_text_term = args
        reified_text = _reified(text_term, state)
        reified_sub_text = _reified(sub_text_term, state)

        if isinstance(reified_text, scalar_type):
            text = (
                reified_text.symbol.name
                if isinstance(reified_text, Atom)
                else reified_text.value
            )
            yield from _run_bound_text_slices(
                program_value,
                state,
                text,
                before_term,
                length_term,
                after_term,
                sub_text_term,
                reified_sub_text,
                scalar_type=scalar_type,
            )
            return

        if not isinstance(reified_text, LogicVar):
            return
        if (
            _non_negative_integer_term(before_term, state) != 0
            or _non_negative_integer_term(after_term, state) != 0
            or not isinstance(reified_sub_text, scalar_type)
        ):
            return
        text = (
            reified_sub_text.symbol.name
            if isinstance(reified_sub_text, Atom)
            else reified_sub_text.value
        )
        constructed = _text_scalar_from_text(text, scalar_type)
        if constructed is None:
            return
        yield from solve_from(
            program_value,
            conj(eq(length_term, num(len(text))), eq(text_term, constructed)),
            state,
        )

    return native_goal(run, text_value, before, length, after, sub_text)


def _run_bound_text_slices(
    program_value: Program,
    state: State,
    text: str,
    before_term: Term,
    length_term: Term,
    after_term: Term,
    sub_text_term: Term,
    reified_sub_text: Term,
    *,
    scalar_type: type[Atom] | type[String],
) -> Iterator[State]:
    before_filter = _bound_non_negative_filter(before_term, state)
    length_filter = _bound_non_negative_filter(length_term, state)
    after_filter = _bound_non_negative_filter(after_term, state)
    sub_text_filter = _bound_text_filter(reified_sub_text, scalar_type)
    if (
        before_filter is False
        or length_filter is False
        or after_filter is False
        or sub_text_filter is False
    ):
        return

    text_length = len(text)
    for before_value in range(text_length + 1):
        for length_value in range(text_length - before_value + 1):
            after_value = text_length - before_value - length_value
            substring = text[before_value : before_value + length_value]
            if before_filter is not None and before_filter != before_value:
                continue
            if length_filter is not None and length_filter != length_value:
                continue
            if after_filter is not None and after_filter != after_value:
                continue
            if sub_text_filter is not None and sub_text_filter != substring:
                continue
            constructed_sub_text = _text_scalar_from_text(substring, scalar_type)
            if constructed_sub_text is None:
                continue
            yield from solve_from(
                program_value,
                conj(
                    eq(before_term, num(before_value)),
                    eq(length_term, num(length_value)),
                    eq(after_term, num(after_value)),
                    eq(sub_text_term, constructed_sub_text),
                ),
                state,
            )


def _bound_non_negative_filter(term_value: Term, state: State) -> int | bool | None:
    reified_value = _reified(term_value, state)
    if isinstance(reified_value, LogicVar):
        return None
    value = _integer_value(reified_value)
    if value is None or value < 0:
        return False
    return value


def _bound_text_filter(
    term_value: Term,
    scalar_type: type[Atom] | type[String],
) -> str | bool | None:
    if isinstance(term_value, LogicVar):
        return None
    if not isinstance(term_value, scalar_type):
        return False
    return term_value.symbol.name if isinstance(term_value, Atom) else term_value.value


def _text_scalar_from_text(
    text: str,
    scalar_type: type[Atom] | type[String],
) -> Atom | String | None:
    if scalar_type is Atom:
        return _atom_from_text(text)
    return String(text)


def sub_atomo(
    atom_value: object,
    before: object,
    length: object,
    after: object,
    sub_atom: object,
) -> GoalExpr:
    """Relate an atom to finite substring positions and a sub-atom."""

    return _text_sliceo(
        atom_value,
        before,
        length,
        after,
        sub_atom,
        scalar_type=Atom,
    )


def sub_stringo(
    string_value: object,
    before: object,
    length: object,
    after: object,
    sub_string: object,
) -> GoalExpr:
    """Relate a string term to finite substring positions and a substring."""

    return _text_sliceo(
        string_value,
        before,
        length,
        after,
        sub_string,
        scalar_type=String,
    )


def atom_concato(left: object, right: object, combined: object) -> GoalExpr:
    """Relate two atoms to their concatenation using finite modes."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term, combined_term = args
        reified_left = _reified(left_term, state)
        reified_right = _reified(right_term, state)
        reified_combined = _reified(combined_term, state)

        left_text = _plain_atom_text(reified_left)
        right_text = _plain_atom_text(reified_right)
        combined_text = _plain_atom_text(reified_combined)

        if left_text is not None and right_text is not None:
            joined = _atom_from_text(left_text + right_text)
            if joined is None:
                return
            yield from solve_from(
                program_value,
                eq(combined_term, joined),
                state,
            )
            return

        if combined_text is None:
            return

        if left_text is not None:
            if combined_text.startswith(left_text):
                suffix = _atom_from_text(combined_text[len(left_text) :])
                if suffix is None:
                    return
                yield from solve_from(
                    program_value,
                    eq(right_term, suffix),
                    state,
                )
            return

        if right_text is not None:
            if combined_text.endswith(right_text):
                prefix = _atom_from_text(combined_text[: -len(right_text) or None])
                if prefix is None:
                    return
                yield from solve_from(
                    program_value,
                    eq(left_term, prefix),
                    state,
                )
            return

        if not isinstance(reified_left, LogicVar) or not isinstance(
            reified_right,
            LogicVar,
        ):
            return

        for index in range(len(combined_text) + 1):
            left_atom = _atom_from_text(combined_text[:index])
            right_atom = _atom_from_text(combined_text[index:])
            if left_atom is None or right_atom is None:
                continue
            yield from solve_from(
                program_value,
                conj(
                    eq(left_term, left_atom),
                    eq(right_term, right_atom),
                ),
                state,
            )

    return native_goal(run, left, right, combined)


def atomic_list_concato(items: object, combined: object) -> GoalExpr:
    """Relate a proper list of atomic terms to their concatenated atom."""

    return _atomic_list_concato(items, "", combined)


def atomic_list_concato_with_separator(
    items: object,
    separator: object,
    combined: object,
) -> GoalExpr:
    """Relate atomic list items, a separator, and their concatenated atom."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        items_term, separator_term, combined_term = args
        reified_separator = _reified(separator_term, state)
        separator_text = _atomic_text(reified_separator)
        if separator_text is None:
            return
        yield from _run_atomic_list_concat(
            program_value,
            state,
            items_term,
            separator_text,
            combined_term,
            allow_split=True,
        )

    return native_goal(run, items, separator, combined)


def _atomic_list_concato(
    items: object,
    separator_text: str,
    combined: object,
) -> GoalExpr:
    """Build an atomic-list concatenation goal with a fixed separator."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        items_term, combined_term = args
        yield from _run_atomic_list_concat(
            program_value,
            state,
            items_term,
            separator_text,
            combined_term,
            allow_split=False,
        )

    return native_goal(run, items, combined)


def _run_atomic_list_concat(
    program_value: Program,
    state: State,
    items_term: Term,
    separator_text: str,
    combined_term: Term,
    *,
    allow_split: bool,
) -> Iterator[State]:
    reified_items = _reified(items_term, state)
    reified_combined = _reified(combined_term, state)

    list_items = _proper_list_items(reified_items)
    if list_items is not None:
        texts: list[str] = []
        for item in list_items:
            text = _atomic_text(item)
            if text is None:
                return
            texts.append(text)
        joined = _atom_from_text(separator_text.join(texts))
        if joined is None:
            return
        yield from solve_from(program_value, eq(combined_term, joined), state)
        return

    if not allow_split or not isinstance(reified_items, LogicVar):
        return

    combined_text = _plain_atom_text(reified_combined)
    if combined_text is None or separator_text == "":
        return

    parts: list[Atom] = []
    for text in combined_text.split(separator_text):
        part = _atom_from_text(text)
        if part is None:
            return
        parts.append(part)
    yield from solve_from(program_value, eq(items_term, logic_list(parts)), state)


def number_stringo(number_value: object, string_value: object) -> GoalExpr:
    """Relate a number to a string term containing its textual representation."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        number_term, string_term = args
        reified_number = _reified(number_term, state)
        reified_string = _reified(string_term, state)

        if isinstance(reified_number, Number):
            yield from solve_from(
                program_value,
                eq(string_term, String(_number_text(reified_number))),
                state,
            )
            return

        if not isinstance(reified_number, LogicVar):
            return
        if not isinstance(reified_string, String):
            return

        parsed = _parse_number_text(reified_string.value)
        if parsed is None:
            return
        yield from solve_from(program_value, eq(number_term, parsed), state)

    return native_goal(run, number_value, string_value)


def _path_text(term_value: Term) -> str | None:
    if isinstance(term_value, String):
        return term_value.value
    if isinstance(term_value, Atom) and term_value.symbol.namespace is None:
        return term_value.symbol.name
    return None


def _read_utf8_file(path_text: str) -> str | None:
    try:
        return Path(path_text).read_text(encoding="utf-8")
    except OSError:
        return None
    except UnicodeDecodeError:
        return None


def exists_fileo(path_value: object) -> GoalExpr:
    """Succeed when a bound atom/string path names an existing regular file."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        [path_term] = args
        path_text = _path_text(_reified(path_term, state))
        if path_text is None:
            return
        if Path(path_text).is_file():
            yield state

    return native_goal(run, path_value)


def read_file_to_stringo(path_value: object, contents: object) -> GoalExpr:
    """Relate a bound atom/string path to the file's UTF-8 contents as a string."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        path_term, contents_term = args
        path_text = _path_text(_reified(path_term, state))
        if path_text is None:
            return
        text = _read_utf8_file(path_text)
        if text is None:
            return
        yield from solve_from(program_value, eq(contents_term, String(text)), state)

    return native_goal(run, path_value, contents)


def read_file_to_codeso(path_value: object, codes: object) -> GoalExpr:
    """Relate a bound atom/string path to the file's UTF-8 code-point list."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        path_term, codes_term = args
        path_text = _path_text(_reified(path_term, state))
        if path_text is None:
            return
        text = _read_utf8_file(path_text)
        if text is None:
            return
        yield from solve_from(
            program_value,
            eq(codes_term, logic_list([num(ord(character)) for character in text])),
            state,
        )

    return native_goal(run, path_value, codes)


def _mode_text(term_value: Term) -> str | None:
    text = _plain_atom_text(term_value)
    if text in {"read", "write", "append"}:
        return text
    return None


def _stream_handle_text(term_value: Term) -> str | None:
    text = _plain_atom_text(term_value)
    if text is None or not text.startswith("$stream_"):
        return None
    return text


def _stream_alias_text(term_value: Term) -> str | None:
    text = _plain_atom_text(term_value)
    if text is None or text.startswith("$stream_"):
        return None
    return text


def _stream_key(term_value: Term) -> str | None:
    handle_text = _stream_handle_text(term_value)
    if handle_text is not None and handle_text in _STREAMS:
        return handle_text
    alias_text = _stream_alias_text(term_value)
    if alias_text is not None:
        return _STREAM_ALIASES.get(alias_text)
    return None


def _stream_handle() -> Atom:
    return atom(f"$stream_{next(_STREAM_IDS)}")


def _open_text_stream(
    path_text: str,
    mode_text: str,
    *,
    alias_text: str | None = None,
) -> _TextStream | None:
    path = Path(path_text)
    try:
        if mode_text == "read":
            if not path.is_file():
                return None
            return _TextStream(
                mode=mode_text,
                path=path,
                contents=path.read_text(encoding="utf-8"),
                alias=alias_text,
            )
        if mode_text == "write":
            path.write_text("", encoding="utf-8")
            return _TextStream(mode=mode_text, path=path, alias=alias_text)
        if mode_text == "append":
            path.parent.mkdir(parents=True, exist_ok=True)
            path.open("a", encoding="utf-8").close()
            return _TextStream(mode=mode_text, path=path, alias=alias_text)
    except OSError:
        return None
    except UnicodeDecodeError:
        return None
    return None


def _open_options(term_value: Term) -> tuple[str | None] | None:
    items = _proper_list_items(term_value)
    if items is None:
        return None

    alias_text: str | None = None
    for item in items:
        if not isinstance(item, Compound) or len(item.args) != 1:
            return None
        option_name = item.functor.name if item.functor.namespace is None else None
        option_arg = item.args[0]

        if option_name == "alias":
            parsed_alias = _stream_alias_text(option_arg)
            if parsed_alias is None:
                return None
            alias_text = parsed_alias
            continue

        if option_name == "encoding":
            encoding_text = _plain_atom_text(option_arg)
            if encoding_text not in {"utf8", "utf-8"}:
                return None
            continue

        if option_name == "type":
            if _plain_atom_text(option_arg) != "text":
                return None
            continue

        return None

    return (alias_text,)


def _register_stream(handle: Atom, stream: _TextStream) -> bool:
    handle_text = handle.symbol.name
    if stream.alias is not None:
        existing = _STREAM_ALIASES.get(stream.alias)
        if existing is not None and existing in _STREAMS:
            return False
        _STREAM_ALIASES[stream.alias] = handle_text
    _STREAMS[handle_text] = stream
    return True


def openo(path_value: object, mode_value: object, stream_value: object) -> GoalExpr:
    """Open a bounded UTF-8 file stream in read, write, or append mode."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        path_term, mode_term, stream_term = args
        path_text = _path_text(_reified(path_term, state))
        mode_text = _mode_text(_reified(mode_term, state))
        if path_text is None or mode_text is None:
            return

        stream = _open_text_stream(path_text, mode_text)
        if stream is None:
            return

        handle = _stream_handle()
        _register_stream(handle, stream)
        yield from solve_from(program_value, eq(stream_term, handle), state)

    return native_goal(run, path_value, mode_value, stream_value)


def open_optionso(
    path_value: object,
    mode_value: object,
    stream_value: object,
    options_value: object,
) -> GoalExpr:
    """Open a bounded UTF-8 file stream with a finite option-list subset."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        path_term, mode_term, stream_term, options_term = args
        path_text = _path_text(_reified(path_term, state))
        mode_text = _mode_text(_reified(mode_term, state))
        parsed_options = _open_options(_reified(options_term, state))
        if path_text is None or mode_text is None or parsed_options is None:
            return

        [alias_text] = parsed_options
        stream = _open_text_stream(path_text, mode_text, alias_text=alias_text)
        if stream is None:
            return

        handle = _stream_handle()
        if not _register_stream(handle, stream):
            return
        yield from solve_from(program_value, eq(stream_term, handle), state)

    return native_goal(run, path_value, mode_value, stream_value, options_value)


def closeo(stream_value: object) -> GoalExpr:
    """Close a bounded stream handle created by ``openo/3``."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        [stream_term] = args
        handle_text = _stream_key(_reified(stream_term, state))
        if handle_text is None or handle_text not in _STREAMS:
            return
        stream = _STREAMS.pop(handle_text)
        if stream.alias is not None:
            _STREAM_ALIASES.pop(stream.alias, None)
        yield state

    return native_goal(run, stream_value)


def _read_stream(term_value: Term) -> _TextStream | None:
    handle_text = _stream_key(term_value)
    if handle_text is None:
        return None
    stream = _STREAMS.get(handle_text)
    if stream is None or stream.mode != "read":
        return None
    return stream


def at_end_of_streamo(stream_value: object) -> GoalExpr:
    """Succeed when a bounded read stream has consumed all available text."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        [stream_term] = args
        stream = _read_stream(_reified(stream_term, state))
        if stream is not None and stream.cursor >= len(stream.contents):
            yield state

    return native_goal(run, stream_value)


def read_stringo(
    stream_value: object,
    length_value: object,
    string_value: object,
) -> GoalExpr:
    """Read up to ``Length`` UTF-8 code points from a bounded read stream."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        stream_term, length_term, string_term = args
        stream = _read_stream(_reified(stream_term, state))
        length = _reified_integer(length_term, state)
        if stream is None or length is None or length < 0:
            return
        start = stream.cursor
        end = min(start + length, len(stream.contents))
        stream.cursor = end
        yield from solve_from(
            program_value,
            eq(string_term, String(stream.contents[start:end])),
            state,
        )

    return native_goal(run, stream_value, length_value, string_value)


def read_line_to_stringo(stream_value: object, string_value: object) -> GoalExpr:
    """Read one line from a bounded read stream as a string or ``end_of_file``."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        stream_term, string_term = args
        stream = _read_stream(_reified(stream_term, state))
        if stream is None:
            return

        if stream.cursor >= len(stream.contents):
            yield from solve_from(
                program_value,
                eq(string_term, atom("end_of_file")),
                state,
            )
            return

        newline_index = stream.contents.find("\n", stream.cursor)
        if newline_index == -1:
            line = stream.contents[stream.cursor :]
            stream.cursor = len(stream.contents)
        else:
            line = stream.contents[stream.cursor : newline_index]
            stream.cursor = newline_index + 1
        yield from solve_from(program_value, eq(string_term, String(line)), state)

    return native_goal(run, stream_value, string_value)


def get_charo(stream_value: object, char_value: object) -> GoalExpr:
    """Read one character atom from a bounded read stream or ``end_of_file``."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        stream_term, char_term = args
        stream = _read_stream(_reified(stream_term, state))
        if stream is None:
            return

        if stream.cursor >= len(stream.contents):
            yield from solve_from(
                program_value,
                eq(char_term, atom("end_of_file")),
                state,
            )
            return

        character = stream.contents[stream.cursor]
        stream.cursor += 1
        yield from solve_from(program_value, eq(char_term, atom(character)), state)

    return native_goal(run, stream_value, char_value)


def _stream_write_text(term_value: Term) -> str | None:
    if isinstance(term_value, String):
        return term_value.value
    if isinstance(term_value, Atom):
        return _plain_atom_text(term_value)
    if isinstance(term_value, Number):
        return _number_text(term_value)
    if isinstance(term_value, Compound):
        return str(term_value)
    return None


def _write_stream(term_value: Term, text: str) -> bool:
    handle_text = _stream_key(term_value)
    if handle_text is None:
        return False
    stream = _STREAMS.get(handle_text)
    if stream is None or stream.mode not in {"write", "append"}:
        return False
    try:
        with stream.path.open("a", encoding="utf-8") as file:
            file.write(text)
    except OSError:
        return False
    return True


def writeo(stream_value: object, term_value: object) -> GoalExpr:
    """Write a bounded textual representation to a write/append stream."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        stream_term, term_term = args
        text = _stream_write_text(_reified(term_term, state))
        if text is None or not _write_stream(_reified(stream_term, state), text):
            return
        yield state

    return native_goal(run, stream_value, term_value)


def nlo(stream_value: object) -> GoalExpr:
    """Write a newline to a write/append stream."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        [stream_term] = args
        if _write_stream(_reified(stream_term, state), "\n"):
            yield state

    return native_goal(run, stream_value)


def flush_outputo(stream_value: object) -> GoalExpr:
    """Validate a write/append stream handle; writes are already flushed."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        [stream_term] = args
        handle_text = _stream_key(_reified(stream_term, state))
        stream = _STREAMS.get(handle_text) if handle_text is not None else None
        if stream is not None and stream.mode in {"write", "append"}:
            yield state

    return native_goal(run, stream_value)


def current_streamo(
    path_value: object,
    mode_value: object,
    stream_value: object,
) -> GoalExpr:
    """Enumerate currently open bounded streams as path, mode, and handle."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        path_term, mode_term, stream_term = args
        for handle_text, stream in tuple(_STREAMS.items()):
            yield from solve_from(
                program_value,
                conj(
                    eq(path_term, atom(str(stream.path))),
                    eq(mode_term, atom(stream.mode)),
                    eq(stream_term, atom(handle_text)),
                ),
                state,
            )

    return native_goal(run, path_value, mode_value, stream_value)


def _stream_properties(handle_text: str, stream: _TextStream) -> tuple[Term, ...]:
    properties: list[Term] = [
        term("file_name", atom(str(stream.path))),
        term("mode", atom(stream.mode)),
        term("position", num(stream.cursor)),
    ]
    if stream.mode == "read":
        properties.append(atom("input"))
        eof_state = "at" if stream.cursor >= len(stream.contents) else "not"
        properties.append(term("end_of_stream", atom(eof_state)))
    else:
        properties.append(atom("output"))
    if stream.alias is not None:
        properties.append(term("alias", atom(stream.alias)))
    properties.append(term("handle", atom(handle_text)))
    return tuple(properties)


def stream_propertyo(stream_value: object, property_value: object) -> GoalExpr:
    """Relate an open bounded stream handle or alias to finite metadata."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        stream_term, property_term = args
        reified_stream = _reified(stream_term, state)
        handle_text = _stream_key(reified_stream)

        streams: tuple[tuple[str, _TextStream], ...]
        if handle_text is not None:
            stream = _STREAMS.get(handle_text)
            streams = () if stream is None else ((handle_text, stream),)
        elif isinstance(reified_stream, LogicVar):
            streams = tuple(_STREAMS.items())
        else:
            return

        for candidate_handle, stream in streams:
            for property_candidate in _stream_properties(candidate_handle, stream):
                stream_goal = (
                    eq(stream_term, atom(candidate_handle))
                    if isinstance(reified_stream, LogicVar)
                    else succeed()
                )
                yield from solve_from(
                    program_value,
                    conj(
                        stream_goal,
                        eq(property_term, property_candidate),
                    ),
                    state,
                )

    return native_goal(run, stream_value, property_value)


def compoundo(term_value: object) -> GoalExpr:
    """Succeed when the current value is a compound term."""

    return _type_checko(term_value, Compound)


def atomico(term_value: object) -> GoalExpr:
    """Succeed when the current value is an atomic term."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        reified_target = _reified(target, state)
        yield from _succeed_if(
            isinstance(reified_target, Atom | Number | String),
            state,
        )

    return native_goal(run, term_value)


def callableo(term_value: object) -> GoalExpr:
    """Succeed when the current value can represent a callable term."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (target,) = args
        reified_target = _reified(target, state)
        yield from _succeed_if(
            isinstance(reified_target, Atom | Compound),
            state,
        )

    return native_goal(run, term_value)


def functoro(term_value: object, name: object, arity: object) -> GoalExpr:
    """Inspect or construct a term from a functor name and arity."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        target, name_target, arity_target = args
        reified_target = _reified(target, state)

        if isinstance(reified_target, Compound):
            goal = conj(
                eq(name_target, atom(reified_target.functor)),
                eq(arity_target, num(len(reified_target.args))),
            )
            yield from solve_from(program_value, goal, state)
            return

        if isinstance(reified_target, Atom | Number | String):
            yield from solve_from(
                program_value,
                conj(eq(name_target, reified_target), eq(arity_target, num(0))),
                state,
            )
            return

        reified_name = _reified(name_target, state)
        reified_arity = _reified(arity_target, state)
        if not isinstance(reified_arity, Number):
            return
        raw_arity = reified_arity.value
        if not isinstance(raw_arity, int) or raw_arity < 0:
            return

        if raw_arity == 0:
            if not isinstance(reified_name, Atom | Number | String):
                return
            yield from solve_from(program_value, eq(target, reified_name), state)
            return

        if not isinstance(reified_name, Atom):
            return

        arguments = tuple(
            LogicVar(id=state.next_var_id + offset)
            for offset in range(raw_arity)
        )
        constructed = Compound(functor=reified_name.symbol, args=arguments)
        construction_state = _state_with_next_var_id(
            state,
            state.next_var_id + raw_arity,
        )
        yield from solve_from(
            program_value,
            eq(target, constructed),
            construction_state,
        )

    return native_goal(run, term_value, name, arity)


def compound_name_argumentso(
    term_value: object,
    name: object,
    arguments: object,
) -> GoalExpr:
    """Inspect or construct a compound from its functor name and arguments."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        target, name_target, arguments_target = args
        reified_target = _reified(target, state)

        if isinstance(reified_target, Compound):
            goal = conj(
                eq(name_target, atom(reified_target.functor)),
                eq(arguments_target, logic_list(list(reified_target.args))),
            )
            yield from solve_from(program_value, goal, state)
            return

        if not isinstance(reified_target, LogicVar):
            return

        reified_name = _reified(name_target, state)
        reified_arguments = _reified(arguments_target, state)
        items = _proper_list_items(reified_arguments)
        if not isinstance(reified_name, Atom) or not items:
            return

        constructed = Compound(functor=reified_name.symbol, args=tuple(items))
        yield from solve_from(program_value, eq(target, constructed), state)

    return native_goal(run, term_value, name, arguments)


def compound_name_arityo(term_value: object, name: object, arity: object) -> GoalExpr:
    """Inspect or construct a compound from its functor name and arity."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        target, name_target, arity_target = args
        reified_target = _reified(target, state)

        if isinstance(reified_target, Compound):
            goal = conj(
                eq(name_target, atom(reified_target.functor)),
                eq(arity_target, num(len(reified_target.args))),
            )
            yield from solve_from(program_value, goal, state)
            return

        if not isinstance(reified_target, LogicVar):
            return

        reified_name = _reified(name_target, state)
        raw_arity = _reified_integer(arity_target, state)
        if not isinstance(reified_name, Atom) or raw_arity is None or raw_arity <= 0:
            return

        arguments, next_var_id = _fresh_logic_vars(raw_arity, state.next_var_id)
        constructed = Compound(functor=reified_name.symbol, args=arguments)
        construction_state = _state_with_next_var_id(state, next_var_id)
        yield from solve_from(
            program_value,
            eq(target, constructed),
            construction_state,
        )

    return native_goal(run, term_value, name, arity)


def argo(index: object, term_value: object, value: object) -> GoalExpr:
    """Inspect a 1-based compound argument."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        index_term, compound_term, value_term = args
        reified_index = _reified(index_term, state)
        reified_compound = _reified(compound_term, state)
        if not isinstance(reified_index, Number):
            return
        if not isinstance(reified_compound, Compound):
            return

        raw_index = reified_index.value
        if not isinstance(raw_index, int) or raw_index <= 0:
            return
        if raw_index > len(reified_compound.args):
            return

        yield from solve_from(
            program_value,
            eq(value_term, reified_compound.args[raw_index - 1]),
            state,
        )

    return native_goal(run, index, term_value, value)


def univo(term_value: object, parts: object) -> GoalExpr:
    """Decompose or construct a term using a Prolog-style functor-first list."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        target, parts_target = args
        reified_target = _reified(target, state)

        if isinstance(reified_target, Compound):
            decomposed = logic_list(
                [atom(reified_target.functor), *reified_target.args],
            )
            yield from solve_from(program_value, eq(parts_target, decomposed), state)
            return

        if isinstance(reified_target, Atom | Number | String):
            yield from solve_from(
                program_value,
                eq(parts_target, logic_list([reified_target])),
                state,
            )
            return

        reified_parts = _reified(parts_target, state)
        items = _proper_list_items(reified_parts)
        if not items:
            return

        if len(items) == 1:
            yield from solve_from(program_value, eq(target, items[0]), state)
            return

        functor_term = items[0]
        if not isinstance(functor_term, Atom):
            return

        constructed = Compound(functor=functor_term.symbol, args=tuple(items[1:]))
        yield from solve_from(program_value, eq(target, constructed), state)

    return native_goal(run, term_value, parts)


def copytermo(source: object, copy: object) -> GoalExpr:
    """Copy a term while replacing source variables with fresh variables."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        source_term, copy_target = args
        copied_source, next_var_id = _fresh_copy(
            _reified(source_term, state),
            {},
            state.next_var_id,
        )
        copy_state = _state_with_next_var_id(state, next_var_id)
        yield from solve_from(program_value, eq(copy_target, copied_source), copy_state)

    return native_goal(run, source, copy)


def _term_variables_in_order(term_value: Term) -> tuple[LogicVar, ...]:
    """Collect unique variables in first left-to-right occurrence order."""

    seen: set[LogicVar] = set()
    ordered: list[LogicVar] = []

    def visit(current: Term) -> None:
        if isinstance(current, LogicVar):
            if current not in seen:
                seen.add(current)
                ordered.append(current)
            return
        if isinstance(current, Compound):
            for argument in current.args:
                visit(argument)

    visit(term_value)
    return tuple(ordered)


def term_variableso(term_value: object, variables: object) -> GoalExpr:
    """Unify ``variables`` with unique variables in a reified term."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        source_term, variables_target = args
        ordered_variables = _term_variables_in_order(_reified(source_term, state))
        yield from solve_from(
            program_value,
            eq(variables_target, logic_list(ordered_variables)),
            state,
        )

    return native_goal(run, term_value, variables)


def unify_with_occurs_checko(left: object, right: object) -> GoalExpr:
    """Unify two terms using the engine's finite-term occurs check."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        yield from solve_from(program_value, eq(left_term, right_term), state)

    return native_goal(run, left, right)


def _unifier_equations(
    left: Term,
    right: Term,
    before: State,
    after: State,
) -> list[Term]:
    """Return first-occurrence equations added by a non-binding unifiability check."""

    equations: list[Term] = []
    for variable in _term_variables_in_order(term("$unifiable", left, right)):
        if before.substitution.walk(variable) != variable:
            continue
        value = reify(variable, after.substitution)
        if value != variable:
            equations.append(term("=", variable, value))
    return equations


def unifiableo(left: object, right: object, unifier: object) -> GoalExpr:
    """Describe how two terms can unify without binding the source terms."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term, unifier_target = args
        unified_state = next(
            solve_from(program_value, eq(left_term, right_term), state),
            None,
        )
        if unified_state is None:
            return

        equations = _unifier_equations(
            _reified(left_term, state),
            _reified(right_term, state),
            state,
            unified_state,
        )
        yield from solve_from(
            program_value,
            eq(unifier_target, logic_list(equations)),
            state,
        )

    return native_goal(run, left, right, unifier)


def numbervarso(term_value: object, start: object, end: object) -> GoalExpr:
    """Bind variables in ``term_value`` to ``'$VAR'(N)`` placeholders."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        source_term, start_term, end_target = args
        start_index = _reified_integer(start_term, state)
        if start_index is None or start_index < 0:
            return

        variables = _term_variables_in_order(_reified(source_term, state))
        goals = [
            eq(variable, term("$VAR", start_index + offset))
            for offset, variable in enumerate(variables)
        ]
        goals.append(eq(end_target, start_index + len(variables)))
        yield from solve_from(program_value, conj(*goals), state)

    return native_goal(run, term_value, start, end)


def _term_hash_key(
    term_value: Term,
    depth_limit: int | None,
    variables: dict[LogicVar, int],
    depth: int,
) -> TermHashKey:
    """Return a deterministic, variant-aware structural key for a term."""

    if depth_limit is not None and depth >= depth_limit:
        return ("depth",)
    if isinstance(term_value, LogicVar):
        index = variables.setdefault(term_value, len(variables))
        return ("var", index)
    if isinstance(term_value, Number):
        return ("number", type(term_value.value).__name__, term_value.value)
    if isinstance(term_value, Atom):
        return ("atom", term_value.symbol.namespace or "", term_value.symbol.name)
    if isinstance(term_value, String):
        return ("string", term_value.value)
    return (
        "compound",
        term_value.functor.namespace or "",
        term_value.functor.name,
        len(term_value.args),
        tuple(
            _term_hash_key(argument, depth_limit, variables, depth + 1)
            for argument in term_value.args
        ),
    )


def _term_hash_value(term_value: Term, depth_limit: int | None, range_size: int) -> int:
    """Hash a term key to a stable non-negative integer within ``range_size``."""

    key = _term_hash_key(term_value, depth_limit, {}, 0)
    digest = blake2b(repr(key).encode("utf-8"), digest_size=8).digest()
    return int.from_bytes(digest, "big") % range_size


def term_hash_boundedo(
    term_value: object,
    depth: object,
    range_value: object,
    hash_value: object,
) -> GoalExpr:
    """Unify ``hash_value`` with a depth/range-bounded structural term hash."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        source_term, depth_term, range_term, hash_target = args
        depth_limit = _reified_integer(depth_term, state)
        range_size = _reified_integer(range_term, state)
        if depth_limit is None or range_size is None:
            return
        if depth_limit < 0 or range_size <= 0:
            return

        hashed = _term_hash_value(
            _reified(source_term, state),
            depth_limit,
            range_size,
        )
        yield from solve_from(program_value, eq(hash_target, hashed), state)

    return native_goal(run, term_value, depth, range_value, hash_value)


def term_hasho(term_value: object, hash_value: object) -> GoalExpr:
    """Unify ``hash_value`` with a deterministic structural term hash."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        source_term, hash_target = args
        hashed = _term_hash_value(
            _reified(source_term, state),
            None,
            _DEFAULT_TERM_HASH_RANGE,
        )
        yield from solve_from(program_value, eq(hash_target, hashed), state)

    return native_goal(run, term_value, hash_value)


def same_termo(left: object, right: object) -> GoalExpr:
    """Succeed when two reified terms are strictly identical without binding."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        yield from _succeed_if(
            _reified(left_term, state) == _reified(right_term, state),
            state,
        )

    return native_goal(run, left, right)


def not_same_termo(left: object, right: object) -> GoalExpr:
    """Succeed when two reified terms are not strictly identical without binding."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        yield from _succeed_if(
            _reified(left_term, state) != _reified(right_term, state),
            state,
        )

    return native_goal(run, left, right)


def _variant_terms(
    left: Term,
    right: Term,
    left_to_right: dict[LogicVar, LogicVar],
    right_to_left: dict[LogicVar, LogicVar],
) -> bool:
    """Return True when two terms differ only by variable renaming."""

    if isinstance(left, LogicVar) and isinstance(right, LogicVar):
        mapped_right = left_to_right.get(left)
        mapped_left = right_to_left.get(right)
        if mapped_right is None and mapped_left is None:
            left_to_right[left] = right
            right_to_left[right] = left
            return True
        return mapped_right == right and mapped_left == left

    if isinstance(left, LogicVar) or isinstance(right, LogicVar):
        return False
    if isinstance(left, Compound) and isinstance(right, Compound):
        return (
            left.functor == right.functor
            and len(left.args) == len(right.args)
            and all(
                _variant_terms(
                    left_arg,
                    right_arg,
                    left_to_right,
                    right_to_left,
                )
                for left_arg, right_arg in zip(left.args, right.args, strict=True)
            )
        )
    return left == right


def _subsumes_term(
    general: Term,
    specific: Term,
    bindings: dict[LogicVar, Term],
) -> bool:
    """Return True when ``specific`` is an instance of ``general``."""

    if isinstance(general, LogicVar):
        previous = bindings.get(general)
        if previous is None:
            bindings[general] = specific
            return True
        return previous == specific

    if isinstance(general, Compound):
        return (
            isinstance(specific, Compound)
            and general.functor == specific.functor
            and len(general.args) == len(specific.args)
            and all(
                _subsumes_term(general_arg, specific_arg, bindings)
                for general_arg, specific_arg in zip(
                    general.args,
                    specific.args,
                    strict=True,
                )
            )
        )
    return general == specific


def variant_termo(left: object, right: object) -> GoalExpr:
    """Succeed when two reified terms are variants of each other."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        yield from _succeed_if(
            _variant_terms(
                _reified(left_term, state),
                _reified(right_term, state),
                {},
                {},
            ),
            state,
        )

    return native_goal(run, left, right)


def not_variant_termo(left: object, right: object) -> GoalExpr:
    """Succeed when two reified terms are not variants of each other."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        yield from _succeed_if(
            not _variant_terms(
                _reified(left_term, state),
                _reified(right_term, state),
                {},
                {},
            ),
            state,
        )

    return native_goal(run, left, right)


def subsumes_termo(general: object, specific: object) -> GoalExpr:
    """Succeed when ``specific`` is an instance of ``general``."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        general_term, specific_term = args
        yield from _succeed_if(
            _subsumes_term(
                _reified(general_term, state),
                _reified(specific_term, state),
                {},
            ),
            state,
        )

    return native_goal(run, general, specific)


def difo(left: object, right: object) -> GoalExpr:
    """Enforce delayed disequality between two terms."""

    return neq(left, right)


def compare_termo(order: object, left: object, right: object) -> GoalExpr:
    """Unify `order` with `<`, `=`, or `>` using standard term ordering."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        order_target, left_term, right_term = args
        comparison = _compare_terms(
            _reified(left_term, state),
            _reified(right_term, state),
        )
        order_atom = atom("<" if comparison < 0 else ">" if comparison > 0 else "=")
        yield from solve_from(program_value, eq(order_target, order_atom), state)

    return native_goal(run, order, left, right)


def _term_compareo(
    left: object,
    right: object,
    predicate: Callable[[int], bool],
) -> GoalExpr:
    """Build a non-binding standard-term-order comparison predicate."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        left_term, right_term = args
        comparison = _compare_terms(
            _reified(left_term, state),
            _reified(right_term, state),
        )
        yield from _succeed_if(predicate(comparison), state)

    return native_goal(run, left, right)


def termo_lto(left: object, right: object) -> GoalExpr:
    """Succeed when `left` is before `right` in standard term order."""

    return _term_compareo(left, right, lambda comparison: comparison < 0)


def termo_leqo(left: object, right: object) -> GoalExpr:
    """Succeed when `left` is not after `right` in standard term order."""

    return _term_compareo(left, right, lambda comparison: comparison <= 0)


def termo_gto(left: object, right: object) -> GoalExpr:
    """Succeed when `left` is after `right` in standard term order."""

    return _term_compareo(left, right, lambda comparison: comparison > 0)


def termo_geqo(left: object, right: object) -> GoalExpr:
    """Succeed when `left` is not before `right` in standard term order."""

    return _term_compareo(left, right, lambda comparison: comparison >= 0)


def _relation_from_indicator(name_term: Term, arity_term: Term) -> Relation | None:
    """Convert reified name/arity terms into an engine relation."""

    if not isinstance(name_term, Atom) or not isinstance(arity_term, Number):
        return None
    raw_arity = arity_term.value
    if isinstance(raw_arity, bool) or not isinstance(raw_arity, int) or raw_arity < 0:
        return None
    return relation(name_term.symbol, raw_arity)


def dynamico(name: object, arity: object) -> GoalExpr:
    """Declare a predicate dynamic in the current proof branch."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_term, arity_term = args
        relation_value = _relation_from_indicator(
            _reified(name_term, state),
            _reified(arity_term, state),
        )
        if relation_value is None:
            return
        updated_state = runtime_declare_dynamic(program_value, state, relation_value)
        if updated_state is not None:
            yield updated_state

    return native_goal(run, name, arity)


def _dynamic_clause_from_arg(clause_term: Term, state: State) -> Clause | None:
    """Parse one reified assertion/retraction argument into a clause."""

    try:
        return clause_from_term(_reified(clause_term, state))
    except TypeError:
        return None


def assertao(clause_term: object) -> GoalExpr:
    """Assert a dynamic clause before existing dynamic clauses in this branch."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (raw_clause_term,) = args
        clause_value = _dynamic_clause_from_arg(raw_clause_term, state)
        if clause_value is None:
            return
        updated_state = runtime_asserta(program_value, state, clause_value)
        if updated_state is not None:
            yield updated_state

    return native_goal(run, _as_callable_term(clause_term))


def assertzo(clause_term: object) -> GoalExpr:
    """Assert a dynamic clause after existing dynamic clauses in this branch."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (raw_clause_term,) = args
        clause_value = _dynamic_clause_from_arg(raw_clause_term, state)
        if clause_value is None:
            return
        updated_state = runtime_assertz(program_value, state, clause_value)
        if updated_state is not None:
            yield updated_state

    return native_goal(run, _as_callable_term(clause_term))


def retracto(clause_term: object) -> GoalExpr:
    """Retract the first matching dynamic clause and expose its bindings."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (raw_clause_term,) = args
        clause_value = _dynamic_clause_from_arg(raw_clause_term, state)
        if clause_value is None:
            return
        yield from runtime_retract_first(program_value, state, clause_value)

    return native_goal(run, _as_callable_term(clause_term))


def retractallo(head_term: object) -> GoalExpr:
    """Retract every dynamic clause whose head matches ``head_term``."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (raw_head_term,) = args
        try:
            clause_value = clause_from_term(_reified(raw_head_term, state))
        except TypeError:
            return
        updated_state = runtime_retract_all(program_value, state, clause_value.head)
        if updated_state is not None:
            yield updated_state

    return native_goal(run, _as_callable_term(head_term))


def abolisho(name: object, arity: object) -> GoalExpr:
    """Abolish a dynamic predicate in the current proof branch."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_term, arity_term = args
        relation_value = _relation_from_indicator(
            _reified(name_term, state),
            _reified(arity_term, state),
        )
        if relation_value is None:
            return
        updated_state = runtime_abolish(program_value, state, relation_value)
        if updated_state is not None:
            yield updated_state

    return native_goal(run, name, arity)


def _predicate_clause_counts(
    program_value: Program,
    state: State | None = None,
) -> dict[tuple[Atom, int], int]:
    """Count source clauses by predicate indicator in source-discovery order."""

    counts: dict[tuple[Atom, int], int] = {}
    for source_clause in visible_clauses(program_value, state):
        indicator = (
            atom(source_clause.head.relation.symbol),
            source_clause.head.relation.arity,
        )
        counts[indicator] = counts.get(indicator, 0) + 1
    return counts


def _builtin_indicators() -> tuple[tuple[Atom, int], ...]:
    """Return builtin predicate indicators in stable documentation order."""

    return tuple((atom(name), arity) for name, arity in _BUILTIN_PREDICATES)


def _predicate_indicators(
    program_value: Program,
    state: State | None = None,
) -> tuple[tuple[Atom, int], ...]:
    """Enumerate source and builtin predicate indicators without duplicates."""

    ordered: dict[tuple[Atom, int], None] = {}
    for key in visible_predicate_keys(program_value, state):
        ordered[(atom(key[0]), key[1])] = None
    for indicator in _predicate_clause_counts(program_value, state):
        ordered[indicator] = None
    for indicator in _builtin_indicators():
        ordered.setdefault(indicator, None)
    return tuple(ordered)


def _predicate_properties(
    program_value: Program,
    state: State,
    name_atom: Atom,
    arity: int,
) -> tuple[Term, ...]:
    """Return observable properties for one predicate indicator."""

    relation_value = relation(name_atom.symbol, arity)
    clause_count = visible_clause_count(program_value, relation_value, state)
    is_builtin = (name_atom, arity) in set(_builtin_indicators())
    is_dynamic = is_dynamic_relation(program_value, relation_value, state)
    if clause_count == 0 and not is_builtin and not is_dynamic:
        return ()

    properties: list[Term] = [atom("defined")]
    if is_dynamic:
        properties.append(atom("dynamic"))
    elif clause_count > 0:
        properties.append(atom("static"))
    if is_builtin:
        properties.append(atom("built_in"))
    properties.append(term("number_of_clauses", num(clause_count)))
    return tuple(properties)


def _remember_atom(ordered: dict[Atom, None], atom_value: Atom) -> None:
    ordered.setdefault(atom_value, None)


def _remember_functor(
    ordered: dict[tuple[Atom, int], None],
    name: Atom,
    arity: int,
) -> None:
    ordered.setdefault((name, arity), None)


def _remember_term_atoms(ordered: dict[Atom, None], term_value: Term) -> None:
    if isinstance(term_value, Atom):
        _remember_atom(ordered, term_value)
        return
    if isinstance(term_value, Compound):
        _remember_atom(ordered, atom(term_value.functor))
        for argument in term_value.args:
            _remember_term_atoms(ordered, argument)


def _remember_goal_atoms(ordered: dict[Atom, None], goal_value: GoalExpr) -> None:
    if isinstance(goal_value, RelationCall):
        _remember_atom(ordered, atom(goal_value.relation.symbol))
        for argument in goal_value.args:
            _remember_term_atoms(ordered, argument)
        return
    if isinstance(goal_value, EqExpr | NeqExpr):
        _remember_term_atoms(ordered, goal_value.left)
        _remember_term_atoms(ordered, goal_value.right)
        return
    if isinstance(goal_value, ConjExpr | DisjExpr):
        for child in goal_value.goals:
            _remember_goal_atoms(ordered, child)
        return
    if isinstance(goal_value, FreshExpr):
        _remember_goal_atoms(ordered, goal_value.body)
        return
    if isinstance(goal_value, NativeGoalExpr | DeferredExpr):
        for argument in goal_value.args:
            _remember_term_atoms(ordered, argument)
        return
    if isinstance(goal_value, SucceedExpr | FailExpr | CutExpr):
        return


def _remember_term_functors(
    ordered: dict[tuple[Atom, int], None],
    term_value: Term,
) -> None:
    if isinstance(term_value, Atom):
        _remember_functor(ordered, term_value, 0)
        return
    if isinstance(term_value, Compound):
        _remember_functor(ordered, atom(term_value.functor), len(term_value.args))
        for argument in term_value.args:
            _remember_term_functors(ordered, argument)


def _remember_goal_functors(
    ordered: dict[tuple[Atom, int], None],
    goal_value: GoalExpr,
) -> None:
    if isinstance(goal_value, RelationCall):
        _remember_functor(
            ordered,
            atom(goal_value.relation.symbol),
            goal_value.relation.arity,
        )
        for argument in goal_value.args:
            _remember_term_functors(ordered, argument)
        return
    if isinstance(goal_value, EqExpr):
        _remember_functor(ordered, atom("="), 2)
        _remember_term_functors(ordered, goal_value.left)
        _remember_term_functors(ordered, goal_value.right)
        return
    if isinstance(goal_value, NeqExpr):
        _remember_functor(ordered, atom("\\="), 2)
        _remember_term_functors(ordered, goal_value.left)
        _remember_term_functors(ordered, goal_value.right)
        return
    if isinstance(goal_value, ConjExpr | DisjExpr):
        for child in goal_value.goals:
            _remember_goal_functors(ordered, child)
        return
    if isinstance(goal_value, FreshExpr):
        _remember_goal_functors(ordered, goal_value.body)
        return
    if isinstance(goal_value, NativeGoalExpr | DeferredExpr):
        for argument in goal_value.args:
            _remember_term_functors(ordered, argument)
        return
    if isinstance(goal_value, SucceedExpr | FailExpr | CutExpr):
        return


def _visible_atoms(program_value: Program, state: State) -> tuple[Atom, ...]:
    """Enumerate atoms observable from source, dynamic state, and builtins."""

    ordered: dict[Atom, None] = {}
    for key in visible_predicate_keys(program_value, state):
        _remember_atom(ordered, atom(key[0]))
    for source_clause in visible_clauses(program_value, state):
        _remember_goal_atoms(ordered, source_clause.head)
        if source_clause.body is not None:
            _remember_goal_atoms(ordered, source_clause.body)
    for name, _arity in _BUILTIN_PREDICATES:
        _remember_atom(ordered, atom(name))
    for flag_name, flag_value in _PROLOG_FLAGS:
        _remember_atom(ordered, flag_name)
        _remember_term_atoms(ordered, flag_value)
    for writable_flag, allowed_values in _PROLOG_WRITABLE_FLAG_VALUES.items():
        _remember_atom(ordered, writable_flag)
        for allowed_value in allowed_values:
            _remember_term_atoms(ordered, allowed_value)
    for property_atom in (
        atom("defined"),
        atom("dynamic"),
        atom("static"),
        atom("built_in"),
        atom("number_of_clauses"),
    ):
        _remember_atom(ordered, property_atom)
    return tuple(ordered)


def _visible_functors(
    program_value: Program,
    state: State,
) -> tuple[tuple[Atom, int], ...]:
    """Enumerate functor indicators visible in source, dynamic, and builtins."""

    ordered: dict[tuple[Atom, int], None] = {}
    for key in visible_predicate_keys(program_value, state):
        _remember_functor(ordered, atom(key[0]), key[1])
    for source_clause in visible_clauses(program_value, state):
        _remember_goal_functors(ordered, source_clause.head)
        if source_clause.body is not None:
            _remember_goal_functors(ordered, source_clause.body)
    for name, arity in _BUILTIN_PREDICATES:
        _remember_functor(ordered, atom(name), arity)
    for flag_name, flag_value in _PROLOG_FLAGS:
        _remember_functor(ordered, flag_name, 0)
        _remember_term_functors(ordered, flag_value)
    for writable_flag, allowed_values in _PROLOG_WRITABLE_FLAG_VALUES.items():
        _remember_functor(ordered, writable_flag, 0)
        for allowed_value in allowed_values:
            _remember_term_functors(ordered, allowed_value)
    _remember_functor(ordered, atom("number_of_clauses"), 1)
    for property_atom in (atom("defined"), atom("dynamic"), atom("static")):
        _remember_functor(ordered, property_atom, 0)
    return tuple(ordered)


def current_atomo(name: object) -> GoalExpr:
    """Enumerate atoms visible in the current source and builtin environment."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        (name_target,) = args
        for name_atom in _visible_atoms(program_value, state):
            yield from solve_from(program_value, eq(name_target, name_atom), state)

    return native_goal(run, name)


def current_functoro(name: object, arity: object) -> GoalExpr:
    """Enumerate functor indicators visible in the current environment."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_target, arity_target = args
        for name_atom, raw_arity in _visible_functors(program_value, state):
            yield from solve_from(
                program_value,
                conj(eq(name_target, name_atom), eq(arity_target, num(raw_arity))),
                state,
            )

    return native_goal(run, name, arity)


def current_prolog_flago(name: object, value: object) -> GoalExpr:
    """Enumerate runtime flags visible in the current proof branch."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_target, value_target = args
        store = _prolog_flag_store(state)
        for flag_name, flag_value in _PROLOG_FLAGS:
            current_value = store.values.get(flag_name, flag_value)
            yield from solve_from(
                program_value,
                conj(eq(name_target, flag_name), eq(value_target, current_value)),
                state,
            )

    return native_goal(run, name, value)


def _reified_flag_atom(flag_name: Term, state: State) -> Atom | None:
    """Return a known Prolog flag atom after reification."""

    reified_name = _reified(flag_name, state)
    if isinstance(reified_name, LogicVar):
        msg = "set_prolog_flag/2 requires an instantiated flag name"
        raise PrologInstantiationError(msg, culprit=reified_name)
    if not isinstance(reified_name, Atom):
        return None
    if reified_name not in _PROLOG_DEFAULT_FLAGS:
        return None
    return reified_name


def _reified_flag_value(flag_value: Term, state: State) -> Term | None:
    """Return a concrete Prolog flag value after reification."""

    reified_value = _reified(flag_value, state)
    if isinstance(reified_value, LogicVar):
        msg = "set_prolog_flag/2 requires an instantiated flag value"
        raise PrologInstantiationError(msg, culprit=reified_value)
    return reified_value


def set_prolog_flago(name: object, value: object) -> GoalExpr:
    """Set a supported Prolog flag in the current proof branch."""

    def run(_program: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_term, value_term = args
        flag_name = _reified_flag_atom(name_term, state)
        if flag_name is None:
            return

        allowed_values = _PROLOG_WRITABLE_FLAG_VALUES.get(flag_name)
        if allowed_values is None:
            return

        flag_value = _reified_flag_value(value_term, state)
        if flag_value not in allowed_values:
            return

        store = _prolog_flag_store(state)
        yield _state_with_prolog_flags(
            state,
            PrologFlagStore(values={**store.values, flag_name: flag_value}),
        )

    return native_goal(run, name, value)


def current_predicateo(name: object, arity: object) -> GoalExpr:
    """Enumerate predicates visible in the current program and builtin layer."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_target, arity_target = args
        for name_atom, raw_arity in _predicate_indicators(program_value, state):
            yield from solve_from(
                program_value,
                conj(eq(name_target, name_atom), eq(arity_target, num(raw_arity))),
                state,
            )

    return native_goal(run, name, arity)


def predicate_propertyo(name: object, arity: object, property_term: object) -> GoalExpr:
    """Enumerate properties for visible source and builtin predicates."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        name_target, arity_target, property_target = args
        for name_atom, raw_arity in _predicate_indicators(program_value, state):
            for property_value in _predicate_properties(
                program_value,
                state,
                name_atom,
                raw_arity,
            ):
                yield from solve_from(
                    program_value,
                    conj(
                        eq(name_target, name_atom),
                        eq(arity_target, num(raw_arity)),
                        eq(property_target, property_value),
                    ),
                    state,
                )

    return native_goal(run, name, arity, property_term)


def clauseo(head: object, body: object) -> GoalExpr:
    """Inspect source clauses as Prolog-style `Head :- Body` data."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        head_target, body_target = args
        for source_clause in visible_clauses(program_value, state):
            fresh_clause, next_var_id = freshen_clause(
                source_clause,
                state.next_var_id,
            )
            try:
                body_term = goal_as_term(clause_body(fresh_clause))
            except TypeError:
                continue

            inspection_state = _state_with_next_var_id(state, next_var_id)
            yield from solve_from(
                program_value,
                conj(
                    eq(head_target, fresh_clause.head.as_term()),
                    eq(body_target, body_term),
                ),
                inspection_state,
            )

    return native_goal(run, _as_callable_term(head), body)
