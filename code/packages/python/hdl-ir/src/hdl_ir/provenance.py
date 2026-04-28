"""Source-language provenance — every HIR node knows where it came from.

Provenance is essential for two reasons:
1. Diagnostics — when synthesis or simulation flags an issue, the user wants
   the file:line:col that produced it, not "somewhere deep in HIR".
2. Re-emission — the back-writer (HIR → Verilog/VHDL) consults provenance to
   pick the target language; without it, ambiguity.

The fields are kept lightweight (a string + three ints) so attaching provenance
to every node has negligible cost. When the source is unknown (e.g., HIR built
by hand for tests), `Provenance` is simply omitted.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class SourceLang(Enum):
    """Which front-end produced this node."""

    VERILOG = "verilog"
    VHDL = "vhdl"
    RUBY_DSL = "ruby_dsl"
    UNKNOWN = "unknown"


@dataclass(frozen=True, slots=True)
class SourceLocation:
    """A point in a source file. Line and column are 1-indexed per
    IEEE / standard convention."""

    file: str
    line: int
    column: int

    def __post_init__(self) -> None:
        if self.line < 1:
            raise ValueError(f"line must be >= 1, got {self.line}")
        if self.column < 1:
            raise ValueError(f"column must be >= 1, got {self.column}")

    def to_dict(self) -> dict[str, str | int]:
        return {"file": self.file, "line": self.line, "column": self.column}

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> SourceLocation:
        return cls(
            file=str(d["file"]),
            line=int(d["line"]),  # type: ignore[arg-type]
            column=int(d["column"]),  # type: ignore[arg-type]
        )


@dataclass(frozen=True, slots=True)
class Provenance:
    """Where an HIR node came from. Both fields optional in JSON
    (omitted when None)."""

    lang: SourceLang
    location: SourceLocation | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"lang": self.lang.value}
        if self.location is not None:
            d["location"] = self.location.to_dict()
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Provenance:
        loc_raw = d.get("location")
        loc = (
            SourceLocation.from_dict(loc_raw)  # type: ignore[arg-type]
            if isinstance(loc_raw, dict)
            else None
        )
        return cls(lang=SourceLang(d["lang"]), location=loc)
