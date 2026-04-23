"""Composable CIL bytecode builder for CLR compiler backends."""

from __future__ import annotations

from cil_bytecode_builder.builder import (
    CILBranchKind,
    CILBuilderError,
    CILBytecodeBuilder,
    CILOpcode,
    encode_i4,
    encode_ldarg,
    encode_ldc_i4,
    encode_ldloc,
    encode_metadata_token,
    encode_starg,
    encode_stloc,
)

__all__ = [
    "CILBranchKind",
    "CILBuilderError",
    "CILBytecodeBuilder",
    "CILOpcode",
    "encode_i4",
    "encode_ldarg",
    "encode_ldc_i4",
    "encode_ldloc",
    "encode_metadata_token",
    "encode_starg",
    "encode_stloc",
]
