"""``twig-clr-compiler`` — Twig source → PE/CLI assembly.

See ``code/specs/TW02-twig-clr-compiler.md`` for the v1 surface
and the multi-step roadmap (TW02.5: closures, cons, print; TW03:
BEAM; TW04: JVM).

Quick start::

    from twig_clr_compiler import run_source
    result = run_source("(if (= 1 1) 100 200)")
    assert result.vm_result.return_value == 100
"""

from __future__ import annotations

from twig_clr_compiler.compiler import (
    ExecutionResult,
    PackageError,
    PackageResult,
    compile_source,
    compile_to_ir,
    run_source,
)

__all__ = [
    "ExecutionResult",
    "PackageError",
    "PackageResult",
    "compile_source",
    "compile_to_ir",
    "run_source",
]
