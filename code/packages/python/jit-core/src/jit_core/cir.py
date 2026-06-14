"""Backwards-compatibility re-export of CIRInstr from codegen-core.

``CIRInstr`` was originally defined here in ``jit_core.cir``.  It has
been moved to ``codegen_core.cir`` (LANG19) so that both ``jit-core`` and
``aot-core`` can import a shared type without ``aot-core`` depending on a
JIT-specific package.

This module re-exports ``CIRInstr`` so existing callers of
``from jit_core.cir import CIRInstr`` or ``from jit_core import CIRInstr``
continue to work without any changes.

New code should import from ``codegen_core`` directly:

    from codegen_core import CIRInstr
"""

from codegen_core.cir import CIRInstr

__all__ = ["CIRInstr"]
