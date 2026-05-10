"""Logic Bytecode VM — direct execution for LP09 loader bytecode.

LP09 introduced a compact bytecode format for standardized logic programs, but
that format was still passive data. This module gives the bytecode a runtime:

- the VM keeps a bytecode instruction pointer
- handlers are registered per opcode
- operands are resolved through bytecode pools at runtime
- once loading finishes, stored queries execute through `logic-engine`

This is still a loader VM rather than a low-level WAM. Its job is to make the
first bytecode layer executable and inspectable.
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field
from itertools import islice
from typing import Protocol

from logic_bytecode import (
    LogicBytecodeInstruction,
    LogicBytecodeOp,
    LogicBytecodeProgram,
    compile_program,
)
from logic_engine import (
    Clause,
    Compound,
    ConjExpr,
    DeferredExpr,
    DisjExpr,
    EqExpr,
    FreshExpr,
    GoalExpr,
    LogicVar,
    NeqExpr,
    Program,
    Relation,
    RelationCall,
    State,
    Term,
    reify,
    solve_from,
)
from logic_engine import (
    fact as engine_fact,
)
from logic_engine import (
    program as engine_program,
)
from logic_engine import (
    rule as engine_rule,
)
from logic_instructions import InstructionProgram, QueryInstruction

type RelationKey = tuple[object, int]
type QueryValue = Term | tuple[Term, ...]


class LogicBytecodeVMError(Exception):
    """Base class for runtime errors raised by `logic-bytecode-vm`."""


class LogicBytecodeVMValidationError(LogicBytecodeVMError):
    """Raised when bytecode violates runtime invariants while loading."""


class UnknownLogicBytecodeOpcodeError(LogicBytecodeVMError):
    """Raised when the VM encounters an unknown raw bytecode opcode."""


@dataclass(frozen=True, slots=True)
class LogicBytecodeVMTraceEntry:
    """A lightweight post-step snapshot for tests and tracing tools."""

    instruction_index: int
    opcode: LogicBytecodeOp
    relation_count: int
    clause_count: int
    query_count: int


@dataclass(slots=True)
class LogicBytecodeVMState:
    """Mutable runtime state owned by one `LogicBytecodeVM` instance."""

    program: LogicBytecodeProgram | None = None
    instruction_pointer: int = 0
    halted: bool = True
    relations: dict[RelationKey, Relation] = field(default_factory=dict)
    dynamic_relations: dict[RelationKey, Relation] = field(default_factory=dict)
    clauses: list[Clause] = field(default_factory=list)
    queries: list[QueryInstruction] = field(default_factory=list)


class LogicBytecodeHandler(Protocol):
    """Protocol implemented by every bytecode opcode handler."""

    def __call__(
        self,
        vm: LogicBytecodeVM,
        instruction: LogicBytecodeInstruction,
    ) -> None:
        ...


def _relation_key(relation_value: Relation) -> RelationKey:
    """Use the relation's stable `(symbol, arity)` identity as the registry key."""

    return relation_value.key()


def _contains_logic_var(term_value: Term) -> bool:
    """Facts must be ground, so variables anywhere inside a term are rejected."""

    if isinstance(term_value, LogicVar):
        return True
    if isinstance(term_value, Compound):
        return any(_contains_logic_var(argument) for argument in term_value.args)
    return False


def _iter_relation_calls(goal: GoalExpr) -> Iterator[RelationCall]:
    """Yield explicit relation calls nested inside a goal tree."""

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
    """Collect free query variables in first-appearance order."""

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
    """Infer query outputs from the goal's free variables."""

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


def _normalize_opcode(opcode_value: int) -> LogicBytecodeOp:
    """Convert one raw integer opcode into the bytecode enum or raise."""

    try:
        return LogicBytecodeOp(opcode_value)
    except ValueError as exc:
        msg = f"unknown logic bytecode opcode 0x{opcode_value:02X}"
        raise UnknownLogicBytecodeOpcodeError(msg) from exc


def _require_operand(instruction: LogicBytecodeInstruction, index: int) -> int:
    """Require that a pool-indexed instruction carries an operand."""

    if instruction.operand is None:
        msg = f"instruction {index} requires an operand"
        raise LogicBytecodeVMValidationError(msg)
    return instruction.operand


