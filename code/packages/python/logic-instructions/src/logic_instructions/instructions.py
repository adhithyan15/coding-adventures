"""Instruction model for executable logic programs.

This package gives the logic stack a new intermediate layer: a logic program
can now be represented as an ordered stream of instructions instead of only as
direct Python object graphs.

The first version intentionally stays *above* raw bytecode. Its job is to make
facts, rules, and queries explicit data so that:

- the current ``logic-engine`` can execute them immediately
- future tooling can serialize, inspect, or trace them
- a later VM can compile the same instruction stream into lower-level opcodes
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field
from enum import StrEnum

from logic_engine import (
    Atom,
    Clause,
    Compound,
    ConjExpr,
    DeferredExpr,
    DisjExpr,
    EqExpr,
    FailExpr,
    FreshExpr,
    GoalExpr,
    LogicVar,
    NativeGoalExpr,
    NeqExpr,
    Number,
    Program,
    Relation,
    RelationCall,
    String,
    SucceedExpr,
    Term,
    atom,
    num,
    solve_all,
    solve_n,
)
from logic_engine import (
    fact as engine_fact,
)
from logic_engine import (
    program as engine_program,
)
from logic_engine import (
    relation as make_relation,
)
from logic_engine import (
    rule as engine_rule,
)
from symbol_core import Symbol

__all__ = [
    "AssembledInstructionProgram",
    "DynamicRelationDefInstruction",
    "FactInstruction",
    "InstructionOpcode",
    "InstructionProgram",
    "LogicInstruction",
    "QueryInstruction",
    "RelationDefInstruction",
    "RuleInstruction",
    "assemble",
    "defdynamic",
    "defrel",
    "fact",
    "instruction_program",
    "query",
    "rule",
    "run_all_queries",
    "run_query",
    "validate",
]


class InstructionOpcode(StrEnum):
    """Top-level instruction mnemonics for logic programs."""

    DEF_REL = "DEF_REL"
    DYNAMIC_REL = "DYNAMIC_REL"
    FACT = "FACT"
    RULE = "RULE"
    QUERY = "QUERY"


@dataclass(frozen=True, slots=True)
class RelationDefInstruction:
    """Declare a relation before later instructions refer to it."""

    relation: Relation
    opcode: InstructionOpcode = field(
        init=False,
        default=InstructionOpcode.DEF_REL,
    )


@dataclass(frozen=True, slots=True)
class DynamicRelationDefInstruction:
    """Declare a relation whose clauses may be mutated at runtime."""

    relation: Relation
    opcode: InstructionOpcode = field(
        init=False,
        default=InstructionOpcode.DYNAMIC_REL,
    )


@dataclass(frozen=True, slots=True)
class FactInstruction:
    """Emit one ground fact into the assembled clause database."""

    head: RelationCall
    opcode: InstructionOpcode = field(
        init=False,
        default=InstructionOpcode.FACT,
    )


@dataclass(frozen=True, slots=True)
class RuleInstruction:
    """Emit one rule into the assembled clause database."""

    head: RelationCall
    body: GoalExpr
    opcode: InstructionOpcode = field(
        init=False,
        default=InstructionOpcode.RULE,
    )


@dataclass(frozen=True, slots=True)
class QueryInstruction:
    """Store one runnable query plus the values that should be reified."""

    goal: GoalExpr
    outputs: tuple[Term, ...] | None = None
    label: str | None = None
    opcode: InstructionOpcode = field(
        init=False,
        default=InstructionOpcode.QUERY,
    )


type LogicInstruction = (
    RelationDefInstruction
    | DynamicRelationDefInstruction
    | FactInstruction
    | RuleInstruction
    | QueryInstruction
)


@dataclass(frozen=True, slots=True)
class InstructionProgram:
    """An immutable ordered stream of logic instructions."""

    instructions: tuple[LogicInstruction, ...]

    def __post_init__(self) -> None:
        for instruction in self.instructions:
            if not isinstance(
                instruction,
                (
                    RelationDefInstruction,
                    DynamicRelationDefInstruction,
                    FactInstruction,
                    RuleInstruction,
                    QueryInstruction,
                ),
            ):
                msg = (
                    "InstructionProgram entries must all be relation "
                    "declarations, facts, rules, or queries"
                )
                raise TypeError(msg)


@dataclass(frozen=True, slots=True)
class AssembledInstructionProgram:
    """A validated instruction stream lowered into the current engine backend."""

    program: Program
    queries: tuple[QueryInstruction, ...]
    relations: tuple[Relation, ...]


def _coerce_term(value: object) -> Term:
    """Match the user-facing coercions of ``logic-engine`` for query outputs."""

    if isinstance(value, Atom | Number | String | LogicVar | Compound):
        return value
    if isinstance(value, Symbol):
        return atom(value)
    if isinstance(value, bool):
        msg = (
            "bool values are ambiguous in logic-instructions; use atoms or "
            "numbers explicitly"
        )
        raise TypeError(msg)
    if isinstance(value, int | float):
        return num(value)
    if isinstance(value, str):
        return atom(value)

    msg = f"cannot coerce {type(value).__name__} into a logic term"
    raise TypeError(msg)


def _coerce_goal(goal: object) -> GoalExpr:
    """Accept only goal expressions already understood by ``logic-engine``."""

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

    msg = (
        f"cannot use {type(goal).__name__} as a logic-instructions "
        "goal expression"
    )
    raise TypeError(msg)


def defrel(
    name: str | Symbol | Relation,
    arity: int | None = None,
) -> RelationDefInstruction:
    """Construct a relation declaration instruction."""

    if isinstance(name, Relation):
        if arity is not None:
            msg = "defrel() does not accept arity when given a Relation object"
            raise ValueError(msg)
        return RelationDefInstruction(relation=name)

    if arity is None:
        msg = "defrel() requires arity when given a relation name or symbol"
        raise ValueError(msg)

    return RelationDefInstruction(relation=make_relation(name, arity))


def defdynamic(
    name: str | Symbol | Relation,
    arity: int | None = None,
) -> DynamicRelationDefInstruction:
    """Construct a dynamic relation declaration instruction."""

    if isinstance(name, Relation):
        if arity is not None:
            msg = "defdynamic() does not accept arity when given a Relation object"
            raise ValueError(msg)
        return DynamicRelationDefInstruction(relation=name)

    if arity is None:
        msg = "defdynamic() requires arity when given a relation name or symbol"
        raise ValueError(msg)

    return DynamicRelationDefInstruction(relation=make_relation(name, arity))


def fact(head: RelationCall) -> FactInstruction:
    """Construct a fact instruction."""

    if not isinstance(head, RelationCall):
        msg = "fact() requires a relation call as its head"
        raise TypeError(msg)
    return FactInstruction(head=head)


def rule(head: RelationCall, body: object) -> RuleInstruction:
    """Construct a rule instruction."""

    if not isinstance(head, RelationCall):
        msg = "rule() requires a relation call as its head"
        raise TypeError(msg)
    return RuleInstruction(head=head, body=_coerce_goal(body))


def query(
    goal: object,
    outputs: tuple[object, ...] | None = None,
    label: str | None = None,
) -> QueryInstruction:
    """Construct a query instruction."""

    normalized_outputs = None
    if outputs is not None:
        normalized_outputs = tuple(_coerce_term(item) for item in outputs)
    return QueryInstruction(
        goal=_coerce_goal(goal),
        outputs=normalized_outputs,
        label=label,
    )


def instruction_program(*instructions: LogicInstruction) -> InstructionProgram:
    """Construct an immutable instruction stream."""

    return InstructionProgram(instructions=tuple(instructions))


def _relation_key(relation_value: Relation) -> tuple[Symbol, int]:
    """Use the engine's stable `(symbol, arity)` identity for declarations."""

    return relation_value.key()


