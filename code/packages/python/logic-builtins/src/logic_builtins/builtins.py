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
    RelationCall,
    State,
    String,
    Term,
    atom,
    clause_body,
    conj,
    eq,
    freshen_clause,
    goal_as_term,
    logic_list,
    native_goal,
    num,
    reify,
    solve_from,
    succeed,
    term,
)
from logic_engine import (
    fail as engine_fail,
)

__all__ = [
    "add",
    "argo",
    "atomico",
    "atomo",
    "callo",
    "callableo",
    "compoundo",
    "copytermo",
    "div",
    "failo",
    "findallo",
    "floordiv",
    "forallo",
    "functoro",
    "geqo",
    "gto",
    "groundo",
    "ifthenelseo",
    "iftheno",
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
    "bagofo",
    "same_termo",
    "setofo",
    "clauseo",
    "stringo",
    "sub",
    "trueo",
    "univo",
    "varo",
]


type NativeArgs = tuple[Term, ...]
type NativeRunner = Callable[[Program, State, NativeArgs], Iterator[State]]
type NumericValue = int | float
type TermSortKey = tuple[object, ...]


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
    )


def _succeed_if(condition: bool, state: State) -> Iterator[State]:
    """Yield the current state exactly when a builtin predicate succeeds."""

    if condition:
        yield state


def _term_sort_key(term_value: Term) -> TermSortKey:
    """Return a deterministic first-pass ordering key for collected terms."""

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
        term_value.functor.namespace or "",
        term_value.functor.name,
        len(term_value.args),
        tuple(_term_sort_key(argument) for argument in term_value.args),
    )


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


def iftheno(condition: object, then_goal: object) -> GoalExpr:
    """Run `then_goal` from the first proof of `condition`, or fail."""

    condition_goal = _as_goal(condition)
    called_then = _as_goal(then_goal)

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

    def run(program_value: Program, state: State, _args: NativeArgs) -> Iterator[State]:
        condition_proofs = solve_from(program_value, condition_goal, state)
        first_condition_state = next(condition_proofs, None)
        if first_condition_state is None:
            yield from solve_from(program_value, called_else, state)
            return
        yield from solve_from(program_value, called_then, first_condition_state)

    return native_goal(run)


def forallo(generator: object, test: object) -> GoalExpr:
    """Succeed once when every generated proof satisfies `test` at least once."""

    generator_goal = _as_goal(generator)
    test_goal = _as_goal(test)

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


def clauseo(head: object, body: object) -> GoalExpr:
    """Inspect source clauses as Prolog-style `Head :- Body` data."""

    def run(program_value: Program, state: State, args: NativeArgs) -> Iterator[State]:
        head_target, body_target = args
        for source_clause in program_value.clauses:
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
