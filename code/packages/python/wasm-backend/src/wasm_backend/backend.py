"""WASMBackend — BackendProtocol implementation for WebAssembly 1.0.

This module wires together two LANG milestones to create a complete
CIR-to-WASM compilation pipeline:

  LANG21 (cir-to-compiler-ir): list[CIRInstr] → IrProgram
  LANG20 (ir-to-wasm-compiler): IrProgram    → WasmModule → bytes

The resulting backend can be dropped into the JIT pipeline:

    from jit_core import JITCore
    from vm_core import VMCore
    from wasm_backend import WASMBackend

    vm      = VMCore()
    backend = WASMBackend()
    jit     = JITCore(vm, backend)
    result  = jit.execute_with_jit(module)

Or used directly with ``TetradRuntime.run_with_jit()``:

    from tetrad_runtime import TetradRuntime
    from wasm_backend import WASMBackend

    rt     = TetradRuntime()
    result = rt.run_with_jit(source, backend=WASMBackend())

Pipeline detail
---------------

::

    list[CIRInstr]                 (jit-core output)
        │
        ▼  lower_cir_to_ir_program()   (LANG21)
    IrProgram
        │   ↑ optional fixup: copy result → register 1 before HALT
        │
        ▼  IrToWasmCompiler().compile()  (LANG20 / ir-to-wasm-compiler)
    WasmModule
        │
        ▼  encode_module()               (wasm-module-encoder)
    bytes   (valid WebAssembly 1.0 binary)
        │
        ▼  WasmRuntime().load_and_run()  (wasm-runtime)
    list[int | float]   → return value

Return-value convention
-----------------------

The WASM compiler reads ``IrRegister(1)`` as the function return value at
HALT (this is the internal ``_REG_SCRATCH`` slot).  LANG21's two-pass
lowering assigns registers by first-occurrence order in the CIR, so the
result of a computation may land in a register other than 1.

``WASMBackend.compile()`` fixes this automatically:

1. Scan the CIR for the last ``ret_{type}`` instruction; record the return
   variable name (``ret_void`` functions return ``None``).
2. Mirror LANG21's Pass 1 to find the register index assigned to that
   variable.
3. After lowering, insert ``ADD_IMM IrRegister(1), result_reg, 0`` before
   the HALT instruction.  This is a no-op if the result is already in
   register 1.

This fixup is a deliberate V1 convenience; LANG22's multi-function calling
convention will introduce a proper return-register protocol.
"""

from __future__ import annotations

from typing import Any

from codegen_core import CIRInstr
from compiler_ir import IDGenerator, IrImmediate, IrInstruction, IrOp, IrRegister

# ---------------------------------------------------------------------------
# Internal: mirror LANG21's Pass 1 to build var → register map
# ---------------------------------------------------------------------------
# LANG21's _CIRLowerer._collect_vars() assigns register indices in
# first-occurrence order (dest before srcs, left-to-right).  We replicate
# that logic here so that WASMBackend can look up which register holds the
# return value *before* calling lower_cir_to_ir_program().
#
# This avoids exposing LANG21's internals — a clean boundary between packages.
# ---------------------------------------------------------------------------

# Ops where ALL srcs are label names (not variable names).
_LABEL_SRC_OPS: frozenset[str] = frozenset({"label", "jmp", "call"})

# Conditional branches: srcs[0] = condition variable, srcs[1] = label name.
_COND_BRANCH_OPS: frozenset[str] = frozenset(
    {"jmp_if_true", "jmp_if_false", "br_true_bool", "br_false_bool"}
)


