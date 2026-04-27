"""``Intel4004Backend`` — the ``BackendProtocol`` implementation.

Compiles a ``list[CIRInstr]`` from ``jit-core`` into Intel 4004 machine
code, then runs that code on the ``intel4004-simulator``.

Why a CIR re-projection step
----------------------------

The codegen module in this package consumes a small SSA-by-name
``IRInstr`` form rather than ``CIRInstr`` directly.  That shape is a
holdover from when the codegen lived in the (now-retired)
``tetrad-jit`` package; we kept the shape when extracting the codegen
so the migration was a pure move.  ``Intel4004Backend.compile``
re-projects ``CIRInstr`` to ``IRInstr`` on the way in.

Once a future PR rewrites the codegen to consume ``CIRInstr`` directly,
``compile`` will become a one-line forwarder and the re-projection
helper goes away.
"""

from __future__ import annotations

from typing import Any

from jit_core.cir import CIRInstr

from intel4004_backend.codegen import codegen, run_on_4004
from intel4004_backend.ir import IRInstr

__all__ = ["Intel4004Backend"]


class Intel4004Backend:
    """Intel 4004 ``BackendProtocol`` implementation.

    Implements ``jit_core.backend.BackendProtocol`` (a structural
    protocol — no inheritance required, but ``isinstance(backend,
    BackendProtocol)`` returns True).

    Usage from a frontend::

        from jit_core import JITCore
        from vm_core import VMCore
        from intel4004_backend import Intel4004Backend

        vm = VMCore()
        jit = JITCore(vm, backend=Intel4004Backend())
        jit.execute_with_jit(module, fn="main")
    """

    name: str = "intel4004"

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        """Translate CIR to a 4004 binary.

        Returns ``None`` for any CIR list that contains an instruction
        the codegen does not yet support — jit-core's deopt machinery
        will then keep the function on the interpreted path.
        """
        ir = _cir_to_ir(cir, IRInstr)
        if ir is None:
            return None
        try:
            return codegen(ir)
        except Exception:
            return None

    def run(self, binary: bytes, args: list[Any]) -> Any:
        """Execute a previously-compiled 4004 binary on the simulator."""
        # The simulator expects ints; coerce.
        int_args = [int(a) & 0xFF for a in args]
        return run_on_4004(binary, int_args)


# ---------------------------------------------------------------------------
# CIR → IRInstr re-projection
# ---------------------------------------------------------------------------
#
# jit-core's CIRInstr carries:
#   op   — typed mnemonic ("add_u8", "cmp_lt_u8", "const_u8", ...)
#   dest — SSA destination name
#   srcs — operands (variable names or literals)
#   type — concrete IIR type ("u8" mostly for Tetrad)
#
# IRInstr (in :mod:`.ir`) has:
#   op   — bare mnemonic ("add", "cmp_lt", "const", ...)
#   dst  — SSA destination
#   srcs — operands
#   ty   — "u8" | "unknown"
#
# The re-projection drops the type suffix from ``op`` and otherwise
# copies fields verbatim.  Returns None if any op cannot be mapped
# (e.g. ``call_runtime`` — the 4004 codegen has no notion of runtime
# calls).
# ---------------------------------------------------------------------------


def _cir_to_ir(cir: list[CIRInstr], IRInstrCls: type) -> list | None:  # type: ignore[type-arg]
    """Re-project a CIR list to the codegen's ``IRInstr`` shape."""
    out: list = []
    for instr in cir:
        op = instr.op
        # Strip type suffix: "add_u8" → "add", "cmp_lt_u8" → "cmp_lt".
        # The codegen only expects untyped mnemonics; the type is implicit
        # (everything is u8 for Tetrad).
        if "_" in op:
            base, _, suffix = op.rpartition("_")
            if suffix in ("u8", "u16", "u32", "u64", "bool"):
                op = base
        # Skip type guards — the 4004 codegen has no concept of them.
        if instr.is_type_guard():
            return None
        if instr.is_generic():
            return None
        ty = "u8" if instr.type == "u8" else "unknown"
        out.append(
            IRInstrCls(
                op=op,
                dst=instr.dest,
                srcs=list(instr.srcs),
                ty=ty,
            )
        )
    return out
