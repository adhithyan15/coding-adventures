"""Loader helpers that bridge parsed Prolog sources into runnable artifacts."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from logic_engine import (
    Clause,
    Compound,
    ConjExpr,
    DisjExpr,
    FreshExpr,
    GoalExpr,
    Program,
    Relation,
    RelationCall,
    State,
    Term,
    conj,
    disj,
    goal_from_term,
    program,
    rule,
    solve_from,
)
from prolog_core import (
    OperatorTable,
    PredicateRegistry,
    PrologDirective,
    PrologModule,
    PrologModuleImport,
    module_import_from_directive,
    module_spec_from_directive,
)
from prolog_parser import ParsedQuery
from symbol_core import Symbol, sym

from prolog_loader.adapters import adapt_prolog_goal

__version__ = "0.1.0"

type GoalAdapter = Callable[[GoalExpr], object]
type RelationResolver = Callable[[Relation], Relation]
_USER_MODULE = sym("user")


class ParsedSourceLike(Protocol):
    """The shared surface returned by dialect-specific Prolog parsers."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry


class InitializableLike(Protocol):
    """Any loaded artifact that can execute initialization directives."""

    program: Program
    initialization_directives: tuple[PrologDirective, ...]
    initialization_terms: tuple[Term, ...]
    initialization_goals: tuple[GoalExpr, ...]


