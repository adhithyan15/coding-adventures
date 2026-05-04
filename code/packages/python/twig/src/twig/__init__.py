"""Twig — a tiny purely-functional Lisp-precursor on the LANG VM.

See ``code/specs/TW00-twig-language.md`` for the v1 surface, the
roadmap toward a full Lisp, and the GC / BEAM-bytecode follow-ons.

Quick start::

    from twig import TwigVM

    vm = TwigVM()
    output, value = vm.run('''
        (define (length xs)
          (if (null? xs) 0 (+ 1 (length (cdr xs)))))
        (length (cons 1 (cons 2 (cons 3 nil))))
    ''')
    assert value == 3

Standard library
----------------
The bundled Twig standard library lives in ``stdlib_twig/stdlib/``
alongside this package.  Use :func:`stdlib_path` to get the root
directory and pass it as a search path to :func:`resolve_modules`::

    from twig import resolve_modules, stdlib_path

    modules = resolve_modules(
        "user/hello",
        search_paths=[my_src_dir, stdlib_path()],
    )

When ``include_stdlib=True`` (the default) is passed to
:func:`resolve_modules`, the stdlib search path is added automatically
so callers only need to supply their own module directories.
"""

from __future__ import annotations

from pathlib import Path

from twig.ast_extract import extract_program
from twig.ast_nodes import Module, Program
from twig.compiler import compile_program
from twig.errors import (
    TwigCompileError,
    TwigError,
    TwigExitRequest,
    TwigParseError,
    TwigRuntimeError,
)
from twig.heap import NIL, Heap, HeapHandle, HeapStats
from twig.lexer import tokenize_twig
from twig.module_resolver import (
    HOST_EXPORTS,
    HOST_MODULE_NAME,
    ResolvedModule,
    resolve_modules,
)
from twig.parser import parse_twig
from twig.vm import TwigVM


def stdlib_path() -> Path:
    """Return the path to the bundled Twig standard library source root.

    The stdlib files live alongside this package in ``stdlib_twig/``.
    The search root is the ``stdlib_twig/`` directory itself, so that
    the module name ``stdlib/io`` resolves to the file::

        <stdlib_twig>/stdlib/io.tw

    Pass this path (or include it automatically via ``include_stdlib=True``)
    when calling :func:`resolve_modules` to enable ``(import stdlib/io)``
    etc. in Twig programs.

    Example::

        from twig import resolve_modules, stdlib_path

        modules = resolve_modules(
            "stdlib/io",
            search_paths=[stdlib_path()],
            include_stdlib=False,  # we're adding it explicitly above
        )
    """
    return Path(__file__).parent / "stdlib_twig"


__all__ = [
    # High-level entry
    "TwigVM",
    # Pipeline stages
    "tokenize_twig",
    "parse_twig",
    "extract_program",
    "compile_program",
    # AST surface (TW04 Phase 4a — module declarations)
    "Module",
    "Program",
    # Module resolution (TW04 Phase 4b)
    "ResolvedModule",
    "resolve_modules",
    "HOST_MODULE_NAME",
    "HOST_EXPORTS",
    # Standard library path (TW04 Phase 4g)
    "stdlib_path",
    # Heap surface
    "Heap",
    "HeapHandle",
    "HeapStats",
    "NIL",
    # Errors
    "TwigError",
    "TwigParseError",
    "TwigCompileError",
    "TwigRuntimeError",
    "TwigExitRequest",
]
