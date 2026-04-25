"""Built-in cell type registry."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class CellTypeSig:
    """Signature of a built-in cell type.

    Pin widths default to 1; override via ``pin_widths`` for any pins that
    are wider (e.g., a hypothetical 8-bit cell)."""

    name: str
    inputs: tuple[str, ...]
    outputs: tuple[str, ...]
    pin_widths: dict[str, int] = field(default_factory=dict)

    def width(self, pin: str) -> int:
        return self.pin_widths.get(pin, 1)

    def has_pin(self, pin: str) -> bool:
        return pin in self.inputs or pin in self.outputs


# Built-in cell type registry. The output of `synthesis.md`'s generic-level
# emission is a netlist where every Instance refers to one of these by name,
# or to a user-defined module elsewhere in the same Netlist.
BUILTIN_CELL_TYPES: dict[str, CellTypeSig] = {
    "BUF":      CellTypeSig("BUF",      ("A",),               ("Y",)),
    "NOT":      CellTypeSig("NOT",      ("A",),               ("Y",)),
    "AND2":     CellTypeSig("AND2",     ("A", "B"),           ("Y",)),
    "AND3":     CellTypeSig("AND3",     ("A", "B", "C"),      ("Y",)),
    "AND4":     CellTypeSig("AND4",     ("A", "B", "C", "D"), ("Y",)),
    "OR2":      CellTypeSig("OR2",      ("A", "B"),           ("Y",)),
    "OR3":      CellTypeSig("OR3",      ("A", "B", "C"),      ("Y",)),
    "OR4":      CellTypeSig("OR4",      ("A", "B", "C", "D"), ("Y",)),
    "NAND2":    CellTypeSig("NAND2",    ("A", "B"),           ("Y",)),
    "NAND3":    CellTypeSig("NAND3",    ("A", "B", "C"),      ("Y",)),
    "NAND4":    CellTypeSig("NAND4",    ("A", "B", "C", "D"), ("Y",)),
    "NOR2":     CellTypeSig("NOR2",     ("A", "B"),           ("Y",)),
    "NOR3":     CellTypeSig("NOR3",     ("A", "B", "C"),      ("Y",)),
    "NOR4":     CellTypeSig("NOR4",     ("A", "B", "C", "D"), ("Y",)),
    "XOR2":     CellTypeSig("XOR2",     ("A", "B"),           ("Y",)),
    "XOR3":     CellTypeSig("XOR3",     ("A", "B", "C"),      ("Y",)),
    "XNOR2":    CellTypeSig("XNOR2",    ("A", "B"),           ("Y",)),
    "XNOR3":    CellTypeSig("XNOR3",    ("A", "B", "C"),      ("Y",)),
    "MUX2":     CellTypeSig("MUX2",     ("A", "B", "S"),      ("Y",)),
    "DFF":      CellTypeSig("DFF",      ("D", "CLK"),         ("Q",)),
    "DFF_R":    CellTypeSig("DFF_R",    ("D", "CLK", "R"),    ("Q",)),
    "DFF_S":    CellTypeSig("DFF_S",    ("D", "CLK", "S"),    ("Q",)),
    "DFF_RS":   CellTypeSig("DFF_RS",   ("D", "CLK", "R", "S"), ("Q",)),
    "DLATCH":   CellTypeSig("DLATCH",   ("D", "EN"),          ("Q",)),
    "TBUF":     CellTypeSig("TBUF",     ("A", "OE"),          ("Y",)),
    "CONST_0":  CellTypeSig("CONST_0",  (),                   ("Y",)),
    "CONST_1":  CellTypeSig("CONST_1",  (),                   ("Y",)),
}
