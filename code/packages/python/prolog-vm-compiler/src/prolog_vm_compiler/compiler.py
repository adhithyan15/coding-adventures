"""Compile loaded Prolog artifacts into executable Logic VM instruction streams."""

from __future__ import annotations

from collections.abc import Callable, Iterable, Iterator, Mapping
from dataclasses import dataclass
from itertools import islice
from pathlib import Path
from types import MappingProxyType

from logic_engine import (
    Clause,
    Compound,
    ConjExpr,
    Disequality,
    DisjExpr,
    FreshExpr,
    GoalExpr,
    LogicVar,
    Program,
    Relation,
    RelationCall,
    State,
    Term,
    reify,
    relation,
    solve_from,
    succeed,
)
from logic_instructions import (
    InstructionProgram,
    LogicInstruction,
    defdynamic,
    defrel,
    fact,
    instruction_program,
    query,
    rule,
    validate,
)
from logic_vm import LogicVM, create_logic_vm
from prolog_core import OperatorTable
from prolog_loader import (
    LoadedPrologProject,
    LoadedPrologSource,
    SourceResolver,
    adapt_prolog_goal,
    load_swi_prolog_file,
    load_swi_prolog_project,
    load_swi_prolog_project_from_files,
    load_swi_prolog_source,
    rewrite_loaded_prolog_query,
)
from prolog_parser import ParsedQuery
from swi_prolog_parser import parse_swi_query
from symbol_core import Symbol

type RelationKey = tuple[Symbol, int]


@dataclass(frozen=True, slots=True)
class CompiledPrologVMProgram:
    """A Prolog source lowered into standardized Logic VM instructions."""

    instructions: InstructionProgram
    initialization_query_count: int = 0
    source_query_count: int = 0
    source_query_variables: tuple[tuple[str, ...], ...] = ()

    def __post_init__(self) -> None:
        if self.source_query_variables:
            return
        object.__setattr__(
            self,
            "source_query_variables",
            tuple(() for _ in range(self.source_query_count)),
        )

    @property
    def query_count(self) -> int:
        """Return the total number of VM query instructions."""

        return self.initialization_query_count + self.source_query_count

    def source_query_vm_index(self, source_query_index: int = 0) -> int:
        """Translate a source query index into the VM query index space."""

        if source_query_index < 0:
            msg = "source query index must be non-negative"
            raise IndexError(msg)
        if source_query_index >= self.source_query_count:
            msg = f"source query index {source_query_index} is out of range"
            raise IndexError(msg)
        return self.initialization_query_count + source_query_index

    def source_query_variable_names(
        self,
        source_query_index: int = 0,
    ) -> tuple[str, ...]:
        """Return visible variable names for one source-level query."""

        self.source_query_vm_index(source_query_index)
        return self.source_query_variables[source_query_index]


@dataclass(frozen=True, slots=True)
class PrologAnswer:
    """A named source-level answer produced by the compiled Prolog VM path."""

    bindings: Mapping[str, Term]
    residual_constraints: tuple[Disequality, ...] = ()

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "bindings",
            MappingProxyType(dict(self.bindings)),
        )
        object.__setattr__(
            self,
            "residual_constraints",
            tuple(self.residual_constraints),
        )

    def as_dict(self) -> dict[str, Term]:
        """Return answer bindings as a plain dictionary."""

        return dict(self.bindings)


class PrologVMInitializationError(RuntimeError):
    """Raised when a compiled initialization query cannot be proven."""


