"""Lower compiler-ir IrProgram → BEAMModule (BEAM01 Phase 3)."""

from ir_to_beam.backend import (
    BEAMBackendConfig,
    BEAMBackendError,
    lower_ir_to_beam,
)

__all__ = [
    "BEAMBackendConfig",
    "BEAMBackendError",
    "lower_ir_to_beam",
]
