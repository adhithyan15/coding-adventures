"""End-to-end Oct to CLR compiler facade."""

from oct_clr_compiler.compiler import (
    ExecutionResult,
    OctClrCompiler,
    PackageError,
    PackageResult,
    compile_source,
    pack_source,
    run_source,
    write_assembly_file,
)

__all__ = [
    "ExecutionResult",
    "OctClrCompiler",
    "PackageError",
    "PackageResult",
    "compile_source",
    "pack_source",
    "run_source",
    "write_assembly_file",
]
