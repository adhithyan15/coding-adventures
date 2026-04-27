"""HIR type system.

HIR types span both synthesis-relevant (logic, vector, integer) and
simulation-only (real, time, string, file) territory because the IR represents
the full IEEE 1076-2008 / 1364-2005 surface.

Synthesis projects all types to bit vectors via `width()` before producing HNL.
Simulation reads the original type to choose value-resolution and arithmetic
semantics. The type model is therefore richer than HNL's bit-only model.

JSON encoding uses a `kind` discriminator on every type object, so the schema
is self-describing for downstream consumers in any language (per the keystone
spec's polyglot-port requirement).
"""

from __future__ import annotations

from dataclasses import dataclass

# ----------------------------------------------------------------------------
# Scalar types
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class TyLogic:
    """4-state value: 0, 1, X (unknown), Z (high-impedance).
    Verilog default. Matches IEEE 1364."""

    def to_dict(self) -> dict[str, str]:
        return {"kind": "logic"}


@dataclass(frozen=True, slots=True)
class TyBit:
    """2-state value: 0 or 1. VHDL `bit`."""

    def to_dict(self) -> dict[str, str]:
        return {"kind": "bit"}


@dataclass(frozen=True, slots=True)
class TyStdLogic:
    """9-state value: U, X, 0, 1, Z, W, L, H, -. IEEE 1164."""

    def to_dict(self) -> dict[str, str]:
        return {"kind": "std_logic"}


@dataclass(frozen=True, slots=True)
class TyReal:
    """IEEE 754 double-precision floating point."""

    def to_dict(self) -> dict[str, str]:
        return {"kind": "real"}


@dataclass(frozen=True, slots=True)
class TyTime:
    """Simulated time. Stored as picoseconds (or whatever the timescale is)."""

    def to_dict(self) -> dict[str, str]:
        return {"kind": "time"}


@dataclass(frozen=True, slots=True)
class TyString:
    """Variable-length character string."""

    def to_dict(self) -> dict[str, str]:
        return {"kind": "string"}


# ----------------------------------------------------------------------------
# Composite types
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class TyVector:
    """N-element array of T. ``msb_first`` distinguishes Verilog ``[N-1:0]``
    (True) from ``[0:N-1]`` (False)."""

    elem: Ty
    width: int
    msb_first: bool = True

    def __post_init__(self) -> None:
        if self.width < 1:
            raise ValueError(f"vector width must be >= 1, got {self.width}")

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "vector",
            "elem": self.elem.to_dict(),  # type: ignore[union-attr]
            "width": self.width,
            "msb_first": self.msb_first,
        }


@dataclass(frozen=True, slots=True)
class TyInteger:
    """Bounded integer type. Default is the canonical 32-bit signed range."""

    low: int = -(2**31)
    high: int = 2**31 - 1

    def __post_init__(self) -> None:
        if self.low > self.high:
            raise ValueError(f"low ({self.low}) > high ({self.high})")

    def to_dict(self) -> dict[str, object]:
        return {"kind": "integer", "low": self.low, "high": self.high}


@dataclass(frozen=True, slots=True)
class TyEnum:
    """User-defined enumeration; e.g. ``type state is (red, green, yellow)``."""

    name: str
    members: tuple[str, ...]

    def __post_init__(self) -> None:
        if not self.members:
            raise ValueError(f"enum {self.name!r} must have >= 1 member")
        if len(set(self.members)) != len(self.members):
            raise ValueError(f"enum {self.name!r} has duplicate members")

    def to_dict(self) -> dict[str, object]:
        return {"kind": "enum", "name": self.name, "members": list(self.members)}


@dataclass(frozen=True, slots=True)
class TyRecord:
    """User-defined record / struct. Fields are ordered."""

    name: str
    fields: tuple[tuple[str, Ty], ...]

    def __post_init__(self) -> None:
        names = [n for n, _ in self.fields]
        if len(set(names)) != len(names):
            raise ValueError(f"record {self.name!r} has duplicate field names")

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "record",
            "name": self.name,
            "fields": [
                {"name": n, "type": t.to_dict()}  # type: ignore[union-attr]
                for n, t in self.fields
            ],
        }


