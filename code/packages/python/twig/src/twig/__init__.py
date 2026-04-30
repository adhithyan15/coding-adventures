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
"""

from __future__ import annotations

from twig.ast_extract import extract_program
from twig.ast_nodes import Module, Program
from twig.compiler import compile_program
from twig.errors import (
    TwigCompileError,
    TwigError,
    TwigParseError,
    TwigRuntimeError,
)
from twig.heap import NIL, Heap, HeapHandle, HeapStats
from twig.lexer import tokenize_twig
from twig.parser import parse_twig
from twig.vm import TwigVM

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
]