@dataclass(frozen=True, slots=True)
class LoadedPrologSource:
    """A parsed-and-loaded Prolog source with derived initialization metadata."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry
    module_spec: PrologModule | None
    module_imports: tuple[PrologModuleImport, ...]
    initialization_directives: tuple[PrologDirective, ...]
    initialization_terms: tuple[Term, ...]
    initialization_goals: tuple[GoalExpr, ...]


@dataclass(frozen=True, slots=True)
class LoadedPrologProject:
    """A linked multi-source Prolog project with resolved module imports."""

    program: Program
    sources: tuple[LoadedPrologSource, ...]
    queries: tuple[ParsedQuery, ...]
    modules: tuple[PrologModule, ...]
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
    module_spec: PrologModule | None = None
    module_imports: list[PrologModuleImport] = []
    for directive_value in parsed_source.directives:
        parsed_module = module_spec_from_directive(directive_value)
        if parsed_module is not None:
            if module_spec is not None:
                msg = "only one module/2 directive is supported per source"
                raise ValueError(msg)
            module_spec = parsed_module

        parsed_import = module_import_from_directive(directive_value)
        if parsed_import is not None:
            module_imports.append(parsed_import)

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
        module_spec=module_spec,
        module_imports=tuple(module_imports),
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


def load_swi_prolog_project(
    *sources: str,
    operator_table: OperatorTable | None = None,
) -> LoadedPrologProject:
    """Parse, load, and link multiple SWI-Prolog sources as one project."""

    loaded_sources = tuple(
        load_swi_prolog_source(source, operator_table=operator_table)
        for source in sources
    )
    return link_loaded_prolog_sources(*loaded_sources)


def link_loaded_prolog_sources(
    *loaded_sources: LoadedPrologSource,
) -> LoadedPrologProject:
    """Link loaded sources into one namespace-aware executable project."""

    module_sources: dict[Symbol, LoadedPrologSource] = {}
    for loaded_source in loaded_sources:
        if loaded_source.module_spec is None:
            continue
        module_name = loaded_source.module_spec.name
        if module_name in module_sources:
            msg = f"duplicate module declaration for {module_name}"
            raise ValueError(msg)
        module_sources[module_name] = loaded_source

    linked_clauses: list[Clause] = []
    linked_queries: list[ParsedQuery] = []
    linked_modules: list[PrologModule] = []
    initialization_directives: list[PrologDirective] = []
    initialization_terms: list[Term] = []
    initialization_goals: list[GoalExpr] = []
    dynamic_relations: list[Relation] = []

    for loaded_source in loaded_sources:
        if loaded_source.module_spec is not None:
            linked_modules.append(loaded_source.module_spec)

        current_module = (
            loaded_source.module_spec.name
            if loaded_source.module_spec is not None
            else _USER_MODULE
        )
        qualify_local = loaded_source.module_spec is not None
        local_keys = {
            clause_value.head.relation.key() for clause_value in loaded_source.clauses
        }
        import_resolution = _import_resolution(
            loaded_source,
            local_keys,
            module_sources,
        )
        resolver = _resolver_for_source(
            current_module,
            qualify_local=qualify_local,
            local_keys=local_keys,
            import_resolution=import_resolution,
        )

        linked_clauses.extend(
            _rewrite_clause(clause_value, resolver)
            for clause_value in loaded_source.clauses
        )
        linked_queries.extend(
            _rewrite_query(query_value, resolver)
            for query_value in loaded_source.queries
        )
        initialization_directives.extend(loaded_source.initialization_directives)
        initialization_terms.extend(loaded_source.initialization_terms)
        initialization_goals.extend(
            _rewrite_goal(goal_value, resolver)
            for goal_value in loaded_source.initialization_goals
        )
        dynamic_relations.extend(
            resolver(relation_value)
            for relation_value in loaded_source.predicate_registry.dynamic_relations()
        )

    return LoadedPrologProject(
        program=program(*linked_clauses, dynamic_relations=dynamic_relations),
        sources=tuple(loaded_sources),
        queries=tuple(linked_queries),
        modules=tuple(linked_modules),
        initialization_directives=tuple(initialization_directives),
        initialization_terms=tuple(initialization_terms),
        initialization_goals=tuple(initialization_goals),
    )


def run_initialization_goals(
    loaded_source: InitializableLike,
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
    loaded_source: InitializableLike,
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


def _qualified_relation(module_name: Symbol, relation_value: Relation) -> Relation:
    return Relation(
        symbol=sym(f"{module_name.name}:{relation_value.symbol.name}"),
        arity=relation_value.arity,
    )


def _import_resolution(
    loaded_source: LoadedPrologSource,
    local_keys: set[tuple[Symbol, int]],
    module_sources: dict[Symbol, LoadedPrologSource],
) -> dict[tuple[Symbol, int], Relation]:
    resolution: dict[tuple[Symbol, int], Relation] = {}
    for import_value in loaded_source.module_imports:
        target_source = module_sources.get(import_value.module_name)
        if target_source is None or target_source.module_spec is None:
            msg = f"use_module target {import_value.module_name} is not a known module"
            raise ValueError(msg)

        exported = {
            exported_relation.key(): exported_relation
            for exported_relation in target_source.module_spec.exports
        }
        imported_relations = (
            tuple(exported.values())
            if import_value.import_all
            else import_value.imports
        )
        for imported_relation in imported_relations:
            exported_relation = exported.get(imported_relation.key())
            if exported_relation is None:
                msg = (
                    f"module {import_value.module_name} does not export "
                    f"{imported_relation}"
                )
                raise ValueError(msg)
            if imported_relation.key() in local_keys:
                continue

            qualified = _qualified_relation(
                import_value.module_name,
                exported_relation,
            )
            existing = resolution.get(imported_relation.key())
            if existing is not None and existing.key() != qualified.key():
                msg = (
                    f"conflicting imports for {imported_relation} in "
                    f"{_module_name_for_error(loaded_source)}"
                )
                raise ValueError(msg)
            resolution[imported_relation.key()] = qualified
    return resolution


def _resolver_for_source(
    current_module: Symbol,
    *,
    qualify_local: bool,
    local_keys: set[tuple[Symbol, int]],
    import_resolution: dict[tuple[Symbol, int], Relation],
) -> RelationResolver:
    def resolve(relation_value: Relation) -> Relation:
        key = relation_value.key()
        if qualify_local and key in local_keys:
            return _qualified_relation(current_module, relation_value)
        imported = import_resolution.get(key)
        if imported is not None:
            return imported
        return relation_value

    return resolve


def _rewrite_clause(clause_value: Clause, resolver: RelationResolver) -> Clause:
    head = _rewrite_relation_call(clause_value.head, resolver)
    if clause_value.body is None:
        return Clause(head=head)
    return rule(head, _rewrite_goal(clause_value.body, resolver))


def _rewrite_query(query_value: ParsedQuery, resolver: RelationResolver) -> ParsedQuery:
    return ParsedQuery(
        goal=_rewrite_goal(query_value.goal, resolver),
        variables=dict(query_value.variables),
    )


def _rewrite_goal(goal_value: GoalExpr, resolver: RelationResolver) -> GoalExpr:
    if isinstance(goal_value, RelationCall):
        return _rewrite_relation_call(goal_value, resolver)
    if isinstance(goal_value, ConjExpr):
        return conj(*(_rewrite_goal(child, resolver) for child in goal_value.goals))
    if isinstance(goal_value, DisjExpr):
        return disj(*(_rewrite_goal(child, resolver) for child in goal_value.goals))
    if isinstance(goal_value, FreshExpr):
        return FreshExpr(
            template_vars=goal_value.template_vars,
            body=_rewrite_goal(goal_value.body, resolver),
        )
    return goal_value


def _rewrite_relation_call(
    call_value: RelationCall,
    resolver: RelationResolver,
) -> RelationCall:
    resolved_relation = resolver(call_value.relation)
    if resolved_relation.key() == call_value.relation.key():
        return call_value
    return resolved_relation(*call_value.args)


def _module_name_for_error(loaded_source: LoadedPrologSource) -> str:
    if loaded_source.module_spec is None:
        return _USER_MODULE.name
    return loaded_source.module_spec.name.name
