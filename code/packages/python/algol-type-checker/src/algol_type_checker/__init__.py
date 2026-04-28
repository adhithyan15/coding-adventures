"""algol-type-checker — Type checker for the first ALGOL 60 compiler subset

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

from algol_type_checker.checker import (
    FRAME_HEADER_SIZE,
    FRAME_REAL_SIZE,
    FRAME_WORD_SIZE,
    AlgolTypeChecker,
    ArrayDescriptor,
    ArrayDimension,
    Diagnostic,
    FrameLayout,
    FrameSlot,
    LabelDescriptor,
    ProcedureDescriptor,
    ProcedureParameter,
    ResolvedArrayAccess,
    ResolvedGoto,
    ResolvedProcedureCall,
    ResolvedReference,
    ResolvedSwitchSelection,
    Scope,
    SemanticBlock,
    SemanticProgram,
    SwitchDescriptor,
    Symbol,
    TypeCheckError,
    TypeCheckResult,
    assert_algol_typed,
    check,
    check_algol,
)

__version__ = "0.1.0"

__all__ = [
    "AlgolTypeChecker",
    "ArrayDescriptor",
    "ArrayDimension",
    "Diagnostic",
    "FRAME_HEADER_SIZE",
    "FRAME_REAL_SIZE",
    "FRAME_WORD_SIZE",
    "FrameLayout",
    "FrameSlot",
    "LabelDescriptor",
    "ProcedureDescriptor",
    "ProcedureParameter",
    "ResolvedArrayAccess",
    "ResolvedGoto",
    "ResolvedProcedureCall",
    "ResolvedReference",
    "ResolvedSwitchSelection",
    "Scope",
    "SemanticBlock",
    "SemanticProgram",
    "Symbol",
    "SwitchDescriptor",
    "TypeCheckError",
    "TypeCheckResult",
    "assert_algol_typed",
    "check",
    "check_algol",
]
