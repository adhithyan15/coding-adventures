"""Lattice error types — structured errors for the AST-to-CSS compiler.

Every error in the Lattice compiler carries:

- A human-readable message explaining what went wrong.
- The line and column where the error occurred (from the originating token).

The error hierarchy mirrors the compiler's three passes:

Pass 1 (Module Resolution):
    ``ModuleNotFoundError`` — @use references a file that doesn't exist.

Pass 2 (Symbol Collection):
    ``ReturnOutsideFunctionError`` — @return appears outside a @function body.

Pass 3 (Expansion):
    ``UndefinedVariableError``  — $var referenced but never declared.
    ``UndefinedMixinError``     — @include references an unknown mixin.
    ``UndefinedFunctionError``  — function call references an unknown function.
    ``WrongArityError``         — mixin/function called with wrong arg count.
    ``CircularReferenceError``  — mixin or function calls itself (directly or
                                  indirectly), forming a cycle.
    ``TypeErrorInExpression``   — arithmetic on incompatible types (e.g., 10px + red).
    ``UnitMismatchError``       — arithmetic on incompatible units (e.g., 10px + 5s).
    ``MissingReturnError``      — function body has no @return statement.

All errors inherit from ``LatticeError`` so callers can catch the whole family
with a single ``except LatticeError`` clause.

Example::

    try:
        css = transpile_lattice(source)
    except LatticeError as e:
        print(f"Error at line {e.line}, column {e.column}: {e}")
"""

from __future__ import annotations


class LatticeError(Exception):
    """Base class for all Lattice compiler errors.

    Every subclass stores the line and column where the error occurred.
    These come from the token that triggered the error — the Lattice
    lexer embeds position info in every token.

    Attributes:
        message: Human-readable error description.
        line: 1-based line number in the source file.
        column: 1-based column number in the source file.
    """

    def __init__(self, message: str, line: int = 0, column: int = 0) -> None:
        self.message = message
        self.line = line
        self.column = column
        location = f" at line {line}, column {column}" if line else ""
        super().__init__(f"{message}{location}")


# ---------------------------------------------------------------------------
# Pass 1: Module Resolution Errors
# ---------------------------------------------------------------------------


class ModuleNotFoundError(LatticeError):
    """Raised when @use references a module that cannot be found.

    Example: @use "nonexistent";
    """

    def __init__(self, module_name: str, line: int = 0, column: int = 0) -> None:
        self.module_name = module_name
        super().__init__(f"Module '{module_name}' not found", line, column)


# ---------------------------------------------------------------------------
# Pass 2: Symbol Collection Errors
# ---------------------------------------------------------------------------


class ReturnOutsideFunctionError(LatticeError):
    """Raised when @return appears outside a @function body.

    Example: @return 42;  (at top level or inside a mixin)
    """

    def __init__(self, line: int = 0, column: int = 0) -> None:
        super().__init__("@return outside @function", line, column)


# ---------------------------------------------------------------------------
# Pass 3: Expansion Errors
# ---------------------------------------------------------------------------


class UndefinedVariableError(LatticeError):
    """Raised when a $variable is referenced but never declared.

    Example: color: $nonexistent;
    """

    def __init__(self, name: str, line: int = 0, column: int = 0) -> None:
        self.name = name
        super().__init__(f"Undefined variable '{name}'", line, column)


class UndefinedMixinError(LatticeError):
    """Raised when @include references a mixin that was never defined.

    Example: @include nonexistent;
    """

    def __init__(self, name: str, line: int = 0, column: int = 0) -> None:
        self.name = name
        super().__init__(f"Undefined mixin '{name}'", line, column)


class UndefinedFunctionError(LatticeError):
    """Raised when a function call references a function that was never defined.

    Note: this only applies to Lattice functions, not CSS functions like
    rgb(), calc(), var(), etc. CSS functions are passed through unchanged.

    Example: padding: spacing(2);  (if spacing was never defined)
    """

    def __init__(self, name: str, line: int = 0, column: int = 0) -> None:
        self.name = name
        super().__init__(f"Undefined function '{name}'", line, column)


class WrongArityError(LatticeError):
    """Raised when a mixin or function is called with the wrong number of args.

    The expected count accounts for parameters that have defaults — only
    parameters without defaults are required.

    Example: @mixin button($bg, $fg) called as @include button(red, blue, green);
    """

    def __init__(
        self,
        kind: str,
        name: str,
        expected: int,
        got: int,
        line: int = 0,
        column: int = 0,
    ) -> None:
        self.name = name
        self.expected = expected
        self.got = got
        super().__init__(
            f"{kind} '{name}' expects {expected} args, got {got}", line, column
        )


class CircularReferenceError(LatticeError):
    """Raised when a mixin or function calls itself, forming a cycle.

    The chain shows the full call path: a → b → a.

    Example: @mixin a { @include b; }  @mixin b { @include a; }
    """

    def __init__(
        self, kind: str, chain: list[str], line: int = 0, column: int = 0
    ) -> None:
        self.chain = chain
        chain_str = " → ".join(chain)
        super().__init__(f"Circular {kind}: {chain_str}", line, column)


class TypeErrorInExpression(LatticeError):
    """Raised when arithmetic is attempted on incompatible types.

    Example: 10px + red  (can't add a dimension and a color/ident)
    """

    def __init__(
        self, op: str, left: str, right: str, line: int = 0, column: int = 0
    ) -> None:
        self.op = op
        self.left_type = left
        self.right_type = right
        super().__init__(f"Cannot {op} '{left}' and '{right}'", line, column)


class UnitMismatchError(LatticeError):
    """Raised when arithmetic combines dimensions with incompatible units.

    Compatible units can be added/subtracted directly: 10px + 5px → 15px.
    Incompatible units produce calc() in CSS: 10px + 2em → calc(10px + 2em).
    But some combinations are never valid: 10px + 5s (length + time).

    Example: 10px + 5s
    """

    def __init__(
        self, left_unit: str, right_unit: str, line: int = 0, column: int = 0
    ) -> None:
        self.left_unit = left_unit
        self.right_unit = right_unit
        super().__init__(
            f"Cannot add '{left_unit}' and '{right_unit}' units", line, column
        )


class MissingReturnError(LatticeError):
    """Raised when a function body has no @return statement.

    Every @function must return a value via @return. A function body
    that contains only variable declarations or control flow with no
    @return in any reachable branch is an error.

    Example: @function noop($x) { $y: $x; }
    """

    def __init__(self, name: str, line: int = 0, column: int = 0) -> None:
        self.name = name
        super().__init__(f"Function '{name}' has no @return", line, column)
