"""Compatibility wrapper for the old backend module path."""

from ir_to_intel_4004_compiler.backend import Intel4004Backend, IrToIntel4004Compiler

__all__ = [
    "IrToIntel4004Compiler",
    "Intel4004Backend",
]
