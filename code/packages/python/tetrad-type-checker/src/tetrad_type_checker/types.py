"""Core type system data structures for the Tetrad type checker (spec TET02b).

The type checker is a bridge between the parser (spec TET02) and the bytecode
compiler (spec TET03).  Its job is to:

  1. Walk the AST bottom-up, inferring types for every expression.
  2. Classify each function as FULLY_TYPED, PARTIALLY_TYPED, or UNTYPED.
  3. Return a TypeCheckResult the compiler consults to decide whether to emit
     feedback slots (slow, untyped path) or skip them (fast, typed path).

The three-tier system is the core innovation:

  FULLY_TYPED      All params and return annotated, all ops infer to u8.
                   Compiler emits zero feedback slots → saves ROM.
                   JIT compiles on the FIRST call (no warmup).

  PARTIALLY_TYPED  Some annotations present. Unknown ops get feedback slots.
                   JIT compiles after 10 calls.

  UNTYPED          No annotations at all.  All ops get feedback slots.
                   JIT compiles after 100 calls.

Tetrad v1 has exactly one concrete type: ``u8`` (unsigned 8-bit, 0–255, wraps
on overflow).  The infrastructure is designed to accommodate more types when
the Lisp front-end is added (u8, pair, symbol, closure, bool, nil).

Internal pseudo-types (never appear in source):
  ``Unknown`` — no annotation and no inference was possible
  ``Void``    — result of out(expr); has no meaningful value
"""

from __future__ import annotations

import enum
from dataclasses import dataclass


class FunctionTypeStatus(enum.Enum):
    """Classification of a function's type completeness.

    The JIT uses this to decide when (and whether) to compile a function to
    native code.  A FULLY_TYPED function is compiled before it is ever called.
    """

    FULLY_TYPED = "fully_typed"
    PARTIALLY_TYPED = "partially_typed"
    UNTYPED = "untyped"


@dataclass
class TypeInfo:
    """The inferred type of one expression node.

    ``ty`` is one of: ``"u8"`` (concrete), ``"Unknown"`` (no info),
    ``"Void"`` (out() result).

    ``source`` records how we learned the type:
      ``"annotation"`` — explicitly written in source (e.g. ``: u8``)
      ``"inferred"``   — derived from sub-expressions
      ``"unknown"``    — could not determine

    ``line`` and ``column`` mirror the expression's source position for
    error messages.
    """

    ty: str
    source: str
    line: int
    column: int


@dataclass
class FunctionType:
    """The declared signature of a function (collected in Phase 1).

    ``param_types[i]`` is the annotation for parameter i, or ``None`` if
    unannotated.  ``return_type`` is ``None`` if unannotated.

    This is what other functions see when they call this function.
    """

    param_types: list[str | None]
    return_type: str | None


@dataclass
class TypeEnvironment:
    """Type information in scope at a given point during type-checking.

    ``functions`` is the global function-signature table, built in Phase 1
    and read-only thereafter.

    ``variables`` maps variable names to their TypeInfo in the current scope.
    Each function body gets a child scope via ``child_scope()``.

    ``function_status`` is populated in Phase 4 (classification) after all
    function bodies have been checked.
    """

    functions: dict[str, FunctionType]
    variables: dict[str, TypeInfo]
    function_status: dict[str, FunctionTypeStatus]

    def lookup_var(self, name: str) -> TypeInfo | None:
        """Return the TypeInfo for ``name``, or None if not in scope."""
        return self.variables.get(name)

    def bind_var(self, name: str, info: TypeInfo) -> None:
        """Add or overwrite the binding for ``name`` in this scope."""
        self.variables[name] = info

    def child_scope(self) -> TypeEnvironment:
        """Return a new environment that inherits this scope's variables.

        Changes to the child's ``variables`` do not affect the parent.
        """
        return TypeEnvironment(
            functions=self.functions,
            variables=dict(self.variables),
            function_status=self.function_status,
        )


@dataclass
class TypeError:
    """A hard type error that should abort compilation.

    Examples: assigning Unknown to a declared u8 variable, or returning
    Unknown from a declared-u8 function.
    """

    message: str
    line: int
    column: int


@dataclass
class TypeWarning:
    """A soft warning — compilation proceeds but the programmer should know.

    Examples: unannotated function (JIT warmup required), typed function
    calling an untyped function (status downgraded to PARTIALLY_TYPED).
    """

    message: str
    line: int
    column: int
    hint: str = ""


@dataclass
class TypeCheckResult:
    """The complete output of the type checker.

    ``program`` is the original AST (unchanged — we do not mutate it).

    ``type_map`` maps ``id(expr_node)`` to a ``TypeInfo``.  The compiler
    reads ``type_map[id(node)].ty`` at every binary op and call site to
    decide whether to emit a feedback slot.

    ``env`` is the final type environment (useful for debugging and tests).

    ``errors`` are hard errors.  If non-empty, the compiler should abort.

    ``warnings`` are soft warnings.  The compiler proceeds regardless.
    """

    program: object  # Program — avoid circular import at the type level
    type_map: dict[int, TypeInfo]
    env: TypeEnvironment
    errors: list[TypeError]
    warnings: list[TypeWarning]
