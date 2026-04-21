"""Lower compiler IR into CIL method-body bytecode.

This package deliberately emits method-body artifacts instead of full PE/CLI
assemblies. CIL call instructions require metadata tokens, so the lowerer takes
an injectable token provider. The default provider assigns deterministic
placeholder tokens that are useful for tests and for composing later stages.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from typing import Protocol

from cil_bytecode_builder import CILBranchKind, CILBytecodeBuilder, CILOpcode
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister

_MAX_STATIC_DATA_BYTES = 16 * 1024 * 1024
_INT32_MIN = -(2**31)
_INT32_MAX = 2**31 - 1


class CILBackendError(ValueError):
    """Raised when a compiler IR program cannot be lowered to CIL."""


class CILHelper(StrEnum):
    """Runtime helper calls required by lowered CIL bytecode."""

    MEM_LOAD_BYTE = "mem_load_byte"
    MEM_STORE_BYTE = "mem_store_byte"
    LOAD_WORD = "load_word"
    STORE_WORD = "store_word"
    SYSCALL = "syscall"


@dataclass(frozen=True)
class CILHelperSpec:
    """A helper method dependency requested by the lowered bytecode."""

    helper: CILHelper
    name: str
    parameter_types: tuple[str, ...]
    return_type: str


@dataclass(frozen=True)
class CILBackendConfig:
    """Configuration for compiler IR to CIL bytecode lowering."""

    syscall_arg_reg: int = 4
    max_static_data_bytes: int = _MAX_STATIC_DATA_BYTES
    method_max_stack: int = 16
    call_register_count: int | None = 0


@dataclass(frozen=True)
class CILMethodArtifact:
    """A lowered CIL method body and its method-level metadata needs."""

    name: str
    body: bytes
    max_stack: int
    local_types: tuple[str, ...]
    return_type: str = "int32"
    parameter_types: tuple[str, ...] = ()

    @property
    def local_count(self) -> int:
        """Return the number of local variable slots used by this method."""
        return len(self.local_types)


@dataclass(frozen=True)
class CILProgramArtifact:
    """The result of lowering a compiler IR program to CIL method bodies."""

    entry_label: str
    methods: tuple[CILMethodArtifact, ...]
    data_offsets: dict[str, int]
    data_size: int
    helper_specs: tuple[CILHelperSpec, ...]
    token_provider: CILTokenProvider

    @property
    def callable_labels(self) -> tuple[str, ...]:
        """Return method names in emitted order."""
        return tuple(method.name for method in self.methods)

    @property
    def entry_method(self) -> CILMethodArtifact:
        """Return the artifact for the configured entry method."""
        for method in self.methods:
            if method.name == self.entry_label:
                return method
        msg = f"Entry method not found in artifact: {self.entry_label}"
        raise CILBackendError(msg)


class CILTokenProvider(Protocol):
    """Resolve metadata tokens needed by call instructions."""

    def method_token(self, method_name: str) -> int:
        """Return the MethodDef token for an emitted IR callable method."""

    def helper_token(self, helper: CILHelper) -> int:
        """Return the MemberRef or MethodDef token for a runtime helper."""


class SequentialCILTokenProvider:
    """Deterministic token provider for standalone bytecode lowering.

    Method tokens start at ``0x06000001`` in emitted callable order. Helper
    tokens start at ``0x0A000001`` in ``CILHelper`` enum order.
    """

    def __init__(self, method_names: tuple[str, ...]) -> None:
        self._method_tokens = {
            name: 0x06000001 + index for index, name in enumerate(method_names)
        }
        self._helper_tokens = {
            helper: 0x0A000001 + index for index, helper in enumerate(CILHelper)
        }

    def method_token(self, method_name: str) -> int:
        """Return the deterministic MethodDef token for ``method_name``."""
        try:
            return self._method_tokens[method_name]
        except KeyError as exc:
            msg = f"Unknown CIL method token target: {method_name}"
            raise CILBackendError(msg) from exc

    def helper_token(self, helper: CILHelper) -> int:
        """Return the deterministic helper token for ``helper``."""
        try:
            return self._helper_tokens[helper]
        except KeyError as exc:
            msg = f"Unknown CIL helper token target: {helper}"
            raise CILBackendError(msg) from exc


@dataclass(frozen=True)
class _CallableRegion:
    name: str
    start_index: int
    end_index: int
    instructions: tuple[IrInstruction, ...]


@dataclass(frozen=True)
class CILLoweringPlan:
    """Validated lowering plan shared by composable pipeline stages."""

    regions: tuple[_CallableRegion, ...]
    data_offsets: dict[str, int]
    data_size: int
    local_count: int
    token_provider: CILTokenProvider


AnalyzeProgramStage = Callable[[IrProgram, CILBackendConfig], CILLoweringPlan]
LowerRegionStage = Callable[
    [IrProgram, CILBackendConfig, CILLoweringPlan, _CallableRegion],
    CILMethodArtifact,
]


class CILLoweringPipeline:
    """Composable compiler IR to CIL bytecode lowering pipeline."""

    def __init__(
        self,
        *,
        analyze_program: AnalyzeProgramStage | None = None,
        lower_region: LowerRegionStage | None = None,
    ) -> None:
        self._analyze_program = analyze_program or _analyze_program
        self._lower_region = lower_region or _lower_region

    def lower(
        self,
        program: IrProgram,
        config: CILBackendConfig | None = None,
    ) -> CILProgramArtifact:
        """Lower ``program`` to CIL method artifacts."""
        resolved_config = config or CILBackendConfig()
        plan = self._analyze_program(program, resolved_config)
        methods = tuple(
            self._lower_region(program, resolved_config, plan, region)
            for region in plan.regions
        )
        return CILProgramArtifact(
            entry_label=program.entry_label,
            methods=methods,
            data_offsets=dict(plan.data_offsets),
            data_size=plan.data_size,
            helper_specs=HELPER_SPECS,
            token_provider=plan.token_provider,
        )


def lower_ir_to_cil_bytecode(
    program: IrProgram,
    config: CILBackendConfig | None = None,
    *,
    token_provider: CILTokenProvider | None = None,
) -> CILProgramArtifact:
    """Lower a compiler IR program to CIL bytecode method artifacts."""
    pipeline = CILLoweringPipeline(
        analyze_program=lambda source, resolved_config: _analyze_program(
            source,
            resolved_config,
            token_provider=token_provider,
        )
    )
    return pipeline.lower(program, config)


HELPER_SPECS: tuple[CILHelperSpec, ...] = (
    CILHelperSpec(CILHelper.MEM_LOAD_BYTE, "__ca_mem_load_byte", ("int32",), "int32"),
    CILHelperSpec(
        CILHelper.MEM_STORE_BYTE,
        "__ca_mem_store_byte",
        ("int32", "int32"),
        "void",
    ),
    CILHelperSpec(CILHelper.LOAD_WORD, "__ca_load_word", ("int32",), "int32"),
    CILHelperSpec(
        CILHelper.STORE_WORD,
        "__ca_store_word",
        ("int32", "int32"),
        "void",
    ),
    CILHelperSpec(CILHelper.SYSCALL, "__ca_syscall", ("int32", "int32"), "int32"),
)


def _analyze_program(
    program: IrProgram,
    config: CILBackendConfig,
    *,
    token_provider: CILTokenProvider | None = None,
) -> CILLoweringPlan:
    _validate_config(config)
    label_positions = _collect_labels(program)
    regions = tuple(_discover_callable_regions(program, label_positions))
    data_offsets, data_size = _assign_data_offsets(program, config)
    local_count = max(_max_register_index(program) + 1, config.syscall_arg_reg + 1, 2)
    provider = token_provider or SequentialCILTokenProvider(
        tuple(region.name for region in regions)
    )
    return CILLoweringPlan(
        regions=regions,
        data_offsets=data_offsets,
        data_size=data_size,
        local_count=local_count,
        token_provider=provider,
    )


def _validate_config(config: CILBackendConfig) -> None:
    if config.syscall_arg_reg < 0:
        msg = "syscall_arg_reg must be non-negative"
        raise CILBackendError(msg)
    if config.max_static_data_bytes < 0:
        msg = "max_static_data_bytes must be non-negative"
        raise CILBackendError(msg)
    if config.method_max_stack <= 0:
        msg = "method_max_stack must be positive"
        raise CILBackendError(msg)
    if config.call_register_count is not None and config.call_register_count < 0:
        msg = "call_register_count must be non-negative or None"
        raise CILBackendError(msg)


def _collect_labels(program: IrProgram) -> dict[str, int]:
    positions: dict[str, int] = {}
    for index, instruction in enumerate(program.instructions):
        if instruction.opcode != IrOp.LABEL:
            continue
        label = _as_label(instruction.operands[0], "LABEL operand")
        if label.name in positions:
            msg = f"Duplicate IR label: {label.name}"
            raise CILBackendError(msg)
        positions[label.name] = index
    return positions


def _discover_callable_regions(
    program: IrProgram,
    label_positions: dict[str, int],
) -> list[_CallableRegion]:
    callable_names = {program.entry_label}
    for instruction in program.instructions:
        if instruction.opcode == IrOp.CALL:
            target = _as_label(instruction.operands[0], "CALL target")
            callable_names.add(target.name)

    if program.entry_label not in label_positions:
        msg = f"Entry label not found: {program.entry_label}"
        raise CILBackendError(msg)
    missing = sorted(callable_names - set(label_positions))
    if missing:
        msg = f"Missing callable labels: {missing}"
        raise CILBackendError(msg)

    ordered_names = sorted(callable_names, key=lambda name: label_positions[name])
    regions: list[_CallableRegion] = []
    for index, name in enumerate(ordered_names):
        start = label_positions[name]
        end = (
            label_positions[ordered_names[index + 1]]
            if index + 1 < len(ordered_names)
            else len(program.instructions)
        )
        regions.append(
            _CallableRegion(
                name=name,
                start_index=start,
                end_index=end,
                instructions=tuple(program.instructions[start:end]),
            )
        )

    callable_lookup = {region.name for region in regions}
    for region in regions:
        for instruction in region.instructions:
            if instruction.opcode in (IrOp.JUMP, IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
                label = _as_label(
                    instruction.operands[-1],
                    f"{instruction.opcode.name} target",
                )
                target_index = label_positions.get(label.name)
                if target_index is None:
                    msg = f"Branch target {label.name!r} does not exist"
                    raise CILBackendError(msg)
                if not (region.start_index <= target_index < region.end_index):
                    msg = (
                        f"Branch target {label.name!r} escapes callable "
                        f"{region.name!r}"
                    )
                    raise CILBackendError(msg)
            elif instruction.opcode == IrOp.CALL:
                label = _as_label(instruction.operands[0], "CALL target")
                if label.name not in callable_lookup:
                    msg = f"CALL target {label.name!r} is not a callable label"
                    raise CILBackendError(msg)
    return regions


def _assign_data_offsets(
    program: IrProgram,
    config: CILBackendConfig,
) -> tuple[dict[str, int], int]:
    offset = 0
    offsets: dict[str, int] = {}
    for declaration in program.data:
        if declaration.size < 0:
            msg = f"Negative data size for {declaration.label!r}"
            raise CILBackendError(msg)
        if declaration.label in offsets:
            msg = f"Duplicate data label: {declaration.label}"
            raise CILBackendError(msg)
        if declaration.init < 0 or declaration.init > 0xFF:
            msg = f"Data init byte outside uint8 range for {declaration.label!r}"
            raise CILBackendError(msg)
        offsets[declaration.label] = offset
        offset += declaration.size
        if offset > config.max_static_data_bytes:
            msg = (
                "Total static data exceeds the CIL backend limit of "
                f"{config.max_static_data_bytes} bytes"
            )
            raise CILBackendError(msg)
    return offsets, offset


def _max_register_index(program: IrProgram) -> int:
    highest = -1
    for instruction in program.instructions:
        for operand in instruction.operands:
            if isinstance(operand, IrRegister):
                _validate_register(operand, "register operand")
                highest = max(highest, operand.index)
    return highest


def _lower_region(
    _program: IrProgram,
    config: CILBackendConfig,
    plan: CILLoweringPlan,
    region: _CallableRegion,
) -> CILMethodArtifact:
    builder = CILBytecodeBuilder()
    call_register_count = _call_register_count(config, plan)

    if region.name != _program.entry_label:
        for index in range(call_register_count):
            builder.emit_ldarg(index)
            builder.emit_stloc(index)

    for instruction in region.instructions:
        _emit_instruction(builder, instruction, config, plan)

    return CILMethodArtifact(
        name=region.name,
        body=builder.assemble(),
        max_stack=max(config.method_max_stack, call_register_count),
        local_types=tuple("int32" for _ in range(plan.local_count)),
        parameter_types=tuple(
            "int32"
            for _ in range(
                call_register_count if region.name != _program.entry_label else 0
            )
        ),
    )


def _emit_instruction(
    builder: CILBytecodeBuilder,
    instruction: IrInstruction,
    config: CILBackendConfig,
    plan: CILLoweringPlan,
) -> None:
    if instruction.opcode == IrOp.LABEL:
        label = _as_label(instruction.operands[0], "LABEL operand")
        builder.mark(label.name)
        return
    if instruction.opcode == IrOp.COMMENT:
        return
    if instruction.opcode == IrOp.NOP:
        builder.emit_opcode(CILOpcode.NOP)
        return

    if instruction.opcode == IrOp.LOAD_IMM:
        dst = _as_register(instruction.operands[0], "LOAD_IMM dst")
        imm = _as_immediate(instruction.operands[1], "LOAD_IMM immediate")
        _emit_store_immediate(builder, dst.index, imm.value)
        return

    if instruction.opcode == IrOp.LOAD_ADDR:
        dst = _as_register(instruction.operands[0], "LOAD_ADDR dst")
        label = _as_label(instruction.operands[1], "LOAD_ADDR label")
        offset = plan.data_offsets.get(label.name)
        if offset is None:
            msg = f"Unknown data label: {label.name}"
            raise CILBackendError(msg)
        _emit_store_immediate(builder, dst.index, offset)
        return

    if instruction.opcode in (IrOp.LOAD_BYTE, IrOp.LOAD_WORD):
        dst = _as_register(instruction.operands[0], f"{instruction.opcode.name} dst")
        base = _as_register(instruction.operands[1], f"{instruction.opcode.name} base")
        offset = _as_register(
            instruction.operands[2],
            f"{instruction.opcode.name} offset",
        )
        helper = (
            CILHelper.MEM_LOAD_BYTE
            if instruction.opcode == IrOp.LOAD_BYTE
            else CILHelper.LOAD_WORD
        )
        builder.emit_ldloc(base.index)
        builder.emit_ldloc(offset.index)
        builder.emit_add()
        builder.emit_call(plan.token_provider.helper_token(helper))
        builder.emit_stloc(dst.index)
        return

    if instruction.opcode in (IrOp.STORE_BYTE, IrOp.STORE_WORD):
        src = _as_register(instruction.operands[0], f"{instruction.opcode.name} src")
        base = _as_register(instruction.operands[1], f"{instruction.opcode.name} base")
        offset = _as_register(
            instruction.operands[2],
            f"{instruction.opcode.name} offset",
        )
        helper = (
            CILHelper.MEM_STORE_BYTE
            if instruction.opcode == IrOp.STORE_BYTE
            else CILHelper.STORE_WORD
        )
        builder.emit_ldloc(base.index)
        builder.emit_ldloc(offset.index)
        builder.emit_add()
        builder.emit_ldloc(src.index)
        builder.emit_call(plan.token_provider.helper_token(helper))
        return

    if instruction.opcode in (
        IrOp.ADD, IrOp.SUB, IrOp.AND, IrOp.OR, IrOp.XOR, IrOp.MUL, IrOp.DIV
    ):
        dst = _as_register(instruction.operands[0], f"{instruction.opcode.name} dst")
        lhs = _as_register(instruction.operands[1], f"{instruction.opcode.name} lhs")
        rhs = _as_register(instruction.operands[2], f"{instruction.opcode.name} rhs")
        builder.emit_ldloc(lhs.index)
        builder.emit_ldloc(rhs.index)
        _emit_binary_op(builder, instruction.opcode)
        builder.emit_stloc(dst.index)
        return

    if instruction.opcode in (IrOp.ADD_IMM, IrOp.AND_IMM, IrOp.OR_IMM, IrOp.XOR_IMM):
        dst = _as_register(instruction.operands[0], f"{instruction.opcode.name} dst")
        src = _as_register(instruction.operands[1], f"{instruction.opcode.name} src")
        imm = _as_immediate(instruction.operands[2], f"{instruction.opcode.name} imm")
        builder.emit_ldloc(src.index)
        builder.emit_ldc_i4(imm.value)
        _emit_binary_op(builder, instruction.opcode)
        builder.emit_stloc(dst.index)
        return

    if instruction.opcode == IrOp.NOT:
        # NOT x = x XOR 0xFFFF_FFFF = x XOR -1
        # CIL does not have a dedicated bitwise-NOT opcode.  The canonical
        # CLR pattern is to push the operand, push the all-ones mask (-1 as a
        # signed int32, which is 0xFFFF_FFFF in two's-complement), and then
        # emit ``xor``.  ``ldc.i4.m1`` (0x15) is the one-byte short form for
        # pushing -1, so the sequence is compact: 1 + 1 + 1 = 3 bytes of
        # operand code (plus whatever ldloc/stloc need).
        dst = _as_register(instruction.operands[0], "NOT dst")
        src = _as_register(instruction.operands[1], "NOT src")
        builder.emit_ldloc(src.index)
        builder.emit_ldc_i4(-1)   # ldc.i4.m1 — push 0xFFFF_FFFF
        builder.emit_xor()        # xor — flips all 32 bits
        builder.emit_stloc(dst.index)
        return

    if instruction.opcode in (IrOp.CMP_EQ, IrOp.CMP_NE, IrOp.CMP_LT, IrOp.CMP_GT):
        dst = _as_register(instruction.operands[0], f"{instruction.opcode.name} dst")
        lhs = _as_register(instruction.operands[1], f"{instruction.opcode.name} lhs")
        rhs = _as_register(instruction.operands[2], f"{instruction.opcode.name} rhs")
        builder.emit_ldloc(lhs.index)
        builder.emit_ldloc(rhs.index)
        if instruction.opcode == IrOp.CMP_EQ:
            builder.emit_ceq()
        elif instruction.opcode == IrOp.CMP_NE:
            builder.emit_ceq()
            builder.emit_ldc_i4(0)
            builder.emit_ceq()
        elif instruction.opcode == IrOp.CMP_LT:
            builder.emit_clt()
        else:
            builder.emit_cgt()
        builder.emit_stloc(dst.index)
        return

    if instruction.opcode == IrOp.JUMP:
        label = _as_label(instruction.operands[0], "JUMP target")
        builder.emit_branch(CILBranchKind.ALWAYS, label.name)
        return

    if instruction.opcode in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
        reg = _as_register(instruction.operands[0], f"{instruction.opcode.name} reg")
        label = _as_label(instruction.operands[1], f"{instruction.opcode.name} target")
        branch_kind = (
            CILBranchKind.FALSE
            if instruction.opcode == IrOp.BRANCH_Z
            else CILBranchKind.TRUE
        )
        builder.emit_ldloc(reg.index)
        builder.emit_branch(branch_kind, label.name)
        return

    if instruction.opcode == IrOp.CALL:
        label = _as_label(instruction.operands[0], "CALL target")
        for index in range(_call_register_count(config, plan)):
            builder.emit_ldloc(index)
        builder.emit_call(plan.token_provider.method_token(label.name))
        builder.emit_stloc(1)
        return

    if instruction.opcode in (IrOp.RET, IrOp.HALT):
        builder.emit_ldloc(1)
        builder.emit_ret()
        return

    if instruction.opcode == IrOp.SYSCALL:
        number = _as_immediate(instruction.operands[0], "SYSCALL number")
        builder.emit_ldc_i4(number.value)
        builder.emit_ldloc(config.syscall_arg_reg)
        builder.emit_call(plan.token_provider.helper_token(CILHelper.SYSCALL))
        builder.emit_stloc(config.syscall_arg_reg)
        return

    msg = f"Unsupported IR opcode in CIL backend: {instruction.opcode}"
    raise CILBackendError(msg)


def _emit_store_immediate(
    builder: CILBytecodeBuilder,
    local_index: int,
    value: int,
) -> None:
    _require_int32(value, "integer immediate")
    builder.emit_ldc_i4(value)
    builder.emit_stloc(local_index)


def _call_register_count(
    config: CILBackendConfig,
    plan: CILLoweringPlan,
) -> int:
    if config.call_register_count is None:
        return plan.local_count
    return min(config.call_register_count, plan.local_count)


def _emit_binary_op(builder: CILBytecodeBuilder, opcode: IrOp) -> None:
    """Emit the single-byte CIL arithmetic or bitwise opcode for a binary IR op.

    The caller has already pushed lhs and rhs (or src and immediate) onto the
    evaluation stack.  This function emits exactly the operation byte; the
    caller emits the surrounding ``ldloc``/``stloc`` pair.

    Supported ops and their CIL bytecodes:

    +-----------+-----------+--------+
    | IR op(s)  | CIL op    | byte   |
    +===========+===========+========+
    | ADD(_IMM) | add       | 0x58   |
    | SUB       | sub       | 0x59   |
    | MUL       | mul       | 0x5A   |
    | DIV       | div       | 0x5B   |
    | AND(_IMM) | and       | 0x5F   |
    | OR(_IMM)  | or        | 0x60   |
    | XOR(_IMM) | xor       | 0x61   |
    +-----------+-----------+--------+
    """
    if opcode in (IrOp.ADD, IrOp.ADD_IMM):
        builder.emit_add()
    elif opcode == IrOp.SUB:
        builder.emit_sub()
    elif opcode == IrOp.MUL:
        builder.emit_mul()
    elif opcode == IrOp.DIV:
        builder.emit_div()
    elif opcode in (IrOp.AND, IrOp.AND_IMM):
        builder.emit_and()
    elif opcode in (IrOp.OR, IrOp.OR_IMM):
        builder.emit_or()
    elif opcode in (IrOp.XOR, IrOp.XOR_IMM):
        builder.emit_xor()
    else:
        msg = f"Unsupported binary opcode: {opcode}"
        raise CILBackendError(msg)


def _as_register(operand: object, context: str) -> IrRegister:
    if not isinstance(operand, IrRegister):
        msg = f"{context} must be an IrRegister"
        raise CILBackendError(msg)
    _validate_register(operand, context)
    return operand


def _as_immediate(operand: object, context: str) -> IrImmediate:
    if not isinstance(operand, IrImmediate):
        msg = f"{context} must be an IrImmediate"
        raise CILBackendError(msg)
    _require_int32(operand.value, context)
    return operand


def _as_label(operand: object, context: str) -> IrLabel:
    if not isinstance(operand, IrLabel):
        msg = f"{context} must be an IrLabel"
        raise CILBackendError(msg)
    if not operand.name:
        msg = f"{context} must not be empty"
        raise CILBackendError(msg)
    return operand


def _validate_register(register: IrRegister, context: str) -> None:
    if register.index < 0:
        msg = f"{context} index must be non-negative"
        raise CILBackendError(msg)
    if register.index > 0xFFFF:
        msg = f"{context} index outside CLR local slot range: {register.index}"
        raise CILBackendError(msg)


def _require_int32(value: int, context: str) -> None:
    if value < _INT32_MIN or value > _INT32_MAX:
        msg = f"{context} outside int32 range: {value}"
        raise CILBackendError(msg)
