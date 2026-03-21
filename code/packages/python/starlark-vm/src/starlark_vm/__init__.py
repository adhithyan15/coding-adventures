"""Starlark VM — Executes Starlark bytecode on the pluggable GenericVM.

This package provides a complete Starlark runtime:

- All ~50 opcode handlers for Starlark bytecode.
- All ~25 Starlark built-in functions (len, range, sorted, etc.).
- Starlark-specific restrictions (recursion limits, freezing).
- Factory function to create a configured GenericVM.
- One-call execute_starlark() convenience function.

Key exports:
    - create_starlark_vm: Factory that returns a configured GenericVM.
    - execute_starlark: Source → result in one call.
    - StarlarkResult: Execution result with variables, output, and traces.
"""

from starlark_vm.builtins import get_all_builtins
from starlark_vm.handlers import StarlarkFunction, StarlarkIterator
from starlark_vm.vm import StarlarkResult, create_starlark_vm, execute_starlark

__all__ = [
    "StarlarkFunction",
    "StarlarkIterator",
    "StarlarkResult",
    "create_starlark_vm",
    "execute_starlark",
    "get_all_builtins",
]