def _contains_logic_var(term_value: Term) -> bool:
    """Facts should be ground, so variables anywhere inside them are rejected."""

    if isinstance(term_value, LogicVar):
        return True
    if isinstance(term_value, Compound):
        return any(_contains_logic_var(argument) for argument in term_value.args)
    return False


def _iter_relation_calls(goal: GoalExpr) -> Iterator[RelationCall]:
    """Yield explicit relation calls inside a goal tree.

    Deferred goals are intentionally treated as opaque in the first version.
    They may expand to relation calls later at solve time, but validating them
    statically would require executing host-language callbacks.
    """

    if isinstance(goal, RelationCall):
        yield goal
        return

    if isinstance(goal, ConjExpr | DisjExpr):
        for child in goal.goals:
            yield from _iter_relation_calls(child)
        return

    if isinstance(goal, FreshExpr):
        yield from _iter_relation_calls(goal.body)


def _collect_term_vars(
    term_value: Term,
    masked: frozenset[LogicVar],
    seen: set[LogicVar],
    ordered: list[LogicVar],
) -> None:
    """Collect free variables from a term in first-appearance order."""

    if isinstance(term_value, LogicVar):
        if term_value not in masked and term_value not in seen:
            seen.add(term_value)
            ordered.append(term_value)
        return

    if isinstance(term_value, Compound):
        for argument in term_value.args:
            _collect_term_vars(argument, masked, seen, ordered)


