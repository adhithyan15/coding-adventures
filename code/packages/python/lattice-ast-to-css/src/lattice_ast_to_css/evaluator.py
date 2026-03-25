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

    Stored as the raw string including the ``#`` prefix. Provides
    conversion helpers for RGB and HSL color spaces, needed by
    Lattice v2 built-in color functions (lighten, darken, mix, etc.).

    **Hex parsing:**

    +-----------+---------+-------+
    | Format    | Example | Alpha |
    +===========+=========+=======+
    | #RGB      | #f00    | 1.0   |
    | #RRGGBB   | #ff0000 | 1.0   |
    | #RRGGBBAA | #ff000080 | 0.5 |
    +-----------+---------+-------+

    **HSL conversion** uses the standard algorithm from CSS Color Level 4.
    Hue is in degrees (0-360), saturation and lightness are percentages (0-100).
    """

    value: str

    def to_rgb(self) -> tuple[int, int, int, float]:
        """Parse hex string to (r, g, b, a) where r/g/b are 0-255, a is 0-1.

        Handles #RGB (3-char shorthand), #RRGGBB (6-char), and
        #RRGGBBAA (8-char with alpha) formats.
        """
        h = self.value.lstrip("#")
        if len(h) == 3:
            r, g, b = int(h[0] * 2, 16), int(h[1] * 2, 16), int(h[2] * 2, 16)
            return (r, g, b, 1.0)
        elif len(h) == 6:
            r, g, b = int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)
            return (r, g, b, 1.0)
        elif len(h) == 8:
            r, g, b = int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)
            a = int(h[6:8], 16) / 255.0
            return (r, g, b, a)
        return (0, 0, 0, 1.0)

    def to_hsl(self) -> tuple[float, float, float, float]:
        """Convert to (h, s, l, a) where h is 0-360, s/l are 0-100, a is 0-1.

        Uses the standard RGB-to-HSL algorithm:

        1. Normalize RGB to 0-1 range
        2. Find min/max channel values
        3. Lightness = (min + max) / 2
        4. If min == max: achromatic (s=0, h=0)
        5. Saturation depends on lightness:
           - L <= 0.5: S = (max-min) / (max+min)
           - L > 0.5:  S = (max-min) / (2 - max - min)
        6. Hue depends on which channel is max:
           - R is max: H = (G-B) / (max-min)
           - G is max: H = 2 + (B-R) / (max-min)
           - B is max: H = 4 + (R-G) / (max-min)
        7. Hue is in 0-6 range, multiply by 60 to get degrees
        """
        r, g, b, a = self.to_rgb()
        rf, gf, bf = r / 255.0, g / 255.0, b / 255.0
        mx = max(rf, gf, bf)
        mn = min(rf, gf, bf)
        light = (mx + mn) / 2.0

        if mx == mn:
            return (0.0, 0.0, light * 100.0, a)

        d = mx - mn
        sat = d / (2.0 - mx - mn) if light > 0.5 else d / (mx + mn)

        if mx == rf:
            hue = (gf - bf) / d + (6.0 if gf < bf else 0.0)
        elif mx == gf:
            hue = (bf - rf) / d + 2.0
        else:
            hue = (rf - gf) / d + 4.0
        hue *= 60.0

        return (hue, sat * 100.0, light * 100.0, a)

    @staticmethod
    def from_rgb(r: int, g: int, b: int, a: float = 1.0) -> "LatticeColor":
        """Create a LatticeColor from RGB(A) components.

        Clamps each channel to its valid range before encoding as hex.
        If alpha is 1.0, emits #RRGGBB; otherwise emits rgba() notation.
        """
        r = max(0, min(255, round(r)))
        g = max(0, min(255, round(g)))
        b = max(0, min(255, round(b)))
        a = max(0.0, min(1.0, a))
        if a >= 1.0:
            return LatticeColor(f"#{r:02x}{g:02x}{b:02x}")
        return LatticeColor(f"rgba({r}, {g}, {b}, {a})")

    @staticmethod
    def from_hsl(h: float, s: float, l: float, a: float = 1.0) -> "LatticeColor":
        """Create a LatticeColor from HSL(A) components.

        Uses the standard HSL-to-RGB algorithm:

        1. Normalize h to 0-360, s/l to 0-1
        2. If s == 0: achromatic (r = g = b = l)
        3. Otherwise compute two helper values (q, p) and use
           hue-to-RGB for each channel offset by 1/3 of the hue circle.
        """
        h = h % 360.0
        s = max(0.0, min(100.0, s)) / 100.0
        l = max(0.0, min(100.0, l)) / 100.0

        if s == 0.0:
            v = round(l * 255)
            return LatticeColor.from_rgb(v, v, v, a)

        q = l * (1 + s) if l < 0.5 else l + s - l * s
        p = 2 * l - q

        def hue_to_rgb(p: float, q: float, t: float) -> float:
            if t < 0: t += 1
            if t > 1: t -= 1
            if t < 1 / 6: return p + (q - p) * 6 * t
            if t < 1 / 2: return q
            if t < 2 / 3: return p + (q - p) * (2 / 3 - t) * 6
            return p

        h_norm = h / 360.0
        r = round(hue_to_rgb(p, q, h_norm + 1 / 3) * 255)
        g = round(hue_to_rgb(p, q, h_norm) * 255)
        b = round(hue_to_rgb(p, q, h_norm - 1 / 3) * 255)

        return LatticeColor.from_rgb(r, g, b, a)

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


@dataclass(frozen=True)
class LatticeMap:
    """An ordered key-value map — Lattice v2 value type.

    Maps are written as parenthesized key-value pairs:

    .. code-block:: scss

        $theme: (
            primary: #4a90d9,
            secondary: #7b68ee,
            background: #ffffff,
        );

    Internally, a map is stored as a tuple of (key, value) pairs to
    maintain insertion order while remaining hashable (frozen dataclass).

    **Lookup semantics:**

    - Keys are strings (identifiers are treated as strings for lookup).
    - Duplicate keys: the last value wins (no error).
    - Maps are always truthy, even when empty.
    - Maps cannot be directly used as CSS values — attempting to do so
      raises a ``TypeError``.

    **Access:** Exclusively through built-in functions:

    - ``map-get($map, $key)`` — get value for key
    - ``map-keys($map)`` — list of all keys
    - ``map-values($map)`` — list of all values
    - ``map-has-key($map, $key)`` — boolean existence check
    - ``map-merge($map1, $map2)`` — combine two maps
    - ``map-remove($map, $keys...)`` — remove keys

    The tuple-of-tuples structure is used instead of a dict because:

    1. It's compatible with ``frozen=True`` (dicts are mutable).
    2. It preserves insertion order explicitly.
    3. Map equality compares keys in insertion order.
    """

    items: tuple[tuple[str, Any], ...]

    def get(self, key: str) -> Any:
        """Look up a value by key. Returns None if not found."""
        for k, v in self.items:
            if k == key:
                return v
        return None

    def keys(self) -> tuple[str, ...]:
        """Return all keys in insertion order."""
        return tuple(k for k, _ in self.items)

    def values(self) -> tuple[Any, ...]:
        """Return all values in insertion order."""
        return tuple(v for _, v in self.items)

    def has_key(self, key: str) -> bool:
        """Check if a key exists in the map."""
        return any(k == key for k, _ in self.items)

    def __str__(self) -> str:
        entries = ", ".join(f"{k}: {v}" for k, v in self.items)
        return f"({entries})"


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
    | LatticeMap
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

    def _eval_value_list(self, node: object) -> LatticeValue:
        """value_list — produced by variable substitution.

        When ``expand_variable_declaration`` substitutes ``$i + 1``, the
        evaluator receives a ``value_list`` AST node whose children are
        ``[NUMBER(2), PLUS, NUMBER(1)]``.  If arithmetic operators are
        present we delegate to the existing additive handler; otherwise
        we simply evaluate the first child.
        """
        children = node.children  # type: ignore[attr-defined]
        if len(children) <= 1:
            return self.evaluate(children[0]) if children else LatticeNull()
        has_ops = any(
            hasattr(c, "value")
            and not hasattr(c, "rule_name")
            and c.value in ("+", "-", "*")  # type: ignore[attr-defined]
            for c in children
        )
        if has_ops:
            return self._eval_lattice_additive(node)
        return self.evaluate(children[0])

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
                                          LatticeBool, LatticeNull, LatticeList,
                                          LatticeMap)):
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

    def _eval_function_call(self, node: object) -> LatticeValue:
        """Evaluate a function_call AST node.

        This handles built-in Lattice functions (map-get, lighten, etc.).
        If the function is not a known built-in, returns LatticeNull (the
        transformer handles user-defined functions separately).
        """
        children = node.children  # type: ignore[attr-defined]
        func_name = None
        args: list[LatticeValue] = []

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "FUNCTION":
                    func_name = child.value.rstrip("(")  # type: ignore[attr-defined]
            elif hasattr(child, "rule_name") and child.rule_name == "function_args":  # type: ignore[attr-defined]
                args = self._collect_function_args(child)

        if func_name and func_name in BUILTIN_FUNCTIONS:
            return BUILTIN_FUNCTIONS[func_name](args, self.scope)

        return LatticeNull()

    def _collect_function_args(self, node: object) -> list[LatticeValue]:
        """Collect evaluated arguments from a function_args AST node.

        Splits on COMMA tokens to produce individual argument values.
        Each argument is evaluated as an expression if possible.
        """
        args: list[LatticeValue] = []
        current_tokens: list[object] = []

        for child in node.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                if _token_type_name(child) == "COMMA":
                    if current_tokens:
                        args.append(self._eval_arg_tokens(current_tokens))
                        current_tokens = []
                    continue
            if hasattr(child, "rule_name") and child.rule_name == "function_arg":  # type: ignore[attr-defined]
                for ic in child.children:  # type: ignore[attr-defined]
                    if not hasattr(ic, "rule_name"):
                        if _token_type_name(ic) == "COMMA":
                            if current_tokens:
                                args.append(self._eval_arg_tokens(current_tokens))
                                current_tokens = []
                            continue
                        current_tokens.append(ic)
                    else:
                        # It's an AST node (expression) — evaluate directly
                        args.append(self.evaluate(ic))
                        current_tokens = []

        if current_tokens:
            args.append(self._eval_arg_tokens(current_tokens))

        return args

    def _eval_arg_tokens(self, tokens: list[object]) -> LatticeValue:
        """Evaluate a sequence of tokens as a single argument value."""
        if len(tokens) == 1:
            tok = tokens[0]
            type_name = _token_type_name(tok)
            if type_name == "VARIABLE":
                result = self.scope.get(tok.value)  # type: ignore[attr-defined]
                if result is not None:
                    if isinstance(result, (LatticeNumber, LatticeDimension,
                                          LatticePercentage, LatticeString,
                                          LatticeIdent, LatticeColor,
                                          LatticeBool, LatticeNull,
                                          LatticeList, LatticeMap)):
                        return result
                    if hasattr(result, "rule_name"):
                        return self._extract_value_from_ast(result)
            return token_to_value(tok)
        # Multiple tokens — take the first meaningful one
        if tokens:
            return token_to_value(tokens[0])
        return LatticeNull()


# ---------------------------------------------------------------------------
# Built-in Function Registry — Lattice v2
# ---------------------------------------------------------------------------
#
# Built-in functions are registered in a dictionary keyed by function name.
# Each function takes a list of LatticeValue arguments and a ScopeChain,
# and returns a LatticeValue.
#
# Functions are organized by category:
#
# 1. **Map functions**: map-get, map-keys, map-values, map-has-key,
#    map-merge, map-remove
# 2. **Color functions**: lighten, darken, saturate, desaturate,
#    adjust-hue, complement, mix, rgba, red, green, blue, hue,
#    saturation, lightness
# 3. **List functions**: nth, length, join, append, index
# 4. **Type functions**: type-of, unit, unitless, comparable
# 5. **Math functions**: math.div, math.floor, math.ceil, math.round,
#    math.abs, math.min, math.max
# ---------------------------------------------------------------------------

import math as _math


def _expect_type(value: LatticeValue, expected_type: type, type_name: str) -> None:
    """Validate that a value is of the expected type.

    Raises TypeErrorInExpression if the type doesn't match.
    """
    if not isinstance(value, expected_type):
        actual = _type_name_of(value)
        raise TypeErrorInExpression(
            "use", f"Expected a {type_name}, got {actual}", ""
        )


def _type_name_of(value: LatticeValue) -> str:
    """Return the Lattice type name for a value.

    Maps internal Python types to user-facing type strings:

    +--------------------+-----------+
    | Python type        | Lattice   |
    +====================+===========+
    | LatticeNumber      | "number"  |
    | LatticeDimension   | "number"  |
    | LatticePercentage  | "number"  |
    | LatticeString      | "string"  |
    | LatticeIdent       | "string"  |
    | LatticeColor       | "color"   |
    | LatticeBool        | "bool"    |
    | LatticeNull        | "null"    |
    | LatticeList        | "list"    |
    | LatticeMap         | "map"     |
    +--------------------+-----------+
    """
    if isinstance(value, (LatticeNumber, LatticeDimension, LatticePercentage)):
        return "number"
    if isinstance(value, (LatticeString, LatticeIdent)):
        return "string"
    if isinstance(value, LatticeColor):
        return "color"
    if isinstance(value, LatticeBool):
        return "bool"
    if isinstance(value, LatticeNull):
        return "null"
    if isinstance(value, LatticeList):
        return "list"
    if isinstance(value, LatticeMap):
        return "map"
    return "unknown"


def _get_numeric_value(v: LatticeValue) -> float:
    """Extract the numeric value from a number-like LatticeValue."""
    if isinstance(v, (LatticeNumber, LatticeDimension, LatticePercentage)):
        return v.value
    raise TypeErrorInExpression("use", f"Expected a number, got {_type_name_of(v)}", "")


# ---------------------------------------------------------------------------
# Map Functions
# ---------------------------------------------------------------------------


def _builtin_map_get(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """map-get($map, $key) — get value for key, or null if not found."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "map-get requires 2 arguments", "")
    m = args[0]
    if not isinstance(m, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m)}", "")
    key = str(args[1]).strip('"')
    result = m.get(key)
    return result if result is not None else LatticeNull()


