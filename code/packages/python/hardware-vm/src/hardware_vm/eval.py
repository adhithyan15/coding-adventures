"""Expression evaluator.

Walks an HIR `Expr` tree and computes its value given a signal-state lookup.

Values are tuples (val, width) where val is an int (Python's arbitrary-precision
integer is fine for v0.1.0) and width is the bit-width.

For 4-state semantics, X and Z are represented as None values; arithmetic on
None propagates None. v0.1.0 uses simple 2-state ints; 4-state is documented as
v0.2.0 work.
"""

from __future__ import annotations

from collections.abc import Callable

from hdl_ir import (
    Attribute,
    BinaryOp,
    Concat,
    Expr,
    FunCall,
    Lit,
    NetRef,
    PortRef,
    Replication,
    Slice,
    SystemCall,
    Ternary,
    UnaryOp,
    VarRef,
)

ValueLookup = Callable[[str], int]


def evaluate(expr: Expr, lookup: ValueLookup) -> int:
    """Evaluate an HIR expression to an integer value.

    `lookup(name)` returns the current integer value of a Net or Port.
    Variables and complex expressions (FunCall, SystemCall, Attribute) are
    evaluated structurally; unknown forms return 0 with a warning trace.
    """
    if isinstance(expr, Lit):
        v = expr.value
        if isinstance(v, bool):
            return int(v)
        if isinstance(v, int):
            return v
        if isinstance(v, tuple):
            # Vector literal: pack MSB-first.
            result = 0
            for bit in v:
                result = (result << 1) | (int(bit) & 1)
            return result
        if isinstance(v, str):
            # Numeric strings interpreted as 0
            try:
                return int(v, 0)
            except ValueError:
                return 0
        return 0

    if isinstance(expr, (NetRef, PortRef, VarRef)):
        return lookup(expr.name)

    if isinstance(expr, Slice):
        base_val = evaluate(expr.base, lookup)
        msb, lsb = expr.msb, expr.lsb
        if msb < lsb:
            msb, lsb = lsb, msb
        width = msb - lsb + 1
        mask = (1 << width) - 1
        return (base_val >> lsb) & mask

    if isinstance(expr, Concat):
        # Pack parts MSB-first; each part contributes its own bit width
        result = 0
        for part in expr.parts:
            part_val = evaluate(part, lookup)
            part_width = _expr_width(part, lookup) or 1
            result = (result << part_width) | (part_val & ((1 << part_width) - 1))
        return result

    if isinstance(expr, Replication):
        count = evaluate(expr.count, lookup)
        body_val = evaluate(expr.body, lookup)
        body_width = _expr_width(expr.body, lookup) or 1
        result = 0
        body_mask = (1 << body_width) - 1
        for _ in range(count):
            result = (result << body_width) | (body_val & body_mask)
        return result

    if isinstance(expr, UnaryOp):
        operand = evaluate(expr.operand, lookup)
        return _apply_unary(expr.op, operand)

    if isinstance(expr, BinaryOp):
        lhs = evaluate(expr.lhs, lookup)
        rhs = evaluate(expr.rhs, lookup)
        return _apply_binary(expr.op, lhs, rhs)

    if isinstance(expr, Ternary):
        cond = evaluate(expr.cond, lookup)
        if cond:
            return evaluate(expr.then_expr, lookup)
        return evaluate(expr.else_expr, lookup)

    if isinstance(expr, FunCall):
        # User-defined functions not supported in v0.1.0
        return 0

    if isinstance(expr, SystemCall):
        # System calls have side effects (e.g., $display); return value is 0
        return 0

    if isinstance(expr, Attribute):
        # Attributes ($event, etc.) require runtime state; return 0 for v0.1.0
        return 0

    return 0


def _apply_unary(op: str, operand: int) -> int:
    """Evaluate a unary op."""
    if op == "NEG":
        return -operand
    if op == "POS":
        return +operand
    if op == "LOGIC_NOT":
        return int(not operand)
    if op == "NOT":
        # Bitwise NOT: invert assuming 32-bit canonical width.
        return ~operand & 0xFFFF_FFFF
    if op in ("AND_RED", "OR_RED", "XOR_RED"):
        # Reduction of all bits in operand.
        if op == "AND_RED":
            # All bits set?
            return int(operand != 0 and (operand & (operand + 1)) == 0)
        if op == "OR_RED":
            return int(operand != 0)
        if op == "XOR_RED":
            x = operand
            r = 0
            while x:
                r ^= x & 1
                x >>= 1
            return r
    if op == "NAND_RED":
        return int(not (operand != 0 and (operand & (operand + 1)) == 0))
    if op == "NOR_RED":
        return int(operand == 0)
    if op == "XNOR_RED":
        return 1 - _apply_unary("XOR_RED", operand)
    raise ValueError(f"unknown unary op: {op}")


