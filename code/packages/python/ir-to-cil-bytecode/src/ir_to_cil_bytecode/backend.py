"""Lower compiler IR into CIL method-body bytecode.

This package deliberately emits method-body artifacts instead of full PE/CLI
assemblies. CIL call instructions require metadata tokens, so the lowerer takes
an injectable token provider. The default provider assigns deterministic
placeholder tokens that are useful for tests and for composing later stages.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from enum import StrEnum
from typing import Final, Protocol

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
    """Configuration for compiler IR to CIL bytecode lowering.

    ``closure_free_var_counts`` declares which IR regions are
    lifted-lambda bodies (TW03 Phase 2 / CLR02 Phase 2c).  Each
    entry maps a region's name to its number of captured free
    variables.  Regions in this map are lowered as the ``Apply``
    method on an auto-generated ``Closure_<name>`` TypeDef, with
    a prologue that copies captures from instance fields and
    arguments from the caller-passed ``int[]`` into the IR's
    expected register slots.

    The lambda body itself sees a captures-first IR register
    layout (matches what twig-clr-compiler produces and what
    BEAM Phase 2 already uses):

      * ``r2..r{1+num_free}``                             — captures
      * ``r{2+num_free}..r{1+num_free+explicit_arity}``   — explicit args
    """

    syscall_arg_reg: int = 4
    max_static_data_bytes: int = _MAX_STATIC_DATA_BYTES
    method_max_stack: int = 16
    call_register_count: int | None = 0
    closure_free_var_counts: dict[str, int] = field(default_factory=dict)


@dataclass(frozen=True)
class CILMethodArtifact:
    """A lowered CIL method body and its method-level metadata needs.

    ``is_instance`` controls whether the method has the ``HASTHIS``
    bit in its calling-convention byte.  Default ``False`` matches
    the existing CLR01 surface (every method on the user's main
    type is static).  CLR02 (closures) introduces instance methods
    on lifted closure types — set ``is_instance=True`` for those.
    Constructors additionally need ``is_special_name=True`` to set
    the ``SpecialName`` and ``RTSpecialName`` flags so the loader
    treats them as ``.ctor``.
    """

    name: str
    body: bytes
    max_stack: int
    local_types: tuple[str, ...]
    return_type: str = "int32"
    parameter_types: tuple[str, ...] = ()
    is_instance: bool = False
    is_special_name: bool = False
    # ``is_abstract`` marks the method as having no body (RVA=0) —
    # required for interface methods.  Abstract methods imply
    # ``is_instance=True`` and an empty ``body``.
    is_abstract: bool = False

    @property
    def local_count(self) -> int:
        """Return the number of local variable slots used by this method."""
        return len(self.local_types)


@dataclass(frozen=True)
class CILFieldArtifact:
    """An instance field on a closure / extra TypeDef.

    Static fields are out of scope (the closure design uses
    instance fields for captures).
    """

    name: str
    type: str = "int32"


@dataclass(frozen=True)
class CILTypeArtifact:
    """An extra ``TypeDef`` row to emit alongside the main user type.

    Used for:
    * ``IClosure`` — the abstract interface every closure
      implements (``is_interface=True``, ``extends=None``).
    * ``Closure_<lambda>`` — one concrete class per lifted
      lambda (``is_interface=False``, ``extends="System.Object"``,
      ``implements=("CodingAdventures.IClosure",)``).

    Field and method order is preserved verbatim — the writer
    assigns consecutive table rows in declaration order.
    """

    name: str
    namespace: str = ""
    is_interface: bool = False
    extends: str | None = "System.Object"
    implements: tuple[str, ...] = ()
    fields: tuple[CILFieldArtifact, ...] = ()
    methods: tuple[CILMethodArtifact, ...] = ()


@dataclass(frozen=True)
class CILProgramArtifact:
    """The result of lowering a compiler IR program to CIL method bodies.

    ``methods`` are emitted on the main user TypeDef (configured via
    ``CLIAssemblyConfig.type_name``).  ``extra_types`` are
    additional TypeDef rows — closure types and the IClosure
    interface for CLR02 Phase 2.  Empty by default for backwards
    compat with CLR01 callers.
    """

    entry_label: str
    methods: tuple[CILMethodArtifact, ...]
    data_offsets: dict[str, int]
    data_size: int
    helper_specs: tuple[CILHelperSpec, ...]
    token_provider: CILTokenProvider
    extra_types: tuple[CILTypeArtifact, ...] = ()

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

    # ── CLR02 Phase 2c — closure metadata tokens ────────────────────────
    #
    # All four return MethodDef / Field tokens (0x06xxxxxx / 0x04xxxxxx)
    # for CLR02 Phase 2c closure lowering.  Implementations compute
    # them deterministically from the closure ordering in the program.
    # The ``cli-assembly-writer`` honours the same ordering so the
    # tokens line up with the actual emitted rows.

    def iclosure_apply_token(self) -> int:
        """Return the MethodDef token for ``IClosure::Apply(int32) → int32``."""

    def closure_ctor_token(self, closure_name: str) -> int:
        """Return the MethodDef token for ``Closure_<closure_name>::.ctor``."""

    def closure_apply_token(self, closure_name: str) -> int:
        """Return the MethodDef token for ``Closure_<closure_name>::Apply``."""

    def closure_field_token(self, closure_name: str, capture_index: int) -> int:
        """Return the Field token for the ``capture_index``-th field
        on ``Closure_<closure_name>``."""

    def system_object_ctor_token(self) -> int:
        """Return the MemberRef token for ``[System.Runtime]System.Object::.ctor()``."""


class SequentialCILTokenProvider:
    """Deterministic token provider for standalone bytecode lowering.

    Method tokens start at ``0x06000001`` in emitted callable order.
    Helper tokens start at ``0x0A000001`` in ``CILHelper`` enum order.

    For CLR02 Phase 2c, closure-related tokens follow a deterministic
    layout that ``cli-assembly-writer`` mirrors:

    * After the M main-method MethodDef rows comes one row for
      ``IClosure::Apply`` (the abstract interface method) — token
      ``0x06000001 + M``.
    * Then for each closure k (0-indexed in declaration order),
      two rows: ``.ctor`` at ``0x06000001 + M + 1 + 2k`` and
      ``Apply`` at ``0x06000001 + M + 2 + 2k``.
    * Field tokens (``0x04xxxxxx``) walk the closures in
      declaration order, with one row per capture in declaration
      order.
    * The ``System.Object::.ctor`` MemberRef is emitted by the
      writer at row ``len(helper_specs) + 1`` whenever any
      closure type is present, giving a deterministic
      ``0x0A000000 | (len(helper_specs) + 1)`` token.
    """

    def __init__(
        self,
        method_names: tuple[str, ...],
        *,
        closure_names: tuple[str, ...] = (),
        closure_free_var_counts: dict[str, int] | None = None,
    ) -> None:
        self._method_tokens = {
            name: 0x06000001 + index for index, name in enumerate(method_names)
        }
        self._helper_tokens = {
            helper: 0x0A000001 + index for index, helper in enumerate(CILHelper)
        }

        # Closure-method tokens.  Layout described in the class
        # docstring: IClosure::Apply, then per-closure (ctor, Apply)
        # pairs in ``closure_names`` order.
        self._closure_names = closure_names
        self._free_counts = closure_free_var_counts or {}
        m = len(method_names)
        self._iclosure_apply = 0x06000001 + m if closure_names else 0
        self._closure_ctors: dict[str, int] = {}
        self._closure_applies: dict[str, int] = {}
        for k, name in enumerate(closure_names):
            self._closure_ctors[name] = 0x06000001 + m + 1 + 2 * k
            self._closure_applies[name] = 0x06000001 + m + 2 + 2 * k

        # Field tokens.  One per capture, walking closures in
        # declaration order.
        self._closure_fields: dict[tuple[str, int], int] = {}
        field_row = 0
        for name in closure_names:
            for i in range(self._free_counts.get(name, 0)):
                field_row += 1
                self._closure_fields[(name, i)] = 0x04000000 | field_row

        # System.Object::.ctor MemberRef — present iff any closure
        # is present (because closures are the only thing that needs
        # to chain into the base ctor).  Row = helper count + 1.
        self._object_ctor = (
            0x0A000000 | (len(CILHelper) + 1) if closure_names else 0
        )

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

    def iclosure_apply_token(self) -> int:
        if not self._iclosure_apply:
            msg = "no closure regions registered with this token provider"
            raise CILBackendError(msg)
        return self._iclosure_apply

    def closure_ctor_token(self, closure_name: str) -> int:
        try:
            return self._closure_ctors[closure_name]
        except KeyError as exc:
            msg = f"unknown closure region: {closure_name!r}"
            raise CILBackendError(msg) from exc

    def closure_apply_token(self, closure_name: str) -> int:
        try:
            return self._closure_applies[closure_name]
        except KeyError as exc:
            msg = f"unknown closure region: {closure_name!r}"
            raise CILBackendError(msg) from exc

    def closure_field_token(self, closure_name: str, capture_index: int) -> int:
        try:
            return self._closure_fields[(closure_name, capture_index)]
        except KeyError as exc:
            msg = (
                f"unknown closure field: {closure_name!r} capture "
                f"{capture_index}"
            )
            raise CILBackendError(msg) from exc

    def system_object_ctor_token(self) -> int:
        if not self._object_ctor:
            msg = "no closure regions registered with this token provider"
            raise CILBackendError(msg)
        return self._object_ctor


@dataclass(frozen=True)
class _CallableRegion:
    name: str
    start_index: int
    end_index: int
    instructions: tuple[IrInstruction, ...]


@dataclass(frozen=True)
class CILLoweringPlan:
    """Validated lowering plan shared by composable pipeline stages.

    ``closure_names`` is the ordered tuple of region names that
    are lifted-lambda bodies (CLR02 Phase 2c).  ``main_region_names``
    contains the remaining regions — the methods that go on the
    user's main TypeDef.  ``closure_free_var_counts`` repeats the
    config knob so the lower stage doesn't need the config.

    ``function_return_types`` (CLR02 Phase 2c.5) maps each region
    name to its computed return type (``"int32"`` or
    ``"object"``).  Functions whose ``r1`` register is object-typed
    at any RET point return ``object``; everything else returns
    ``int32``.  The map is computed by a fixed-point analysis in
    ``_classify_function_return_types`` so callers know which
    receiving-side stloc to emit.
    """

    regions: tuple[_CallableRegion, ...]
    data_offsets: dict[str, int]
    data_size: int
    local_count: int
    token_provider: CILTokenProvider
    closure_names: tuple[str, ...] = ()
    main_region_names: tuple[str, ...] = ()
    closure_free_var_counts: dict[str, int] = field(default_factory=dict)
    function_return_types: dict[str, str] = field(default_factory=dict)


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
        # Main-type methods first (existing path); closures go into
        # extra_types (CLR02 Phase 2c).
        main_methods: list[CILMethodArtifact] = []
        closure_apply_methods: dict[str, CILMethodArtifact] = {}
        for region in plan.regions:
            artifact = self._lower_region(program, resolved_config, plan, region)
            if region.name in plan.closure_free_var_counts:
                closure_apply_methods[region.name] = artifact
            else:
                main_methods.append(artifact)

        extra_types = _build_closure_extra_types(plan, closure_apply_methods)

        return CILProgramArtifact(
            entry_label=program.entry_label,
            methods=tuple(main_methods),
            data_offsets=dict(plan.data_offsets),
            data_size=plan.data_size,
            helper_specs=HELPER_SPECS,
            token_provider=plan.token_provider,
            extra_types=extra_types,
        )


# ---------------------------------------------------------------------------
# Pre-flight validation
# ---------------------------------------------------------------------------
#
# ``validate_for_clr`` inspects an IrProgram for CLR backend incompatibilities
# *before* any bytecode is generated.  It mirrors the pattern established by
# ``validate_for_jvm`` in the JVM backend.
#
# The CLR host (brainfuck-clr-compiler / CLR VM) wires up three syscalls:
#
#   SYSCALL 1  — write byte to stdout  (``System.Console.Write(char)``)
#   SYSCALL 2  — read byte from stdin  (``System.Console.Read()``)
#   SYSCALL 10 — process exit          (``System.Environment.Exit(code)``)
#
# Oct's Intel 8008 I/O intrinsics map to different numbers:
#   out(PORT, val) → SYSCALL 40+PORT   (e.g. out(17,v) → SYSCALL 57)
#   in(PORT)       → SYSCALL 20+PORT   (e.g. in(3)    → SYSCALL 23)
#
# Those 8008-specific numbers are not wired in the CLR host.  Without a
# compile-time check the program would pass through CIL lowering silently and
# then raise CLRVMError *at runtime* — a much later and harder-to-diagnose
# failure.  The pre-flight validator surfaces the mismatch before any bytes
# are produced.
# ---------------------------------------------------------------------------

_CLR_SUPPORTED_SYSCALLS: frozenset[int] = frozenset({1, 2, 10})

# Every IrOp that the V1 CIL lowerer handles.  Any opcode absent from this set
# is rejected by validate_for_clr() so callers get a clear error instead of
# an "Unsupported IR opcode" exception buried inside lowering.
_CLR_SUPPORTED_OPCODES: frozenset[IrOp] = frozenset({
    IrOp.LABEL,
    IrOp.COMMENT,
    IrOp.NOP,
    IrOp.HALT,
    IrOp.RET,
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
    IrOp.JUMP,
    IrOp.BRANCH_Z,
    IrOp.BRANCH_NZ,
    IrOp.CALL,
    IrOp.SYSCALL,
    # CLR02 Phase 2c — closures.
    IrOp.MAKE_CLOSURE,
    IrOp.APPLY_CLOSURE,
})


def validate_for_clr(program: IrProgram) -> list[str]:
    """Inspect ``program`` for CLR backend incompatibilities without generating
    any bytecode.

    Checks performed:

    1. **Opcode support** — every opcode must appear in
       ``_CLR_SUPPORTED_OPCODES``.  Opcodes that the V1 CLR backend does not
       handle (e.g. future IR extensions) are rejected with a precise
       diagnostic before any CIL bytes are produced.

    2. **Constant range** — every ``IrImmediate`` in a ``LOAD_IMM`` or
       ``ADD_IMM`` instruction must fit in a CIL ``int32``
       (−2 147 483 648 to 2 147 483 647).  Larger constants cannot be loaded
       by a single ``ldc.i4`` instruction.

    3. **SYSCALL number** — only SYSCALL numbers 1 (write byte), 2 (read byte),
       and 10 (process exit) are wired in the V1 CLR host.  Oct's 8008-specific
       SYSCALL numbers (20+PORT for input, 40+PORT for output) are caught here
       instead of failing silently at runtime.

    Args:
        program: The ``IrProgram`` to inspect.

    Returns:
        A list of human-readable error strings.  An empty list means the
        program is compatible with the CLR V1 backend.

    Example — a pure-arithmetic Oct program passes validation::

        errors = validate_for_clr(program)
        assert errors == []

    Example — Oct's out(17, val) → SYSCALL 57 is rejected::

        errors = validate_for_clr(program_with_oct_io)
        assert any("57" in e for e in errors)
    """
    errors: list[str] = []

    for instr in program.instructions:
        op = instr.opcode

        # ── Rule 1: opcode must be in the supported set ──────────────────────
        if op not in _CLR_SUPPORTED_OPCODES:
            errors.append(f"unsupported opcode {op.name} in V1 CLR backend")
            continue

        # ── Rule 2: constant range for LOAD_IMM / ADD_IMM ───────────────────
        if op in (IrOp.LOAD_IMM, IrOp.ADD_IMM):
            for operand in instr.operands:
                if isinstance(operand, IrImmediate) and not (
                    _INT32_MIN <= operand.value <= _INT32_MAX
                ):
                    errors.append(
                        f"immediate value {operand.value} out of int32 range "
                        f"[{_INT32_MIN}, {_INT32_MAX}] in {op.name}"
                    )

        # ── Rule 3: SYSCALL number ───────────────────────────────────────────
        elif op == IrOp.SYSCALL:
            for operand in instr.operands:
                if (
                    isinstance(operand, IrImmediate)
                    and operand.value not in _CLR_SUPPORTED_SYSCALLS
                ):
                    errors.append(
                        f"unsupported SYSCALL {operand.value}: "
                        f"only SYSCALL numbers {sorted(_CLR_SUPPORTED_SYSCALLS)} "
                        f"are wired in the V1 CLR backend"
                    )
                    break

    return errors


def lower_ir_to_cil_bytecode(
    program: IrProgram,
    config: CILBackendConfig | None = None,
    *,
    token_provider: CILTokenProvider | None = None,
) -> CILProgramArtifact:
    """Lower a compiler IR program to CIL bytecode method artifacts.

    Runs ``validate_for_clr`` as a pre-flight check before any CIL bytes are
    produced.  Any validation failure raises ``CILBackendError`` with a
    human-readable summary so callers get a clear error at compile time instead
    of a runtime exception inside the CLR VM.
    """
    errors = validate_for_clr(program)
    if errors:
        joined = "; ".join(errors)
        raise CILBackendError(
            f"IR program failed CLR pre-flight validation "
            f"({len(errors)} error{'s' if len(errors) != 1 else ''}): {joined}"
        )
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

    # Split regions into "main" and "closure" buckets.  Closure
    # regions become Apply methods on auto-generated Closure_<name>
    # TypeDefs (CLR02 Phase 2c).  Main regions become methods on
    # the user's main TypeDef (the existing CLR01 path).
    closure_set = set(config.closure_free_var_counts)
    unknown_closures = closure_set - {r.name for r in regions}
    if unknown_closures:
        msg = (
            "closure_free_var_counts references regions that don't "
            f"exist in the IR: {sorted(unknown_closures)}"
        )
        raise CILBackendError(msg)
    main_region_names = tuple(r.name for r in regions if r.name not in closure_set)
    # Closure ordering follows IR-region order so token assignment
    # is deterministic.
    closure_names = tuple(r.name for r in regions if r.name in closure_set)

    provider = token_provider or SequentialCILTokenProvider(
        main_region_names,
        closure_names=closure_names,
        closure_free_var_counts=dict(config.closure_free_var_counts),
    )

    function_return_types = _classify_function_return_types(
        regions, set(config.closure_free_var_counts),
    )

    return CILLoweringPlan(
        regions=regions,
        data_offsets=data_offsets,
        data_size=data_size,
        local_count=local_count,
        token_provider=provider,
        closure_names=closure_names,
        main_region_names=main_region_names,
        closure_free_var_counts=dict(config.closure_free_var_counts),
        function_return_types=function_return_types,
    )


# ---------------------------------------------------------------------------
# CLR02 Phase 2c.5 — typed register pool
# ---------------------------------------------------------------------------
#
# Closure refs are managed pointers (object), but the existing CLR
# backend uses int32-uniform locals/parameters.  Storing an object
# ref into an int32 local truncates the pointer.  Phase 2c.5
# resolves this with a per-region register-typing pass that
# allocates parallel ``object`` locals for any IR register that
# ever holds an object ref.  Operations choose between the int and
# object slot based on the type they're producing/consuming.
#
# Algorithm sketch:
#
# 1. Compute ``function_return_types`` (region → "int32"|"object")
#    by iterating to a fixed point.  A region returns object iff
#    its r1 is object-typed at any RET site.  Closure regions
#    (lifted lambda Apply methods) always return int32 per the
#    IClosure contract.
#
# 2. For each region's lowering, run a linear pass over its
#    instructions tracking the current type of each IR register.
#    The set of registers that are object-typed at any point
#    becomes the region's "object register" pool.
#
# 3. Allocate CIL locals: existing int32 slots 0..N-1, plus extra
#    object slots N..N+M-1 (one per object-typed register).
#
# 4. Emit instructions choosing the slot based on the
#    just-computed type at that program point.

_REG_HALT_RESULT: Final = 1


def _instr_register_type_writes(
    instr: IrInstruction, current_types: dict[int, str],
    function_return_types: dict[str, str],
) -> dict[int, str]:
    """Apply ``instr``'s effect to ``current_types`` and return
    the new type map.  Pure function — does not mutate the input.

    The type-tracking rules:

    * ``MAKE_CLOSURE dst, ...`` — dst becomes object.
    * ``APPLY_CLOSURE dst, ...`` — dst becomes int32 (apply
      returns int32 per the IClosure interface contract).
    * ``CALL target`` — r1 becomes ``function_return_types[target]``.
    * ``ADD_IMM dst, src, 0`` — MOV idiom: dst inherits src's
      current type (so closure refs propagate through register
      moves cleanly).
    * Other arithmetic / LOAD_IMM / etc — dst becomes int32.
    """
    new_types = dict(current_types)
    op = instr.opcode
    if op is IrOp.MAKE_CLOSURE:
        dst = _as_register(instr.operands[0], "MAKE_CLOSURE dst")
        new_types[dst.index] = "object"
    elif op is IrOp.APPLY_CLOSURE:
        dst = _as_register(instr.operands[0], "APPLY_CLOSURE dst")
        new_types[dst.index] = "int32"
    elif op is IrOp.CALL:
        target = _as_label(instr.operands[0], "CALL target")
        new_types[_REG_HALT_RESULT] = function_return_types.get(
            target.name, "int32",
        )
    elif op is IrOp.ADD_IMM:
        dst = _as_register(instr.operands[0], "ADD_IMM dst")
        src = _as_register(instr.operands[1], "ADD_IMM src")
        imm = _as_immediate(instr.operands[2], "ADD_IMM imm")
        if imm.value == 0:
            new_types[dst.index] = current_types.get(src.index, "int32")
        else:
            new_types[dst.index] = "int32"
    elif op in (
        IrOp.LOAD_IMM, IrOp.LOAD_ADDR, IrOp.LOAD_BYTE, IrOp.LOAD_WORD,
        IrOp.ADD, IrOp.SUB, IrOp.AND, IrOp.AND_IMM, IrOp.OR, IrOp.OR_IMM,
        IrOp.XOR, IrOp.XOR_IMM, IrOp.NOT, IrOp.MUL, IrOp.DIV,
        IrOp.CMP_EQ, IrOp.CMP_NE, IrOp.CMP_LT, IrOp.CMP_GT,
    ):
        # All single-dst int-producing ops (operand 0 is the dst register).
        if instr.operands and isinstance(instr.operands[0], IrRegister):
            new_types[instr.operands[0].index] = "int32"
    return new_types


def _classify_function_return_types(
    regions: tuple[_CallableRegion, ...],
    closure_region_names: set[str],
) -> dict[str, str]:
    """Compute each region's return type by iterating to a fixed
    point.  Closure regions (lambdas) always return int32 per the
    IClosure interface contract.
    """
    return_types: dict[str, str] = {r.name: "int32" for r in regions}
    changed = True
    iteration_cap = len(regions) + 4
    while changed and iteration_cap > 0:
        changed = False
        iteration_cap -= 1
        for region in regions:
            if region.name in closure_region_names:
                continue
            inferred = _infer_region_return_type(region, return_types)
            if inferred != return_types[region.name]:
                return_types[region.name] = inferred
                changed = True
    return return_types


def _infer_region_return_type(
    region: _CallableRegion,
    return_types: dict[str, str],
) -> str:
    """Linear-trace inference: what type holds in r1 at the end
    of the region?"""
    types: dict[int, str] = {}
    for instr in region.instructions:
        types = _instr_register_type_writes(instr, types, return_types)
    return types.get(_REG_HALT_RESULT, "int32")


def _collect_object_typed_registers(
    region: _CallableRegion,
    return_types: dict[str, str],
) -> tuple[set[int], list[dict[int, str]]]:
    """For ``region``, compute (a) the set of IR register indices
    that ever hold an object ref, and (b) a per-instruction list
    of register-type maps (state AFTER each instruction).

    The per-instruction map lets the lowerer choose the correct
    slot (int32 vs object) for each register at each program
    point.
    """
    obj_regs: set[int] = set()
    instr_types: list[dict[int, str]] = []
    types: dict[int, str] = {}
    for instr in region.instructions:
        types = _instr_register_type_writes(instr, types, return_types)
        instr_types.append(types)
        for reg_idx, type_name in types.items():
            if type_name == "object":
                obj_regs.add(reg_idx)
    return obj_regs, instr_types


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
        elif instruction.opcode == IrOp.MAKE_CLOSURE:
            # MAKE_CLOSURE dst, fn_label, num_captured, capt0, ...
            # The lambda body is a callable region too, even though
            # it's invoked indirectly via callvirt at the apply site
            # rather than directly via CALL.
            target = _as_label(
                instruction.operands[1], "MAKE_CLOSURE fn_label",
            )
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
    if region.name in plan.closure_free_var_counts:
        return _lower_closure_region(_program, config, plan, region)

    builder = CILBytecodeBuilder()
    call_register_count = _call_register_count(config, plan)

    # Phase 2c.5: per-region register typing.  Allocate parallel
    # ``object`` locals for any register that ever holds an object
    # ref.  Map IR reg index → CIL local index (int slot is just
    # the IR index; object slot is plan.local_count + offset).
    obj_regs, instr_types = _collect_object_typed_registers(
        region, plan.function_return_types,
    )
    obj_local_for: dict[int, int] = {
        reg_idx: plan.local_count + offset
        for offset, reg_idx in enumerate(sorted(obj_regs))
    }
    return_type = plan.function_return_types.get(region.name, "int32")

    if region.name != _program.entry_label:
        for index in range(call_register_count):
            builder.emit_ldarg(index)
            builder.emit_stloc(index)

    ctx = _RegionEmitContext(
        plan=plan,
        config=config,
        obj_local_for=obj_local_for,
        instr_types=instr_types,
        function_return_types=plan.function_return_types,
        return_type=return_type,
    )

    for index, instruction in enumerate(region.instructions):
        ctx.current_instr_index = index
        _emit_instruction(builder, instruction, config, plan, ctx=ctx)

    local_types = tuple("int32" for _ in range(plan.local_count)) + tuple(
        "object" for _ in range(len(obj_regs))
    )

    return CILMethodArtifact(
        name=region.name,
        body=builder.assemble(),
        max_stack=max(config.method_max_stack, call_register_count),
        local_types=local_types,
        return_type=return_type,
        parameter_types=tuple(
            "int32"
            for _ in range(
                call_register_count if region.name != _program.entry_label else 0
            )
        ),
    )


@dataclass
class _RegionEmitContext:
    """Per-region state threaded through ``_emit_instruction`` so
    closure-typed register reads/writes can pick the right slot
    (int32 vs object) without breaking the existing emission paths.

    For non-closure regions ``obj_local_for`` is empty and every
    register read/write goes through the int32 slot — exactly the
    pre-Phase-2c.5 behaviour.
    """

    plan: CILLoweringPlan
    config: CILBackendConfig
    obj_local_for: dict[int, int]
    instr_types: list[dict[int, str]]
    function_return_types: dict[str, str]
    return_type: str
    current_instr_index: int = 0

    def reg_type_after_current(self, reg_idx: int) -> str:
        """Type of ``reg_idx`` as of *after* the current
        instruction (i.e. once it's stored its result)."""
        if not self.instr_types:
            return "int32"
        return self.instr_types[self.current_instr_index].get(reg_idx, "int32")

    def reg_type_before_current(self, reg_idx: int) -> str:
        """Type of ``reg_idx`` BEFORE the current instruction
        runs — i.e. what slot to read from for an operand."""
        if self.current_instr_index == 0:
            return "int32"
        return self.instr_types[self.current_instr_index - 1].get(
            reg_idx, "int32",
        )


# CLR02 Phase 2c — closure body lowering.
#
# A closure's lifted lambda becomes ``Apply(int32) → int32`` on a
# per-lambda ``Closure_<name>`` TypeDef.  The instance method gets:
#
#   * Param 0 = ``this`` (the closure object)
#   * Param 1 = the single explicit argument (v1 supports arity-1 only)
#
# The IR body uses the captures-first register convention shared with
# the BEAM backend:
#
#   r2..r{1+num_free}                           — captures
#   r{2+num_free}                               — explicit arg (arity-1)
#
# The Apply method's prologue copies captures from instance fields
# and the explicit arg from ldarg.1 into those slots so the rest of
# the IR body's lowering works unmodified.
_CLR_CLOSURE_EXPLICIT_ARITY: int = 1
_REG_PARAM_BASE: int = 2


def _lower_closure_region(
    _program: IrProgram,
    config: CILBackendConfig,
    plan: CILLoweringPlan,
    region: _CallableRegion,
) -> CILMethodArtifact:
    builder = CILBytecodeBuilder()
    num_free = plan.closure_free_var_counts[region.name]

    # Prologue: captures from this.fieldI → r{2..1+num_free}
    for i in range(num_free):
        field_token = plan.token_provider.closure_field_token(region.name, i)
        builder.emit_ldarg(0)  # this
        builder.emit_token_instruction(0x7B, field_token)  # ldfld
        builder.emit_stloc(_REG_PARAM_BASE + i)

    # The single explicit arg (param 1) → r{2+num_free}.
    builder.emit_ldarg(1)
    builder.emit_stloc(_REG_PARAM_BASE + num_free)

    for instruction in region.instructions:
        _emit_instruction(builder, instruction, config, plan)

    # Apply has fixed name on every closure type (overrides the
    # IClosure interface's abstract method); the IR's region name
    # (e.g. ``_lambda_0``) becomes the TypeDef name instead.
    return CILMethodArtifact(
        name="Apply",
        body=builder.assemble(),
        max_stack=max(config.method_max_stack, 2),
        local_types=tuple("int32" for _ in range(plan.local_count)),
        return_type="int32",
        parameter_types=("int32",),
        is_instance=True,
    )


def _build_closure_extra_types(
    plan: CILLoweringPlan,
    closure_apply_methods: dict[str, CILMethodArtifact],
) -> tuple[CILTypeArtifact, ...]:
    """Build the IClosure interface + one Closure_<name> TypeDef per
    closure region.  Returns them in declaration order so the writer
    assigns matching MethodDef row indices.
    """
    if not plan.closure_names:
        return ()

    iclosure = CILTypeArtifact(
        name="IClosure",
        namespace="CodingAdventures",
        is_interface=True,
        extends=None,
        methods=(
            CILMethodArtifact(
                name="Apply",
                body=b"",
                max_stack=0,
                local_types=(),
                return_type="int32",
                parameter_types=("int32",),
                is_instance=True,
                is_abstract=True,
            ),
        ),
    )
    extras: list[CILTypeArtifact] = [iclosure]

    object_ctor_token = plan.token_provider.system_object_ctor_token()

    for closure_name in plan.closure_names:
        num_free = plan.closure_free_var_counts[closure_name]
        # Fields: capt0, capt1, ...
        fields = tuple(
            CILFieldArtifact(name=f"capt{i}", type="int32")
            for i in range(num_free)
        )
        ctor = _build_closure_ctor(
            num_free, object_ctor_token,
            field_token=lambda i, _name=closure_name: (
                plan.token_provider.closure_field_token(_name, i)
            ),
        )
        extras.append(
            CILTypeArtifact(
                name=f"Closure_{closure_name}",
                namespace="CodingAdventures",
                extends="System.Object",
                implements=("CodingAdventures.IClosure",),
                fields=fields,
                methods=(ctor, closure_apply_methods[closure_name]),
            )
        )

    return tuple(extras)


def _build_closure_ctor(
    num_free: int,
    object_ctor_token: int,
    *,
    field_token: Callable[[int], int],
) -> CILMethodArtifact:
    """Synthesise a ``.ctor(int32, ..., int32)`` body that chains
    into ``System.Object::.ctor()`` then stores each capture
    parameter into its instance field.
    """
    builder = CILBytecodeBuilder()
    # Chain into base ctor: ldarg.0; call instance void Object::.ctor()
    builder.emit_ldarg(0)
    builder.emit_call(object_ctor_token)
    # For each capture i: ldarg.0; ldarg.{1+i}; stfld capt_i
    for i in range(num_free):
        builder.emit_ldarg(0)
        builder.emit_ldarg(1 + i)
        builder.emit_token_instruction(0x7D, field_token(i))  # stfld
    builder.emit_ret()
    return CILMethodArtifact(
        name=".ctor",
        body=builder.assemble(),
        max_stack=2,
        local_types=(),
        return_type="void",
        parameter_types=tuple("int32" for _ in range(num_free)),
        is_instance=True,
        is_special_name=True,
    )


def _emit_instruction(
    builder: CILBytecodeBuilder,
    instruction: IrInstruction,
    config: CILBackendConfig,
    plan: CILLoweringPlan,
    *,
    ctx: _RegionEmitContext | None = None,
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

        # Phase 2c.5: ADD_IMM with imm=0 is the MOV idiom — when
        # the source is currently object-typed, the move must
        # propagate the obj-slot, not push int + 0 + add.
        if (
            instruction.opcode is IrOp.ADD_IMM
            and imm.value == 0
            and ctx is not None
            and ctx.reg_type_before_current(src.index) == "object"
        ):
            builder.emit_ldloc(ctx.obj_local_for[src.index])
            builder.emit_stloc(ctx.obj_local_for[dst.index])
            return

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
        # Phase 2c.5: if the callee returns an object (closure ref),
        # store the result into r1's object slot instead of its
        # int32 slot.  The callee's method signature already
        # declares ``return_type="object"`` so the verifier is
        # happy with the typed stloc.
        if ctx is not None:
            callee_return = ctx.function_return_types.get(label.name, "int32")
            if callee_return == "object":
                builder.emit_stloc(ctx.obj_local_for[1])
                return
        builder.emit_stloc(1)
        return

    if instruction.opcode in (IrOp.RET, IrOp.HALT):
        # Phase 2c.5: if the function returns object, ldloc from
        # r1's object slot rather than its int32 slot.
        if ctx is not None and ctx.return_type == "object":
            builder.emit_ldloc(ctx.obj_local_for[1])
        else:
            builder.emit_ldloc(1)
        builder.emit_ret()
        return

    if instruction.opcode == IrOp.MAKE_CLOSURE:
        # MAKE_CLOSURE dst, fn_label, num_captured, capt0, ..., captN-1
        # → ldloc capt0; ...; ldloc captN-1; newobj Closure_<fn>::.ctor(...);
        #   stloc dst
        if len(instruction.operands) < 3:
            msg = (
                f"MAKE_CLOSURE expects at least 3 operands "
                f"(dst, fn_label, num_captured, capt...), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "MAKE_CLOSURE dst")
        fn_label = _as_label(instruction.operands[1], "MAKE_CLOSURE fn_label")
        num_captured = _as_immediate(
            instruction.operands[2], "MAKE_CLOSURE num_captured"
        ).value
        if num_captured != plan.closure_free_var_counts.get(fn_label.name):
            msg = (
                f"MAKE_CLOSURE for {fn_label.name!r}: num_captured="
                f"{num_captured} but config.closure_free_var_counts says "
                f"{plan.closure_free_var_counts.get(fn_label.name)}"
            )
            raise CILBackendError(msg)
        if len(instruction.operands) != 3 + num_captured:
            msg = (
                f"MAKE_CLOSURE for {fn_label.name!r}: num_captured="
                f"{num_captured} but {len(instruction.operands) - 3} "
                "capture operands provided"
            )
            raise CILBackendError(msg)

        for i in range(num_captured):
            capt_reg = _as_register(
                instruction.operands[3 + i],
                f"MAKE_CLOSURE capture {i}",
            )
            builder.emit_ldloc(capt_reg.index)
        # newobj instance void Closure_<fn>::.ctor(int32, ...)
        builder.emit_token_instruction(
            0x73, plan.token_provider.closure_ctor_token(fn_label.name),
        )
        # Phase 2c.5: store the new closure ref into dst's object
        # slot (managed pointers can't fit in an int32 local).
        if ctx is not None and dst.index in ctx.obj_local_for:
            builder.emit_stloc(ctx.obj_local_for[dst.index])
        else:
            builder.emit_stloc(dst.index)
        return

    if instruction.opcode == IrOp.APPLY_CLOSURE:
        # APPLY_CLOSURE dst, closure_reg, num_args, arg0
        # → ldloc closure; ldloc arg0;
        #   callvirt instance int32 IClosure::Apply(int32);
        #   stloc dst
        # v1 supports exactly arity-1.
        if len(instruction.operands) < 3:
            msg = (
                f"APPLY_CLOSURE expects at least 3 operands "
                f"(dst, closure, num_args, args...), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "APPLY_CLOSURE dst")
        closure_reg = _as_register(
            instruction.operands[1], "APPLY_CLOSURE closure_reg"
        )
        num_args = _as_immediate(
            instruction.operands[2], "APPLY_CLOSURE num_args"
        ).value
        if num_args != _CLR_CLOSURE_EXPLICIT_ARITY:
            msg = (
                f"APPLY_CLOSURE: V1 CLR backend only supports "
                f"arity-{_CLR_CLOSURE_EXPLICIT_ARITY} closures, got "
                f"num_args={num_args}.  Multi-arity closures land in a "
                "later phase."
            )
            raise CILBackendError(msg)
        if len(instruction.operands) != 3 + num_args:
            msg = (
                f"APPLY_CLOSURE: num_args={num_args} but "
                f"{len(instruction.operands) - 3} arg operands provided"
            )
            raise CILBackendError(msg)
        arg_reg = _as_register(instruction.operands[3], "APPLY_CLOSURE arg 0")

        # Phase 2c.5: closure_reg is an object-typed register —
        # read it from the obj slot.  arg_reg is int32, dst is
        # int32 (apply returns int32).
        if ctx is not None and closure_reg.index in ctx.obj_local_for:
            builder.emit_ldloc(ctx.obj_local_for[closure_reg.index])
        else:
            builder.emit_ldloc(closure_reg.index)
        builder.emit_ldloc(arg_reg.index)
        builder.emit_callvirt(plan.token_provider.iclosure_apply_token())
        builder.emit_stloc(dst.index)
        return

    if instruction.opcode == IrOp.SYSCALL:
        number = _as_immediate(instruction.operands[0], "SYSCALL number")
        builder.emit_ldc_i4(number.value)
        builder.emit_ldloc(config.syscall_arg_reg)
        builder.emit_call(plan.token_provider.helper_token(CILHelper.SYSCALL))
        # Store the syscall return value into the register given by operands[1],
        # if present and a register (e.g. SYSCALL 2 / read stores its result into
        # a scratch register rather than the write-arg register).
        # Fall back to syscall_arg_reg when no result operand is present.
        result_reg: int
        if (
            len(instruction.operands) >= 2
            and isinstance(instruction.operands[1], IrRegister)
        ):
            result_reg = instruction.operands[1].index
        else:
            result_reg = config.syscall_arg_reg
        builder.emit_stloc(result_reg)
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