def _builtin_map_keys(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """map-keys($map) — return all keys as a list."""
    if not args:
        raise TypeErrorInExpression("call", "map-keys requires 1 argument", "")
    m = args[0]
    if not isinstance(m, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m)}", "")
    return LatticeList(tuple(LatticeIdent(k) for k in m.keys()))


def _builtin_map_values(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """map-values($map) — return all values as a list."""
    if not args:
        raise TypeErrorInExpression("call", "map-values requires 1 argument", "")
    m = args[0]
    if not isinstance(m, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m)}", "")
    return LatticeList(tuple(m.values()))


def _builtin_map_has_key(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """map-has-key($map, $key) — return true if key exists."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "map-has-key requires 2 arguments", "")
    m = args[0]
    if not isinstance(m, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m)}", "")
    key = str(args[1]).strip('"')
    return LatticeBool(m.has_key(key))


def _builtin_map_merge(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """map-merge($map1, $map2) — merge two maps, second wins on conflicts."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "map-merge requires 2 arguments", "")
    m1, m2 = args[0], args[1]
    if not isinstance(m1, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m1)}", "")
    if not isinstance(m2, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m2)}", "")
    # Build merged dict preserving insertion order, m2 overwrites m1
    merged: dict[str, LatticeValue] = dict(m1.items)
    for k, v in m2.items:
        merged[k] = v
    return LatticeMap(tuple(merged.items()))


