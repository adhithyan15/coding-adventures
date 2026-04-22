"""Tetrad Register VM (spec TET04) — built on GenericRegisterVM.

This package implements the Tetrad VM as a thin language backend on top of
the ``register_vm.GenericRegisterVM`` chassis.  The chassis provides:

  * Fetch-decode-execute loop with pluggable opcode dispatch
  * Per-instruction trace hook (``trace_builder``)
  * Recursive frame management for function calls

The ``TetradVM`` class registers one handler per Tetrad opcode and provides
the Tetrad-specific public API: feedback vectors, branch statistics, loop
iteration counts, and the immediate-JIT queue.

Why this architecture?
----------------------
Writing a separate VM loop for each language produces a maintenance burden
that scales badly.  The Tetrad pipeline will eventually be followed by other
interpreted languages (Lisp, μScheme, …).  Each one needs the same primitives:
accumulator, register file, tracing, debugger hooks.  By delegating all of
that to ``GenericRegisterVM``, we get:

  * **One place for debugger support** — the ``trace_builder`` hook and
    any future breakpoint mechanism live in register-vm, not duplicated here.
  * **Thin language backends** — TetradVM is ~handler-registration code +
    public metrics API.  No dispatch loop.
  * **Composable tracing** — ``execute_traced`` sets ``trace_builder`` on
    the shared chassis, so nested function calls automatically appear in
    the same trace.

Call convention
---------------
Arguments are placed in R0..R(argc-1) by the caller before CALL.  At
function entry the preamble copies R[i] → locals[param_i] via
``LDA_REG i; STA_VAR i``.  All local variable accesses go through
``frame.user_data["locals"]``.  Global writes go to ``vm._globals``.

u8 semantics
------------
All arithmetic results are taken mod 256.  All register/accumulator values
are plain Python ``int``s; the handler enforces the mod-256 contract on write.

Public API
----------
::

    from tetrad_vm import TetradVM, VMError
    from tetrad_vm.metrics import VMMetrics, SlotState, SlotKind, BranchStats

    vm = TetradVM()
    result = vm.execute(code)
    result, trace = vm.execute_traced(code)

    hot   = vm.hot_functions(threshold=100)
    prof  = vm.type_profile("add", slot=0)
    loops = vm.loop_iterations("count_down")
    shape = vm.call_site_shape("main", slot=0)
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from register_vm.generic_vm import GenericRegisterVM, GenericVMError, RegisterFrame
from tetrad_compiler.bytecode import CodeObject, Instruction, Op

from tetrad_vm.metrics import BranchStats, SlotKind, SlotState, VMMetrics, VMTrace

__all__ = [
    "TetradVM",
    "VMError",
]


# ---------------------------------------------------------------------------
# Error type
# ---------------------------------------------------------------------------


class VMError(Exception):
    """Raised by Tetrad VM handlers for runtime errors.

    All error messages follow the format specified in TET04:
    ``<condition> at fn '<name>' ip N``
    """


# ---------------------------------------------------------------------------
# Feedback slot update helper
# ---------------------------------------------------------------------------


def _update_slot(slot: SlotState, ty: str) -> None:
    """Update a feedback slot's type profile in-place.

    State machine (V8 Ignition model):

        UNINITIALIZED → MONOMORPHIC   (first observation)
        MONOMORPHIC   → MONOMORPHIC   (same type again)
        MONOMORPHIC   → POLYMORPHIC   (2nd–4th distinct type)
        POLYMORPHIC   → MEGAMORPHIC   (5th+ distinct type)
        MEGAMORPHIC   → MEGAMORPHIC   (terminal)
    """
    slot.count += 1
    if slot.kind is SlotKind.MEGAMORPHIC:
        return
    if ty not in slot.observations:
        slot.observations.append(ty)
    n = len(slot.observations)
    if n == 1:
        slot.kind = SlotKind.MONOMORPHIC
    elif n <= 4:
        slot.kind = SlotKind.POLYMORPHIC
    else:
        slot.kind = SlotKind.MEGAMORPHIC


# ---------------------------------------------------------------------------
# TetradVM
# ---------------------------------------------------------------------------


class TetradVM:
    """Tetrad bytecode interpreter built on ``GenericRegisterVM``.

    All opcode semantics live in ``_h_*`` handler methods registered once at
    construction time.  The ``GenericRegisterVM`` chassis drives the dispatch
    loop.

    State that persists across ``execute()`` calls (until ``reset_metrics()``)
    --------------------------------------------------------------------------
    ``_metrics``          — instruction counts, function call counts, etc.
    ``_feedback_vectors`` — per-function type-feedback accumulation.

    State reset on each ``execute()`` call
    --------------------------------------
    ``_globals``    — global variable store.
    ``_main_code``  — the CodeObject passed to ``execute()``.
    """

    def __init__(
        self,
        *,
        io_in: Callable[[], int] | None = None,
        io_out: Callable[[int], None] | None = None,
    ) -> None:
        """Create a new TetradVM.

        ``io_in``  — callable returning a u8 for ``IO_IN`` instructions.
        ``io_out`` — callable accepting a u8 for ``IO_OUT`` instructions.
        """
        self._io_in: Callable[[], int] = io_in if io_in is not None else lambda: int(input())
        self._io_out: Callable[[int], None] = io_out if io_out is not None else print

        self._globals: dict[str, int] = {}
        self._feedback_vectors: dict[str, list[SlotState]] = {}
        self._metrics: VMMetrics = VMMetrics()
        self._main_code: CodeObject | None = None

        # Trace state (set in execute_traced, cleared after).
        self._trace_list: list[VMTrace] = []
        self._tracing: bool = False

        # Build the generic register VM and register all Tetrad handlers.
        self._grvm: GenericRegisterVM = GenericRegisterVM()
        self._register_handlers()

    # ------------------------------------------------------------------
    # Public execute API
    # ------------------------------------------------------------------

    def execute(self, code: CodeObject) -> int:
        """Execute a compiled Tetrad CodeObject; return the final accumulator.

        Resets ``_globals`` and ``_main_code`` before each run.
        Metrics and feedback vectors accumulate across calls.
        """
        self._globals = {}
        self._main_code = code

        for fn in code.functions:
            if fn.immediate_jit_eligible:
                self._metrics.immediate_jit_queue.append(fn.name)

        main_fv = self._get_or_create_fv(code)
        frame = RegisterFrame(
            instructions=code.instructions,
            ip=0,
            acc=0,
            registers=[0] * 8,
            depth=0,
            caller_frame=None,
            user_data={
                "feedback_vector": main_fv,
                "locals": {},
                "code": code,
            },
        )
        try:
            return self._grvm.run(frame)
        except GenericVMError as exc:
            raise VMError(f"unknown opcode: {exc}") from exc

    def execute_traced(self, code: CodeObject) -> tuple[int, list[VMTrace]]:
        """Execute with per-instruction ``VMTrace`` recording.

        The ``trace_builder`` hook on the GenericRegisterVM is installed for
        the duration of this call so that all nested function calls
        (dispatched recursively by the CALL handler) appear in the same trace.

        Returns ``(result, traces)``.
        """
        self._globals = {}
        self._main_code = code
        self._trace_list = []
        self._tracing = True

        for fn in code.functions:
            if fn.immediate_jit_eligible:
                self._metrics.immediate_jit_queue.append(fn.name)

        main_fv = self._get_or_create_fv(code)
        frame = RegisterFrame(
            instructions=code.instructions,
            ip=0,
            acc=0,
            registers=[0] * 8,
            depth=0,
            caller_frame=None,
            user_data={
                "feedback_vector": main_fv,
                "locals": {},
                "code": code,
            },
        )

        self._grvm.trace_builder = self._build_trace
        try:
            result = self._grvm.run(frame)
        except GenericVMError as exc:
            raise VMError(f"unknown opcode: {exc}") from exc
        finally:
            self._grvm.trace_builder = None
            self._tracing = False

        return result, list(self._trace_list)

    # ------------------------------------------------------------------
    # Metrics API
    # ------------------------------------------------------------------

    def hot_functions(self, threshold: int = 100) -> list[str]:
        """Return names of functions called at least ``threshold`` times."""
        return [
            name
            for name, count in self._metrics.function_call_counts.items()
            if count >= threshold
        ]

    def feedback_vector(self, fn_name: str) -> list[SlotState] | None:
        """Return the full feedback vector for a function, or None."""
        return self._feedback_vectors.get(fn_name)

    def type_profile(self, fn_name: str, slot: int) -> SlotState | None:
        """Return the SlotState for one feedback slot in one function."""
        fv = self._feedback_vectors.get(fn_name)
        if fv is None or slot >= len(fv):
            return None
        return fv[slot]

    def branch_profile(self, fn_name: str, slot: int) -> BranchStats | None:
        """Return branch stats for the JZ/JNZ instruction at IP ``slot``."""
        fn_branches = self._metrics.branch_stats.get(fn_name)
        if fn_branches is None:
            return None
        return fn_branches.get(slot)

    def loop_iterations(self, fn_name: str) -> dict[int, int]:
        """Return loop back-edge counts keyed by JMP_LOOP instruction IP."""
        return self._metrics.loop_back_edge_counts.get(fn_name, {})

    def call_site_shape(self, fn_name: str, slot: int) -> SlotKind:
        """Return the IC shape of one CALL feedback slot in ``fn_name``."""
        fv = self._feedback_vectors.get(fn_name)
        if fv is None or slot >= len(fv):
            return SlotKind.UNINITIALIZED
        return fv[slot].kind

    def metrics(self) -> VMMetrics:
        """Return the raw ``VMMetrics`` object."""
        return self._metrics

    def reset_metrics(self) -> None:
        """Reset all accumulated metrics and feedback vectors."""
        self._metrics = VMMetrics()
        self._feedback_vectors = {}

    # ------------------------------------------------------------------
    # Internal: feedback vector management
    # ------------------------------------------------------------------

    def _get_or_create_fv(self, code: CodeObject) -> list[SlotState]:
        """Return (creating if needed) the persistent feedback vector for ``code``."""
        if code.feedback_slot_count == 0:
            return []
        if code.name not in self._feedback_vectors:
            self._feedback_vectors[code.name] = [
                SlotState(kind=SlotKind.UNINITIALIZED, observations=[], count=0)
                for _ in range(code.feedback_slot_count)
            ]
        return self._feedback_vectors[code.name]

    # ------------------------------------------------------------------
    # Internal: trace builder (installed during execute_traced)
    # ------------------------------------------------------------------

    def _build_trace(
        self,
        frame: RegisterFrame,
        instr: Any,  # noqa: ANN401
        ip_before: int,
        acc_before: Any,  # noqa: ANN401
        regs_before: list[Any],
    ) -> None:
        """Post-instruction hook that builds a ``VMTrace`` with feedback delta."""
        fv_delta: list[tuple[int, SlotState]] = frame.user_data.pop("_fv_delta", [])
        code: CodeObject = frame.user_data.get("code", self._main_code)  # type: ignore[assignment]
        self._trace_list.append(VMTrace(
            frame_depth=frame.depth,
            fn_name=code.name if code else "<unknown>",
            ip=ip_before,
            instruction=instr,
            acc_before=acc_before,
            acc_after=frame.acc,
            registers_before=regs_before,
            registers_after=list(frame.registers),
            feedback_delta=fv_delta,
        ))

    # ------------------------------------------------------------------
    # Internal: feedback recording helper used by handlers
    # ------------------------------------------------------------------

    def _record_fv(
        self,
        frame: RegisterFrame,
        slot: int,
        ty: str,
    ) -> None:
        """Update feedback slot ``slot`` with type ``ty`` and record delta."""
        fv: list[SlotState] = frame.user_data.get("feedback_vector", [])
        if slot < len(fv):
            _update_slot(fv[slot], ty)
            if self._tracing:
                frame.user_data.setdefault("_fv_delta", []).append((slot, fv[slot]))

    # ------------------------------------------------------------------
    # Internal: metric recording helpers
    # ------------------------------------------------------------------

    def _count_instruction(self, opcode: int) -> None:
        self._metrics.total_instructions += 1
        self._metrics.instruction_counts[opcode] = (
            self._metrics.instruction_counts.get(opcode, 0) + 1
        )

    # ------------------------------------------------------------------
    # Handler registration
    # ------------------------------------------------------------------

    def _register_handlers(self) -> None:
        """Register all Tetrad opcode handlers on the GenericRegisterVM."""
        rh = self._grvm.register_handler
        rh(Op.LDA_IMM,      self._h_lda_imm)
        rh(Op.LDA_ZERO,     self._h_lda_zero)
        rh(Op.LDA_REG,      self._h_lda_reg)
        rh(Op.LDA_VAR,      self._h_lda_var)
        rh(Op.STA_REG,      self._h_sta_reg)
        rh(Op.STA_VAR,      self._h_sta_var)
        rh(Op.ADD,          self._h_add)
        rh(Op.SUB,          self._h_sub)
        rh(Op.MUL,          self._h_mul)
        rh(Op.DIV,          self._h_div)
        rh(Op.MOD,          self._h_mod)
        rh(Op.ADD_IMM,      self._h_add_imm)
        rh(Op.SUB_IMM,      self._h_sub_imm)
        rh(Op.AND,          self._h_and)
        rh(Op.OR,           self._h_or)
        rh(Op.XOR,          self._h_xor)
        rh(Op.NOT,          self._h_not)
        rh(Op.SHL,          self._h_shl)
        rh(Op.SHR,          self._h_shr)
        rh(Op.AND_IMM,      self._h_and_imm)
        rh(Op.EQ,           self._h_eq)
        rh(Op.NEQ,          self._h_neq)
        rh(Op.LT,           self._h_lt)
        rh(Op.LTE,          self._h_lte)
        rh(Op.GT,           self._h_gt)
        rh(Op.GTE,          self._h_gte)
        rh(Op.LOGICAL_NOT,  self._h_logical_not)
        rh(Op.LOGICAL_AND,  self._h_logical_and)
        rh(Op.LOGICAL_OR,   self._h_logical_or)
        rh(Op.JMP,          self._h_jmp)
        rh(Op.JZ,           self._h_jz)
        rh(Op.JNZ,          self._h_jnz)
        rh(Op.JMP_LOOP,     self._h_jmp_loop)
        rh(Op.CALL,         self._h_call)
        rh(Op.RET,          self._h_ret)
        rh(Op.IO_IN,        self._h_io_in)
        rh(Op.IO_OUT,       self._h_io_out)
        rh(Op.HALT,         self._h_halt)

    # ------------------------------------------------------------------
    # Opcode handlers — accumulator loads
    # ------------------------------------------------------------------

    def _h_lda_imm(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = instr.operands[0]

    def _h_lda_zero(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 0

    def _h_lda_reg(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = frame.registers[instr.operands[0]]

    def _h_lda_var(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        code: CodeObject = frame.user_data["code"]
        name = code.var_names[instr.operands[0]]
        locals_: dict[str, int] = frame.user_data["locals"]
        if name in locals_:
            frame.acc = locals_[name]
        elif name in self._globals:
            frame.acc = self._globals[name]
        else:
            raise VMError(
                f"undefined variable '{name}'"
                f" at fn '{code.name}' ip {frame.ip - 1}"
            )

    # ------------------------------------------------------------------
    # Opcode handlers — stores
    # ------------------------------------------------------------------

    def _h_sta_reg(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.registers[instr.operands[0]] = frame.acc

    def _h_sta_var(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        code: CodeObject = frame.user_data["code"]
        name = code.var_names[instr.operands[0]]
        locals_: dict[str, int] = frame.user_data["locals"]
        if name in locals_:
            locals_[name] = frame.acc
        else:
            self._globals[name] = frame.acc

    # ------------------------------------------------------------------
    # Opcode handlers — arithmetic (typed: 1 operand; untyped: 2 operands)
    # ------------------------------------------------------------------

    def _h_add(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        r = instr.operands[0]
        frame.acc = (frame.acc + frame.registers[r]) % 256
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_sub(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        r = instr.operands[0]
        frame.acc = (frame.acc - frame.registers[r]) % 256
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_mul(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        r = instr.operands[0]
        frame.acc = (frame.acc * frame.registers[r]) % 256
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_div(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        code: CodeObject = frame.user_data["code"]
        divisor = frame.registers[instr.operands[0]]
        if divisor == 0:
            raise VMError(
                f"division by zero at fn '{code.name}' ip {frame.ip - 1}"
            )
        frame.acc = frame.acc // divisor
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_mod(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        code: CodeObject = frame.user_data["code"]
        divisor = frame.registers[instr.operands[0]]
        if divisor == 0:
            raise VMError(
                f"division by zero at fn '{code.name}' ip {frame.ip - 1}"
            )
        frame.acc = frame.acc % divisor
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_add_imm(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = (frame.acc + instr.operands[0]) % 256
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_sub_imm(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = (frame.acc - instr.operands[0]) % 256
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    # ------------------------------------------------------------------
    # Opcode handlers — bitwise (always typed, no feedback slots)
    # ------------------------------------------------------------------

    def _h_and(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = frame.acc & frame.registers[instr.operands[0]]

    def _h_or(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = frame.acc | frame.registers[instr.operands[0]]

    def _h_xor(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = frame.acc ^ frame.registers[instr.operands[0]]

    def _h_not(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = (~frame.acc) & 0xFF

    def _h_shl(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = (frame.acc << frame.registers[instr.operands[0]]) & 0xFF

    def _h_shr(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = frame.acc >> frame.registers[instr.operands[0]]

    def _h_and_imm(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = frame.acc & instr.operands[0]

    # ------------------------------------------------------------------
    # Opcode handlers — comparisons (produce 0 or 1)
    # ------------------------------------------------------------------

    def _h_eq(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if frame.acc == frame.registers[instr.operands[0]] else 0
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_neq(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if frame.acc != frame.registers[instr.operands[0]] else 0
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_lt(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if frame.acc < frame.registers[instr.operands[0]] else 0
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_lte(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if frame.acc <= frame.registers[instr.operands[0]] else 0
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_gt(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if frame.acc > frame.registers[instr.operands[0]] else 0
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    def _h_gte(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if frame.acc >= frame.registers[instr.operands[0]] else 0
        if len(instr.operands) > 1:
            self._record_fv(frame, instr.operands[1], "u8")

    # ------------------------------------------------------------------
    # Opcode handlers — logical
    # ------------------------------------------------------------------

    def _h_logical_not(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 0 if frame.acc != 0 else 1

    def _h_logical_and(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if (frame.acc != 0 and frame.registers[instr.operands[0]] != 0) else 0

    def _h_logical_or(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = 1 if (frame.acc != 0 or frame.registers[instr.operands[0]] != 0) else 0

    # ------------------------------------------------------------------
    # Opcode handlers — control flow
    # ------------------------------------------------------------------

    def _h_jmp(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.ip += instr.operands[0]

    def _h_jz(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        ip_before = frame.ip - 1
        code: CodeObject = frame.user_data["code"]
        taken = frame.acc == 0
        if taken:
            frame.ip += instr.operands[0]
        fn_branches = self._metrics.branch_stats.setdefault(code.name, {})
        if ip_before not in fn_branches:
            fn_branches[ip_before] = BranchStats()
        if taken:
            fn_branches[ip_before].taken_count += 1
        else:
            fn_branches[ip_before].not_taken_count += 1

    def _h_jnz(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        ip_before = frame.ip - 1
        code: CodeObject = frame.user_data["code"]
        taken = frame.acc != 0
        if taken:
            frame.ip += instr.operands[0]
        fn_branches = self._metrics.branch_stats.setdefault(code.name, {})
        if ip_before not in fn_branches:
            fn_branches[ip_before] = BranchStats()
        if taken:
            fn_branches[ip_before].taken_count += 1
        else:
            fn_branches[ip_before].not_taken_count += 1

    def _h_jmp_loop(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        ip_before = frame.ip - 1
        code: CodeObject = frame.user_data["code"]
        frame.ip += instr.operands[0]
        fn_loops = self._metrics.loop_back_edge_counts.setdefault(code.name, {})
        fn_loops[ip_before] = fn_loops.get(ip_before, 0) + 1

    # ------------------------------------------------------------------
    # Opcode handlers — CALL / RET
    # ------------------------------------------------------------------

    def _h_call(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        assert self._main_code is not None
        func_idx, argc, slot = instr.operands
        if func_idx >= len(self._main_code.functions):
            code: CodeObject = frame.user_data["code"]
            raise VMError(
                f"undefined function index {func_idx} at fn '{code.name}'"
            )
        callee = self._main_code.functions[func_idx]
        expected = len(callee.params)
        if argc != expected:
            raise VMError(f"'{callee.name}' expects {expected} args, got {argc}")

        # Record call-site feedback (always "u8" in Tetrad v1).
        self._record_fv(frame, slot, "u8")

        # Track hot function detection.
        self._metrics.function_call_counts[callee.name] = (
            self._metrics.function_call_counts.get(callee.name, 0) + 1
        )

        # Enforce the 4004-inspired 4-frame depth limit.
        if frame.depth >= 3:
            raise VMError("call stack overflow: max depth 4 exceeded")

        # Copy argument registers from caller into callee's register file.
        new_regs = [frame.registers[i] if i < argc else 0 for i in range(8)]

        callee_fv = self._get_or_create_fv(callee)
        callee_frame = RegisterFrame(
            instructions=callee.instructions,
            ip=0,
            acc=0,
            registers=new_regs,
            depth=frame.depth + 1,
            caller_frame=frame,
            user_data={
                "feedback_vector": callee_fv,
                "locals": {name: 0 for name in callee.var_names},
                "code": callee,
            },
        )
        # Recursive execution.  The chassis's _run_frame handles the nested
        # frame's instructions; any trace_builder calls automatically flow
        # through because trace_builder is set on the shared grvm instance.
        result = grvm._run_frame(callee_frame)  # noqa: SLF001
        frame.acc = result

    def _h_ret(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        grvm.ret(frame.acc)

    # ------------------------------------------------------------------
    # Opcode handlers — I/O
    # ------------------------------------------------------------------

    def _h_io_in(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        frame.acc = self._io_in() & 0xFF

    def _h_io_out(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        self._io_out(frame.acc)

    # ------------------------------------------------------------------
    # Opcode handlers — VM control
    # ------------------------------------------------------------------

    def _h_halt(self, grvm: GenericRegisterVM, frame: RegisterFrame, instr: Instruction) -> None:
        self._count_instruction(instr.opcode)
        grvm.halt()
