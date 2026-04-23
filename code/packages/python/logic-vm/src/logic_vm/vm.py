"""Logic VM — a dispatch-table runtime for LP07 instruction streams.

This VM is intentionally small and explicit. It does not try to be a WAM or a
bytecode machine yet. Instead, it answers a simpler and still important
question:

    "If logic programs are an ordered stream of instructions, what runtime
    consumes them?"

The answer in LP08 is a dispatch loop with a mutable runtime state:

- the VM keeps a program counter
- each instruction opcode maps to one handler
- handlers grow runtime state incrementally
- once loading is done, stored queries run through `logic-engine`

That means the logic stack now has both:

- a declarative instruction format (`logic-instructions`)
- an execution chassis that can step, trace, and run those instructions
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field
from typing import Protocol

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
    Term,
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
    rule as engine_rule,
)
from logic_instructions import (
    DynamicRelationDefInstruction,
    FactInstruction,
    InstructionOpcode,
    InstructionProgram,
    QueryInstruction,
    RelationDefInstruction,
    RuleInstruction,
)

type LogicInstruction = (
    RelationDefInstruction
    | DynamicRelationDefInstruction
    | FactInstruction
    | RuleInstruction
    | QueryInstruction
)

type RelationKey = tuple[object, int]


class LogicVMError(Exception):
    """Base class for runtime errors raised by `logic-vm`."""


class LogicVMValidationError(LogicVMError):
    """Raised when an instruction violates runtime invariants while loading."""


class UnknownInstructionOpcodeError(LogicVMError):
    """Raised when the VM encounters an opcode with no registered handler."""


@dataclass(frozen=True, slots=True)
class LogicVMTraceEntry:
    """A lightweight post-step snapshot for tests and future tracing tools."""

    instruction_index: int
    opcode: InstructionOpcode
    relation_count: int
    clause_count: int
    query_count: int


@dataclass(slots=True)
class LogicVMState:
    """Mutable runtime state owned by one `LogicVM` instance.

    The first logic VM does not yet model choice points, trails, or stack
    frames. Its job is narrower: it loads the high-level instruction stream,
    remembers the declarations and clauses it has seen so far, and stores the
    queries that should later be run.
    """

    program: InstructionProgram | None = None
    instruction_pointer: int = 0
    halted: bool = True
    relations: dict[RelationKey, Relation] = field(default_factory=dict)
    dynamic_relations: dict[RelationKey, Relation] = field(default_factory=dict)
    clauses: list[Clause] = field(default_factory=list)
    queries: list[QueryInstruction] = field(default_factory=list)


class LogicInstructionHandler(Protocol):
    """Protocol implemented by every instruction handler."""

    def __call__(self, vm: LogicVM, instruction: LogicInstruction) -> None: ...


def _relation_key(relation_value: Relation) -> RelationKey:
    """Use the relation's stable `(symbol, arity)` identity as the registry key."""

    return relation_value.key()


def _contains_logic_var(term_value: Term) -> bool:
    """Facts must be ground, so any variable inside the term is rejected."""

    if isinstance(term_value, LogicVar):
        return True
    if isinstance(term_value, Compound):
        return any(_contains_logic_var(argument) for argument in term_value.args)
    return False


