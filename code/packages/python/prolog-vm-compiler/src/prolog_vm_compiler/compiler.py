"""Compile loaded Prolog artifacts into executable Logic VM instruction streams."""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass

from logic_engine import (
    Clause,
    Compound,
    ConjExpr,
    DisjExpr,
    FreshExpr,
    GoalExpr,
    LogicVar,
    Program,
    Relation,
    RelationCall,
    Term,
    relation,
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
from prolog_loader import (
    LoadedPrologProject,
    LoadedPrologSource,
    adapt_prolog_goal,
    load_swi_prolog_project,
    load_swi_prolog_source,
)
from prolog_parser import ParsedQuery
from symbol_core import Symbol

type RelationKey = tuple[Symbol, int]


@dataclass(frozen=True, slots=True)
class CompiledPrologVMProgram:
    """A Prolog source lowered into standardized Logic VM instructions."""

    instructions: InstructionProgram
    initialization_query_count: int = 0
    source_query_count: int = 0

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


def compile_swi_prolog_project(
    *sources: str,
    adapt_builtins: bool = True,
) -> CompiledPrologVMProgram:
    """Parse, link, and compile multiple SWI-compatible Prolog sources."""

    return compile_loaded_prolog_project(
        load_swi_prolog_project(*sources),
        adapt_builtins=adapt_builtins,
    )


def load_compiled_prolog_vm(compiled_program: CompiledPrologVMProgram) -> LogicVM:
    """Load a compiled Prolog instruction stream into a fresh Logic VM."""

    vm = create_logic_vm()
    vm.load(compiled_program.instructions)
    vm.run()
    return vm


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
