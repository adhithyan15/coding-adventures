"""Compile loaded Prolog programs into standardized Logic VM instructions."""

from prolog_vm_compiler.compiler import (
    CompiledPrologVMProgram,
    PrologAnswer,
    PrologVMInitializationError,
    compile_loaded_prolog_project,
    compile_loaded_prolog_source,
    compile_swi_prolog_project,
    compile_swi_prolog_source,
    load_compiled_prolog_vm,
    run_compiled_prolog_initializations,
    run_compiled_prolog_queries,
    run_compiled_prolog_query,
    run_compiled_prolog_query_answers,
    run_initialized_compiled_prolog_query,
    run_initialized_compiled_prolog_query_answers,
)

__all__ = [
    "__version__",
    "CompiledPrologVMProgram",
    "PrologAnswer",
    "PrologVMInitializationError",
    "compile_loaded_prolog_project",
    "compile_loaded_prolog_source",
    "compile_swi_prolog_project",
    "compile_swi_prolog_source",
    "load_compiled_prolog_vm",
    "run_compiled_prolog_initializations",
    "run_compiled_prolog_queries",
    "run_compiled_prolog_query",
    "run_compiled_prolog_query_answers",
    "run_initialized_compiled_prolog_query",
    "run_initialized_compiled_prolog_query_answers",
]

__version__ = "0.1.0"