def _iter_relation_calls(goal: GoalExpr) -> Iterator[RelationCall]:
    """Yield the explicit relation calls inside a goal tree.

    Deferred goals stay opaque at this layer. Inspecting them statically would
    require executing host-language callbacks, which defeats the point of the
    runtime boundary.
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


def _require_declared_relation(
    state: LogicVMState,
    relation_value: Relation,
    context: str,
) -> None:
    """Require that a relation has already been declared in the runtime."""

    if _relation_key(relation_value) not in state.relations:
        msg = f"{context} references undeclared relation {relation_value}"
        raise LogicVMValidationError(msg)


def _handle_defrel(vm: LogicVM, instruction: LogicInstruction) -> None:
    """Register one relation in the runtime registry."""

    if not isinstance(instruction, RelationDefInstruction):
        msg = "DEF_REL handler received a non-definition instruction"
        raise TypeError(msg)

    key = _relation_key(instruction.relation)
    if key in vm.state.relations:
        msg = f"relation {instruction.relation} was declared more than once"
        raise LogicVMValidationError(msg)

    vm.state.relations[key] = instruction.relation


def _handle_dynamic_rel(vm: LogicVM, instruction: LogicInstruction) -> None:
    """Register one dynamic relation in the runtime registry."""

    if not isinstance(instruction, DynamicRelationDefInstruction):
        msg = "DYNAMIC_REL handler received a non-definition instruction"
        raise TypeError(msg)

    key = _relation_key(instruction.relation)
    if key in vm.state.relations:
        msg = f"relation {instruction.relation} was declared more than once"
        raise LogicVMValidationError(msg)

    vm.state.relations[key] = instruction.relation
    vm.state.dynamic_relations[key] = instruction.relation


def _handle_fact(vm: LogicVM, instruction: LogicInstruction) -> None:
    """Validate and load one fact clause into the runtime."""

    if not isinstance(instruction, FactInstruction):
        msg = "FACT handler received a non-fact instruction"
        raise TypeError(msg)

    _require_declared_relation(vm.state, instruction.head.relation, "fact")
    if any(_contains_logic_var(argument) for argument in instruction.head.args):
        msg = f"facts must be ground: {instruction.head}"
        raise LogicVMValidationError(msg)

    vm.state.clauses.append(engine_fact(instruction.head))


def _handle_rule(vm: LogicVM, instruction: LogicInstruction) -> None:
    """Validate and load one rule clause into the runtime."""

    if not isinstance(instruction, RuleInstruction):
        msg = "RULE handler received a non-rule instruction"
        raise TypeError(msg)

    _require_declared_relation(vm.state, instruction.head.relation, "rule head")
    for relation_call in _iter_relation_calls(instruction.body):
        _require_declared_relation(vm.state, relation_call.relation, "rule body")

    vm.state.clauses.append(engine_rule(instruction.head, instruction.body))


def _handle_query(vm: LogicVM, instruction: LogicInstruction) -> None:
    """Validate and store one query for later execution."""

    if not isinstance(instruction, QueryInstruction):
        msg = "QUERY handler received a non-query instruction"
        raise TypeError(msg)

    for relation_call in _iter_relation_calls(instruction.goal):
        _require_declared_relation(vm.state, relation_call.relation, "query")

    vm.state.queries.append(instruction)


class LogicVM:
    """A dispatch-table VM for standardized logic instructions.

    The VM's job is split into two phases:

    1. Load-time: walk the instruction stream and accumulate runtime state.
    2. Query-time: turn the accumulated clauses into a `logic-engine.Program`
       and execute stored queries.

    This gives us a real runtime boundary now without locking the project into
    a low-level opcode format too early.
    """

    def __init__(self) -> None:
        self.state = LogicVMState()
        self._handlers: dict[InstructionOpcode, LogicInstructionHandler] = {}

    def register(
        self,
        opcode: InstructionOpcode,
        handler: LogicInstructionHandler,
    ) -> None:
        """Register one opcode handler.

        Double registration is rejected immediately so the VM configuration is
        stable and predictable.
        """

        if opcode in self._handlers:
            msg = f"handler already registered for opcode {opcode}"
            raise LogicVMError(msg)
        self._handlers[opcode] = handler

    def reset(self) -> None:
        """Clear all loaded runtime state while keeping registered handlers."""

        self.state = LogicVMState()

    def load(self, program: InstructionProgram) -> None:
        """Install a new instruction stream and reset the runtime state."""

        self.reset()
        self.state.program = program
        self.state.instruction_pointer = 0
        self.state.halted = len(program.instructions) == 0

    def _require_loaded_program(self) -> InstructionProgram:
        """Return the loaded program or raise a VM error."""

        if self.state.program is None:
            msg = "logic-vm has no loaded instruction program"
            raise LogicVMError(msg)
        return self.state.program

    def step(self) -> LogicVMTraceEntry:
        """Execute exactly one instruction and return a post-step trace entry."""

        program_value = self._require_loaded_program()
        if self.state.halted:
            msg = "logic-vm is halted"
            raise LogicVMError(msg)

        instruction_index = self.state.instruction_pointer
        instruction = program_value.instructions[instruction_index]
        handler = self._handlers.get(instruction.opcode)
        if handler is None:
            msg = f"no handler registered for opcode {instruction.opcode}"
            raise UnknownInstructionOpcodeError(msg)

        handler(self, instruction)

        self.state.instruction_pointer += 1
        if self.state.instruction_pointer >= len(program_value.instructions):
            self.state.halted = True

        return LogicVMTraceEntry(
            instruction_index=instruction_index,
            opcode=instruction.opcode,
            relation_count=len(self.state.relations),
            clause_count=len(self.state.clauses),
            query_count=len(self.state.queries),
        )

    def run(self) -> list[LogicVMTraceEntry]:
        """Execute instructions until the current program halts."""

        self._require_loaded_program()
        trace: list[LogicVMTraceEntry] = []
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
            msg = "logic-vm must finish loading before queries can run"
            raise LogicVMError(msg)

    def run_query(
        self,
        query_index: int = 0,
        limit: int | None = None,
    ) -> list[Term | tuple[Term, ...]]:
        """Execute one stored query against the loaded runtime state."""

        self._require_finished_loading()
        try:
            selected = self.state.queries[query_index]
        except IndexError as exc:
            msg = f"query index {query_index} is out of range"
            raise LogicVMError(msg) from exc

        outputs = selected.outputs
        if outputs is None:
            outputs = _infer_outputs_from_goal(selected.goal)

        query_value: object | tuple[Term, ...]
        query_value = outputs[0] if len(outputs) == 1 else outputs

        if limit is None:
            return solve_all(self.assembled_program(), query_value, selected.goal)
        return solve_n(self.assembled_program(), limit, query_value, selected.goal)

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


def create_logic_vm() -> LogicVM:
    """Create a `LogicVM` with handlers for the current LP07 instruction set."""

    vm = LogicVM()
    vm.register(InstructionOpcode.DEF_REL, _handle_defrel)
    vm.register(InstructionOpcode.DYNAMIC_REL, _handle_dynamic_rel)
    vm.register(InstructionOpcode.FACT, _handle_fact)
    vm.register(InstructionOpcode.RULE, _handle_rule)
    vm.register(InstructionOpcode.QUERY, _handle_query)
    return vm


def execute(
    program: InstructionProgram,
    query_index: int = 0,
    limit: int | None = None,
) -> list[Term | tuple[Term, ...]]:
    """Load a program into a fresh VM, run it, and execute one stored query."""

    vm = create_logic_vm()
    vm.load(program)
    vm.run()
    return vm.run_query(query_index=query_index, limit=limit)


def execute_all(
    program: InstructionProgram,
    limit: int | None = None,
) -> list[list[Term | tuple[Term, ...]]]:
    """Load a program into a fresh VM, run it, and execute every stored query."""

    vm = create_logic_vm()
    vm.load(program)
    vm.run()
    return vm.run_all_queries(limit=limit)
