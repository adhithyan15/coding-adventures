"""Lower compiler_ir programs into JVM class-file bytes.

This module implements a deliberately small, verifier-friendly JVM backend for
the repository's lower-level AOT IR. The generated classes are intentionally
"boring": plain static fields, plain static methods, integer arithmetic,
ordinary array loads/stores, and ordinary method calls.
"""

from __future__ import annotations

import os
import re
import struct
from contextlib import suppress
from dataclasses import dataclass, field
from pathlib import Path
from typing import Final

from compiler_ir import (
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from jvm_class_file import ACC_PUBLIC, ACC_STATIC, ACC_SUPER

_ACC_PRIVATE = 0x0002
_ACC_FINAL = 0x0010

_CONSTANT_UTF8 = 1
_CONSTANT_INTEGER = 3
_CONSTANT_CLASS = 7
_CONSTANT_STRING = 8
_CONSTANT_FIELDREF = 9
_CONSTANT_METHODREF = 10
_CONSTANT_INTERFACE_METHODREF = 11
_CONSTANT_NAME_AND_TYPE = 12

_OP_ICONST_M1 = 0x02
_OP_ICONST_0 = 0x03
_OP_ICONST_1 = 0x04
_OP_ICONST_2 = 0x05
_OP_ICONST_3 = 0x06
_OP_ICONST_4 = 0x07
_OP_ICONST_5 = 0x08
_OP_BIPUSH = 0x10
_OP_SIPUSH = 0x11
_OP_LDC = 0x12
_OP_LDC_W = 0x13
_OP_ILOAD = 0x15
_OP_ILOAD_0 = 0x1A
_OP_ISTORE = 0x36
_OP_ISTORE_0 = 0x3B
_OP_IALOAD = 0x2E
_OP_BALOAD = 0x33
_OP_IASTORE = 0x4F
_OP_BASTORE = 0x54
_OP_POP = 0x57
_OP_DUP = 0x59
_OP_ACONST_NULL = 0x01
_OP_IADD = 0x60
_OP_ISUB = 0x64
_OP_IMUL = 0x68
_OP_IDIV = 0x6C
_OP_ISHL = 0x78
_OP_ISHR = 0x7A
_OP_IAND = 0x7E
_OP_IOR = 0x80
_OP_IXOR = 0x82   # bitwise XOR of two ints
_OP_I2B = 0x91
_OP_IFEQ = 0x99
_OP_IFNE = 0x9A
_OP_IF_ICMPEQ = 0x9F
_OP_IF_ICMPNE = 0xA0
_OP_IF_ICMPLT = 0xA1
_OP_IF_ICMPGT = 0xA3
_OP_GOTO = 0xA7
_OP_IRETURN = 0xAC
_OP_RETURN = 0xB1
_OP_GETSTATIC = 0xB2
_OP_PUTSTATIC = 0xB3
_OP_GETFIELD = 0xB4
_OP_PUTFIELD = 0xB5
_OP_INVOKEVIRTUAL = 0xB6
_OP_INVOKESPECIAL = 0xB7   # for chaining into super .ctor
_OP_INVOKESTATIC = 0xB8
_OP_INVOKEINTERFACE = 0xB9 # for closure apply dispatch (JVM02 Phase 2c)
_OP_NEW = 0xBB             # for newobj of a closure subclass
_OP_NEWARRAY = 0xBC
_OP_ANEWARRAY = 0xBD       # reserved for closure pool init
_OP_ALOAD_0 = 0x2A         # load `this` (or any aload short form)
_OP_AASTORE = 0x53
_OP_AALOAD = 0x32
_OP_NOP = 0x00

_ATYPE_INT = 10
_ATYPE_BYTE = 8

_DESC_INT = "I"
_DESC_VOID = "V"
_DESC_INT_ARRAY = "[I"
_DESC_BYTE_ARRAY = "[B"
_DESC_MAIN = "([Ljava/lang/String;)V"
_DESC_NOARGS_INT = "()I"
_DESC_NOARGS_VOID = "()V"
_DESC_INT_TO_INT = "(I)I"
_DESC_INT_INT_TO_VOID = "(II)V"
_DESC_ARRAYS_FILL_BYTE_RANGE = "([BIIB)V"
_DESC_PRINTSTREAM_WRITE = "(I)V"
_DESC_INPUTSTREAM_READ = "()I"

_JAVA_BINARY_NAME_RE = re.compile(
    r"[A-Za-z_$][A-Za-z0-9_$]*(?:\.[A-Za-z_$][A-Za-z0-9_$]*)*"
)
_MAX_STATIC_DATA_BYTES = 16 * 1024 * 1024


class JvmBackendError(ValueError):
    """Raised when an IR program cannot be lowered by this backend."""


@dataclass(frozen=True)
class JvmBackendConfig:
    """Configuration for JVM class-file lowering.

    ``closure_free_var_counts`` declares which IR regions are
    lifted-lambda bodies (TW03 Phase 2 / JVM02 Phase 2c).  Each
    entry maps a region's name to its number of captured free
    variables.  Regions in this map are routed to per-lambda
    ``Closure_<name>`` subclasses via the multi-class output
    instead of becoming methods on the main user class.

    The lambda body sees a captures-first IR register layout
    (matches what twig-jvm-compiler will produce and what BEAM
    Phase 2 + CLR Phase 2c already use):

      * ``r2..r{1+num_free}``                             — captures
      * ``r{2+num_free}..r{1+num_free+explicit_arity}``   — explicit args
    """

    class_name: str
    class_file_major: int = 49
    class_file_minor: int = 0
    emit_main_wrapper: bool = True
    syscall_arg_reg: int = 4  # register holding the SYSCALL print/read argument (Brainfuck=4, BASIC=0)
    closure_free_var_counts: dict[str, int] = field(default_factory=dict)


@dataclass(frozen=True)
class JVMClassArtifact:
    """The result of lowering an IR program to JVM class-file bytes."""

    class_name: str
    class_bytes: bytes
    callable_labels: tuple[str, ...]
    data_offsets: dict[str, int]

    @property
    def class_filename(self) -> str:
        """Return the relative class-file path inside a classpath root."""
        return self.class_name.replace(".", "/") + ".class"


# JVM02 Phase 2b — multi-class output for closures.
#
# The single-class ``JVMClassArtifact`` is enough for IR programs that
# don't use closures; closure-enabled programs emit a ``Closure``
# interface plus one ``Closure_<lambda>`` class per lifted lambda
# alongside the main user class.  ``JVMMultiClassArtifact`` is the
# shape that future phases (2c lowering, 2d twig frontend) will
# return.

# The shared ``Closure`` interface that every closure object
# implements.  Lives at a fixed binary name under the runtime
# package so that ``coding_adventures.twig.<assembly>.Closure_<lambda>``
# classes can reference it via a stable ``Closure`` symbol.
CLOSURE_INTERFACE_BINARY_NAME: Final = (
    "coding_adventures/twig/runtime/Closure"
)

# The single method on ``Closure``: ``int apply(int[] args)``.
# Future closure-apply lowering uses ``invokeinterface`` against
# this descriptor.
CLOSURE_INTERFACE_METHOD_NAME: Final = "apply"
CLOSURE_INTERFACE_METHOD_DESCRIPTOR: Final = "([I)I"


@dataclass(frozen=True)
class JVMMultiClassArtifact:
    """A multi-class lowering result — the main user class plus any
    closure-support classes.

    JVM02 Phase 2b ships the data shape and the ``Closure`` interface;
    Phase 2c populates ``classes`` with one ``Closure_<name>`` per
    lifted lambda; Phase 2d wires the JAR packaging.

    Invariant: ``classes[0]`` is always the main user class (whose
    ``class_name`` matches ``JvmBackendConfig.class_name``).
    Subsequent entries are runtime helpers / closure subclasses in
    a deterministic order so callers can rely on the layout when
    constructing JARs.
    """

    classes: tuple[JVMClassArtifact, ...]

    @property
    def main(self) -> JVMClassArtifact:
        """The main user-program class — the JAR's ``Main-Class``."""
        if not self.classes:
            msg = (
                "JVMMultiClassArtifact must contain at least the main "
                "user class"
            )
            raise JvmBackendError(msg)
        return self.classes[0]

    @property
    def class_filenames(self) -> tuple[str, ...]:
        """Relative ``.class`` paths for every artifact, suitable for
        passing to a JAR builder."""
        return tuple(a.class_filename for a in self.classes)


@dataclass(frozen=True)
class _FieldSpec:
    access_flags: int
    name: str
    descriptor: str


@dataclass(frozen=True)
class _MethodSpec:
    access_flags: int
    name: str
    descriptor: str
    code: bytes
    max_stack: int
    max_locals: int


@dataclass(frozen=True)
class _LabelMarker:
    name: str


@dataclass(frozen=True)
class _RawBytes:
    data: bytes


@dataclass(frozen=True)
class _BranchRef:
    opcode: int
    label: str


class _ConstantPoolBuilder:
    """Minimal constant-pool encoder with deduplication."""

    def __init__(self) -> None:
        self._entries: list[bytes] = []
        self._indices: dict[tuple[object, ...], int] = {}

    def _add(self, key: tuple[object, ...], payload: bytes) -> int:
        existing = self._indices.get(key)
        if existing is not None:
            return existing
        self._entries.append(payload)
        index = len(self._entries)
        self._indices[key] = index
        return index

    def utf8(self, value: str) -> int:
        encoded = value.encode("utf-8")
        return self._add(
            ("Utf8", value),
            bytes([_CONSTANT_UTF8]) + _u2(len(encoded)) + encoded,
        )

    def integer(self, value: int) -> int:
        return self._add(
            ("Integer", value),
            bytes([_CONSTANT_INTEGER]) + struct.pack(">i", value),
        )

    def class_ref(self, internal_name: str) -> int:
        name_index = self.utf8(internal_name)
        return self._add(
            ("Class", internal_name),
            bytes([_CONSTANT_CLASS]) + _u2(name_index),
        )

    def string(self, value: str) -> int:
        string_index = self.utf8(value)
        return self._add(
            ("String", value),
            bytes([_CONSTANT_STRING]) + _u2(string_index),
        )

    def name_and_type(self, name: str, descriptor: str) -> int:
        return self._add(
            ("NameAndType", name, descriptor),
            bytes([_CONSTANT_NAME_AND_TYPE])
            + _u2(self.utf8(name))
            + _u2(self.utf8(descriptor)),
        )

    def field_ref(self, owner: str, name: str, descriptor: str) -> int:
        return self._add(
            ("Fieldref", owner, name, descriptor),
            bytes([_CONSTANT_FIELDREF])
            + _u2(self.class_ref(owner))
            + _u2(self.name_and_type(name, descriptor)),
        )

    def method_ref(self, owner: str, name: str, descriptor: str) -> int:
        return self._add(
            ("Methodref", owner, name, descriptor),
            bytes([_CONSTANT_METHODREF])
            + _u2(self.class_ref(owner))
            + _u2(self.name_and_type(name, descriptor)),
        )

    def interface_method_ref(
        self, owner: str, name: str, descriptor: str,
    ) -> int:
        """``InterfaceMethodref`` constant — operand to
        ``invokeinterface`` (JVMS §4.4.2)."""
        return self._add(
            ("InterfaceMethodref", owner, name, descriptor),
            bytes([_CONSTANT_INTERFACE_METHODREF])
            + _u2(self.class_ref(owner))
            + _u2(self.name_and_type(name, descriptor)),
        )

    def encode(self) -> bytes:
        return b"".join(self._entries)

    @property
    def count(self) -> int:
        return len(self._entries) + 1


class _BytecodeBuilder:
    """Two-pass JVM bytecode assembler for a small instruction subset."""

    def __init__(self) -> None:
        self._items: list[_LabelMarker | _RawBytes | _BranchRef] = []

    def mark(self, label: str) -> None:
        self._items.append(_LabelMarker(label))

    def emit_raw(self, data: bytes) -> None:
        self._items.append(_RawBytes(data))

    def emit_opcode(self, opcode: int) -> None:
        self.emit_raw(bytes([opcode]))

    def emit_u1_instruction(self, opcode: int, operand: int) -> None:
        self.emit_raw(bytes([opcode, operand & 0xFF]))

    def emit_u2_instruction(self, opcode: int, operand: int) -> None:
        self.emit_raw(bytes([opcode]) + _u2(operand))

    def emit_branch(self, opcode: int, label: str) -> None:
        self._items.append(_BranchRef(opcode, label))

    def assemble(self) -> bytes:
        label_offsets: dict[str, int] = {}
        offset = 0
        for item in self._items:
            if isinstance(item, _LabelMarker):
                label_offsets[item.name] = offset
            elif isinstance(item, _RawBytes):
                offset += len(item.data)
            else:
                offset += 3

        output = bytearray()
        offset = 0
        for item in self._items:
            if isinstance(item, _LabelMarker):
                continue
            if isinstance(item, _RawBytes):
                output.extend(item.data)
                offset += len(item.data)
                continue
            target = label_offsets.get(item.label)
            if target is None:
                raise JvmBackendError(f"Unknown bytecode label: {item.label}")
            branch_offset = target - offset
            if branch_offset < -32768 or branch_offset > 32767:
                raise JvmBackendError(
                    "Branch offset out of range for label "
                    f"{item.label}: {branch_offset}"
                )
            output.extend(bytes([item.opcode]) + struct.pack(">h", branch_offset))
            offset += 3
        return bytes(output)


@dataclass(frozen=True)
class _CallableRegion:
    name: str
    start_index: int
    end_index: int
    instructions: tuple[IrInstruction, ...]


class _JvmClassLowerer:
    """Internal lowering engine."""

    def __init__(self, program: IrProgram, config: JvmBackendConfig) -> None:
        self.program = program
        self.config = config
        self.internal_name = config.class_name.replace(".", "/")
        self.cp = _ConstantPoolBuilder()
        self._data_offsets: dict[str, int] = {}
        self._fresh_label_id = 0
        self._helper_reg_field = "__ca_regs"
        self._helper_mem_field = "__ca_memory"
        self._helper_reg_get = "__ca_regGet"
        self._helper_reg_set = "__ca_regSet"
        self._helper_mem_load_byte = "__ca_memLoadByte"
        self._helper_mem_store_byte = "__ca_memStoreByte"
        self._helper_load_word = "__ca_loadWord"
        self._helper_store_word = "__ca_storeWord"
        self._helper_syscall = "__ca_syscall"

    def lower(self) -> JVMClassArtifact:
        self._validate_class_name()
        label_positions = self._collect_labels()
        callable_regions = self._discover_callable_regions(label_positions)
        self._validate_helper_name_collisions(callable_regions)
        data_offsets = self._assign_data_offsets()
        self._data_offsets = data_offsets
        # Register 1 is read by every HALT/RET emission (_emit_reg_get(builder, 1))
        # even when the IR program only references register 0 explicitly.  The
        # array must therefore be at least 2 elements long so index 1 is valid.
        reg_count = max(self._max_register_index() + 1, 2)

        fields = [
            _FieldSpec(
                _ACC_PRIVATE | ACC_STATIC,
                self._helper_reg_field,
                _DESC_INT_ARRAY,
            ),
            _FieldSpec(
                _ACC_PRIVATE | ACC_STATIC,
                self._helper_mem_field,
                _DESC_BYTE_ARRAY,
            ),
        ]

        methods = [
            self._build_class_initializer(reg_count, data_offsets),
            self._build_reg_get_method(),
            self._build_reg_set_method(),
            self._build_mem_load_byte_method(),
            self._build_mem_store_byte_method(),
            self._build_load_word_method(),
            self._build_store_word_method(),
            self._build_syscall_method(),
        ]
        # Closure regions (lifted lambdas) become methods on their
        # own ``Closure_<name>`` subclasses (built by
        # ``build_closure_subclass_artifact`` in the multi-class
        # path) — skip them here so the main user class doesn't
        # double-host the body.
        closure_region_names = set(self.config.closure_free_var_counts)
        methods.extend(
            self._build_callable_method(region, reg_count)
            for region in callable_regions
            if region.name not in closure_region_names
        )
        if self.config.emit_main_wrapper:
            methods.append(self._build_main_method())

        class_bytes = self._encode_class_file(fields, methods)
        return JVMClassArtifact(
            class_name=self.config.class_name,
            class_bytes=class_bytes,
            callable_labels=tuple(
                region.name for region in callable_regions
                if region.name not in closure_region_names
            ),
            data_offsets=data_offsets,
        )

    def _validate_class_name(self) -> None:
        if not self.config.class_name:
            raise JvmBackendError("class_name must not be empty")
        if not _JAVA_BINARY_NAME_RE.fullmatch(self.config.class_name):
            raise JvmBackendError(
                "class_name must be a legal Java binary name made of "
                "dot-separated identifiers"
            )

    def _collect_labels(self) -> dict[str, int]:
        positions: dict[str, int] = {}
        for index, instruction in enumerate(self.program.instructions):
            if instruction.opcode != IrOp.LABEL:
                continue
            label = _as_label(instruction.operands[0], "LABEL operand")
            if label.name in positions:
                raise JvmBackendError(f"Duplicate IR label: {label.name}")
            positions[label.name] = index
        return positions

    def _discover_callable_regions(
        self,
        label_positions: dict[str, int],
    ) -> list[_CallableRegion]:
        callable_names = {self.program.entry_label}
        for instruction in self.program.instructions:
            if instruction.opcode == IrOp.CALL:
                target = _as_label(instruction.operands[0], "CALL target")
                callable_names.add(target.name)
            elif instruction.opcode == IrOp.MAKE_CLOSURE:
                # MAKE_CLOSURE dst, fn_label, num_captured, capt0, ...
                # The lambda body is a callable region too — invoked
                # indirectly via ``invokeinterface Closure.apply([I)I``
                # rather than directly via ``invokestatic``.
                target = _as_label(
                    instruction.operands[1], "MAKE_CLOSURE fn_label",
                )
                callable_names.add(target.name)

        if self.program.entry_label not in label_positions:
            raise JvmBackendError(f"Entry label not found: {self.program.entry_label}")
        missing = sorted(callable_names - set(label_positions))
        if missing:
            raise JvmBackendError(f"Missing callable labels: {missing}")

        ordered_names = sorted(callable_names, key=lambda name: label_positions[name])
        regions: list[_CallableRegion] = []
        for index, name in enumerate(ordered_names):
            start = label_positions[name]
            end = (
                label_positions[ordered_names[index + 1]]
                if index + 1 < len(ordered_names)
                else len(self.program.instructions)
            )
            region_instructions = tuple(self.program.instructions[start:end])
            regions.append(
                _CallableRegion(
                    name=name,
                    start_index=start,
                    end_index=end,
                    instructions=region_instructions,
                )
            )

        callable_lookup = {region.name for region in regions}
        for region in regions:
            for instruction in region.instructions:
                if instruction.opcode in (IrOp.JUMP, IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
                    label_operand = _as_label(
                        instruction.operands[-1],
                        f"{instruction.opcode.name} target",
                    )
                    target_index = label_positions.get(label_operand.name)
                    if target_index is None:
                        raise JvmBackendError(
                            f"Branch target {label_operand.name!r} does not exist"
                        )
                    if not (region.start_index <= target_index < region.end_index):
                        raise JvmBackendError(
                            "Branch target "
                            f"{label_operand.name!r} escapes callable "
                            f"{region.name!r}"
                        )
                elif instruction.opcode == IrOp.CALL:
                    label_operand = _as_label(instruction.operands[0], "CALL target")
                    if label_operand.name not in callable_lookup:
                        raise JvmBackendError(
                            "CALL target "
                            f"{label_operand.name!r} is not a callable label"
                        )
        return regions

    def _validate_helper_name_collisions(self, regions: list[_CallableRegion]) -> None:
        reserved = {
            self._helper_reg_get,
            self._helper_reg_set,
            self._helper_mem_load_byte,
            self._helper_mem_store_byte,
            self._helper_load_word,
            self._helper_store_word,
            self._helper_syscall,
            "<clinit>",
            "main",
        }
        collisions = sorted(
            reserved.intersection(region.name for region in regions)
        )
        if collisions:
            raise JvmBackendError(
                "Callable labels collide with helper names: "
                f"{collisions}"
            )

    def _assign_data_offsets(self) -> dict[str, int]:
        offset = 0
        offsets: dict[str, int] = {}
        for declaration in self.program.data:
            if declaration.size < 0:
                raise JvmBackendError(f"Negative data size for {declaration.label!r}")
            offsets[declaration.label] = offset
            offset += declaration.size
            if offset > _MAX_STATIC_DATA_BYTES:
                raise JvmBackendError(
                    "Total static data exceeds the JVM backend limit of "
                    f"{_MAX_STATIC_DATA_BYTES} bytes"
                )
        return offsets

    def _max_register_index(self) -> int:
        highest = -1
        for instruction in self.program.instructions:
            for operand in instruction.operands:
                if isinstance(operand, IrRegister):
                    highest = max(highest, operand.index)
        return highest

    def _fresh_label(self, prefix: str) -> str:
        self._fresh_label_id += 1
        return f"__ca_{prefix}_{self._fresh_label_id}"

    def _field_ref(self, name: str, descriptor: str) -> int:
        return self.cp.field_ref(self.internal_name, name, descriptor)

    def _method_ref(self, name: str, descriptor: str) -> int:
        return self.cp.method_ref(self.internal_name, name, descriptor)

    def _emit_push_int(self, builder: _BytecodeBuilder, value: int) -> None:
        if value == -1:
            builder.emit_opcode(_OP_ICONST_M1)
            return
        if 0 <= value <= 5:
            builder.emit_opcode(_OP_ICONST_0 + value)
            return
        if -128 <= value <= 127:
            builder.emit_u1_instruction(_OP_BIPUSH, value)
            return
        if -32768 <= value <= 32767:
            builder.emit_raw(bytes([_OP_SIPUSH]) + struct.pack(">h", value))
            return
        constant_index = self.cp.integer(value)
        if constant_index <= 0xFF:
            builder.emit_u1_instruction(_OP_LDC, constant_index)
        else:
            builder.emit_u2_instruction(_OP_LDC_W, constant_index)

    def _emit_iload(self, builder: _BytecodeBuilder, index: int) -> None:
        if 0 <= index <= 3:
            builder.emit_opcode(_OP_ILOAD_0 + index)
        else:
            builder.emit_u1_instruction(_OP_ILOAD, index)

    def _emit_istore(self, builder: _BytecodeBuilder, index: int) -> None:
        if 0 <= index <= 3:
            builder.emit_opcode(_OP_ISTORE_0 + index)
        else:
            builder.emit_u1_instruction(_OP_ISTORE, index)

    def _emit_reg_get(self, builder: _BytecodeBuilder, index: int) -> None:
        self._emit_push_int(builder, index)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_reg_get, _DESC_INT_TO_INT),
        )

    def _emit_reg_set(
        self,
        builder: _BytecodeBuilder,
        dst_index: int,
        value: int,
    ) -> None:
        self._emit_push_int(builder, dst_index)
        self._emit_push_int(builder, value)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
        )

    # ------------------------------------------------------------------
    # JVM01 — caller-saves convention around CALL
    # ------------------------------------------------------------------
    #
    # The static ``__ca_regs`` array holds every "IR register" and is
    # shared across method invocations.  That breaks recursion: when
    # ``fact(5)`` calls ``fact(4)``, ``fact(5)``'s own parameter
    # register r2 (= 5) is overwritten by the call setup writing 4
    # there.  When ``fact(4)`` returns, the outer multiplication
    # reads r2 = 4 instead of 5.
    #
    # Fix: each ``IrOp.CALL`` sandwich ``save → invoke → restore``.
    # The save sequence snapshots the entire static array into JVM
    # locals 0..N-1 (per-method, isolated by the JVM frame
    # discipline).  The restore sequence writes everything back
    # *except* register 1 — that's the convention HALT/RET use to
    # carry the callee's return value back to the caller.
    #
    # Cost: 2 × N bytecode pairs per CALL.  N is small (the IR's
    # max register index + 1) so the overhead is bounded.

    def _emit_caller_save_registers(
        self, builder: _BytecodeBuilder, reg_count: int
    ) -> None:
        """Snapshot static-array regs 0..reg_count-1 → JVM locals 0..N-1."""
        for reg_idx in range(reg_count):
            builder.emit_u2_instruction(
                _OP_GETSTATIC,
                self._field_ref(self._helper_reg_field, _DESC_INT_ARRAY),
            )
            self._emit_push_int(builder, reg_idx)
            builder.emit_opcode(_OP_IALOAD)
            self._emit_istore(builder, reg_idx)

    def _emit_caller_restore_registers(
        self, builder: _BytecodeBuilder, reg_count: int
    ) -> None:
        """Write JVM locals 0..N-1 back to static-array regs.

        Skips index 1 — the callee's return value lives there per the
        existing HALT/RET convention, and we want the caller to see
        the new value, not the saved-pre-call one.
        """
        for reg_idx in range(reg_count):
            if reg_idx == 1:
                continue  # preserve callee's return value
            builder.emit_u2_instruction(
                _OP_GETSTATIC,
                self._field_ref(self._helper_reg_field, _DESC_INT_ARRAY),
            )
            self._emit_push_int(builder, reg_idx)
            self._emit_iload(builder, reg_idx)
            builder.emit_opcode(_OP_IASTORE)

    # ------------------------------------------------------------------
    # JVM02 Phase 2c — closure op emission (structural)
    # ------------------------------------------------------------------

    def _emit_make_closure(
        self,
        builder: _BytecodeBuilder,
        instruction: IrInstruction,
    ) -> None:
        """Emit ``new Closure_<fn>; dup; iload caps; invokespecial ctor``.

        Phase 2c structural-only: the resulting reference is popped
        because the existing register convention is int[] only and
        can't hold an object reference.  Phase 2c.5 adds a parallel
        ``Object[]`` pool to retain the reference; until then,
        MAKE_CLOSURE compiles + verifies but the closure value is
        not actually retrievable downstream.
        """
        if len(instruction.operands) < 3:
            raise JvmBackendError(
                f"MAKE_CLOSURE expects at least 3 operands "
                f"(dst, fn_label, num_captured, capt...), got "
                f"{len(instruction.operands)}"
            )
        # dst is unused at this phase — the reference is popped.
        _as_register(instruction.operands[0], "MAKE_CLOSURE dst")
        fn_label = _as_label(instruction.operands[1], "MAKE_CLOSURE fn_label")
        num_captured = _as_immediate(
            instruction.operands[2], "MAKE_CLOSURE num_captured",
        ).value

        configured = self.config.closure_free_var_counts.get(fn_label.name)
        if configured is None:
            raise JvmBackendError(
                f"MAKE_CLOSURE references {fn_label.name!r} but no "
                "closure region with that name is declared in "
                "JvmBackendConfig.closure_free_var_counts"
            )
        if configured != num_captured:
            raise JvmBackendError(
                f"MAKE_CLOSURE for {fn_label.name!r}: num_captured="
                f"{num_captured} but config.closure_free_var_counts "
                f"says {configured}"
            )
        if len(instruction.operands) != 3 + num_captured:
            raise JvmBackendError(
                f"MAKE_CLOSURE for {fn_label.name!r}: num_captured="
                f"{num_captured} but {len(instruction.operands) - 3} "
                "capture operands provided"
            )

        sanitised = _sanitise_class_name_segment(fn_label.name)
        closure_internal = f"coding_adventures/twig/runtime/Closure_{sanitised}"
        ctor_descriptor = "(" + ("I" * num_captured) + ")V"

        # new Closure_X
        builder.emit_u2_instruction(
            _OP_NEW, self.cp.class_ref(closure_internal),
        )
        # dup so the reference survives the ctor invocation
        builder.emit_opcode(_OP_DUP)
        # Push each capture by reading __ca_regs[capt_i].
        for i in range(num_captured):
            capt_reg = _as_register(
                instruction.operands[3 + i], f"MAKE_CLOSURE capture {i}",
            )
            self._emit_reg_get(builder, capt_reg.index)
        builder.emit_u2_instruction(
            _OP_INVOKESPECIAL,
            self.cp.method_ref(closure_internal, "<init>", ctor_descriptor),
        )
        # Phase 2c structural-only: discard the reference.  Phase 2c.5
        # will replace this with an aastore into __ca_objregs[dst].
        builder.emit_opcode(_OP_POP)

    def _emit_apply_closure(
        self,
        builder: _BytecodeBuilder,
        instruction: IrInstruction,
    ) -> None:
        """Emit closure-apply bytecode.

        Phase 2c structural-only: pushes ``aconst_null`` instead of
        the real closure reference (because the int[] register
        convention can't hold one), then builds the int[] args
        array and emits ``invokeinterface Closure.apply([I)I``.
        The result is popped; ``dst`` is left unwritten.

        At runtime this would NPE when the JVM tries to dispatch
        through the null reference — that's why the integration
        test for end-to-end semantics is xfail until Phase 2c.5.
        The bytecode still verifies cleanly, which proves the
        invokeinterface descriptor and the args-array construction
        are correct.
        """
        if len(instruction.operands) < 3:
            raise JvmBackendError(
                f"APPLY_CLOSURE expects at least 3 operands "
                f"(dst, closure, num_args, args...), got "
                f"{len(instruction.operands)}"
            )
        # dst, closure_reg unused at this phase — see docstring.
        _as_register(instruction.operands[0], "APPLY_CLOSURE dst")
        _as_register(instruction.operands[1], "APPLY_CLOSURE closure_reg")
        num_args = _as_immediate(
            instruction.operands[2], "APPLY_CLOSURE num_args",
        ).value

        if len(instruction.operands) != 3 + num_args:
            raise JvmBackendError(
                f"APPLY_CLOSURE: num_args={num_args} but "
                f"{len(instruction.operands) - 3} arg operands provided"
            )

        # Push placeholder closure reference.
        builder.emit_opcode(_OP_ACONST_NULL)
        # Build int[] args
        self._emit_push_int(builder, num_args)
        builder.emit_u1_instruction(_OP_NEWARRAY, _ATYPE_INT)
        for i in range(num_args):
            arg_reg = _as_register(
                instruction.operands[3 + i], f"APPLY_CLOSURE arg {i}",
            )
            builder.emit_opcode(_OP_DUP)
            self._emit_push_int(builder, i)
            self._emit_reg_get(builder, arg_reg.index)
            builder.emit_opcode(_OP_IASTORE)
        # invokeinterface Closure.apply([I)I — count operand = 2
        # (the receiver + the int[] arg, per JVMS §6.5.invokeinterface).
        iface_method_ref = self.cp.interface_method_ref(
            CLOSURE_INTERFACE_BINARY_NAME,
            CLOSURE_INTERFACE_METHOD_NAME,
            CLOSURE_INTERFACE_METHOD_DESCRIPTOR,
        )
        builder.emit_raw(
            bytes([_OP_INVOKEINTERFACE])
            + iface_method_ref.to_bytes(2, "big")
            + bytes([2, 0])
        )
        # Phase 2c structural-only: discard the int result.  Phase 2c.5
        # will store it into __ca_regs[dst].
        builder.emit_opcode(_OP_POP)

    def _build_class_initializer(
        self,
        reg_count: int,
        data_offsets: dict[str, int],
    ) -> _MethodSpec:
        builder = _BytecodeBuilder()

        self._emit_push_int(builder, reg_count)
        builder.emit_u1_instruction(_OP_NEWARRAY, _ATYPE_INT)
        builder.emit_u2_instruction(
            _OP_PUTSTATIC,
            self._field_ref(self._helper_reg_field, _DESC_INT_ARRAY),
        )

        total_bytes = sum(declaration.size for declaration in self.program.data)
        self._emit_push_int(builder, total_bytes)
        builder.emit_u1_instruction(_OP_NEWARRAY, _ATYPE_BYTE)
        builder.emit_u2_instruction(
            _OP_PUTSTATIC,
            self._field_ref(self._helper_mem_field, _DESC_BYTE_ARRAY),
        )

        for declaration in self.program.data:
            if declaration.init == 0 or declaration.size == 0:
                continue
            start = data_offsets[declaration.label]
            self._emit_fill_byte_range(
                builder,
                start=start,
                size=declaration.size,
                value=declaration.init,
            )

        builder.emit_opcode(_OP_RETURN)
        return _MethodSpec(
            access_flags=ACC_STATIC,
            name="<clinit>",
            descriptor=_DESC_NOARGS_VOID,
            code=builder.assemble(),
            max_stack=8,
            max_locals=0,
        )

    def _build_reg_get_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._helper_reg_field, _DESC_INT_ARRAY),
        )
        self._emit_iload(builder, 0)
        builder.emit_opcode(_OP_IALOAD)
        builder.emit_opcode(_OP_IRETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_reg_get,
            descriptor=_DESC_INT_TO_INT,
            code=builder.assemble(),
            max_stack=2,
            max_locals=1,
        )

    def _build_reg_set_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._helper_reg_field, _DESC_INT_ARRAY),
        )
        self._emit_iload(builder, 0)
        self._emit_iload(builder, 1)
        builder.emit_opcode(_OP_IASTORE)
        builder.emit_opcode(_OP_RETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_reg_set,
            descriptor=_DESC_INT_INT_TO_VOID,
            code=builder.assemble(),
            max_stack=3,
            max_locals=2,
        )

    def _emit_fill_byte_range(
        self,
        builder: _BytecodeBuilder,
        *,
        start: int,
        size: int,
        value: int,
    ) -> None:
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._helper_mem_field, _DESC_BYTE_ARRAY),
        )
        self._emit_push_int(builder, start)
        self._emit_push_int(builder, start + size)
        self._emit_push_int(builder, value)
        builder.emit_opcode(_OP_I2B)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self.cp.method_ref(
                "java/util/Arrays",
                "fill",
                _DESC_ARRAYS_FILL_BYTE_RANGE,
            ),
        )

    def _build_mem_load_byte_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._helper_mem_field, _DESC_BYTE_ARRAY),
        )
        self._emit_iload(builder, 0)
        builder.emit_opcode(_OP_BALOAD)
        self._emit_push_int(builder, 0xFF)
        builder.emit_opcode(_OP_IAND)
        builder.emit_opcode(_OP_IRETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_mem_load_byte,
            descriptor=_DESC_INT_TO_INT,
            code=builder.assemble(),
            max_stack=2,
            max_locals=1,
        )

    def _build_mem_store_byte_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._helper_mem_field, _DESC_BYTE_ARRAY),
        )
        self._emit_iload(builder, 0)
        self._emit_iload(builder, 1)
        builder.emit_opcode(_OP_BASTORE)
        builder.emit_opcode(_OP_RETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_mem_store_byte,
            descriptor=_DESC_INT_INT_TO_VOID,
            code=builder.assemble(),
            max_stack=3,
            max_locals=2,
        )

    def _build_load_word_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()

        self._emit_iload(builder, 0)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_mem_load_byte, _DESC_INT_TO_INT),
        )

        for shift, extra in ((8, 1), (16, 2), (24, 3)):
            self._emit_iload(builder, 0)
            self._emit_push_int(builder, extra)
            builder.emit_opcode(_OP_IADD)
            builder.emit_u2_instruction(
                _OP_INVOKESTATIC,
                self._method_ref(self._helper_mem_load_byte, _DESC_INT_TO_INT),
            )
            self._emit_push_int(builder, shift)
            builder.emit_opcode(_OP_ISHL)
            builder.emit_opcode(_OP_IOR)

        builder.emit_opcode(_OP_IRETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_load_word,
            descriptor=_DESC_INT_TO_INT,
            code=builder.assemble(),
            max_stack=4,
            max_locals=1,
        )

    def _build_store_word_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()

        for shift, extra in ((0, 0), (8, 1), (16, 2), (24, 3)):
            self._emit_iload(builder, 0)
            if extra:
                self._emit_push_int(builder, extra)
                builder.emit_opcode(_OP_IADD)
            self._emit_iload(builder, 1)
            if shift:
                self._emit_push_int(builder, shift)
                builder.emit_opcode(_OP_ISHR)
            builder.emit_u2_instruction(
                _OP_INVOKESTATIC,
                self._method_ref(
                    self._helper_mem_store_byte,
                    _DESC_INT_INT_TO_VOID,
                ),
            )

        builder.emit_opcode(_OP_RETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_store_word,
            descriptor=_DESC_INT_INT_TO_VOID,
            code=builder.assemble(),
            max_stack=4,
            max_locals=2,
        )

    def _build_syscall_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()
        label_read = self._fresh_label("sys_read")
        label_halt = self._fresh_label("sys_halt")
        label_have_input = self._fresh_label("sys_have_input")

        self._emit_iload(builder, 0)
        self._emit_push_int(builder, 1)
        builder.emit_branch(_OP_IF_ICMPNE, label_read)
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self.cp.field_ref(
                "java/lang/System",
                "out",
                "Ljava/io/PrintStream;",
            ),
        )
        # Load __ca_regs[arg_reg] — arg_reg is local variable 1 (runtime value).
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self.cp.field_ref(self.config.class_name, "__ca_regs", "[I"),
        )
        self._emit_iload(builder, 1)   # arg_reg index
        builder.emit_opcode(_OP_IALOAD)
        self._emit_push_int(builder, 0xFF)
        builder.emit_opcode(_OP_IAND)
        builder.emit_u2_instruction(
            _OP_INVOKEVIRTUAL,
            self.cp.method_ref(
                "java/io/PrintStream",
                "write",
                _DESC_PRINTSTREAM_WRITE,
            ),
        )
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self.cp.field_ref(
                "java/lang/System",
                "out",
                "Ljava/io/PrintStream;",
            ),
        )
        builder.emit_u2_instruction(
            _OP_INVOKEVIRTUAL,
            self.cp.method_ref(
                "java/io/PrintStream",
                "flush",
                _DESC_NOARGS_VOID,
            ),
        )
        builder.emit_opcode(_OP_RETURN)

        builder.mark(label_read)
        self._emit_iload(builder, 0)
        self._emit_push_int(builder, 2)
        builder.emit_branch(_OP_IF_ICMPNE, label_halt)
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self.cp.field_ref(
                "java/lang/System",
                "in",
                "Ljava/io/InputStream;",
            ),
        )
        builder.emit_u2_instruction(
            _OP_INVOKEVIRTUAL,
            self.cp.method_ref(
                "java/io/InputStream",
                "read",
                _DESC_INPUTSTREAM_READ,
            ),
        )
        # local 2 = byte read from stdin (local 1 is arg_reg)
        self._emit_istore(builder, 2)
        self._emit_iload(builder, 2)
        self._emit_push_int(builder, -1)
        builder.emit_branch(_OP_IF_ICMPNE, label_have_input)
        # EOF: store 0 into regs[arg_reg]
        self._emit_iload(builder, 1)   # arg_reg
        self._emit_push_int(builder, 0)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
        )
        builder.emit_opcode(_OP_RETURN)

        builder.mark(label_have_input)
        # Store read byte into regs[arg_reg]
        self._emit_iload(builder, 1)   # arg_reg
        self._emit_iload(builder, 2)   # byte value
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
        )
        builder.emit_opcode(_OP_RETURN)

        builder.mark(label_halt)
        builder.emit_opcode(_OP_RETURN)
        return _MethodSpec(
            access_flags=_ACC_PRIVATE | ACC_STATIC,
            name=self._helper_syscall,
            descriptor="(II)V",   # syscall_num, arg_reg
            code=builder.assemble(),
            max_stack=4,
            max_locals=3,          # 0=syscall_num, 1=arg_reg, 2=read_byte
        )

    def _build_callable_method(
        self, region: _CallableRegion, reg_count: int
    ) -> _MethodSpec:
        builder = _BytecodeBuilder()
        # ``reg_count`` is the program-wide max register count; the
        # caller-saves CALL convention (JVM01) uses JVM locals
        # 0..reg_count-1 to snapshot the static-array register state
        # across nested calls.  We pass it through so the method's
        # ``max_locals`` can be set high enough below.

        for instruction in region.instructions:
            if instruction.opcode == IrOp.LABEL:
                label = _as_label(instruction.operands[0], "LABEL operand")
                builder.mark(label.name)
                continue
            if instruction.opcode == IrOp.COMMENT:
                continue
            if instruction.opcode == IrOp.NOP:
                builder.emit_opcode(_OP_NOP)
                continue

            if instruction.opcode == IrOp.LOAD_IMM:
                dst = _as_register(instruction.operands[0], "LOAD_IMM dst")
                imm = _as_immediate(instruction.operands[1], "LOAD_IMM immediate")
                self._emit_push_int(builder, dst.index)
                self._emit_push_int(builder, imm.value)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode == IrOp.LOAD_ADDR:
                dst = _as_register(instruction.operands[0], "LOAD_ADDR dst")
                label = _as_label(instruction.operands[1], "LOAD_ADDR label")
                offset = self._data_offsets.get(label.name)
                if offset is None:
                    raise JvmBackendError(f"Unknown data label: {label.name}")
                self._emit_push_int(builder, dst.index)
                self._emit_push_int(builder, offset)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode == IrOp.LOAD_BYTE:
                dst = _as_register(instruction.operands[0], "LOAD_BYTE dst")
                base = _as_register(instruction.operands[1], "LOAD_BYTE base")
                offset = _as_register(instruction.operands[2], "LOAD_BYTE offset")
                self._emit_push_int(builder, dst.index)
                self._emit_reg_get(builder, base.index)
                self._emit_reg_get(builder, offset.index)
                builder.emit_opcode(_OP_IADD)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_mem_load_byte, _DESC_INT_TO_INT),
                )
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode == IrOp.STORE_BYTE:
                src = _as_register(instruction.operands[0], "STORE_BYTE src")
                base = _as_register(instruction.operands[1], "STORE_BYTE base")
                offset = _as_register(instruction.operands[2], "STORE_BYTE offset")
                self._emit_reg_get(builder, base.index)
                self._emit_reg_get(builder, offset.index)
                builder.emit_opcode(_OP_IADD)
                self._emit_reg_get(builder, src.index)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(
                        self._helper_mem_store_byte,
                        _DESC_INT_INT_TO_VOID,
                    ),
                )
                continue

            if instruction.opcode == IrOp.LOAD_WORD:
                dst = _as_register(instruction.operands[0], "LOAD_WORD dst")
                base = _as_register(instruction.operands[1], "LOAD_WORD base")
                offset = _as_register(instruction.operands[2], "LOAD_WORD offset")
                self._emit_push_int(builder, dst.index)
                self._emit_reg_get(builder, base.index)
                self._emit_reg_get(builder, offset.index)
                builder.emit_opcode(_OP_IADD)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_load_word, _DESC_INT_TO_INT),
                )
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode == IrOp.STORE_WORD:
                src = _as_register(instruction.operands[0], "STORE_WORD src")
                base = _as_register(instruction.operands[1], "STORE_WORD base")
                offset = _as_register(instruction.operands[2], "STORE_WORD offset")
                self._emit_reg_get(builder, base.index)
                self._emit_reg_get(builder, offset.index)
                builder.emit_opcode(_OP_IADD)
                self._emit_reg_get(builder, src.index)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(
                        self._helper_store_word,
                        _DESC_INT_INT_TO_VOID,
                    ),
                )
                continue

            if instruction.opcode in (IrOp.ADD, IrOp.SUB, IrOp.AND, IrOp.OR, IrOp.XOR, IrOp.MUL, IrOp.DIV):
                dst = _as_register(
                    instruction.operands[0],
                    f"{instruction.opcode.name} dst",
                )
                lhs = _as_register(
                    instruction.operands[1],
                    f"{instruction.opcode.name} lhs",
                )
                rhs = _as_register(
                    instruction.operands[2],
                    f"{instruction.opcode.name} rhs",
                )
                self._emit_push_int(builder, dst.index)
                self._emit_reg_get(builder, lhs.index)
                self._emit_reg_get(builder, rhs.index)
                if instruction.opcode == IrOp.ADD:
                    builder.emit_opcode(_OP_IADD)
                elif instruction.opcode == IrOp.SUB:
                    builder.emit_opcode(_OP_ISUB)
                elif instruction.opcode == IrOp.MUL:
                    builder.emit_opcode(_OP_IMUL)
                elif instruction.opcode == IrOp.DIV:
                    builder.emit_opcode(_OP_IDIV)
                elif instruction.opcode == IrOp.OR:
                    builder.emit_opcode(_OP_IOR)
                elif instruction.opcode == IrOp.XOR:
                    builder.emit_opcode(_OP_IXOR)
                else:
                    builder.emit_opcode(_OP_IAND)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode in (IrOp.ADD_IMM, IrOp.AND_IMM, IrOp.OR_IMM, IrOp.XOR_IMM):
                dst = _as_register(
                    instruction.operands[0],
                    f"{instruction.opcode.name} dst",
                )
                src = _as_register(
                    instruction.operands[1],
                    f"{instruction.opcode.name} src",
                )
                imm = _as_immediate(
                    instruction.operands[2],
                    f"{instruction.opcode.name} imm",
                )
                self._emit_push_int(builder, dst.index)
                self._emit_reg_get(builder, src.index)
                self._emit_push_int(builder, imm.value)
                if instruction.opcode == IrOp.ADD_IMM:
                    builder.emit_opcode(_OP_IADD)
                elif instruction.opcode == IrOp.OR_IMM:
                    builder.emit_opcode(_OP_IOR)
                elif instruction.opcode == IrOp.XOR_IMM:
                    builder.emit_opcode(_OP_IXOR)
                else:
                    builder.emit_opcode(_OP_IAND)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode == IrOp.NOT:
                # NOT(x) = x XOR 0xFFFFFFFF = x XOR -1 in two's complement.
                # We push src, push iconst_m1 (-1 = all bits set), then ixor,
                # which flips every bit in the 32-bit int.
                dst = _as_register(instruction.operands[0], "NOT dst")
                src = _as_register(instruction.operands[1], "NOT src")
                self._emit_push_int(builder, dst.index)
                self._emit_reg_get(builder, src.index)
                builder.emit_opcode(_OP_ICONST_M1)   # push -1 (all 32 bits set)
                builder.emit_opcode(_OP_IXOR)          # XOR: all bits flipped
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode in (
                IrOp.CMP_EQ,
                IrOp.CMP_NE,
                IrOp.CMP_LT,
                IrOp.CMP_GT,
            ):
                dst = _as_register(
                    instruction.operands[0],
                    f"{instruction.opcode.name} dst",
                )
                lhs = _as_register(
                    instruction.operands[1],
                    f"{instruction.opcode.name} lhs",
                )
                rhs = _as_register(
                    instruction.operands[2],
                    f"{instruction.opcode.name} rhs",
                )
                true_label = self._fresh_label("cmp_true")
                done_label = self._fresh_label("cmp_done")
                branch_opcode = {
                    IrOp.CMP_EQ: _OP_IF_ICMPEQ,
                    IrOp.CMP_NE: _OP_IF_ICMPNE,
                    IrOp.CMP_LT: _OP_IF_ICMPLT,
                    IrOp.CMP_GT: _OP_IF_ICMPGT,
                }[instruction.opcode]
                self._emit_push_int(builder, dst.index)
                self._emit_reg_get(builder, lhs.index)
                self._emit_reg_get(builder, rhs.index)
                builder.emit_branch(branch_opcode, true_label)
                self._emit_push_int(builder, 0)
                builder.emit_branch(_OP_GOTO, done_label)
                builder.mark(true_label)
                self._emit_push_int(builder, 1)
                builder.mark(done_label)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                continue

            if instruction.opcode == IrOp.JUMP:
                label = _as_label(instruction.operands[0], "JUMP target")
                builder.emit_branch(_OP_GOTO, label.name)
                continue

            if instruction.opcode in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
                reg = _as_register(
                    instruction.operands[0],
                    f"{instruction.opcode.name} reg",
                )
                label = _as_label(
                    instruction.operands[1],
                    f"{instruction.opcode.name} target",
                )
                self._emit_reg_get(builder, reg.index)
                builder.emit_branch(
                    _OP_IFEQ if instruction.opcode == IrOp.BRANCH_Z else _OP_IFNE,
                    label.name,
                )
                continue

            if instruction.opcode == IrOp.CALL:
                label = _as_label(instruction.operands[0], "CALL target")
                # ── JVM01: caller-saves convention around CALL ────────
                # All "IR registers" live in a class-level static int
                # array (``__ca_regs``).  Without intervention, a
                # recursive call clobbers the caller's register state
                # — fact(5)'s own param r2 = 5 gets overwritten when
                # fact(4) is set up.
                #
                # Fix: snapshot the entire static array into JVM locals
                # before the call, restore everything except register 1
                # afterwards (register 1 carries the callee's return
                # value, by long-standing convention with HALT/RET).
                #
                # JVM locals are per-method (the JVM spec guarantees
                # fresh frames per invokestatic), so the snapshot is
                # private to each invocation — this is what makes
                # recursion work.
                self._emit_caller_save_registers(builder, reg_count)
                self._emit_push_int(builder, 1)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(label.name, _DESC_NOARGS_INT),
                )
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                self._emit_caller_restore_registers(builder, reg_count)
                continue

            if instruction.opcode == IrOp.RET or instruction.opcode == IrOp.HALT:
                self._emit_reg_get(builder, 1)
                builder.emit_opcode(_OP_IRETURN)
                continue

            if instruction.opcode == IrOp.SYSCALL:
                # SYSCALL carries two operands: the syscall number (immediate) and
                # the argument register (register).  The register operand makes the
                # IR self-describing — no backend config is needed.
                number = _as_immediate(instruction.operands[0], "SYSCALL number")
                arg_reg = _as_register(instruction.operands[1], "SYSCALL arg register")
                self._emit_push_int(builder, number.value)
                self._emit_push_int(builder, arg_reg.index)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_syscall, "(II)V"),
                )
                continue

            if instruction.opcode == IrOp.MAKE_CLOSURE:
                # MAKE_CLOSURE dst, fn_label, num_captured, capt0, ..., captN-1
                # → new Closure_<fn>; dup; iload caps from __ca_regs;
                #   invokespecial Closure_<fn>::<init>(I,...,I)V; pop ref
                #
                # Phase 2c structural-only: the resulting reference is
                # popped (not stored) because the existing register
                # convention is int[] only.  Phase 2c.5 will add the
                # parallel Object[] pool to retain the reference.
                self._emit_make_closure(builder, instruction)
                continue

            if instruction.opcode == IrOp.APPLY_CLOSURE:
                # APPLY_CLOSURE dst, closure_reg, num_args, arg0
                # → push closure (placeholder null until Phase 2c.5);
                #   build int[] args; invokeinterface Closure.apply([I)I;
                #   istore int into __ca_regs[dst]
                self._emit_apply_closure(builder, instruction)
                continue

            raise JvmBackendError(
                "Unsupported IR opcode in prototype backend: "
                f"{instruction.opcode}"
            )

        code = builder.assemble()
        access_flags = (
            ACC_PUBLIC | ACC_STATIC
            if region.name == self.program.entry_label
            else _ACC_PRIVATE | ACC_STATIC
        )
        return _MethodSpec(
            access_flags=access_flags,
            name=region.name,
            descriptor=_DESC_NOARGS_INT,
            code=code,
            max_stack=16,
            # JVM01: caller-saves convention uses JVM locals
            # 0..reg_count-1 as snapshot storage across CALL
            # boundaries.  Reserve at least ``reg_count`` locals so
            # the JVM verifier accepts the method.
            max_locals=reg_count,
        )

    def _build_main_method(self) -> _MethodSpec:
        builder = _BytecodeBuilder()
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self.program.entry_label, _DESC_NOARGS_INT),
        )
        builder.emit_opcode(_OP_POP)
        builder.emit_opcode(_OP_RETURN)
        return _MethodSpec(
            access_flags=ACC_PUBLIC | ACC_STATIC,
            name="main",
            descriptor=_DESC_MAIN,
            code=builder.assemble(),
            max_stack=1,
            max_locals=1,
        )

    def _encode_class_file(
        self,
        fields: list[_FieldSpec],
        methods: list[_MethodSpec],
    ) -> bytes:
        this_class_index = self.cp.class_ref(self.internal_name)
        super_class_index = self.cp.class_ref("java/lang/Object")

        field_bytes = b"".join(
            _u2(field.access_flags)
            + _u2(self.cp.utf8(field.name))
            + _u2(self.cp.utf8(field.descriptor))
            + _u2(0)
            for field in fields
        )

        code_name_index = self.cp.utf8("Code")
        method_bytes = b"".join(
            self._encode_method(method, code_name_index) for method in methods
        )

        return b"".join(
            [
                _u4(0xCAFEBABE),
                _u2(self.config.class_file_minor),
                _u2(self.config.class_file_major),
                _u2(self.cp.count),
                self.cp.encode(),
                _u2(ACC_PUBLIC | ACC_SUPER | _ACC_FINAL),
                _u2(this_class_index),
                _u2(super_class_index),
                _u2(0),
                _u2(len(fields)),
                field_bytes,
                _u2(len(methods)),
                method_bytes,
                _u2(0),
            ]
        )

    def _encode_method(self, method: _MethodSpec, code_name_index: int) -> bytes:
        code_attribute_body = b"".join(
            [
                _u2(method.max_stack),
                _u2(method.max_locals),
                _u4(len(method.code)),
                method.code,
                _u2(0),
                _u2(0),
            ]
        )
        code_attribute = b"".join(
            [
                _u2(code_name_index),
                _u4(len(code_attribute_body)),
                code_attribute_body,
            ]
        )
        return b"".join(
            [
                _u2(method.access_flags),
                _u2(self.cp.utf8(method.name)),
                _u2(self.cp.utf8(method.descriptor)),
                _u2(1),
                code_attribute,
            ]
        )