@dataclass(slots=True)
class PrologVMRuntime:
    """A loaded Prolog VM runtime that can answer ad-hoc source queries."""

    compiled_program: CompiledPrologVMProgram
    vm: LogicVM
    state: State
    operator_table: OperatorTable | None = None
    adapt_builtins: bool = True
    query_rewriter: Callable[[ParsedQuery], ParsedQuery] | None = None

    def query(
        self,
        source: str | ParsedQuery,
        *,
        limit: int | None = None,
        commit: bool = False,
    ) -> list[PrologAnswer]:
        """Run an ad-hoc query and return source-variable bindings."""

        parsed_query = self._parse_query(source)
        if self.query_rewriter is not None:
            parsed_query = self.query_rewriter(parsed_query)
        goal = _adapt_goal(parsed_query.goal, adapt_builtins=self.adapt_builtins)
        outputs = tuple(parsed_query.variables.values())
        proof_states = solve_from(self.vm.assembled_program(), goal, self.state)

        if limit is not None and limit < 0:
            msg = "query limit must be non-negative"
            raise ValueError(msg)

        answers: list[PrologAnswer] = []
        committed_state: State | None = None
        for index, proof_state in enumerate(proof_states):
            if limit is not None and index >= limit:
                break
            if committed_state is None:
                committed_state = proof_state
            answers.append(
                _answer_from_terms(
                    parsed_query.variables.keys(),
                    outputs,
                    proof_state,
                ),
            )

        if commit and committed_state is not None:
            self.state = _persistent_state(committed_state)

        return answers

    def query_values(
        self,
        source: str | ParsedQuery,
        *,
        limit: int | None = None,
        commit: bool = False,
    ) -> list[Term | tuple[Term, ...]]:
        """Run an ad-hoc query and return raw tuple/singleton values."""

        return [
            _answer_value(answer)
            for answer in self.query(source, limit=limit, commit=commit)
        ]

    def run_initializations(self) -> State:
        """Run initialization slots against this runtime's current state."""

        self.state = _run_initializations_on_vm(
            self.vm,
            self.compiled_program,
            state=self.state,
        )
        return self.state

    def _parse_query(self, source: str | ParsedQuery) -> ParsedQuery:
        if isinstance(source, ParsedQuery):
            return source
        return parse_swi_query(
            _normalize_query_source(source),
            operator_table=self.operator_table,
        )


