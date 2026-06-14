"""``twig-beam-compiler`` — Twig source → ``.beam`` (real erl).

See ``code/specs/BEAM01-twig-on-real-erl.md`` Phase 4.

TW04 Phase 4f adds multi-module support: ``compile_modules`` lowers a
topologically ordered list of ``ResolvedModule`` objects (from
``twig.resolve_modules``) to individual ``.beam`` files; ``run_modules``
writes them to a temp directory and invokes ``erl`` to execute the entry
module's ``main()`` function.
"""

from __future__ import annotations

from twig_beam_compiler.compiler import (
    BeamPackageError,
    BeamPackageResult,
    BeamRunResult,
    ModuleBeamCompileResult,
    MultiModuleBeamExecutionResult,
    MultiModuleBeamResult,
    compile_modules,
    compile_source,
    compile_to_ir,
    erl_available,
    module_name_to_beam_module,
    run_modules,
    run_source,
)

__all__ = [
    "BeamPackageError",
    "BeamPackageResult",
    "BeamRunResult",
    "ModuleBeamCompileResult",
    "MultiModuleBeamExecutionResult",
    "MultiModuleBeamResult",
    "compile_modules",
    "compile_source",
    "compile_to_ir",
    "erl_available",
    "module_name_to_beam_module",
    "run_modules",
    "run_source",
]
