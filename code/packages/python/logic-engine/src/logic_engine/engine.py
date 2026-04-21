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
    "DeferredExpr",
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
    "clauses_matching",
    "conj",
    "defer",
    "disj",
    "eq",
    "fact",
    "fail",
    "fresh",
    "freshen_clause",
    "goal_as_term",
    "goal_from_term",
    "logic_list",
    "native_goal",
    "neq",
    "num",
    "program",
    "relation",
    "reify",
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
        yield from core_eq(goal.left, goal.right)(state)
        return

    if isinstance(goal, NeqExpr):
        yield from core_neq(goal.left, goal.right)(state)
        return

    if isinstance(goal, NativeGoalExpr):
        yield from goal.runner(program_value, state, goal.args)
        return

    if isinstance(goal, ConjExpr):
        yield from _solve_conjunction(program_value, goal.goals, state)
        return

    if isinstance(goal, DisjExpr):
        for branch in goal.goals:
            yield from _solve_goal(program_value, branch, state)
        return

    if isinstance(goal, DeferredExpr):
        expanded_goal = _coerce_goal(goal.builder(*goal.args))
        yield from _solve_goal(program_value, expanded_goal, state)
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