def _pool_get[T](pool: tuple[T, ...], pool_name: str, index: int) -> T:
    """Return one pool entry or raise a VM validation error with context."""

    if index < 0:
        msg = f"{pool_name} index {index} is out of range"
        raise LogicBytecodeVMValidationError(msg)

    try:
        return pool[index]
    except IndexError as exc:
        msg = f"{pool_name} index {index} is out of range"
        raise LogicBytecodeVMValidationError(msg) from exc


def _require_declared_relation(
    state: LogicBytecodeVMState,
    relation_value: Relation,
    context: str,
) -> None:
    """Require that a relation has already been declared in the runtime."""

    if _relation_key(relation_value) not in state.relations:
        msg = f"{context} references undeclared relation {relation_value}"
        raise LogicBytecodeVMValidationError(msg)


def _handle_emit_relation(
    vm: LogicBytecodeVM,
    instruction: LogicBytecodeInstruction,
) -> None:
    """Register one relation in the runtime registry."""

    program_value = vm._require_loaded_program()
    operand = _require_operand(instruction, vm.state.instruction_pointer)
    relation_value = _pool_get(program_value.relation_pool, "relation pool", operand)

    key = _relation_key(relation_value)
    if key in vm.state.relations:
        msg = f"relation {relation_value} was declared more than once"
        raise LogicBytecodeVMValidationError(msg)

    vm.state.relations[key] = relation_value


def _handle_emit_dynamic_relation(
    vm: LogicBytecodeVM,
    instruction: LogicBytecodeInstruction,
) -> None:
    """Register one dynamic relation in the runtime registry."""

    program_value = vm._require_loaded_program()
    operand = _require_operand(instruction, vm.state.instruction_pointer)
    relation_value = _pool_get(program_value.relation_pool, "relation pool", operand)

    key = _relation_key(relation_value)
    if key in vm.state.relations:
        msg = f"relation {relation_value} was declared more than once"
        raise LogicBytecodeVMValidationError(msg)

    vm.state.relations[key] = relation_value
    vm.state.dynamic_relations[key] = relation_value


def _handle_emit_fact(
    vm: LogicBytecodeVM,
    instruction: LogicBytecodeInstruction,
) -> None:
    """Validate and load one fact clause into the runtime."""

    program_value = vm._require_loaded_program()
    operand = _require_operand(instruction, vm.state.instruction_pointer)
    fact_instruction = _pool_get(program_value.fact_pool, "fact pool", operand)

    _require_declared_relation(vm.state, fact_instruction.head.relation, "fact")
    if any(_contains_logic_var(argument) for argument in fact_instruction.head.args):
        msg = f"facts must be ground: {fact_instruction.head}"
        raise LogicBytecodeVMValidationError(msg)

    vm.state.clauses.append(engine_fact(fact_instruction.head))


def _handle_emit_rule(
    vm: LogicBytecodeVM,
    instruction: LogicBytecodeInstruction,
) -> None:
    """Validate and load one rule clause into the runtime."""

    program_value = vm._require_loaded_program()
    operand = _require_operand(instruction, vm.state.instruction_pointer)
    rule_instruction = _pool_get(program_value.rule_pool, "rule pool", operand)

    _require_declared_relation(vm.state, rule_instruction.head.relation, "rule head")
    for relation_call in _iter_relation_calls(rule_instruction.body):
        _require_declared_relation(vm.state, relation_call.relation, "rule body")

    vm.state.clauses.append(engine_rule(rule_instruction.head, rule_instruction.body))


def _handle_emit_query(
    vm: LogicBytecodeVM,
    instruction: LogicBytecodeInstruction,
) -> None:
    """Validate and store one query for later execution."""

    program_value = vm._require_loaded_program()
    operand = _require_operand(instruction, vm.state.instruction_pointer)
    query_instruction = _pool_get(program_value.query_pool, "query pool", operand)

    for relation_call in _iter_relation_calls(query_instruction.goal):
        _require_declared_relation(vm.state, relation_call.relation, "query")

    vm.state.queries.append(query_instruction)