def _apply_binary(op: str, lhs: int, rhs: int) -> int:
    """Evaluate a binary op."""
    if op == "+":
        return lhs + rhs
    if op == "-":
        return lhs - rhs
    if op == "*":
        return lhs * rhs
    if op == "/":
        if rhs == 0:
            return 0
        return lhs // rhs
    if op == "%":
        if rhs == 0:
            return 0
        return lhs % rhs
    if op == "**":
        return lhs**rhs
    if op == "AND" or op == "&":
        return lhs & rhs
    if op == "OR" or op == "|":
        return lhs | rhs
    if op == "XOR" or op == "^":
        return lhs ^ rhs
    if op == "NAND":
        return ~(lhs & rhs) & 0xFFFF_FFFF
    if op == "NOR":
        return ~(lhs | rhs) & 0xFFFF_FFFF
    if op == "XNOR":
        return ~(lhs ^ rhs) & 0xFFFF_FFFF
    if op == "<<":
        return lhs << rhs
    if op == ">>":
        return lhs >> rhs
    if op == "<<<":
        return lhs << rhs
    if op == ">>>":
        return lhs >> rhs  # arithmetic-right would need width-aware sign-extension
    if op == "==":
        return int(lhs == rhs)
    if op == "!=":
        return int(lhs != rhs)
    if op == "===":
        return int(lhs == rhs)
    if op == "!==":
        return int(lhs != rhs)
    if op == "<":
        return int(lhs < rhs)
    if op == "<=":
        return int(lhs <= rhs)
    if op == ">":
        return int(lhs > rhs)
    if op == ">=":
        return int(lhs >= rhs)
    if op == "&&":
        return int(bool(lhs) and bool(rhs))
    if op == "||":
        return int(bool(lhs) or bool(rhs))
    raise ValueError(f"unknown binary op: {op}")


def _expr_width(expr: Expr, lookup: ValueLookup) -> int:
    """Estimate the bit-width of an expression (used for concat/replication packing)."""
    if isinstance(expr, Lit):
        from hdl_ir.types import TyVector

        if isinstance(expr.type, TyVector):
            return expr.type.width
        return 1
    if isinstance(expr, Slice):
        return abs(expr.msb - expr.lsb) + 1
    if isinstance(expr, Concat):
        return sum(_expr_width(p, lookup) or 1 for p in expr.parts)
    if isinstance(expr, Replication):
        try:
            count = evaluate(expr.count, lookup)
            return count * (_expr_width(expr.body, lookup) or 1)
        except Exception:
            return 1
    return 1


def referenced_signals(expr: Expr) -> set[str]:
    """Return all Net/Port/Var names referenced in this expression — used for
    sensitivity inference on continuous assignments."""
    if isinstance(expr, (NetRef, PortRef, VarRef)):
        return {expr.name}
    if isinstance(expr, Lit):
        return set()
    if isinstance(expr, Slice):
        return referenced_signals(expr.base)
    if isinstance(expr, Concat):
        out: set[str] = set()
        for p in expr.parts:
            out |= referenced_signals(p)
        return out
    if isinstance(expr, Replication):
        return referenced_signals(expr.count) | referenced_signals(expr.body)
    if isinstance(expr, UnaryOp):
        return referenced_signals(expr.operand)
    if isinstance(expr, BinaryOp):
        return referenced_signals(expr.lhs) | referenced_signals(expr.rhs)
    if isinstance(expr, Ternary):
        return (
            referenced_signals(expr.cond)
            | referenced_signals(expr.then_expr)
            | referenced_signals(expr.else_expr)
        )
    if isinstance(expr, (FunCall, SystemCall)):
        out = set()
        for a in expr.args:
            out |= referenced_signals(a)
        return out
    if isinstance(expr, Attribute):
        out = referenced_signals(expr.base)
        for a in expr.args:
            out |= referenced_signals(a)
        return out
    return set()
