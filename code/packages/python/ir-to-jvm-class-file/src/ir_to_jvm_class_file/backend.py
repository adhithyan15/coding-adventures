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
_OP_SWAP = 0x5F
_OP_ACONST_NULL = 0x01
_OP_CHECKCAST = 0xC0
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
_OP_ALOAD_1 = 0x2B         # load arg slot 1 (used in closure Apply forwarder)
_OP_ALOAD_2 = 0x2C         # load arg slot 2 (used in Cons ctor for tail param)
_OP_ALOAD_3 = 0x2D         # load arg slot 3 (used in obj-pool caller-saves)
_OP_ALOAD = 0x19           # generic aload (uses next byte as local index)
_OP_ASTORE_0 = 0x4B        # store ref to local 0 (used in obj-pool caller-saves)
_OP_ASTORE_2 = 0x4D        # store ref to local 2
_OP_ASTORE_3 = 0x4E        # store ref to local 3
_OP_ASTORE = 0x3A          # generic astore (uses next byte as local index)
_OP_AASTORE = 0x53
_OP_AALOAD = 0x32
_OP_NOP = 0x00
_OP_ARETURN = 0xB0          # return a reference (used by Symbol.intern)
_OP_INSTANCEOF = 0xC1       # for IS_PAIR / IS_SYMBOL
_OP_IF_ACMPNE = 0xA6        # for IS_NULL identity test (vs Nil.INSTANCE)
_OP_IFNONNULL = 0xC7        # for Symbol.intern fast-path branch
_OP_ASTORE_1 = 0x4C         # store ref to local 1 (Symbol.intern temp)

_ATYPE_INT = 10
_ATYPE_BYTE = 8

_DESC_INT = "I"
_DESC_VOID = "V"
_DESC_INT_ARRAY = "[I"
_DESC_BYTE_ARRAY = "[B"
_DESC_OBJECT = "Ljava/lang/Object;"
_DESC_OBJECT_ARRAY = "[Ljava/lang/Object;"
_DESC_CLOSURE_INTERFACE = "Lcoding_adventures/twig/runtime/Closure;"
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

# JVM02 Phase 2c.5 — captures-first IR register layout for lifted
# lambda bodies (matches BEAM/CLR closure conventions).
_REG_PARAM_BASE: Final[int] = 2
_CLR_CLOSURE_EXPLICIT_ARITY: Final[int] = 1  # Arity-1 closures only in v1.


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