def _handle_halt(vm: LogicBytecodeVM, instruction: LogicBytecodeInstruction) -> None:
    """Validate final `HALT` placement and mark the VM as halted."""

    del instruction
    program_value = vm._require_loaded_program()
    if vm.state.instruction_pointer != len(program_value.instructions) - 1:
        msg = "HALT must be the final bytecode instruction"
        raise LogicBytecodeVMValidationError(msg)
    vm.state.halted = True


class LogicBytecodeVM:
    """A dispatch-table VM for LP09 loader bytecode."""

    def __init__(self) -> None:
        self.state = LogicBytecodeVMState()
        self._handlers: dict[LogicBytecodeOp, LogicBytecodeHandler] = {}

    def register(
        self,
        opcode: LogicBytecodeOp,
        handler: LogicBytecodeHandler,
    ) -> None:
        """Register one opcode handler."""

        if opcode in self._handlers:
            msg = f"handler already registered for opcode {opcode}"
            raise LogicBytecodeVMError(msg)
        self._handlers[opcode] = handler

    def reset(self) -> None:
        """Clear all loaded runtime state while keeping registered handlers."""

        self.state = LogicBytecodeVMState()

    def load(self, program: LogicBytecodeProgram) -> None:
        """Install a new bytecode program and reset the runtime state."""

        self.reset()
        self.state.program = program
        self.state.instruction_pointer = 0
        self.state.halted = False

    def _require_loaded_program(self) -> LogicBytecodeProgram:
        """Return the loaded bytecode program or raise a VM error."""

        if self.state.program is None:
            msg = "logic-bytecode-vm has no loaded bytecode program"
            raise LogicBytecodeVMError(msg)
        return self.state.program

    def step(self) -> LogicBytecodeVMTraceEntry:
        """Execute exactly one bytecode instruction and return a trace entry."""

        program_value = self._require_loaded_program()
        if self.state.halted:
            msg = "logic-bytecode-vm is halted"
            raise LogicBytecodeVMError(msg)
        if self.state.instruction_pointer >= len(program_value.instructions):
            msg = "bytecode program is missing a final HALT instruction"
            raise LogicBytecodeVMValidationError(msg)

        instruction_index = self.state.instruction_pointer
        instruction = program_value.instructions[instruction_index]
        opcode = _normalize_opcode(instruction.opcode)
        handler = self._handlers.get(opcode)
        if handler is None:
            msg = f"no handler registered for bytecode opcode {opcode}"
            raise LogicBytecodeVMError(msg)

        handler(self, instruction)

        if not self.state.halted:
            self.state.instruction_pointer += 1
            if self.state.instruction_pointer >= len(program_value.instructions):
                msg = "bytecode program is missing a final HALT instruction"
                raise LogicBytecodeVMValidationError(msg)

        return LogicBytecodeVMTraceEntry(
            instruction_index=instruction_index,
            opcode=opcode,
            relation_count=len(self.state.relations),
            clause_count=len(self.state.clauses),
            query_count=len(self.state.queries),
        )

    def run(self) -> list[LogicBytecodeVMTraceEntry]:
        """Execute instructions until the current program halts."""

        self._require_loaded_program()
        trace: list[LogicBytecodeVMTraceEntry] = []
        while not self.state.halted:
            trace.append(self.step())
        return trace

    def assembled_program(self) -> Program:
        """Return the currently loaded clauses as an immutable engine program."""

        return engine_program(
            *self.state.clauses,
            dynamic_relations=tuple(self.state.dynamic_relations.values()),
        )

    def _require_finished_loading(self) -> None:
        """Require that the loaded program has finished executing."""

        self._require_loaded_program()
        if not self.state.halted:
            msg = "logic-bytecode-vm must finish loading before queries can run"
            raise LogicBytecodeVMError(msg)

    def run_query(
        self,
        query_index: int = 0,
        limit: int | None = None,
    ) -> list[Term | tuple[Term, ...]]:
        """Execute one stored query against the loaded runtime state."""

        return self.run_query_from(State(), query_index=query_index, limit=limit)

    def solve_query_from(
        self,
        state: State,
        query_index: int = 0,
    ) -> Iterator[State]:
        """Yield proof states for one stored query from an existing state."""

        self._require_finished_loading()
        try:
            selected = self.state.queries[query_index]
        except IndexError as exc:
            msg = f"query index {query_index} is out of range"
            raise LogicBytecodeVMError(msg) from exc

        yield from solve_from(self.assembled_program(), selected.goal, state)

    def run_query_from(
        self,
        state: State,
        query_index: int = 0,
        limit: int | None = None,
    ) -> list[Term | tuple[Term, ...]]:
        """Execute one stored query from an existing logic state."""

        self._require_finished_loading()
        try:
            selected = self.state.queries[query_index]
        except IndexError as exc:
            msg = f"query index {query_index} is out of range"
            raise LogicBytecodeVMError(msg) from exc

        outputs = selected.outputs
        if outputs is None:
            outputs = _infer_outputs_from_goal(selected.goal)

        query_value: QueryValue
        query_value = outputs[0] if len(outputs) == 1 else outputs

        proof_states = self.solve_query_from(state, query_index=query_index)
        if limit is not None:
            if limit < 0:
                msg = "run_query_from() requires a non-negative limit"
                raise ValueError(msg)
            proof_states = islice(proof_states, limit)

        results: list[Term | tuple[Term, ...]] = []
        for proof_state in proof_states:
            if isinstance(query_value, tuple):
                results.append(
                    tuple(
                        reify(item, proof_state.substitution)
                        for item in query_value
                    ),
                )
            else:
                results.append(reify(query_value, proof_state.substitution))
        return results

    def run_all_queries(
        self,
        limit: int | None = None,
    ) -> list[list[Term | tuple[Term, ...]]]:
        """Execute every stored query in source order."""

        self._require_finished_loading()
        return [
            self.run_query(query_index=index, limit=limit)
            for index, _query in enumerate(self.state.queries)
        ]

    def run_all_queries_from(
        self,
        state: State,
        limit: int | None = None,
    ) -> list[list[Term | tuple[Term, ...]]]:
        """Execute every stored query from an existing logic state."""

        self._require_finished_loading()
        return [
            self.run_query_from(state, query_index=index, limit=limit)
            for index, _query in enumerate(self.state.queries)
        ]


