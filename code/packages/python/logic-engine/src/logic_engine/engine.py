"""Logic Engine — relations, clauses, programs, and resolution.

``logic-core`` already provides the *semantic atoms* of logic programming:
terms, substitutions, unification, and generator-based backtracking states.
What this module adds is the layer that makes those semantics feel like a real
logic-programming environment:

- named relations such as ``parent/2`` and ``ancestor/2``
- facts and rules
- programs that collect clauses
- a solver that resolves relation calls against those clauses

This is the missing bridge between a tiny relational kernel and a future
Prolog implementation. The later Prolog parser should mostly lower syntax into
the data structures defined here.
"""

from __future__ import annotations

from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass, field
from itertools import count, islice

from logic_core import (
    Atom,
    Compound,
    Disequality,
    LogicVar,
    Number,
    State,
    String,
    Substitution,
    Term,
    atom,
    logic_list,
    num,
    reify,
    string,
    term,
    unify,
    var,
)
from logic_core import (
    eq as core_eq,
)
from logic_core import (
    neq as core_neq,
)
from symbol_core import Symbol, sym

__all__ = [
    "Atom",
    "Clause",
    "Compound",
    "ConjExpr",
    "CutExpr",
    "DeferredExpr",
    "DynamicDatabase",
    "DisjExpr",
    "Disequality",
    "EqExpr",
    "FailExpr",
    "FreshExpr",
    "GoalExpr",
    "LogicVar",
    "NativeGoalExpr",
    "NativeGoalRunner",
    "NeqExpr",
    "Number",
    "Program",
    "Relation",
    "RelationCall",
    "State",
    "String",
    "Substitution",
    "SucceedExpr",
    "Term",
    "abolish",
    "all_different",
    "asserta",
    "assertz",
    "atom",
    "clause_as_term",
    "clause_body",
    "clause_from_term",
    "clauses_matching",
    "conj",
    "cut",
    "declare_dynamic",
    "defer",
    "disj",
    "eq",
    "fact",
    "fail",
    "fresh",
    "freshen_clause",
    "goal_as_term",
    "goal_from_term",
    "is_dynamic_relation",
    "logic_list",
    "native_goal",
    "neq",
    "num",
    "program",
    "relation",
    "reify",
    "runtime_abolish",
    "runtime_asserta",
    "runtime_assertz",
    "runtime_declare_dynamic",
    "runtime_retract_all",
    "runtime_retract_first",
    "retract_all",
    "retract_first",
    "rule",
    "solve",
    "solve_all",
    "solve_from",
    "solve_n",
    "string",
    "succeed",
    "term",
    "var",
    "visible_clause_count",
    "visible_clauses",
    "visible_clauses_for",
    "visible_predicate_keys",
]


# Template variables live far away from query variables so the two namespaces
# do not collide before the solver remaps clause-local variables into search
# local runtime ids.
_TEMPLATE_VAR_IDS = count(start=-1_000_000, step=-1)

type RelationKey = tuple[Symbol, int]


def _coerce_term(value: object) -> Term:
    """Convert host-language convenience inputs into LP00 terms.

    This mirrors the user-facing ergonomics of ``logic-core``: strings become
    atoms, numbers become numeric terms, and existing term objects pass through
    unchanged.
    """

    if isinstance(value, Atom | Number | String | LogicVar | Compound):
        return value
    if isinstance(value, Symbol):
        return atom(value)
    if isinstance(value, bool):
        msg = (
            "bool values are ambiguous in logic-engine; use atoms or "
            "numbers explicitly"
        )
        raise TypeError(msg)
    if isinstance(value, int | float):
        return num(value)
    if isinstance(value, str):
        return atom(value)

    msg = f"cannot coerce {type(value).__name__} into a logic term"
    raise TypeError(msg)


@dataclass(frozen=True, slots=True)
class Relation:
    """A named predicate with fixed arity, such as ``parent/2``."""

    symbol: Symbol
    arity: int

    def __post_init__(self) -> None:
        if self.arity < 0:
            msg = "relation arity must be non-negative"
            raise ValueError(msg)

    def __call__(self, *args: object) -> RelationCall:
        if len(args) != self.arity:
            msg = (
                f"relation {self.symbol}/{self.arity} expected {self.arity} "
                f"arguments but got {len(args)}"
            )
            raise ValueError(msg)
        return RelationCall(
            relation=self,
            args=tuple(_coerce_term(argument) for argument in args),
        )

    def key(self) -> tuple[Symbol, int]:
        """Return the immutable lookup key used for program indexing."""

        return (self.symbol, self.arity)

    def __str__(self) -> str:
        return f"{self.symbol}/{self.arity}"


def relation(name: str | Symbol, arity: int) -> Relation:
    """Construct a relation object."""

    symbol = name if isinstance(name, Symbol) else sym(name)
    return Relation(symbol=symbol, arity=arity)


@dataclass(frozen=True, slots=True)
class RelationCall:
    """A call expression such as ``parent(homer, bart)`` or ``ancestor(X, Y)``."""

    relation: Relation
    args: tuple[Term, ...]

    def as_term(self) -> Compound:
        """View the relation call as an LP00 compound term for unification."""

        return Compound(functor=self.relation.symbol, args=self.args)

    def __str__(self) -> str:
        if not self.args:
            return str(self.relation.symbol)
        rendered_args = ", ".join(str(argument) for argument in self.args)
        return f"{self.relation.symbol}({rendered_args})"


@dataclass(frozen=True, slots=True)
class SucceedExpr:
    """A goal expression that yields the current state unchanged."""


@dataclass(frozen=True, slots=True)
class FailExpr:
    """A goal expression that yields no successor states."""


@dataclass(frozen=True, slots=True)
class CutExpr:
    """A goal expression that commits to choices made in the current frame."""


@dataclass(frozen=True, slots=True)
class EqExpr:
    """A structured equality goal that delegates unification to LP00."""

    left: Term
    right: Term


@dataclass(frozen=True, slots=True)
class NeqExpr:
    """A structured disequality goal delegated to ``logic-core`` constraints."""

    left: Term
    right: Term


@dataclass(frozen=True, slots=True)
class ConjExpr:
    """A conjunction of goal expressions."""

    goals: tuple[GoalExpr, ...]


@dataclass(frozen=True, slots=True)
class DisjExpr:
    """A disjunction of goal expressions."""

    goals: tuple[GoalExpr, ...]


@dataclass(frozen=True, slots=True)
class FreshExpr:
    """A goal expression that introduces search-local variables.

    The variables stored here are *template* variables. They are placeholders
    that get renamed to fresh runtime variables each time the expression is
    solved.
    """

    template_vars: tuple[LogicVar, ...]
    body: GoalExpr


