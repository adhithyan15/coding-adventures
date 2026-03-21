"""Lisp Compiler — Compiles McCarthy's 1960 Lisp into bytecode.

This package provides a compiler that transforms Lisp source code into
CodeObject bytecode for execution on the GenericVM with the Lisp VM plugin.

Usage::

    from lisp_compiler import compile_lisp, run_lisp

    code = compile_lisp("(+ 1 2)")
    result = run_lisp("(+ 1 2)")  # => 3
"""

from lisp_compiler.compiler import (
    compile_lisp,
    create_lisp_compiler,
    run_lisp,
)

__all__ = [
    "compile_lisp",
    "create_lisp_compiler",
    "run_lisp",
]