def _builtin_map_remove(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """map-remove($map, $keys...) — return new map without the specified keys."""
    if not args:
        raise TypeErrorInExpression("call", "map-remove requires at least 1 argument", "")
    m = args[0]
    if not isinstance(m, LatticeMap):
        raise TypeErrorInExpression("use", f"Expected a map, got {_type_name_of(m)}", "")
    keys_to_remove = {str(a).strip('"') for a in args[1:]}
    new_items = tuple((k, v) for k, v in m.items if k not in keys_to_remove)
    return LatticeMap(new_items)


# ---------------------------------------------------------------------------
# Color Functions
# ---------------------------------------------------------------------------


def _ensure_color(v: LatticeValue) -> LatticeColor:
    """Validate that a value is a color and return it."""
    if not isinstance(v, LatticeColor):
        raise TypeErrorInExpression("use", f"Expected a color, got {_type_name_of(v)}", "")
    return v


def _ensure_amount(v: LatticeValue) -> float:
    """Extract a percentage amount (0-100) from a value."""
    from lattice_ast_to_css.errors import RangeError
    val = _get_numeric_value(v)
    if val < 0 or val > 100:
        raise RangeError("Amount must be between 0% and 100%")
    return val


def _builtin_lighten(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """lighten($color, $amount) — increase lightness by $amount (0-100%)."""
    color = _ensure_color(args[0])
    amount = _ensure_amount(args[1])
    h, s, l, a = color.to_hsl()
    l = min(100.0, l + amount)
    return LatticeColor.from_hsl(h, s, l, a)


def _builtin_darken(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """darken($color, $amount) — decrease lightness by $amount (0-100%)."""
    color = _ensure_color(args[0])
    amount = _ensure_amount(args[1])
    h, s, l, a = color.to_hsl()
    l = max(0.0, l - amount)
    return LatticeColor.from_hsl(h, s, l, a)


def _builtin_saturate_fn(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """saturate($color, $amount) — increase saturation by $amount (0-100%)."""
    color = _ensure_color(args[0])
    amount = _ensure_amount(args[1])
    h, s, l, a = color.to_hsl()
    s = min(100.0, s + amount)
    return LatticeColor.from_hsl(h, s, l, a)


def _builtin_desaturate(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """desaturate($color, $amount) — decrease saturation by $amount (0-100%)."""
    color = _ensure_color(args[0])
    amount = _ensure_amount(args[1])
    h, s, l, a = color.to_hsl()
    s = max(0.0, s - amount)
    return LatticeColor.from_hsl(h, s, l, a)


def _builtin_adjust_hue(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """adjust-hue($color, $degrees) — rotate hue by $degrees."""
    color = _ensure_color(args[0])
    degrees = _get_numeric_value(args[1])
    h, s, l, a = color.to_hsl()
    h = (h + degrees) % 360.0
    return LatticeColor.from_hsl(h, s, l, a)


def _builtin_complement(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """complement($color) — rotate hue by 180 degrees."""
    color = _ensure_color(args[0])
    h, s, l, a = color.to_hsl()
    h = (h + 180.0) % 360.0
    return LatticeColor.from_hsl(h, s, l, a)


def _builtin_mix(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """mix($color1, $color2, $weight: 50%) — blend two colors.

    The $weight parameter (0-100%) controls the proportion of $color1.
    50% produces an even blend. 100% returns $color1, 0% returns $color2.
    """
    c1 = _ensure_color(args[0])
    c2 = _ensure_color(args[1])
    weight = 50.0
    if len(args) >= 3:
        weight = _get_numeric_value(args[2])
    w = weight / 100.0
    r1, g1, b1, a1 = c1.to_rgb()
    r2, g2, b2, a2 = c2.to_rgb()
    r = round(r1 * w + r2 * (1 - w))
    g = round(g1 * w + g2 * (1 - w))
    b = round(b1 * w + b2 * (1 - w))
    a = a1 * w + a2 * (1 - w)
    return LatticeColor.from_rgb(r, g, b, a)


def _builtin_rgba(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """rgba($color, $alpha) or rgba($r, $g, $b, $a) — set alpha or construct color."""
    if len(args) == 2 and isinstance(args[0], LatticeColor):
        color = args[0]
        alpha = _get_numeric_value(args[1])
        r, g, b, _ = color.to_rgb()
        return LatticeColor.from_rgb(r, g, b, alpha)
    if len(args) == 4:
        r = round(_get_numeric_value(args[0]))
        g = round(_get_numeric_value(args[1]))
        b = round(_get_numeric_value(args[2]))
        a = _get_numeric_value(args[3])
        return LatticeColor.from_rgb(r, g, b, a)
    # Passthrough — let CSS handle it
    return LatticeNull()


def _builtin_red(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """red($color) — extract red channel (0-255)."""
    color = _ensure_color(args[0])
    r, _, _, _ = color.to_rgb()
    return LatticeNumber(float(r))


def _builtin_green(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """green($color) — extract green channel (0-255)."""
    color = _ensure_color(args[0])
    _, g, _, _ = color.to_rgb()
    return LatticeNumber(float(g))


def _builtin_blue(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """blue($color) — extract blue channel (0-255)."""
    color = _ensure_color(args[0])
    _, _, b, _ = color.to_rgb()
    return LatticeNumber(float(b))


def _builtin_hue(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """hue($color) — extract hue component (0-360deg)."""
    color = _ensure_color(args[0])
    h, _, _, _ = color.to_hsl()
    return LatticeDimension(round(h), "deg")


def _builtin_saturation(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """saturation($color) — extract saturation (0-100%)."""
    color = _ensure_color(args[0])
    _, s, _, _ = color.to_hsl()
    return LatticePercentage(round(s))


def _builtin_lightness(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """lightness($color) — extract lightness (0-100%)."""
    color = _ensure_color(args[0])
    _, _, l, _ = color.to_hsl()
    return LatticePercentage(round(l))


# ---------------------------------------------------------------------------
# List Functions
# ---------------------------------------------------------------------------


def _builtin_nth(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """nth($list, $n) — get the nth item (1-indexed)."""
    from lattice_ast_to_css.errors import RangeError
    if len(args) < 2:
        raise TypeErrorInExpression("call", "nth requires 2 arguments", "")
    lst = args[0]
    n = int(_get_numeric_value(args[1]))
    if n < 1:
        raise RangeError("List index must be 1 or greater")
    if isinstance(lst, LatticeList):
        if n > len(lst.items):
            raise RangeError(f"Index {n} out of bounds for list of length {len(lst.items)}")
        return lst.items[n - 1]
    # Single value treated as list of 1
    if n == 1:
        return lst
    raise RangeError(f"Index {n} out of bounds for list of length 1")


def _builtin_length(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """length($list) — number of items in a list or map."""
    if not args:
        raise TypeErrorInExpression("call", "length requires 1 argument", "")
    v = args[0]
    if isinstance(v, LatticeList):
        return LatticeNumber(float(len(v.items)))
    if isinstance(v, LatticeMap):
        return LatticeNumber(float(len(v.items)))
    # Single value has length 1
    return LatticeNumber(1.0)


def _builtin_join(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """join($list1, $list2, $separator: auto) — concatenate two lists."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "join requires at least 2 arguments", "")
    l1 = args[0]
    l2 = args[1]
    items1 = l1.items if isinstance(l1, LatticeList) else (l1,)
    items2 = l2.items if isinstance(l2, LatticeList) else (l2,)
    return LatticeList(items1 + items2)


def _builtin_append(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """append($list, $val, $separator: auto) — add a value to end of list."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "append requires at least 2 arguments", "")
    lst = args[0]
    val = args[1]
    items = lst.items if isinstance(lst, LatticeList) else (lst,)
    return LatticeList(items + (val,))


def _builtin_index(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """index($list, $value) — find position of value (1-indexed), or null."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "index requires 2 arguments", "")
    lst = args[0]
    target = args[1]
    items = lst.items if isinstance(lst, LatticeList) else (lst,)
    target_str = str(target)
    for i, item in enumerate(items):
        if str(item) == target_str:
            return LatticeNumber(float(i + 1))
    return LatticeNull()


# ---------------------------------------------------------------------------
# Type Introspection Functions
# ---------------------------------------------------------------------------


def _builtin_type_of(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """type-of($value) — returns the type as a string."""
    if not args:
        raise TypeErrorInExpression("call", "type-of requires 1 argument", "")
    return LatticeString(_type_name_of(args[0]))


def _builtin_unit(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """unit($number) — returns the unit as a string."""
    if not args:
        raise TypeErrorInExpression("call", "unit requires 1 argument", "")
    v = args[0]
    if isinstance(v, LatticeDimension):
        return LatticeString(v.unit)
    if isinstance(v, LatticePercentage):
        return LatticeString("%")
    if isinstance(v, LatticeNumber):
        return LatticeString("")
    raise TypeErrorInExpression("use", f"Expected a number, got {_type_name_of(v)}", "")


def _builtin_unitless(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """unitless($number) — returns true if the number has no unit."""
    if not args:
        raise TypeErrorInExpression("call", "unitless requires 1 argument", "")
    v = args[0]
    return LatticeBool(isinstance(v, LatticeNumber))


def _builtin_comparable(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """comparable($n1, $n2) — returns true if two numbers can be compared."""
    if len(args) < 2:
        raise TypeErrorInExpression("call", "comparable requires 2 arguments", "")
    a, b = args[0], args[1]
    # Same type → comparable
    if type(a) == type(b):
        if isinstance(a, LatticeDimension) and isinstance(b, LatticeDimension):
            return LatticeBool(a.unit == b.unit)
        return LatticeBool(True)
    # Number and any numeric → comparable
    if isinstance(a, (LatticeNumber, LatticeDimension, LatticePercentage)) and \
       isinstance(b, (LatticeNumber, LatticeDimension, LatticePercentage)):
        if isinstance(a, LatticeNumber) or isinstance(b, LatticeNumber):
            return LatticeBool(True)
        return LatticeBool(False)
    return LatticeBool(False)


# ---------------------------------------------------------------------------
# Math Functions
# ---------------------------------------------------------------------------


def _builtin_math_div(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.div($a, $b) — division.

    Division rules mirror multiplication but in reverse:

    - Number / Number → Number
    - Dimension / Number → Dimension (scales the value down)
    - Percentage / Number → Percentage
    - Dimension / Dimension (same unit) → Number (units cancel)
    """
    from lattice_ast_to_css.errors import ZeroDivisionInExpressionError
    if len(args) < 2:
        raise TypeErrorInExpression("call", "math.div requires 2 arguments", "")
    a, b = args[0], args[1]
    b_val = _get_numeric_value(b)
    if b_val == 0:
        raise ZeroDivisionInExpressionError()
    a_val = _get_numeric_value(a)

    if isinstance(a, LatticeDimension) and isinstance(b, LatticeNumber):
        return LatticeDimension(a_val / b_val, a.unit)
    if isinstance(a, LatticeDimension) and isinstance(b, LatticeDimension):
        if a.unit == b.unit:
            return LatticeNumber(a_val / b_val)
    if isinstance(a, LatticePercentage) and isinstance(b, LatticeNumber):
        return LatticePercentage(a_val / b_val)
    return LatticeNumber(a_val / b_val)


def _builtin_math_floor(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.floor($n) — round down to nearest integer."""
    if not args:
        raise TypeErrorInExpression("call", "math.floor requires 1 argument", "")
    v = args[0]
    val = _get_numeric_value(v)
    result = _math.floor(val)
    if isinstance(v, LatticeDimension):
        return LatticeDimension(float(result), v.unit)
    if isinstance(v, LatticePercentage):
        return LatticePercentage(float(result))
    return LatticeNumber(float(result))


def _builtin_math_ceil(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.ceil($n) — round up to nearest integer."""
    if not args:
        raise TypeErrorInExpression("call", "math.ceil requires 1 argument", "")
    v = args[0]
    val = _get_numeric_value(v)
    result = _math.ceil(val)
    if isinstance(v, LatticeDimension):
        return LatticeDimension(float(result), v.unit)
    if isinstance(v, LatticePercentage):
        return LatticePercentage(float(result))
    return LatticeNumber(float(result))


def _builtin_math_round(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.round($n) — round to nearest integer."""
    if not args:
        raise TypeErrorInExpression("call", "math.round requires 1 argument", "")
    v = args[0]
    val = _get_numeric_value(v)
    result = round(val)
    if isinstance(v, LatticeDimension):
        return LatticeDimension(float(result), v.unit)
    if isinstance(v, LatticePercentage):
        return LatticePercentage(float(result))
    return LatticeNumber(float(result))


def _builtin_math_abs(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.abs($n) — absolute value."""
    if not args:
        raise TypeErrorInExpression("call", "math.abs requires 1 argument", "")
    v = args[0]
    val = _get_numeric_value(v)
    result = abs(val)
    if isinstance(v, LatticeDimension):
        return LatticeDimension(result, v.unit)
    if isinstance(v, LatticePercentage):
        return LatticePercentage(result)
    return LatticeNumber(result)


def _builtin_math_min(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.min($numbers...) — minimum of all arguments."""
    if not args:
        raise TypeErrorInExpression("call", "math.min requires at least 1 argument", "")
    best = args[0]
    best_val = _get_numeric_value(best)
    for arg in args[1:]:
        val = _get_numeric_value(arg)
        if val < best_val:
            best = arg
            best_val = val
    return best


def _builtin_math_max(args: list[LatticeValue], scope: ScopeChain) -> LatticeValue:
    """math.max($numbers...) — maximum of all arguments."""
    if not args:
        raise TypeErrorInExpression("call", "math.max requires at least 1 argument", "")
    best = args[0]
    best_val = _get_numeric_value(best)
    for arg in args[1:]:
        val = _get_numeric_value(arg)
        if val > best_val:
            best = arg
            best_val = val
    return best


# ---------------------------------------------------------------------------
# Function Registry
# ---------------------------------------------------------------------------
#
# All built-in functions in a single dictionary. The transformer checks this
# registry before looking up user-defined functions. User-defined functions
# shadow built-ins with the same name (matching Sass behavior).
# ---------------------------------------------------------------------------

BUILTIN_FUNCTIONS: dict[str, Any] = {
    # Map functions
    "map-get": _builtin_map_get,
    "map-keys": _builtin_map_keys,
    "map-values": _builtin_map_values,
    "map-has-key": _builtin_map_has_key,
    "map-merge": _builtin_map_merge,
    "map-remove": _builtin_map_remove,
    # Color functions
    "lighten": _builtin_lighten,
    "darken": _builtin_darken,
    "saturate": _builtin_saturate_fn,
    "desaturate": _builtin_desaturate,
    "adjust-hue": _builtin_adjust_hue,
    "complement": _builtin_complement,
    "mix": _builtin_mix,
    "rgba": _builtin_rgba,
    "red": _builtin_red,
    "green": _builtin_green,
    "blue": _builtin_blue,
    "hue": _builtin_hue,
    "saturation": _builtin_saturation,
    "lightness": _builtin_lightness,
    # List functions
    "nth": _builtin_nth,
    "length": _builtin_length,
    "join": _builtin_join,
    "append": _builtin_append,
    "index": _builtin_index,
    # Type functions
    "type-of": _builtin_type_of,
    "unit": _builtin_unit,
    "unitless": _builtin_unitless,
    "comparable": _builtin_comparable,
    # Math functions
    "math.div": _builtin_math_div,
    "math.floor": _builtin_math_floor,
    "math.ceil": _builtin_math_ceil,
    "math.round": _builtin_math_round,
    "math.abs": _builtin_math_abs,
    "math.min": _builtin_math_min,
    "math.max": _builtin_math_max,
}
