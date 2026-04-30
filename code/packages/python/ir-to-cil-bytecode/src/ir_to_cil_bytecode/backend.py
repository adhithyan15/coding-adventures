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

    # ── TW03 Phase 3c — heap-primitive tokens ───────────────────────────
    #
    # When ``include_heap_types=True`` the provider also lays out the
    # Cons / Symbol / Nil TypeDef rows after the closure rows.  Each
    # type contributes one ``.ctor`` MethodDef row; Cons additionally
    # contributes head + tail Field rows; Symbol contributes a name
    # Field row.

    def heap_cons_ctor_token(self) -> int:
        """Return the MethodDef token for ``Cons::.ctor(int32, object)``."""

    def heap_cons_head_token(self) -> int:
        """Return the Field token for ``Cons::head : int32``."""

    def heap_cons_tail_token(self) -> int:
        """Return the Field token for ``Cons::tail : object``."""

    def heap_symbol_ctor_token(self) -> int:
        """Return the MethodDef token for ``Symbol::.ctor(string)``."""

    def heap_symbol_name_token(self) -> int:
        """Return the Field token for ``Symbol::name : string``."""

    def heap_nil_ctor_token(self) -> int:
        """Return the MethodDef token for ``Nil::.ctor()``."""

    def heap_cons_typedef_token(self) -> int:
        """Return the TypeDef token for ``Cons`` (used by ``isinst``)."""

    def heap_symbol_typedef_token(self) -> int:
        """Return the TypeDef token for ``Symbol`` (used by ``isinst``)."""

    def heap_nil_typedef_token(self) -> int:
        """Return the TypeDef token for ``Nil`` (used by ``isinst``)."""


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
        include_heap_types: bool = False,
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
        # OR any heap type is present (both need to chain into the
        # base ctor).  Row = helper count + 1.
        needs_object_ctor = bool(closure_names) or include_heap_types
        self._object_ctor = (
            0x0A000000 | (len(CILHelper) + 1) if needs_object_ctor else 0
        )

        # ── TW03 Phase 3c — heap-primitive token layout ────────────
        #
        # When ``include_heap_types`` is set, the writer appends three
        # extra TypeDef rows (Cons, Symbol, Nil) AFTER any closure
        # types.  Each contributes one ``.ctor`` MethodDef row in
        # declaration order; Cons additionally contributes head + tail
        # Field rows, Symbol contributes a name Field row.
        #
        # Method rows so far: M (main) + (1 + 2*K) closure rows
        # (IClosure.Apply + per-closure ctor/Apply).  Heap ctors
        # follow at offsets +0 (Cons), +1 (Symbol), +2 (Nil).
        #
        # Field rows so far: closure capture count.  Heap fields
        # follow at offsets +0 (Cons.head), +1 (Cons.tail),
        # +2 (Symbol.name).
        #
        # TypeDef rows: <Module>=1, MainType=2, then closure types
        # (IClosure + Closure_<name> per closure).  Heap types follow.
        self._include_heap_types = include_heap_types
        closure_method_rows = (1 + 2 * len(closure_names)) if closure_names else 0
        heap_method_base = 0x06000001 + m + closure_method_rows
        self._heap_cons_ctor = heap_method_base if include_heap_types else 0
        self._heap_symbol_ctor = (heap_method_base + 1) if include_heap_types else 0
        self._heap_nil_ctor = (heap_method_base + 2) if include_heap_types else 0

        heap_field_base = 0x04000001 + field_row
        self._heap_cons_head = heap_field_base if include_heap_types else 0
        self._heap_cons_tail = (heap_field_base + 1) if include_heap_types else 0
        self._heap_symbol_name = (heap_field_base + 2) if include_heap_types else 0

        # TypeDef rows: <Module>=1, MainType=2.  Then closures (if
        # any) contribute 1 + len(closure_names) (IClosure +
        # Closure_<name>...).  Heap types follow.
        closure_type_rows = (1 + len(closure_names)) if closure_names else 0
        heap_typedef_base = 0x02000003 + closure_type_rows
        self._heap_cons_typedef = heap_typedef_base if include_heap_types else 0
        self._heap_symbol_typedef = (
            (heap_typedef_base + 1) if include_heap_types else 0
        )
        self._heap_nil_typedef = (
            (heap_typedef_base + 2) if include_heap_types else 0
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

    # ── Heap-primitive token getters (TW03 Phase 3c) ────────────────────

    def _heap_token(self, value: int, name: str) -> int:
        if not value:
            msg = (
                f"heap token {name!r} unavailable — "
                "include_heap_types=True must be set on the token provider"
            )
            raise CILBackendError(msg)
        return value

    def heap_cons_ctor_token(self) -> int:
        return self._heap_token(self._heap_cons_ctor, "cons_ctor")

    def heap_cons_head_token(self) -> int:
        return self._heap_token(self._heap_cons_head, "cons_head")

    def heap_cons_tail_token(self) -> int:
        return self._heap_token(self._heap_cons_tail, "cons_tail")

    def heap_symbol_ctor_token(self) -> int:
        return self._heap_token(self._heap_symbol_ctor, "symbol_ctor")

    def heap_symbol_name_token(self) -> int:
        return self._heap_token(self._heap_symbol_name, "symbol_name")

    def heap_nil_ctor_token(self) -> int:
        return self._heap_token(self._heap_nil_ctor, "nil_ctor")

    def heap_cons_typedef_token(self) -> int:
        return self._heap_token(self._heap_cons_typedef, "cons_typedef")

    def heap_symbol_typedef_token(self) -> int:
        return self._heap_token(self._heap_symbol_typedef, "symbol_typedef")

    def heap_nil_typedef_token(self) -> int:
        return self._heap_token(self._heap_nil_typedef, "nil_typedef")


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
    # TW03 Phase 3 follow-up: per-region parameter types — maps
    # region name → tuple of "int32" or "object" per param slot.
    # Computed by ``_classify_function_parameter_types`` once
    # obj_regs is known per region.  CALL sites consult this to
    # ldloc args from the right slot; function entries consult it
    # to starg into the right slot.
    function_parameter_types: dict[str, tuple[str, ...]] = field(
        default_factory=dict,
    )


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

        # TW03 Phase 3c — auto-include Cons/Symbol/Nil TypeDefs when
        # the program uses any heap opcode.  Programs without heap ops
        # see zero extra TypeDef rows.
        uses_heap = any(
            i.opcode in _HEAP_OPCODES for i in program.instructions
        )
        if uses_heap:
            extra_types = extra_types + _build_heap_extra_types(plan)

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
    # TW03 Phase 3c — heap primitives (cons / symbol / nil).
    IrOp.MAKE_CONS,
    IrOp.CAR,
    IrOp.CDR,
    IrOp.IS_NULL,
    IrOp.IS_PAIR,
    IrOp.MAKE_SYMBOL,
    IrOp.IS_SYMBOL,
    IrOp.LOAD_NIL,
})


