"""Lower the generic compiler IR into a WebAssembly 1.0 module."""

from __future__ import annotations

import math
import re
import struct
from dataclasses import dataclass

from compiler_ir import (
    IrDataDecl,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from wasm_leb128 import encode_signed, encode_unsigned
from wasm_opcodes import get_opcode_by_name
from wasm_types import (
    BlockType,
    DataSegment,
    Export,
    ExternalKind,
    FunctionBody,
    FuncType,
    Import,
    Limits,
    MemoryType,
    ValueType,
    WasmModule,
)

_LOOP_START_RE = re.compile(r"^loop_\d+_start$")
_IF_ELSE_RE = re.compile(r"^if_\d+_else$")
_FUNCTION_COMMENT_RE = re.compile(r"^function:\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$")

_SYSCALL_WRITE = 1
_SYSCALL_READ = 2
_SYSCALL_EXIT = 10

_WASI_MODULE = "wasi_snapshot_preview1"
_WASI_IOVEC_OFFSET = 0
_WASI_COUNT_OFFSET = 8
_WASI_BYTE_OFFSET = 12
_WASI_SCRATCH_SIZE = 16

_MEMORY_OPS = frozenset(
    {
        IrOp.LOAD_ADDR,
        IrOp.LOAD_BYTE,
        IrOp.STORE_BYTE,
        IrOp.LOAD_WORD,
        IrOp.STORE_WORD,
        IrOp.LOAD_F64,
        IrOp.STORE_F64,
    }
)

_OPCODE = {
    name: get_opcode_by_name(name).opcode  # type: ignore[union-attr]
    for name in (
        "nop",
        "block",
        "loop",
        "if",
        "else",
        "end",
        "br",
        "br_if",
        "return",
        "call",
        "local.get",
        "local.set",
        "i32.load",
        "i32.load8_u",
        "i32.store",
        "i32.store8",
        "i32.const",
        "i32.eqz",
        "i32.eq",
        "i32.ne",
        "i32.lt_s",
        "i32.gt_s",
        "i32.add",
        "i32.sub",
        "i32.and",
        "i32.or",
        "i32.xor",
        "i32.mul",
        "i32.div_s",
        "f64.load",
        "f64.store",
        "f64.const",
        "f64.eq",
        "f64.ne",
        "f64.lt",
        "f64.gt",
        "f64.le",
        "f64.ge",
        "f64.add",
        "f64.sub",
        "f64.mul",
        "f64.div",
        "f64.sqrt",
        "f64.convert_i32_s",
        "i32.trunc_f64_s",
        "drop",
    )
}


# ---------------------------------------------------------------------------
# WASM i32 range and supported opcode set
# ---------------------------------------------------------------------------
#
# WASM uses 32-bit two's-complement integers (``i32``).
# Signed range: -2 147 483 648 (−2^31) to 2 147 483 647 (2^31 − 1).

_WASM_I32_MIN: int = -(1 << 31)   # -2 147 483 648
_WASM_I32_MAX: int =  (1 << 31) - 1  # 2 147 483 647

# The V1 WASM backend handles exactly these opcodes.
_WASM_SUPPORTED_OPCODES: frozenset[IrOp] = frozenset({
    IrOp.LABEL,
    IrOp.COMMENT,
    IrOp.NOP,
    IrOp.HALT,
    IrOp.JUMP,
    IrOp.LOAD_IMM,
    IrOp.LOAD_ADDR,
    IrOp.LOAD_BYTE,
    IrOp.LOAD_WORD,
    IrOp.STORE_BYTE,
    IrOp.STORE_WORD,
    IrOp.ADD,
    IrOp.ADD_IMM,
    IrOp.SUB,
    IrOp.AND,
    IrOp.AND_IMM,
    IrOp.OR,
    IrOp.OR_IMM,
    IrOp.XOR,
    IrOp.XOR_IMM,
    IrOp.NOT,
    IrOp.LOAD_F64_IMM,
    IrOp.LOAD_F64,
    IrOp.STORE_F64,
    IrOp.F64_ADD,
    IrOp.F64_SUB,
    IrOp.F64_MUL,
    IrOp.F64_DIV,
    IrOp.F64_SQRT,
    IrOp.F64_CMP_EQ,
    IrOp.F64_CMP_NE,
    IrOp.F64_CMP_LT,
    IrOp.F64_CMP_GT,
    IrOp.F64_CMP_LE,
    IrOp.F64_CMP_GE,
    IrOp.F64_FROM_I32,
    IrOp.I32_TRUNC_FROM_F64,
    IrOp.MUL,
    IrOp.DIV,
    IrOp.CMP_EQ,
    IrOp.CMP_NE,
    IrOp.CMP_LT,
    IrOp.CMP_GT,
    IrOp.BRANCH_Z,
    IrOp.BRANCH_NZ,
    IrOp.CALL,
    IrOp.RET,
    IrOp.SYSCALL,
})

# WASM WASI syscalls supported by the V1 backend (must mirror _SYSCALL_* constants):
#   SYSCALL 1  → _SYSCALL_WRITE / fd_write (print one byte to stdout)
#   SYSCALL 2  → _SYSCALL_READ  / fd_read  (read one byte from stdin)
#   SYSCALL 10 → _SYSCALL_EXIT  / proc_exit
_WASM_SUPPORTED_SYSCALLS: frozenset[int] = frozenset({
    _SYSCALL_WRITE,   # 1
    _SYSCALL_READ,    # 2
    _SYSCALL_EXIT,    # 10
})


def validate_for_wasm(program: IrProgram) -> list[str]:
    """Inspect ``program`` for WASM backend incompatibilities without
    generating any WASM bytes.

    Checks performed:

    1. **Opcode support** — every opcode must appear in
       ``_WASM_SUPPORTED_OPCODES``.  Unknown opcodes are rejected before any
       module bytes are produced.

    2. **Constant range** — every ``IrImmediate`` in a ``LOAD_IMM`` or
       ``ADD_IMM`` instruction must fit in a WASM 32-bit signed integer
       (−2 147 483 648 to 2 147 483 647).  WASM's ``i32.const`` encodes
       its operand as a 32-bit LEB128 value; constants outside this range
       cannot be represented.

    3. **SYSCALL number** — only the WASI syscall numbers in
       ``_WASM_SUPPORTED_SYSCALLS`` (1=fd_write, 4=fd_read, 10=proc_exit)
       are wired up in the V1 WASM backend.  Any other number is rejected.

    Args:
        program: The ``IrProgram`` to inspect.

    Returns:
        A list of human-readable error strings.  An empty list means the
        program is compatible with the WASM V1 backend.
    """
    errors: list[str] = []

    for instr in program.instructions:
        op = instr.opcode

        # ── Rule 1: opcode must be in the supported set ─────────────────────
        if op not in _WASM_SUPPORTED_OPCODES:
            errors.append(
                f"unsupported opcode {op.name} in V1 WASM backend"
            )
            continue

        # ── Rule 2: constant range on LOAD_IMM and ADD_IMM ──────────────────
        if op in (IrOp.LOAD_IMM, IrOp.ADD_IMM):
            for operand in instr.operands:
                if isinstance(operand, IrImmediate):
                    v = operand.value
                    if not (_WASM_I32_MIN <= v <= _WASM_I32_MAX):
                        errors.append(
                            f"{op.name}: constant {v:,} overflows WASM i32 "
                            f"(valid range {_WASM_I32_MIN:,} to {_WASM_I32_MAX:,})"
                        )

        # ── Rule 3: SYSCALL number ───────────────────────────────────────────
        elif op == IrOp.SYSCALL:
            for operand in instr.operands:
                if isinstance(operand, IrImmediate) and operand.value not in _WASM_SUPPORTED_SYSCALLS:
                    errors.append(
                        f"unsupported SYSCALL {operand.value}: "
                        f"only SYSCALL numbers {sorted(_WASM_SUPPORTED_SYSCALLS)} "
                        f"are wired in the V1 WASM backend"
                    )
                    break

    return errors


class WasmLoweringError(Exception):
    """Raised when an IrProgram cannot be lowered to WASM."""


@dataclass(frozen=True)
class FunctionSignature:
    """WASM-facing signature metadata for a lowered IR function."""

    label: str
    param_count: int
    export_name: str | None = None
    require_explicit_args: bool = False
    param_types: tuple[ValueType, ...] | None = None
    result_types: tuple[ValueType, ...] = (ValueType.I32,)

    def __post_init__(self) -> None:
        if self.param_types is not None and len(self.param_types) != self.param_count:
            raise ValueError(
                f"FunctionSignature({self.label!r}) param_types length "
                f"{len(self.param_types)} does not match param_count {self.param_count}"
            )
        if len(self.result_types) > 1:
            raise ValueError(
                f"FunctionSignature({self.label!r}) supports at most one result"
            )

    @property
    def wasm_param_types(self) -> tuple[ValueType, ...]:
        if self.param_types is not None:
            return self.param_types
        return (ValueType.I32,) * self.param_count

    @property
    def wasm_result_types(self) -> tuple[ValueType, ...]:
        return self.result_types


@dataclass(frozen=True)
class _FunctionIR:
    label: str
    instructions: list[IrInstruction]
    signature: FunctionSignature
    max_reg: int
    register_types: tuple[ValueType, ...]


@dataclass(frozen=True)
class _WasiImport:
    syscall_number: int
    name: str
    func_type: FuncType

    @property
    def type_key(self) -> str:
        return f"wasi::{self.name}"


@dataclass(frozen=True)
class _WasiContext:
    function_indices: dict[int, int]
    scratch_base: int | None


class IrToWasmCompiler:
    """Compile a generic IrProgram into a WasmModule."""

    def compile(
        self,
        program: IrProgram,
        function_signatures: list[FunctionSignature] | None = None,
        *,
        strategy: str = "structured",
    ) -> WasmModule:
        errors = validate_for_wasm(program)
        if errors:
            joined = "; ".join(errors)
            raise WasmLoweringError(
                f"IR program failed WASM pre-flight validation "
                f"({len(errors)} error{'s' if len(errors) != 1 else ''}): {joined}"
            )
        signatures = infer_function_signatures_from_comments(program)
        if function_signatures:
            for signature in function_signatures:
                signatures[signature.label] = signature

        functions = self._split_functions(program, signatures)
        imports = self._collect_wasi_imports(program)
        type_indices, types = self._build_type_table(functions, imports)
        data_offsets = self._layout_data(program.data)
        scratch_base = None
        if self._needs_wasi_scratch(program):
            scratch_base = _align_up(sum(decl.size for decl in program.data), 4)

        module = WasmModule()
        module.types.extend(types)
        module.imports.extend(
            Import(
                module_name=_WASI_MODULE,
                name=imp.name,
                kind=ExternalKind.FUNCTION,
                type_info=type_indices[imp.type_key],
            )
            for imp in imports
        )
        function_index_base = len(imports)
        function_indices = {
            function.label: function_index_base + index
            for index, function in enumerate(functions)
        }
        module.functions.extend(type_indices[function.label] for function in functions)

        total_bytes = sum(decl.size for decl in program.data)
        if scratch_base is not None:
            total_bytes = max(total_bytes, scratch_base + _WASI_SCRATCH_SIZE)

        if self._needs_memory(program) or scratch_base is not None:
            page_count = max(1, math.ceil(total_bytes / 65536)) if total_bytes else 1
            module.memories.append(MemoryType(limits=Limits(min=page_count, max=None)))
            module.exports.append(Export(name="memory", kind=ExternalKind.MEMORY, index=0))
            module.data.extend(
                DataSegment(
                    memory_index=0,
                    offset_expr=_const_expr(offset),
                    data=bytes([decl.init & 0xFF]) * decl.size,
                )
                for decl, offset in ((decl, data_offsets[decl.label]) for decl in program.data)
            )

        if strategy not in ("structured", "dispatch_loop"):
            raise WasmLoweringError(f"unknown lowering strategy: {strategy!r}")
        lowerer_class: type[_FunctionLowerer] = (
            _DispatchLoopLowerer if strategy == "dispatch_loop" else _FunctionLowerer
        )

        wasi_context = _WasiContext(
            function_indices={imp.syscall_number: index for index, imp in enumerate(imports)},
            scratch_base=scratch_base,
        )
        for function in functions:
            module.code.append(
                lowerer_class(
                    function=function,
                    signatures=signatures,
                    function_indices=function_indices,
                    data_offsets=data_offsets,
                    wasi_context=wasi_context,
                ).lower()
            )
            if function.signature.export_name is not None:
                module.exports.append(
                    Export(
                        name=function.signature.export_name,
                        kind=ExternalKind.FUNCTION,
                        index=function_indices[function.label],
                    )
                )

        return module

    def _build_type_table(
        self,
        functions: list[_FunctionIR],
        imports: list[_WasiImport],
    ) -> tuple[dict[str, int], list[FuncType]]:
        type_indices: dict[FuncType, int] = {}
        function_types: list[FuncType] = []
        function_to_type_index: dict[str, int] = {}

        for imp in imports:
            if imp.func_type not in type_indices:
                type_indices[imp.func_type] = len(function_types)
                function_types.append(imp.func_type)
            function_to_type_index[imp.type_key] = type_indices[imp.func_type]

        for function in functions:
            func_type = FuncType(
                params=function.signature.wasm_param_types,
                results=function.signature.wasm_result_types,
            )
            if func_type not in type_indices:
                type_indices[func_type] = len(function_types)
                function_types.append(func_type)
            function_to_type_index[function.label] = type_indices[func_type]

        return function_to_type_index, function_types

    def _layout_data(self, decls: list[IrDataDecl]) -> dict[str, int]:
        offsets: dict[str, int] = {}
        cursor = 0
        for decl in decls:
            offsets[decl.label] = cursor
            cursor += decl.size
        return offsets

    def _needs_memory(self, program: IrProgram) -> bool:
        if program.data:
            return True
        return any(instr.opcode in _MEMORY_OPS for instr in program.instructions)

    def _needs_wasi_scratch(self, program: IrProgram) -> bool:
        for instruction in program.instructions:
            if instruction.opcode != IrOp.SYSCALL or not instruction.operands:
                continue
            syscall = _expect_immediate(instruction.operands[0], "SYSCALL number").value
            if syscall in (_SYSCALL_WRITE, _SYSCALL_READ):
                return True
        return False

    def _collect_wasi_imports(self, program: IrProgram) -> list[_WasiImport]:
        required_syscalls: set[int] = set()
        for instruction in program.instructions:
            if instruction.opcode != IrOp.SYSCALL or not instruction.operands:
                continue
            required_syscalls.add(_expect_immediate(instruction.operands[0], "SYSCALL number").value)

        ordered_imports = (
            _WasiImport(
                syscall_number=_SYSCALL_WRITE,
                name="fd_write",
                func_type=FuncType(
                    params=(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                    results=(ValueType.I32,),
                ),
            ),
            _WasiImport(
                syscall_number=_SYSCALL_READ,
                name="fd_read",
                func_type=FuncType(
                    params=(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                    results=(ValueType.I32,),
                ),
            ),
            _WasiImport(
                syscall_number=_SYSCALL_EXIT,
                name="proc_exit",
                func_type=FuncType(
                    params=(ValueType.I32,),
                    results=(),
                ),
            ),
        )

        supported_syscalls = {imp.syscall_number for imp in ordered_imports}
        unsupported = sorted(required_syscalls - supported_syscalls)
        if unsupported:
            raise WasmLoweringError(f"unsupported SYSCALL number(s): {', '.join(map(str, unsupported))}")

        return [imp for imp in ordered_imports if imp.syscall_number in required_syscalls]

    def _split_functions(
        self,
        program: IrProgram,
        signatures: dict[str, FunctionSignature],
    ) -> list[_FunctionIR]:
        functions: list[_FunctionIR] = []
        start_index: int | None = None
        start_label: str | None = None

        for index, instruction in enumerate(program.instructions):
            label_name = _function_label_name(instruction)
            if label_name is None:
                continue

            if start_label is not None and start_index is not None:
                functions.append(
                    _make_function_ir(
                        label=start_label,
                        instructions=program.instructions[start_index:index],
                        signatures=signatures,
                    )
                )

            start_label = label_name
            start_index = index

        if start_label is not None and start_index is not None:
            functions.append(
                _make_function_ir(
                    label=start_label,
                    instructions=program.instructions[start_index:],
                    signatures=signatures,
                )
            )

        return functions


class _FunctionLowerer:
    def __init__(
        self,
        *,
        function: _FunctionIR,
        signatures: dict[str, FunctionSignature],
        function_indices: dict[str, int],
        data_offsets: dict[str, int],
        wasi_context: _WasiContext,
    ) -> None:
        self.function = function
        self.signatures = signatures
        self.function_indices = function_indices
        self.data_offsets = data_offsets
        self.wasi_context = wasi_context
        self.param_count = function.signature.param_count
        self._register_types = function.register_types
        self._bytes = bytearray()
        self._instructions = function.instructions
        self._label_to_index = {
            label.name: index
            for index, instruction in enumerate(self._instructions)
            if instruction.opcode == IrOp.LABEL
            for label in instruction.operands
            if isinstance(label, IrLabel)
        }

    def lower(self) -> FunctionBody:
        self._copy_params_into_ir_registers()
        self._emit_region(1, len(self._instructions))
        self._emit_opcode("end")

        return FunctionBody(
            locals=self.function.register_types,
            code=bytes(self._bytes),
        )

    def _copy_params_into_ir_registers(self) -> None:
        for param_index in range(self.param_count):
            self._emit_opcode("local.get")
            self._emit_u32(param_index)
            self._emit_opcode("local.set")
            self._emit_u32(self._local_index(_REG_VAR_BASE + param_index))

    def _emit_region(self, start: int, end: int) -> None:
        index = start
        while index < end:
            instruction = self._instructions[index]

            if instruction.opcode == IrOp.COMMENT:
                index += 1
                continue

            label_name = _label_name(instruction)
            if label_name is not None and _LOOP_START_RE.match(label_name):
                index = self._emit_loop(index)
                continue

            if (
                instruction.opcode in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ)
                and len(instruction.operands) == 2
                and isinstance(instruction.operands[1], IrLabel)
                and _IF_ELSE_RE.match(instruction.operands[1].name)
            ):
                index = self._emit_if(index)
                continue

            if instruction.opcode == IrOp.LABEL:
                index += 1
                continue

            if instruction.opcode in (IrOp.JUMP, IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
                msg = f"unexpected unstructured control flow in {self.function.label}"
                raise WasmLoweringError(msg)

            self._emit_simple(instruction)
            index += 1

    def _emit_if(self, branch_index: int) -> int:
        branch = self._instructions[branch_index]
        cond_reg = _expect_register(branch.operands[0], "if condition")
        else_label = _expect_label(branch.operands[1], "if else label").name
        end_label = else_label.removesuffix("_else") + "_end"

        else_index = self._require_label_index(else_label)
        end_index = self._require_label_index(end_label)
        jump_index = self._find_last_jump_to_label(branch_index + 1, else_index, end_label)

        self._emit_local_get(cond_reg.index)
        if branch.opcode == IrOp.BRANCH_NZ:
            self._emit_opcode("i32.eqz")
        self._emit_opcode("if")
        self._bytes.append(int(BlockType.EMPTY))

        self._emit_region(branch_index + 1, jump_index)

        if else_index + 1 < end_index:
            self._emit_opcode("else")
            self._emit_region(else_index + 1, end_index)

        self._emit_opcode("end")
        return end_index + 1

    def _emit_loop(self, label_index: int) -> int:
        start_label = _label_name(self._instructions[label_index])
        if start_label is None:
            raise WasmLoweringError("loop lowering expected a start label")
        end_label = start_label.removesuffix("_start") + "_end"

        end_index = self._require_label_index(end_label)
        branch_index = self._find_first_branch_to_label(label_index + 1, end_index, end_label)
        backedge_index = self._find_last_jump_to_label(branch_index + 1, end_index, start_label)

        branch = self._instructions[branch_index]
        cond_reg = _expect_register(branch.operands[0], "loop condition")

        self._emit_opcode("block")
        self._bytes.append(int(BlockType.EMPTY))
        self._emit_opcode("loop")
        self._bytes.append(int(BlockType.EMPTY))

        self._emit_region(label_index + 1, branch_index)

        self._emit_local_get(cond_reg.index)
        if branch.opcode == IrOp.BRANCH_Z:
            self._emit_opcode("i32.eqz")
        self._emit_opcode("br_if")
        self._emit_u32(1)

        self._emit_region(branch_index + 1, backedge_index)
        self._emit_opcode("br")
        self._emit_u32(0)

        self._emit_opcode("end")
        self._emit_opcode("end")
        return end_index + 1

    def _emit_simple(self, instruction: IrInstruction) -> None:
        match instruction.opcode:
            case IrOp.LOAD_IMM:
                dst = _expect_register(instruction.operands[0], "LOAD_IMM dst")
                imm = _expect_immediate(instruction.operands[1], "LOAD_IMM imm")
                self._emit_i32_const(imm.value)
                self._emit_local_set(dst.index)
            case IrOp.LOAD_F64_IMM:
                dst = _expect_register(instruction.operands[0], "LOAD_F64_IMM dst")
                imm = _expect_float_immediate(
                    instruction.operands[1], "LOAD_F64_IMM imm"
                )
                self._emit_f64_const(imm.value)
                self._emit_local_set(dst.index)
            case IrOp.LOAD_ADDR:
                dst = _expect_register(instruction.operands[0], "LOAD_ADDR dst")
                label = _expect_label(instruction.operands[1], "LOAD_ADDR label")
                if label.name not in self.data_offsets:
                    raise WasmLoweringError(f"unknown data label: {label.name}")
                self._emit_i32_const(self.data_offsets[label.name])
                self._emit_local_set(dst.index)
            case IrOp.LOAD_BYTE:
                dst = _expect_register(instruction.operands[0], "LOAD_BYTE dst")
                base = _expect_register(instruction.operands[1], "LOAD_BYTE base")
                offset = _expect_register(instruction.operands[2], "LOAD_BYTE offset")
                self._emit_address(base.index, offset.index)
                self._emit_opcode("i32.load8_u")
                self._emit_memarg(0, 0)
                self._emit_local_set(dst.index)
            case IrOp.STORE_BYTE:
                src = _expect_register(instruction.operands[0], "STORE_BYTE src")
                base = _expect_register(instruction.operands[1], "STORE_BYTE base")
                offset = _expect_register(instruction.operands[2], "STORE_BYTE offset")
                self._emit_address(base.index, offset.index)
                self._emit_local_get(src.index)
                self._emit_opcode("i32.store8")
                self._emit_memarg(0, 0)
            case IrOp.LOAD_WORD:
                dst = _expect_register(instruction.operands[0], "LOAD_WORD dst")
                base = _expect_register(instruction.operands[1], "LOAD_WORD base")
                offset = _expect_register(instruction.operands[2], "LOAD_WORD offset")
                self._emit_address(base.index, offset.index)
                self._emit_opcode("i32.load")
                self._emit_memarg(2, 0)
                self._emit_local_set(dst.index)
            case IrOp.STORE_WORD:
                src = _expect_register(instruction.operands[0], "STORE_WORD src")
                base = _expect_register(instruction.operands[1], "STORE_WORD base")
                offset = _expect_register(instruction.operands[2], "STORE_WORD offset")
                self._emit_address(base.index, offset.index)
                self._emit_local_get(src.index)
                self._emit_opcode("i32.store")
                self._emit_memarg(2, 0)
            case IrOp.LOAD_F64:
                dst = _expect_register(instruction.operands[0], "LOAD_F64 dst")
                base = _expect_register(instruction.operands[1], "LOAD_F64 base")
                offset = _expect_register(instruction.operands[2], "LOAD_F64 offset")
                self._emit_address(base.index, offset.index)
                self._emit_opcode("f64.load")
                self._emit_memarg(3, 0)
                self._emit_local_set(dst.index)
            case IrOp.STORE_F64:
                src = _expect_register(instruction.operands[0], "STORE_F64 src")
                base = _expect_register(instruction.operands[1], "STORE_F64 base")
                offset = _expect_register(instruction.operands[2], "STORE_F64 offset")
                self._emit_address(base.index, offset.index)
                self._emit_local_get(src.index)
                self._emit_opcode("f64.store")
                self._emit_memarg(3, 0)
            case IrOp.ADD:
                self._emit_binary_numeric("i32.add", instruction)
            case IrOp.ADD_IMM:
                dst = _expect_register(instruction.operands[0], "ADD_IMM dst")
                src = _expect_register(instruction.operands[1], "ADD_IMM src")
                imm = _expect_immediate(instruction.operands[2], "ADD_IMM imm")
                self._emit_local_get(src.index)
                self._emit_i32_const(imm.value)
                self._emit_opcode("i32.add")
                self._emit_local_set(dst.index)
            case IrOp.SUB:
                self._emit_binary_numeric("i32.sub", instruction)
            case IrOp.AND:
                self._emit_binary_numeric("i32.and", instruction)
            case IrOp.MUL:
                self._emit_binary_numeric("i32.mul", instruction)
            case IrOp.DIV:
                self._emit_binary_numeric("i32.div_s", instruction)
            case IrOp.AND_IMM:
                dst = _expect_register(instruction.operands[0], "AND_IMM dst")
                src = _expect_register(instruction.operands[1], "AND_IMM src")
                imm = _expect_immediate(instruction.operands[2], "AND_IMM imm")
                self._emit_local_get(src.index)
                self._emit_i32_const(imm.value)
                self._emit_opcode("i32.and")
                self._emit_local_set(dst.index)
            case IrOp.OR:
                self._emit_binary_numeric("i32.or", instruction)
            case IrOp.OR_IMM:
                # OR_IMM dst, src, imm  →  dst = src | imm
                dst = _expect_register(instruction.operands[0], "OR_IMM dst")
                src = _expect_register(instruction.operands[1], "OR_IMM src")
                imm = _expect_immediate(instruction.operands[2], "OR_IMM imm")
                self._emit_local_get(src.index)
                self._emit_i32_const(imm.value)
                self._emit_opcode("i32.or")
                self._emit_local_set(dst.index)
            case IrOp.XOR:
                self._emit_binary_numeric("i32.xor", instruction)
            case IrOp.XOR_IMM:
                # XOR_IMM dst, src, imm  →  dst = src ^ imm
                dst = _expect_register(instruction.operands[0], "XOR_IMM dst")
                src = _expect_register(instruction.operands[1], "XOR_IMM src")
                imm = _expect_immediate(instruction.operands[2], "XOR_IMM imm")
                self._emit_local_get(src.index)
                self._emit_i32_const(imm.value)
                self._emit_opcode("i32.xor")
                self._emit_local_set(dst.index)
            case IrOp.NOT:
                # WASM has no bitwise-NOT opcode.  XOR with 0xFFFFFFFF (all ones)
                # flips every bit of the 32-bit value, which is exactly NOT for i32.
                # NOT dst, src  →  dst = src ^ 0xFFFFFFFF
                dst = _expect_register(instruction.operands[0], "NOT dst")
                src = _expect_register(instruction.operands[1], "NOT src")
                self._emit_local_get(src.index)
                self._emit_i32_const(0xFFFFFFFF)
                self._emit_opcode("i32.xor")
                self._emit_local_set(dst.index)
            case IrOp.F64_ADD:
                self._emit_binary_numeric("f64.add", instruction)
            case IrOp.F64_SUB:
                self._emit_binary_numeric("f64.sub", instruction)
            case IrOp.F64_MUL:
                self._emit_binary_numeric("f64.mul", instruction)
            case IrOp.F64_DIV:
                self._emit_binary_numeric("f64.div", instruction)
            case IrOp.F64_SQRT:
                dst = _expect_register(instruction.operands[0], "F64_SQRT dst")
                src = _expect_register(instruction.operands[1], "F64_SQRT src")
                self._emit_local_get(src.index)
                self._emit_opcode("f64.sqrt")
                self._emit_local_set(dst.index)
            case IrOp.CMP_EQ:
                self._emit_binary_numeric("i32.eq", instruction)
            case IrOp.CMP_NE:
                self._emit_binary_numeric("i32.ne", instruction)
            case IrOp.CMP_LT:
                self._emit_binary_numeric("i32.lt_s", instruction)
            case IrOp.CMP_GT:
                self._emit_binary_numeric("i32.gt_s", instruction)
            case IrOp.F64_CMP_EQ:
                self._emit_binary_numeric("f64.eq", instruction)
            case IrOp.F64_CMP_NE:
                self._emit_binary_numeric("f64.ne", instruction)
            case IrOp.F64_CMP_LT:
                self._emit_binary_numeric("f64.lt", instruction)
            case IrOp.F64_CMP_GT:
                self._emit_binary_numeric("f64.gt", instruction)
            case IrOp.F64_CMP_LE:
                self._emit_binary_numeric("f64.le", instruction)
            case IrOp.F64_CMP_GE:
                self._emit_binary_numeric("f64.ge", instruction)
            case IrOp.F64_FROM_I32:
                dst = _expect_register(instruction.operands[0], "F64_FROM_I32 dst")
                src = _expect_register(instruction.operands[1], "F64_FROM_I32 src")
                self._emit_local_get(src.index)
                self._emit_opcode("f64.convert_i32_s")
                self._emit_local_set(dst.index)
            case IrOp.I32_TRUNC_FROM_F64:
                dst = _expect_register(
                    instruction.operands[0], "I32_TRUNC_FROM_F64 dst"
                )
                src = _expect_register(
                    instruction.operands[1], "I32_TRUNC_FROM_F64 src"
                )
                self._emit_local_get(src.index)
                self._emit_opcode("i32.trunc_f64_s")
                self._emit_local_set(dst.index)
            case IrOp.CALL:
                label = _expect_label(instruction.operands[0], "CALL target")
                signature = self.signatures.get(label.name)
                if signature is None:
                    raise WasmLoweringError(f"missing function signature for {label.name}")
                if label.name not in self.function_indices:
                    raise WasmLoweringError(f"unknown function label: {label.name}")
                explicit_args = instruction.operands[1:]
                if (
                    signature.require_explicit_args
                    and len(explicit_args) != signature.param_count
                ):
                    raise WasmLoweringError(
                        f"CALL {label.name} expects {signature.param_count} "
                        f"explicit argument register(s), got {len(explicit_args)}"
                    )
                if (
                    not signature.require_explicit_args
                    and explicit_args
                    and len(explicit_args) != signature.param_count
                ):
                    raise WasmLoweringError(
                        f"CALL {label.name} expects {signature.param_count} "
                        f"argument register(s), got {len(explicit_args)}"
                    )
                if explicit_args:
                    for operand, expected_type in zip(
                        explicit_args, signature.wasm_param_types, strict=True
                    ):
                        arg = _expect_register(operand, "CALL argument")
                        actual_type = self._register_types[arg.index]
                        if actual_type != expected_type:
                            raise WasmLoweringError(
                                f"CALL {label.name} argument v{arg.index} has type "
                                f"{actual_type.name}, expected {expected_type.name}"
                            )
                        self._emit_local_get(arg.index)
                else:
                    for param_index in range(signature.param_count):
                        actual_type = self._register_types[_REG_VAR_BASE + param_index]
                        expected_type = signature.wasm_param_types[param_index]
                        if actual_type != expected_type:
                            raise WasmLoweringError(
                                f"CALL {label.name} implicit argument v{_REG_VAR_BASE + param_index} "
                                f"has type {actual_type.name}, expected {expected_type.name}"
                            )
                        self._emit_local_get(_REG_VAR_BASE + param_index)
                self._emit_opcode("call")
                self._emit_u32(self.function_indices[label.name])
                if signature.wasm_result_types:
                    result_reg = (
                        _REG_F64_SCRATCH
                        if signature.wasm_result_types[0] == ValueType.F64
                        else _REG_SCRATCH
                    )
                    self._emit_local_set(result_reg)
            case IrOp.RET | IrOp.HALT:
                result_reg = _function_result_register(self.function.signature)
                if result_reg is not None:
                    self._emit_local_get(result_reg)
                self._emit_opcode("return")
            case IrOp.NOP:
                self._emit_opcode("nop")
            case IrOp.SYSCALL:
                self._emit_syscall(instruction)
            case _:
                raise WasmLoweringError(f"unsupported opcode: {instruction.opcode.name}")

    def _emit_syscall(self, instruction: IrInstruction) -> None:
        syscall = _expect_immediate(instruction.operands[0], "SYSCALL number").value
        arg_reg = _expect_register(instruction.operands[1], "SYSCALL arg register").index

        if syscall == _SYSCALL_WRITE:
            self._emit_wasi_write(arg_reg)
            return
        if syscall == _SYSCALL_READ:
            self._emit_wasi_read(arg_reg)
            return
        if syscall == _SYSCALL_EXIT:
            self._emit_wasi_exit(arg_reg)
            return
        raise WasmLoweringError(f"unsupported SYSCALL number: {syscall}")

    def _emit_wasi_write(self, arg_reg: int) -> None:
        scratch_base = self._require_wasi_scratch()
        iovec_ptr = scratch_base + _WASI_IOVEC_OFFSET
        nwritten_ptr = scratch_base + _WASI_COUNT_OFFSET
        byte_ptr = scratch_base + _WASI_BYTE_OFFSET

        self._emit_i32_const(byte_ptr)
        self._emit_local_get(arg_reg)
        self._emit_opcode("i32.store8")
        self._emit_memarg(0, 0)

        self._emit_store_const_i32(iovec_ptr, byte_ptr)
        self._emit_store_const_i32(iovec_ptr + 4, 1)

        self._emit_i32_const(1)
        self._emit_i32_const(iovec_ptr)
        self._emit_i32_const(1)
        self._emit_i32_const(nwritten_ptr)
        self._emit_wasi_call(_SYSCALL_WRITE)
        self._emit_opcode("drop")  # discard fd_write errno; storing in _REG_SCRATCH would clobber variable A

    def _emit_wasi_read(self, arg_reg: int) -> None:
        scratch_base = self._require_wasi_scratch()
        iovec_ptr = scratch_base + _WASI_IOVEC_OFFSET
        nread_ptr = scratch_base + _WASI_COUNT_OFFSET
        byte_ptr = scratch_base + _WASI_BYTE_OFFSET

        self._emit_i32_const(byte_ptr)
        self._emit_i32_const(0)
        self._emit_opcode("i32.store8")
        self._emit_memarg(0, 0)

        self._emit_store_const_i32(iovec_ptr, byte_ptr)
        self._emit_store_const_i32(iovec_ptr + 4, 1)

        self._emit_i32_const(0)
        self._emit_i32_const(iovec_ptr)
        self._emit_i32_const(1)
        self._emit_i32_const(nread_ptr)
        self._emit_wasi_call(_SYSCALL_READ)
        self._emit_opcode("drop")  # discard fd_read errno

        self._emit_i32_const(byte_ptr)
        self._emit_opcode("i32.load8_u")
        self._emit_memarg(0, 0)
        self._emit_local_set(arg_reg)

    def _emit_wasi_exit(self, arg_reg: int) -> None:
        self._emit_local_get(arg_reg)
        self._emit_wasi_call(_SYSCALL_EXIT)
        self._emit_i32_const(0)
        self._emit_opcode("return")

    def _emit_store_const_i32(self, address: int, value: int) -> None:
        self._emit_i32_const(address)
        self._emit_i32_const(value)
        self._emit_opcode("i32.store")
        self._emit_memarg(2, 0)

    def _emit_wasi_call(self, syscall_number: int) -> None:
        function_index = self.wasi_context.function_indices.get(syscall_number)
        if function_index is None:
            raise WasmLoweringError(f"missing WASI import for SYSCALL {syscall_number}")
        self._emit_opcode("call")
        self._emit_u32(function_index)

    def _require_wasi_scratch(self) -> int:
        if self.wasi_context.scratch_base is None:
            raise WasmLoweringError("SYSCALL lowering requires WASM scratch memory")
        return self.wasi_context.scratch_base

    def _emit_binary_numeric(self, wasm_op: str, instruction: IrInstruction) -> None:
        dst = _expect_register(instruction.operands[0], f"{instruction.opcode.name} dst")
        left = _expect_register(instruction.operands[1], f"{instruction.opcode.name} lhs")
        right = _expect_register(instruction.operands[2], f"{instruction.opcode.name} rhs")
        self._emit_local_get(left.index)
        self._emit_local_get(right.index)
        self._emit_opcode(wasm_op)
        self._emit_local_set(dst.index)

    def _emit_address(self, base_index: int, offset_index: int) -> None:
        self._emit_local_get(base_index)
        self._emit_local_get(offset_index)
        self._emit_opcode("i32.add")

    def _emit_local_get(self, reg_index: int) -> None:
        self._emit_opcode("local.get")
        self._emit_u32(self._local_index(reg_index))

    def _emit_local_set(self, reg_index: int) -> None:
        self._emit_opcode("local.set")
        self._emit_u32(self._local_index(reg_index))

    def _emit_i32_const(self, value: int) -> None:
        self._emit_opcode("i32.const")
        self._bytes.extend(encode_signed(value))

    def _emit_f64_const(self, value: float) -> None:
        self._emit_opcode("f64.const")
        self._bytes.extend(struct.pack("<d", value))

    def _emit_memarg(self, align: int, offset: int) -> None:
        self._emit_u32(align)
        self._emit_u32(offset)

    def _emit_opcode(self, name: str) -> None:
        self._bytes.append(_OPCODE[name])

    def _emit_u32(self, value: int) -> None:
        self._bytes.extend(encode_unsigned(value))

    def _local_index(self, reg_index: int) -> int:
        return self.param_count + reg_index

    def _require_label_index(self, label: str) -> int:
        if label not in self._label_to_index:
            raise WasmLoweringError(f"missing label {label} in {self.function.label}")
        return self._label_to_index[label]

    def _find_first_branch_to_label(self, start: int, end: int, label: str) -> int:
        for index in range(start, end):
            instruction = self._instructions[index]
            if instruction.opcode not in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
                continue
            target = _label_name_from_operand(instruction.operands[1])
            if target == label:
                return index
        raise WasmLoweringError(f"expected branch to {label} in {self.function.label}")

    def _find_last_jump_to_label(self, start: int, end: int, label: str) -> int:
        for index in range(end - 1, start - 1, -1):
            instruction = self._instructions[index]
            if instruction.opcode != IrOp.JUMP:
                continue
            target = _label_name_from_operand(instruction.operands[0])
            if target == label:
                return index
        raise WasmLoweringError(f"expected jump to {label} in {self.function.label}")


class _DispatchLoopLowerer(_FunctionLowerer):
    """Lower a function with unstructured control flow using a virtual program counter.

    The resulting WASM looks like::

        [copy params into IR registers]
        i32.const 0 ; local.set $pc          ← start at segment 0
        block $prog_end (void)
          loop $dispatch (void)
            block $seg_0 (void)              ← one block per label
              local.get $pc; i32.const 0; i32.ne; br_if 0   ← skip if wrong
              [instructions for segment 0]
              i32.const 1; local.set $pc; br 1              ← fall-through
            end
            block $seg_1 (void)
              local.get $pc; i32.const 1; i32.ne; br_if 0
              [instructions for segment 1]
              ...
            end
          end loop                           ← falls through on out-of-range $pc
        end block
        local.get function result register  ← function return value
        end function

    br depth table (from inside block $seg_N):
        br 0 → block $seg_N   (skip this segment)
        br 1 → loop $dispatch (continue dispatch — loop restart)
        br 2 → block $prog_end (exit program)

    Inside a generated ``if`` block (for BRANCH_Z / BRANCH_NZ):
        br 0 → if block
        br 1 → block $seg_N
        br 2 → loop $dispatch
        br 3 → block $prog_end
    """

    def lower(self) -> FunctionBody:
        label_to_seg, segments = self._index_segments()
        # $pc lives at the local slot just past all IR virtual registers
        pc_reg = self.function.max_reg + 1

        self._copy_params_into_ir_registers()

        # Initialise $pc = 0 (start at segment 0)
        self._emit_i32_const(0)
        self._emit_local_set(pc_reg)

        # block $prog_end (void)
        self._emit_opcode("block")
        self._bytes.append(int(BlockType.EMPTY))
        # loop $dispatch (void)
        self._emit_opcode("loop")
        self._bytes.append(int(BlockType.EMPTY))

        for seg_idx, instrs in enumerate(segments):
            self._emit_segment(seg_idx, instrs, pc_reg, label_to_seg)

        # end loop
        self._emit_opcode("end")
        # end block $prog_end
        self._emit_opcode("end")

        # Function return value (HALT via br 2 lands here)
        result_reg = _function_result_register(self.function.signature)
        if result_reg is not None:
            self._emit_local_get(result_reg)
        self._emit_opcode("end")

        return FunctionBody(
            # max_reg + 1 IR virtual-register slots, plus one for $pc
            locals=self.function.register_types + (ValueType.I32,),
            code=bytes(self._bytes),
        )

    def _index_segments(
        self,
    ) -> tuple[dict[str, int], list[list[IrInstruction]]]:
        """Assign each LABEL a segment index and split instructions between labels."""
        label_to_seg: dict[str, int] = {}
        # segment_starts[i] = instruction index *after* the i-th LABEL
        segment_starts: list[int] = []

        for i, instr in enumerate(self._instructions):
            if (
                instr.opcode == IrOp.LABEL
                and instr.operands
                and isinstance(instr.operands[0], IrLabel)
            ):
                label_to_seg[instr.operands[0].name] = len(segment_starts)
                segment_starts.append(i + 1)

        segments: list[list[IrInstruction]] = []
        for i, start in enumerate(segment_starts):
            # The next LABEL is at segment_starts[i+1]-1; exclude it from this segment.
            end = segment_starts[i + 1] - 1 if i + 1 < len(segment_starts) else len(self._instructions)
            segments.append(list(self._instructions[start:end]))

        return label_to_seg, segments

    def _emit_segment(
        self,
        seg_idx: int,
        instrs: list[IrInstruction],
        pc_reg: int,
        label_to_seg: dict[str, int],
    ) -> None:
        # block $seg_N (void)
        self._emit_opcode("block")
        self._bytes.append(int(BlockType.EMPTY))

        # Skip this segment when $pc ≠ seg_idx
        self._emit_local_get(pc_reg)
        self._emit_i32_const(seg_idx)
        self._emit_opcode("i32.ne")
        self._emit_opcode("br_if")
        self._emit_u32(0)  # br 0 → end of block $seg_N

        terminated = False
        for instr in instrs:
            if terminated:
                break

            if instr.opcode == IrOp.COMMENT:
                continue

            if instr.opcode == IrOp.JUMP:
                target = _expect_label(instr.operands[0], "JUMP target").name
                target_idx = label_to_seg.get(target)
                if target_idx is None:
                    raise WasmLoweringError(f"JUMP to unknown label: {target!r}")
                self._emit_i32_const(target_idx)
                self._emit_local_set(pc_reg)
                self._emit_opcode("br")
                self._emit_u32(1)  # br 1 → loop $dispatch (restart)
                terminated = True

            elif instr.opcode in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
                # Conditional jump: set $pc and continue dispatch *if* condition fires.
                # Does NOT terminate the segment — falls through to next instruction.
                cond_reg = _expect_register(instr.operands[0], "BRANCH condition").index
                target = _expect_label(instr.operands[1], "BRANCH target").name
                target_idx = label_to_seg.get(target)
                if target_idx is None:
                    raise WasmLoweringError(f"BRANCH to unknown label: {target!r}")
                self._emit_local_get(cond_reg)
                if instr.opcode == IrOp.BRANCH_Z:
                    # Branch when zero: invert so WASM if fires when reg == 0
                    self._emit_opcode("i32.eqz")
                self._emit_opcode("if")
                self._bytes.append(int(BlockType.EMPTY))
                self._emit_i32_const(target_idx)
                self._emit_local_set(pc_reg)
                self._emit_opcode("br")
                self._emit_u32(2)  # br 2 (inside if) → loop $dispatch
                self._emit_opcode("end")
                # Intentionally NOT setting terminated — execution continues

            elif instr.opcode in (IrOp.HALT, IrOp.RET):
                self._emit_opcode("br")
                self._emit_u32(2)  # br 2 → block $prog_end (exit program)
                terminated = True

            elif instr.opcode == IrOp.SYSCALL:
                self._emit_syscall(instr)

            else:
                self._emit_simple(instr)

        if not terminated:
            # Fall-through: advance $pc to the next segment in source order
            self._emit_i32_const(seg_idx + 1)
            self._emit_local_set(pc_reg)
            self._emit_opcode("br")
            self._emit_u32(1)  # br 1 → loop $dispatch (restart)

        # end block $seg_N
        self._emit_opcode("end")


def infer_function_signatures_from_comments(program: IrProgram) -> dict[str, FunctionSignature]:
    """Infer Nib-style function signatures from debug COMMENT instructions."""

    signatures: dict[str, FunctionSignature] = {}
    pending_comment: str | None = None

    for instruction in program.instructions:
        if instruction.opcode == IrOp.COMMENT:
            pending_comment = _label_name_from_operand(instruction.operands[0])
            continue

        label_name = _function_label_name(instruction)
        if label_name is not None:
            if label_name == "_start":
                signatures[label_name] = FunctionSignature(
                    label=label_name,
                    param_count=0,
                    export_name="_start",
                )
            elif label_name.startswith("_fn_") and pending_comment is not None:
                export_name = label_name.removeprefix("_fn_")
                match = _FUNCTION_COMMENT_RE.match(pending_comment)
                if match and match.group(1) == export_name:
                    params_blob = match.group(2).strip()
                    param_count = 0 if not params_blob else len(
                        [piece for piece in params_blob.split(",") if piece.strip()]
                    )
                    signatures[label_name] = FunctionSignature(
                        label=label_name,
                        param_count=param_count,
                        export_name=export_name,
                    )
            pending_comment = None
            continue

        if instruction.opcode != IrOp.COMMENT:
            pending_comment = None

    return signatures


def _make_function_ir(
    *,
    label: str,
    instructions: list[IrInstruction],
    signatures: dict[str, FunctionSignature],
) -> _FunctionIR:
    if label == "_start":
        signature = signatures.get(label, FunctionSignature(label=label, param_count=0, export_name="_start"))
    else:
        signature = signatures.get(label)
        if signature is None:
            raise WasmLoweringError(f"missing function signature for {label}")

    max_reg = max(
        [
            1,
            _REG_VAR_BASE + max(signature.param_count - 1, 0),
            _function_result_register(signature) or 1,
            _REG_F64_SCRATCH if _needs_f64_scratch(instructions, signatures) else 1,
        ]
        + [
            operand.index
            for instruction in instructions
            for operand in instruction.operands
            if isinstance(operand, IrRegister)
        ]
    )

    return _FunctionIR(
        label=label,
        instructions=instructions,
        signature=signature,
        max_reg=max_reg,
        register_types=_infer_register_types(
            instructions=instructions,
            signature=signature,
            max_reg=max_reg,
            signatures=signatures,
        ),
    )


def _needs_f64_scratch(
    instructions: list[IrInstruction],
    signatures: dict[str, FunctionSignature],
) -> bool:
    for instruction in instructions:
        if instruction.opcode != IrOp.CALL:
            continue
        target = instruction.operands[0]
        if not isinstance(target, IrLabel):
            continue
        callee = signatures.get(target.name)
        if callee is not None and callee.wasm_result_types == (ValueType.F64,):
            return True
    return False


def _const_expr(value: int) -> bytes:
    return bytes([_OPCODE["i32.const"]]) + encode_signed(value) + bytes([_OPCODE["end"]])


def _function_label_name(instruction: IrInstruction) -> str | None:
    label_name = _label_name(instruction)
    if label_name == "_start" or (label_name is not None and label_name.startswith("_fn_")):
        return label_name
    return None


def _label_name(instruction: IrInstruction) -> str | None:
    if instruction.opcode != IrOp.LABEL or not instruction.operands:
        return None
    if not isinstance(instruction.operands[0], IrLabel):
        return None
    return instruction.operands[0].name


def _label_name_from_operand(operand: object) -> str:
    if not isinstance(operand, IrLabel):
        raise WasmLoweringError(f"expected label operand, got {operand!r}")
    return operand.name


def _expect_register(operand: object, context: str) -> IrRegister:
    if not isinstance(operand, IrRegister):
        raise WasmLoweringError(f"{context}: expected register, got {operand!r}")
    return operand


def _expect_immediate(operand: object, context: str) -> IrImmediate:
    if not isinstance(operand, IrImmediate):
        raise WasmLoweringError(f"{context}: expected immediate, got {operand!r}")
    return operand


def _expect_float_immediate(operand: object, context: str) -> IrFloatImmediate:
    if not isinstance(operand, IrFloatImmediate):
        raise WasmLoweringError(f"{context}: expected float immediate, got {operand!r}")
    return operand


def _expect_label(operand: object, context: str) -> IrLabel:
    if not isinstance(operand, IrLabel):
        raise WasmLoweringError(f"{context}: expected label, got {operand!r}")
    return operand


def _align_up(value: int, alignment: int) -> int:
    return ((value + alignment - 1) // alignment) * alignment


_REG_SCRATCH = 1
_REG_F64_SCRATCH = 31
_REG_VAR_BASE = 2


def _function_result_register(signature: FunctionSignature) -> int | None:
    if not signature.wasm_result_types:
        return None
    return (
        _REG_F64_SCRATCH
        if signature.wasm_result_types[0] == ValueType.F64
        else _REG_SCRATCH
    )


def _infer_register_types(
    *,
    instructions: list[IrInstruction],
    signature: FunctionSignature,
    max_reg: int,
    signatures: dict[str, FunctionSignature],
) -> tuple[ValueType, ...]:
    reg_types: list[ValueType | None] = [None] * (max_reg + 1)

    for param_index, param_type in enumerate(signature.wasm_param_types):
        _assign_register_type(
            reg_types,
            _REG_VAR_BASE + param_index,
            param_type,
            f"{signature.label} param {param_index}",
        )

    result_reg = _function_result_register(signature)
    if result_reg is not None:
        _assign_register_type(
            reg_types,
            result_reg,
            signature.wasm_result_types[0],
            f"{signature.label} result",
        )

    for instruction in instructions:
        match instruction.opcode:
            case IrOp.LOAD_F64_IMM | IrOp.LOAD_F64:
                dst = _expect_register(instruction.operands[0], f"{instruction.opcode.name} dst")
                _assign_register_type(
                    reg_types, dst.index, ValueType.F64, instruction.opcode.name
                )
            case (
                IrOp.LOAD_IMM
                | IrOp.LOAD_ADDR
                | IrOp.LOAD_BYTE
                | IrOp.LOAD_WORD
                | IrOp.ADD
                | IrOp.ADD_IMM
                | IrOp.SUB
                | IrOp.AND
                | IrOp.AND_IMM
                | IrOp.OR
                | IrOp.OR_IMM
                | IrOp.XOR
                | IrOp.XOR_IMM
                | IrOp.NOT
                | IrOp.MUL
                | IrOp.DIV
                | IrOp.CMP_EQ
                | IrOp.CMP_NE
                | IrOp.CMP_LT
                | IrOp.CMP_GT
                | IrOp.F64_CMP_EQ
                | IrOp.F64_CMP_NE
                | IrOp.F64_CMP_LT
                | IrOp.F64_CMP_GT
                | IrOp.F64_CMP_LE
                | IrOp.F64_CMP_GE
                | IrOp.I32_TRUNC_FROM_F64
            ):
                dst = _expect_register(instruction.operands[0], f"{instruction.opcode.name} dst")
                _assign_register_type(
                    reg_types, dst.index, ValueType.I32, instruction.opcode.name
                )
            case (
                IrOp.F64_ADD
                | IrOp.F64_SUB
                | IrOp.F64_MUL
                | IrOp.F64_DIV
                | IrOp.F64_SQRT
                | IrOp.F64_FROM_I32
            ):
                dst = _expect_register(instruction.operands[0], f"{instruction.opcode.name} dst")
                _assign_register_type(
                    reg_types, dst.index, ValueType.F64, instruction.opcode.name
                )
            case IrOp.CALL:
                target = _expect_label(instruction.operands[0], "CALL target")
                callee = signatures.get(target.name)
                if callee is None:
                    raise WasmLoweringError(f"missing function signature for {target.name}")
                if callee.wasm_result_types:
                    result_reg = (
                        _REG_F64_SCRATCH
                        if callee.wasm_result_types[0] == ValueType.F64
                        else _REG_SCRATCH
                    )
                    _assign_register_type(
                        reg_types,
                        result_reg,
                        callee.wasm_result_types[0],
                        f"CALL {target.name}",
                    )
            case _:
                continue

    return tuple(
        reg_type if reg_type is not None else ValueType.I32 for reg_type in reg_types
    )


def _assign_register_type(
    reg_types: list[ValueType | None],
    reg_index: int,
    value_type: ValueType,
    context: str,
) -> None:
    existing = reg_types[reg_index]
    if existing is not None and existing != value_type:
        raise WasmLoweringError(
            f"{context}: register v{reg_index} has conflicting WASM types "
            f"{existing.name} and {value_type.name}"
        )
    reg_types[reg_index] = value_type
