"""Loader helpers that bridge parsed Prolog sources into runnable artifacts."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from logic_engine import (
    Clause,
    Compound,
    GoalExpr,
    Program,
    State,
    Term,
    goal_from_term,
    solve_from,
)
from prolog_core import OperatorTable, PredicateRegistry, PrologDirective
from prolog_parser import ParsedQuery

from prolog_loader.adapters import adapt_prolog_goal

__version__ = "0.1.0"

type GoalAdapter = Callable[[GoalExpr], object]


class ParsedSourceLike(Protocol):
    """The shared surface returned by dialect-specific Prolog parsers."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry


@dataclass(frozen=True, slots=True)
class LoadedPrologSource:
    """A parsed-and-loaded Prolog source with derived initialization metadata."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry
    initialization_directives: tuple[PrologDirective, ...]
    initialization_terms: tuple[Term, ...]
    initialization_goals: tuple[GoalExpr, ...]


class PrologInitializationError(RuntimeError):
    """Raised when a loader initialization directive cannot complete."""

    def __init__(
        self,
        index: int,
        directive: PrologDirective,
        goal_term: Term,
        reason: str,
    ) -> None:
        self.index = index
        self.directive = directive
        self.goal_term = goal_term
        super().__init__(
            f"initialization directive {index} {reason}: {goal_term}",
        )


def load_parsed_prolog_source(parsed_source: ParsedSourceLike) -> LoadedPrologSource:
    """Normalize a dialect-specific parsed source into one loader result."""

    predicate_registry = parsed_source.predicate_registry
    initialization_directives = predicate_registry.initialization_directives
    initialization_terms = tuple(
        _initialization_term(directive_value)
        for directive_value in initialization_directives
    )
    return LoadedPrologSource(
        program=parsed_source.program,
        clauses=parsed_source.clauses,
        queries=parsed_source.queries,
        directives=parsed_source.directives,
        operator_table=parsed_source.operator_table,
        predicate_registry=predicate_registry,
        initialization_directives=initialization_directives,
        initialization_terms=initialization_terms,
        initialization_goals=tuple(
            goal_from_term(term_value) for term_value in initialization_terms
        ),
    )


def load_iso_prolog_source(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> LoadedPrologSource:
    """Parse and load one ISO/Core Prolog source file."""

    from iso_prolog_parser import parse_iso_source

    return load_parsed_prolog_source(
        parse_iso_source(source, operator_table=operator_table),
    )


def load_swi_prolog_source(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
) -> LoadedPrologSource:
    """Parse and load one SWI-Prolog source file."""

    from swi_prolog_parser import parse_swi_source

    return load_parsed_prolog_source(
        parse_swi_source(source, operator_table=operator_table),
    )


def run_initialization_goals(
    loaded_source: LoadedPrologSource,
    *,
    state: State | None = None,
    goal_adapter: GoalAdapter | None = None,
) -> State:
    """Execute collected ``initialization/1`` directives in source order.

    Parsing and loading stay side-effect free. This helper gives callers an
    explicit place to run startup goals later, optionally adapting parsed goals
    into richer runtime/builtin goals before execution.
    """

    current_state = State() if state is None else state
    for index, (directive_value, goal_term, goal_value) in enumerate(
        zip(
            loaded_source.initialization_directives,
            loaded_source.initialization_terms,
            loaded_source.initialization_goals,
            strict=True,
        ),
        start=1,
    ):
        active_goal: object = goal_value
        if goal_adapter is not None:
            active_goal = goal_adapter(goal_value)

        try:
            next_state = next(
                solve_from(loaded_source.program, active_goal, current_state),
                None,
            )
        except Exception as error:
            raise PrologInitializationError(
                index,
                directive_value,
                goal_term,
                "raised an exception while running",
            ) from error

        if next_state is None:
            raise PrologInitializationError(
                index,
                directive_value,
                goal_term,
                "failed",
            )

        current_state = next_state

    return current_state


def run_prolog_initialization_goals(
    loaded_source: LoadedPrologSource,
    *,
    state: State | None = None,
) -> State:
    """Run initialization directives with the shared Prolog builtin adapter."""

    return run_initialization_goals(
        loaded_source,
        state=state,
        goal_adapter=adapt_prolog_goal,
    )


def _initialization_term(directive_value: PrologDirective) -> Term:
    term_value = directive_value.term
    if not isinstance(term_value, Compound) or len(term_value.args) != 1:
        msg = "initialization directives must have the form initialization(Goal)"
        raise TypeError(msg)
    return term_value.args[0]
