"""Composable CLR VM simulator backed by the CLI runtime model."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol

from cli_runtime_model import (
    CLI_INT32,
    CLI_OBJECT,
    CLI_STRING,
    CLI_VOID,
    CliCallArguments,
    CliCallKind,
    CliEvaluationStack,
    CliMethodDescriptor,
    CliMethodSignature,
    CliRuntimeModelError,
    CliType,
    CliValue,
    ClrFrame,
    ClrFrameSnapshot,
    ClrHeap,
    ClrThreadSnapshot,
    ClrThreadState,
    collect_call_arguments,
)
from clr_bytecode_disassembler import (
    CLRInstruction,
    CLRMethodBody,
    disassemble_clr_method,
)
from clr_pe_file import (
    CLRMemberReference,
    CLRMethodDef,
    CLRMethodSignature,
    CLRPEFile,
    decode_clr_pe_file,
)

_GE225_TO_ASCII = {
    0o00: "0",
    0o01: "1",
    0o02: "2",
    0o03: "3",
    0o04: "4",
    0o05: "5",
    0o06: "6",
    0o07: "7",
    0o10: "8",
    0o11: "9",
    0o13: "/",
    0o21: "A",
    0o22: "B",
    0o23: "C",
    0o24: "D",
    0o25: "E",
    0o26: "F",
    0o27: "G",
    0o30: "H",
    0o31: "I",
    0o33: "-",
    0o37: "\n",
    0o40: ".",
    0o41: "J",
    0o42: "K",
    0o43: "L",
    0o44: "M",
    0o45: "N",
    0o46: "O",
    0o47: "P",
    0o50: "Q",
    0o51: "R",
    0o53: "$",
    0o60: " ",
    0o62: "S",
    0o63: "T",
    0o64: "U",
    0o65: "V",
    0o66: "W",
    0o67: "X",
    0o70: "Y",
    0o71: "Z",
}


class CLRVMError(RuntimeError):
    """Raised when CLR VM execution fails."""


class _CLRVMExit(RuntimeError):
    def __init__(self, code: int) -> None:
        super().__init__(code)
        self.code = code


class CLRVMHost(Protocol):
    """Host bridge for external MemberRef calls."""

    output: list[str]

    def call_member(
        self,
        method: CLRMemberReference,
        args: tuple[CliValue, ...],
    ) -> CliValue | None:
        """Handle an external method call."""


@dataclass(frozen=True)
class CLRVMTrace:
    """One executed instruction and its frame snapshots."""

    method: str
    offset: int
    opcode: str
    operand: object | None
    frame_before: ClrFrameSnapshot
    frame_after: ClrFrameSnapshot


@dataclass(frozen=True)
class CLRVMResult:
    """Result of executing a CLR method or assembly entry point."""

    return_value: CliValue | None
    output: str
    traces: tuple[CLRVMTrace, ...]
    final_thread: ClrThreadSnapshot


class CLRVMStdlibHost:
    """Default host for console output and compiler-runtime helpers."""

    def __init__(
        self,
        *,
        memory_size: int = 65536,
        input_bytes: bytes = b"",
    ) -> None:
        self.output: list[str] = []
        self._memory = bytearray(memory_size)
        self._words: dict[int, int] = {}
        self._input = bytes(input_bytes)
        self._input_offset = 0
        self.exit_code: int | None = None

    def call_member(
        self,
        method: CLRMemberReference,
        args: tuple[CliValue, ...],
    ) -> CliValue | None:
        """Handle supported external CLR calls."""
        if (
            method.declaring_type == "System.Console"
            and method.name == "WriteLine"
            and method.signature.parameter_types == ("string",)
        ):
            self.output.append(f"{_as_python(args[0])}\n")
            return None

        if method.name == "__ca_syscall":
            return self._call_syscall(args)
        if method.name == "__ca_mem_load_byte":
            address = _as_int32(args[0])
            if address < 0 or address >= len(self._memory):
                return CliValue.int32(0)
            return CliValue.int32(self._memory[address])
        if method.name == "__ca_mem_store_byte":
            address = _as_int32(args[0])
            value = _as_int32(args[1])
            if 0 <= address < len(self._memory):
                self._memory[address] = value & 0xFF
            return None
        if method.name == "__ca_load_word":
            return CliValue.int32(self._words.get(_as_int32(args[0]), 0))
        if method.name == "__ca_store_word":
            self._words[_as_int32(args[0])] = _as_int32(args[1])
            return None

        msg = (
            "unsupported CLR host call "
            f"{method.declaring_type}.{method.name}"
            f"{method.signature.parameter_types}"
        )
        raise CLRVMError(msg)

    def _call_syscall(self, args: tuple[CliValue, ...]) -> CliValue:
        number = _as_int32(args[0])
        value = _as_int32(args[1])
        if number == 1:
            self.output.append(_GE225_TO_ASCII.get(value, chr(value & 0xFF)))
            return CliValue.int32(0)
        if number == 2:
            if self._input_offset >= len(self._input):
                return CliValue.int32(0)
            byte = self._input[self._input_offset]
            self._input_offset += 1
            return CliValue.int32(byte)
        if number == 10:
            self.exit_code = value
            raise _CLRVMExit(value)
        msg = f"unsupported compiler helper syscall: {number}"
        raise CLRVMError(msg)


class CLRVM:
    """Execute decoded CLR assemblies with composable runtime state."""

    def __init__(
        self,
        *,
        host: CLRVMHost | None = None,
        max_steps: int = 10000,
    ) -> None:
        if max_steps <= 0:
            msg = "max_steps must be positive"
            raise CLRVMError(msg)
        self.host = host or CLRVMStdlibHost()
        self.max_steps = max_steps
        self.heap = ClrHeap()
        self.thread = ClrThreadState()
        self._assembly: CLRPEFile | None = None
        self._traces: list[CLRVMTrace] = []

    def run_assembly(self, assembly_bytes: bytes) -> CLRVMResult:
        """Decode and execute the assembly entry point."""
        return self.run_entry_point(decode_clr_pe_file(assembly_bytes))

    def run_entry_point(self, assembly: CLRPEFile) -> CLRVMResult:
        """Execute ``assembly`` from its entry point."""
        entry = assembly.get_entry_point_method()
        args = tuple(
            CliValue.default_for(_type_from_name(name))
            for name in entry.signature.parameter_types
        )
        return self.run_method(assembly, entry, args)

    def run_method(
        self,
        assembly: CLRPEFile,
        method: CLRMethodDef,
        args: tuple[CliValue, ...] = (),
    ) -> CLRVMResult:
        """Execute ``method`` in ``assembly`` and return the VM result."""
        self._assembly = assembly
        self._traces.clear()
        self.host.output.clear()
        self.thread = ClrThreadState()
        try:
            return_value = self._invoke_method_def(method, args)
        except _CLRVMExit as exc:
            return_value = CliValue.int32(exc.code)
        return CLRVMResult(
            return_value=return_value,
            output="".join(self.host.output),
            traces=tuple(self._traces),
            final_thread=self.thread.snapshot(),
        )

    def _invoke_method_def(
        self,
        method: CLRMethodDef,
        args: tuple[CliValue, ...],
    ) -> CliValue | None:
        assembly = self._require_assembly()
        body = disassemble_clr_method(assembly, method)
        descriptor = _method_descriptor(method)
        frame = ClrFrame.create(
            descriptor,
            args,
            local_types=tuple(CLI_INT32 for _ in range(body.local_count)),
        )
        for local in frame.locals:
            local.store(CliValue.default_for(local.declared_type))
        self.thread.push_frame(frame)
        try:
            return self._execute_body(body, method)
        finally:
            self.thread.pop_frame()

    def _execute_body(
        self,
        body: CLRMethodBody,
        method: CLRMethodDef,
    ) -> CliValue | None:
        instruction_map = {
            instruction.offset: instruction for instruction in body.instructions
        }
        frame = self.thread.current_frame
        frame.instruction_pointer = (
            body.instructions[0].offset if body.instructions else 0
        )
        steps = 0
        while True:
            if steps >= self.max_steps:
                msg = f"CLR VM exceeded max_steps={self.max_steps}"
                raise CLRVMError(msg)
            instruction = instruction_map.get(frame.instruction_pointer)
            if instruction is None:
                msg = (
                    f"no instruction at IL offset {frame.instruction_pointer} "
                    f"in {method.name}"
                )
                raise CLRVMError(msg)
            steps += 1
            frame_before = frame.snapshot()
            return_value = self._execute_instruction(frame, instruction)
            frame_after = frame.snapshot()
            self._traces.append(
                CLRVMTrace(
                    method=method.name,
                    offset=instruction.offset,
                    opcode=instruction.opcode,
                    operand=instruction.operand,
                    frame_before=frame_before,
                    frame_after=frame_after,
                )
            )
            if instruction.opcode == "ret":
                return return_value

    def _execute_instruction(
        self,
        frame: ClrFrame,
        instruction: CLRInstruction,
    ) -> CliValue | None:
        opcode = instruction.opcode
        next_offset = instruction.offset + instruction.size

        if opcode == "nop":
            frame.instruction_pointer = next_offset
            return None
        if opcode == "ldnull":
            frame.evaluation_stack.push(CliValue.null(CLI_OBJECT))
            frame.instruction_pointer = next_offset
            return None
        if opcode.startswith("ldc.i4"):
            frame.evaluation_stack.push(CliValue.int32(_int_operand(instruction)))
            frame.instruction_pointer = next_offset
            return None
        if opcode == "ldstr":
            if not isinstance(instruction.operand, str):
                msg = "ldstr requires a string operand"
                raise CLRVMError(msg)
            frame.evaluation_stack.push(CliValue.string(instruction.operand))
            frame.instruction_pointer = next_offset
            return None
        if opcode.startswith("ldarg"):
            frame.evaluation_stack.push(frame.load_argument(_int_operand(instruction)))
            frame.instruction_pointer = next_offset
            return None
        if opcode.startswith("starg"):
            frame.store_argument(
                _int_operand(instruction),
                frame.evaluation_stack.pop(),
            )
            frame.instruction_pointer = next_offset
            return None
        if opcode.startswith("ldloc"):
            frame.evaluation_stack.push(frame.load_local(_int_operand(instruction)))
            frame.instruction_pointer = next_offset
            return None
        if opcode.startswith("stloc"):
            frame.store_local(_int_operand(instruction), frame.evaluation_stack.pop())
            frame.instruction_pointer = next_offset
            return None
        if opcode in {"add", "sub", "mul", "div", "and", "or", "shl", "shr"}:
            self._execute_binary(frame.evaluation_stack, opcode)
            frame.instruction_pointer = next_offset
            return None
        if opcode in {"ceq", "cgt", "clt"}:
            self._execute_compare(frame.evaluation_stack, opcode)
            frame.instruction_pointer = next_offset
            return None
        if opcode in {"br", "br.s"}:
            frame.instruction_pointer = _branch_target(instruction)
            return None
        if opcode in {"brfalse", "brfalse.s", "brtrue", "brtrue.s"}:
            value = _as_int32(frame.evaluation_stack.pop())
            take = value == 0 if opcode.startswith("brfalse") else value != 0
            frame.instruction_pointer = (
                _branch_target(instruction) if take else next_offset
            )
            return None
        if opcode in {
            "beq",
            "beq.s",
            "bge",
            "bge.s",
            "bgt",
            "bgt.s",
            "ble",
            "ble.s",
            "blt",
            "blt.s",
            "bne.un",
            "bne.un.s",
        }:
            take = self._evaluate_relational_branch(frame.evaluation_stack, opcode)
            frame.instruction_pointer = (
                _branch_target(instruction) if take else next_offset
            )
            return None
        if opcode in {"call", "callvirt"}:
            self._execute_call(frame, instruction, CliCallKind(opcode))
            frame.instruction_pointer = next_offset
            return None
        if opcode == "ret":
            return frame.evaluation_stack.pop() if len(frame.evaluation_stack) else None

        msg = f"unsupported CLR VM opcode: {opcode}"
        raise CLRVMError(msg)

    def _execute_binary(self, stack: CliEvaluationStack, opcode: str) -> None:
        rhs = _as_int32(stack.pop())
        lhs = _as_int32(stack.pop())
        if opcode == "add":
            result = lhs + rhs
        elif opcode == "sub":
            result = lhs - rhs
        elif opcode == "mul":
            result = lhs * rhs
        elif opcode == "div":
            if rhs == 0:
                msg = "division by zero"
                raise CLRVMError(msg)
            result = int(lhs / rhs)
        elif opcode == "and":
            result = lhs & rhs
        elif opcode == "or":
            result = lhs | rhs
        elif opcode == "shl":
            result = lhs << rhs
        else:
            result = lhs >> rhs
        stack.push(CliValue.int32(_int32(result)))

    def _execute_compare(self, stack: CliEvaluationStack, opcode: str) -> None:
        rhs = _as_int32(stack.pop())
        lhs = _as_int32(stack.pop())
        if opcode == "ceq":
            result = lhs == rhs
        elif opcode == "cgt":
            result = lhs > rhs
        else:
            result = lhs < rhs
        stack.push(CliValue.int32(1 if result else 0))

    def _evaluate_relational_branch(
        self,
        stack: CliEvaluationStack,
        opcode: str,
    ) -> bool:
        rhs = _as_int32(stack.pop())
        lhs = _as_int32(stack.pop())
        root = opcode.removesuffix(".s")
        if root == "beq":
            return lhs == rhs
        if root == "bge":
            return lhs >= rhs
        if root == "bgt":
            return lhs > rhs
        if root == "ble":
            return lhs <= rhs
        if root == "blt":
            return lhs < rhs
        return lhs != rhs

    def _execute_call(
        self,
        frame: ClrFrame,
        instruction: CLRInstruction,
        kind: CliCallKind,
    ) -> None:
        operand = instruction.operand
        if isinstance(operand, CLRMethodDef):
            descriptor = _method_descriptor(operand)
            call_args = _collect_call_arguments(
                frame.evaluation_stack, descriptor, kind
            )
            result = self._invoke_method_def(operand, call_args.parameters)
        elif isinstance(operand, CLRMemberReference):
            descriptor = _member_descriptor(operand)
            call_args = _collect_call_arguments(
                frame.evaluation_stack, descriptor, kind
            )
            result = self.host.call_member(operand, call_args.parameters)
        else:
            msg = f"unsupported call operand: {operand!r}"
            raise CLRVMError(msg)

        if result is not None:
            frame.evaluation_stack.push(result)

    def _require_assembly(self) -> CLRPEFile:
        if self._assembly is None:
            msg = "CLR VM has no assembly loaded"
            raise CLRVMError(msg)
        return self._assembly


def run_clr_entry_point(
    assembly_bytes: bytes,
    *,
    host: CLRVMHost | None = None,
    max_steps: int = 10000,
) -> CLRVMResult:
    """Decode and execute a managed assembly entry point."""
    return CLRVM(host=host, max_steps=max_steps).run_assembly(assembly_bytes)


def _method_descriptor(method: CLRMethodDef) -> CliMethodDescriptor:
    return CliMethodDescriptor(
        token=method.token,
        declaring_type=CliType.reference(method.declaring_type),
        name=method.name,
        signature=_signature(method.signature),
        is_static=not method.signature.has_this,
    )


def _member_descriptor(member: CLRMemberReference) -> CliMethodDescriptor:
    return CliMethodDescriptor(
        token=member.token,
        declaring_type=CliType.reference(member.declaring_type),
        name=member.name,
        signature=_signature(member.signature),
        is_static=not member.signature.has_this,
    )


def _signature(signature: CLRMethodSignature) -> CliMethodSignature:
    return CliMethodSignature(
        parameter_types=tuple(
            _type_from_name(name) for name in signature.parameter_types
        ),
        return_type=_type_from_name(signature.return_type),
        has_this=signature.has_this,
    )


def _collect_call_arguments(
    stack: CliEvaluationStack,
    method: CliMethodDescriptor,
    kind: CliCallKind,
) -> CliCallArguments:
    try:
        return collect_call_arguments(stack, method, kind=kind)
    except CliRuntimeModelError as exc:
        raise CLRVMError(str(exc)) from exc


def _type_from_name(name: str) -> CliType:
    if name == "void":
        return CLI_VOID
    if name in {"int32", "bool", "char", "int64"}:
        return (
            CLI_INT32
            if name in {"int32", "bool", "char"}
            else CliType.primitive(name)
        )
    if name == "string":
        return CLI_STRING
    if name == "object":
        return CLI_OBJECT
    if name.endswith("[]"):
        return CliType.szarray(_type_from_name(name[:-2]))
    return CliType.reference(name)


def _int_operand(instruction: CLRInstruction) -> int:
    if not isinstance(instruction.operand, int):
        msg = f"{instruction.opcode} requires an integer operand"
        raise CLRVMError(msg)
    return instruction.operand


def _branch_target(instruction: CLRInstruction) -> int:
    if not isinstance(instruction.operand, int):
        msg = f"{instruction.opcode} requires a branch target"
        raise CLRVMError(msg)
    return instruction.operand


def _as_int32(value: CliValue) -> int:
    if value.cli_type != CLI_INT32 or not isinstance(value.value, int):
        msg = f"expected int32 value, got {value.cli_type.name}"
        raise CLRVMError(msg)
    return value.value


def _as_python(value: CliValue) -> object | None:
    if value.is_null:
        return None
    return value.value


def _int32(value: int) -> int:
    value &= 0xFFFF_FFFF
    if value >= 0x8000_0000:
        value -= 0x1_0000_0000
    return value
