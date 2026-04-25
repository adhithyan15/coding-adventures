"""HIR expression nodes.

Expressions form the right-hand side of assignments, conditions in if/case,
loop bounds, parameter values, and so on. Every expression has a type
(checked at elaboration time, not enforced here at the data-structure level
because the elaborator may not yet have resolved all references).

JSON encoding uses a ``kind`` discriminator on every expression object,
mirroring the type system in `types.py`.
"""

from __future__ import annotations

from dataclasses import dataclass

from hdl_ir.provenance import Provenance
from hdl_ir.types import Ty, ty_from_dict

# ----------------------------------------------------------------------------
# Helpers for provenance JSON
# ----------------------------------------------------------------------------


def _prov_to_dict(p: Provenance | None) -> dict[str, object] | None:
    return p.to_dict() if p is not None else None


def _prov_from_dict(d: object) -> Provenance | None:
    if isinstance(d, dict):
        return Provenance.from_dict(d)
    return None


# ----------------------------------------------------------------------------
# Atomic expressions
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Lit:
    """Literal: integer, bool, float, or vector tuple. Type is explicit."""

    value: int | bool | float | str | tuple[int, ...]
    type: Ty
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "lit",
            "value": list(self.value) if isinstance(self.value, tuple) else self.value,
            "type": self.type.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class NetRef:
    """Reference to a Net in the enclosing scope."""

    name: str
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"kind": "net_ref", "name": self.name}
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class VarRef:
    """Reference to a process-local Variable."""

    name: str
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"kind": "var_ref", "name": self.name}
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class PortRef:
    """Reference to a Port of the enclosing Module."""

    name: str
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"kind": "port_ref", "name": self.name}
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Composite expressions
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Slice:
    """Bit / range select. ``msb`` and ``lsb`` are inclusive indices into the
    target's bit positions."""

    base: Expr
    msb: int
    lsb: int
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "slice",
            "base": self.base.to_dict(),  # type: ignore[union-attr]
            "msb": self.msb,
            "lsb": self.lsb,
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class Concat:
    """``{a, b, c}`` — concatenation in MSB-first order."""

    parts: tuple[Expr, ...]
    provenance: Provenance | None = None

    def __post_init__(self) -> None:
        if not self.parts:
            raise ValueError("Concat must have >= 1 part")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "concat",
            "parts": [p.to_dict() for p in self.parts],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class Replication:
    """``{N{x}}`` — replicate ``body`` ``count`` times."""

    count: Expr
    body: Expr
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "replication",
            "count": self.count.to_dict(),  # type: ignore[union-attr]
            "body": self.body.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Operators
# ----------------------------------------------------------------------------


# Recognized unary operators.
UNARY_OPS = frozenset(
    {
        "NOT",  # ~ / not
        "NEG",  # unary -
        "POS",  # unary +
        "AND_RED",  # &x reduction AND
        "OR_RED",  # |x reduction OR
        "XOR_RED",  # ^x reduction XOR
        "NAND_RED",
        "NOR_RED",
        "XNOR_RED",
        "LOGIC_NOT",  # !x
    }
)


# Recognized binary operators.
BINARY_OPS = frozenset(
    {
        "+",
        "-",
        "*",
        "/",
        "%",
        "**",
        "AND",
        "OR",
        "XOR",
        "NAND",
        "NOR",
        "XNOR",
        "<<",
        ">>",
        "<<<",
        ">>>",
        "<",
        "<=",
        ">",
        ">=",
        "==",
        "!=",
        "===",
        "!==",
        "&&",
        "||",
        "&",
        "|",
        "^",
    }
)