@dataclass(frozen=True, slots=True)
class DeferredExpr:
    """A goal builder call that should expand only when the solver reaches it.

    This is the adapter that lets host-language helper libraries define
    recursive relations without triggering eager Python recursion while the
    goal tree is still being built.
    """

    builder: Callable[..., object]
    args: tuple[Term, ...]


type NativeGoalRunner = Callable[[Program, State, tuple[Term, ...]], Iterator[State]]


@dataclass(frozen=True, slots=True)
class NativeGoalExpr:
    """A state-aware host-language goal implemented by a Python callable.

    Most logic goals can be represented as pure expression trees. A few
    Prolog-style builtins need to inspect the current substitution or run a
    nested goal from the current state. ``NativeGoalExpr`` is the small escape
    hatch for those predicates.
    """

    runner: NativeGoalRunner
    args: tuple[Term, ...]


type GoalExpr = (
    RelationCall
    | SucceedExpr
    | FailExpr
    | CutExpr
    | EqExpr
    | NeqExpr
    | NativeGoalExpr
    | DeferredExpr
    | ConjExpr
    | DisjExpr
    | FreshExpr
)


def _coerce_goal(goal: object) -> GoalExpr:
    """Validate that ``goal`` is a supported engine expression."""

    if isinstance(
        goal,
        (
            RelationCall
            | SucceedExpr
            | FailExpr
            | CutExpr
            | EqExpr
            | NeqExpr
            | NativeGoalExpr
            | DeferredExpr
            | ConjExpr
            | DisjExpr
            | FreshExpr
        ),
    ):
        return goal

    msg = f"cannot use {type(goal).__name__} as a logic-engine goal expression"
    raise TypeError(msg)


def succeed() -> GoalExpr:
    """Construct a success goal expression."""

    return SucceedExpr()


def fail() -> GoalExpr:
    """Construct a failure goal expression."""

    return FailExpr()


def cut() -> GoalExpr:
    """Construct a scoped Prolog-style cut goal expression.

    Cut succeeds once, then prunes choicepoints created earlier in the current
    query or predicate invocation. Predicate calls consume cuts raised by their
    own clause bodies, so a cut inside a relation commits that relation without
    accidentally pruning the caller's surrounding search.
    """

    return CutExpr()


def eq(left: object, right: object) -> GoalExpr:
    """Construct an equality goal expression."""

    return EqExpr(left=_coerce_term(left), right=_coerce_term(right))


def neq(left: object, right: object) -> GoalExpr:
    """Construct a disequality goal expression."""

    return NeqExpr(left=_coerce_term(left), right=_coerce_term(right))


def defer(builder: Callable[..., object], *args: object) -> GoalExpr:
    """Delay a helper-goal expansion until solve time.

    ``fresh(...)`` builds expression trees eagerly. Recursive host-language
    helper functions therefore need a way to store recursive calls as data and
    expand them only when the solver actually reaches them.
    """

    return DeferredExpr(
        builder=builder,
        args=tuple(_coerce_term(argument) for argument in args),
    )


def native_goal(runner: NativeGoalRunner, *args: object) -> GoalExpr:
    """Construct a state-aware native goal expression.

    Native goals are intentionally rare. They are for builtins such as
    ``ground`` and ``once`` that need access to the active search state, while
    ordinary relational helpers should keep using ``eq``, ``conj``, ``disj``,
    ``fresh``, and relation calls.
    """

    return NativeGoalExpr(
        runner=runner,
        args=tuple(_coerce_term(argument) for argument in args),
    )


def conj(*goals: object) -> GoalExpr:
    """Construct a conjunction.

    An empty conjunction behaves like logical truth, so it succeeds.
    """

    flattened: list[GoalExpr] = []
    for candidate in goals:
        goal = _coerce_goal(candidate)
        if isinstance(goal, ConjExpr):
            flattened.extend(goal.goals)
        else:
            flattened.append(goal)

    if not flattened:
        return succeed()
    if len(flattened) == 1:
        return flattened[0]
    return ConjExpr(goals=tuple(flattened))


def disj(*goals: object) -> GoalExpr:
    """Construct a disjunction.

    An empty disjunction behaves like logical falsehood, so it fails.
    """

    flattened: list[GoalExpr] = []
    for candidate in goals:
        goal = _coerce_goal(candidate)
        if isinstance(goal, DisjExpr):
            flattened.extend(goal.goals)
        else:
            flattened.append(goal)

    if not flattened:
        return fail()
    if len(flattened) == 1:
        return flattened[0]
    return DisjExpr(goals=tuple(flattened))


def all_different(*terms: object) -> GoalExpr:
    """Require every supplied term to remain pairwise distinct.

    This is the first convenience combinator for puzzle-style search. It lowers
    to a conjunction of pairwise disequalities.
    """

    goals: list[GoalExpr] = []
    for index, left in enumerate(terms):
        for right in terms[index + 1 :]:
            goals.append(neq(left, right))
    return conj(*goals)


def fresh(count: int, fn: Callable[..., object]) -> GoalExpr:
    """Construct a fresh-variable goal expression.

    The callback receives template variables immediately. The resulting body is
    stored as a pure data expression and only receives real runtime ids when it
    is actually solved.
    """

    if count <= 0:
        msg = "fresh() requires at least one variable"
        raise ValueError(msg)

    template_vars = tuple(
        LogicVar(id=next(_TEMPLATE_VAR_IDS))
        for _ in range(count)
    )
    return FreshExpr(
        template_vars=template_vars,
        body=_coerce_goal(fn(*template_vars)),
    )


@dataclass(frozen=True, slots=True)
class Clause:
    """A fact or rule in a logic program."""

    head: RelationCall
    body: GoalExpr | None = None

    def is_fact(self) -> bool:
        """Return True when the clause has no body."""

        return self.body is None


def fact(head: RelationCall) -> Clause:
    """Construct a fact clause."""

    if not isinstance(head, RelationCall):
        msg = "fact() requires a relation call as its head"
        raise TypeError(msg)
    return Clause(head=head)


def rule(head: RelationCall, body: object) -> Clause:
    """Construct a rule clause."""

    if not isinstance(head, RelationCall):
        msg = "rule() requires a relation call as its head"
        raise TypeError(msg)
    return Clause(head=head, body=_coerce_goal(body))


@dataclass(frozen=True, slots=True)
class DynamicDatabase:
    """Branch-local runtime clauses and dynamic declarations.

    Instances are immutable, so normal generator backtracking restores an older
    database snapshot automatically when the solver explores another branch.
    """

    declared_relations: frozenset[RelationKey] = frozenset()
    abolished_relations: frozenset[RelationKey] = frozenset()
    prepended_clauses: tuple[Clause, ...] = ()
    appended_clauses: tuple[Clause, ...] = ()
    removed_program_clause_indexes: frozenset[int] = frozenset()


