"""Symbolic VM — pluggable evaluator for the universal symbolic IR.

This package provides a generic tree-walking virtual machine for
:mod:`symbolic_ir` trees, together with two reference evaluation
policies:

- :class:`StrictBackend` — errors on unbound names, numeric only,
  Python-like semantics.
- :class:`SymbolicBackend` — leaves unbound names as free variables,
  applies algebraic identities, and knows enough calculus to compute
  derivatives through the standard rules.

Both backends share the same handler table for arithmetic, comparisons,
logic, binding, and elementary functions; the only difference is what
happens when an expression can't fold to a value.

Quick start::

    from macsyma_parser import parse_macsyma
    from macsyma_compiler import compile_macsyma
    from symbolic_vm import SymbolicBackend, VM

    program = "f(x) := x^2; diff(f(x), x);"
    statements = compile_macsyma(parse_macsyma(program))

    vm = VM(SymbolicBackend())
    result = vm.eval_program(statements)
    print(result)  # 2*x  (more or less)
"""

from symbolic_vm.backend import Backend, Handler, Rule, RulePredicate, RuleTransform
from symbolic_vm.backends import StrictBackend, SymbolicBackend
from symbolic_vm.vm import VM

__all__ = [
    "VM",
    "Backend",
    "StrictBackend",
    "SymbolicBackend",
    "Handler",
    "Rule",
    "RulePredicate",
    "RuleTransform",
]