# TW03 Phase 3c — opcode set that triggers the heap-primitive runtime
# TypeDefs (Cons / Symbol / Nil) auto-include in the multi-TypeDef
# assembly artifact.  Programs without these ops see zero extra
# TypeDef rows.
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

    # TW03 Phase 3c: detect heap-primitive ops so the token provider
    # also lays out Cons / Symbol / Nil typedef + method + field
    # tokens after the closure rows.
    uses_heap = any(
        i.opcode in _HEAP_OPCODES for i in program.instructions
    )

    provider = token_provider or SequentialCILTokenProvider(
        main_region_names,
        closure_names=closure_names,
        closure_free_var_counts=dict(config.closure_free_var_counts),
        include_heap_types=uses_heap,
    )

    function_return_types = _classify_function_return_types(
        regions, set(config.closure_free_var_counts),
    )

    # TW03 Phase 3 follow-up: classify per-region parameter types so
    # CALL sites can ldloc obj args from the obj slot and function
    # entries can starg into the obj slot.
    call_register_count = (
        config.call_register_count
        if config.call_register_count is not None
        else local_count
    )
    function_parameter_types = _classify_function_parameter_types(
        regions=regions,
        closure_region_names=set(config.closure_free_var_counts),
        return_types=function_return_types,
        call_register_count=call_register_count,
        entry_label=program.entry_label,
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
        function_parameter_types=function_parameter_types,
    )


