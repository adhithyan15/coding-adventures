"""Compatibility wrapper for the old codegen module path."""

from ir_to_intel_4004_compiler.codegen import CodeGenerator, _vreg_to_pair

__all__ = [
    "CodeGenerator",
    "_vreg_to_pair",
]
