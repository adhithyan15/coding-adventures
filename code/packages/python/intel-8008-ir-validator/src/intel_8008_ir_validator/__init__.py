"""Intel 8008 IR validator package.

Pre-flight hardware-constraint validation for IrPrograms targeting the
Intel 8008 processor.  Call ``IrValidator().validate(program)`` before
passing the program to the code generator.
"""

from intel_8008_ir_validator.validator import IrValidationError, IrValidator

__all__ = [
    "IrValidationError",
    "IrValidator",
]
