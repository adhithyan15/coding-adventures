"""Lower compiler IR programs into composable CIL bytecode artifacts."""

from ir_to_cil_bytecode.backend import (
    CILBackendConfig,
    CILBackendError,
    CILHelper,
    CILHelperSpec,
    CILLoweringPipeline,
    CILLoweringPlan,
    CILMethodArtifact,
    CILProgramArtifact,
    CILTokenProvider,
    SequentialCILTokenProvider,
    lower_ir_to_cil_bytecode,
    validate_for_clr,
)

__all__ = [
    "CILBackendConfig",
    "CILBackendError",
    "CILHelper",
    "CILHelperSpec",
    "CILLoweringPipeline",
    "CILLoweringPlan",
    "CILMethodArtifact",
    "CILProgramArtifact",
    "CILTokenProvider",
    "SequentialCILTokenProvider",
    "lower_ir_to_cil_bytecode",
    "validate_for_clr",
]