def create_logic_bytecode_vm() -> LogicBytecodeVM:
    """Create a `LogicBytecodeVM` with handlers for the current bytecode set."""

    vm = LogicBytecodeVM()
    vm.register(LogicBytecodeOp.EMIT_RELATION, _handle_emit_relation)
    vm.register(LogicBytecodeOp.EMIT_DYNAMIC_RELATION, _handle_emit_dynamic_relation)
    vm.register(LogicBytecodeOp.EMIT_FACT, _handle_emit_fact)
    vm.register(LogicBytecodeOp.EMIT_RULE, _handle_emit_rule)
    vm.register(LogicBytecodeOp.EMIT_QUERY, _handle_emit_query)
    vm.register(LogicBytecodeOp.HALT, _handle_halt)
    return vm


def execute(
    program: LogicBytecodeProgram,
    query_index: int = 0,
    limit: int | None = None,
) -> list[Term | tuple[Term, ...]]:
    """Load bytecode into a fresh VM, run it, and execute one stored query."""

    vm = create_logic_bytecode_vm()
    vm.load(program)
    vm.run()
    return vm.run_query(query_index=query_index, limit=limit)


def execute_all(
    program: LogicBytecodeProgram,
    limit: int | None = None,
) -> list[list[Term | tuple[Term, ...]]]:
    """Load bytecode into a fresh VM, run it, and execute every stored query."""

    vm = create_logic_bytecode_vm()
    vm.load(program)
    vm.run()
    return vm.run_all_queries(limit=limit)


def compile_and_execute(
    instruction_program: InstructionProgram,
    query_index: int = 0,
    limit: int | None = None,
) -> list[Term | tuple[Term, ...]]:
    """Compile one instruction stream to bytecode, then execute one query."""

    return execute(
        compile_program(instruction_program),
        query_index=query_index,
        limit=limit,
    )


def compile_and_execute_all(
    instruction_program: InstructionProgram,
    limit: int | None = None,
) -> list[list[Term | tuple[Term, ...]]]:
    """Compile one instruction stream to bytecode, then execute all queries."""

    return execute_all(compile_program(instruction_program), limit=limit)
