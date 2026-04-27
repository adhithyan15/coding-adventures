"""Compile loaded Prolog programs into standardized Logic VM instructions."""

from prolog_vm_compiler.compiler import (
    CompiledPrologVMProgram,
    compile_loaded_prolog_project,
    compile_loaded_prolog_source,
    compile_swi_prolog_project,
    compile_swi_prolog_source,
    load_compiled_prolog_vm,
    run_compiled_prolog_queries,
    run_compiled_prolog_query,
)

__all__ = [
    "__version__",
    "CompiledPrologVMProgram",
    "compile_loaded_prolog_project",
    "compile_loaded_prolog_source",
    "compile_swi_prolog_project",
    "compile_swi_prolog_source",
    "load_compiled_prolog_vm",
    "run_compiled_prolog_queries",
    "run_compiled_prolog_query",
]

__version__ = "0.1.0"
