"""logic-core — a Python-first relational programming prototype.

This package is the semantic core that later Prolog work will build on top of.
It intentionally starts with the timeless pieces of logic programming rather
than syntax:

- symbolic terms
- logic variables
- substitutions
- unification
- goals
- backtracking search

The design is "library first." A future Prolog parser should lower source code
into these objects rather than inventing a parallel execution model.
"""

from __future__ import annotations

from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from itertools import count, islice

from symbol_core import Symbol, sym

__all__ = [
    "Atom",
    "Compound",
    "Disequality",
    "Goal",
    "LogicVar",
    "Number",
    "State",
    "String",
    "Substitution",
    "Term",
    "__version__",
    "atom",
    "conj",
    "disj",
    "eq",
    "fail",
    "fresh",
    "logic_list",
    "neq",
    "num",
    "reify",
    "run",
    "run_all",
    "run_n",
    "string",
    "succeed",
    "term",
    "unify",
    "var",
]

__version__ = "0.4.0"


@dataclass(frozen=True, slots=True)
class Atom:
    """A zero-arity symbolic constant such as ``homer`` or ``[]``."""

    symbol: Symbol

    def __str__(self) -> str:
        return str(self.symbol)

    def __repr__(self) -> str:
        return f"Atom({self.symbol!r})"


@dataclass(frozen=True, slots=True)
class Number:
    """A numeric logic term used for arithmetic-friendly examples."""

    value: int | float

    def __post_init__(self) -> None:
        if isinstance(self.value, bool):
            msg = "bool values are not valid Number terms"
            raise TypeError(msg)
        if not isinstance(self.value, int | float):
            msg = "Number terms require int or float values"
            raise TypeError(msg)

    def __str__(self) -> str:
        return str(self.value)


@dataclass(frozen=True, slots=True)
class String:
    """A quoted string term distinct from atoms."""

    value: str

    def __str__(self) -> str:
        return repr(self.value)


@dataclass(frozen=True, slots=True, eq=False)
class LogicVar:
    """A bindable variable whose identity is its numeric id.

    ``display_name`` is pedagogical metadata only. Two variables with the same
    display name are still different variables unless they share the same id.
    """

    id: int
    display_name: Symbol | None = field(default=None, compare=False)

    def __hash__(self) -> int:
        return hash(self.id)

    def __eq__(self, other: object) -> bool:
        return isinstance(other, LogicVar) and self.id == other.id

    def __str__(self) -> str:
        if self.display_name is None:
            return f"_{self.id}"
        return str(self.display_name)

    def __repr__(self) -> str:
        if self.display_name is None:
            return f"LogicVar(id={self.id})"
        return f"LogicVar(id={self.id}, display_name={self.display_name!r})"


@dataclass(frozen=True, slots=True)
class Compound:
    """A functor symbol plus an ordered tuple of argument terms."""

    functor: Symbol
    args: tuple[Term, ...]

    def __str__(self) -> str:
        rendered_args = ", ".join(str(argument) for argument in self.args)
        return f"{self.functor}({rendered_args})"

    def __repr__(self) -> str:
        return f"Compound(functor={self.functor!r}, args={self.args!r})"


type Term = Atom | Number | String | LogicVar | Compound


def atom(name: str | Symbol) -> Atom:
    """Construct an atom from either a raw name or an existing symbol."""

    if isinstance(name, Symbol):
        return Atom(name)
    return Atom(sym(name))


def num(value: int | float) -> Number:
    """Construct a numeric term."""

    return Number(value)


def string(value: str) -> String:
    """Construct a quoted string term."""

    return String(value)


_QUERY_VAR_IDS = count(start=-1, step=-1)


def var(name: str | Symbol | None = None) -> LogicVar:
    """Construct a user-facing logic variable with a globally unique id.

    Query variables live in the negative id range. The search engine uses
    non-negative ids for ``fresh()`` variables, so the two namespaces never
    collide.
    """

    display_name: Symbol | None
    if name is None:
        display_name = None
    elif isinstance(name, Symbol):
        display_name = name
    else:
        display_name = sym(name)

    return LogicVar(id=next(_QUERY_VAR_IDS), display_name=display_name)


def _coerce_term(value: object) -> Term:
    """Convert host-language convenience inputs into logic terms."""

    if isinstance(value, Atom | Number | String | LogicVar | Compound):
        return value
    if isinstance(value, Symbol):
        return atom(value)
    if isinstance(value, bool):
        msg = (
            "bool values are ambiguous in logic-core; use atoms or "
            "numbers explicitly"
        )
        raise TypeError(msg)
    if isinstance(value, int | float):
        return num(value)
    if isinstance(value, str):
        return atom(value)

    msg = f"cannot coerce {type(value).__name__} into a logic term"
    raise TypeError(msg)