def _classify_function_parameter_types(
    *,
    regions: tuple[_CallableRegion, ...],
    closure_region_names: set[str],
    return_types: dict[str, str],
    call_register_count: int,
    entry_label: str,
) -> dict[str, tuple[str, ...]]:
    """For each non-closure region (other than the entry point),
    classify each parameter slot as ``"int32"`` or ``"object"``.

    The classifier reuses ``_collect_object_typed_registers`` to
    discover which register indices are obj-typed in the body.
    Param slots 0..call_register_count-1 that appear in obj_regs
    get declared ``"object"``; everything else is ``"int32"``.

    Closure regions are skipped — their parameter types are
    governed by the ``IClosure::Apply(int32) → int32`` interface
    contract (always int32).  The entry-point region also has no
    explicit params (its descriptor is ``()V`` from main()).
    """
    out: dict[str, tuple[str, ...]] = {}
    for region in regions:
        if region.name in closure_region_names:
            continue
        if region.name == entry_label:
            out[region.name] = ()
            continue
        obj_regs, _ = _collect_object_typed_registers(region, return_types)
        out[region.name] = tuple(
            "object" if i in obj_regs else "int32"
            for i in range(call_register_count)
        )
    return out


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
    # TW03 Phase 3c — heap-primitive type-tracking rules.  Same shape
    # as JVM Phase 3b: CAR reads the int head; CDR / MAKE_CONS /
    # MAKE_SYMBOL / LOAD_NIL produce object refs; IS_NULL / IS_PAIR /
    # IS_SYMBOL produce int32 0/1 results ready for BRANCH_Z.
    elif op in (
        IrOp.MAKE_CONS, IrOp.CDR, IrOp.MAKE_SYMBOL, IrOp.LOAD_NIL,
    ):
        dst = _as_register(instr.operands[0], f"{op.name} dst")
        new_types[dst.index] = "object"
    elif op in (IrOp.CAR, IrOp.IS_NULL, IrOp.IS_PAIR, IrOp.IS_SYMBOL):
        dst = _as_register(instr.operands[0], f"{op.name} dst")
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
    *,
    seed_types: dict[int, str] | None = None,
) -> tuple[set[int], list[dict[int, str]]]:
    """For ``region``, compute (a) the set of IR register indices
    that ever hold an object ref, and (b) a per-instruction list
    of register-type maps (state AFTER each instruction).

    The per-instruction map lets the lowerer choose the correct
    slot (int32 vs object) for each register at each program
    point.

    Two sources contribute to ``obj_regs``:

    * **Writes** that produce object refs (``MAKE_CONS``,
      ``MAKE_SYMBOL``, ``LOAD_NIL``, ``CDR``, ``MAKE_CLOSURE``,
      ``CALL`` of an object-returning function, ``ADD_IMM 0`` from
      an object-typed source).  Tracked by
      ``_instr_register_type_writes``.
    * **Reads** by ops that consume an object operand (``CAR``,
      ``CDR``, ``IS_NULL``, ``IS_PAIR``, ``IS_SYMBOL``,
      ``MAKE_CONS``'s tail operand, ``APPLY_CLOSURE``'s closure
      operand).  Tracked by ``_instr_obj_source_reads``.

    The reads pass exists because **parameter slots** receive
    values written by the caller, never by the body — so if the
    body only READS them as obj refs (e.g. ``length``'s ``xs``
    param consumed by ``CDR``), the writes-only inference would
    miss them and the lowerer would emit ``ldloc`` from the
    int32 slot.
    """
    obj_regs: set[int] = set()
    instr_types: list[dict[int, str]] = []
    # Seed with parameter types so the body's first ADD_IMM-0
    # propagates the obj slot when the source is an obj-typed
    # parameter (e.g. ``ADD_IMM v10, v2, 0`` at the top of
    # ``length`` where v2 is the obj-typed xs param).  Without
    # the seed, the type pool would default v2 to int32 and the
    # move would copy garbage from the int slot.
    types: dict[int, str] = dict(seed_types or {})
    for reg_idx, type_name in types.items():
        if type_name == "object":
            obj_regs.add(reg_idx)
    for instr in region.instructions:
        types = _instr_register_type_writes(instr, types, return_types)
        instr_types.append(types)
        for reg_idx, type_name in types.items():
            if type_name == "object":
                obj_regs.add(reg_idx)
        # Obj-source reads also contribute (catches obj-typed
        # parameter slots that the body only ever reads).
        for reg_idx in _instr_obj_source_reads(instr):
            obj_regs.add(reg_idx)

    # Back-propagate through ADD_IMM-0 (the move idiom) to a
    # fixed point.  If a register is obj-typed and reaches it via
    # ``ADD_IMM dst, src, 0`` chains, the source registers must
    # also be obj-typed — otherwise their slot would be read from
    # the int pool and the move would propagate junk.
    #
    # Concrete pattern: ``length(xs)`` opens with
    # ``ADD_IMM v10, v2, 0`` (Twig copies the param into a holding
    # reg).  v10 gets classified obj (read by CDR), but v2 — the
    # parameter — needs the back-prop pass to also become obj so
    # the function signature declares it as object and the entry
    # shuffle stores ldarg into v2's obj slot.
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
            if not isinstance(instr.operands[0], IrRegister):
                continue
            if not isinstance(instr.operands[1], IrRegister):
                continue
            if not isinstance(instr.operands[2], IrImmediate):
                continue
            if instr.operands[2].value != 0:
                continue
            dst_idx = instr.operands[0].index
            src_idx = instr.operands[1].index
            if dst_idx in obj_regs and src_idx not in obj_regs:
                obj_regs.add(src_idx)
                changed = True
    return obj_regs, instr_types