@dataclass(frozen=True, slots=True)
class _VisibleClause:
    """A visible dynamic clause plus enough identity to remove it."""

    clause: Clause
    source: str
    index: int


@dataclass(frozen=True, slots=True)
class _SearchResult:
    """One internal proof event plus whether a cut fired on that path.

    ``state=None`` represents a failing path that still needs to propagate a
    cut, such as the Prolog pattern ``!, fail``.
    """

    state: State | None
    cut: bool = False


def _normalize_relation_keys(keys: Iterable[RelationKey]) -> frozenset[RelationKey]:
    """Validate relation-key collections stored on programs and databases."""

    normalized: set[RelationKey] = set()
    for key in keys:
        if (
            not isinstance(key, tuple)
            or len(key) != 2
            or not isinstance(key[0], Symbol)
            or isinstance(key[1], bool)
            or not isinstance(key[1], int)
            or key[1] < 0
        ):
            msg = "dynamic relation keys must be (Symbol, non-negative int) tuples"
            raise TypeError(msg)
        normalized.add(key)
    return frozenset(normalized)


@dataclass(frozen=True, slots=True)
class Program:
    """An immutable clause database indexed by relation."""

    clauses: tuple[Clause, ...]
    dynamic_relations: frozenset[RelationKey] = field(default_factory=frozenset)
    _index: dict[tuple[Symbol, int], tuple[Clause, ...]] = field(
        init=False,
        repr=False,
        compare=False,
    )
    _indexed_clauses: dict[RelationKey, tuple[tuple[int, Clause], ...]] = field(
        init=False,
        repr=False,
        compare=False,
    )

    def __post_init__(self) -> None:
        buckets: dict[tuple[Symbol, int], list[Clause]] = {}
        indexed_buckets: dict[RelationKey, list[tuple[int, Clause]]] = {}
        for index, clause in enumerate(self.clauses):
            if not isinstance(clause, Clause):
                msg = "Program clauses must all be Clause objects"
                raise TypeError(msg)
            key = clause.head.relation.key()
            buckets.setdefault(key, []).append(clause)
            indexed_buckets.setdefault(key, []).append((index, clause))

        frozen_index = {
            key: tuple(grouped_clauses)
            for key, grouped_clauses in buckets.items()
        }
        frozen_indexed_clauses = {
            key: tuple(grouped_clauses)
            for key, grouped_clauses in indexed_buckets.items()
        }
        object.__setattr__(
            self,
            "dynamic_relations",
            _normalize_relation_keys(self.dynamic_relations),
        )
        object.__setattr__(self, "_index", frozen_index)
        object.__setattr__(self, "_indexed_clauses", frozen_indexed_clauses)

    def clauses_for(self, relation_value: Relation) -> tuple[Clause, ...]:
        """Return clauses for ``relation_value`` in source order."""

        return self._index.get(relation_value.key(), ())


def program(
    *clauses: Clause,
    dynamic_relations: Iterable[Relation] = (),
) -> Program:
    """Construct an immutable logic program."""

    dynamic_keys = frozenset(
        relation_value.key() for relation_value in dynamic_relations
    )
    return Program(clauses=tuple(clauses), dynamic_relations=dynamic_keys)


def declare_dynamic(program_value: Program, *relations: Relation) -> Program:
    """Return a new program whose listed predicates allow runtime mutation."""

    checked_program = _require_program(program_value)
    dynamic_keys = set(checked_program.dynamic_relations)
    for relation_value in relations:
        dynamic_keys.add(_require_relation(relation_value).key())
    return Program(
        clauses=checked_program.clauses,
        dynamic_relations=frozenset(dynamic_keys),
    )


def _require_program(program_value: object) -> Program:
    """Validate that a database helper received a program."""

    if not isinstance(program_value, Program):
        msg = "database helpers require a Program"
        raise TypeError(msg)
    return program_value


def _require_clause(clause_value: object) -> Clause:
    """Validate that a database assertion received a clause."""

    if not isinstance(clause_value, Clause):
        msg = "assertion helpers require a Clause"
        raise TypeError(msg)
    return clause_value


def _require_relation(relation_value: object) -> Relation:
    """Validate that relation-key database helpers received a relation."""

    if not isinstance(relation_value, Relation):
        msg = "abolish() requires a Relation"
        raise TypeError(msg)
    return relation_value


def _require_relation_call(head_pattern: object) -> RelationCall:
    """Validate that pattern-based helpers received a relation call."""

    if not isinstance(head_pattern, RelationCall):
        msg = "database pattern helpers require a RelationCall"
        raise TypeError(msg)
    return head_pattern


def _clause_head_matches(head_pattern: RelationCall, candidate: Clause) -> bool:
    """Return True when a clause head unifies with a relation-call pattern."""

    if head_pattern.relation.key() != candidate.head.relation.key():
        return False
    return unify(head_pattern.as_term(), candidate.head.as_term()) is not None


def asserta(program_value: Program, clause_value: Clause) -> Program:
    """Return a new program with ``clause_value`` inserted at the front."""

    checked_program = _require_program(program_value)
    checked_clause = _require_clause(clause_value)
    return Program(clauses=(checked_clause, *checked_program.clauses))


def assertz(program_value: Program, clause_value: Clause) -> Program:
    """Return a new program with ``clause_value`` appended at the end."""

    checked_program = _require_program(program_value)
    checked_clause = _require_clause(clause_value)
    return Program(clauses=(*checked_program.clauses, checked_clause))


def clauses_matching(
    program_value: Program,
    head_pattern: RelationCall,
) -> tuple[Clause, ...]:
    """Return clauses whose heads unify with ``head_pattern`` in source order."""

    checked_program = _require_program(program_value)
    checked_pattern = _require_relation_call(head_pattern)
    return tuple(
        clause
        for clause in checked_program.clauses
        if _clause_head_matches(checked_pattern, clause)
    )


def retract_first(
    program_value: Program,
    head_pattern: RelationCall,
) -> Program | None:
    """Return a new program with the first matching clause removed."""

    checked_program = _require_program(program_value)
    checked_pattern = _require_relation_call(head_pattern)
    retained: list[Clause] = []
    removed = False
    for clause in checked_program.clauses:
        if not removed and _clause_head_matches(checked_pattern, clause):
            removed = True
            continue
        retained.append(clause)

    if not removed:
        return None
    return Program(clauses=tuple(retained))


