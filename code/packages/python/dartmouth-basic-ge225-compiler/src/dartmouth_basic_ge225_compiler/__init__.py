"""Dartmouth BASIC → GE-225 compiled pipeline.

This package ties together every stage of the Dartmouth BASIC ahead-of-time
compiler into a single ``run_basic()`` call:

::

    BASIC source  →  lexer/parser  →  IR compiler  →  GE-225 backend  →  simulator

The pipeline faithfully recreates the experience of running BASIC programs on
the 1964 Dartmouth time-sharing system: programs are compiled to GE-225 20-bit
machine words and executed on a behavioural simulator of the same hardware.

Usage::

    from dartmouth_basic_ge225_compiler import run_basic

    result = run_basic(\"\"\"
    10 FOR I = 1 TO 5
    20 PRINT I
    30 NEXT I
    40 END
    \"\"\")
    print(result.output)   # "1\\n2\\n3\\n4\\n5\\n"
    print(result.steps)    # number of GE-225 instructions executed
"""

from dartmouth_basic_ge225_compiler.runner import BasicError, RunResult, run_basic

__all__ = ["run_basic", "RunResult", "BasicError"]
