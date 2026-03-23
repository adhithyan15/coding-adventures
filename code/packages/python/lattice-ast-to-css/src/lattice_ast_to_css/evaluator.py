"""Expression evaluator — compile-time evaluation of Lattice expressions.

Lattice expressions appear in three contexts:

1. ``@if`` conditions: ``@if $theme == dark { ... }``
2. ``@for`` bounds: ``@for $i from 1 through $count { ... }``
3. ``@return`` values: ``@return $n * 8px;``

The evaluator walks ``lattice_expression`` AST nodes and computes their
values at compile time. This is similar to constant folding in a compiler,
but Lattice evaluates ALL expressions at compile time (there's no runtime).

Value Types
-----------

The evaluator works with a small set of value types that mirror CSS/Lattice:

- **LatticeNumber**: Pure numbers like ``42``, ``3.14``
- **LatticeDimension**: Numbers with units like ``16px``, ``2em``, ``50vh``
- **LatticePercentage**: Numbers with ``%`` like ``50%``, ``100%``
- **LatticeString**: Quoted strings like ``"hello"``, ``'world'``
- **LatticeIdent**: Unquoted identifiers like ``red``, ``bold``, ``dark``
- **LatticeColor**: Hash colors like ``#4a90d9``, ``#fff``
- **LatticeBool**: Boolean values ``true`` and ``false``
- **LatticeNull**: The null value (falsy, like Sass's ``null``)
- **LatticeList**: Comma-separated lists (for ``@each``)

Operator Precedence
-------------------

From tightest to loosest binding (matching the grammar):

1. Unary minus: ``-$x``
2. Multiplication: ``$a * $b``
3. Addition/subtraction: ``$a + $b``, ``$a - $b``
4. Comparison: ``==``, ``!=``, ``>``, ``>=``, ``<=``
5. Logical AND: ``$a and $b``
6. Logical OR: ``$a or $b``

The grammar encodes this precedence via nested rules, so the evaluator
just recursively evaluates the AST — no precedence climbing needed.

Arithmetic Rules
----------------

Addition/subtraction:

- Number ± Number → Number
- Dimension ± Dimension (same unit) → Dimension
- Percentage ± Percentage → Percentage

Multiplication:

- Number × Number → Number
- Number × Dimension → Dimension (scales the value)
- Dimension × Number → Dimension (commutative)

Everything else raises ``TypeErrorInExpression``.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from lattice_ast_to_css.errors import TypeErrorInExpression
from lattice_ast_to_css.scope import ScopeChain


# ---------------------------------------------------------------------------
# Value Types
# ---------------------------------------------------------------------------
#
# These are the types that flow through the evaluator. Each represents a
# CSS/Lattice value that the compiler can manipulate at compile time.
#
# Why not just use Python primitives? Because we need to track units
# (10px vs 10em), distinguish idents from strings ("red" vs red), and
# handle CSS-specific semantics.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class LatticeNumber:
    """A pure number without units. Maps to CSS NUMBER token.

    Examples: 42, 3.14, 0, -1
    """

    value: float

    def __str__(self) -> str:
        # Emit integers without decimal point: 42 not 42.0
        if self.value == int(self.value):
            return str(int(self.value))
        return str(self.value)


@dataclass(frozen=True)
class LatticeDimension:
    """A number with a CSS unit. Maps to CSS DIMENSION token.

    Examples: 16px, 2em, 1.5rem, 100vh, 300ms

    The unit is a string like "px", "em", "rem", etc. Arithmetic
    is only valid between dimensions with the same unit.
    """

    value: float
    unit: str

    def __str__(self) -> str:
        if self.value == int(self.value):
            return f"{int(self.value)}{self.unit}"
        return f"{self.value}{self.unit}"


@dataclass(frozen=True)
class LatticePercentage:
    """A percentage value. Maps to CSS PERCENTAGE token.

    Examples: 50%, 100%, 33.33%
    """

    value: float

    def __str__(self) -> str:
        if self.value == int(self.value):
            return f"{int(self.value)}%"
        return f"{self.value}%"


@dataclass(frozen=True)
class LatticeString:
    """A quoted string value. Maps to CSS STRING token.

    Examples: "hello", 'world'

    The quotes are not stored — they're added back during CSS emission.
    """

    value: str

    def __str__(self) -> str:
        return f'"{self.value}"'


@dataclass(frozen=True)
class LatticeIdent:
    """An unquoted identifier. Maps to CSS IDENT token.

    Examples: red, bold, dark, sans-serif, transparent

    CSS color keywords (red, blue, etc.) are idents, not a special type.
    The evaluator treats them as opaque identifiers — no color arithmetic.
    """

    value: str

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True)
class LatticeColor:
    """A hex color value. Maps to CSS HASH token in color context.

    Examples: #4a90d9, #fff, #00000080

    Stored as the raw string including the ``#`` prefix.
    """

    value: str

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True)
class LatticeBool:
    """A boolean value — true or false.

    Lattice boolean literals are idents that the grammar matches via
    literal text: ``"true"`` and ``"false"``.

    Truthiness: ``false`` is falsy, ``true`` is truthy.
    """

    value: bool

    def __str__(self) -> str:
        return "true" if self.value else "false"


@dataclass(frozen=True)
class LatticeNull:
    """The null value.

    ``null`` is falsy and stringifies to empty string (like Sass).
    Used for optional parameters and missing values.
    """

    def __str__(self) -> str:
        return ""


@dataclass(frozen=True)
class LatticeList:
    """A comma-separated list of values.

    Used in ``@each`` directives and multi-value declarations.
    Each item is a LatticeValue.
    """

    items: tuple[Any, ...]  # tuple for frozen=True compatibility

    def __str__(self) -> str:
        return ", ".join(str(item) for item in self.items)


# Type alias for all Lattice value types.
LatticeValue = (
    LatticeNumber
    | LatticeDimension
    | LatticePercentage
    | LatticeString
    | LatticeIdent
    | LatticeColor
    | LatticeBool
    | LatticeNull
    | LatticeList
)


# ---------------------------------------------------------------------------
# Truthiness
# ---------------------------------------------------------------------------


def is_truthy(value: LatticeValue) -> bool:
    """Determine whether a Lattice value is truthy.

    Truthiness rules (matching Sass conventions):

    - ``false`` → falsy
    - ``null`` → falsy
    - ``0`` (LatticeNumber with value 0) → falsy
    - Everything else → truthy (including empty strings and empty lists)

    Args:
        value: The value to test.

    Returns:
        True if the value is truthy, False otherwise.
    """
    if isinstance(value, LatticeBool):
        return value.value
    if isinstance(value, LatticeNull):
        return False
    if isinstance(value, LatticeNumber) and value.value == 0:
        return False
    return True


# ---------------------------------------------------------------------------
# Token ↔ Value Conversion
# ---------------------------------------------------------------------------
#
# These functions bridge the gap between the parser's token world and the
# evaluator's value world. The parser gives us AST nodes containing tokens;
# the evaluator needs typed values for arithmetic and comparison.
# ---------------------------------------------------------------------------


def _token_type_name(token: object) -> str:
    """Get the string name of a token's type.

    Token types can be either strings (from GrammarLexer) or enum values
    (from hand-written lexers). This handles both.
    """
    t = token.type  # type: ignore[attr-defined]
    if isinstance(t, str):
        return t
    return t.name


def token_to_value(token: object) -> LatticeValue:
    """Convert a parser Token to a LatticeValue.

    Maps token types to value types:

    - NUMBER → LatticeNumber
    - DIMENSION → LatticeDimension
    - PERCENTAGE → LatticePercentage
    - STRING → LatticeString
    - IDENT → LatticeIdent (or LatticeBool/LatticeNull for literals)
    - HASH → LatticeColor

    Args:
        token: A Token from the parser.

    Returns:
        The corresponding LatticeValue.
    """
    type_name = _token_type_name(token)
    value = token.value  # type: ignore[attr-defined]

    if type_name == "NUMBER":
        return LatticeNumber(float(value))

    elif type_name == "DIMENSION":
        # Split "16px" into number (16) and unit (px).
        # Find where digits/dots end and the unit begins.
        i = 0
        while i < len(value) and (value[i].isdigit() or value[i] == "." or (i == 0 and value[i] == "-")):
            i += 1
        num = float(value[:i])
        unit = value[i:]
        return LatticeDimension(num, unit)

    elif type_name == "PERCENTAGE":
        # "50%" → LatticePercentage(50)
        return LatticePercentage(float(value.rstrip("%")))

    elif type_name == "STRING":
        return LatticeString(value)

    elif type_name == "HASH":
        return LatticeColor(value)

    elif type_name == "IDENT":
        if value == "true":
            return LatticeBool(True)
        elif value == "false":
            return LatticeBool(False)
        elif value == "null":
            return LatticeNull()
        return LatticeIdent(value)

    # Fallback for unexpected token types — treat as ident.
    return LatticeIdent(str(value))


def value_to_css(value: LatticeValue) -> str:
    """Convert a LatticeValue to its CSS text representation.

    This is used when substituting evaluated values back into CSS output.
    Each value type knows how to represent itself as CSS text.

    Args:
        value: The value to convert.

    Returns:
        CSS text representation of the value.
    """
    return str(value)


# ---------------------------------------------------------------------------
# Expression Evaluator
# ---------------------------------------------------------------------------


class ExpressionEvaluator:
    """Evaluates Lattice expression AST nodes at compile time.

    The evaluator walks the AST produced by the grammar parser's expression
    rules (``lattice_expression``, ``lattice_or_expr``, etc.) and computes
    a ``LatticeValue`` result.

    The grammar's nesting of rules already encodes operator precedence, so
    the evaluator just recursively evaluates each node without needing its
    own precedence table.

    Example::

        evaluator = ExpressionEvaluator(scope)
        result = evaluator.evaluate(expression_node)
        # result is a LatticeValue like LatticeNumber(42)

    Args:
        scope: The current scope chain for variable lookup.
    """

    def __init__(self, scope: ScopeChain) -> None:
        self.scope = scope

    def evaluate(self, node: object) -> LatticeValue:
        """Evaluate an expression AST node.

        Dispatches on ``rule_name`` to the appropriate handler. If the
        node is a token (leaf), converts it directly to a value.

        Args:
            node: An ASTNode from the parser.

        Returns:
            The evaluated LatticeValue.
        """
        # If it's a raw token (not an ASTNode), convert directly.
        if not hasattr(node, "rule_name"):
            return token_to_value(node)

        rule = node.rule_name  # type: ignore[attr-defined]

        # Dispatch to handler based on rule name.
        handler = getattr(self, f"_eval_{rule}", None)
        if handler:
            return handler(node)

        # For wrapper rules with a single child, unwrap.
        children = node.children  # type: ignore[attr-defined]
        if len(children) == 1:
            return self.evaluate(children[0])

        # Default: try to evaluate the first meaningful child.
        for child in children:
            if hasattr(child, "rule_name") or hasattr(child, "type"):
                return self.evaluate(child)

        return LatticeNull()

    def _eval_lattice_expression(self, node: object) -> LatticeValue:
        """lattice_expression = lattice_or_expr ;"""
        children = node.children  # type: ignore[attr-defined]
        return self.evaluate(children[0])

    def _eval_lattice_or_expr(self, node: object) -> LatticeValue:
        """lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;

        Short-circuit evaluation: returns first truthy operand, or last.
        """
        children = node.children  # type: ignore[attr-defined]
        # Children alternate: and_expr "or" and_expr "or" and_expr ...
        result = self.evaluate(children[0])
        i = 1
        while i < len(children):
            # Skip the "or" token
            if hasattr(children[i], "value") and children[i].value == "or":  # type: ignore[attr-defined]
                i += 1
                continue
            if is_truthy(result):
                return result
            result = self.evaluate(children[i])
            i += 1
        return result

    def _eval_lattice_and_expr(self, node: object) -> LatticeValue:
        """lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;

        Short-circuit evaluation: returns first falsy operand, or last.
        """
        children = node.children  # type: ignore[attr-defined]
        result = self.evaluate(children[0])
        i = 1
        while i < len(children):
            if hasattr(children[i], "value") and children[i].value == "and":  # type: ignore[attr-defined]
                i += 1
                continue
            if not is_truthy(result):
                return result
            result = self.evaluate(children[i])
            i += 1
        return result

    def _eval_lattice_comparison(self, node: object) -> LatticeValue:
        """lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;"""
        children = node.children  # type: ignore[attr-defined]
        left = self.evaluate(children[0])

        if len(children) == 1:
            return left

        # Find the comparison_op node and right operand.
        op_node = None
        right_node = None
        for i, child in enumerate(children[1:], 1):
            if hasattr(child, "rule_name") and child.rule_name == "comparison_op":
                op_node = child
            elif op_node is not None:
                right_node = child
                break

        if op_node is None or right_node is None:
            return left

        right = self.evaluate(right_node)
        op_token = op_node.children[0]  # type: ignore[attr-defined]
        op_type = _token_type_name(op_token)

        return self._compare(left, right, op_type)

    def _compare(
        self, left: LatticeValue, right: LatticeValue, op: str
    ) -> LatticeBool:
        """Perform a comparison operation.

        Compares numeric values when both operands are numeric (same type).
        Falls back to string equality for other types.
        """
        # Numeric comparison for same types
        if isinstance(left, (LatticeNumber, LatticeDimension, LatticePercentage)) and type(left) == type(right):
            lv = left.value
            rv = right.value  # type: ignore[attr-defined]
            # For dimensions, units must match for ordering
            if isinstance(left, LatticeDimension) and isinstance(right, LatticeDimension):
                if left.unit != right.unit and op not in ("EQUALS_EQUALS", "NOT_EQUALS"):
                    return LatticeBool(op == "NOT_EQUALS")

            if op == "EQUALS_EQUALS":
                if isinstance(left, LatticeDimension) and isinstance(right, LatticeDimension):
                    return LatticeBool(lv == rv and left.unit == right.unit)
                return LatticeBool(lv == rv)
            elif op == "NOT_EQUALS":
                if isinstance(left, LatticeDimension) and isinstance(right, LatticeDimension):
                    return LatticeBool(lv != rv or left.unit != right.unit)
                return LatticeBool(lv != rv)
            elif op == "GREATER":
                return LatticeBool(lv > rv)
            elif op == "GREATER_EQUALS":
                return LatticeBool(lv >= rv)
            elif op == "LESS_EQUALS":
                return LatticeBool(lv <= rv)

        # Equality comparison via string representation for mixed types
        left_str = str(left)
        right_str = str(right)
        if op == "EQUALS_EQUALS":
            return LatticeBool(left_str == right_str)
        elif op == "NOT_EQUALS":
            return LatticeBool(left_str != right_str)

        # Can't order non-numeric types
        return LatticeBool(False)

    def _eval_lattice_additive(self, node: object) -> LatticeValue:
        """lattice_additive = lattice_multiplicative
                             { ( PLUS | MINUS ) lattice_multiplicative } ;"""
        children = node.children  # type: ignore[attr-defined]
        result = self.evaluate(children[0])

        i = 1
        while i < len(children):
            child = children[i]
            if hasattr(child, "value"):
                op = child.value  # type: ignore[attr-defined]
                if op in ("+", "-"):
                    i += 1
                    right = self.evaluate(children[i])
                    if op == "+":
                        result = self._add(result, right)
                    else:
                        result = self._subtract(result, right)
            i += 1
        return result

    def _add(self, left: LatticeValue, right: LatticeValue) -> LatticeValue:
        """Addition: Number + Number, Dimension + Dimension (same unit), etc."""
        if isinstance(left, LatticeNumber) and isinstance(right, LatticeNumber):
            return LatticeNumber(left.value + right.value)

        if isinstance(left, LatticeDimension) and isinstance(right, LatticeDimension):
            if left.unit == right.unit:
                return LatticeDimension(left.value + right.value, left.unit)
            raise TypeErrorInExpression("add", str(left), str(right))

        if isinstance(left, LatticePercentage) and isinstance(right, LatticePercentage):
            return LatticePercentage(left.value + right.value)

        # String concatenation
        if isinstance(left, LatticeString) and isinstance(right, LatticeString):
            return LatticeString(left.value + right.value)

        raise TypeErrorInExpression("add", str(left), str(right))

    def _subtract(self, left: LatticeValue, right: LatticeValue) -> LatticeValue:
        """Subtraction: mirrors addition but subtracts."""
        if isinstance(left, LatticeNumber) and isinstance(right, LatticeNumber):
            return LatticeNumber(left.value - right.value)

        if isinstance(left, LatticeDimension) and isinstance(right, LatticeDimension):
            if left.unit == right.unit:
                return LatticeDimension(left.value - right.value, left.unit)
            raise TypeErrorInExpression("subtract", str(left), str(right))

        if isinstance(left, LatticePercentage) and isinstance(right, LatticePercentage):
            return LatticePercentage(left.value - right.value)

        raise TypeErrorInExpression("subtract", str(left), str(right))

    def _eval_lattice_multiplicative(self, node: object) -> LatticeValue:
        """lattice_multiplicative = lattice_unary { STAR lattice_unary } ;"""
        children = node.children  # type: ignore[attr-defined]
        result = self.evaluate(children[0])

        i = 1
        while i < len(children):
            child = children[i]
            if hasattr(child, "value") and child.value == "*":  # type: ignore[attr-defined]
                i += 1
                right = self.evaluate(children[i])
                result = self._multiply(result, right)
            i += 1
        return result

    def _multiply(self, left: LatticeValue, right: LatticeValue) -> LatticeValue:
        """Multiplication.

        - Number × Number → Number
        - Number × Dimension → Dimension
        - Dimension × Number → Dimension
        - Number × Percentage → Percentage
        - Percentage × Number → Percentage
        """
        if isinstance(left, LatticeNumber) and isinstance(right, LatticeNumber):
            return LatticeNumber(left.value * right.value)

        if isinstance(left, LatticeNumber) and isinstance(right, LatticeDimension):
            return LatticeDimension(left.value * right.value, right.unit)

        if isinstance(left, LatticeDimension) and isinstance(right, LatticeNumber):
            return LatticeDimension(left.value * right.value, left.unit)

        if isinstance(left, LatticeNumber) and isinstance(right, LatticePercentage):
            return LatticePercentage(left.value * right.value)

        if isinstance(left, LatticePercentage) and isinstance(right, LatticeNumber):
            return LatticePercentage(left.value * right.value)

        raise TypeErrorInExpression("multiply", str(left), str(right))

    def _eval_lattice_unary(self, node: object) -> LatticeValue:
        """lattice_unary = MINUS lattice_unary | lattice_primary ;"""
        children = node.children  # type: ignore[attr-defined]

        # Check if first child is a MINUS token
        if len(children) >= 2 and hasattr(children[0], "value") and children[0].value == "-":  # type: ignore[attr-defined]
            operand = self.evaluate(children[1])
            return self._negate(operand)

        return self.evaluate(children[0])

    def _negate(self, value: LatticeValue) -> LatticeValue:
        """Negate a numeric value."""
        if isinstance(value, LatticeNumber):
            return LatticeNumber(-value.value)
        if isinstance(value, LatticeDimension):
            return LatticeDimension(-value.value, value.unit)
        if isinstance(value, LatticePercentage):
            return LatticePercentage(-value.value)
        raise TypeErrorInExpression("negate", str(value), "")

    def _eval_lattice_primary(self, node: object) -> LatticeValue:
        """lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
                          | STRING | IDENT | HASH
                          | "true" | "false" | "null"
                          | function_call
                          | LPAREN lattice_expression RPAREN ;"""
        children = node.children  # type: ignore[attr-defined]

        for child in children:
            # Skip parentheses tokens
            if hasattr(child, "value"):
                val = child.value  # type: ignore[attr-defined]
                type_name = _token_type_name(child)

                if type_name == "VARIABLE":
                    # Look up the variable in scope
                    result = self.scope.get(val)
                    if result is None:
                        # Return the ident for now; transformer handles errors
                        return LatticeIdent(val)
                    if isinstance(result, (LatticeNumber, LatticeDimension, LatticePercentage,
                                          LatticeString, LatticeIdent, LatticeColor,
                                          LatticeBool, LatticeNull, LatticeList)):
                        return result
                    # If it's an ASTNode (e.g., value_list), extract its value
                    if hasattr(result, "rule_name"):
                        return self._extract_value_from_ast(result)
                    # If it's a raw token, convert it
                    return token_to_value(result)

                if type_name in ("LPAREN", "RPAREN"):
                    continue

                return token_to_value(child)

            # It's an ASTNode — recurse
            if hasattr(child, "rule_name"):
                return self.evaluate(child)

        return LatticeNull()

    def _extract_value_from_ast(self, node: object) -> LatticeValue:
        """Extract a LatticeValue from an AST node.

        When a variable is bound to a value_list node (from the parser),
        we need to extract the actual value. A value_list like "dark"
        contains a single value node wrapping an IDENT token.

        For multi-token value_lists (e.g., "Helvetica, sans-serif"),
        we take the first token's value.
        """
        if hasattr(node, "children"):
            children = node.children  # type: ignore[attr-defined]
            for child in children:
                if not hasattr(child, "rule_name"):
                    # It's a token — convert it
                    return token_to_value(child)
                else:
                    # Recurse into child nodes
                    result = self._extract_value_from_ast(child)
                    if not isinstance(result, LatticeNull):
                        return result
        return LatticeNull()

    def _eval_comparison_op(self, node: object) -> LatticeValue:
        """comparison_op is handled by _eval_lattice_comparison."""
        # This shouldn't be called directly
        children = node.children  # type: ignore[attr-defined]
        return token_to_value(children[0])