def retract_all(program_value: Program, head_pattern: RelationCall) -> Program:
    """Return a new program with every matching clause removed."""

    checked_program = _require_program(program_value)
    checked_pattern = _require_relation_call(head_pattern)
    return Program(
        clauses=tuple(
            clause
            for clause in checked_program.clauses
            if not _clause_head_matches(checked_pattern, clause)
        ),
    )


def abolish(program_value: Program, relation_value: Relation) -> Program:
    """Return a new program without any clauses for ``relation_value``."""

    checked_program = _require_program(program_value)
    checked_relation = _require_relation(relation_value)
    return Program(
        clauses=tuple(
            clause
            for clause in checked_program.clauses
            if clause.head.relation.key() != checked_relation.key()
        ),
    )


def clause_body(clause_value: object) -> GoalExpr:
    """Return a clause body goal, using logical truth for facts."""

    checked_clause = _require_clause(clause_value)
    if checked_clause.body is None:
        return succeed()
    return checked_clause.body


def _nested_goal_term(functor: str, goals: tuple[GoalExpr, ...]) -> Term:
    """Encode n-ary engine nodes as right-nested binary Prolog terms."""

    if len(goals) == 1:
        return goal_as_term(goals[0])
    return term(functor, goal_as_term(goals[0]), _nested_goal_term(functor, goals[1:]))


def goal_as_term(goal_value: object) -> Term:
    """Encode a representable goal expression as first-order term data."""

    goal = _coerce_goal(goal_value)
    if isinstance(goal, RelationCall):
        return goal.as_term()
    if isinstance(goal, SucceedExpr):
        return atom("true")
    if isinstance(goal, FailExpr):
        return atom("fail")
    if isinstance(goal, CutExpr):
        return atom("!")
    if isinstance(goal, EqExpr):
        return term("=", goal.left, goal.right)
    if isinstance(goal, NeqExpr):
        return term("\\=", goal.left, goal.right)
    if isinstance(goal, ConjExpr):
        if not goal.goals:
            return atom("true")
        return _nested_goal_term(",", goal.goals)
    if isinstance(goal, DisjExpr):
        if not goal.goals:
            return atom("fail")
        return _nested_goal_term(";", goal.goals)

    msg = (
        f"cannot encode {type(goal).__name__} as a first-order goal term"
    )
    raise TypeError(msg)


def _is_plain_atom(term_value: Atom, name: str) -> bool:
    """Return True when an atom is the unqualified symbolic name."""

    return term_value.symbol.namespace is None and term_value.symbol.name == name


def _term_functor_name(term_value: Compound) -> str | None:
    """Return an unqualified compound functor name, or None for namespaced ones."""

    if term_value.functor.namespace is not None:
        return None
    return term_value.functor.name


def _binary_goal_arguments(term_value: Compound, operator: str) -> tuple[Term, Term]:
    """Read a binary Prolog control/equality term or raise a clear error."""

    if len(term_value.args) != 2:
        msg = f"{operator}/2 goal terms require exactly two arguments"
        raise TypeError(msg)
    return term_value.args


def goal_from_term(term_value: object) -> GoalExpr:
    """Lower first-order goal data back into an executable goal expression.

    LP17 taught the engine how to reify goals as terms. This helper is the
    inverse for the representable Prolog-shaped subset: truth, failure,
    equality, disequality, conjunction, disjunction, and relation calls.
    """

    callable_term = _coerce_term(term_value)

    if isinstance(callable_term, Atom):
        if _is_plain_atom(callable_term, "true"):
            return succeed()
        if _is_plain_atom(callable_term, "fail"):
            return fail()
        if _is_plain_atom(callable_term, "!"):
            return cut()
        return RelationCall(
            relation=Relation(symbol=callable_term.symbol, arity=0),
            args=(),
        )

    if isinstance(callable_term, Compound):
        functor_name = _term_functor_name(callable_term)
        if functor_name == "=":
            left, right = _binary_goal_arguments(callable_term, "=")
            return eq(left, right)
        if functor_name == "\\=":
            left, right = _binary_goal_arguments(callable_term, "\\=")
            return neq(left, right)
        if functor_name == ",":
            left, right = _binary_goal_arguments(callable_term, ",")
            return conj(goal_from_term(left), goal_from_term(right))
        if functor_name == ";":
            left, right = _binary_goal_arguments(callable_term, ";")
            return disj(goal_from_term(left), goal_from_term(right))
        return RelationCall(
            relation=Relation(
                symbol=callable_term.functor,
                arity=len(callable_term.args),
            ),
            args=callable_term.args,
        )

    msg = f"cannot lower {type(callable_term).__name__} into a callable goal"
    raise TypeError(msg)


def _relation_call_from_term(term_value: Term) -> RelationCall:
    """Lower a callable head term into a relation call."""

    if isinstance(term_value, Atom):
        return RelationCall(
            relation=Relation(symbol=term_value.symbol, arity=0),
            args=(),
        )
    if isinstance(term_value, Compound):
        return RelationCall(
            relation=Relation(symbol=term_value.functor, arity=len(term_value.args)),
            args=term_value.args,
        )

    msg = f"cannot lower {type(term_value).__name__} into a clause head"
    raise TypeError(msg)


def clause_from_term(term_value: object) -> Clause:
    """Lower Prolog-shaped clause data into a fact or rule clause.

    A bare callable term becomes a fact. A ``:-(Head, Body)`` compound becomes
    a rule whose body is lowered through ``goal_from_term``.
    """

    clause_term = _coerce_term(term_value)
    if isinstance(clause_term, Compound) and _term_functor_name(clause_term) == ":-":
        head_term, body_term = _binary_goal_arguments(clause_term, ":-")
        return Clause(
            head=_relation_call_from_term(head_term),
            body=goal_from_term(body_term),
        )

    return Clause(head=_relation_call_from_term(clause_term))


def clause_as_term(clause_value: object) -> Compound:
    """Encode a clause as the Prolog-shaped term ``:-(Head, Body)``."""

    checked_clause = _require_clause(clause_value)
    return term(
        ":-",
        checked_clause.head.as_term(),
        goal_as_term(clause_body(checked_clause)),
    )


def _rename_term(term_value: Term, mapping: dict[LogicVar, LogicVar]) -> Term:
    """Rename logic variables inside a term according to ``mapping``."""

    if isinstance(term_value, LogicVar):
        return mapping.get(term_value, term_value)
    if isinstance(term_value, Compound):
        return Compound(
            functor=term_value.functor,
            args=tuple(_rename_term(argument, mapping) for argument in term_value.args),
        )
    return term_value