def _collect_cir_registers(cir: list[CIRInstr]) -> dict[str, int]:
    """Build a ``variable → register index`` map for a CIR list.

    This is an exact mirror of ``_CIRLowerer._collect_vars()`` in LANG21.
    It assigns each unique variable name the next free integer index in
    order of first occurrence:

    1.  ``dest`` of each instruction (if not ``None``)
    2.  ``srcs`` entries that are variable names (strings), in order

    Label names and type-name strings are skipped using the same rules as
    the original pass.

    Args:
        cir: CIR instruction list produced by ``jit_core.specialise()``.

    Returns:
        ``dict[str, int]`` mapping each variable name to its register index.
        Order matches LANG21's two-pass lowering exactly.
    """
    reg: dict[str, int] = {}
    n: int = 0

    def _assign(name: str) -> None:
        nonlocal n
        if name not in reg:
            reg[name] = n
            n += 1

    for instr in cir:
        if instr.dest is not None:
            _assign(instr.dest)

        for idx, src in enumerate(instr.srcs):
            if not isinstance(src, str):
                continue  # integer / float / bool literal → skip
            if instr.op in _LABEL_SRC_OPS:
                continue  # all srcs are label names here
            if instr.op in _COND_BRANCH_OPS and idx == 1:
                continue  # srcs[1] is the branch-target label
            if instr.op == "type_assert" and idx == 1:
                continue  # srcs[1] is a type-name string
            _assign(src)

    return reg


# ---------------------------------------------------------------------------
# WASMBackend
# ---------------------------------------------------------------------------


