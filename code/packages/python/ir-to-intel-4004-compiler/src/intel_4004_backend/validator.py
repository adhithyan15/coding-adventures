"""Compatibility wrapper for the old validator module path."""

from intel_4004_ir_validator import IrValidationError, IrValidator

__all__ = [
    "IrValidationError",
    "IrValidator",
]
