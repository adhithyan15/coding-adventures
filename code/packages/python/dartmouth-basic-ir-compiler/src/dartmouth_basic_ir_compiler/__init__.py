"""Dartmouth BASIC IR Compiler — lowers a BASIC AST to target-independent IR.

This package is the Dartmouth BASIC frontend of the ahead-of-time compiler
pipeline. It accepts a parsed AST from ``dartmouth_basic_parser`` and emits
a ``compiler_ir.IrProgram`` that any backend (GE-225, WASM, JVM) can compile
to native code.

V1 supports: LET, FOR/NEXT, IF/THEN, GOTO, PRINT (string literals), END, STOP, REM.

Usage::

    from dartmouth_basic_parser import parse_dartmouth_basic
    from dartmouth_basic_ir_compiler import compile_basic, CompileError

    source = "10 FOR I = 1 TO 5\\n20 PRINT \"HELLO\"\\n30 NEXT I\\n40 END\\n"
    ast = parse_dartmouth_basic(source)
    result = compile_basic(ast)
    # result.program: IrProgram ready for the GE-225 backend
    # result.var_regs["I"]: virtual register index for variable I
"""

from dartmouth_basic_ir_compiler.compiler import (
    CompileError,
    CompileResult,
    compile_basic,
)
from dartmouth_basic_ir_compiler.ge225_codes import (
    CARRIAGE_RETURN_CODE,
    GE225_CODES,
    ascii_to_ge225,
)

__all__ = [
    "compile_basic",
    "CompileResult",
    "CompileError",
    "GE225_CODES",
    "CARRIAGE_RETURN_CODE",
    "ascii_to_ge225",
]