def term(functor: str | Symbol, *args: object) -> Compound:
    """Construct a compound term such as ``parent(homer, bart)``."""

    functor_symbol = functor if isinstance(functor, Symbol) else sym(functor)

    return Compound(
        functor=functor_symbol,
        args=tuple(_coerce_term(argument) for argument in args),
    )


def logic_list(items: list[object], tail: object | None = None) -> Term:
    """Build a canonical Prolog-style list from host-language items.

    ``logic_list([a, b, c])`` becomes ``.(a, .(b, .(c, [])))``.
    """

    current_tail = atom("[]") if tail is None else _coerce_term(tail)
    for item in reversed(items):
        current_tail = term(".", _coerce_term(item), current_tail)
    return current_tail


@dataclass(frozen=True, slots=True)
class Substitution:
    """A persistent mapping from logic variables to terms."""

    bindings: dict[LogicVar, Term] = field(default_factory=dict)

    def get(self, variable: LogicVar) -> Term | None:
        """Return the direct binding for ``variable`` if it exists."""

        return self.bindings.get(variable)

    def walk(self, value: Term) -> Term:
        """Follow variable bindings until a non-variable term is reached."""

        current = value
        while isinstance(current, LogicVar):
            replacement = self.bindings.get(current)
            if replacement is None:
                return current
            current = replacement
        return current

    def extend(self, variable: LogicVar, value: Term) -> Substitution:
        """Return a new substitution with one additional binding."""

        updated = dict(self.bindings)
        updated[variable] = value
        return Substitution(bindings=updated)

    def reify(self, value: Term) -> Term:
        """Convenience wrapper around the module-level ``reify()`` helper."""

        return reify(value, self)


@dataclass(frozen=True, slots=True)
class Disequality:
    """A delayed constraint that two terms must never become equal."""

    left: Term
    right: Term


@dataclass(frozen=True, slots=True)
class State:
    """One point in the search tree."""

    substitution: Substitution = field(default_factory=Substitution)
    constraints: tuple[Disequality, ...] = ()
    next_var_id: int = 0
    database: object | None = None
    fd_store: object | None = None
    prolog_flags: object | None = None


type Goal = Callable[[State], Iterator[State]]


def _occurs(variable: LogicVar, value: Term, substitution: Substitution) -> bool:
    """Detect cyclic bindings for the occurs check."""

    walked = substitution.walk(value)
    if walked == variable:
        return True
    if isinstance(walked, Compound):
        return any(
            _occurs(variable, argument, substitution)
            for argument in walked.args
        )
    return False


def unify(
    left: object,
    right: object,
    substitution: Substitution | None = None,
    *,
    occurs_check: bool = True,
) -> Substitution | None:
    """Unify two terms under the supplied substitution.

    Success returns a new substitution. Failure returns ``None``.
    """

    current = Substitution() if substitution is None else substitution
    left_term = current.walk(_coerce_term(left))
    right_term = current.walk(_coerce_term(right))

    if left_term == right_term:
        return current

    if isinstance(left_term, LogicVar):
        if occurs_check and _occurs(left_term, right_term, current):
            return None
        return current.extend(left_term, right_term)

    if isinstance(right_term, LogicVar):
        if occurs_check and _occurs(right_term, left_term, current):
            return None
        return current.extend(right_term, left_term)

    if isinstance(left_term, Compound) and isinstance(right_term, Compound):
        if left_term.functor != right_term.functor:
            return None
        if len(left_term.args) != len(right_term.args):
            return None

        running = current
        for left_argument, right_argument in zip(
            left_term.args,
            right_term.args,
            strict=True,
        ):
            running = unify(
                left_argument,
                right_argument,
                running,
                occurs_check=occurs_check,
            )
            if running is None:
                return None
        return running

    return None


def reify(value: object, substitution: Substitution) -> Term:
    """Resolve a term against a substitution until only concrete structure remains."""

    walked = substitution.walk(_coerce_term(value))
    if isinstance(walked, Compound):
        return Compound(
            functor=walked.functor,
            args=tuple(reify(argument, substitution) for argument in walked.args),
        )
    return walked


def _reconcile_disequalities(
    substitution: Substitution,
    constraints: tuple[Disequality, ...],
) -> tuple[Disequality, ...] | None:
    """Re-check delayed disequalities after a substitution changes.

    Each constraint falls into one of three buckets:

    - violated: the two sides are equal now -> fail the whole state
    - satisfied: the two sides can no longer unify -> drop the constraint
    - pending: equality is still possible later -> keep a normalized copy
    """

    pending: list[Disequality] = []
    for constraint in constraints:
        left = reify(constraint.left, substitution)
        right = reify(constraint.right, substitution)

        if left == right:
            return None

        trial = unify(left, right, substitution)
        if trial is None:
            continue

        pending.append(Disequality(left=left, right=right))

    return tuple(pending)