class WASMBackend:
    """BackendProtocol implementation that compiles CIR to WebAssembly 1.0.

    Implements the two-method ``BackendProtocol`` from ``jit-core`` / ``codegen-core``:

    - ``compile(cir)`` → ``bytes | None``
    - ``run(binary, args)`` → ``Any``

    Because the protocol is structural (duck-typed), ``WASMBackend`` does
    not need to explicitly import or inherit from ``BackendProtocol``.
    Any object with the right method signatures satisfies the check:

        >>> from codegen_core import BackendProtocol
        >>> isinstance(WASMBackend(), BackendProtocol)
        True

    Attributes:
        entry_label:
            The WASM function label / export name.  Defaults to ``"_start"``
            which is the entry point expected by WASM runtimes.

    Example::

        from jit_core import JITCore, specialise
        from vm_core import VMCore
        from wasm_backend import WASMBackend
        from tetrad_runtime import compile_to_iir

        module  = compile_to_iir("fn main() -> u8 { return 40 + 2; }")
        backend = WASMBackend()
        vm      = VMCore()
        jit     = JITCore(vm, backend, min_observations=0)
        result  = jit.execute_with_jit(module)
        # result == 42
    """

    #: Short human-readable backend identifier.  Required by ``BackendProtocol``
    #: for diagnostics (stored in ``CodegenResult.backend_name``).
    name: str = "wasm"

    def __init__(self, *, entry_label: str = "_start") -> None:
        self.entry_label = entry_label

    # ------------------------------------------------------------------
    # BackendProtocol — compile()
    # ------------------------------------------------------------------

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        """Lower CIR → IrProgram → WASM bytes.

        Steps
        -----
        1. Find the return variable (last ``ret_{type}`` instruction's source).
        2. Mirror LANG21's Pass 1 to look up the return variable's register.
        3. Call ``lower_cir_to_ir_program()`` (LANG21).
        4. If the return register is not ``IrRegister(1)``, insert
           ``ADD_IMM IrRegister(1), result_reg, IrImmediate(0)`` before
           the HALT instruction.
        5. Compile to ``WasmModule`` via ``IrToWasmCompiler``.
        6. Encode to bytes via ``encode_module``.

        Args:
            cir: CIR instruction list from ``jit_core.specialise()``.

        Returns:
            WebAssembly 1.0 binary bytes on success, or ``None`` if
            lowering fails (which causes the JIT to deoptimise and
            fall back to interpretation).
        """
        # Late imports keep the module importable even when test environments
        # only have a subset of the dependencies available.
        try:
            from cir_to_compiler_ir import CIRLoweringError, lower_cir_to_ir_program
            from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler
            from wasm_module_encoder import encode_module
            from wasm_types import ValueType
        except ImportError:
            # If any dependency is missing, deopt gracefully.
            return None

        try:
            # ── Step 1: find the return variable ────────────────────────────
            #
            # The specialiser emits ``ret_{type}`` with ``srcs[0]`` = the
            # variable holding the return value.  ``ret_void`` has no srcs.
            result_var: str | None = None
            for instr in reversed(cir):
                if instr.op.startswith("ret_") and instr.op != "ret_void":
                    if instr.srcs and isinstance(instr.srcs[0], str):
                        result_var = instr.srcs[0]
                    break

            # ── Step 2: find the return variable's register index ────────────
            #
            # _collect_cir_registers() mirrors LANG21's Pass 1 exactly, so
            # the index it returns matches what lower_cir_to_ir_program()
            # will assign to that variable.
            reg_map = _collect_cir_registers(cir)
            result_reg_idx: int | None = (
                reg_map.get(result_var) if result_var is not None else None
            )

            # ── Step 3: LANG21 lowering ──────────────────────────────────────
            #
            # The WASM compiler's ``_split_functions`` only recognises LABEL
            # instructions whose name is ``"_start"`` or starts with ``"_fn_"``.
            # We therefore always emit the IrProgram with ``"_start"`` as the
            # entry label, and export it under ``self.entry_label`` in the
            # FunctionSignature below.
            prog = lower_cir_to_ir_program(cir, entry_label="_start")

            # ── Step 4: WASM return-value fixup ─────────────────────────────
            #
            # The WASM compiler reads IrRegister(1) (``_REG_SCRATCH``) at
            # HALT as the function return value.  If the result landed in a
            # different register, copy it with ``ADD_IMM dst=1, src=result, 0``.
            #
            # We skip the fixup when:
            #   • No return value (ret_void or unknown)
            #   • Result is already in register 1
            if result_reg_idx is not None and result_reg_idx != 1:
                halt_idx: int | None = next(
                    (
                        i
                        for i, ins in enumerate(prog.instructions)
                        if ins.opcode == IrOp.HALT
                    ),
                    None,
                )
                if halt_idx is not None:
                    gen = IDGenerator()
                    # ADD_IMM IrRegister(1), IrRegister(result_reg_idx), 0
                    # is the canonical "MOV via ADD immediate-zero" pattern
                    # used throughout the IrProgram tests.
                    fixup = IrInstruction(
                        opcode=IrOp.ADD_IMM,
                        operands=[
                            IrRegister(1),
                            IrRegister(result_reg_idx),
                            IrImmediate(0),
                        ],
                        id=gen.next(),
                    )
                    prog.instructions.insert(halt_idx, fixup)

            # ── Step 5: compile IrProgram → WasmModule ───────────────────────
            #
            # Use label ``"_start"`` (matches the IrProgram entry label emitted
            # above) and ``export_name=self.entry_label`` so the WASM module
            # exports the function under the user-supplied name.
            has_result = result_reg_idx is not None
            sig = FunctionSignature(
                label="_start",
                param_count=0,
                export_name=self.entry_label,
                result_types=(ValueType.I32,) if has_result else (),
            )
            wasm_module = IrToWasmCompiler().compile(prog, [sig])

            # ── Step 6: encode to bytes ──────────────────────────────────────
            return encode_module(wasm_module)

        except (CIRLoweringError, Exception):
            # Any failure → deopt.  The JIT falls back to interpreted execution.
            return None

    # ------------------------------------------------------------------
    # BackendProtocol — run()
    # ------------------------------------------------------------------

    def run(self, binary: bytes, args: list[Any]) -> Any:
        """Execute a WASM binary on the WasmRuntime.

        Args:
            binary: WebAssembly 1.0 bytes from ``compile()``.
            args:   Arguments to pass to the entry function.
                    For V1 single-function programs, this is always ``[]``.

        Returns:
            The first return value from the WASM function, or ``None`` if
            the function returns nothing (``ret_void``).
        """
        try:
            from wasm_runtime import WasmRuntime

            results = WasmRuntime().load_and_run(binary, self.entry_label, args)
            return results[0] if results else None
        except Exception:
            return None