# ---------------------------------------------------------------------------
# JVM word-size constraints and supported opcode set
# ---------------------------------------------------------------------------
#
# The JVM uses 32-bit two's-complement integers (Java ``int``).
# Signed range: -2 147 483 648 (−2^31) to 2 147 483 647 (2^31 − 1).

_JVM_INT_MIN: int = -(1 << 31)   # -2 147 483 648
_JVM_INT_MAX: int =  (1 << 31) - 1  # 2 147 483 647

# The V1 JVM backend handles exactly these opcodes.  Any opcode absent from
# this set is rejected by validate_for_jvm() before code generation begins.
_JVM_SUPPORTED_OPCODES: frozenset[IrOp] = frozenset({
    IrOp.LABEL,
    IrOp.COMMENT,
    IrOp.NOP,
    IrOp.HALT,
    IrOp.RET,
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
    IrOp.MUL,
    IrOp.DIV,
    IrOp.CMP_EQ,
    IrOp.CMP_NE,
    IrOp.CMP_LT,
    IrOp.CMP_GT,
    IrOp.BRANCH_Z,
    IrOp.BRANCH_NZ,
    IrOp.CALL,
    IrOp.SYSCALL,
    # JVM02 Phase 2c — closures.
    IrOp.MAKE_CLOSURE,
    IrOp.APPLY_CLOSURE,
})


