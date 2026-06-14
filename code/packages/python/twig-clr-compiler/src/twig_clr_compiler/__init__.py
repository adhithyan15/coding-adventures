"""``twig-clr-compiler`` — Twig source → real ``dotnet``.

Completes the Twig real-runtime trilogy alongside JVM and BEAM.
"""

from __future__ import annotations

from twig_clr_compiler.compiler import (
    ClrPackageError,
    ClrPackageResult,
    ClrRunResult,
    compile_source,
    compile_to_ir,
    dotnet_available,
    run_source,
)

__all__ = [
    "ClrPackageError",
    "ClrPackageResult",
    "ClrRunResult",
    "compile_source",
    "compile_to_ir",
    "dotnet_available",
    "run_source",
]