@dataclass(frozen=True, slots=True)
class UnaryOp:
    """Unary operation. ``op`` must be one of UNARY_OPS."""

    op: str
    operand: Expr
    provenance: Provenance | None = None

    def __post_init__(self) -> None:
        if self.op not in UNARY_OPS:
            raise ValueError(f"unknown unary op: {self.op!r}")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "unary",
            "op": self.op,
            "operand": self.operand.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class BinaryOp:
    """Binary operation. ``op`` must be one of BINARY_OPS."""

    op: str
    lhs: Expr
    rhs: Expr
    provenance: Provenance | None = None

    def __post_init__(self) -> None:
        if self.op not in BINARY_OPS:
            raise ValueError(f"unknown binary op: {self.op!r}")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "binary",
            "op": self.op,
            "lhs": self.lhs.to_dict(),  # type: ignore[union-attr]
            "rhs": self.rhs.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class Ternary:
    """``cond ? then_expr : else_expr``."""

    cond: Expr
    then_expr: Expr
    else_expr: Expr
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "ternary",
            "cond": self.cond.to_dict(),  # type: ignore[union-attr]
            "then_expr": self.then_expr.to_dict(),  # type: ignore[union-attr]
            "else_expr": self.else_expr.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Calls
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class FunCall:
    """Function call. The function must be defined in the enclosing scope."""

    name: str
    args: tuple[Expr, ...] = ()
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "fun_call",
            "name": self.name,
            "args": [a.to_dict() for a in self.args],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class SystemCall:
    """``$display``, ``$time``, ``$random``, etc.

    The leading ``$`` is included in ``name`` for fidelity to the source."""

    name: str
    args: tuple[Expr, ...] = ()
    provenance: Provenance | None = None

    def __post_init__(self) -> None:
        if not self.name.startswith("$"):
            raise ValueError(f"system call name must start with '$', got {self.name!r}")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "system_call",
            "name": self.name,
            "args": [a.to_dict() for a in self.args],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class Attribute:
    """Predefined or user attribute access: ``signal'event``, ``arr'length``."""

    base: Expr
    name: str
    args: tuple[Expr, ...] = ()
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "attr",
            "base": self.base.to_dict(),  # type: ignore[union-attr]
            "name": self.name,
            "args": [a.to_dict() for a in self.args],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Union and JSON deserialization
# ----------------------------------------------------------------------------


Expr = (
    Lit
    | NetRef
    | VarRef
    | PortRef
    | Slice
    | Concat
    | Replication
    | UnaryOp
    | BinaryOp
    | Ternary
    | FunCall
    | SystemCall
    | Attribute
)


def expr_from_dict(d: dict[str, object]) -> Expr:
    """Reconstruct an Expr from its JSON form."""
    kind = d["kind"]
    prov = _prov_from_dict(d.get("provenance"))

    if kind == "lit":
        v = d["value"]
        value: int | bool | float | str | tuple[int, ...] = (
            tuple(int(x) for x in v) if isinstance(v, list) else v  # type: ignore[assignment]
        )
        return Lit(
            value=value,
            type=ty_from_dict(d["type"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "net_ref":
        return NetRef(name=str(d["name"]), provenance=prov)
    if kind == "var_ref":
        return VarRef(name=str(d["name"]), provenance=prov)
    if kind == "port_ref":
        return PortRef(name=str(d["name"]), provenance=prov)
    if kind == "slice":
        return Slice(
            base=expr_from_dict(d["base"]),  # type: ignore[arg-type]
            msb=int(d["msb"]),  # type: ignore[arg-type]
            lsb=int(d["lsb"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "concat":
        return Concat(
            parts=tuple(expr_from_dict(p) for p in d["parts"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "replication":
        return Replication(
            count=expr_from_dict(d["count"]),  # type: ignore[arg-type]
            body=expr_from_dict(d["body"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "unary":
        return UnaryOp(
            op=str(d["op"]),
            operand=expr_from_dict(d["operand"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "binary":
        return BinaryOp(
            op=str(d["op"]),
            lhs=expr_from_dict(d["lhs"]),  # type: ignore[arg-type]
            rhs=expr_from_dict(d["rhs"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "ternary":
        return Ternary(
            cond=expr_from_dict(d["cond"]),  # type: ignore[arg-type]
            then_expr=expr_from_dict(d["then_expr"]),  # type: ignore[arg-type]
            else_expr=expr_from_dict(d["else_expr"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "fun_call":
        return FunCall(
            name=str(d["name"]),
            args=tuple(expr_from_dict(a) for a in d.get("args", [])),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "system_call":
        return SystemCall(
            name=str(d["name"]),
            args=tuple(expr_from_dict(a) for a in d.get("args", [])),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "attr":
        return Attribute(
            base=expr_from_dict(d["base"]),  # type: ignore[arg-type]
            name=str(d["name"]),
            args=tuple(expr_from_dict(a) for a in d.get("args", [])),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    raise ValueError(f"unknown expression kind: {kind!r}")
