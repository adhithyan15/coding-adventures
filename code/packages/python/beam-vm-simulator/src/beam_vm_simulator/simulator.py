"""A small but runnable BEAM VM simulator."""

from __future__ import annotations

from dataclasses import dataclass

from beam_bytecode_disassembler import (
    BeamDisassembledModule,
    BeamInstruction,
    BeamOperand,
    disassemble_bytes,
)
from beam_opcode_metadata import OTP_28_PROFILE, BeamProfile
from simulator_protocol import ExecutionResult, StepTrace


@dataclass(frozen=True)
class BeamVMState:
    """Immutable snapshot of the BEAM simulator state."""

    profile_name: str
    module_name: str | None
    pc: int
    halted: bool
    x_registers: tuple[object | None, ...]
    y_registers: tuple[object | None, ...]
    call_depth: int


class BeamVMSimulator:
    """A minimal BEAM simulator for a carefully chosen subset of opcodes."""

    def __init__(
        self,
        profile: BeamProfile = OTP_28_PROFILE,
        entry_function: str = "main",
        entry_arity: int = 0,
    ) -> None:
        self.profile = profile
        self.entry_function = entry_function
        self.entry_arity = entry_arity
        self.module: BeamDisassembledModule | None = None
        self.pc = 0
        self.halted = False
        self.x_registers: list[object | None] = [None] * 16
        self.y_registers: list[object | None] = []
        self.call_stack: list[int] = []

        self._bifs: dict[tuple[str, str, int], object] = {
            ("erlang", "+", 2): lambda a, b: a + b,
            ("erlang", "-", 2): lambda a, b: a - b,
            ("erlang", "*", 2): lambda a, b: a * b,
            ("erlang", "div", 2): lambda a, b: a // b,
        }

    def load(self, program: bytes) -> None:
        """Load and disassemble a raw `.beam` module."""
        self.reset()
        self.module = disassemble_bytes(program, self.profile)
        self.pc = self.module.find_export(self.entry_function, self.entry_arity)

    def step(self) -> StepTrace:
        """Execute one BEAM instruction."""
        if self.module is None:
            msg = "No BEAM module has been loaded"
            raise RuntimeError(msg)
        if self.halted:
            msg = "The simulator has halted"
            raise RuntimeError(msg)
        if self.pc >= len(self.module.instructions):
            msg = f"PC {self.pc} is past the end of the instruction stream"
            raise RuntimeError(msg)

        instruction = self.module.instructions[self.pc]
        pc_before = self.pc
        self._execute_instruction(instruction)
        return StepTrace(
            pc_before=pc_before,
            pc_after=self.pc,
            mnemonic=instruction.opcode.name,
            description=f"{instruction.opcode.name} @ instruction {pc_before}",
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> ExecutionResult[BeamVMState]:
        """Load a `.beam` module, execute it, and return the final result."""
        self.load(program)
        traces: list[StepTrace] = []
        steps = 0
        error: str | None = None

        try:
            while not self.halted and steps < max_steps:
                traces.append(self.step())
                steps += 1
        except Exception as exc:  # pragma: no cover - defensive
            error = str(exc)

        if error is None and not self.halted:
            error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self.halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=traces,
        )

    def get_state(self) -> BeamVMState:
        """Return a frozen state snapshot."""
        return BeamVMState(
            profile_name=self.profile.name,
            module_name=self.module.module_name if self.module is not None else None,
            pc=self.pc,
            halted=self.halted,
            x_registers=tuple(self.x_registers),
            y_registers=tuple(self.y_registers),
            call_depth=len(self.call_stack),
        )

    def reset(self) -> None:
        """Reset execution state while preserving configuration."""
        self.pc = 0
        self.halted = False
        self.x_registers = [None] * 16
        self.y_registers = []
        self.call_stack = []

    def _execute_instruction(self, instruction: BeamInstruction) -> None:
        name = instruction.opcode.name

        if name in {"func_info", "label", "line", "debug_line", "executable_line"}:
            self.pc += 1
            return

        if name == "move":
            source = self._read_operand(instruction.args[0])
            self._write_operand(instruction.args[1], source)
            self.pc += 1
            return

        if name == "call_ext":
            arity = int(self._read_operand(instruction.args[0]))
            import_index = int(self._read_operand(instruction.args[1]))
            self._call_import(import_index, arity)
            self.pc += 1
            return

        if name == "call_ext_only":
            arity = int(self._read_operand(instruction.args[0]))
            import_index = int(self._read_operand(instruction.args[1]))
            self._call_import(import_index, arity)
            self.halted = True
            return

        if name == "call":
            label = self._expect_label(instruction.args[1])
            self.call_stack.append(self.pc + 1)
            self.pc = self._resolve_label(label)
            return

        if name == "call_only":
            label = self._expect_label(instruction.args[1])
            self.pc = self._resolve_label(label)
            return

        if name == "jump":
            label = self._expect_label(instruction.args[0])
            self.pc = self._resolve_label(label)
            return

        if name == "return":
            if self.call_stack:
                self.pc = self.call_stack.pop()
            else:
                self.halted = True
            return

        if name in {
            "allocate",
            "allocate_heap",
            "test_heap",
            "deallocate",
            "init_yregs",
        }:
            self.pc += 1
            return

        msg = f"Unsupported opcode in initial BEAM simulator slice: {name}"
        raise NotImplementedError(msg)

    def _read_operand(self, operand: BeamOperand) -> object:
        if operand.kind in {"integer", "u", "atom", "nil"}:
            return operand.value
        if operand.kind == "x":
            return self.x_registers[int(operand.value)]
        if operand.kind == "y":
            return self.y_registers[int(operand.value)]
        if operand.kind == "label":
            return operand.value
        msg = f"Unsupported operand kind {operand.kind!r}"
        raise NotImplementedError(msg)

    def _write_operand(self, operand: BeamOperand, value: object) -> None:
        if operand.kind == "x":
            self.x_registers[int(operand.value)] = value
            return
        if operand.kind == "y":
            index = int(operand.value)
            while len(self.y_registers) <= index:
                self.y_registers.append(None)
            self.y_registers[index] = value
            return
        msg = f"Operand kind {operand.kind!r} is not writable"
        raise ValueError(msg)

    def _expect_label(self, operand: BeamOperand) -> int:
        if operand.kind != "label":
            msg = f"Expected label operand, got {operand.kind!r}"
            raise ValueError(msg)
        return int(operand.value)

    def _resolve_label(self, label: int) -> int:
        if self.module is None:
            msg = "No module is loaded"
            raise RuntimeError(msg)
        return self.module.label_to_index[label]

    def _call_import(self, import_index: int, arity: int) -> None:
        if self.module is None:
            msg = "No module is loaded"
            raise RuntimeError(msg)
        module_name, function_name, import_arity = self.module.imports[import_index]
        if import_arity != arity:
            msg = (
                f"Arity mismatch for import "
                f"{module_name}:{function_name}/{import_arity}"
            )
            raise ValueError(msg)
        bif = self._bifs.get((module_name, function_name, arity))
        if bif is None:
            msg = f"Unsupported import {module_name}:{function_name}/{arity}"
            raise NotImplementedError(msg)
        args = [self.x_registers[index] for index in range(arity)]
        self.x_registers[0] = bif(*args)  # type: ignore[misc]
