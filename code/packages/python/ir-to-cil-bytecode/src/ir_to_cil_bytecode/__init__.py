"""Lower compiler IR programs into composable CIL bytecode artifacts.

LANG20: ``CILCodeGenerator`` implements ``CodeGenerator[IrProgram, CILProgramArtifact]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.
"""

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
from ir_to_cil_bytecode.generator import CILCodeGenerator

__all__ = [
    "CILBackendConfig",
    "CILBackendError",
    "CILCodeGenerator",
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