def _rename_goal(goal: GoalExpr, mapping: dict[LogicVar, LogicVar]) -> GoalExpr:
    """Rename variables inside a goal expression.

    ``FreshExpr`` introduces a new variable scope, so its template variables
    shadow the incoming mapping.
    """

    if isinstance(goal, RelationCall):
        return RelationCall(
            relation=goal.relation,
            args=tuple(_rename_term(argument, mapping) for argument in goal.args),
        )
    if isinstance(goal, EqExpr):
        return EqExpr(
            left=_rename_term(goal.left, mapping),
            right=_rename_term(goal.right, mapping),
        )
    if isinstance(goal, NeqExpr):
        return NeqExpr(
            left=_rename_term(goal.left, mapping),
            right=_rename_term(goal.right, mapping),
        )
    if isinstance(goal, DeferredExpr):
        return DeferredExpr(
            builder=goal.builder,
            args=tuple(_rename_term(argument, mapping) for argument in goal.args),
        )
    if isinstance(goal, NativeGoalExpr):
        return NativeGoalExpr(
            runner=goal.runner,
            args=tuple(_rename_term(argument, mapping) for argument in goal.args),
        )
    if isinstance(goal, ConjExpr):
        return ConjExpr(
            goals=tuple(_rename_goal(child, mapping) for child in goal.goals),
        )
    if isinstance(goal, DisjExpr):
        return DisjExpr(
            goals=tuple(_rename_goal(child, mapping) for child in goal.goals),
        )
    if isinstance(goal, FreshExpr):
        masked_mapping = {
            variable: renamed
            for variable, renamed in mapping.items()
            if variable not in goal.template_vars
        }
        return FreshExpr(
            template_vars=goal.template_vars,
            body=_rename_goal(goal.body, masked_mapping),
        )
    return goal


def _freshen_term(
    term_value: Term,
    mapping: dict[LogicVar, LogicVar],
    next_var_id: int,
) -> tuple[Term, int]:
    """Rename ordinary clause variables into fresh runtime variables."""

    if isinstance(term_value, LogicVar):
        replacement = mapping.get(term_value)
        if replacement is not None:
            return replacement, next_var_id

        fresh_variable = LogicVar(
            id=next_var_id,
            display_name=term_value.display_name,
        )
        mapping[term_value] = fresh_variable
        return fresh_variable, next_var_id + 1

    if isinstance(term_value, Compound):
        renamed_args: list[Term] = []
        running_id = next_var_id
        for argument in term_value.args:
            renamed_argument, running_id = _freshen_term(argument, mapping, running_id)
            renamed_args.append(renamed_argument)
        return (
            Compound(functor=term_value.functor, args=tuple(renamed_args)),
            running_id,
        )

    return term_value, next_var_id


def _freshen_goal(
    goal: GoalExpr,
    mapping: dict[LogicVar, LogicVar],
    next_var_id: int,
) -> tuple[GoalExpr, int]:
    """Standardize a clause body apart from the current search state."""

    if isinstance(goal, RelationCall):
        renamed_args: list[Term] = []
        running_id = next_var_id
        for argument in goal.args:
            renamed_argument, running_id = _freshen_term(argument, mapping, running_id)
            renamed_args.append(renamed_argument)
        return (
            RelationCall(relation=goal.relation, args=tuple(renamed_args)),
            running_id,
        )

    if isinstance(goal, EqExpr):
        left, running_id = _freshen_term(goal.left, mapping, next_var_id)
        right, running_id = _freshen_term(goal.right, mapping, running_id)
        return EqExpr(left=left, right=right), running_id

    if isinstance(goal, NeqExpr):
        left, running_id = _freshen_term(goal.left, mapping, next_var_id)
        right, running_id = _freshen_term(goal.right, mapping, running_id)
        return NeqExpr(left=left, right=right), running_id

    if isinstance(goal, DeferredExpr):
        renamed_args: list[Term] = []
        running_id = next_var_id
        for argument in goal.args:
            renamed_argument, running_id = _freshen_term(
                argument,
                mapping,
                running_id,
            )
            renamed_args.append(renamed_argument)
        return (
            DeferredExpr(builder=goal.builder, args=tuple(renamed_args)),
            running_id,
        )

    if isinstance(goal, NativeGoalExpr):
        renamed_args: list[Term] = []
        running_id = next_var_id
        for argument in goal.args:
            renamed_argument, running_id = _freshen_term(
                argument,
                mapping,
                running_id,
            )
            renamed_args.append(renamed_argument)
        return (
            NativeGoalExpr(runner=goal.runner, args=tuple(renamed_args)),
            running_id,
        )

    if isinstance(goal, ConjExpr):
        running_id = next_var_id
        renamed_goals: list[GoalExpr] = []
        for child in goal.goals:
            renamed_child, running_id = _freshen_goal(child, mapping, running_id)
            renamed_goals.append(renamed_child)
        return ConjExpr(goals=tuple(renamed_goals)), running_id

    if isinstance(goal, DisjExpr):
        running_id = next_var_id
        renamed_goals: list[GoalExpr] = []
        for child in goal.goals:
            renamed_child, running_id = _freshen_goal(child, mapping, running_id)
            renamed_goals.append(renamed_child)
        return DisjExpr(goals=tuple(renamed_goals)), running_id

    if isinstance(goal, FreshExpr):
        masked_mapping = {
            variable: renamed
            for variable, renamed in mapping.items()
            if variable not in goal.template_vars
        }
        renamed_body, running_id = _freshen_goal(goal.body, masked_mapping, next_var_id)
        return (
            FreshExpr(template_vars=goal.template_vars, body=renamed_body),
            running_id,
        )

    return goal, next_var_id


def _freshen_clause(clause: Clause, next_var_id: int) -> tuple[Clause, int]:
    """Standardize an entire clause apart for one clause application."""

    mapping: dict[LogicVar, LogicVar] = {}
    renamed_head, running_id = _freshen_goal(clause.head, mapping, next_var_id)
    assert isinstance(renamed_head, RelationCall)

    if clause.body is None:
        return Clause(head=renamed_head), running_id

    renamed_body, running_id = _freshen_goal(clause.body, mapping, running_id)
    return Clause(head=renamed_head, body=renamed_body), running_id


def freshen_clause(clause_value: object, next_var_id: int) -> tuple[Clause, int]:
    """Return a clause standardized apart from an active search state."""

    checked_clause = _require_clause(clause_value)
    if isinstance(next_var_id, bool) or not isinstance(next_var_id, int):
        msg = "freshen_clause() requires an integer next_var_id"
        raise TypeError(msg)
    if next_var_id < 0:
        msg = "freshen_clause() requires a non-negative next_var_id"
        raise ValueError(msg)
    return _freshen_clause(checked_clause, next_var_id)


