"""``twig-beam-compiler`` — Twig source → ``.beam`` (real erl).

See ``code/specs/BEAM01-twig-on-real-erl.md`` Phase 4.
"""

from __future__ import annotations

from twig_beam_compiler.compiler import (
    BeamPackageError,
    BeamPackageResult,
    BeamRunResult,
    compile_source,
    compile_to_ir,
    erl_available,
    run_source,
)

__all__ = [
    "BeamPackageError",
    "BeamPackageResult",
    "BeamRunResult",
    "compile_source",
    "compile_to_ir",
    "erl_available",
    "run_source",
]