def _instr_obj_source_reads(instr: IrInstruction) -> set[int]:
    """Return register indices that ``instr`` reads as an object ref.

    Distinct from ``_instr_register_type_writes`` (which tracks
    written types) because the type pool needs to know about
    read-as-obj registers too — most importantly parameter slots
    that arrive from the caller and are only ever read.
    """
    op = instr.opcode
    if op in (IrOp.CAR, IrOp.CDR, IrOp.IS_NULL, IrOp.IS_PAIR, IrOp.IS_SYMBOL):
        # 2 operands: dst, src — src is obj.
        if len(instr.operands) >= 2 and isinstance(
            instr.operands[1], IrRegister,
        ):
            return {instr.operands[1].index}
    elif op is IrOp.MAKE_CONS:
        # MAKE_CONS dst, head_int, tail_obj — only tail is obj.
        if len(instr.operands) >= 3 and isinstance(
            instr.operands[2], IrRegister,
        ):
            return {instr.operands[2].index}
    elif op is IrOp.APPLY_CLOSURE:
        # APPLY_CLOSURE dst, closure_reg, num_args, args... — closure_reg is obj.
        if len(instr.operands) >= 2 and isinstance(
            instr.operands[1], IrRegister,
        ):
            return {instr.operands[1].index}
    return set()


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
    #
    # Seed the analysis with the region's classified parameter
    # types so reads of obj-typed param slots see "object" in the
    # type pool from instruction 0 — needed for ADD_IMM-0 obj
    # propagation right at the top of the body (the standard Twig
    # idiom of moving the param into a holding reg).
    seed = {
        i: t
        for i, t in enumerate(plan.function_parameter_types.get(region.name, ()))
        if t == "object"
    }
    obj_regs, instr_types = _collect_object_typed_registers(
        region, plan.function_return_types, seed_types=seed,
    )
    obj_local_for: dict[int, int] = {
        reg_idx: plan.local_count + offset
        for offset, reg_idx in enumerate(sorted(obj_regs))
    }
    return_type = plan.function_return_types.get(region.name, "int32")
    # Per-param typing — slot N is "object" iff N is obj-typed in
    # the body (so the body's reads of that slot go through the
    # obj_local_for path).  Entry shuffle below stores each arg
    # into the matching slot.
    region_param_types = plan.function_parameter_types.get(region.name, ())

    if region.name != _program.entry_label:
        for index in range(call_register_count):
            builder.emit_ldarg(index)
            if (
                index < len(region_param_types)
                and region_param_types[index] == "object"
                and index in obj_local_for
            ):
                builder.emit_stloc(obj_local_for[index])
            else:
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

    if region.name == _program.entry_label:
        parameter_types: tuple[str, ...] = ()
    else:
        # Per-region: int32 by default, "object" for slots the body
        # treats as obj-typed (e.g. a cons-cell parameter consumed
        # by ``CDR``).  Falls back to all-int32 if the plan didn't
        # classify (back-compat with callers that don't compute
        # per-region param types).
        parameter_types = region_param_types or tuple(
            "int32" for _ in range(call_register_count)
        )

    return CILMethodArtifact(
        name=region.name,
        body=builder.assemble(),
        max_stack=max(config.method_max_stack, call_register_count),
        local_types=local_types,
        return_type=return_type,
        parameter_types=parameter_types,
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


def _build_heap_extra_types(
    plan: CILLoweringPlan,
) -> tuple[CILTypeArtifact, ...]:
    """Build the Cons / Symbol / Nil TypeArtifacts (TW03 Phase 3c).

    Three TypeDefs are appended after any closure types:

    * ``Cons`` — fields ``int32 head`` + ``object tail``; ctor takes
      both and stores them.
    * ``Symbol`` — field ``string name``; ctor takes a string and
      stores it.
    * ``Nil`` — empty class with a no-arg ctor; used as the
      ``isinst`` target for ``IS_NULL``.

    Each ctor chains into ``System.Object::.ctor()`` first, matching
    the closure-ctor pattern.

    The token provider must have been constructed with
    ``include_heap_types=True`` so the field/method tokens line up
    with the row indices the writer will assign.
    """
    object_ctor_token = plan.token_provider.system_object_ctor_token()

    # ── Cons ────────────────────────────────────────────────────────
    cons_ctor_builder = CILBytecodeBuilder()
    # ldarg.0; call Object::.ctor()
    cons_ctor_builder.emit_ldarg(0)
    cons_ctor_builder.emit_call(object_ctor_token)
    # ldarg.0; ldarg.1; stfld head
    cons_ctor_builder.emit_ldarg(0)
    cons_ctor_builder.emit_ldarg(1)
    cons_ctor_builder.emit_token_instruction(
        0x7D, plan.token_provider.heap_cons_head_token(),
    )
    # ldarg.0; ldarg.2; stfld tail
    cons_ctor_builder.emit_ldarg(0)
    cons_ctor_builder.emit_ldarg(2)
    cons_ctor_builder.emit_token_instruction(
        0x7D, plan.token_provider.heap_cons_tail_token(),
    )
    cons_ctor_builder.emit_ret()
    cons_ctor = CILMethodArtifact(
        name=".ctor",
        body=cons_ctor_builder.assemble(),
        max_stack=2,
        local_types=(),
        return_type="void",
        parameter_types=("int32", "object"),
        is_instance=True,
        is_special_name=True,
    )
    cons_type = CILTypeArtifact(
        name="Cons",
        namespace="CodingAdventures",
        extends="System.Object",
        fields=(
            CILFieldArtifact(name="head", type="int32"),
            CILFieldArtifact(name="tail", type="object"),
        ),
        methods=(cons_ctor,),
    )

    # ── Symbol ──────────────────────────────────────────────────────
    sym_ctor_builder = CILBytecodeBuilder()
    sym_ctor_builder.emit_ldarg(0)
    sym_ctor_builder.emit_call(object_ctor_token)
    sym_ctor_builder.emit_ldarg(0)
    sym_ctor_builder.emit_ldarg(1)
    sym_ctor_builder.emit_token_instruction(
        0x7D, plan.token_provider.heap_symbol_name_token(),
    )
    sym_ctor_builder.emit_ret()
    sym_ctor = CILMethodArtifact(
        name=".ctor",
        body=sym_ctor_builder.assemble(),
        max_stack=2,
        local_types=(),
        return_type="void",
        parameter_types=("string",),
        is_instance=True,
        is_special_name=True,
    )
    symbol_type = CILTypeArtifact(
        name="Symbol",
        namespace="CodingAdventures",
        extends="System.Object",
        fields=(CILFieldArtifact(name="name", type="string"),),
        methods=(sym_ctor,),
    )

    # ── Nil ─────────────────────────────────────────────────────────
    nil_ctor_builder = CILBytecodeBuilder()
    nil_ctor_builder.emit_ldarg(0)
    nil_ctor_builder.emit_call(object_ctor_token)
    nil_ctor_builder.emit_ret()
    nil_ctor = CILMethodArtifact(
        name=".ctor",
        body=nil_ctor_builder.assemble(),
        max_stack=1,
        local_types=(),
        return_type="void",
        parameter_types=(),
        is_instance=True,
        is_special_name=True,
    )
    nil_type = CILTypeArtifact(
        name="Nil",
        namespace="CodingAdventures",
        extends="System.Object",
        fields=(),
        methods=(nil_ctor,),
    )

    return (cons_type, symbol_type, nil_type)


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
        # TW03 Phase 3 follow-up: per-arg ldloc picks int32 vs object
        # slot based on the CALLEE's parameter typing.  Without this,
        # cons / symbol / nil refs marshalled into a param slot via
        # ``ADD_IMM dst, src, 0`` would be ldloc'd from the int slot
        # (= 0) and the recursive call would receive a null reference.
        callee_param_types: tuple[str, ...] = ()
        if plan.function_parameter_types:
            callee_param_types = plan.function_parameter_types.get(
                label.name, (),
            )
        for index in range(_call_register_count(config, plan)):
            wants_obj = (
                index < len(callee_param_types)
                and callee_param_types[index] == "object"
            )
            if wants_obj and ctx is not None and index in ctx.obj_local_for:
                builder.emit_ldloc(ctx.obj_local_for[index])
            elif wants_obj:
                # Caller doesn't have this slot in its obj pool — pass
                # null.  In well-formed Twig output this shouldn't
                # happen because the frontend always emits an
                # obj-propagating move into the param slot first.
                builder.emit_opcode(0x14)  # ldnull
            else:
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

    # ── TW03 Phase 3c — heap-primitive lowering ─────────────────────────
    #
    # Each op picks the int32 vs object slot for its register reads/
    # writes via ``ctx.obj_local_for`` (mirrors the closure pattern).
    # CIL opcode bytes used:
    #   0x14  ldnull              (one byte)
    #   0x73  newobj <token>
    #   0x74  castclass <token>
    #   0x75  isinst <token>
    #   0x7B  ldfld <token>
    #   0xFE 0x03  cgt.un          (two bytes)
    if instruction.opcode == IrOp.MAKE_CONS:
        if len(instruction.operands) != 3:
            msg = (
                f"MAKE_CONS expects 3 operands (dst, head, tail), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "MAKE_CONS dst")
        head = _as_register(instruction.operands[1], "MAKE_CONS head")
        tail = _as_register(instruction.operands[2], "MAKE_CONS tail")
        # head: int32 (from int slot); tail: object (from obj slot)
        builder.emit_ldloc(head.index)
        if ctx is not None and tail.index in ctx.obj_local_for:
            builder.emit_ldloc(ctx.obj_local_for[tail.index])
        else:
            builder.emit_ldloc(tail.index)
        builder.emit_token_instruction(
            0x73, plan.token_provider.heap_cons_ctor_token(),
        )
        # store ref into dst's object slot
        if ctx is not None and dst.index in ctx.obj_local_for:
            builder.emit_stloc(ctx.obj_local_for[dst.index])
        else:
            builder.emit_stloc(dst.index)
        return

    if instruction.opcode == IrOp.CAR:
        if len(instruction.operands) != 2:
            msg = (
                f"CAR expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "CAR dst")
        src = _as_register(instruction.operands[1], "CAR src")
        if ctx is not None and src.index in ctx.obj_local_for:
            builder.emit_ldloc(ctx.obj_local_for[src.index])
        else:
            builder.emit_ldloc(src.index)
        builder.emit_token_instruction(
            0x74, plan.token_provider.heap_cons_typedef_token(),
        )
        builder.emit_token_instruction(
            0x7B, plan.token_provider.heap_cons_head_token(),
        )
        builder.emit_stloc(dst.index)  # int32 result → int slot
        return

    if instruction.opcode == IrOp.CDR:
        if len(instruction.operands) != 2:
            msg = (
                f"CDR expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "CDR dst")
        src = _as_register(instruction.operands[1], "CDR src")
        if ctx is not None and src.index in ctx.obj_local_for:
            builder.emit_ldloc(ctx.obj_local_for[src.index])
        else:
            builder.emit_ldloc(src.index)
        builder.emit_token_instruction(
            0x74, plan.token_provider.heap_cons_typedef_token(),
        )
        builder.emit_token_instruction(
            0x7B, plan.token_provider.heap_cons_tail_token(),
        )
        if ctx is not None and dst.index in ctx.obj_local_for:
            builder.emit_stloc(ctx.obj_local_for[dst.index])
        else:
            builder.emit_stloc(dst.index)
        return

    if instruction.opcode in (IrOp.IS_NULL, IrOp.IS_PAIR, IrOp.IS_SYMBOL):
        op_name = instruction.opcode.name
        if len(instruction.operands) != 2:
            msg = (
                f"{op_name} expects 2 operands (dst, src), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], f"{op_name} dst")
        src = _as_register(instruction.operands[1], f"{op_name} src")
        if instruction.opcode is IrOp.IS_NULL:
            type_token = plan.token_provider.heap_nil_typedef_token()
        elif instruction.opcode is IrOp.IS_PAIR:
            type_token = plan.token_provider.heap_cons_typedef_token()
        else:
            type_token = plan.token_provider.heap_symbol_typedef_token()
        if ctx is not None and src.index in ctx.obj_local_for:
            builder.emit_ldloc(ctx.obj_local_for[src.index])
        else:
            builder.emit_ldloc(src.index)
        # isinst Type; ldnull; cgt.un  → 1 if src is Type, else 0.
        builder.emit_token_instruction(0x75, type_token)
        builder.emit_opcode(0x14)  # ldnull
        builder.emit_raw(bytes([0xFE, 0x03]))  # cgt.un
        builder.emit_stloc(dst.index)  # int32 result
        return

    if instruction.opcode == IrOp.MAKE_SYMBOL:
        if len(instruction.operands) != 2:
            msg = (
                f"MAKE_SYMBOL expects 2 operands (dst, name_label), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "MAKE_SYMBOL dst")
        # Phase 3c structural: pass null as the name argument.  The
        # ldstr UserString wiring lands in 3c.5 along with the
        # writer-side intern table — until then, two MAKE_SYMBOL
        # calls with the same label produce DIFFERENT Symbol
        # instances (semantically wrong but bytecode-shape correct).
        builder.emit_opcode(0x14)  # ldnull (placeholder for the name string)
        builder.emit_token_instruction(
            0x73, plan.token_provider.heap_symbol_ctor_token(),
        )
        if ctx is not None and dst.index in ctx.obj_local_for:
            builder.emit_stloc(ctx.obj_local_for[dst.index])
        else:
            builder.emit_stloc(dst.index)
        return

    if instruction.opcode == IrOp.LOAD_NIL:
        if len(instruction.operands) != 1:
            msg = (
                f"LOAD_NIL expects 1 operand (dst), got "
                f"{len(instruction.operands)}"
            )
            raise CILBackendError(msg)
        dst = _as_register(instruction.operands[0], "LOAD_NIL dst")
        # Phase 3c structural: newobj Nil.ctor() each time.  The
        # singleton-INSTANCE wire-up lands in 3c.5; until then
        # IS_NULL still works correctly via isinst Nil because
        # every Nil instance qualifies as null.
        builder.emit_token_instruction(
            0x73, plan.token_provider.heap_nil_ctor_token(),
        )
        if ctx is not None and dst.index in ctx.obj_local_for:
            builder.emit_stloc(ctx.obj_local_for[dst.index])
        else:
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