def _empty_dynamic_database() -> DynamicDatabase:
    """Return an empty dynamic runtime database overlay."""

    return DynamicDatabase()


def _runtime_database(state: State) -> DynamicDatabase:
    """Read the branch-local database overlay from a search state."""

    if isinstance(state.database, DynamicDatabase):
        return state.database
    return _empty_dynamic_database()


def _state_with_database(state: State, database: DynamicDatabase) -> State:
    """Return ``state`` with the supplied runtime database overlay."""

    return State(
        substitution=state.substitution,
        constraints=state.constraints,
        next_var_id=state.next_var_id,
        database=database,
        fd_store=state.fd_store,
        prolog_flags=state.prolog_flags,
    )


def _state_with_next_var_id(state: State, next_var_id: int) -> State:
    """Return ``state`` after reserving fresh runtime variable ids."""

    return State(
        substitution=state.substitution,
        constraints=state.constraints,
        next_var_id=next_var_id,
        database=state.database,
        fd_store=state.fd_store,
        prolog_flags=state.prolog_flags,
    )


def _program_has_source_clauses(program_value: Program, key: RelationKey) -> bool:
    """Return True when the immutable program contains clauses for ``key``."""

    return bool(program_value._indexed_clauses.get(key, ()))


def _is_program_dynamic(
    program_value: Program,
    key: RelationKey,
    database: DynamicDatabase,
) -> bool:
    """Return True when a program-level dynamic declaration is currently active."""

    return (
        key in program_value.dynamic_relations
        and key not in database.abolished_relations
    )


def _is_dynamic_key(
    program_value: Program,
    key: RelationKey,
    database: DynamicDatabase,
) -> bool:
    """Return True when ``key`` is mutable in the current proof branch."""

    return key in database.declared_relations or _is_program_dynamic(
        program_value,
        key,
        database,
    )


def _dynamic_clauses_for(
    clauses: tuple[Clause, ...],
    key: RelationKey,
) -> tuple[_VisibleClause, ...]:
    """Filter asserted runtime clauses by relation key."""

    return tuple(
        _VisibleClause(clause=clause, source="asserted", index=index)
        for index, clause in enumerate(clauses)
        if clause.head.relation.key() == key
    )


def _visible_dynamic_clauses_for(
    program_value: Program,
    relation_value: Relation,
    database: DynamicDatabase,
) -> tuple[_VisibleClause, ...]:
    """Return dynamic clauses visible for one relation in proof order."""

    key = relation_value.key()
    if not _is_dynamic_key(program_value, key, database):
        return ()

    visible: list[_VisibleClause] = []
    visible.extend(
        _VisibleClause(clause=item.clause, source="prepended", index=item.index)
        for item in _dynamic_clauses_for(database.prepended_clauses, key)
    )
    visible.extend(
        _VisibleClause(clause=clause, source="program", index=index)
        for index, clause in program_value._indexed_clauses.get(key, ())
        if index not in database.removed_program_clause_indexes
    )
    visible.extend(
        _VisibleClause(clause=item.clause, source="appended", index=item.index)
        for item in _dynamic_clauses_for(database.appended_clauses, key)
    )
    return tuple(visible)


def visible_clauses_for(
    program_value: Program,
    relation_value: Relation,
    state: State | None = None,
) -> tuple[Clause, ...]:
    """Return source and runtime clauses visible for ``relation_value``."""

    checked_program = _require_program(program_value)
    checked_relation = _require_relation(relation_value)
    if state is None:
        return checked_program.clauses_for(checked_relation)

    database = _runtime_database(state)
    key = checked_relation.key()
    if not _is_dynamic_key(checked_program, key, database):
        return checked_program.clauses_for(checked_relation)

    return tuple(
        item.clause
        for item in _visible_dynamic_clauses_for(
            checked_program,
            checked_relation,
            database,
        )
    )


def visible_clauses(
    program_value: Program,
    state: State | None = None,
) -> tuple[Clause, ...]:
    """Return every clause visible in source/search order."""

    checked_program = _require_program(program_value)
    if state is None:
        return checked_program.clauses

    ordered: list[Clause] = []
    seen_keys: dict[RelationKey, None] = {}
    database = _runtime_database(state)

    for clause in database.prepended_clauses:
        seen_keys.setdefault(clause.head.relation.key(), None)
    for clause in checked_program.clauses:
        seen_keys.setdefault(clause.head.relation.key(), None)
    for clause in database.appended_clauses:
        seen_keys.setdefault(clause.head.relation.key(), None)
    for key in checked_program.dynamic_relations | database.declared_relations:
        seen_keys.setdefault(key, None)

    for key in seen_keys:
        relation_value = Relation(symbol=key[0], arity=key[1])
        ordered.extend(visible_clauses_for(checked_program, relation_value, state))

    return tuple(ordered)


def visible_predicate_keys(
    program_value: Program,
    state: State | None = None,
) -> tuple[RelationKey, ...]:
    """Return visible predicate indicators in deterministic discovery order."""

    checked_program = _require_program(program_value)
    ordered: dict[RelationKey, None] = {}
    for clause in visible_clauses(checked_program, state):
        ordered.setdefault(clause.head.relation.key(), None)

    if state is not None:
        database = _runtime_database(state)
        for key in checked_program.dynamic_relations:
            if key not in database.abolished_relations:
                ordered.setdefault(key, None)
        for key in database.declared_relations:
            ordered.setdefault(key, None)

    return tuple(ordered)


def visible_clause_count(
    program_value: Program,
    relation_value: Relation,
    state: State | None = None,
) -> int:
    """Return the number of visible clauses for one relation."""

    return len(visible_clauses_for(program_value, relation_value, state))


def is_dynamic_relation(
    program_value: Program,
    relation_value: Relation,
    state: State | None = None,
) -> bool:
    """Return True when a relation is dynamic in the supplied state."""

    checked_program = _require_program(program_value)
    checked_relation = _require_relation(relation_value)
    key = checked_relation.key()
    if state is None:
        return key in checked_program.dynamic_relations
    return _is_dynamic_key(checked_program, key, _runtime_database(state))


def runtime_declare_dynamic(
    program_value: Program,
    state: State,
    relation_value: Relation,
) -> State | None:
    """Declare a branch-local dynamic predicate, failing for static sources."""

    checked_program = _require_program(program_value)
    checked_relation = _require_relation(relation_value)
    key = checked_relation.key()
    database = _runtime_database(state)

    if (
        _program_has_source_clauses(checked_program, key)
        and key not in checked_program.dynamic_relations
    ):
        return None

    return _state_with_database(
        state,
        DynamicDatabase(
            declared_relations=database.declared_relations | frozenset({key}),
            abolished_relations=database.abolished_relations - frozenset({key}),
            prepended_clauses=database.prepended_clauses,
            appended_clauses=database.appended_clauses,
            removed_program_clause_indexes=database.removed_program_clause_indexes,
        ),
    )


