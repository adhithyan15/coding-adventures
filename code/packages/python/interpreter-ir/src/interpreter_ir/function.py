"""IIRFunction — a named, parameterised sequence of IIRInstr.

A function is the unit of compilation in the LANG pipeline.  The JIT compiles
one function at a time; the profiler tracks call counts per function.

Type status
-----------
``FunctionTypeStatus`` mirrors the status used by Tetrad (TET02b) and will be
the shared enum across all languages:

- ``FULLY_TYPED``     — every instruction has a concrete ``type_hint``
- ``PARTIALLY_TYPED`` — some instructions are typed, some are ``"any"``
- ``UNTYPED``         — all instructions are ``"any"``

The JIT uses this to decide *when* to compile:

- ``FULLY_TYPED``     → compile before the first interpreted call
- ``PARTIALLY_TYPED`` → compile after 10 interpreter calls
- ``UNTYPED``         → compile after 100 interpreter calls

Example::

    fn = IIRFunction(
        name="add",
        params=[("a", "u8"), ("b", "u8")],
        return_type="u8",
        instructions=[
            IIRInstr("add", "v0", ["a", "b"], "u8"),
            IIRInstr("ret",  None, ["v0"],    "u8"),
        ],
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto

from interpreter_ir.exception_table import ExceptionTableEntry
from interpreter_ir.instr import IIRInstr


class FunctionTypeStatus(Enum):
    """Compilation tier based on how much type information is available."""

    FULLY_TYPED = auto()
    """All parameters and instructions carry concrete type hints."""

    PARTIALLY_TYPED = auto()
    """Some instructions are typed; others are ``"any"``."""

    UNTYPED = auto()
    """No static types; all type information must come from profiling."""


@dataclass
class IIRFunction:
    """A single function in an IIRModule.

    Parameters
    ----------
    name:
        The function name, used as the key in the JIT cache and the vm-core
        call-count metrics table.
    params:
        Ordered list of ``(param_name, type_hint)`` pairs.  The VM loads
        arguments into the first ``len(params)`` registers before the first
        instruction executes.
    return_type:
        Declared return type (``"u8"``, ``"void"``, ``"any"``, …).
    instructions:
        The body of the function as a flat list of ``IIRInstr``.  Labels are
        also represented as instructions (op ``"label"``) so the index of each
        instruction is its unique address within the function.
    register_count:
        Number of VM registers this function uses (including parameters).
        ``vm-core`` allocates exactly this many register slots per frame.
    type_status:
        Compilation tier.  Derived automatically from ``params`` and
        ``instructions`` if not supplied explicitly.
    call_count:
        Incremented by ``vm-core`` on each interpreted call.  Read by
        ``jit-core`` to trigger tier promotion.
    """

    name: str
    params: list[tuple[str, str]]   # [(param_name, type_hint), …]
    return_type: str
    instructions: list[IIRInstr]
    register_count: int = 8
    type_status: FunctionTypeStatus = FunctionTypeStatus.UNTYPED
    call_count: int = field(default=0, repr=False, compare=False)

    # ----------------------------------------------------------------------
    # Optional frontend-owned side tables (LANG17 PR4).
    #
    # ``feedback_slots`` and ``source_map`` let language frontends carry
    # their own indexing schemes alongside the generic IIR.  vm-core does
    # not interpret either field — they are passive metadata that
    # frontends populate at compile time and read back at metric
    # observation time.
    # ----------------------------------------------------------------------

    feedback_slots: dict[int, int] = field(
        default_factory=dict, repr=False, compare=False
    )
    """Optional: ``slot_index → iir_instr_index`` mapping.

    Frontends that allocate named feedback slots at compile time (Tetrad,
    SpiderMonkey, V8) populate this dict so a slot index can be resolved
    back to the IIR instruction that owns it.  ``vm-core`` does not
    populate or read this field — it is the frontend's contract with
    its own metric APIs (e.g. ``TetradRuntime.feedback_vector(fn)``
    returns a list indexed by slot index).
    """

    source_map: list[tuple[int, int, int]] = field(
        default_factory=list, repr=False, compare=False
    )
    """Optional: ``(iir_index, source_a, source_b)`` triples.

    Frontends populate this with whatever indexing scheme they need on
    the read side.  Conventional uses:

    - ``(iir_index, source_line, source_column)`` — the LANG17 spec's
      default reading.  Debuggers map IIR back to source positions.
    - ``(iir_index, original_byte_code_ip, 0)`` — Tetrad's reading.
      Lets ``TetradRuntime.branch_profile(fn, tetrad_ip)`` re-key the
      generic IIR-IP-keyed counters back into Tetrad-IP space.

    The third field's meaning is frontend-defined; vm-core does not look
    at it.
    """

    # -----------------------------------------------------------------------
    # VMCOND00 Phase 2 — static exception table (Layer 2: Unwind Exceptions)
    # -----------------------------------------------------------------------
    #
    # The exception table is a flat list of ``ExceptionTableEntry`` objects.
    # The THROW handler (in vm-core) walks this list linearly, checking each
    # entry's guarded range ``[from_ip, to_ip)`` against the IP where the
    # THROW occurred, then checking the condition type.  The first matching
    # entry wins (innermost-first ordering is the frontend's responsibility).
    #
    # Design choices:
    #
    # - Per-function (not per-module): matches JVM and CPython.  The VM only
    #   reads the table for the frame it is searching.
    # - compare=False: two IIRFunction objects with the same instructions but
    #   different exception tables are considered equal by value.  Exception
    #   tables are attached at compile time; they do not affect the logical
    #   program identity the equality check is used for (test deduplication,
    #   JIT cache keying).
    # - repr=False: keeps __repr__ concise; the table is rarely useful in
    #   debugging print output.
    # - Not serialised: the serialiser (serialise.py) does not write exception
    #   tables.  They are repopulated each time a frontend compiles source.
    #   This is the same policy as ``feedback_slots`` and ``source_map``.
    # -----------------------------------------------------------------------

    exception_table: list[ExceptionTableEntry] = field(
        default_factory=list, repr=False, compare=False
    )
    """Static exception table for VMCOND00 Layer 2 (Unwind Exceptions).

    A flat list of :class:`~interpreter_ir.exception_table.ExceptionTableEntry`
    objects describing guarded code ranges and their catch handlers.  The THROW
    handler in ``vm-core`` walks this list to find a matching entry when a
    ``throw`` instruction executes.

    Frontends populate this at compile time.  Each entry covers a half-open
    instruction-index range ``[from_ip, to_ip)`` within this function's
    instruction list.  Entries should be listed innermost-first (the THROW
    handler uses first-match wins).

    Functions that never throw leave this field empty — the THROW handler
    pops the frame immediately and propagates to the caller.

    Example — a try/catch that catches everything::

        from interpreter_ir.exception_table import ExceptionTableEntry, CATCH_ALL

        fn.exception_table = [
            ExceptionTableEntry(
                from_ip=1,    # instruction after the try-start label
                to_ip=4,      # instruction of the catch label (exclusive)
                handler_ip=4, # index of the catch-block entry-point label
                type_id=CATCH_ALL,
                val_reg="ex",
            )
        ]
    """

    # -----------------------------------------------------------------------
    # Helpers
    # -----------------------------------------------------------------------

    def param_names(self) -> list[str]:
        """Return just the parameter names in order."""
        return [name for name, _ in self.params]

    def param_types(self) -> list[str]:
        """Return just the parameter type hints in order."""
        return [t for _, t in self.params]

    def infer_type_status(self) -> FunctionTypeStatus:
        """Derive type status from params and instruction type hints.

        A function is FULLY_TYPED if every param type and every instruction
        ``type_hint`` is a concrete type (not ``"any"``).  It is UNTYPED if
        none of them are.  Otherwise PARTIALLY_TYPED.
        """
        from interpreter_ir.opcodes import CONCRETE_TYPES

        all_hints: list[str] = [t for _, t in self.params] + [
            i.type_hint for i in self.instructions
        ]
        typed = sum(1 for h in all_hints if h in CONCRETE_TYPES)
        total = len(all_hints)
        if total == 0 or typed == 0:
            return FunctionTypeStatus.UNTYPED
        if typed == total:
            return FunctionTypeStatus.FULLY_TYPED
        return FunctionTypeStatus.PARTIALLY_TYPED

    def label_index(self, label_name: str) -> int:
        """Return the instruction index of the named label, or raise KeyError."""
        for idx, instr in enumerate(self.instructions):
            if instr.op == "label" and instr.srcs and instr.srcs[0] == label_name:
                return idx
        raise KeyError(f"label {label_name!r} not found in function {self.name!r}")

    def __repr__(self) -> str:
        params_str = ", ".join(f"{n}: {t}" for n, t in self.params)
        return (
            f"IIRFunction({self.name!r}, "
            f"params=[{params_str}], "
            f"return={self.return_type!r}, "
            f"instrs={len(self.instructions)}, "
            f"status={self.type_status.name})"
        )