def compile_loaded_prolog_source(
    loaded_source: LoadedPrologSource,
    *,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Compile one loaded Prolog source into Logic VM instructions."""

    return _compile_loaded_prolog(
        loaded_source.clauses,
        loaded_source.queries,
        loaded_source.initialization_goals,
        loaded_source.program,
        adapt_builtins=adapt_builtins,
    )


def compile_loaded_prolog_project(
    loaded_project: LoadedPrologProject,
    *,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Compile a linked Prolog project into Logic VM instructions."""

    return _compile_loaded_prolog(
        loaded_project.program.clauses,
        loaded_project.queries,
        loaded_project.initialization_goals,
        loaded_project.program,
        adapt_builtins=adapt_builtins,
    )


def compile_swi_prolog_source(
    source: str,
    *,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Parse, load, and compile one SWI-compatible Prolog source string."""

    return compile_loaded_prolog_source(
        load_swi_prolog_source(source),
        adapt_builtins=adapt_builtins,
    )


def compile_swi_prolog_file(
    path: str | Path,
    *,
    source_resolver: SourceResolver | None = None,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Read, load, and compile one SWI-compatible Prolog source file."""

    return compile_loaded_prolog_source(
        load_swi_prolog_file(path, source_resolver=source_resolver),
        adapt_builtins=adapt_builtins,
    )


def compile_swi_prolog_project(
    *sources: str,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Parse, link, and compile multiple SWI-compatible Prolog sources."""

    return compile_loaded_prolog_project(
        load_swi_prolog_project(*sources),
        adapt_builtins=adapt_builtins,
    )


def compile_swi_prolog_project_from_files(
    *entry_paths: str | Path,
    source_resolver: SourceResolver | None = None,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Load, link, and compile a SWI-compatible Prolog file graph."""

    return compile_loaded_prolog_project(
        load_swi_prolog_project_from_files(
            *entry_paths,
            source_resolver=source_resolver,
        ),
        adapt_builtins=adapt_builtins,
    )


def load_compiled_prolog_vm(compiled_program: CompiledPrologVMProgram) -> LogicVM:
    """Load a compiled Prolog instruction stream into a fresh Logic VM."""

    vm = create_logic_vm()
    vm.load(compiled_program.instructions)
    vm.run()
    return vm


def create_prolog_vm_runtime(
    compiled_program: CompiledPrologVMProgram,
    *,
    initialize: bool = True,
    operator_table: OperatorTable | None = None,
    adapt_builtins: bool = True,
    query_rewriter: Callable[[ParsedQuery], ParsedQuery] | None = None,
) -> PrologVMRuntime:
    """Create a stateful ad-hoc query runtime from a compiled program."""

    vm = load_compiled_prolog_vm(compiled_program)
    runtime = PrologVMRuntime(
        compiled_program=compiled_program,
        vm=vm,
        state=State(),
        operator_table=operator_table,
        adapt_builtins=adapt_builtins,
        query_rewriter=query_rewriter,
    )
    if initialize:
        runtime.run_initializations()
    return runtime


def create_swi_prolog_vm_runtime(
    source: str,
    *,
    initialize: bool = True,
    adapt_builtins: bool = True,
) -> PrologVMRuntime:
    """Parse, load, compile, and initialize a SWI-compatible query runtime."""

    loaded_source = load_swi_prolog_source(source)
    return create_prolog_vm_runtime(
        compile_loaded_prolog_source(
            loaded_source,
            adapt_builtins=adapt_builtins,
        ),
        initialize=initialize,
        operator_table=loaded_source.operator_table,
        adapt_builtins=adapt_builtins,
    )


def create_swi_prolog_file_runtime(
    path: str | Path,
    *,
    source_resolver: SourceResolver | None = None,
    initialize: bool = True,
    adapt_builtins: bool = True,
) -> PrologVMRuntime:
    """Create a stateful query runtime from one SWI-compatible Prolog file."""

    loaded_source = load_swi_prolog_file(path, source_resolver=source_resolver)
    return create_prolog_vm_runtime(
        compile_loaded_prolog_source(
            loaded_source,
            adapt_builtins=adapt_builtins,
        ),
        initialize=initialize,
        operator_table=loaded_source.operator_table,
        adapt_builtins=adapt_builtins,
    )


def create_swi_prolog_project_runtime(
    *sources: str,
    query_module: str | Symbol | None = None,
    initialize: bool = True,
    adapt_builtins: bool = True,
) -> PrologVMRuntime:
    """Create a stateful query runtime from linked SWI-compatible sources."""

    loaded_project = load_swi_prolog_project(*sources)
    return create_prolog_vm_runtime(
        compile_loaded_prolog_project(
            loaded_project,
            adapt_builtins=adapt_builtins,
        ),
        initialize=initialize,
        operator_table=_project_operator_table(loaded_project),
        adapt_builtins=adapt_builtins,
        query_rewriter=lambda query_value: rewrite_loaded_prolog_query(
            loaded_project,
            query_value,
            module=query_module,
        ),
    )


def create_swi_prolog_project_file_runtime(
    *entry_paths: str | Path,
    source_resolver: SourceResolver | None = None,
    query_module: str | Symbol | None = None,
    initialize: bool = True,
    adapt_builtins: bool = True,
) -> PrologVMRuntime:
    """Create a stateful query runtime from a SWI-compatible Prolog file graph."""

    loaded_project = load_swi_prolog_project_from_files(
        *entry_paths,
        source_resolver=source_resolver,
    )
    return create_prolog_vm_runtime(
        compile_loaded_prolog_project(
            loaded_project,
            adapt_builtins=adapt_builtins,
        ),
        initialize=initialize,
        operator_table=_project_operator_table(loaded_project),
        adapt_builtins=adapt_builtins,
        query_rewriter=lambda query_value: rewrite_loaded_prolog_query(
            loaded_project,
            query_value,
            module=query_module,
        ),
    )


def run_compiled_prolog_query(
    compiled_program: CompiledPrologVMProgram,
    source_query_index: int = 0,
    limit: int | None = None,
) -> list[Term | tuple[Term, ...]]:
    """Run one source-level query from a compiled Prolog VM program."""

    vm = load_compiled_prolog_vm(compiled_program)
    return vm.run_query(
        query_index=compiled_program.source_query_vm_index(source_query_index),
        limit=limit,
    )


def run_compiled_prolog_query_answers(
    compiled_program: CompiledPrologVMProgram,
    source_query_index: int = 0,
    limit: int | None = None,
) -> list[PrologAnswer]:
    """Run one source query and return bindings keyed by Prolog variable name."""

    vm = load_compiled_prolog_vm(compiled_program)
    return _answers_from_vm_query(
        compiled_program,
        vm,
        State(),
        source_query_index,
        limit=limit,
    )


def run_compiled_prolog_queries(
    compiled_program: CompiledPrologVMProgram,
    *,
    limit: int | None = None,
) -> list[list[Term | tuple[Term, ...]]]:
    """Run all source-level queries from a compiled Prolog VM program."""

    vm = load_compiled_prolog_vm(compiled_program)
    return [
        vm.run_query(
            query_index=compiled_program.source_query_vm_index(index),
            limit=limit,
        )
        for index in range(compiled_program.source_query_count)
    ]


def run_compiled_prolog_initializations(
    compiled_program: CompiledPrologVMProgram,
    *,
    state: State | None = None,
) -> State:
    """Run compiled initialization query slots in order and return final state."""

    vm = load_compiled_prolog_vm(compiled_program)
    return _run_initializations_on_vm(vm, compiled_program, state=state)


def run_initialized_compiled_prolog_query(
    compiled_program: CompiledPrologVMProgram,
    source_query_index: int = 0,
    limit: int | None = None,
) -> list[Term | tuple[Term, ...]]:
    """Run initializations, then execute one source query from that state."""

    vm = load_compiled_prolog_vm(compiled_program)
    initialized_state = _run_initializations_on_vm(vm, compiled_program)
    return vm.run_query_from(
        initialized_state,
        query_index=compiled_program.source_query_vm_index(source_query_index),
        limit=limit,
    )


def run_initialized_compiled_prolog_query_answers(
    compiled_program: CompiledPrologVMProgram,
    source_query_index: int = 0,
    limit: int | None = None,
) -> list[PrologAnswer]:
    """Run initializations and return named bindings for one source query."""

    vm = load_compiled_prolog_vm(compiled_program)
    initialized_state = _run_initializations_on_vm(vm, compiled_program)
    return _answers_from_vm_query(
        compiled_program,
        vm,
        initialized_state,
        source_query_index,
        limit=limit,
    )


def _compile_loaded_prolog(
    clauses: tuple[Clause, ...],
    source_queries: tuple[ParsedQuery, ...],
    initialization_goals: tuple[GoalExpr, ...],
    program_value: Program,
    *,
    adapt_builtins: bool,
) -> CompiledPrologVMProgram:
    adapted_clauses = tuple(
        _adapt_clause(clause_value, adapt_builtins=adapt_builtins)
        for clause_value in clauses
    )
    adapted_initialization_goals = tuple(
        _adapt_goal(goal_value, adapt_builtins=adapt_builtins)
        for goal_value in initialization_goals
    )
    adapted_source_queries = tuple(
        _adapt_query(query_value, adapt_builtins=adapt_builtins)
        for query_value in source_queries
    )

    relations = _collect_relations(
        adapted_clauses,
        adapted_initialization_goals,
        tuple(query_value.goal for query_value in adapted_source_queries),
        program_value.dynamic_relations,
    )
    dynamic_keys = set(program_value.dynamic_relations)
    instructions: list[LogicInstruction] = []

    for relation_value in relations.values():
        if relation_value.key() in dynamic_keys:
            instructions.append(defdynamic(relation_value))
        else:
            instructions.append(defrel(relation_value))

    for clause_value in adapted_clauses:
        if clause_value.body is not None or _relation_call_has_variables(
            clause_value.head,
        ):
            instructions.append(rule(clause_value.head, clause_value.body or succeed()))
        else:
            instructions.append(fact(clause_value.head))

    for index, goal_value in enumerate(adapted_initialization_goals, start=1):
        instructions.append(query(goal_value, label=f"initialization:{index}"))

    for index, query_value in enumerate(adapted_source_queries, start=1):
        instructions.append(
            query(
                query_value.goal,
                outputs=_query_outputs(query_value),
                label=f"query:{index}",
            ),
        )

    compiled_instructions = instruction_program(*instructions)
    validate(compiled_instructions)
    return CompiledPrologVMProgram(
        instructions=compiled_instructions,
        initialization_query_count=len(adapted_initialization_goals),
        source_query_count=len(adapted_source_queries),
        source_query_variables=tuple(
            tuple(query_value.variables) for query_value in adapted_source_queries
        ),
    )


def _adapt_clause(clause_value: Clause, *, adapt_builtins: bool) -> Clause:
    if clause_value.body is None:
        return clause_value
    return Clause(
        head=clause_value.head,
        body=_adapt_goal(clause_value.body, adapt_builtins=adapt_builtins),
    )


def _adapt_goal(goal_value: GoalExpr, *, adapt_builtins: bool) -> GoalExpr:
    if not adapt_builtins:
        return goal_value
    return adapt_prolog_goal(goal_value)


def _adapt_query(query_value: ParsedQuery, *, adapt_builtins: bool) -> ParsedQuery:
    goal_value = query_value.goal
    if not adapt_builtins:
        return query_value
    return ParsedQuery(
        goal=adapt_prolog_goal(goal_value),
        variables=query_value.variables,
    )


def _project_operator_table(project: LoadedPrologProject) -> OperatorTable | None:
    if not project.sources:
        return None
    return project.sources[0].operator_table


def _collect_relations(
    clauses: tuple[Clause, ...],
    initialization_goals: tuple[GoalExpr, ...],
    source_query_goals: tuple[GoalExpr, ...],
    dynamic_relation_keys: frozenset[RelationKey],
) -> dict[RelationKey, Relation]:
    relations: dict[RelationKey, Relation] = {}

    for clause_value in clauses:
        _remember_relation(relations, clause_value.head.relation)
        if clause_value.body is not None:
            for call in _iter_relation_calls(clause_value.body):
                _remember_relation(relations, call.relation)

    for goal_value in (*initialization_goals, *source_query_goals):
        for call in _iter_relation_calls(goal_value):
            _remember_relation(relations, call.relation)

    for key in sorted(dynamic_relation_keys, key=lambda item: (item[0].name, item[1])):
        relations.setdefault(key, relation(key[0], key[1]))

    return relations


def _remember_relation(
    relations: dict[RelationKey, Relation],
    relation_value: Relation,
) -> None:
    relations.setdefault(relation_value.key(), relation_value)


def _relation_call_has_variables(call: RelationCall) -> bool:
    return any(_term_has_variables(argument) for argument in call.args)


def _term_has_variables(term_value: Term) -> bool:
    if isinstance(term_value, LogicVar):
        return True
    if isinstance(term_value, Compound):
        return any(_term_has_variables(argument) for argument in term_value.args)
    return False


def _iter_relation_calls(goal_value: GoalExpr) -> Iterator[RelationCall]:
    if isinstance(goal_value, RelationCall):
        yield goal_value
        return

    if isinstance(goal_value, ConjExpr | DisjExpr):
        for child in goal_value.goals:
            yield from _iter_relation_calls(child)
        return

    if isinstance(goal_value, FreshExpr):
        yield from _iter_relation_calls(goal_value.body)


def _query_outputs(query_value: ParsedQuery) -> tuple[Term, ...] | None:
    outputs = tuple(query_value.variables.values())
    return outputs or None


def _run_initializations_on_vm(
    vm: LogicVM,
    compiled_program: CompiledPrologVMProgram,
    *,
    state: State | None = None,
) -> State:
    current_state = State() if state is None else state
    for query_index in range(compiled_program.initialization_query_count):
        next_state = next(vm.solve_query_from(current_state, query_index), None)
        if next_state is None:
            msg = f"initialization query {query_index + 1} failed"
            raise PrologVMInitializationError(msg)
        current_state = next_state
    return current_state


def _answers_from_vm_query(
    compiled_program: CompiledPrologVMProgram,
    vm: LogicVM,
    state: State,
    source_query_index: int,
    *,
    limit: int | None,
) -> list[PrologAnswer]:
    if limit is not None and limit < 0:
        msg = "query limit must be non-negative"
        raise ValueError(msg)

    vm_query_index = compiled_program.source_query_vm_index(source_query_index)
    names = compiled_program.source_query_variable_names(source_query_index)
    try:
        outputs = vm.state.queries[vm_query_index].outputs or ()
    except IndexError as exc:
        msg = f"query index {vm_query_index} is out of range"
        raise IndexError(msg) from exc

    proof_states = vm.solve_query_from(state, vm_query_index)
    if limit is not None:
        proof_states = islice(proof_states, limit)
    return [
        _answer_from_terms(names, tuple(outputs), proof_state)
        for proof_state in proof_states
    ]


def _answer_from_terms(
    names: Iterable[str],
    outputs: tuple[Term, ...],
    state: State,
) -> PrologAnswer:
    residual_constraints = _residual_constraints(state)
    if not outputs:
        return PrologAnswer({}, residual_constraints=residual_constraints)
    return PrologAnswer(
        {
            name: reify(output, state.substitution)
            for name, output in zip(names, outputs, strict=True)
        },
        residual_constraints=residual_constraints,
    )


def _residual_constraints(state: State) -> tuple[Disequality, ...]:
    return tuple(
        Disequality(
            left=reify(constraint.left, state.substitution),
            right=reify(constraint.right, state.substitution),
        )
        for constraint in state.constraints
    )


def _answer_value(answer: PrologAnswer) -> Term | tuple[Term, ...]:
    values = tuple(answer.bindings.values())
    if len(values) == 1:
        return values[0]
    return values


def _persistent_state(state: State) -> State:
    return State(
        next_var_id=state.next_var_id,
        database=state.database,
        fd_store=state.fd_store,
    )


def _normalize_query_source(source: str) -> str:
    stripped = source.strip()
    if not stripped:
        msg = "query source must not be empty"
        raise ValueError(msg)
    if not stripped.startswith("?-"):
        stripped = f"?- {stripped}"
    if not stripped.endswith("."):
        stripped = f"{stripped}."
    return f"{stripped}\n"
