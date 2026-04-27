"""End-to-end Nib to CLR compiler facade."""

from nib_clr_compiler.compiler import (
    ExecutionResult,
    NibClrCompiler,
    PackageError,
    PackageResult,
    compile_source,
    pack_source,
    run_source,
    write_assembly_file,
)

__all__ = [
    "ExecutionResult",
    "NibClrCompiler",
    "PackageError",
    "PackageResult",
    "compile_source",
    "pack_source",
    "run_source",
    "write_assembly_file",
]
