"""Version-aware decoder for CLR Portable Executable assemblies."""

from __future__ import annotations

from clr_pe_file.pe_file import (
    CLRAssemblyRef,
    CLRMemberReference,
    CLRMethodBodyHeader,
    CLRMethodDef,
    CLRMethodSignature,
    CLRPEFile,
    CLRTypeDef,
    CLRTypeReference,
    decode_clr_pe_file,
)

__all__ = [
    "CLRAssemblyRef",
    "CLRMemberReference",
    "CLRMethodBodyHeader",
    "CLRMethodDef",
    "CLRMethodSignature",
    "CLRPEFile",
    "CLRTypeDef",
    "CLRTypeReference",
    "decode_clr_pe_file",
]