def _require_runtime_dynamic(
    program_value: Program,
    state: State,
    key: RelationKey,
) -> DynamicDatabase | None:
    """Return the runtime database when ``key`` may be mutated."""

    database = _runtime_database(state)
    if not _is_dynamic_key(program_value, key, database):
        return None
    return database


def runtime_asserta(
    program_value: Program,
    state: State,
    clause_value: Clause,
) -> State | None:
    """Return a new state with ``clause_value`` before visible dynamic clauses."""

    checked_program = _require_program(program_value)
    checked_clause = _require_clause(clause_value)
    key = checked_clause.head.relation.key()
    database = _require_runtime_dynamic(checked_program, state, key)
    if database is None:
        return None
    return _state_with_database(
        state,
        DynamicDatabase(
            declared_relations=database.declared_relations,
            abolished_relations=database.abolished_relations,
            prepended_clauses=(checked_clause, *database.prepended_clauses),
            appended_clauses=database.appended_clauses,
            removed_program_clause_indexes=database.removed_program_clause_indexes,
        ),
    )


def runtime_assertz(
    program_value: Program,
    state: State,
    clause_value: Clause,
) -> State | None:
    """Return a new state with ``clause_value`` after visible dynamic clauses."""

    checked_program = _require_program(program_value)
    checked_clause = _require_clause(clause_value)
    key = checked_clause.head.relation.key()
    database = _require_runtime_dynamic(checked_program, state, key)
    if database is None:
        return None
    return _state_with_database(
        state,
        DynamicDatabase(
            declared_relations=database.declared_relations,
            abolished_relations=database.abolished_relations,
            prepended_clauses=database.prepended_clauses,
            appended_clauses=(*database.appended_clauses, checked_clause),
            removed_program_clause_indexes=database.removed_program_clause_indexes,
        ),
    )


def _remove_visible_clause(
    database: DynamicDatabase,
    visible_clause: _VisibleClause,
) -> DynamicDatabase:
    """Return a runtime database with one visible dynamic clause removed."""

    if visible_clause.source == "prepended":
        return DynamicDatabase(
            declared_relations=database.declared_relations,
            abolished_relations=database.abolished_relations,
            prepended_clauses=tuple(
                clause
                for index, clause in enumerate(database.prepended_clauses)
                if index != visible_clause.index
            ),
            appended_clauses=database.appended_clauses,
            removed_program_clause_indexes=database.removed_program_clause_indexes,
        )
    if visible_clause.source == "appended":
        return DynamicDatabase(
            declared_relations=database.declared_relations,
            abolished_relations=database.abolished_relations,
            prepended_clauses=database.prepended_clauses,
            appended_clauses=tuple(
                clause
                for index, clause in enumerate(database.appended_clauses)
                if index != visible_clause.index
            ),
            removed_program_clause_indexes=database.removed_program_clause_indexes,
        )
    return DynamicDatabase(
        declared_relations=database.declared_relations,
        abolished_relations=database.abolished_relations,
        prepended_clauses=database.prepended_clauses,
        appended_clauses=database.appended_clauses,
        removed_program_clause_indexes=database.removed_program_clause_indexes
        | frozenset({visible_clause.index}),
    )


def _clause_match_term(clause_value: Clause, *, fact_only: bool) -> Term | None:
    """Return the term shape used for retract matching."""

    if fact_only:
        if clause_value.body is not None:
            return None
        return clause_value.head.as_term()
    try:
        return clause_as_term(clause_value)
    except TypeError:
        return None


def runtime_retract_first(
    program_value: Program,
    state: State,
    clause_pattern: Clause,
) -> Iterator[State]:
    """Yield states for retracting the first matching dynamic clause."""

    checked_program = _require_program(program_value)
    checked_pattern = _require_clause(clause_pattern)
    key = checked_pattern.head.relation.key()
    database = _require_runtime_dynamic(checked_program, state, key)
    if database is None:
        return

    pattern_term = _clause_match_term(
        checked_pattern,
        fact_only=checked_pattern.body is None,
    )
    assert pattern_term is not None

    for visible_clause in _visible_dynamic_clauses_for(
        checked_program,
        checked_pattern.head.relation,
        database,
    ):
        fresh_clause, next_var_id = _freshen_clause(
            visible_clause.clause,
            state.next_var_id,
        )
        candidate_term = _clause_match_term(
            fresh_clause,
            fact_only=checked_pattern.body is None,
        )
        if candidate_term is None:
            continue

        for unified_state in core_eq(pattern_term, candidate_term)(state):
            updated_database = _remove_visible_clause(database, visible_clause)
            yield _state_with_database(
                _state_with_next_var_id(unified_state, next_var_id),
                updated_database,
            )
            return


def runtime_retract_all(
    program_value: Program,
    state: State,
    head_pattern: RelationCall,
) -> State | None:
    """Return a state with every dynamic clause matching ``head_pattern`` removed."""

    checked_program = _require_program(program_value)
    checked_head = _require_relation_call(head_pattern)
    key = checked_head.relation.key()
    database = _require_runtime_dynamic(checked_program, state, key)
    if database is None:
        return None

    remove_prepended: set[int] = set()
    remove_appended: set[int] = set()
    remove_program: set[int] = set()
    for visible_clause in _visible_dynamic_clauses_for(
        checked_program,
        checked_head.relation,
        database,
    ):
        if unify(
            checked_head.as_term(),
            visible_clause.clause.head.as_term(),
            state.substitution,
        ) is not None:
            if visible_clause.source == "prepended":
                remove_prepended.add(visible_clause.index)
            elif visible_clause.source == "appended":
                remove_appended.add(visible_clause.index)
            else:
                remove_program.add(visible_clause.index)

    return _state_with_database(
        state,
        DynamicDatabase(
            declared_relations=database.declared_relations,
            abolished_relations=database.abolished_relations,
            prepended_clauses=tuple(
                clause
                for index, clause in enumerate(database.prepended_clauses)
                if index not in remove_prepended
            ),
            appended_clauses=tuple(
                clause
                for index, clause in enumerate(database.appended_clauses)
                if index not in remove_appended
            ),
            removed_program_clause_indexes=database.removed_program_clause_indexes
            | frozenset(remove_program),
        ),
    )