def validate_for_jvm(program: IrProgram) -> list[str]:
    """Inspect ``program`` for JVM backend incompatibilities without generating
    any bytecode.

    Checks performed:

    1. **Opcode support** — every opcode must appear in ``_JVM_SUPPORTED_OPCODES``.
       Opcodes that the V1 JVM backend does not handle (e.g. future IR
       extensions) are rejected with a precise diagnostic before any class-file
       bytes are produced.

    2. **Constant range** — every ``IrImmediate`` in a ``LOAD_IMM`` or
       ``ADD_IMM`` instruction must fit in a JVM 32-bit signed integer
       (−2 147 483 648 to 2 147 483 647).  The JVM stack is a 32-bit
       operand stack; constants outside this range cannot be represented as a
       JVM ``int`` and would require a ``long`` (64-bit) type, which the
       backend does not support.

    3. **SYSCALL number** — the V1 JVM backend wires up SYSCALL 1 (print byte)
       and SYSCALL 4 (read byte).  Any other syscall number is rejected.

    Args:
        program: The ``IrProgram`` to inspect.

    Returns:
        A list of human-readable error strings.  An empty list means the
        program is compatible with the JVM V1 backend.
    """
    errors: list[str] = []
    _SUPPORTED_SYSCALLS = {1, 4}

    for instr in program.instructions:
        op = instr.opcode

        # ── Rule 1: opcode must be in the supported set ─────────────────────
        if op not in _JVM_SUPPORTED_OPCODES:
            errors.append(
                f"unsupported opcode {op.name} in V1 JVM backend"
            )
            continue

        # ── Rule 2: constant range on LOAD_IMM and ADD_IMM ──────────────────
        if op in (IrOp.LOAD_IMM, IrOp.ADD_IMM):
            for operand in instr.operands:
                if isinstance(operand, IrImmediate):
                    v = operand.value
                    if not (_JVM_INT_MIN <= v <= _JVM_INT_MAX):
                        errors.append(
                            f"{op.name}: constant {v:,} overflows JVM 32-bit "
                            f"signed integer (valid range "
                            f"{_JVM_INT_MIN:,} to {_JVM_INT_MAX:,})"
                        )

        # ── Rule 3: SYSCALL number ───────────────────────────────────────────
        elif op == IrOp.SYSCALL:
            for operand in instr.operands:
                if isinstance(operand, IrImmediate) and operand.value not in _SUPPORTED_SYSCALLS:
                    errors.append(
                        f"unsupported SYSCALL {operand.value}: "
                        f"only SYSCALL numbers {sorted(_SUPPORTED_SYSCALLS)} "
                        f"are wired in the V1 JVM backend"
                    )
                    break

    return errors


