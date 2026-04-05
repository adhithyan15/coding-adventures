"""wasm-validator --- WebAssembly 1.0 module validator."""

__version__ = "0.1.0"

from wasm_validator.validator import (
    IndexSpaces,
    ValidatedModule,
    ValidationError,
    ValidationErrorKind,
    validate,
    validate_structure,
)

__all__ = [
    "validate",
    "validate_structure",
    "ValidatedModule",
    "IndexSpaces",
    "ValidationError",
    "ValidationErrorKind",
]
