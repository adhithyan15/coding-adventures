"""Nib type checker — language-level type checking for the Nib language.

=============================================================================
WHAT THIS PACKAGE DOES
=============================================================================

The ``nib-type-checker`` package is Stage 3 of the Nib compiler pipeline.
It takes the untyped ``ASTNode`` tree produced by ``nib-parser`` and:

1. **Verifies type safety**: Every variable is declared before use, every
   expression has consistent types, every assignment matches its declared
   type, every function call passes the right number and types of arguments.

2. **Enforces language rules**: BCD arithmetic only uses ``+%`` and ``-``,
   ``for``-loop bounds are compile-time constants, no recursion, ``if``/``for``
   conditions are ``bool``.

3. **Annotates the AST**: Each expression node gets a ``._nib_type``
   attribute containing its resolved ``NibType``. Later stages (IR compiler,
   backend validator, code generator) read these annotations instead of
   repeating type inference.

The checker returns a ``TypeCheckResult``:

- ``result.ok`` — ``True`` if the program is type-safe.
- ``result.errors`` — list of ``TypeErrorDiagnostic`` objects, each with
  a ``message``, ``line``, and ``column``.
- ``result.typed_ast`` — the same AST annotated with type information.

=============================================================================
WHY LANGUAGE INVARIANTS ONLY?
=============================================================================

This checker enforces *language-level* invariants — rules that apply
regardless of what machine the program is compiled for. It does NOT check:

- **Call depth ≤ 2** — that is a constraint of the Intel 4004's 3-level
  hardware call stack, not the Nib language. It belongs in the
  ``intel-4004-ir-validator`` package.

- **Total static RAM ≤ 160 bytes** — also a hardware constraint.

- **Physical register count** — also a hardware constraint.

Keeping hardware constraints out of the type checker means the same
``NibTypeChecker`` can target the Intel 4004, an ARM Cortex-M0, a WASM
module, or any other backend without any modification.

=============================================================================
USAGE
=============================================================================

::

    from nib_parser import parse_nib
    from nib_type_checker import check

    source = '''
        const MAX: u8 = 100;
        fn add(a: u4, b: u4) -> u4 {
            return a +% b;
        }
        fn main() {
            let result: u4 = add(3, 4);
        }
    '''

    ast = parse_nib(source)
    result = check(ast)

    if result.ok:
        print("Type check passed!")
    else:
        for err in result.errors:
            print(f"Line {err.line}, Col {err.column}: {err.message}")

=============================================================================
PUBLIC API
=============================================================================

- ``check(ast: ASTNode) -> TypeCheckResult[ASTNode]`` — main entry point.
- ``NibTypeChecker`` — the checker class (if you need multiple instances
  or want to subclass).
- ``NibType`` — the four Nib types: ``U4``, ``U8``, ``BCD``, ``BOOL``.
- ``Symbol`` — symbol table entry (name + type + flags).
- ``ScopeChain`` — lexical scope chain used internally by the checker.
"""

from __future__ import annotations

from nib_type_checker.checker import NibTypeChecker, check
from nib_type_checker.scope import ScopeChain, Symbol
from nib_type_checker.types import NibType

__all__ = [
    "NibTypeChecker",
    "NibType",
    "ScopeChain",
    "Symbol",
    "check",
]