def lower_ir_to_jvm_class_file(
    program: IrProgram,
    config: JvmBackendConfig,
) -> JVMClassArtifact:
    """Lower an IR program to a JVM class artifact.

    Runs ``validate_for_jvm`` as a pre-flight check before any bytecode is
    generated.  If the IR contains an unsupported opcode or an out-of-range
    constant, a ``JvmBackendError`` is raised immediately with a precise
    per-instruction diagnostic.
    """
    errors = validate_for_jvm(program)
    if errors:
        joined = "; ".join(errors)
        raise JvmBackendError(
            f"IR program failed JVM pre-flight validation "
            f"({len(errors)} error{'s' if len(errors) != 1 else ''}): {joined}"
        )
    return _JvmClassLowerer(program, config).lower()


# JVM02 Phase 2b — interface and multi-class API.

# JVM access flags for a public abstract interface (JVMS §4.1).
_ACC_INTERFACE: Final[int] = 0x0200
_ACC_ABSTRACT: Final[int] = 0x0400


def build_closure_interface_artifact() -> JVMClassArtifact:
    """Return the ``Closure`` interface as a ``JVMClassArtifact``.

    The interface declares one abstract method
    ``int apply(int[] args)``.  Closure subclasses (emitted in
    Phase 2c, one per lifted lambda) implement it.  ``APPLY_CLOSURE``
    sites lower to ``invokeinterface Closure.apply([I)I``, which
    HotSpot can monomorphise at warm-up.

    The class lives under
    ``coding_adventures.twig.runtime.Closure`` (see
    ``CLOSURE_INTERFACE_BINARY_NAME``) — a stable location every
    closure-aware artifact references.

    Phase 2b ships this as a standalone callable so that:
      * tests can verify the interface bytecode loads cleanly under
        real ``java`` (catches verifier complaints early);
      * the Phase 2d JAR-packaging step can include it without the
        full Phase 2c lowering being implemented yet.
    """
    interface_class_name = CLOSURE_INTERFACE_BINARY_NAME
    super_class_name = "java/lang/Object"
    method_name = CLOSURE_INTERFACE_METHOD_NAME
    method_descriptor = CLOSURE_INTERFACE_METHOD_DESCRIPTOR

    class_bytes = _build_interface_class_bytes(
        class_name=interface_class_name,
        super_class_name=super_class_name,
        abstract_methods=((method_name, method_descriptor),),
    )

    # Restore dotted form for ``JVMClassArtifact.class_name`` so the
    # ``class_filename`` property produces ``coding_adventures/twig/
    # runtime/Closure.class`` correctly.
    dotted_name = interface_class_name.replace("/", ".")
    return JVMClassArtifact(
        class_name=dotted_name,
        class_bytes=class_bytes,
        callable_labels=(),
        data_offsets={},
    )


