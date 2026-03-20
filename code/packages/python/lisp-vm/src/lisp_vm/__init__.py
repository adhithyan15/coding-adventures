"""Lisp VM — McCarthy's 1960 Lisp on the pluggable GenericVM framework.

This package provides a VM plugin that registers Lisp-specific opcodes
(CONS, CAR, CDR, MAKE_SYMBOL, etc.) with the GenericVM, plus garbage
collection integration, symbol interning, closures, and tail call
optimization.

Usage::

    from lisp_vm import create_lisp_vm, LispOp, NIL

    vm = create_lisp_vm()
    # ... execute Lisp bytecode via vm.execute(code)
"""

from lisp_vm.handlers import NIL, LispFunction
from lisp_vm.opcodes import LispOp
from lisp_vm.vm import create_lisp_vm

__all__ = [
    "LispFunction",
    "LispOp",
    "NIL",
    "create_lisp_vm",
]
