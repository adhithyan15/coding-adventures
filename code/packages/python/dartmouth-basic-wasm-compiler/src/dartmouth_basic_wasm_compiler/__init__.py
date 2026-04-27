"""Dartmouth BASIC → WebAssembly compiled pipeline.

This package ties together every stage of the Dartmouth BASIC ahead-of-time
compiler targeting WebAssembly into a single ``run_basic()`` call:

::

    BASIC source  →  lexer/parser  →  IR compiler  →  WASM backend  →  WASM runtime

The IR compiler runs in ASCII char-encoding mode so that WASM's ``fd_write``
syscall receives standard ASCII bytes rather than GE-225 typewriter codes.

Usage::

    from dartmouth_basic_wasm_compiler import run_basic

    result = run_basic(\"\"\"
    10 FOR I = 1 TO 5
    20 PRINT I
    30 NEXT I
    40 END
    \"\"\")
    print(result.output)   # "1\\n2\\n3\\n4\\n5\\n"
"""

from dartmouth_basic_wasm_compiler.runner import BasicError, RunResult, run_basic

__all__ = ["run_basic", "RunResult", "BasicError"]