def lower_ir_to_jvm_classes(
    program: IrProgram,
    config: JvmBackendConfig,
    *,
    include_closure_interface: bool = False,
) -> JVMMultiClassArtifact:
    """Lower ``program`` into one or more JVM class artifacts.

    Returns a ``JVMMultiClassArtifact`` whose ``classes[0]`` is
    always the main user class.  The ``Closure`` interface is
    appended when either:

    * ``include_closure_interface=True`` (callers opt in
      explicitly — the Phase 2b path), OR
    * ``config.closure_free_var_counts`` is non-empty (the
      program contains lifted lambdas, so the interface is
      required for ``invokeinterface`` dispatch).

    When closure regions are declared, one ``Closure_<name>``
    subclass is appended per lambda — fields per capture, ``.ctor``
    chaining into ``Object::.ctor()`` and storing captures, and
    an ``apply(int[])`` method that reads back the captures and
    runs the lifted body.

    Phase 2c v1 ships this as **structural-only**: the bytecode
    shape is verifier-correct, multi-class plumbing works, but
    end-to-end runtime semantics for the captured-state and
    ``Apply``-body data flow are wired in Phase 2c.5 (parallel
    ``Object[]`` register pool + cross-class register access).
    The placeholder ``apply`` body returns 0 so the class loads
    and its method-resolution path is exercised even without
    Phase 2c.5.
    """
    main = lower_ir_to_jvm_class_file(program, config)
    classes: list[JVMClassArtifact] = [main]

    needs_interface = include_closure_interface or bool(
        config.closure_free_var_counts
    )
    if needs_interface:
        classes.append(build_closure_interface_artifact())

    for closure_name, num_free in config.closure_free_var_counts.items():
        classes.append(
            build_closure_subclass_artifact(closure_name, num_free=num_free)
        )

    return JVMMultiClassArtifact(classes=tuple(classes))


