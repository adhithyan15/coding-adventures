"""``twig-jvm-compiler`` — Twig source → JVM class file (real java).

See ``code/specs/TW02-twig-jvm-compiler.md`` for the v1 surface
and the multi-step roadmap (TW02.5: lambdas / cons cells / print;
TW04: BEAM; TW05: WASM updates).
"""

from __future__ import annotations

from twig_jvm_compiler.compiler import (
    ExecutionResult,
    ModuleCompileResult,
    MultiModuleExecutionResult,
    MultiModuleResult,
    PackageError,
    PackageResult,
    compile_modules,
    compile_source,
    compile_to_ir,
    java_available,
    module_name_to_jvm_class,
    run_modules,
    run_source,
)

__all__ = [
    "ExecutionResult",
    "ModuleCompileResult",
    "MultiModuleExecutionResult",
    "MultiModuleResult",
    "PackageError",
    "PackageResult",
    "compile_modules",
    "compile_source",
    "compile_to_ir",
    "java_available",
    "module_name_to_jvm_class",
    "run_modules",
    "run_source",
]
