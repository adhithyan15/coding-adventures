"""Logic Bytecode — compact loader opcodes for standardized logic programs.

LP07 instruction streams are rich and readable Python objects. That is ideal
for teaching and direct manipulation, but not yet the same thing as bytecode.

This module introduces the first compact opcode layer for the logic stack:

- opcodes are integers
- operands are pool indexes
- bytecode programs carry separate pools for the referenced objects
- decoding reconstructs the original instruction stream

This is still a *loader* bytecode rather than a proof-search bytecode. Its job
is to compact and standardize the current LP07 model, not to replace the
eventual lower-level execution format.
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass
from enum import IntEnum

from logic_engine import ConjExpr, DisjExpr, FreshExpr, GoalExpr, Relation, RelationCall
from logic_instructions import (
    DynamicRelationDefInstruction,
    FactInstruction,
    InstructionProgram,
    QueryInstruction,
    RelationDefInstruction,
    RuleInstruction,
    instruction_program,
    validate,
)

__all__ = [
    "LogicBytecodeDisassemblyLine",
    "LogicBytecodeError",
    "LogicBytecodeInstruction",
    "LogicBytecodeOp",
    "LogicBytecodeProgram",
    "compile_program",
    "decode_program",
    "disassemble",
    "disassemble_text",
]


class LogicBytecodeOp(IntEnum):
    """Opcode values for the first logic loader bytecode.

    The values are intentionally sparse. This leaves room for later groups of
    execution-oriented opcodes without renumbering the initial loader layer.
    """

    EMIT_RELATION = 0x00
    EMIT_FACT = 0x01
    EMIT_RULE = 0x02
    EMIT_QUERY = 0x03
    EMIT_DYNAMIC_RELATION = 0x04
    HALT = 0xF0


class LogicBytecodeError(Exception):
    """Raised when a bytecode program is malformed or cannot be decoded."""


@dataclass(frozen=True, slots=True)
class LogicBytecodeInstruction:
    """A single loader-bytecode instruction.

    `opcode` is stored as an integer instead of only as `LogicBytecodeOp`
    because real bytecode is raw numeric data. That makes malformed-opcode
    tests straightforward and keeps decoding explicit.
    """

    opcode: int
    operand: int | None = None


@dataclass(frozen=True, slots=True)
class LogicBytecodeProgram:
    """A compiled bytecode object plus the pools it indexes into."""

    instructions: tuple[LogicBytecodeInstruction, ...]
    relation_pool: tuple[Relation, ...]
    fact_pool: tuple[FactInstruction, ...]
    rule_pool: tuple[RuleInstruction, ...]
    query_pool: tuple[QueryInstruction, ...]


@dataclass(frozen=True, slots=True)
class LogicBytecodeDisassemblyLine:
    """One rendered line of human-readable disassembly."""

    index: int
    opcode: str
    operand: int | None = None
    comment: str | None = None

    def __str__(self) -> str:
        if self.operand is None:
            base = f"{self.index:04d}: {self.opcode}"
        else:
            base = f"{self.index:04d}: {self.opcode} {self.operand}"
        if self.comment is None:
            return base
        return f"{base} ; {self.comment}"


def _normalize_opcode(opcode_value: int) -> LogicBytecodeOp:
    """Convert a raw integer opcode into the bytecode enum or raise."""

    try:
        return LogicBytecodeOp(opcode_value)
    except ValueError as exc:
        msg = f"unknown logic bytecode opcode 0x{opcode_value:02X}"
        raise LogicBytecodeError(msg) from exc


def _iter_relation_calls(goal: GoalExpr) -> Iterator[RelationCall]:
    """Yield the explicit relation calls nested inside a goal tree."""

    if isinstance(goal, RelationCall):
        yield goal
        return

    if isinstance(goal, ConjExpr | DisjExpr):
        for child in goal.goals:
            yield from _iter_relation_calls(child)
        return

    if isinstance(goal, FreshExpr):
        yield from _iter_relation_calls(goal.body)


def _collect_relations(program_value: InstructionProgram) -> tuple[Relation, ...]:
    """Collect every relation mentioned in the instruction stream once."""

    seen: set[tuple[object, int]] = set()
    ordered: list[Relation] = []

    def visit(relation_value: Relation) -> None:
        key = relation_value.key()
        if key not in seen:
            seen.add(key)
            ordered.append(relation_value)

    for instruction in program_value.instructions:
        if isinstance(
            instruction,
            RelationDefInstruction | DynamicRelationDefInstruction,
        ):
            visit(instruction.relation)
        elif isinstance(instruction, FactInstruction):
            visit(instruction.head.relation)
        elif isinstance(instruction, RuleInstruction):
            visit(instruction.head.relation)
            for relation_call in _iter_relation_calls(instruction.body):
                visit(relation_call.relation)
        else:
            for relation_call in _iter_relation_calls(instruction.goal):
                visit(relation_call.relation)

    return tuple(ordered)


def compile_program(program_value: InstructionProgram) -> LogicBytecodeProgram:
    """Compile an LP07 instruction stream into loader bytecode."""

    validate(program_value)

    relation_pool = _collect_relations(program_value)
    relation_index = {
        relation_value.key(): index
        for index, relation_value in enumerate(relation_pool)
    }

    fact_pool: list[FactInstruction] = []
    rule_pool: list[RuleInstruction] = []
    query_pool: list[QueryInstruction] = []
    instructions: list[LogicBytecodeInstruction] = []

    for instruction in program_value.instructions:
        if isinstance(instruction, DynamicRelationDefInstruction):
            instructions.append(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_DYNAMIC_RELATION,
                    operand=relation_index[instruction.relation.key()],
                ),
            )
        elif isinstance(instruction, RelationDefInstruction):
            instructions.append(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_RELATION,
                    operand=relation_index[instruction.relation.key()],
                ),
            )
        elif isinstance(instruction, FactInstruction):
            fact_pool.append(instruction)
            instructions.append(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_FACT,
                    operand=len(fact_pool) - 1,
                ),
            )
        elif isinstance(instruction, RuleInstruction):
            rule_pool.append(instruction)
            instructions.append(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_RULE,
                    operand=len(rule_pool) - 1,
                ),
            )
        else:
            query_pool.append(instruction)
            instructions.append(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_QUERY,
                    operand=len(query_pool) - 1,
                ),
            )

    instructions.append(LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT))

    return LogicBytecodeProgram(
        instructions=tuple(instructions),
        relation_pool=relation_pool,
        fact_pool=tuple(fact_pool),
        rule_pool=tuple(rule_pool),
        query_pool=tuple(query_pool),
    )


def _require_operand(instruction: LogicBytecodeInstruction, index: int) -> int:
    """Require that a pool-indexed instruction carries an operand."""

    if instruction.operand is None:
        msg = f"instruction {index} requires an operand"
        raise LogicBytecodeError(msg)
    return instruction.operand


def _pool_get[T](pool: tuple[T, ...], pool_name: str, index: int) -> T:
    """Return one pool entry or raise a bytecode error with context."""

    if index < 0:
        msg = f"{pool_name} index {index} is out of range"
        raise LogicBytecodeError(msg)

    try:
        return pool[index]
    except IndexError as exc:
        msg = f"{pool_name} index {index} is out of range"
        raise LogicBytecodeError(msg) from exc


def decode_program(program_value: LogicBytecodeProgram) -> InstructionProgram:
    """Decode loader bytecode back into the original LP07 instruction stream."""

    decoded_instructions: list[
        RelationDefInstruction
        | DynamicRelationDefInstruction
        | FactInstruction
        | RuleInstruction
        | QueryInstruction
    ] = []
    halted = False

    for index, instruction in enumerate(program_value.instructions):
        opcode = _normalize_opcode(instruction.opcode)

        if opcode is LogicBytecodeOp.HALT:
            halted = True
            if index != len(program_value.instructions) - 1:
                msg = "HALT must be the final bytecode instruction"
                raise LogicBytecodeError(msg)
            break

        operand = _require_operand(instruction, index)
        if opcode is LogicBytecodeOp.EMIT_DYNAMIC_RELATION:
            decoded_instructions.append(
                DynamicRelationDefInstruction(
                    relation=_pool_get(
                        program_value.relation_pool,
                        "relation pool",
                        operand,
                    ),
                ),
            )
        elif opcode is LogicBytecodeOp.EMIT_RELATION:
            decoded_instructions.append(
                RelationDefInstruction(
                    relation=_pool_get(
                        program_value.relation_pool,
                        "relation pool",
                        operand,
                    ),
                ),
            )
        elif opcode is LogicBytecodeOp.EMIT_FACT:
            decoded_instructions.append(
                _pool_get(program_value.fact_pool, "fact pool", operand),
            )
        elif opcode is LogicBytecodeOp.EMIT_RULE:
            decoded_instructions.append(
                _pool_get(program_value.rule_pool, "rule pool", operand),
            )
        else:
            decoded_instructions.append(
                _pool_get(program_value.query_pool, "query pool", operand),
            )

    if not halted:
        msg = "bytecode program is missing a HALT instruction"
        raise LogicBytecodeError(msg)

    return instruction_program(*decoded_instructions)


def _comment_for_instruction(
    program_value: LogicBytecodeProgram,
    instruction: LogicBytecodeInstruction,
    opcode: LogicBytecodeOp,
    index: int,
) -> str | None:
    """Render the referenced pool entry as a short comment."""

    if opcode is LogicBytecodeOp.HALT:
        return None

    operand = _require_operand(instruction, index)
    if opcode in {
        LogicBytecodeOp.EMIT_RELATION,
        LogicBytecodeOp.EMIT_DYNAMIC_RELATION,
    }:
        return str(_pool_get(program_value.relation_pool, "relation pool", operand))
    if opcode is LogicBytecodeOp.EMIT_FACT:
        return str(_pool_get(program_value.fact_pool, "fact pool", operand).head)
    if opcode is LogicBytecodeOp.EMIT_RULE:
        rule_value = _pool_get(program_value.rule_pool, "rule pool", operand)
        return f"{rule_value.head} :- {rule_value.body}"

    query_value = _pool_get(program_value.query_pool, "query pool", operand)
    return str(query_value.goal)


def disassemble(
    program_value: LogicBytecodeProgram,
) -> tuple[LogicBytecodeDisassemblyLine, ...]:
    """Disassemble bytecode into structured human-readable lines."""

    lines: list[LogicBytecodeDisassemblyLine] = []
    halted = False

    for index, instruction in enumerate(program_value.instructions):
        opcode = _normalize_opcode(instruction.opcode)
        lines.append(
            LogicBytecodeDisassemblyLine(
                index=index,
                opcode=opcode.name,
                operand=instruction.operand,
                comment=_comment_for_instruction(
                    program_value,
                    instruction,
                    opcode,
                    index,
                ),
            ),
        )
        if opcode is LogicBytecodeOp.HALT:
            halted = True
            if index != len(program_value.instructions) - 1:
                msg = "HALT must be the final bytecode instruction"
                raise LogicBytecodeError(msg)
            break

    if not halted:
        msg = "bytecode program is missing a HALT instruction"
        raise LogicBytecodeError(msg)

    return tuple(lines)


def disassemble_text(program_value: LogicBytecodeProgram) -> str:
    """Return disassembly lines joined into one multi-line string."""

    return "\n".join(str(line) for line in disassemble(program_value))