@dataclass(frozen=True, slots=True)
class TyArray:
    """Multi-dimensional array. Range [low..high] inclusive."""

    elem: Ty
    low: int
    high: int

    def __post_init__(self) -> None:
        if self.low > self.high:
            raise ValueError(f"array bounds: low ({self.low}) > high ({self.high})")

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "array",
            "elem": self.elem.to_dict(),  # type: ignore[union-attr]
            "low": self.low,
            "high": self.high,
        }


@dataclass(frozen=True, slots=True)
class TyFile:
    """File handle (VHDL textio, Verilog $fopen)."""

    elem: Ty

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "file",
            "elem": self.elem.to_dict(),  # type: ignore[union-attr]
        }


Ty = (
    TyLogic
    | TyBit
    | TyStdLogic
    | TyReal
    | TyTime
    | TyString
    | TyVector
    | TyInteger
    | TyEnum
    | TyRecord
    | TyArray
    | TyFile
)


# ----------------------------------------------------------------------------
# JSON deserialization
# ----------------------------------------------------------------------------


def ty_from_dict(d: dict[str, object]) -> Ty:
    """Reconstruct a Ty from its JSON form. The ``kind`` field discriminates."""
    kind = d["kind"]
    if kind == "logic":
        return TyLogic()
    if kind == "bit":
        return TyBit()
    if kind == "std_logic":
        return TyStdLogic()
    if kind == "real":
        return TyReal()
    if kind == "time":
        return TyTime()
    if kind == "string":
        return TyString()
    if kind == "vector":
        return TyVector(
            elem=ty_from_dict(d["elem"]),  # type: ignore[arg-type]
            width=int(d["width"]),  # type: ignore[arg-type]
            msb_first=bool(d.get("msb_first", True)),
        )
    if kind == "integer":
        return TyInteger(
            low=int(d["low"]),  # type: ignore[arg-type]
            high=int(d["high"]),  # type: ignore[arg-type]
        )
    if kind == "enum":
        return TyEnum(name=str(d["name"]), members=tuple(d["members"]))  # type: ignore[arg-type]
    if kind == "record":
        return TyRecord(
            name=str(d["name"]),
            fields=tuple(
                (str(f["name"]), ty_from_dict(f["type"]))  # type: ignore[arg-type, index]
                for f in d["fields"]  # type: ignore[union-attr]
            ),
        )
    if kind == "array":
        return TyArray(
            elem=ty_from_dict(d["elem"]),  # type: ignore[arg-type]
            low=int(d["low"]),  # type: ignore[arg-type]
            high=int(d["high"]),  # type: ignore[arg-type]
        )
    if kind == "file":
        return TyFile(elem=ty_from_dict(d["elem"]))  # type: ignore[arg-type]
    raise ValueError(f"unknown type kind: {kind!r}")


# ----------------------------------------------------------------------------
# Width projection (synthesis-relevant types only)
# ----------------------------------------------------------------------------


def width(t: Ty) -> int:
    """Bit-width of a type after projection to bits.

    Synthesizable types only. Calling on TyReal / TyTime / TyString / TyFile
    raises ValueError because those don't have a deterministic bit projection
    (synthesis would reject these constructs).
    """
    if isinstance(t, (TyLogic, TyBit, TyStdLogic)):
        return 1
    if isinstance(t, TyVector):
        return t.width * width(t.elem)
    if isinstance(t, TyInteger):
        span = t.high - t.low + 1
        return max(1, (span - 1).bit_length()) if span > 1 else 1
    if isinstance(t, TyEnum):
        n = len(t.members)
        return max(1, (n - 1).bit_length()) if n > 1 else 1
    if isinstance(t, TyRecord):
        return sum(width(ft) for _, ft in t.fields)
    if isinstance(t, TyArray):
        return (t.high - t.low + 1) * width(t.elem)
    raise ValueError(f"width() not defined for {t!r} (unsynthesizable type)")
