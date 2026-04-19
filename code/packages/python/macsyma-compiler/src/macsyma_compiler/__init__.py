"""MACSYMA compiler — lowers a parsed MACSYMA AST to the universal
symbolic IR.

The parser produces a deeply nested generic ``ASTNode`` tree whose
nodes mirror the grammar's precedence cascade:
``assign → logical_or → logical_and → logical_not → comparison →
additive → multiplicative → unary → power → postfix → atom``.

This compiler walks that tree and emits a compact ``IRApply``-based
representation where every compound expression has the uniform shape
``IRApply(head, args)`` and ``head`` is a standard ``IRSymbol``.

Well-known MACSYMA function names are mapped to canonical IR heads:

- ``diff`` → ``D``
- ``integrate`` → ``Integrate``
- ``sin``/``cos``/``log``/``exp``/``sqrt`` → their capitalized IR heads

Other names pass through unchanged (``f(x)`` in MACSYMA becomes
``IRApply(IRSymbol('f'), (IRSymbol('x'),))`` in IR), so user-defined
functions coexist with standard library functions without special
handling.

Usage::

    from macsyma_parser import parse_macsyma
    from macsyma_compiler import compile_macsyma

    ast = parse_macsyma("diff(x^2 + 1, x);")
    ir_statements = compile_macsyma(ast)
    for stmt in ir_statements:
        print(stmt)
"""

from macsyma_compiler.compiler import CompileError, compile_expression, compile_macsyma

__all__ = [
    "compile_macsyma",
    "compile_expression",
    "CompileError",
]
