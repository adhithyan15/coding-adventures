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

from collections.abc import Callable, Iterator

from logic_engine import (
    Atom,
    Compound,
    GoalExpr,
    LogicVar,
    Number,
    Program,
    State,
    String,
    Term,
    atom,
    conj,
    eq,
    native_goal,
    num,
    reify,
    solve_from,
    term,
)

__all__ = [
    "add",
    "argo",
    "atomo",
    "callo",
    "compoundo",
    "div",
    "floordiv",
    "functoro",
    "geqo",
    "gto",
    "groundo",
    "iso",
    "leqo",
    "lto",
    "mod",
    "mul",
    "neg",
    "nonvaro",
    "noto",
    "numeqo",
    "numneqo",
    "numbero",
    "onceo",
    "stringo",
    "sub",
    "varo",
]


type NativeArgs = tuple[Term, ...]
type NativeRunner = Callable[[Program, State, NativeArgs], Iterator[State]]
type NumericValue = int | float


def _as_goal(goal: object) -> GoalExpr:
    """Validate and normalize one goal-like object using the engine API."""

    return conj(goal)


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


def _succeed_if(condition: bool, state: State) -> Iterator[State]:
    """Yield the current state exactly when a builtin predicate succeeds."""

    if condition:
        yield state


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


def iso(result: object, expression: object) -> GoalExpr:
    """Evaluate an arithmetic expression and unify the numeric result."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        result_term, expression_term = args
        value = _numeric_value(expression_term, state)
        if value is None:
            return
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


def numeqo(left: object, right: object) -> GoalExpr:
    """Succeed when two arithmetic expressions evaluate to equal numbers."""

    return _numeric_compareo(
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


def lto(left: object, right: object) -> GoalExpr:
    """Succeed when the left arithmetic expression is less than the right."""

    return _numeric_compareo(
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


def gto(left: object, right: object) -> GoalExpr:
    """Succeed when the left arithmetic expression is greater than the right."""

    return _numeric_compareo(
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


def callo(goal: object) -> GoalExpr:
    """Run a goal supplied as data.

    Python callers already hold goal objects directly, so this is mostly a
    composability adapter and a stepping stone toward Prolog's `call/1`.
    """

    called_goal = _as_goal(goal)

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        yield from solve_from(program_value, called_goal, state)

    return native_goal(run)


def onceo(goal: object) -> GoalExpr:
    """Run `goal` and keep at most its first solution."""

    called_goal = _as_goal(goal)

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


def stringo(term_value: object) -> GoalExpr:
    """Succeed when the current value is a string term."""

    return _type_checko(term_value, String)


def compoundo(term_value: object) -> GoalExpr:
    """Succeed when the current value is a compound term."""

    return _type_checko(term_value, Compound)


def functoro(term_value: object, name: object, arity: object) -> GoalExpr:
    """Inspect a compound term's functor name and arity.

    This first slice supports inspection mode. Construction mode can come later
    once the runtime has a clearer policy for using partially instantiated
    structural builtins to allocate new compound terms.
    """

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        target, name_target, arity_target = args
        reified_target = _reified(target, state)
        if not isinstance(reified_target, Compound):
            return

        goal = conj(
            eq(name_target, atom(reified_target.functor)),
            eq(arity_target, num(len(reified_target.args))),
        )
        yield from solve_from(program_value, goal, state)

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