def build_closure_subclass_artifact(
    closure_name: str,
    *,
    num_free: int,
) -> JVMClassArtifact:
    """Return one ``Closure_<closure_name>.class`` artifact.

    The class:

    * lives at ``coding_adventures/twig/runtime/Closure_<sanitised>``
      — same package as the ``Closure`` interface for cross-class
      simplicity.  IR labels with ``-``/``.``/``$`` are sanitised
      to ``_`` so they map onto valid JVM binary names.
    * implements ``coding_adventures/twig/runtime/Closure``.
    * has one ``private final int captI`` field per capture
      (``capt0``, ``capt1``, …).
    * has a ``.ctor(int, int, …)V`` that calls
      ``Object::.ctor()`` then stores each parameter into its
      corresponding capture field.
    * has a placeholder ``apply([I)I`` method that returns ``0``.

    The placeholder ``apply`` is the Phase 2c structural-only
    compromise: the class loads, ``Closure`` dispatch resolves to
    it, but the lifted lambda's actual body is not yet wired
    through.  Phase 2c.5 will fill in the body using either:

    1. A new lowering path that uses JVM locals for the IR
       register frame (instead of the main class's static
       ``__ca_regs`` field), OR
    2. Cross-class access to the main class's static helpers.

    Either approach requires substantial refactoring of the
    register-storage convention; doing it here would balloon the
    Phase 2c PR.
    """
    sanitised = _sanitise_class_name_segment(closure_name)
    package = "coding_adventures/twig/runtime"
    binary_name = f"{package}/Closure_{sanitised}"

    pool = _ConstantPoolBuilder()
    this_class_index = pool.class_ref(binary_name)
    super_class_index = pool.class_ref("java/lang/Object")
    interface_class_index = pool.class_ref(CLOSURE_INTERFACE_BINARY_NAME)
    object_ctor_ref = pool.method_ref(
        "java/lang/Object", "<init>", "()V",
    )

    # Field UTF8 + name+type entries.
    field_name_indices = [pool.utf8(f"capt{i}") for i in range(num_free)]
    field_descriptor_index = pool.utf8("I")
    field_refs = [
        pool.field_ref(binary_name, f"capt{i}", "I") for i in range(num_free)
    ]

    code_attribute_name_index = pool.utf8("Code")

    # ── ctor body ──────────────────────────────────────────────────────
    # ldarg.0; invokespecial Object::<init>()V
    # for each capture i: ldarg.0; ldarg{1+i}; putfield captI
    # return
    ctor_builder = _BytecodeBuilder()
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_u2_instruction(_OP_INVOKESPECIAL, object_ctor_ref)
    for i in range(num_free):
        ctor_builder.emit_opcode(_OP_ALOAD_0)
        # ldarg loads int — JVM has iload_1..3 short forms.
        if i + 1 <= 3:
            ctor_builder.emit_opcode(_OP_ILOAD_0 + (i + 1))
        else:
            ctor_builder.emit_u1_instruction(_OP_ILOAD, i + 1)
        ctor_builder.emit_u2_instruction(_OP_PUTFIELD, field_refs[i])
    ctor_builder.emit_opcode(_OP_RETURN)
    ctor_code = ctor_builder.assemble()
    ctor_descriptor = "(" + ("I" * num_free) + ")V"
    ctor_name_index = pool.utf8("<init>")
    ctor_descriptor_index = pool.utf8(ctor_descriptor)

    # ── apply body — placeholder: iconst_0; ireturn ──────────────────
    apply_builder = _BytecodeBuilder()
    apply_builder.emit_opcode(_OP_ICONST_0)
    apply_builder.emit_opcode(_OP_IRETURN)
    apply_code = apply_builder.assemble()
    apply_name_index = pool.utf8(CLOSURE_INTERFACE_METHOD_NAME)
    apply_descriptor_index = pool.utf8(CLOSURE_INTERFACE_METHOD_DESCRIPTOR)

    # Encode the class.
    pool_bytes = pool.encode()
    class_bytes_parts: list[bytes] = [
        b"\xca\xfe\xba\xbe",
        _u2(0),                      # minor_version
        _u2(49),                     # major_version
        _u2(pool.count),
        pool_bytes,
        _u2(ACC_PUBLIC | ACC_SUPER),
        _u2(this_class_index),
        _u2(super_class_index),
        _u2(1),                      # interfaces_count
        _u2(interface_class_index),
        _u2(num_free),               # fields_count
    ]
    for name_index in field_name_indices:
        class_bytes_parts.append(_u2(_ACC_PRIVATE | _ACC_FINAL))
        class_bytes_parts.append(_u2(name_index))
        class_bytes_parts.append(_u2(field_descriptor_index))
        class_bytes_parts.append(_u2(0))  # 0 attributes per field

    # Methods: ctor + apply.
    class_bytes_parts.append(_u2(2))  # methods_count

    # ctor
    class_bytes_parts.append(
        _build_method_info(
            access_flags=ACC_PUBLIC,
            name_index=ctor_name_index,
            descriptor_index=ctor_descriptor_index,
            code=ctor_code,
            max_stack=2,
            max_locals=1 + num_free,
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    # apply
    apply_access = ACC_PUBLIC
    class_bytes_parts.append(
        _build_method_info(
            access_flags=apply_access,
            name_index=apply_name_index,
            descriptor_index=apply_descriptor_index,
            code=apply_code,
            max_stack=1,
            max_locals=2,                 # this + int[] arg
            code_attribute_name_index=code_attribute_name_index,
        )
    )

    class_bytes_parts.append(_u2(0))  # 0 class attributes

    return JVMClassArtifact(
        class_name=binary_name.replace("/", "."),
        class_bytes=b"".join(class_bytes_parts),
        callable_labels=(closure_name,),
        data_offsets={},
    )


# Sanitise IR labels (which may contain ``_lambda_0``, ``-``, ``.``, ``$``)
# into JVM-safe class-name segments.  The JVMS §4.2.1 binary-name grammar
# allows ``[A-Za-z_$][A-Za-z0-9_$]*`` per segment.
_SANITISE_RE = re.compile(r"[^A-Za-z0-9_$]")


def _sanitise_class_name_segment(label: str) -> str:
    cleaned = _SANITISE_RE.sub("_", label)
    if cleaned and cleaned[0].isdigit():
        cleaned = "_" + cleaned
    return cleaned or "_"


def _build_method_info(
    *,
    access_flags: int,
    name_index: int,
    descriptor_index: int,
    code: bytes,
    max_stack: int,
    max_locals: int,
    code_attribute_name_index: int,
) -> bytes:
    """Build a ``method_info`` blob with one Code attribute.

    Layout per JVMS §4.6 + §4.7.3.
    """
    code_attribute_body = b"".join(
        [
            _u2(max_stack),
            _u2(max_locals),
            _u4(len(code)),
            code,
            _u2(0),  # exception_table_length
            _u2(0),  # attributes_count
        ]
    )
    code_attribute = b"".join(
        [
            _u2(code_attribute_name_index),
            _u4(len(code_attribute_body)),
            code_attribute_body,
        ]
    )
    return b"".join(
        [
            _u2(access_flags),
            _u2(name_index),
            _u2(descriptor_index),
            _u2(1),  # 1 attribute (Code)
            code_attribute,
        ]
    )


def _build_interface_class_bytes(
    *,
    class_name: str,
    super_class_name: str,
    abstract_methods: tuple[tuple[str, str], ...],
) -> bytes:
    """Hand-roll a JVM interface ``.class`` byte stream.

    ``build_minimal_class_file`` only knows how to emit a single
    concrete method with a Code attribute, but interface methods
    have no Code attribute and the class itself has the
    ``ACC_INTERFACE | ACC_ABSTRACT`` flags.  Open-coding the layout
    is small enough (~50 lines) that adding a new public API to
    ``jvm-class-file`` would be heavier than just emitting bytes
    directly here.

    Layout per JVMS §4.1:

        magic                       u4 = 0xCAFEBABE
        minor_version               u2 = 0
        major_version               u2 = 49 (Java 5; matches main class)
        constant_pool_count         u2
        constant_pool[]             cp_info...
        access_flags                u2 = ACC_PUBLIC | ACC_INTERFACE | ACC_ABSTRACT
        this_class                  u2 = CONSTANT_Class index
        super_class                 u2 = CONSTANT_Class for java/lang/Object
        interfaces_count            u2 = 0
        fields_count                u2 = 0
        methods_count               u2 = N
        method_info[]               method_info...
        attributes_count            u2 = 0
    """
    pool = _ConstantPoolBuilder()
    this_class_index = pool.class_ref(class_name)
    super_class_index = pool.class_ref(super_class_name)

    method_blobs: list[bytes] = []
    for name, descriptor in abstract_methods:
        method_access = ACC_PUBLIC | _ACC_ABSTRACT
        method_blobs.append(
            _u2(method_access)
            + _u2(pool.utf8(name))
            + _u2(pool.utf8(descriptor))
            + _u2(0)  # 0 attributes — abstract methods have no Code.
        )

    pool_bytes = pool.encode()
    access_flags = ACC_PUBLIC | _ACC_INTERFACE | _ACC_ABSTRACT
    return b"".join(
        [
            b"\xca\xfe\xba\xbe",
            _u2(0),                          # minor_version
            _u2(49),                         # major_version (Java 5)
            _u2(pool.count),                 # constant_pool_count
            pool_bytes,
            _u2(access_flags),
            _u2(this_class_index),
            _u2(super_class_index),
            _u2(0),                          # interfaces_count
            _u2(0),                          # fields_count
            _u2(len(method_blobs)),          # methods_count
            *method_blobs,
            _u2(0),                          # attributes_count
        ]
    )


def write_class_file(artifact: JVMClassArtifact, output_dir: str | Path) -> Path:
    """Write a generated class file into a classpath root."""

    root = Path(output_dir)
    relative_path = _validated_output_relative_path(artifact.class_filename)
    root.mkdir(parents=True, exist_ok=True)

    open_directory_flags = os.O_RDONLY
    if hasattr(os, "O_DIRECTORY"):
        open_directory_flags |= os.O_DIRECTORY
    if hasattr(os, "O_NOFOLLOW"):
        open_directory_flags |= os.O_NOFOLLOW

    open_file_flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    if hasattr(os, "O_NOFOLLOW"):
        open_file_flags |= os.O_NOFOLLOW

    directory_fds: list[int] = []
    try:
        current_fd = os.open(root, open_directory_flags)
        directory_fds.append(current_fd)

        # Traverse relative output components through directory FDs so later
        # writes cannot be redirected by swapping in a symlink after validation.
        for component in relative_path.parts[:-1]:
            with suppress(FileExistsError):
                os.mkdir(component, dir_fd=current_fd)
            try:
                next_fd = os.open(component, open_directory_flags, dir_fd=current_fd)
            except OSError as exc:
                raise JvmBackendError(
                    "class_filename contains a symlinked or invalid directory "
                    f"component: {component}"
                ) from exc
            directory_fds.append(next_fd)
            current_fd = next_fd

        try:
            file_fd = os.open(
                relative_path.name,
                open_file_flags,
                0o644,
                dir_fd=current_fd,
            )
        except OSError as exc:
            raise JvmBackendError(
                "class_filename points at a symlinked or invalid output file"
            ) from exc
        with os.fdopen(file_fd, "wb", closefd=True) as handle:
            handle.write(artifact.class_bytes)
    finally:
        for directory_fd in reversed(directory_fds):
            os.close(directory_fd)

    target = root / relative_path
    return target


def _u2(value: int) -> bytes:
    return int(value).to_bytes(2, byteorder="big", signed=False)


def _u4(value: int) -> bytes:
    return int(value).to_bytes(4, byteorder="big", signed=False)


def _as_label(operand: object, context: str) -> IrLabel:
    if not isinstance(operand, IrLabel):
        raise JvmBackendError(
            f"{context} must be an IrLabel, got {type(operand).__name__}"
        )
    return operand


def _as_register(operand: object, context: str) -> IrRegister:
    if not isinstance(operand, IrRegister):
        raise JvmBackendError(
            f"{context} must be an IrRegister, got {type(operand).__name__}"
        )
    return operand


def _as_immediate(operand: object, context: str) -> IrImmediate:
    if not isinstance(operand, IrImmediate):
        raise JvmBackendError(
            f"{context} must be an IrImmediate, got {type(operand).__name__}"
        )
    return operand


def _validated_output_relative_path(class_filename: str) -> Path:
    relative_path = Path(class_filename)
    if relative_path.is_absolute():
        raise JvmBackendError("class_filename escapes the requested output directory")
    if any(component in ("", ".", "..") for component in relative_path.parts):
        raise JvmBackendError("class_filename escapes the requested output directory")
    return relative_path
