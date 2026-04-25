"""``Intel4004Backend`` — ``BackendProtocol`` adapter for the 4004 simulator.

Implements ``jit_core.backend.BackendProtocol``: takes a ``list[CIRInstr]``
from jit-core's specialise / optimise passes, translates it through a
small SSA-by-name re-projection (``_cir_to_legacy_ir``), runs it through
the 4004 codegen, and executes the resulting binary on
``intel4004-simulator``.

The codegen / IR types live in :mod:`tetrad_runtime._intel4004_codegen` —
they were originally part of ``tetrad-jit``'s public surface but moved
in-tree when ``tetrad-jit`` was retired.  See that subpackage's module
docstring for the deprecation history.

This module is still a deliberate **bridge**: the codegen consumes a
non-CIR shape and ``Intel4004Backend`` re-projects to fit.  When a
CIR-native 4004 backend lands (planned ``intel4004-backend`` package),
this module becomes a one-line forwarder and the
``_intel4004_codegen`` subpackage retires.

Why a bridge instead of "wait for the rewrite"
----------------------------------------------
The bridge lets us exercise jit-core, the backend protocol, the deopt
path, and the JIT cache *today*.  It defers the codegen rewrite as
follow-up work without blocking the LANG migration on it.
"""

from __future__ import annotations

from typing import Any

from jit_core.cir import CIRInstr

from tetrad_runtime._intel4004_codegen import (
    IRInstr,
    codegen,
    run_on_4004,
)

__all__ = ["Intel4004Backend"]


class Intel4004Backend:
    """Backend that compiles CIR → Intel 4004 binary via the in-tree codegen.

    Implements the ``jit_core.backend.BackendProtocol`` structural protocol.
    """

    name: str = "intel4004"

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        """Translate CIR to a 4004 binary.

        Returns ``None`` for any CIR list that contains an instruction
        the codegen does not yet support — jit-core's deopt machinery
        will then keep the function on the interpreted path.
        """
        ir = _cir_to_legacy_ir(cir, IRInstr)
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
# CIR → in-tree IRInstr re-projection
# ---------------------------------------------------------------------------
#
# jit-core's CIRInstr carries:
#   op   — typed mnemonic ("add_u8", "cmp_lt_u8", "const_u8", ...)
#   dest — SSA destination name
#   srcs — operands (variable names or literals)
#   type — concrete IIR type ("u8" mostly for Tetrad)
#
# In-tree IRInstr (in :mod:`._intel4004_codegen.ir`) has:
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


def _cir_to_legacy_ir(cir: list[CIRInstr], IRInstrCls: type) -> list | None:  # type: ignore[type-arg]
    """Re-project a CIR list to the in-tree IRInstr shape."""
    out: list = []
    for instr in cir:
        op = instr.op
        # Strip type suffix: "add_u8" → "add", "cmp_lt_u8" → "cmp_lt".
        # The codegen only expects untyped mnemonics; the type is implicit
        # (everything is u8 for Tetrad).
        if "_" in op:
            base, _, suffix = op.rpartition("_")
            # Recognise typed suffixes; otherwise keep the original op.
            if suffix in ("u8", "u16", "u32", "u64", "bool"):
                op = base
        # Skip type guards — the 4004 codegen has no concept of them.
        # jit-core inserts these only when a type assumption needs to be
        # checked at runtime; for fully-typed Tetrad the specialise pass
        # should never emit one, but bail safely if it does.
        if instr.is_type_guard():
            return None
        if instr.is_generic():
            return None
        # Map the CIR concrete type onto the in-tree IR's two-state field:
        # "u8" stays "u8"; anything else is treated as "unknown" (which the
        # codegen handles as a deopt in most cases).
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