def _infer_outputs_from_goal(
    goal: GoalExpr,
    masked: frozenset[LogicVar] = frozenset(),
    seen: set[LogicVar] | None = None,
    ordered: list[LogicVar] | None = None,
) -> tuple[LogicVar, ...]:
    """Infer query outputs from free variables in source order."""

    if seen is None:
        seen = set()
    if ordered is None:
        ordered = []

    if isinstance(goal, RelationCall):
        for argument in goal.args:
            _collect_term_vars(argument, masked, seen, ordered)
    elif isinstance(goal, EqExpr | NeqExpr):
        _collect_term_vars(goal.left, masked, seen, ordered)
        _collect_term_vars(goal.right, masked, seen, ordered)
    elif isinstance(goal, DeferredExpr):
        for argument in goal.args:
            _collect_term_vars(argument, masked, seen, ordered)
    elif isinstance(goal, ConjExpr | DisjExpr):
        for child in goal.goals:
            _infer_outputs_from_goal(child, masked, seen, ordered)
    elif isinstance(goal, FreshExpr):
        _infer_outputs_from_goal(
            goal.body,
            masked | frozenset(goal.template_vars),
            seen,
            ordered,
        )

    return tuple(ordered)


def validate(program_value: InstructionProgram) -> None:
    """Validate a standardized instruction stream before lowering it."""

    declared: dict[tuple[Symbol, int], Relation] = {}

    for instruction in program_value.instructions:
        if isinstance(
            instruction,
            RelationDefInstruction | DynamicRelationDefInstruction,
        ):
            key = _relation_key(instruction.relation)
            if key in declared:
                msg = f"relation {instruction.relation} was declared more than once"
                raise ValueError(msg)
            declared[key] = instruction.relation
            continue

        if isinstance(instruction, FactInstruction):
            key = _relation_key(instruction.head.relation)
            if key not in declared:
                msg = f"fact references undeclared relation {instruction.head.relation}"
                raise ValueError(msg)
            if any(_contains_logic_var(argument) for argument in instruction.head.args):
                msg = f"facts must be ground: {instruction.head}"
                raise ValueError(msg)
            continue

        if isinstance(instruction, RuleInstruction):
            key = _relation_key(instruction.head.relation)
            if key not in declared:
                msg = (
                    "rule head references undeclared relation "
                    f"{instruction.head.relation}"
                )
                raise ValueError(msg)
            for relation_call in _iter_relation_calls(instruction.body):
                body_key = _relation_key(relation_call.relation)
                if body_key not in declared:
                    msg = (
                        "rule body references undeclared relation "
                        f"{relation_call.relation}"
                    )
                    raise ValueError(msg)
            continue

        if isinstance(instruction, QueryInstruction):
            for relation_call in _iter_relation_calls(instruction.goal):
                key = _relation_key(relation_call.relation)
                if key not in declared:
                    msg = (
                        "query references undeclared relation "
                        f"{relation_call.relation}"
                    )
                    raise ValueError(msg)
            continue


def assemble(program_value: InstructionProgram) -> AssembledInstructionProgram:
    """Lower instructions into the current ``logic-engine`` backend."""

    validate(program_value)

    clauses: list[Clause] = []
    queries: list[QueryInstruction] = []
    declared: list[Relation] = []
    dynamic_relations: list[Relation] = []

    for instruction in program_value.instructions:
        if isinstance(instruction, RelationDefInstruction):
            declared.append(instruction.relation)
        elif isinstance(instruction, DynamicRelationDefInstruction):
            declared.append(instruction.relation)
            dynamic_relations.append(instruction.relation)
        elif isinstance(instruction, FactInstruction):
            clauses.append(engine_fact(instruction.head))
        elif isinstance(instruction, RuleInstruction):
            clauses.append(engine_rule(instruction.head, instruction.body))
        else:
            queries.append(instruction)

    return AssembledInstructionProgram(
        program=engine_program(*clauses, dynamic_relations=tuple(dynamic_relations)),
        queries=tuple(queries),
        relations=tuple(declared),
    )


def run_query(
    program_value: InstructionProgram,
    query_index: int = 0,
    limit: int | None = None,
) -> list[Term | tuple[Term, ...]]:
    """Execute one stored query through the current direct engine backend."""

    assembled = assemble(program_value)
    try:
        selected = assembled.queries[query_index]
    except IndexError as exc:
        msg = f"query index {query_index} is out of range"
        raise IndexError(msg) from exc

    outputs = selected.outputs
    if outputs is None:
        outputs = _infer_outputs_from_goal(selected.goal)

    query_value: object | tuple[Term, ...]
    query_value = outputs[0] if len(outputs) == 1 else outputs

    if limit is None:
        return solve_all(assembled.program, query_value, selected.goal)

    return solve_n(assembled.program, limit, query_value, selected.goal)


def run_all_queries(
    program_value: InstructionProgram,
    limit: int | None = None,
) -> list[list[Term | tuple[Term, ...]]]:
    """Execute every stored query in source order."""

    assembled = assemble(program_value)
    return [
        run_query(program_value, query_index=index, limit=limit)
        for index, _query in enumerate(assembled.queries)
    ]
