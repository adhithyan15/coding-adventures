"""Gate and arithmetic primitives — imported from the logic-gates and arithmetic packages.

This module re-exports the gates and adders that fp-arithmetic uses, sourced
from the actual logic-gates and arithmetic packages lower in the stack.

The dependency chain is:
    Logic Gates (AND, OR, NOT, XOR)
        └── Arithmetic (half_adder, full_adder, ripple_carry_adder)
            └── FP Arithmetic (this package)

Every floating-point operation ultimately reduces to these gate-level
primitives — the same gates that would be etched in silicon.
"""

from __future__ import annotations

# Re-export gates from the logic-gates package
from logic_gates import AND, NOT, OR, XOR

# Re-export adders from the arithmetic package
from arithmetic.adders import full_adder, ripple_carry_adder

__all__ = ["AND", "OR", "NOT", "XOR", "full_adder", "ripple_carry_adder"]
