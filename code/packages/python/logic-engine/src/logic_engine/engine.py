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

from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from itertools import count, islice

from logic_core import (
    Atom,
    Compound,
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
from symbol_core import Symbol, sym

__all__ = [
    "Atom",
    "Clause",
    "Compound",
    "ConjExpr",
    "DisjExpr",
    "EqExpr",
    "FailExpr",
    "FreshExpr",
    "GoalExpr",
    "LogicVar",
    "Number",
    "Program",
    "Relation",
    "RelationCall",
    "State",
    "String",
    "Substitution",
    "SucceedExpr",
    "Term",
    "atom",
    "conj",
    "disj",
    "eq",
    "fact",
    "fail",
    "fresh",
    "logic_list",
    "num",
    "program",
    "relation",
    "rule",
    "solve",
    "solve_all",
    "solve_n",
    "string",
    "succeed",
    "term",
    "var",
]


# Template variables live far away from query variables so the two namespaces
# do not collide before the solver remaps clause-local variables into search
# local runtime ids.
_TEMPLATE_VAR_IDS = count(start=-1_000_000, step=-1)


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
class EqExpr:
    """A structured equality goal that delegates unification to LP00."""

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


type GoalExpr = (
    RelationCall | SucceedExpr | FailExpr | EqExpr | ConjExpr | DisjExpr | FreshExpr
)


def _coerce_goal(goal: object) -> GoalExpr:
    """Validate that ``goal`` is a supported engine expression."""

    if isinstance(
        goal,
        (
            RelationCall
            | SucceedExpr
            | FailExpr
            | EqExpr
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


def eq(left: object, right: object) -> GoalExpr:
    """Construct an equality goal expression."""

    return EqExpr(left=_coerce_term(left), right=_coerce_term(right))


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
class Program:
    """An immutable clause database indexed by relation."""

    clauses: tuple[Clause, ...]
    _index: dict[tuple[Symbol, int], tuple[Clause, ...]] = field(
        init=False,
        repr=False,
        compare=False,
    )

    def __post_init__(self) -> None:
        buckets: dict[tuple[Symbol, int], list[Clause]] = {}
        for clause in self.clauses:
            if not isinstance(clause, Clause):
                msg = "Program clauses must all be Clause objects"
                raise TypeError(msg)
            buckets.setdefault(clause.head.relation.key(), []).append(clause)

        frozen_index = {
            key: tuple(grouped_clauses)
            for key, grouped_clauses in buckets.items()
        }
        object.__setattr__(self, "_index", frozen_index)

    def clauses_for(self, relation_value: Relation) -> tuple[Clause, ...]:
        """Return clauses for ``relation_value`` in source order."""

        return self._index.get(relation_value.key(), ())


def program(*clauses: Clause) -> Program:
    """Construct an immutable logic program."""

    return Program(clauses=tuple(clauses))


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
        next_var_id=running_id,
    )
    return renamed_body, next_state


def _solve_goal(
    program_value: Program,
    goal: GoalExpr,
    state: State,
) -> Iterator[State]:
    """Interpret one goal expression against ``program_value``."""

    if isinstance(goal, SucceedExpr):
        yield state
        return

    if isinstance(goal, FailExpr):
        return

    if isinstance(goal, EqExpr):
        unified = unify(goal.left, goal.right, state.substitution)
        if unified is not None:
            yield State(substitution=unified, next_var_id=state.next_var_id)
        return

    if isinstance(goal, ConjExpr):
        yield from _solve_conjunction(program_value, goal.goals, state)
        return

    if isinstance(goal, DisjExpr):
        for branch in goal.goals:
            yield from _solve_goal(program_value, branch, state)
        return

    if isinstance(goal, FreshExpr):
        renamed_body, next_state = _instantiate_fresh(goal, state)
        yield from _solve_goal(program_value, renamed_body, next_state)
        return

    for clause in program_value.clauses_for(goal.relation):
        fresh_clause, next_var_id = _freshen_clause(clause, state.next_var_id)
        unified = unify(
            goal.as_term(),
            fresh_clause.head.as_term(),
            state.substitution,
        )
        if unified is None:
            continue

        clause_state = State(substitution=unified, next_var_id=next_var_id)
        if fresh_clause.body is None:
            yield clause_state
        else:
            yield from _solve_goal(program_value, fresh_clause.body, clause_state)


def _solve_conjunction(
    program_value: Program,
    goals: tuple[GoalExpr, ...],
    state: State,
) -> Iterator[State]:
    """Solve conjunctions left-to-right, threading substitutions through."""

    if not goals:
        yield state
        return

    first, *rest = goals
    for next_state in _solve_goal(program_value, first, state):
        yield from _solve_conjunction(program_value, tuple(rest), next_state)


def solve(program_value: Program, goal: object) -> Iterator[State]:
    """Solve ``goal`` against ``program_value`` from the empty state."""

    yield from _solve_goal(program_value, _coerce_goal(goal), State())


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
