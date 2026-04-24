"""``Intel4004Backend`` — ``BackendProtocol`` adapter for the 4004 simulator.

The legacy ``tetrad-jit`` package contains a working bytecode → 4004
abstract-assembly → binary pipeline (``codegen_4004.py``) and a runner
(``run_on_4004``).  Re-implementing that from scratch against jit-core's
``CIRInstr`` shape is a substantial follow-up project; for the migration
PR we adopt a **hybrid strategy**:

1.  Translate the post-specialise / post-optimise ``list[CIRInstr]`` into
    the legacy ``tetrad_jit.ir.IRInstr`` shape (a small re-projection — both
    are flat SSA-by-name lists).
2.  Hand the IRInstr list to the existing ``codegen()`` function, which
    returns either ``bytes`` or ``None`` (None = the function uses an
    opcode the 4004 codegen does not yet support — jit-core treats this
    as deopt and falls back to the interpreter).
3.  ``run`` defers to ``tetrad_jit.codegen_4004.run_on_4004`` which feeds
    the binary into ``Intel4004Simulator`` and returns the u8 result.

This is a deliberate **bridge**.  When the 4004 backend is rewritten to
consume CIR directly (a future package, ``intel4004-backend``), this
module becomes a one-line forwarder.  Until then, this is the path that
gets a working JIT through the LANG pipeline today.

Why a bridge instead of "wait for the rewrite"
----------------------------------------------
The point of the migration is to prove Tetrad runs on LANG.  Forcing a
rewrite of the codegen as a prerequisite would block that proof on weeks
of unrelated work.  A bridge lets us exercise jit-core, the backend
protocol, the deopt path, and the JIT cache *today*, with the clear
signal that the codegen replacement is what's left.
"""

from __future__ import annotations

from typing import Any

from jit_core.cir import CIRInstr

__all__ = ["Intel4004Backend"]


class Intel4004Backend:
    """Backend that compiles CIR → Intel 4004 binary via the legacy codegen.

    Implements the ``jit_core.backend.BackendProtocol`` structural protocol.
    """

    name: str = "intel4004"

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        """Translate CIR to a 4004 binary.

        Returns ``None`` for any CIR list that contains an instruction the
        legacy codegen does not yet support — jit-core's deopt machinery
        will then keep the function on the interpreted path.
        """
        try:
            from tetrad_jit.codegen_4004 import codegen
            from tetrad_jit.ir import IRInstr
        except ImportError:
            return None

        ir = _cir_to_legacy_ir(cir, IRInstr)
        if ir is None:
            return None
        try:
            return codegen(ir)
        except Exception:
            return None

    def run(self, binary: bytes, args: list[Any]) -> Any:
        """Execute a previously-compiled 4004 binary on the simulator."""
        from tetrad_jit.codegen_4004 import run_on_4004
        # The simulator expects ints; coerce.
        int_args = [int(a) & 0xFF for a in args]
        return run_on_4004(binary, int_args)


# ---------------------------------------------------------------------------
# CIR → legacy IRInstr re-projection
# ---------------------------------------------------------------------------
#
# jit-core's CIRInstr carries:
#   op   — typed mnemonic ("add_u8", "cmp_lt_u8", "const_u8", ...)
#   dest — SSA destination name
#   srcs — operands (variable names or literals)
#   type — concrete IIR type ("u8" mostly for Tetrad)
#
# Legacy tetrad-jit IRInstr (per tetrad_jit.ir module — discovered at
# runtime to avoid an import cycle when tetrad-jit is not installed) has:
#   op   — bare mnemonic ("add", "cmp_lt", "const", ...)
#   dest — SSA destination
#   srcs — operands
#
# The re-projection drops the type suffix from ``op`` and otherwise copies
# fields verbatim.  Returns None if any op cannot be mapped (e.g.,
# ``call_runtime`` — the 4004 codegen has no notion of runtime calls).
# ---------------------------------------------------------------------------


def _cir_to_legacy_ir(cir: list[CIRInstr], IRInstrCls: type) -> list | None:  # type: ignore[type-arg]
    """Re-project a CIR list to the legacy tetrad-jit IRInstr shape."""
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
        out.append(
            IRInstrCls(
                op=op,
                dest=instr.dest,
                srcs=list(instr.srcs),
            )
        )
    return out
