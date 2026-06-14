"""Pattern-based trig identity rules.

Each rule is a ``(pattern, replacement)`` pair compatible with
``cas_pattern_matching.Rule``. Rules are tried in the order they appear.
Specific rules come before general ones.

Pythagorean identities
~~~~~~~~~~~~~~~~~~~~~~
  sin²(x) + cos²(x) = 1
  1 - sin²(x) = cos²(x)
  1 - cos²(x) = sin²(x)

Sign / parity rules
~~~~~~~~~~~~~~~~~~~
  sin(-x) = -sin(x)
  cos(-x) = cos(x)
  tan(-x) = -tan(x)

Complementary angle rules
~~~~~~~~~~~~~~~~~~~~~~~~~
  sin(π/2 - x) = cos(x)   (out of scope for Phase 1 — complex pattern)
"""

from __future__ import annotations

from cas_pattern_matching import Blank, Pattern, Rule
from symbolic_ir import (
    ADD,
    MUL,
    NEG,
    POW,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")
TAN = IRSymbol("Tan")
SUB = IRSymbol("Sub")

ONE = IRInteger(1)
TWO = IRInteger(2)

# Pattern variables (x_ matches any single expression)
_x = Pattern("x", Blank())
_y = Pattern("y", Blank())


def _sin_x() -> IRApply:
    return IRApply(SIN, (Pattern("x", Blank()),))


def _cos_x() -> IRApply:
    return IRApply(COS, (Pattern("x", Blank()),))


def _sin2_x() -> IRApply:
    return IRApply(POW, (_sin_x(), TWO))


def _cos2_x() -> IRApply:
    return IRApply(POW, (_cos_x(), TWO))


# ---------------------------------------------------------------------------
# Public: list of Rule objects for use with cas_pattern_matching.rewrite
# ---------------------------------------------------------------------------

# Rule 1: sin²(x) + cos²(x) → 1
PYTHAGOREAN_1 = Rule(
    IRApply(ADD, (_sin2_x(), _cos2_x())),
    ONE,
)

# Rule 2: cos²(x) + sin²(x) → 1  (reordered)
PYTHAGOREAN_1B = Rule(
    IRApply(ADD, (_cos2_x(), _sin2_x())),
    ONE,
)

# Rule 3: sin(-x) → -sin(x)
SIN_NEG = Rule(
    IRApply(SIN, (IRApply(NEG, (Pattern("x", Blank()),)),)),
    IRApply(NEG, (IRApply(SIN, (Pattern("x", Blank()),)),)),
)

# Rule 4: cos(-x) → cos(x)
COS_NEG = Rule(
    IRApply(COS, (IRApply(NEG, (Pattern("x", Blank()),)),)),
    IRApply(COS, (Pattern("x", Blank()),)),
)

# Rule 5: tan(-x) → -tan(x)
TAN_NEG = Rule(
    IRApply(TAN, (IRApply(NEG, (Pattern("x", Blank()),)),)),
    IRApply(NEG, (IRApply(TAN, (Pattern("x", Blank()),)),)),
)


TRIG_RULES: list[Rule] = [
    PYTHAGOREAN_1,
    PYTHAGOREAN_1B,
    SIN_NEG,
    COS_NEG,
    TAN_NEG,
]