# TW03 Phase 3b — heap-primitive runtime classes.
#
# Three additional classes ride alongside the closure runtime under
# the same ``coding_adventures.twig.runtime`` package so the JAR's
# class layout stays predictable.  Each is auto-included by
# ``lower_ir_to_jvm_classes`` whenever the program uses any heap
# opcode.
#
# Cons   — pair of ``int head`` and ``Object tail``.  Phase 3b v1
#          targets list-of-ints (the spec acceptance criterion);
#          a follow-up phase widens ``head`` to ``Object`` for fully
#          polymorphic cells once typed-register inference covers
#          the head slot.
# Symbol — interned identifier; ``String name`` field plus a static
#          ``intern(String) Symbol`` method backed by a static
#          ``HashMap<String,Symbol>``.  Single-threaded — Twig has no
#          concurrency surface today.
# Nil    — singleton sentinel; one ``public static final Nil INSTANCE``
#          plus a ``private`` no-arg ctor.  ``IS_NULL`` lowers to
#          identity comparison against ``Nil.INSTANCE``.
CONS_BINARY_NAME: Final = "coding_adventures/twig/runtime/Cons"
SYMBOL_BINARY_NAME: Final = "coding_adventures/twig/runtime/Symbol"
NIL_BINARY_NAME: Final = "coding_adventures/twig/runtime/Nil"
_DESC_CONS = f"L{CONS_BINARY_NAME};"
_DESC_SYMBOL = f"L{SYMBOL_BINARY_NAME};"
_DESC_NIL = f"L{NIL_BINARY_NAME};"
_DESC_STRING = "Ljava/lang/String;"
_DESC_HASHMAP = "Ljava/util/HashMap;"
_DESC_OBJECT_TO_OBJECT = "(Ljava/lang/Object;)Ljava/lang/Object;"
_DESC_OBJECT_OBJECT_TO_OBJECT = (
    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"
)


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
        # JVM02 Phase 2c.5 — parallel object pool for closure refs.
        # Allocated only when ``closure_free_var_counts`` is non-empty
        # so non-closure programs see no extra fields / clinit work.
        self._objreg_field = "__ca_objregs"
        self._helper_reg_get = "__ca_regGet"
        self._helper_reg_set = "__ca_regSet"
        self._helper_mem_load_byte = "__ca_memLoadByte"
        self._helper_mem_store_byte = "__ca_memStoreByte"
        self._helper_load_word = "__ca_loadWord"
        self._helper_store_word = "__ca_storeWord"
        self._helper_syscall = "__ca_syscall"
        # Set in ``lower()`` once we know if any heap or closure op
        # appears.  Default False so methods that don't need obj-pool
        # caller-saves emit the original int-only sequence.
        self._needs_objregs = False
        # Per-region set of registers used as object refs.  Set by
        # ``_emit_region_instructions`` before each region's body
        # emission.  Used by ADD_IMM-0 emission to gate obj-slot
        # propagation (see ``_collect_region_obj_regs`` for why).
        self._region_obj_regs: set[int] = set()

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

        # JVM02 Phase 2c.5: when any closure is declared, allocate a
        # parallel ``Object[]`` static field for closure refs and
        # initialize it in <clinit>.  Non-closure programs see zero
        # extra emission.
        # TW03 Phase 3b extends this trigger: heap ops (cons / symbol /
        # nil) also need the parallel object pool, so any HEAP_OPCODES
        # opcode in the program flips the same flag.
        closure_region_names = set(self.config.closure_free_var_counts)
        has_closures = bool(closure_region_names)
        uses_heap = any(
            i.opcode in _HEAP_OPCODES for i in self.program.instructions
        )
        needs_objregs = has_closures or uses_heap
        # Stash on self so CALL emission can pick the right caller-saves
        # discipline (int-only vs int + obj).
        self._needs_objregs = needs_objregs

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
        if needs_objregs:
            fields.append(
                _FieldSpec(
                    _ACC_PRIVATE | ACC_STATIC,
                    self._objreg_field,
                    _DESC_OBJECT_ARRAY,
                )
            )

        methods = [
            self._build_class_initializer(reg_count, data_offsets, needs_objregs),
            self._build_reg_get_method(),
            self._build_reg_set_method(),
            self._build_mem_load_byte_method(),
            self._build_mem_store_byte_method(),
            self._build_load_word_method(),
            self._build_store_word_method(),
            self._build_syscall_method(),
        ]
        # JVM02 Phase 2c.5: closure regions (lifted lambdas) are NOW
        # emitted as PUBLIC static methods on the main class so the
        # closure subclass's ``Apply`` method can invoke them via
        # ``invokestatic`` from a different package.  Their arity is
        # widened to ``num_free + explicit_arity`` (capture params +
        # explicit lambda params) and a JVM-args → __ca_regs prologue
        # gets prepended so the existing IR-body emitter can run
        # unchanged.
        for region in callable_regions:
            if region.name in closure_region_names:
                num_free = self.config.closure_free_var_counts[region.name]
                methods.append(
                    self._build_lifted_lambda_method(
                        region, reg_count, num_free,
                    )
                )
            else:
                methods.append(
                    self._build_callable_method(region, reg_count)
                )
        if self.config.emit_main_wrapper:
            methods.append(self._build_main_method())

        class_bytes = self._encode_class_file(fields, methods)
        return JVMClassArtifact(
            class_name=self.config.class_name,
            class_bytes=class_bytes,
            # JVM02 Phase 2c.5: lifted lambda regions ARE on the
            # main class now (with widened arity), so include them
            # in ``callable_labels`` so JAR builders / test
            # harnesses can iterate every emitted method.
            callable_labels=tuple(
                region.name for region in callable_regions
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
    # TW03 Phase 3 follow-up — caller-saves for the obj register pool
    # ------------------------------------------------------------------
    #
    # The JVM01 caller-saves above only snapshot the int register pool
    # (``__ca_regs``).  Programs that use heap primitives or closures
    # ALSO have an object register pool (``__ca_objregs``) holding cons
    # cells, symbols, nil sentinels, and closure refs.  Without obj-pool
    # caller-saves, recursion through any heap-typed register (e.g.
    # ``length`` walking a cons list with the tail in an obj-typed reg)
    # clobbers the caller's obj reg when the recursive call's body
    # writes the same slot.
    #
    # The fix mirrors the int-pool pattern but uses JVM local slots
    # ``reg_count..2*reg_count-1`` and the ``aload``/``astore`` +
    # ``aaload``/``aastore`` opcode pair (verifier types these locals
    # as ``Object`` independently of the int locals 0..reg_count-1).
    #
    # Like the int pool, the restore skips index 1 — the callee's
    # return value lives in ``__ca_regs[1]`` (an int).  The obj pool
    # has no analogous "return value" slot today; restoring all entries
    # is correct because closure/heap calls return ints.

    def _emit_caller_save_objregs(
        self, builder: _BytecodeBuilder, reg_count: int
    ) -> None:
        """Snapshot ``__ca_objregs[0..reg_count-1]`` → JVM ref locals
        ``reg_count..2*reg_count-1``."""
        for reg_idx in range(reg_count):
            builder.emit_u2_instruction(
                _OP_GETSTATIC,
                self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
            )
            self._emit_push_int(builder, reg_idx)
            builder.emit_opcode(_OP_AALOAD)
            self._emit_astore(builder, reg_count + reg_idx)

    def _emit_caller_restore_objregs(
        self, builder: _BytecodeBuilder, reg_count: int
    ) -> None:
        """Write the saved JVM ref locals back into ``__ca_objregs``.

        Skips index 1 — same convention as the int-pool restore.
        Functions that return an object reference (e.g. ``make_adder``
        returning a closure) put the result in ``__ca_objregs[1]``,
        and we want the caller to see the new value, not the
        saved-pre-call one.
        """
        for reg_idx in range(reg_count):
            if reg_idx == 1:
                continue  # preserve callee's object return value
            builder.emit_u2_instruction(
                _OP_GETSTATIC,
                self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
            )
            self._emit_push_int(builder, reg_idx)
            self._emit_aload(builder, reg_count + reg_idx)
            builder.emit_opcode(_OP_AASTORE)

    def _collect_region_obj_regs(
        self, region: _CallableRegion,
    ) -> set[int]:
        """Compute the IR-register indices that ``region`` uses as
        object refs (cons / symbol / nil / closure).

        Used by ADD_IMM-0 emission to gate obj-slot propagation:
        only copy ``__ca_objregs[src] → __ca_objregs[dst]`` when
        ``src`` actually holds an object in this region's
        type-flow.  Without this gate the canonical move idiom
        would propagate junk (zeros) into the obj pool whenever
        an int-typed source gets copied — and because
        ``__ca_objregs`` is a SHARED static array, that junk
        clobbers obj refs other parts of the program rely on.

        Three sources of "obj-ness" contribute:

        * **Writes** that produce object refs: MAKE_CLOSURE,
          MAKE_CONS, MAKE_SYMBOL, LOAD_NIL, CDR.  Direct producers.
        * **Reads** by ops that consume an object operand: CAR,
          CDR, IS_NULL, IS_PAIR, IS_SYMBOL, MAKE_CONS-tail,
          APPLY_CLOSURE-closure_reg.  Catches obj-typed parameter
          slots that the body only ever reads (mirrors the CLR
          backend's obj-source-read inference).
        * **Back-prop** through ADD_IMM-0 (the move idiom): if
          dst is obj-typed, src is too.  Iterated to a fixed
          point.  Catches the param→holding-reg copy at the top
          of the body.
        """
        obj_regs: set[int] = set()
        for instr in region.instructions:
            op = instr.opcode
            if op in (
                IrOp.MAKE_CLOSURE, IrOp.MAKE_CONS, IrOp.MAKE_SYMBOL,
                IrOp.LOAD_NIL, IrOp.CDR,
            ):
                if instr.operands and isinstance(
                    instr.operands[0], IrRegister,
                ):
                    obj_regs.add(instr.operands[0].index)
            if op in (
                IrOp.CAR, IrOp.CDR, IrOp.IS_NULL, IrOp.IS_PAIR, IrOp.IS_SYMBOL,
            ):
                if (
                    len(instr.operands) >= 2
                    and isinstance(instr.operands[1], IrRegister)
                ):
                    obj_regs.add(instr.operands[1].index)
            if op is IrOp.MAKE_CONS:
                if (
                    len(instr.operands) >= 3
                    and isinstance(instr.operands[2], IrRegister)
                ):
                    obj_regs.add(instr.operands[2].index)
            if op is IrOp.APPLY_CLOSURE:
                if (
                    len(instr.operands) >= 2
                    and isinstance(instr.operands[1], IrRegister)
                ):
                    obj_regs.add(instr.operands[1].index)
        # Back-prop through ADD_IMM-0 to a fixed point.
        changed = True
        cap = len(region.instructions) + 4
        while changed and cap > 0:
            changed = False
            cap -= 1
            for instr in region.instructions:
                if instr.opcode is not IrOp.ADD_IMM:
                    continue
                if len(instr.operands) < 3:
                    continue
                ops = instr.operands
                if not (
                    isinstance(ops[0], IrRegister)
                    and isinstance(ops[1], IrRegister)
                    and isinstance(ops[2], IrImmediate)
                    and ops[2].value == 0
                ):
                    continue
                if ops[0].index in obj_regs and ops[1].index not in obj_regs:
                    obj_regs.add(ops[1].index)
                    changed = True
        return obj_regs

    def _emit_aload(self, builder: _BytecodeBuilder, index: int) -> None:
        """Load an object reference from JVM local slot ``index``."""
        if 0 <= index <= 3:
            builder.emit_opcode(_OP_ALOAD_0 + index)
        else:
            builder.emit_u1_instruction(_OP_ALOAD, index)

    def _emit_astore(self, builder: _BytecodeBuilder, index: int) -> None:
        """Store an object reference into JVM local slot ``index``."""
        if 0 <= index <= 3:
            builder.emit_opcode(_OP_ASTORE_0 + index)
        else:
            builder.emit_u1_instruction(_OP_ASTORE, index)

    # ------------------------------------------------------------------
    # JVM02 Phase 2c — closure op emission (structural)
    # ------------------------------------------------------------------

    def _emit_make_closure(
        self,
        builder: _BytecodeBuilder,
        instruction: IrInstruction,
    ) -> None:
        """Emit ``new Closure_<fn>; dup; iload caps; invokespecial
        ctor; aastore __ca_objregs[dst]``.

        Phase 2c.5: the new reference is now stored into the
        parallel ``Object[]`` pool's slot ``dst`` so APPLY_CLOSURE
        and ADD_IMM-mov can retrieve it later.
        """
        if len(instruction.operands) < 3:
            raise JvmBackendError(
                f"MAKE_CLOSURE expects at least 3 operands "
                f"(dst, fn_label, num_captured, capt...), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "MAKE_CLOSURE dst")
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

        # Phase 2c.5: build the closure + store into __ca_objregs.
        # Emission shape:
        #   getstatic __ca_objregs           ; push the static array
        #   ldc dst                          ; push array index
        #   new Closure_X                    ; allocate
        #   dup                              ; reference for ctor
        #   ldloc capt0..captN-1 (via __ca_regGet)
        #   invokespecial Closure_X.<init>(I,...,I)V
        #   aastore                          ; objregs[dst] = ref
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
        )
        self._emit_push_int(builder, dst.index)
        builder.emit_u2_instruction(
            _OP_NEW, self.cp.class_ref(closure_internal),
        )
        builder.emit_opcode(_OP_DUP)
        for i in range(num_captured):
            capt_reg = _as_register(
                instruction.operands[3 + i], f"MAKE_CLOSURE capture {i}",
            )
            self._emit_reg_get(builder, capt_reg.index)
        builder.emit_u2_instruction(
            _OP_INVOKESPECIAL,
            self.cp.method_ref(closure_internal, "<init>", ctor_descriptor),
        )
        builder.emit_opcode(_OP_AASTORE)

    def _emit_apply_closure(
        self,
        builder: _BytecodeBuilder,
        instruction: IrInstruction,
    ) -> None:
        """Emit closure-apply bytecode.

        Phase 2c.5: the closure reference is now read from
        ``__ca_objregs[closure_reg]`` (instead of the placeholder
        ``aconst_null`` Phase 2c shipped with), so the call
        actually dispatches to the lifted lambda's body.  The
        ``int`` return is stored into ``__ca_regs[dst]``.
        """
        if len(instruction.operands) < 3:
            raise JvmBackendError(
                f"APPLY_CLOSURE expects at least 3 operands "
                f"(dst, closure, num_args, args...), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "APPLY_CLOSURE dst")
        closure_reg = _as_register(
            instruction.operands[1], "APPLY_CLOSURE closure_reg",
        )
        num_args = _as_immediate(
            instruction.operands[2], "APPLY_CLOSURE num_args",
        ).value

        if len(instruction.operands) != 3 + num_args:
            raise JvmBackendError(
                f"APPLY_CLOSURE: num_args={num_args} but "
                f"{len(instruction.operands) - 3} arg operands provided"
            )

        # Phase 2c.5: load the closure ref from __ca_objregs[closure_reg].
        # The aaload returns Object so we checkcast to the Closure
        # interface type before invokeinterface.
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
        )
        self._emit_push_int(builder, closure_reg.index)
        builder.emit_opcode(_OP_AALOAD)
        builder.emit_u2_instruction(
            _OP_CHECKCAST,
            self.cp.class_ref(CLOSURE_INTERFACE_BINARY_NAME),
        )
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
        # Phase 2c.5: store the int return value into __ca_regs[dst]
        # via the existing helper.  Stack effect: invokeinterface
        # leaves int on top; __ca_regSet expects (int idx, int val).
        # We need idx UNDER val on the stack, so push idx first then
        # swap.  Easier: emit a small helper sequence.
        # Stack now: [int_result]
        # Goal:     [] with __ca_regs[dst.index] = int_result
        # We can avoid swap by computing idx into a local first OR
        # by structuring as: ldc dst; swap; invokestatic regSet.
        # `swap` swaps two single-slot stack entries (both ints) —
        # safe here since both are int32.
        self._emit_push_int(builder, dst.index)
        builder.emit_opcode(_OP_SWAP)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
        )
        # Closure-returning closure: when dst is obj-typed in this
        # region (e.g. ((mk2 a) b) where the inner result is itself
        # a closure that gets passed to a third APPLY_CLOSURE), the
        # callee's RET propagated the obj ref into __ca_objregs[1]
        # via the lifted lambda's ADD_IMM-0 obj propagation.  Copy
        # __ca_objregs[1] → __ca_objregs[dst] so the next ADD_IMM-0
        # / APPLY_CLOSURE can pick it up.
        #
        # Without this, IClosure.apply([I)I's int return type drops
        # the obj-slot propagation chain at the call boundary and
        # 3-deep curries like (((mk2 a) b) c) NPE on the second
        # APPLY_CLOSURE.
        if dst.index in self._region_obj_regs:
            builder.emit_u2_instruction(
                _OP_GETSTATIC,
                self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
            )
            self._emit_push_int(builder, dst.index)
            builder.emit_u2_instruction(
                _OP_GETSTATIC,
                self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
            )
            self._emit_push_int(builder, 1)
            builder.emit_opcode(_OP_AALOAD)
            builder.emit_opcode(_OP_AASTORE)

    # ------------------------------------------------------------------
    # TW03 Phase 3b — heap-primitive lowering (cons / symbol / nil)
    # ------------------------------------------------------------------
    #
    # All eight heap ops route reference values through the parallel
    # ``__ca_objregs`` array (allocated in <clinit> when ``needs_objregs``
    # is true) so they sit alongside closure references in the same
    # object pool.  Type-test ops (IS_NULL / IS_PAIR / IS_SYMBOL) write
    # their 0/1 result into the int pool ``__ca_regs[dst]`` so the
    # downstream BRANCH_Z / BRANCH_NZ doesn't need to know the value
    # came from a reference test.
    #
    # Phase 3b v1 limitation: ``Cons.head`` is typed ``int`` and
    # ``Cons.tail`` is typed ``Object``.  This matches the spec
    # acceptance criterion (``length`` of a list-of-ints) while keeping
    # the typed-register inference work scoped to a follow-up.
    # ``MAKE_CONS head_reg`` is read from ``__ca_regs``; ``tail_reg`` is
    # read from ``__ca_objregs``.  ``CAR`` writes to ``__ca_regs[dst]``;
    # ``CDR`` writes to ``__ca_objregs[dst]``.

    def _emit_objreg_load(
        self, builder: _BytecodeBuilder, reg_index: int,
    ) -> None:
        """Push ``__ca_objregs[reg_index]`` (an ``Object`` ref) onto
        the stack."""
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
        )
        self._emit_push_int(builder, reg_index)
        builder.emit_opcode(_OP_AALOAD)

    def _emit_objreg_store_top_ref(
        self, builder: _BytecodeBuilder, dst_index: int,
    ) -> None:
        """Pop the ref currently on top of the stack into
        ``__ca_objregs[dst_index]``.

        Stack before: ``[..., ref]`` — after: ``[...]``.

        Implementation: stash the ref into ``__ca_objregs[dst]`` via
        ``getstatic; swap; ldc dst; swap; aastore`` would be ideal but
        ``swap`` only handles single-slot values.  We instead pre-stage
        the array+index *underneath* the ref by reorganising the stack
        with a local-variable spill is verifier-safe but expensive;
        the cheap pattern is:

          [ref]                            ; entry
          getstatic __ca_objregs           ; → [ref, arr]
          swap                             ; → [arr, ref]
          ldc dst                          ; → [arr, ref, dst]
          swap                             ; → [arr, dst, ref]
          aastore                          ; → []

        Three single-slot swaps — verifier-clean and stack-effect-correct
        because ``ref`` is one slot.
        """
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
        )
        builder.emit_opcode(_OP_SWAP)
        self._emit_push_int(builder, dst_index)
        builder.emit_opcode(_OP_SWAP)
        builder.emit_opcode(_OP_AASTORE)

    def _emit_intreg_store_top_int(
        self, builder: _BytecodeBuilder, dst_index: int,
    ) -> None:
        """Pop the int currently on top of the stack into
        ``__ca_regs[dst_index]``.  Mirrors the pattern used by
        APPLY_CLOSURE return-value handling above."""
        self._emit_push_int(builder, dst_index)
        builder.emit_opcode(_OP_SWAP)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
        )

    def _emit_make_cons(
        self, builder: _BytecodeBuilder, instruction: IrInstruction,
    ) -> None:
        """Lower ``MAKE_CONS dst, head_reg, tail_reg``.

        Heterogeneous-cons: head is now ``Object``-typed (was
        ``int``).  When the source register is obj-typed in the
        current region, load directly from the obj pool.  When
        int-typed, read the int and box via
        ``Integer.valueOf(int) Integer``.

        Tail is always ``Object`` and read from the obj pool —
        unchanged from the prior list-of-ints design.
        """
        if len(instruction.operands) != 3:
            raise JvmBackendError(
                f"MAKE_CONS expects 3 operands (dst, head, tail), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "MAKE_CONS dst")
        head_reg = _as_register(instruction.operands[1], "MAKE_CONS head")
        tail_reg = _as_register(instruction.operands[2], "MAKE_CONS tail")
        builder.emit_u2_instruction(_OP_NEW, self.cp.class_ref(CONS_BINARY_NAME))
        builder.emit_opcode(_OP_DUP)
        # Head: load from obj slot if obj-typed in this region; else
        # load int and box to Integer.
        if head_reg.index in self._region_obj_regs:
            self._emit_objreg_load(builder, head_reg.index)
        else:
            self._emit_reg_get(builder, head_reg.index)
            # Integer.valueOf(int) → Integer (which IS-A Object).
            builder.emit_u2_instruction(
                _OP_INVOKESTATIC,
                self.cp.method_ref(
                    "java/lang/Integer", "valueOf",
                    "(I)Ljava/lang/Integer;",
                ),
            )
        self._emit_objreg_load(builder, tail_reg.index)
        ctor_descriptor = "(" + _DESC_OBJECT + _DESC_OBJECT + ")V"
        builder.emit_u2_instruction(
            _OP_INVOKESPECIAL,
            self.cp.method_ref(CONS_BINARY_NAME, "<init>", ctor_descriptor),
        )
        # Stack: [ref] → store into __ca_objregs[dst]
        self._emit_objreg_store_top_ref(builder, dst.index)

    def _emit_car(
        self, builder: _BytecodeBuilder, instruction: IrInstruction,
    ) -> None:
        """Lower ``CAR dst, src`` — read ``Cons.head``.

        Heterogeneous-cons: head is ``Object``-typed.  If dst is
        obj-typed in this region (e.g. ``(car (cons 'foo nil))``
        where the car flows to another obj op), store the ref
        directly into the obj slot.  If int-typed (the common
        list-of-ints case), unwrap via
        ``Integer.intValue() int`` after a checkcast to Integer.
        """
        if len(instruction.operands) != 2:
            raise JvmBackendError(
                f"CAR expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "CAR dst")
        src = _as_register(instruction.operands[1], "CAR src")
        self._emit_objreg_load(builder, src.index)
        builder.emit_u2_instruction(
            _OP_CHECKCAST, self.cp.class_ref(CONS_BINARY_NAME),
        )
        builder.emit_u2_instruction(
            _OP_GETFIELD,
            self.cp.field_ref(CONS_BINARY_NAME, "head", _DESC_OBJECT),
        )
        # Stack: [Object head]
        if dst.index in self._region_obj_regs:
            # Direct obj store — head was an obj ref (cons / symbol /
            # nil / closure).  No unwrap needed.
            self._emit_objreg_store_top_ref(builder, dst.index)
        else:
            # head is a boxed Integer (or unsafe int read of a
            # non-int head — well-formed Twig avoids this).  Cast
            # and unwrap.
            builder.emit_u2_instruction(
                _OP_CHECKCAST, self.cp.class_ref("java/lang/Integer"),
            )
            builder.emit_u2_instruction(
                _OP_INVOKEVIRTUAL,
                self.cp.method_ref(
                    "java/lang/Integer", "intValue", "()I",
                ),
            )
            self._emit_intreg_store_top_int(builder, dst.index)

    def _emit_cdr(
        self, builder: _BytecodeBuilder, instruction: IrInstruction,
    ) -> None:
        """Lower ``CDR dst, src`` — read ``Cons.tail`` (Object)."""
        if len(instruction.operands) != 2:
            raise JvmBackendError(
                f"CDR expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "CDR dst")
        src = _as_register(instruction.operands[1], "CDR src")
        self._emit_objreg_load(builder, src.index)
        builder.emit_u2_instruction(
            _OP_CHECKCAST, self.cp.class_ref(CONS_BINARY_NAME),
        )
        builder.emit_u2_instruction(
            _OP_GETFIELD,
            self.cp.field_ref(CONS_BINARY_NAME, "tail", _DESC_OBJECT),
        )
        # Stack: [ref] → __ca_objregs[dst].
        self._emit_objreg_store_top_ref(builder, dst.index)

    def _emit_is_null(
        self, builder: _BytecodeBuilder, instruction: IrInstruction,
    ) -> None:
        """Lower ``IS_NULL dst, src`` — identity test against
        ``Nil.INSTANCE``.

        Bytecode:
          aload src; getstatic Nil.INSTANCE
          if_acmpne <NOT_NULL>
            iconst_1; goto END
          NOT_NULL:
            iconst_0
          END:
            → __ca_regs[dst]
        """
        if len(instruction.operands) != 2:
            raise JvmBackendError(
                f"IS_NULL expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "IS_NULL dst")
        src = _as_register(instruction.operands[1], "IS_NULL src")
        not_null_label = self._fresh_label("is_null_no")
        end_label = self._fresh_label("is_null_end")
        self._emit_objreg_load(builder, src.index)
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self.cp.field_ref(NIL_BINARY_NAME, "INSTANCE", _DESC_NIL),
        )
        builder.emit_branch(_OP_IF_ACMPNE, not_null_label)
        builder.emit_opcode(_OP_ICONST_0 + 1)  # iconst_1
        builder.emit_branch(_OP_GOTO, end_label)
        builder.mark(not_null_label)
        builder.emit_opcode(_OP_ICONST_0)
        builder.mark(end_label)
        self._emit_intreg_store_top_int(builder, dst.index)

    def _emit_instanceof_test(
        self,
        builder: _BytecodeBuilder,
        instruction: IrInstruction,
        target_internal_name: str,
        opname: str,
    ) -> None:
        """Shared lowering for IS_PAIR / IS_SYMBOL.

        ``instanceof`` already returns 0 or 1 — feed straight into
        ``__ca_regs[dst]``."""
        if len(instruction.operands) != 2:
            raise JvmBackendError(
                f"{opname} expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], f"{opname} dst")
        src = _as_register(instruction.operands[1], f"{opname} src")
        self._emit_objreg_load(builder, src.index)
        builder.emit_u2_instruction(
            _OP_INSTANCEOF, self.cp.class_ref(target_internal_name),
        )
        self._emit_intreg_store_top_int(builder, dst.index)

    def _emit_make_symbol(
        self, builder: _BytecodeBuilder, instruction: IrInstruction,
    ) -> None:
        """Lower ``MAKE_SYMBOL dst, name_label`` — call
        ``Symbol.intern("<name>")`` and store the result ref."""
        if len(instruction.operands) != 2:
            raise JvmBackendError(
                f"MAKE_SYMBOL expects 2 operands (dst, name_label), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "MAKE_SYMBOL dst")
        name_label = _as_label(
            instruction.operands[1], "MAKE_SYMBOL name_label",
        )
        # ldc "<name>"; invokestatic Symbol.intern(String) Symbol
        string_index = self.cp.string(name_label.name)
        if string_index <= 0xFF:
            builder.emit_u1_instruction(_OP_LDC, string_index)
        else:
            builder.emit_u2_instruction(_OP_LDC_W, string_index)
        builder.emit_u2_instruction(
            _OP_INVOKESTATIC,
            self.cp.method_ref(
                SYMBOL_BINARY_NAME, "intern",
                f"({_DESC_STRING}){_DESC_SYMBOL}",
            ),
        )
        self._emit_objreg_store_top_ref(builder, dst.index)

    def _emit_load_nil(
        self, builder: _BytecodeBuilder, instruction: IrInstruction,
    ) -> None:
        """Lower ``LOAD_NIL dst`` — getstatic ``Nil.INSTANCE`` and
        store the singleton ref into ``__ca_objregs[dst]``."""
        if len(instruction.operands) != 1:
            raise JvmBackendError(
                f"LOAD_NIL expects 1 operand (dst), got "
                f"{len(instruction.operands)}"
            )
        dst = _as_register(instruction.operands[0], "LOAD_NIL dst")
        builder.emit_u2_instruction(
            _OP_GETSTATIC,
            self.cp.field_ref(NIL_BINARY_NAME, "INSTANCE", _DESC_NIL),
        )
        self._emit_objreg_store_top_ref(builder, dst.index)

    def _build_class_initializer(
        self,
        reg_count: int,
        data_offsets: dict[str, int],
        needs_objregs: bool = False,
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

        # JVM02 Phase 2c.5 / TW03 Phase 3b: parallel ``Object[]`` for
        # closure refs and heap values (cons / symbol / nil).
        # ``anewarray Object`` allocates a fresh ``Object[reg_count]``
        # initialised to all-null; the cells get populated by
        # MAKE_CLOSURE / MAKE_CONS / MAKE_SYMBOL / LOAD_NIL /
        # ADD_IMM-mov as the program runs.
        if needs_objregs:
            self._emit_push_int(builder, reg_count)
            builder.emit_u2_instruction(
                _OP_ANEWARRAY,
                self.cp.class_ref("java/lang/Object"),
            )
            builder.emit_u2_instruction(
                _OP_PUTSTATIC,
                self._field_ref(self._objreg_field, _DESC_OBJECT_ARRAY),
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

    def _build_lifted_lambda_method(
        self,
        region: _CallableRegion,
        reg_count: int,
        num_free: int,
    ) -> _MethodSpec:
        """JVM02 Phase 2c.5 — emit a lifted lambda's IR body as a
        PUBLIC static method on the main class.

        Two key differences from a normal callable:

        1. The descriptor takes ``num_free + explicit_arity`` JVM
           int args (instead of zero), so the closure subclass's
           ``Apply`` method can ``invokestatic`` it from a
           different package without poking at the main class's
           private static ``__ca_regs`` array.
        2. A prologue copies each ldarg.i → ``__ca_regs[REG_PARAM_BASE+i]``
           so the existing IR-body emitter — which reads/writes
           the static array via the ``__ca_regGet`` / ``__ca_regSet``
           helpers — runs unchanged.
        """
        explicit_arity = _CLR_CLOSURE_EXPLICIT_ARITY  # currently 1
        total_arity = num_free + explicit_arity

        builder = _BytecodeBuilder()
        # Prologue: ldarg.i; invokestatic __ca_regSet(REG_PARAM_BASE+i, val)
        # The lambda body's IR uses captures-first layout: r2 holds
        # capt0, r3 holds capt1, ..., r{2+num_free} holds explicit
        # arg 0.  This matches what BEAM/CLR backends use.
        for i in range(total_arity):
            self._emit_push_int(builder, _REG_PARAM_BASE + i)
            self._emit_jvm_iload_arg(builder, i)
            builder.emit_u2_instruction(
                _OP_INVOKESTATIC,
                self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
            )

        # Run the IR body (re-uses _build_callable_method's
        # instruction-emission loop).  The body's RET reads
        # __ca_regs[1] and ireturns.
        self._emit_callable_body(builder, region, reg_count)

        descriptor = "(" + "I" * total_arity + ")I"
        # max_locals = total_arity (for ldarg) + caller-save snapshot
        # storage (reg_count for int pool, +reg_count again for the obj
        # pool when the program uses heap/closures — see
        # _emit_caller_save_objregs).
        snapshot_locals = (2 * reg_count) if self._needs_objregs else reg_count
        return _MethodSpec(
            access_flags=ACC_PUBLIC | ACC_STATIC,
            name=region.name,
            descriptor=descriptor,
            code=builder.assemble(),
            max_stack=16,
            max_locals=total_arity + snapshot_locals,
        )

    def _emit_jvm_iload_arg(
        self, builder: _BytecodeBuilder, arg_index: int,
    ) -> None:
        """Emit ``iload arg_index`` — picking the short form
        (iload_0..3) when possible."""
        if 0 <= arg_index <= 3:
            builder.emit_opcode(_OP_ILOAD_0 + arg_index)
        else:
            builder.emit_u1_instruction(_OP_ILOAD, arg_index)

    def _emit_callable_body(
        self,
        builder: _BytecodeBuilder,
        region: _CallableRegion,
        reg_count: int,
    ) -> None:
        """Emit just the IR-body instructions for ``region`` into
        ``builder``.  Shared between ``_build_callable_method`` (for
        normal regions) and ``_build_lifted_lambda_method`` (for
        lifted lambdas, which prepend a JVM-args prologue first).

        Body emission terminates at the first RET / HALT.
        Implementation: delegate to ``_build_callable_method``'s
        instruction-by-instruction emission via a slim wrapper —
        avoids duplicating the dispatch table.
        """
        # Use the existing body emission by calling the same loop
        # _build_callable_method uses.  We extract that loop into
        # a helper so both call sites share it.
        self._emit_region_instructions(builder, region, reg_count)

    def _build_callable_method(
        self, region: _CallableRegion, reg_count: int
    ) -> _MethodSpec:
        builder = _BytecodeBuilder()
        # ``reg_count`` is the program-wide max register count; the
        # caller-saves CALL convention (JVM01) uses JVM locals
        # 0..reg_count-1 to snapshot the static-array register state
        # across nested calls.  We pass it through so the method's
        # ``max_locals`` can be set high enough below.

        self._emit_region_instructions(builder, region, reg_count)

        code = builder.assemble()
        access_flags = (
            ACC_PUBLIC | ACC_STATIC
            if region.name == self.program.entry_label
            else _ACC_PRIVATE | ACC_STATIC
        )
        # JVM01: int-pool caller-saves uses JVM locals 0..reg_count-1.
        # TW03 Phase 3 follow-up: obj-pool caller-saves additionally
        # uses locals reg_count..2*reg_count-1.  Reserve the extra
        # space only when the obj pool is actually in play.
        max_locals = (2 * reg_count) if self._needs_objregs else reg_count
        return _MethodSpec(
            access_flags=access_flags,
            name=region.name,
            descriptor=_DESC_NOARGS_INT,
            code=code,
            max_stack=16,
            max_locals=max_locals,
        )

    def _emit_region_instructions(
        self,
        builder: _BytecodeBuilder,
        region: _CallableRegion,
        reg_count: int,
    ) -> None:
        """Emit one IR region's instructions into ``builder``.

        Extracted from ``_build_callable_method`` so the lifted-
        lambda method (Phase 2c.5) can prepend a JVM-args prologue
        and re-use the same instruction-by-instruction emission.
        """
        # Per-region obj-typed register analysis.  The JVM uses a
        # SHARED static ``__ca_objregs`` array, so a method that
        # writes garbage to slot N (e.g. an int-typed ADD_IMM-0
        # propagating zeros into the obj pool) corrupts every
        # caller's view of that slot.  Stash the set of registers
        # this region uses obj-style so the ADD_IMM-0 obj
        # propagation only fires when the source actually holds an
        # object — same correctness rule the CLR backend's typed
        # register pool enforces, just adapted to JVM's static
        # shared layout.
        self._region_obj_regs = self._collect_region_obj_regs(region)
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
                # TW03 Phase 3 follow-up: ``ADD_IMM dst, src, 0`` is
                # the canonical register-move idiom in our compilers
                # (twig-jvm-compiler emits it for things like closure
                # propagation: ``ADD_IMM r1, r11, 0`` to move a ref
                # into the return slot).  Without obj-slot propagation
                # the move only copies the int half of the register
                # and any object reference held in the source's obj
                # slot is lost — which broke closure-returning
                # functions once the obj-pool caller-saves landed.
                # Mirror the int copy on the obj pool whenever heap
                # or closure ops are in play.  Net: the move idiom now
                # propagates BOTH int and obj slots, making ``ADD_IMM
                # dst, src, 0`` a true register-to-register copy.
                if (
                    self._needs_objregs
                    and instruction.opcode == IrOp.ADD_IMM
                    and imm.value == 0
                    # Gate on per-region obj-typing — only propagate
                    # the obj slot when SRC actually holds an object
                    # in this region.  Without the gate, an int-typed
                    # ADD_IMM-0 (e.g. ``ADD_IMM v11, v3, 0`` inside a
                    # lambda body where v3 is an int arg) would write
                    # null to ``__ca_objregs[v11]`` and clobber a
                    # caller's obj-typed v11 slot — the JVM static
                    # ``__ca_objregs`` pool is shared across method
                    # invocations, so any unconditional write to an
                    # obj slot is a memory-safety concern.
                    and src.index in self._region_obj_regs
                ):
                    builder.emit_u2_instruction(
                        _OP_GETSTATIC,
                        self._field_ref(
                            self._objreg_field, _DESC_OBJECT_ARRAY,
                        ),
                    )
                    self._emit_push_int(builder, dst.index)
                    builder.emit_u2_instruction(
                        _OP_GETSTATIC,
                        self._field_ref(
                            self._objreg_field, _DESC_OBJECT_ARRAY,
                        ),
                    )
                    self._emit_push_int(builder, src.index)
                    builder.emit_opcode(_OP_AALOAD)
                    builder.emit_opcode(_OP_AASTORE)
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
                # TW03 Phase 3 follow-up — also snapshot the obj pool
                # when the program uses heap primitives or closures.
                # Without this, recursion through any obj-typed register
                # (e.g. ``length`` walking a cons list) clobbers the
                # caller's reference when the recursive call writes the
                # same slot.  This is the obj-pool analogue of the JVM01
                # caller-saves fix that unblocked int-register recursion.
                if self._needs_objregs:
                    self._emit_caller_save_objregs(builder, reg_count)
                self._emit_push_int(builder, 1)
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(label.name, _DESC_NOARGS_INT),
                )
                builder.emit_u2_instruction(
                    _OP_INVOKESTATIC,
                    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
                )
                if self._needs_objregs:
                    self._emit_caller_restore_objregs(builder, reg_count)
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

            # TW03 Phase 3b — heap-primitive dispatch.
            if instruction.opcode == IrOp.MAKE_CONS:
                self._emit_make_cons(builder, instruction)
                continue
            if instruction.opcode == IrOp.CAR:
                self._emit_car(builder, instruction)
                continue
            if instruction.opcode == IrOp.CDR:
                self._emit_cdr(builder, instruction)
                continue
            if instruction.opcode == IrOp.IS_NULL:
                self._emit_is_null(builder, instruction)
                continue
            if instruction.opcode == IrOp.IS_PAIR:
                self._emit_instanceof_test(
                    builder, instruction, CONS_BINARY_NAME, "IS_PAIR",
                )
                continue
            if instruction.opcode == IrOp.IS_SYMBOL:
                self._emit_instanceof_test(
                    builder, instruction, SYMBOL_BINARY_NAME, "IS_SYMBOL",
                )
                continue
            if instruction.opcode == IrOp.MAKE_SYMBOL:
                self._emit_make_symbol(builder, instruction)
                continue
            if instruction.opcode == IrOp.LOAD_NIL:
                self._emit_load_nil(builder, instruction)
                continue

            raise JvmBackendError(
                "Unsupported IR opcode in prototype backend: "
                f"{instruction.opcode}"
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
    # TW03 Phase 3b — heap primitives (cons / symbol / nil).
    IrOp.MAKE_CONS,
    IrOp.CAR,
    IrOp.CDR,
    IrOp.IS_NULL,
    IrOp.IS_PAIR,
    IrOp.MAKE_SYMBOL,
    IrOp.IS_SYMBOL,
    IrOp.LOAD_NIL,
})


# TW03 Phase 3b — opcode set that triggers ``__ca_objregs`` allocation
# alongside the existing closure trigger (``closure_free_var_counts``).
# Any program containing any of these opcodes needs the parallel
# ``Object[]`` register pool because cons cells, symbols, and nil values
# are all heap-allocated references that ride in the object slots.
_HEAP_OPCODES: frozenset[IrOp] = frozenset({
    IrOp.MAKE_CONS,
    IrOp.CAR,
    IrOp.CDR,
    IrOp.IS_NULL,
    IrOp.IS_PAIR,
    IrOp.MAKE_SYMBOL,
    IrOp.IS_SYMBOL,
    IrOp.LOAD_NIL,
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

    main_binary_name = config.class_name.replace(".", "/")
    for closure_name, num_free in config.closure_free_var_counts.items():
        classes.append(
            build_closure_subclass_artifact(
                closure_name,
                num_free=num_free,
                main_class_binary_name=main_binary_name,
            )
        )

    # TW03 Phase 3b — auto-include heap-primitive runtime classes
    # (Cons, Symbol, Nil) when the program uses any heap opcode.
    # Programs that don't use them see zero extra class-file overhead.
    uses_heap = any(i.opcode in _HEAP_OPCODES for i in program.instructions)
    if uses_heap:
        classes.append(build_cons_class_artifact())
        classes.append(build_symbol_class_artifact())
        classes.append(build_nil_class_artifact())

    return JVMMultiClassArtifact(classes=tuple(classes))


def build_closure_subclass_artifact(
    closure_name: str,
    *,
    num_free: int,
    main_class_binary_name: str | None = None,
    explicit_arity: int = 1,
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
    * has an ``apply([I)I`` method whose body forwards to a
      PUBLIC static method on the main user class
      (``main_class_binary_name._closure_name``) — the lifted
      lambda's IR body lives there.  Phase 2c.5 wired up this
      forwarder so closures actually run end-to-end on real
      ``java``.

    When ``main_class_binary_name`` is ``None`` (the Phase 2b
    structural backward-compat path), ``apply`` falls back to
    the placeholder ``iconst_0; ireturn`` body that lets the
    class load + dispatch resolve.  ``ir-to-jvm-class-file``'s
    high-level multi-class API
    (``lower_ir_to_jvm_classes``) always passes the main class
    name, so this fallback only matters for
    ``build_closure_subclass_artifact()`` direct callers in
    older test code.
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

    # ── apply body ────────────────────────────────────────────────────
    # Phase 2c.5: forward to the main class's public static
    # ``_<closure_name>`` method.  The forwarder pushes
    # ``num_free + explicit_arity`` int args (captures from this.captI
    # fields, then explicit args from the args[] parameter) and
    # invokestatic's the lifted lambda.  Returns whatever int the
    # lifted lambda returns.
    #
    # Phase 2b backward-compat: when no main class is provided,
    # emit the placeholder body.  Real lowering always supplies it.
    apply_builder = _BytecodeBuilder()
    if main_class_binary_name is None:
        apply_builder.emit_opcode(_OP_ICONST_0)
        apply_builder.emit_opcode(_OP_IRETURN)
        apply_max_stack = 1
        apply_max_locals = 2
    else:
        # Push captures from instance fields.
        for i in range(num_free):
            apply_builder.emit_opcode(_OP_ALOAD_0)  # this
            apply_builder.emit_u2_instruction(_OP_GETFIELD, field_refs[i])
        # Push explicit args from the int[] parameter.
        for i in range(explicit_arity):
            apply_builder.emit_opcode(_OP_ALOAD_1)  # args
            # ldc i; iaload
            if i <= 5:
                apply_builder.emit_opcode(_OP_ICONST_0 + i)
            else:
                apply_builder.emit_u1_instruction(_OP_BIPUSH, i)
            apply_builder.emit_opcode(_OP_IALOAD)
        # invokestatic Main._<closure_name>(I,I,...)I
        total_arity = num_free + explicit_arity
        lambda_descriptor = "(" + "I" * total_arity + ")I"
        lambda_method_ref = pool.method_ref(
            main_class_binary_name, closure_name, lambda_descriptor,
        )
        apply_builder.emit_u2_instruction(_OP_INVOKESTATIC, lambda_method_ref)
        apply_builder.emit_opcode(_OP_IRETURN)
        # Stack peaks during the args[i] loads: at the point we
        # iaload arg_i with i captures already on the stack, we
        # have (i captures so far) + (loaded captures up to now) +
        # (args[] reference) + (index) on the stack — total is
        # ``num_free + 2`` for the args[]+index pair.  Be generous:
        # ``total_arity + 2`` covers all intermediate arrangements.
        apply_max_stack = total_arity + 2
        apply_max_locals = 2  # this + args[]
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
            max_stack=apply_max_stack,
            max_locals=apply_max_locals,
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


# ---------------------------------------------------------------------------
# TW03 Phase 3b — heap-primitive runtime classes (Cons / Symbol / Nil)
# ---------------------------------------------------------------------------
#
# These three builders mirror ``build_closure_interface_artifact`` /
# ``build_closure_subclass_artifact`` from Phase 2 — each returns a
# ``JVMClassArtifact`` with hand-rolled bytecode for the runtime class
# the heap-op lowering instructions reference.
#
# The classes are deliberately tiny (one or two methods each) so:
#   - Real ``java`` verifies them in microseconds.
#   - Future polymorphic-cons / typed-symbol work can replace one
#     class without churning the surrounding plumbing.
#   - Tests can assert on byte-stable class layouts.

def build_cons_class_artifact() -> JVMClassArtifact:
    """Return the ``Cons`` class — heterogeneous pair of ``(Object head,
    Object tail)``.

    Heterogeneous-cons follow-up: ``head`` is now typed ``Object``
    (was ``int``) so cons cells can hold ANY Twig value — int
    (boxed via ``Integer.valueOf`` at MAKE_CONS), symbol, another
    cons, closure, nil.  This unblocks list-of-symbols, AST-shaped
    data, and other heterogeneous heap structures that a real Lisp
    program needs.

    Class layout::

        public final class Cons {
            public final Object head;
            public final Object tail;

            public Cons(Object head, Object tail) {
                super();
                this.head = head;
                this.tail = tail;
            }
        }

    No ``Nil``-injection at allocation time: it's the caller's job
    (the lowering site) to ensure the tail is either another
    ``Cons`` / ``Nil.INSTANCE`` / ``Symbol`` / etc.  Same for head.
    """
    pool = _ConstantPoolBuilder()
    this_class_index = pool.class_ref(CONS_BINARY_NAME)
    super_class_index = pool.class_ref("java/lang/Object")
    object_ctor_ref = pool.method_ref("java/lang/Object", "<init>", "()V")

    head_field_name = pool.utf8("head")
    head_field_desc = pool.utf8(_DESC_OBJECT)
    tail_field_name = pool.utf8("tail")
    tail_field_desc = pool.utf8(_DESC_OBJECT)
    head_field_ref = pool.field_ref(CONS_BINARY_NAME, "head", _DESC_OBJECT)
    tail_field_ref = pool.field_ref(CONS_BINARY_NAME, "tail", _DESC_OBJECT)
    code_attribute_name_index = pool.utf8("Code")

    # ── ctor body ──────────────────────────────────────────────────────
    ctor_builder = _BytecodeBuilder()
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_u2_instruction(_OP_INVOKESPECIAL, object_ctor_ref)
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_opcode(_OP_ALOAD_1)  # head: Object arg in slot 1
    ctor_builder.emit_u2_instruction(_OP_PUTFIELD, head_field_ref)
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_opcode(_OP_ALOAD_2)  # tail: Object arg in slot 2
    ctor_builder.emit_u2_instruction(_OP_PUTFIELD, tail_field_ref)
    ctor_builder.emit_opcode(_OP_RETURN)
    ctor_code = ctor_builder.assemble()
    ctor_descriptor = "(" + _DESC_OBJECT + _DESC_OBJECT + ")V"
    ctor_name_index = pool.utf8("<init>")
    ctor_descriptor_index = pool.utf8(ctor_descriptor)

    pool_bytes = pool.encode()

    # Two public final fields, one ctor.
    class_bytes_parts: list[bytes] = [
        b"\xca\xfe\xba\xbe",
        _u2(0),                     # minor_version
        _u2(49),                    # major_version
        _u2(pool.count),
        pool_bytes,
        _u2(ACC_PUBLIC | ACC_SUPER | _ACC_FINAL),
        _u2(this_class_index),
        _u2(super_class_index),
        _u2(0),                     # interfaces_count
        _u2(2),                     # fields_count
        _u2(ACC_PUBLIC | _ACC_FINAL),
        _u2(head_field_name),
        _u2(head_field_desc),
        _u2(0),                     # 0 attributes
        _u2(ACC_PUBLIC | _ACC_FINAL),
        _u2(tail_field_name),
        _u2(tail_field_desc),
        _u2(0),
        _u2(1),                     # methods_count (just the ctor)
    ]
    class_bytes_parts.append(
        _build_method_info(
            access_flags=ACC_PUBLIC,
            name_index=ctor_name_index,
            descriptor_index=ctor_descriptor_index,
            code=ctor_code,
            max_stack=2,
            max_locals=3,           # this + head + tail
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    class_bytes_parts.append(_u2(0))  # 0 class attributes

    return JVMClassArtifact(
        class_name=CONS_BINARY_NAME.replace("/", "."),
        class_bytes=b"".join(class_bytes_parts),
        callable_labels=(),
        data_offsets={},
    )


def build_nil_class_artifact() -> JVMClassArtifact:
    """Return the ``Nil`` sentinel class — singleton via static
    ``INSTANCE`` field.

    Class layout::

        public final class Nil {
            public static final Nil INSTANCE;
            static { INSTANCE = new Nil(); }
            private Nil() { super(); }
        }

    ``IS_NULL src`` lowering reads ``Nil.INSTANCE`` and identity-tests
    the source register against it; ``LOAD_NIL dst`` simply reads the
    static field and stores the ref into ``__ca_objregs[dst]``.
    """
    pool = _ConstantPoolBuilder()
    this_class_index = pool.class_ref(NIL_BINARY_NAME)
    super_class_index = pool.class_ref("java/lang/Object")
    object_ctor_ref = pool.method_ref("java/lang/Object", "<init>", "()V")

    instance_field_name = pool.utf8("INSTANCE")
    instance_field_desc = pool.utf8(_DESC_NIL)
    instance_field_ref = pool.field_ref(NIL_BINARY_NAME, "INSTANCE", _DESC_NIL)
    code_attribute_name_index = pool.utf8("Code")

    # ── ctor body (private, no-arg) ────────────────────────────────────
    ctor_builder = _BytecodeBuilder()
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_u2_instruction(_OP_INVOKESPECIAL, object_ctor_ref)
    ctor_builder.emit_opcode(_OP_RETURN)
    ctor_code = ctor_builder.assemble()
    ctor_name_index = pool.utf8("<init>")
    ctor_descriptor_index = pool.utf8("()V")

    # ── <clinit> body — INSTANCE = new Nil() ──────────────────────────
    clinit_builder = _BytecodeBuilder()
    clinit_builder.emit_u2_instruction(_OP_NEW, this_class_index)
    clinit_builder.emit_opcode(_OP_DUP)
    nil_ctor_ref = pool.method_ref(NIL_BINARY_NAME, "<init>", "()V")
    clinit_builder.emit_u2_instruction(_OP_INVOKESPECIAL, nil_ctor_ref)
    clinit_builder.emit_u2_instruction(_OP_PUTSTATIC, instance_field_ref)
    clinit_builder.emit_opcode(_OP_RETURN)
    clinit_code = clinit_builder.assemble()
    clinit_name_index = pool.utf8("<clinit>")
    clinit_descriptor_index = pool.utf8("()V")

    pool_bytes = pool.encode()

    class_bytes_parts: list[bytes] = [
        b"\xca\xfe\xba\xbe",
        _u2(0),
        _u2(49),
        _u2(pool.count),
        pool_bytes,
        _u2(ACC_PUBLIC | ACC_SUPER | _ACC_FINAL),
        _u2(this_class_index),
        _u2(super_class_index),
        _u2(0),                     # interfaces_count
        _u2(1),                     # fields_count
        _u2(ACC_PUBLIC | ACC_STATIC | _ACC_FINAL),
        _u2(instance_field_name),
        _u2(instance_field_desc),
        _u2(0),
        _u2(2),                     # methods_count: ctor + <clinit>
    ]
    class_bytes_parts.append(
        _build_method_info(
            access_flags=_ACC_PRIVATE,
            name_index=ctor_name_index,
            descriptor_index=ctor_descriptor_index,
            code=ctor_code,
            max_stack=1,
            max_locals=1,
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    class_bytes_parts.append(
        _build_method_info(
            access_flags=ACC_STATIC,
            name_index=clinit_name_index,
            descriptor_index=clinit_descriptor_index,
            code=clinit_code,
            max_stack=2,
            max_locals=0,
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    class_bytes_parts.append(_u2(0))

    return JVMClassArtifact(
        class_name=NIL_BINARY_NAME.replace("/", "."),
        class_bytes=b"".join(class_bytes_parts),
        callable_labels=(),
        data_offsets={},
    )


def build_symbol_class_artifact() -> JVMClassArtifact:
    """Return the ``Symbol`` class — interned identifier with a
    static ``HashMap``-backed intern table.

    Class layout::

        public final class Symbol {
            private final String name;
            private static final HashMap INTERN_TABLE;
            static { INTERN_TABLE = new HashMap(); }
            private Symbol(String name) { super(); this.name = name; }
            public static Symbol intern(String name) {
                Symbol s = (Symbol) INTERN_TABLE.get(name);
                if (s == null) {
                    s = new Symbol(name);
                    INTERN_TABLE.put(name, s);
                }
                return s;
            }
        }

    Single-threaded — Twig has no concurrency surface today.  When a
    threaded runtime spec lands, swap ``HashMap`` for
    ``ConcurrentHashMap`` here without touching call sites.

    Two ``Symbol.intern`` calls with the same ``name`` argument always
    return the same reference — ``IS_SYMBOL`` instanceof tests rely
    only on the type tag, not the identity, but other Lisp ops
    (``eq?``) will eventually need this guarantee.
    """
    pool = _ConstantPoolBuilder()
    this_class_index = pool.class_ref(SYMBOL_BINARY_NAME)
    super_class_index = pool.class_ref("java/lang/Object")
    object_ctor_ref = pool.method_ref("java/lang/Object", "<init>", "()V")

    name_field_name = pool.utf8("name")
    name_field_desc = pool.utf8(_DESC_STRING)
    name_field_ref = pool.field_ref(SYMBOL_BINARY_NAME, "name", _DESC_STRING)

    intern_table_field_name = pool.utf8("INTERN_TABLE")
    intern_table_field_desc = pool.utf8(_DESC_HASHMAP)
    intern_table_field_ref = pool.field_ref(
        SYMBOL_BINARY_NAME, "INTERN_TABLE", _DESC_HASHMAP,
    )

    hashmap_class_index = pool.class_ref("java/util/HashMap")
    hashmap_ctor_ref = pool.method_ref("java/util/HashMap", "<init>", "()V")
    hashmap_get_ref = pool.method_ref(
        "java/util/HashMap", "get", _DESC_OBJECT_TO_OBJECT,
    )
    hashmap_put_ref = pool.method_ref(
        "java/util/HashMap", "put", _DESC_OBJECT_OBJECT_TO_OBJECT,
    )

    code_attribute_name_index = pool.utf8("Code")

    # ── ctor body — Symbol(String name) ───────────────────────────────
    ctor_builder = _BytecodeBuilder()
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_u2_instruction(_OP_INVOKESPECIAL, object_ctor_ref)
    ctor_builder.emit_opcode(_OP_ALOAD_0)
    ctor_builder.emit_opcode(_OP_ALOAD_1)
    ctor_builder.emit_u2_instruction(_OP_PUTFIELD, name_field_ref)
    ctor_builder.emit_opcode(_OP_RETURN)
    ctor_code = ctor_builder.assemble()
    ctor_name_index = pool.utf8("<init>")
    ctor_descriptor_index = pool.utf8(f"({_DESC_STRING})V")

    # ── <clinit> body — INTERN_TABLE = new HashMap() ──────────────────
    clinit_builder = _BytecodeBuilder()
    clinit_builder.emit_u2_instruction(_OP_NEW, hashmap_class_index)
    clinit_builder.emit_opcode(_OP_DUP)
    clinit_builder.emit_u2_instruction(_OP_INVOKESPECIAL, hashmap_ctor_ref)
    clinit_builder.emit_u2_instruction(_OP_PUTSTATIC, intern_table_field_ref)
    clinit_builder.emit_opcode(_OP_RETURN)
    clinit_code = clinit_builder.assemble()
    clinit_name_index = pool.utf8("<clinit>")
    clinit_descriptor_index = pool.utf8("()V")

    # ── intern body — Symbol intern(String name) ──────────────────────
    # locals: 0 = name, 1 = s
    # bytecode:
    #   getstatic INTERN_TABLE
    #   aload_0                   ; name
    #   invokevirtual HashMap.get(Object) Object
    #   checkcast Symbol
    #   astore_1                  ; s
    #   aload_1
    #   ifnonnull RETURN_S
    #     new Symbol; dup; aload_0; invokespecial Symbol.<init>(String)
    #     astore_1
    #     getstatic INTERN_TABLE; aload_0; aload_1
    #     invokevirtual HashMap.put(Object,Object) Object
    #     pop
    #   RETURN_S:
    #     aload_1; areturn
    intern_builder = _BytecodeBuilder()
    return_label = "__sym_intern_ret"
    intern_builder.emit_u2_instruction(_OP_GETSTATIC, intern_table_field_ref)
    intern_builder.emit_opcode(_OP_ALOAD_0)
    intern_builder.emit_u2_instruction(_OP_INVOKEVIRTUAL, hashmap_get_ref)
    intern_builder.emit_u2_instruction(_OP_CHECKCAST, this_class_index)
    intern_builder.emit_opcode(_OP_ASTORE_1)
    intern_builder.emit_opcode(_OP_ALOAD_1)
    intern_builder.emit_branch(_OP_IFNONNULL, return_label)
    # cache miss: build a new Symbol and put it into the table.
    intern_builder.emit_u2_instruction(_OP_NEW, this_class_index)
    intern_builder.emit_opcode(_OP_DUP)
    intern_builder.emit_opcode(_OP_ALOAD_0)
    sym_ctor_ref = pool.method_ref(
        SYMBOL_BINARY_NAME, "<init>", f"({_DESC_STRING})V",
    )
    intern_builder.emit_u2_instruction(_OP_INVOKESPECIAL, sym_ctor_ref)
    intern_builder.emit_opcode(_OP_ASTORE_1)
    intern_builder.emit_u2_instruction(_OP_GETSTATIC, intern_table_field_ref)
    intern_builder.emit_opcode(_OP_ALOAD_0)
    intern_builder.emit_opcode(_OP_ALOAD_1)
    intern_builder.emit_u2_instruction(_OP_INVOKEVIRTUAL, hashmap_put_ref)
    intern_builder.emit_opcode(_OP_POP)
    intern_builder.mark(return_label)
    intern_builder.emit_opcode(_OP_ALOAD_1)
    intern_builder.emit_opcode(_OP_ARETURN)
    intern_code = intern_builder.assemble()
    intern_name_index = pool.utf8("intern")
    intern_descriptor_index = pool.utf8(f"({_DESC_STRING}){_DESC_SYMBOL}")

    pool_bytes = pool.encode()

    class_bytes_parts: list[bytes] = [
        b"\xca\xfe\xba\xbe",
        _u2(0),
        _u2(49),
        _u2(pool.count),
        pool_bytes,
        _u2(ACC_PUBLIC | ACC_SUPER | _ACC_FINAL),
        _u2(this_class_index),
        _u2(super_class_index),
        _u2(0),                     # interfaces_count
        _u2(2),                     # fields_count
        _u2(_ACC_PRIVATE | _ACC_FINAL),
        _u2(name_field_name),
        _u2(name_field_desc),
        _u2(0),
        _u2(_ACC_PRIVATE | ACC_STATIC | _ACC_FINAL),
        _u2(intern_table_field_name),
        _u2(intern_table_field_desc),
        _u2(0),
        _u2(3),                     # methods_count: ctor + <clinit> + intern
    ]
    class_bytes_parts.append(
        _build_method_info(
            access_flags=_ACC_PRIVATE,
            name_index=ctor_name_index,
            descriptor_index=ctor_descriptor_index,
            code=ctor_code,
            max_stack=2,
            max_locals=2,
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    class_bytes_parts.append(
        _build_method_info(
            access_flags=ACC_STATIC,
            name_index=clinit_name_index,
            descriptor_index=clinit_descriptor_index,
            code=clinit_code,
            max_stack=2,
            max_locals=0,
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    class_bytes_parts.append(
        _build_method_info(
            access_flags=ACC_PUBLIC | ACC_STATIC,
            name_index=intern_name_index,
            descriptor_index=intern_descriptor_index,
            code=intern_code,
            max_stack=3,
            max_locals=2,
            code_attribute_name_index=code_attribute_name_index,
        )
    )
    class_bytes_parts.append(_u2(0))

    return JVMClassArtifact(
        class_name=SYMBOL_BINARY_NAME.replace("/", "."),
        class_bytes=b"".join(class_bytes_parts),
        callable_labels=(),
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
