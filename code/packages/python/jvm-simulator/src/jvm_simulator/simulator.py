"""JVM simulator that executes disassembled JVM method bodies."""

from __future__ import annotations

from dataclasses import dataclass

from jvm_bytecode_disassembler import (
    JVMInstruction,
    JVMMethodBody,
    JVMOpcode,
    JVMVersion,
    assemble_jvm,
    disassemble_method_body,
    encode_iconst,
    encode_iload,
    encode_istore,
)
from simulator_protocol import ExecutionResult, StepTrace

from jvm_simulator.state import JVMState


@dataclass
class JVMTrace:
    pc: int
    opcode: str
    stack_before: list[int]
    stack_after: list[int]
    locals_snapshot: list[int | None]
    description: str


class JVMSimulator:
    """Execute a versioned, disassembled JVM method body."""

    def __init__(self, host: object | None = None) -> None:
        self.stack: list[object] = []
        self.locals: list[object | None] = [None] * 16
        self.constants: list[object] = []
        self.pc: int = 0
        self.halted: bool = False
        self.return_value: object | None = None
        self.method: JVMMethodBody | None = None
        self._num_locals: int = 16
        self._constant_pool_lookup: dict[int, object] = {}
        self._bytecode_length: int = 0
        self._decode_error: str | None = None
        self._host = host

    def load_method(self, method: JVMMethodBody) -> None:
        self.method = method
        self._num_locals = method.max_locals or 16
        self._constant_pool_lookup = method.constant_pool_lookup
        self.constants = [value for _, value in sorted(method.constant_pool)]
        self._bytecode_length = sum(
            instruction.size for instruction in method.instructions
        )
        self._decode_error = None
        self.stack = []
        self.locals = [None] * self._num_locals
        self.pc = 0
        self.halted = False
        self.return_value = None

    def load(
        self,
        bytecode: bytes,
        constants: list[object] | None = None,
        num_locals: int = 16,
        version: JVMVersion | None = None,
    ) -> None:
        constant_lookup = (
            {}
            if constants is None
            else {index: value for index, value in enumerate(constants)}
        )
        self._bytecode_length = len(bytecode)
        self._decode_error = None
        try:
            method = disassemble_method_body(
                bytecode,
                version=version,
                max_locals=num_locals,
                constant_pool=constant_lookup,
            )
        except ValueError as exc:
            self.method = None
            self._num_locals = num_locals
            self._constant_pool_lookup = {}
            self.constants = []
            self.stack = []
            self.locals = [None] * self._num_locals
            self.pc = 0
            self.halted = False
            self.return_value = None
            self._decode_error = str(exc)
            return
        self.load_method(method)

    def step(self) -> JVMTrace:
        if self.halted:
            msg = "JVM simulator has halted -- no more instructions to execute"
            raise RuntimeError(msg)
        if self._decode_error is not None:
            raise RuntimeError(self._decode_error)
        if self.method is None:
            msg = "No JVM method body has been loaded"
            raise RuntimeError(msg)
        if self.pc >= self._bytecode_length:
            msg = (
                f"PC ({self.pc}) is past end of bytecode "
                f"({self._bytecode_length} bytes)"
            )
            raise RuntimeError(msg)

        try:
            instruction = self.method.instruction_at(self.pc)
        except KeyError as exc:
            msg = f"PC ({self.pc}) does not point at a valid instruction boundary"
            raise RuntimeError(msg) from exc

        stack_before = list(self.stack)
        return self._execute_instruction(instruction, stack_before)

    def run(self, max_steps: int = 10000) -> list[JVMTrace]:
        traces: list[JVMTrace] = []
        for _ in range(max_steps):
            if self.halted:
                break
            traces.append(self.step())
        return traces

    def get_state(self) -> JVMState:
        return JVMState(
            stack=tuple(self.stack),
            locals=tuple(self.locals),
            constants=tuple(self.constants),
            pc=self.pc,
            halted=self.halted,
            return_value=self.return_value,
        )

    def execute_method(
        self,
        method: JVMMethodBody,
        max_steps: int = 100_000,
    ) -> ExecutionResult[JVMState]:
        self.load_method(method)
        return self._run_execution(max_steps=max_steps)

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
        *,
        version: JVMVersion | None = None,
    ) -> ExecutionResult[JVMState]:
        self.load(program, version=version)
        return self._run_execution(max_steps=max_steps)

    def reset(self) -> None:
        self.stack = []
        self.locals = [None] * self._num_locals
        self.constants = []
        self.pc = 0
        self.halted = False
        self.return_value = None
        self.method = None
        self._constant_pool_lookup = {}
        self._bytecode_length = 0
        self._decode_error = None

    def _run_execution(self, *, max_steps: int) -> ExecutionResult[JVMState]:
        step_traces: list[StepTrace] = []
        steps = 0
        error: str | None = None

        try:
            while not self.halted and steps < max_steps:
                pc_before = self.pc
                jvm_trace = self.step()
                step_traces.append(
                    StepTrace(
                        pc_before=pc_before,
                        pc_after=self.pc,
                        mnemonic=jvm_trace.opcode,
                        description=jvm_trace.description,
                    )
                )
                steps += 1
        except Exception as exc:
            error = str(exc)

        if error is None and not self.halted:
            error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self.halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=step_traces,
        )

    def _execute_instruction(
        self,
        instruction: JVMInstruction,
        stack_before: list[int],
    ) -> JVMTrace:
        pc = instruction.offset
        opcode = instruction.opcode

        if instruction.literal is not None and opcode in {
            JVMOpcode.ICONST_0,
            JVMOpcode.ICONST_1,
            JVMOpcode.ICONST_2,
            JVMOpcode.ICONST_3,
            JVMOpcode.ICONST_4,
            JVMOpcode.ICONST_5,
            JVMOpcode.BIPUSH,
            JVMOpcode.SIPUSH,
        }:
            self.stack.append(instruction.literal)
            self.pc = pc + instruction.size
            return JVMTrace(
                pc=pc,
                opcode=instruction.mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push {instruction.literal}",
            )

        if opcode == JVMOpcode.LDC:
            assert instruction.constant_pool_index is not None
            if instruction.constant_pool_index not in self._constant_pool_lookup:
                msg = (
                    f"Constant pool index {instruction.constant_pool_index} "
                    "is not loadable"
                )
                raise RuntimeError(msg)
            value = self._constant_pool_lookup[instruction.constant_pool_index]
            self.stack.append(value)
            self.pc = pc + instruction.size
            return JVMTrace(
                pc=pc,
                opcode="ldc",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=(
                    f"push constant[{instruction.constant_pool_index}] = {value!r}"
                ),
            )

        if opcode in {
            JVMOpcode.ILOAD,
            JVMOpcode.ILOAD_0,
            JVMOpcode.ILOAD_1,
            JVMOpcode.ILOAD_2,
            JVMOpcode.ILOAD_3,
        }:
            assert instruction.local_slot is not None
            value = self.locals[instruction.local_slot]
            if value is None:
                msg = (
                    f"Local variable {instruction.local_slot} "
                    "has not been initialized"
                )
                raise RuntimeError(msg)
            self.stack.append(value)
            self.pc = pc + instruction.size
            return JVMTrace(
                pc=pc,
                opcode=instruction.mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push locals[{instruction.local_slot}] = {value}",
            )

        if opcode in {
            JVMOpcode.ISTORE,
            JVMOpcode.ISTORE_0,
            JVMOpcode.ISTORE_1,
            JVMOpcode.ISTORE_2,
            JVMOpcode.ISTORE_3,
        }:
            assert instruction.local_slot is not None
            if len(self.stack) < 1:
                msg = f"Stack underflow: {instruction.mnemonic} requires 1 operand"
                raise RuntimeError(msg)
            value = self.stack.pop()
            self.locals[instruction.local_slot] = value
            self.pc = pc + instruction.size
            return JVMTrace(
                pc=pc,
                opcode=instruction.mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {value}, store in locals[{instruction.local_slot}]",
            )

        if opcode == JVMOpcode.IADD:
            return self._do_binary_op(pc, instruction, stack_before, lambda a, b: a + b)
        if opcode == JVMOpcode.ISUB:
            return self._do_binary_op(pc, instruction, stack_before, lambda a, b: a - b)
        if opcode == JVMOpcode.IMUL:
            return self._do_binary_op(pc, instruction, stack_before, lambda a, b: a * b)
        if opcode == JVMOpcode.IDIV:
            if len(self.stack) < 2:
                msg = "Stack underflow: idiv requires 2 operands"
                raise RuntimeError(msg)
            if self.stack[-1] == 0:
                msg = "ArithmeticException: division by zero"
                raise RuntimeError(msg)
            return self._do_binary_op(
                pc,
                instruction,
                stack_before,
                lambda a, b: int(a / b),
            )

        if opcode == JVMOpcode.GOTO:
            assert instruction.branch_target is not None
            self.pc = instruction.branch_target
            return JVMTrace(
                pc=pc,
                opcode="goto",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"jump to PC={instruction.branch_target}",
            )

        if opcode == JVMOpcode.IF_ICMPEQ:
            return self._do_if_icmp(instruction, stack_before, lambda a, b: a == b)
        if opcode == JVMOpcode.IF_ICMPGT:
            return self._do_if_icmp(instruction, stack_before, lambda a, b: a > b)

        if opcode == JVMOpcode.IRETURN:
            if len(self.stack) < 1:
                msg = "Stack underflow: ireturn requires 1 operand"
                raise RuntimeError(msg)
            self.return_value = self.stack.pop()
            self.halted = True
            self.pc = pc + instruction.size
            return JVMTrace(
                pc=pc,
                opcode="ireturn",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"return {self.return_value}",
            )

        if opcode == JVMOpcode.RETURN:
            self.halted = True
            self.pc = pc + instruction.size
            return JVMTrace(
                pc=pc,
                opcode="return",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description="return void",
            )

        if opcode == JVMOpcode.GETSTATIC:
            return self._do_getstatic(instruction, stack_before)

        if opcode == JVMOpcode.INVOKEVIRTUAL:
            return self._do_invokevirtual(instruction, stack_before)

        msg = f"Unimplemented opcode: {instruction.mnemonic} (0x{opcode:02X})"
        raise RuntimeError(msg)

    def _do_binary_op(
        self,
        pc: int,
        instruction: JVMInstruction,
        stack_before: list[int],
        op: object,
        ) -> JVMTrace:
        if len(self.stack) < 2:
            msg = f"Stack underflow: {instruction.mnemonic} requires 2 operands"
            raise RuntimeError(msg)
        b = self.stack.pop()
        a = self.stack.pop()
        if not isinstance(a, int) or not isinstance(b, int):
            msg = f"Type error: {instruction.mnemonic} requires integer operands"
            raise RuntimeError(msg)
        result = op(a, b)  # type: ignore[operator]
        result = self._to_i32(result)
        self.stack.append(result)
        self.pc = pc + instruction.size
        return JVMTrace(
            pc=pc,
            opcode=instruction.mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"pop {b} and {a}, push {result}",
        )

    def _do_if_icmp(
        self,
        instruction: JVMInstruction,
        stack_before: list[int],
        condition: object,
    ) -> JVMTrace:
        if len(self.stack) < 2:
            msg = f"Stack underflow: {instruction.mnemonic} requires 2 operands"
            raise RuntimeError(msg)

        assert instruction.branch_target is not None
        b = self.stack.pop()
        a = self.stack.pop()
        if not isinstance(a, int) or not isinstance(b, int):
            msg = f"Type error: {instruction.mnemonic} requires integer operands"
            raise RuntimeError(msg)
        taken = condition(a, b)  # type: ignore[operator]

        if taken:
            self.pc = instruction.branch_target
            desc = (
                f"pop {b} and {a}, "
                f"{a} {'==' if 'eq' in instruction.mnemonic else '>'} {b} is true, "
                f"jump to PC={instruction.branch_target}"
            )
        else:
            self.pc = instruction.offset + instruction.size
            desc = (
                f"pop {b} and {a}, "
                f"{a} {'==' if 'eq' in instruction.mnemonic else '>'} {b} is false, "
                "fall through"
            )

        return JVMTrace(
            pc=instruction.offset,
            opcode=instruction.mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=desc,
        )

    @staticmethod
    def _to_i32(value: int) -> int:
        value = value & 0xFFFFFFFF
        if value >= 0x80000000:
            value -= 0x100000000
        return value

    def _do_getstatic(
        self,
        instruction: JVMInstruction,
        stack_before: list[object],
    ) -> JVMTrace:
        if self._host is None or not hasattr(self._host, "get_static"):
            msg = "No JVM host is configured for getstatic"
            raise RuntimeError(msg)
        assert instruction.constant_pool_index is not None
        reference = self._constant_pool_lookup.get(instruction.constant_pool_index)
        value = self._host.get_static(reference)  # type: ignore[attr-defined]
        self.stack.append(value)
        self.pc = instruction.offset + instruction.size
        return JVMTrace(
            pc=instruction.offset,
            opcode=instruction.mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"push static field {reference!r}",
        )

    def _do_invokevirtual(
        self,
        instruction: JVMInstruction,
        stack_before: list[object],
    ) -> JVMTrace:
        if self._host is None or not hasattr(self._host, "invoke_virtual"):
            msg = "No JVM host is configured for invokevirtual"
            raise RuntimeError(msg)
        assert instruction.constant_pool_index is not None
        reference = self._constant_pool_lookup.get(instruction.constant_pool_index)
        descriptor = getattr(reference, "descriptor", "")
        arg_count = _descriptor_argument_count(descriptor)
        if len(self.stack) < arg_count + 1:
            msg = "Stack underflow: invokevirtual requires receiver and arguments"
            raise RuntimeError(msg)
        args = [self.stack.pop() for _ in range(arg_count)]
        args.reverse()
        receiver = self.stack.pop()
        result = self._host.invoke_virtual(reference, receiver, args)  # type: ignore[attr-defined]
        self.pc = instruction.offset + instruction.size
        if _descriptor_returns_value(descriptor):
            self.stack.append(result)
            desc = f"invoke {reference!r}, push {result!r}"
        else:
            desc = f"invoke {reference!r}"
        return JVMTrace(
            pc=instruction.offset,
            opcode=instruction.mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=desc,
        )


def _descriptor_argument_count(descriptor: str) -> int:
    if not descriptor.startswith("("):
        return 0
    count = 0
    i = 1
    while i < len(descriptor) and descriptor[i] != ")":
        ch = descriptor[i]
        if ch in "BCDFIJSZ":
            count += 1
            i += 1
        elif ch == "L":
            count += 1
            i = descriptor.index(";", i) + 1
        elif ch == "[":
            i += 1
            while descriptor[i] == "[":
                i += 1
            if descriptor[i] == "L":
                i = descriptor.index(";", i) + 1
            else:
                i += 1
            count += 1
        else:
            raise RuntimeError(f"Unsupported method descriptor: {descriptor}")
    return count


def _descriptor_returns_value(descriptor: str) -> bool:
    return descriptor.partition(")")[2] != "V"


__all__ = [
    "JVMOpcode",
    "JVMSimulator",
    "JVMTrace",
    "JVMVersion",
    "assemble_jvm",
    "disassemble_method_body",
    "encode_iconst",
    "encode_iload",
    "encode_istore",
]
