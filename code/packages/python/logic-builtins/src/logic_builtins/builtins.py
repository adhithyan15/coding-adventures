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
from itertools import product

from logic_engine import (
    Atom,
    Clause,
    Compound,
    GoalExpr,
    LogicVar,
    Number,
    Program,
    Relation,
    RelationCall,
    State,
    String,
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
    "all_differento",
    "argo",
    "atomico",
    "atomo",
    "betweeno",
    "callo",
    "callableo",
    "calltermo",
    "compoundo",
    "compare_termo",
    "copytermo",
    "convlisto",
    "current_predicateo",
    "cuto",
    "difo",
    "dynamico",
    "div",
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
    "excludeo",
    "FiniteDomainConstraint",
    "FiniteDomainStore",
    "findallo",
    "floordiv",
    "foldlo",
    "forallo",
    "functoro",
    "geqo",
    "gto",
    "groundo",
    "ifthenelseo",
    "iftheno",
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
    "numeqo",
    "numneqo",
    "numbero",
    "onceo",
    "bagofo",
    "partitiono",
    "predicate_propertyo",
    "PrologEvaluationError",
    "PrologInstantiationError",
    "PrologRuntimeError",
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
    "clauseo",
    "stringo",
    "sub",
    "succo",
    "termo_geqo",
    "termo_gto",
    "termo_leqo",
    "termo_lto",
    "trueo",
    "univo",
    "varo",
]


type NativeArgs = tuple[Term, ...]
type NativeRunner = Callable[[Program, State, NativeArgs], Iterator[State]]
type NumericValue = int | float
type TermSortKey = tuple[object, ...]
type FdOperator = str


_MAX_FD_DOMAIN_SIZE = 10_000


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


_DEFAULT_LABELING_OPTIONS = LabelingOptions()


_BUILTIN_PREDICATES: tuple[tuple[str, int], ...] = (
    ("abolisho", 2),
    ("all_differento", 1),
    ("argo", 3),
    ("assertao", 1),
    ("assertzo", 1),
    ("atomico", 1),
    ("atomo", 1),
    ("bagofo", 3),
    ("betweeno", 3),
    ("callableo", 1),
    ("callo", 1),
    ("calltermo", 1),
    ("calltermo", 2),
    ("calltermo", 3),
    ("calltermo", 4),
    ("calltermo", 5),
    ("calltermo", 6),
    ("calltermo", 7),
    ("calltermo", 8),
    ("clauseo", 2),
    ("compare_termo", 3),
    ("compoundo", 1),
    ("copytermo", 2),
    ("convlisto", 3),
    ("current_predicateo", 2),
    ("cuto", 0),
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
    ("numbero", 1),
    ("onceo", 1),
    ("partitiono", 4),
    ("predicate_propertyo", 3),
    ("retractallo", 1),
    ("retracto", 1),
    ("same_termo", 2),
    ("scanlo", 4),
    ("scanlo", 5),
    ("scanlo", 6),
    ("scanlo", 7),
    ("setofo", 3),
    ("stringo", 1),
    ("succo", 2),
    ("termo_geqo", 2),
    ("termo_gto", 2),
    ("termo_leqo", 2),
    ("termo_lto", 2),
    ("trueo", 0),
    ("univo", 2),
    ("varo", 1),
)


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


def _unify_collection(
    program_value: Program,
    state: State,
    results: Term,
    values: list[Term],
) -> Iterator[State]:
    """Unify a result term with a canonical logic list from the outer state."""

    yield from solve_from(program_value, eq(results, logic_list(values)), state)


def findallo(template: object, goal: object, results: object) -> GoalExpr:
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
                reified_goal = goal_from_term(_reified(called_goal_term, state))
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


def bagofo(template: object, goal: object, results: object) -> GoalExpr:
    """Collect a non-empty proof-order bag of solutions."""

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
                reified_goal = goal_from_term(_reified(called_goal_term, state))
            except TypeError:
                return
            values = _collect_template_values(
                program_value,
                state,
                template_term,
                reified_goal,
            )
            if not values:
                return
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
        if not values:
            return
        yield from _unify_collection(program_value, state, results_term, values)

    return native_goal(run, template, results)


def setofo(template: object, goal: object, results: object) -> GoalExpr:
    """Collect a non-empty sorted set of solutions."""

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
                reified_goal = goal_from_term(_reified(called_goal_term, state))
            except TypeError:
                return
            values = _collect_template_values(
                program_value,
                state,
                template_term,
                reified_goal,
            )
            if not values:
                return
            yield from _unify_collection(
                program_value,
                state,
                results_term,
                _unique_sorted_terms(values),
            )

        return native_goal(run_terms, template, goal_term, results)

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        template_term, results_term = args
        values = _collect_template_values(
            program_value,
            state,
            template_term,
            called_goal,
        )
        if not values:
            return
        yield from _unify_collection(
            program_value,
            state,
            results_term,
            _unique_sorted_terms(values),
        )

    return native_goal(run, template, results)


def trueo() -> GoalExpr:
    """Succeed once without changing the current logic state."""

    return succeed()


def failo() -> GoalExpr:
    """Fail without yielding any successor states."""

    return engine_fail()


def cuto() -> GoalExpr:
    """Commit to choices made so far in the current search-control frame."""

    return cut()


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
