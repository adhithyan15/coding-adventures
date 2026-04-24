"""Loader helpers that bridge parsed Prolog sources into runnable artifacts."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Literal, Protocol

from logic_engine import (
    Atom,
    Clause,
    Compound,
    FreshExpr,
    GoalExpr,
    Number,
    Program,
    Relation,
    RelationCall,
    State,
    String,
    Term,
    atom,
    goal_as_term,
    goal_from_term,
    program,
    rule,
    solve_from,
    term,
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
_MODULE_QUALIFIER = sym(":")
_CONJUNCTION = sym(",")
_DISJUNCTION = sym(";")
_CALL = sym("call")
_ONCE = sym("once")
_NOT = sym("not")
_NEGATION = sym("\\+")
_PHRASE = sym("phrase")


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


type DependencyKind = Literal["consult", "ensure_loaded", "use_module"]


@dataclass(frozen=True, slots=True)
class PrologSourceDependency:
    """One file dependency referenced by a Prolog source directive."""

    kind: DependencyKind
    requested: str
    resolved_path: Path
    imports: tuple[Relation, ...] = ()
    import_all: bool = False


@dataclass(frozen=True, slots=True)
class LoadedPrologSource:
    """A parsed-and-loaded Prolog source with derived initialization metadata."""

    source_path: Path | None
    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry
    module_spec: PrologModule | None
    module_imports: tuple[PrologModuleImport, ...]
    file_dependencies: tuple[PrologSourceDependency, ...]
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


def load_parsed_prolog_source(
    parsed_source: ParsedSourceLike,
    *,
    source_path: str | Path | None = None,
) -> LoadedPrologSource:
    """Normalize a dialect-specific parsed source into one loader result."""

    predicate_registry = parsed_source.predicate_registry
    normalized_source_path = (
        None if source_path is None else Path(source_path).expanduser().resolve()
    )
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
        if parsed_import is not None and (
            normalized_source_path is None
            or not _is_file_backed_use_module_directive(
                directive_value,
                source_path=normalized_source_path,
            )
        ):
            module_imports.append(parsed_import)

    file_dependencies = _source_dependencies(
        parsed_source.directives,
        source_path=normalized_source_path,
    )
    initialization_directives = predicate_registry.initialization_directives
    initialization_terms = tuple(
        _initialization_term(directive_value)
        for directive_value in initialization_directives
    )
    return LoadedPrologSource(
        source_path=normalized_source_path,
        program=parsed_source.program,
        clauses=parsed_source.clauses,
        queries=parsed_source.queries,
        directives=parsed_source.directives,
        operator_table=parsed_source.operator_table,
        predicate_registry=predicate_registry,
        module_spec=module_spec,
        module_imports=tuple(module_imports),
        file_dependencies=file_dependencies,
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
    source_path: str | Path | None = None,
) -> LoadedPrologSource:
    """Parse and load one ISO/Core Prolog source file."""

    from iso_prolog_parser import parse_iso_source

    return load_parsed_prolog_source(
        parse_iso_source(source, operator_table=operator_table),
        source_path=source_path,
    )


def load_swi_prolog_source(
    source: str,
    *,
    operator_table: OperatorTable | None = None,
    source_path: str | Path | None = None,
) -> LoadedPrologSource:
    """Parse and load one SWI-Prolog source file."""

    from swi_prolog_parser import parse_swi_source

    return load_parsed_prolog_source(
        parse_swi_source(source, operator_table=operator_table),
        source_path=source_path,
    )


def load_swi_prolog_file(
    path: str | Path,
    *,
    operator_table: OperatorTable | None = None,
) -> LoadedPrologSource:
    """Read, parse, and load one SWI-Prolog source file."""

    normalized_path = Path(path).expanduser().resolve()
    return load_swi_prolog_source(
        normalized_path.read_text(encoding="utf-8"),
        operator_table=operator_table,
        source_path=normalized_path,
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


def load_swi_prolog_project_from_files(
    *entry_paths: str | Path,
    operator_table: OperatorTable | None = None,
) -> LoadedPrologProject:
    """Load, resolve, and link a SWI-Prolog file graph."""

    pending_paths = [
        Path(entry_path).expanduser().resolve() for entry_path in entry_paths
    ]
    loaded_by_path: dict[Path, LoadedPrologSource] = {}

    while pending_paths:
        current_path = pending_paths.pop(0)
        if current_path in loaded_by_path:
            continue

        loaded_source = load_swi_prolog_file(
            current_path,
            operator_table=operator_table,
        )
        loaded_by_path[current_path] = loaded_source
        pending_paths.extend(
            dependency.resolved_path
            for dependency in loaded_source.file_dependencies
            if dependency.resolved_path not in loaded_by_path
        )

    normalized_sources = tuple(
        _normalize_file_module_imports(
            loaded_source,
            loaded_by_path=loaded_by_path,
        )
        for loaded_source in loaded_by_path.values()
    )
    return link_loaded_prolog_sources(*normalized_sources)


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
    source_resolvers: dict[int, RelationResolver] = {}
    module_resolvers: dict[Symbol, RelationResolver] = {}

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
        source_resolvers[id(loaded_source)] = resolver
        if loaded_source.module_spec is not None:
            module_resolvers[loaded_source.module_spec.name] = resolver

    for loaded_source in loaded_sources:
        resolver = source_resolvers[id(loaded_source)]

        linked_clauses.extend(
            _rewrite_clause(clause_value, resolver, module_resolvers)
            for clause_value in loaded_source.clauses
        )
        linked_queries.extend(
            _rewrite_query(query_value, resolver, module_resolvers)
            for query_value in loaded_source.queries
        )
        initialization_directives.extend(loaded_source.initialization_directives)
        initialization_terms.extend(loaded_source.initialization_terms)
        initialization_goals.extend(
            _rewrite_goal(goal_value, resolver, module_resolvers)
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


def _normalize_file_module_imports(
    loaded_source: LoadedPrologSource,
    *,
    loaded_by_path: dict[Path, LoadedPrologSource],
) -> LoadedPrologSource:
    normalized_imports = list(loaded_source.module_imports)
    for dependency in loaded_source.file_dependencies:
        if dependency.kind != "use_module":
            continue

        target_source = loaded_by_path.get(dependency.resolved_path)
        if target_source is None or target_source.module_spec is None:
            msg = (
                f"use_module file target {dependency.resolved_path} "
                "must load a module/2 declaration"
            )
            raise ValueError(msg)

        normalized_imports.append(
            PrologModuleImport(
                module_name=target_source.module_spec.name,
                imports=dependency.imports,
                import_all=dependency.import_all,
            ),
        )

    return replace(
        loaded_source,
        module_imports=tuple(normalized_imports),
    )


def _source_dependencies(
    directives: tuple[PrologDirective, ...],
    *,
    source_path: Path | None,
) -> tuple[PrologSourceDependency, ...]:
    if source_path is None:
        return ()

    dependencies: list[PrologSourceDependency] = []
    for directive_value in directives:
        parsed = _dependency_from_directive(
            directive_value,
            source_path=source_path,
        )
        dependencies.extend(parsed)
    return tuple(dependencies)


def _is_file_backed_use_module_directive(
    directive_value: PrologDirective,
    *,
    source_path: Path,
) -> bool:
    return any(
        dependency.kind == "use_module"
        for dependency in _dependency_from_directive(
            directive_value,
            source_path=source_path,
        )
    )


def _dependency_from_directive(
    directive_value: PrologDirective,
    *,
    source_path: Path,
) -> tuple[PrologSourceDependency, ...]:
    term_value = directive_value.term
    if not isinstance(term_value, Compound):
        return ()

    if term_value.functor.name in {"consult", "ensure_loaded"}:
        if len(term_value.args) != 1:
            msg = f"{term_value.functor}/1 directives require exactly one argument"
            raise ValueError(msg)
        return tuple(
            PrologSourceDependency(
                kind=term_value.functor.name,  # type: ignore[arg-type]
                requested=requested,
                resolved_path=resolved_path,
            )
            for requested, resolved_path in _resolved_source_terms(
                term_value.args[0],
                source_path=source_path,
                resolve_plain_atoms=True,
            )
        )

    if term_value.functor.name == "use_module":
        if len(term_value.args) not in {1, 2}:
            msg = "use_module directives require one or two arguments"
            raise ValueError(msg)

        resolved = _resolved_source_term(
            term_value.args[0],
            source_path=source_path,
            resolve_plain_atoms=True,
        )
        if resolved is None:
            return ()

        imports: tuple[Relation, ...] = ()
        import_all = True
        if len(term_value.args) == 2:
            imports = _directive_relations(
                term_value.args[1],
                directive_name="use_module",
            )
            import_all = False

        requested, resolved_path = resolved
        return (
            PrologSourceDependency(
                kind="use_module",
                requested=requested,
                resolved_path=resolved_path,
                imports=imports,
                import_all=import_all,
            ),
        )

    return ()


def _initialization_term(directive_value: PrologDirective) -> Term:
    term_value = directive_value.term
    if not isinstance(term_value, Compound) or len(term_value.args) != 1:
        msg = "initialization directives must have the form initialization(Goal)"
        raise TypeError(msg)
    return term_value.args[0]


def _resolved_source_terms(
    term_value: Term,
    *,
    source_path: Path,
    resolve_plain_atoms: bool,
) -> tuple[tuple[str, Path], ...]:
    items = _logic_list_items(term_value)
    if items is None:
        resolved = _resolved_source_term(
            term_value,
            source_path=source_path,
            resolve_plain_atoms=resolve_plain_atoms,
        )
        if resolved is None:
            msg = f"unsupported source reference {term_value}"
            raise TypeError(msg)
        return (resolved,)

    resolved_items: list[tuple[str, Path]] = []
    for item in items:
        resolved = _resolved_source_term(
            item,
            source_path=source_path,
            resolve_plain_atoms=resolve_plain_atoms,
        )
        if resolved is None:
            msg = f"unsupported source reference {item}"
            raise TypeError(msg)
        resolved_items.append(resolved)
    return tuple(resolved_items)


def _resolved_source_term(
    term_value: Term,
    *,
    source_path: Path,
    resolve_plain_atoms: bool,
) -> tuple[str, Path] | None:
    requested = _source_reference_text(term_value)
    if requested is None:
        return None

    explicit_path = (
        "/" in requested
        or "\\" in requested
        or requested.startswith(".")
        or requested.endswith(".pl")
    )
    if explicit_path:
        candidate = Path(requested).expanduser()
        if not candidate.is_absolute():
            candidate = source_path.parent / candidate
        return (requested, candidate.resolve())

    if not resolve_plain_atoms:
        return None

    return (
        requested,
        (source_path.parent / f"{requested}.pl").resolve(),
    )


def _source_reference_text(term_value: Term) -> str | None:
    if isinstance(term_value, Atom):
        return term_value.symbol.name
    if isinstance(term_value, String):
        return term_value.value
    return None


def _directive_relations(
    indicators: Term,
    *,
    directive_name: str,
) -> tuple[Relation, ...]:
    items = _logic_list_items(indicators)
    if items is None:
        relation_value = _directive_relation(indicators, directive_name=directive_name)
        return (relation_value,)

    return tuple(
        _directive_relation(item, directive_name=directive_name) for item in items
    )


def _directive_relation(term_value: Term, *, directive_name: str) -> Relation:
    if (
        isinstance(term_value, Compound)
        and term_value.functor.name == "/"
        and len(term_value.args) == 2
        and isinstance(term_value.args[0], Atom)
    ):
        arity_term = term_value.args[1]
        if not isinstance(arity_term, Number) or not isinstance(arity_term.value, int):
            msg = f"{directive_name} indicators must use atom/integer pairs"
            raise TypeError(msg)
        return Relation(symbol=term_value.args[0].symbol, arity=arity_term.value)

    msg = f"{directive_name} directives must contain predicate indicators"
    raise TypeError(msg)


def _logic_list_items(term_value: Term) -> list[Term] | None:
    items: list[Term] = []
    current = term_value
    while True:
        if isinstance(current, Atom) and current.symbol.name == "[]":
            return items
        if (
            isinstance(current, Compound)
            and current.functor.name == "."
            and len(current.args) == 2
        ):
            items.append(current.args[0])
            current = current.args[1]
            continue
        return None


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


def _rewrite_clause(
    clause_value: Clause,
    resolver: RelationResolver,
    module_resolvers: dict[Symbol, RelationResolver],
) -> Clause:
    head = _rewrite_relation_call(clause_value.head, resolver)
    if clause_value.body is None:
        return Clause(head=head)
    return rule(head, _rewrite_goal(clause_value.body, resolver, module_resolvers))


def _rewrite_query(
    query_value: ParsedQuery,
    resolver: RelationResolver,
    module_resolvers: dict[Symbol, RelationResolver],
) -> ParsedQuery:
    return ParsedQuery(
        goal=_rewrite_goal(query_value.goal, resolver, module_resolvers),
        variables=dict(query_value.variables),
    )


def _rewrite_goal(
    goal_value: GoalExpr,
    resolver: RelationResolver,
    module_resolvers: dict[Symbol, RelationResolver],
) -> GoalExpr:
    if isinstance(goal_value, FreshExpr):
        return FreshExpr(
            template_vars=goal_value.template_vars,
            body=_rewrite_goal(goal_value.body, resolver, module_resolvers),
        )
    return goal_from_term(
        _rewrite_goal_term(
            goal_as_term(goal_value),
            resolver,
            module_resolvers,
        ),
    )


def _rewrite_relation_call(
    call_value: RelationCall,
    resolver: RelationResolver,
) -> RelationCall:
    resolved_relation = resolver(call_value.relation)
    if resolved_relation.key() == call_value.relation.key():
        return call_value
    return resolved_relation(*call_value.args)


def _rewrite_goal_term(
    term_value: Term,
    resolver: RelationResolver,
    module_resolvers: dict[Symbol, RelationResolver],
) -> Term:
    if isinstance(term_value, Atom):
        resolved_relation = resolver(Relation(symbol=term_value.symbol, arity=0))
        if resolved_relation.symbol == term_value.symbol:
            return term_value
        return atom(resolved_relation.symbol)

    if not isinstance(term_value, Compound):
        return term_value

    qualified = _qualified_goal(term_value)
    if qualified is not None:
        module_name, goal_term = qualified
        target_resolver = module_resolvers.get(module_name)
        if target_resolver is None:
            msg = f"module qualification references unknown module {module_name}"
            raise ValueError(msg)
        return _rewrite_goal_term(goal_term, target_resolver, module_resolvers)

    if term_value.functor == _CONJUNCTION and len(term_value.args) == 2:
        return term(
            ",",
            _rewrite_goal_term(term_value.args[0], resolver, module_resolvers),
            _rewrite_goal_term(term_value.args[1], resolver, module_resolvers),
        )

    if term_value.functor == _DISJUNCTION and len(term_value.args) == 2:
        return term(
            ";",
            _rewrite_goal_term(term_value.args[0], resolver, module_resolvers),
            _rewrite_goal_term(term_value.args[1], resolver, module_resolvers),
        )

    if term_value.functor in {_CALL, _ONCE}:
        return term(
            term_value.functor,
            _rewrite_goal_term(term_value.args[0], resolver, module_resolvers),
        )

    if term_value.functor in {_NOT, _NEGATION}:
        return term(
            term_value.functor,
            _rewrite_goal_term(term_value.args[0], resolver, module_resolvers),
        )

    if (
        term_value.functor == _PHRASE and len(term_value.args) in {2, 3}
    ):
        rewritten_args = (
            _rewrite_goal_term(term_value.args[0], resolver, module_resolvers),
            *term_value.args[1:],
        )
        return term(term_value.functor, *rewritten_args)

    resolved_relation = resolver(
        Relation(symbol=term_value.functor, arity=len(term_value.args)),
    )
    if resolved_relation.symbol == term_value.functor:
        return term_value
    return term(resolved_relation.symbol, *term_value.args)


def _qualified_goal(term_value: Compound) -> tuple[Symbol, Term] | None:
    if term_value.functor != _MODULE_QUALIFIER or len(term_value.args) != 2:
        return None

    module_term, goal_term = term_value.args
    if not isinstance(module_term, Atom):
        return None
    return (module_term.symbol, goal_term)


def _module_name_for_error(loaded_source: LoadedPrologSource) -> str:
    if loaded_source.module_spec is None:
        return _USER_MODULE.name
    return loaded_source.module_spec.name.name