def succeed() -> Goal:
    """Return a goal that yields its input state unchanged."""

    def goal(state: State) -> Iterator[State]:
        yield state

    return goal


def fail() -> Goal:
    """Return a goal that yields no successor states."""

    def goal(state: State) -> Iterator[State]:
        if False:
            yield state

    return goal


def eq(left: object, right: object) -> Goal:
    """Create a goal that attempts to unify two terms."""

    def goal(state: State) -> Iterator[State]:
        unified = unify(left, right, state.substitution)
        if unified is None:
            return
        constraints = _reconcile_disequalities(unified, state.constraints)
        if constraints is None:
            return
        yield State(
            substitution=unified,
            constraints=constraints,
            next_var_id=state.next_var_id,
            database=state.database,
            fd_store=state.fd_store,
            prolog_flags=state.prolog_flags,
        )

    return goal


def neq(left: object, right: object) -> Goal:
    """Create a goal that enforces disequality, immediately or lazily.

    If the terms are already provably different, the goal succeeds immediately.
    If they are already equal, the goal fails immediately.
    Otherwise the engine stores a delayed disequality constraint that future
    unifications must continue to respect.
    """

    def goal(state: State) -> Iterator[State]:
        normalized = Disequality(
            left=reify(left, state.substitution),
            right=reify(right, state.substitution),
        )

        if normalized.left == normalized.right:
            return

        trial = unify(normalized.left, normalized.right, state.substitution)
        if trial is None:
            yield state
            return

        constraints = _reconcile_disequalities(
            state.substitution,
            state.constraints + (normalized,),
        )
        if constraints is None:
            return

        yield State(
            substitution=state.substitution,
            constraints=constraints,
            next_var_id=state.next_var_id,
            database=state.database,
            fd_store=state.fd_store,
            prolog_flags=state.prolog_flags,
        )

    return goal


def disj(*goals: Goal) -> Goal:
    """Create a goal that tries each branch and concatenates their answers."""

    def goal(state: State) -> Iterator[State]:
        for branch in goals:
            yield from branch(state)

    return goal


def conj(*goals: Goal) -> Goal:
    """Create a goal that threads states through each goal in sequence."""

    def step(index: int, state: State) -> Iterator[State]:
        if index == len(goals):
            yield state
            return

        for next_state in goals[index](state):
            yield from step(index + 1, next_state)

    def goal(state: State) -> Iterator[State]:
        yield from step(0, state)

    return goal


def fresh(count: int, fn: Callable[..., Goal]) -> Goal:
    """Allocate fresh search-local variables and pass them to ``fn``."""

    if count <= 0:
        msg = "fresh() requires at least one variable"
        raise ValueError(msg)

    def goal(state: State) -> Iterator[State]:
        variables = tuple(
            LogicVar(id=state.next_var_id + index)
            for index in range(count)
        )
        next_state = State(
            substitution=state.substitution,
            constraints=state.constraints,
            next_var_id=state.next_var_id + count,
            database=state.database,
            fd_store=state.fd_store,
            prolog_flags=state.prolog_flags,
        )
        produced_goal = fn(*variables)
        yield from produced_goal(next_state)

    return goal


def run(goal: Goal) -> Iterator[State]:
    """Run a goal from the empty state."""

    yield from goal(State())


def run_all(
    query: object | tuple[object, ...],
    goal: Goal,
) -> list[Term | tuple[Term, ...]]:
    """Collect every answer for ``query`` under ``goal``."""

    results: list[Term | tuple[Term, ...]] = []
    for state in run(goal):
        if isinstance(query, tuple):
            results.append(
                tuple(reify(item, state.substitution) for item in query),
            )
        else:
            results.append(reify(query, state.substitution))
    return results


def run_n(
    n: int,
    query: object | tuple[object, ...],
    goal: Goal,
) -> list[Term | tuple[Term, ...]]:
    """Collect at most ``n`` answers for ``query`` under ``goal``."""

    if n < 0:
        msg = "run_n() requires a non-negative limit"
        raise ValueError(msg)

    results: list[Term | tuple[Term, ...]] = []
    for state in islice(run(goal), n):
        if isinstance(query, tuple):
            results.append(
                tuple(reify(item, state.substitution) for item in query),
            )
        else:
            results.append(reify(query, state.substitution))
    return results