def runtime_abolish(
    program_value: Program,
    state: State,
    relation_value: Relation,
) -> State | None:
    """Return a state where a dynamic predicate is abolished in this branch."""

    checked_program = _require_program(program_value)
    checked_relation = _require_relation(relation_value)
    key = checked_relation.key()
    database = _require_runtime_dynamic(checked_program, state, key)
    if database is None:
        return None

    return _state_with_database(
        state,
        DynamicDatabase(
            declared_relations=database.declared_relations - frozenset({key}),
            abolished_relations=database.abolished_relations | frozenset({key}),
            prepended_clauses=tuple(
                clause
                for clause in database.prepended_clauses
                if clause.head.relation.key() != key
            ),
            appended_clauses=tuple(
                clause
                for clause in database.appended_clauses
                if clause.head.relation.key() != key
            ),
            removed_program_clause_indexes=database.removed_program_clause_indexes
            | frozenset(
                index
                for index, _clause in checked_program._indexed_clauses.get(key, ())
            ),
        ),
    )


def _instantiate_fresh(goal: FreshExpr, state: State) -> tuple[GoalExpr, State]:
    """Allocate runtime variables for a ``FreshExpr`` scope."""

    mapping: dict[LogicVar, LogicVar] = {}
    running_id = state.next_var_id
    for template_var in goal.template_vars:
        mapping[template_var] = LogicVar(
            id=running_id,
            display_name=template_var.display_name,
        )
        running_id += 1

    renamed_body = _rename_goal(goal.body, mapping)
    next_state = State(
        substitution=state.substitution,
        constraints=state.constraints,
        next_var_id=running_id,
        database=state.database,
        fd_store=state.fd_store,
        prolog_flags=state.prolog_flags,
    )
    return renamed_body, next_state


def _solve_goal_results(
    program_value: Program,
    goal: GoalExpr,
    state: State,
) -> Iterator[_SearchResult]:
    """Interpret one goal expression and retain internal cut signals."""

    if isinstance(goal, SucceedExpr):
        yield _SearchResult(state)
        return

    if isinstance(goal, FailExpr):
        return

    if isinstance(goal, CutExpr):
        yield _SearchResult(state, cut=True)
        return

    if isinstance(goal, EqExpr):
        for next_state in core_eq(goal.left, goal.right)(state):
            yield _SearchResult(next_state)
        return

    if isinstance(goal, NeqExpr):
        for next_state in core_neq(goal.left, goal.right)(state):
            yield _SearchResult(next_state)
        return

    if isinstance(goal, NativeGoalExpr):
        for next_state in goal.runner(program_value, state, goal.args):
            yield _SearchResult(next_state)
        return

    if isinstance(goal, ConjExpr):
        yield from _solve_conjunction_results(program_value, goal.goals, state)
        return

    if isinstance(goal, DisjExpr):
        for branch in goal.goals:
            branch_cut = False
            for result in _solve_goal_results(program_value, branch, state):
                branch_cut = branch_cut or result.cut
                yield result
            if branch_cut:
                break
        return

    if isinstance(goal, DeferredExpr):
        expanded_goal = _coerce_goal(goal.builder(*goal.args))
        yield from _solve_goal_results(program_value, expanded_goal, state)
        return

    if isinstance(goal, FreshExpr):
        renamed_body, next_state = _instantiate_fresh(goal, state)
        yield from _solve_goal_results(program_value, renamed_body, next_state)
        return

    for clause in visible_clauses_for(program_value, goal.relation, state):
        clause_cut = False
        fresh_clause, next_var_id = _freshen_clause(clause, state.next_var_id)
        for unified_state in core_eq(goal.as_term(), fresh_clause.head.as_term())(
            state,
        ):
            clause_state = _state_with_next_var_id(unified_state, next_var_id)
            if fresh_clause.body is None:
                yield _SearchResult(clause_state)
            else:
                for result in _solve_goal_results(
                    program_value,
                    fresh_clause.body,
                    clause_state,
                ):
                    clause_cut = clause_cut or result.cut
                    if result.state is not None:
                        yield _SearchResult(result.state)
                if clause_cut:
                    break
        if clause_cut:
            break


def _solve_conjunction_results(
    program_value: Program,
    goals: tuple[GoalExpr, ...],
    state: State,
) -> Iterator[_SearchResult]:
    """Solve conjunctions left-to-right while propagating scoped cuts."""

    if not goals:
        yield _SearchResult(state)
        return

    first, *rest = goals
    for first_result in _solve_goal_results(program_value, first, state):
        cut_seen = first_result.cut
        if first_result.state is None:
            if cut_seen:
                yield _SearchResult(None, cut=True)
                break
            continue
        emitted_rest_result = False
        for rest_result in _solve_conjunction_results(
            program_value,
            tuple(rest),
            first_result.state,
        ):
            emitted_rest_result = True
            cut_seen = cut_seen or rest_result.cut
            if rest_result.state is None:
                yield _SearchResult(None, cut=cut_seen)
            else:
                yield _SearchResult(rest_result.state, cut=cut_seen)
        if first_result.cut and not emitted_rest_result:
            yield _SearchResult(None, cut=True)
        if cut_seen:
            break


def _solve_goal(
    program_value: Program,
    goal: GoalExpr,
    state: State,
) -> Iterator[State]:
    """Interpret one goal expression against ``program_value``."""

    for result in _solve_goal_results(program_value, goal, state):
        if result.state is not None:
            yield result.state


def solve(program_value: Program, goal: object) -> Iterator[State]:
    """Solve ``goal`` against ``program_value`` from the empty state."""

    yield from _solve_goal(program_value, _coerce_goal(goal), State())


def solve_from(program_value: Program, goal: object, state: State) -> Iterator[State]:
    """Solve ``goal`` against ``program_value`` starting from ``state``."""

    yield from _solve_goal(program_value, _coerce_goal(goal), state)


def solve_all(
    program_value: Program,
    query: object | tuple[object, ...],
    goal: object,
) -> list[Term | tuple[Term, ...]]:
    """Collect every answer for ``query`` under ``goal``."""

    results: list[Term | tuple[Term, ...]] = []
    for state in solve(program_value, goal):
        if isinstance(query, tuple):
            results.append(tuple(reify(item, state.substitution) for item in query))
        else:
            results.append(reify(query, state.substitution))
    return results


def solve_n(
    program_value: Program,
    n: int,
    query: object | tuple[object, ...],
    goal: object,
) -> list[Term | tuple[Term, ...]]:
    """Collect at most ``n`` answers for ``query`` under ``goal``."""

    if n < 0:
        msg = "solve_n() requires a non-negative limit"
        raise ValueError(msg)

    results: list[Term | tuple[Term, ...]] = []
    for state in islice(solve(program_value, goal), n):
        if isinstance(query, tuple):
            results.append(tuple(reify(item, state.substitution) for item in query))
        else:
            results.append(reify(query, state.substitution))
    return results
