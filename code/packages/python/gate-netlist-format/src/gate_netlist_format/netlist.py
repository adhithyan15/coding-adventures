"""HNL (Hardware NetList) data model + JSON round-trip + validators."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path

from gate_netlist_format.cells import BUILTIN_CELL_TYPES

SCHEMA_VERSION = "0.1.0"


class Direction(Enum):
    INPUT = "input"
    OUTPUT = "output"
    INOUT = "inout"


class Level(Enum):
    GENERIC = "generic"
    STDCELL = "stdcell"
    MIXED = "mixed"


@dataclass(frozen=True, slots=True)
class Port:
    name: str
    direction: Direction
    width: int  # >= 1

    def __post_init__(self) -> None:
        if self.width < 1:
            raise ValueError(f"port {self.name!r}: width must be >= 1, got {self.width}")

    def to_dict(self) -> dict[str, object]:
        return {"name": self.name, "dir": self.direction.value, "width": self.width}

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Port:
        return cls(
            name=str(d["name"]),
            direction=Direction(d["dir"]),
            width=int(d["width"]),  # type: ignore[arg-type]
        )


@dataclass(frozen=True, slots=True)
class Net:
    name: str
    width: int

    def __post_init__(self) -> None:
        if self.width < 1:
            raise ValueError(f"net {self.name!r}: width must be >= 1")

    def to_dict(self) -> dict[str, object]:
        return {"name": self.name, "width": self.width}

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Net:
        return cls(name=str(d["name"]), width=int(d["width"]))  # type: ignore[arg-type]


@dataclass(frozen=True, slots=True)
class NetSlice:
    """A reference to specific bits of a named net."""

    net: str
    bits: tuple[int, ...]

    def width(self) -> int:
        return len(self.bits)

    def to_dict(self) -> dict[str, object]:
        return {"net": self.net, "bits": list(self.bits)}

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> NetSlice:
        return cls(net=str(d["net"]), bits=tuple(int(b) for b in d["bits"]))  # type: ignore[arg-type, union-attr]


@dataclass(frozen=True, slots=True)
class Instance:
    """An instantiation of a cell type or user module within a Module."""

    name: str
    cell_type: str
    connections: dict[str, NetSlice] = field(default_factory=dict)
    parameters: dict[str, int | str] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        return {
            "name": self.name,
            "type": self.cell_type,
            "connections": {k: v.to_dict() for k, v in self.connections.items()},
            "parameters": dict(self.parameters),
        }

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Instance:
        conns_raw = d.get("connections", {})
        return cls(
            name=str(d["name"]),
            cell_type=str(d["type"]),
            connections={
                k: NetSlice.from_dict(v)  # type: ignore[arg-type]
                for k, v in conns_raw.items()  # type: ignore[union-attr]
            },
            parameters=dict(d.get("parameters", {})),  # type: ignore[arg-type]
        )


@dataclass
class Module:
    name: str
    ports: list[Port] = field(default_factory=list)
    nets: list[Net] = field(default_factory=list)
    instances: list[Instance] = field(default_factory=list)

    def port(self, name: str) -> Port | None:
        for p in self.ports:
            if p.name == name:
                return p
        return None

    def net(self, name: str) -> Net | None:
        for n in self.nets:
            if n.name == name:
                return n
        return None

    def to_dict(self) -> dict[str, object]:
        return {
            "ports": [p.to_dict() for p in self.ports],
            "nets": [n.to_dict() for n in self.nets],
            "instances": [i.to_dict() for i in self.instances],
        }

    @classmethod
    def from_dict(cls, name: str, d: dict[str, object]) -> Module:
        return cls(
            name=name,
            ports=[Port.from_dict(p) for p in d.get("ports", [])],  # type: ignore[arg-type]
            nets=[Net.from_dict(n) for n in d.get("nets", [])],  # type: ignore[arg-type]
            instances=[Instance.from_dict(i) for i in d.get("instances", [])],  # type: ignore[arg-type]
        )


@dataclass
class Netlist:
    top: str
    modules: dict[str, Module] = field(default_factory=dict)
    level: Level = Level.GENERIC
    version: str = SCHEMA_VERSION

    def to_dict(self) -> dict[str, object]:
        return {
            "format": "HNL",
            "version": self.version,
            "level": self.level.value,
            "top": self.top,
            "modules": {k: m.to_dict() for k, m in self.modules.items()},
        }

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Netlist:
        if d.get("format") != "HNL":
            raise ValueError(f"not an HNL document (format={d.get('format')!r})")
        version = str(d.get("version", SCHEMA_VERSION))
        major = version.split(".")[0]
        if major != SCHEMA_VERSION.split(".")[0]:
            raise ValueError(f"HNL major version mismatch: {version} vs {SCHEMA_VERSION}")
        mods_raw = d.get("modules", {})
        return cls(
            top=str(d["top"]),
            modules={
                name: Module.from_dict(name, body)  # type: ignore[arg-type]
                for name, body in mods_raw.items()  # type: ignore[union-attr]
            },
            level=Level(d.get("level", "generic")),
            version=version,
        )

    def to_json(self, path: str | Path) -> None:
        Path(path).write_text(json.dumps(self.to_dict(), indent=2))

    @classmethod
    def from_json(cls, path: str | Path) -> Netlist:
        return cls.from_dict(json.loads(Path(path).read_text()))

    def to_json_str(self) -> str:
        return json.dumps(self.to_dict(), indent=2)

    @classmethod
    def from_json_str(cls, s: str) -> Netlist:
        return cls.from_dict(json.loads(s))

    def stats(self) -> NetlistStats:
        cell_counts: dict[str, int] = {}
        total_cells = 0
        total_nets = 0
        for mod in self.modules.values():
            total_nets += len(mod.nets)
            for inst in mod.instances:
                cell_counts[inst.cell_type] = cell_counts.get(inst.cell_type, 0) + 1
                total_cells += 1
        return NetlistStats(
            cell_counts=cell_counts,
            total_cells=total_cells,
            total_nets=total_nets,
        )

    def validate(self) -> ValidationReport:
        return validate_netlist(self)


@dataclass
class NetlistStats:
    cell_counts: dict[str, int]
    total_cells: int
    total_nets: int


# ----------------------------------------------------------------------------
# Validators
# ----------------------------------------------------------------------------


@dataclass
class ValidationReport:
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.errors


def validate_netlist(nl: Netlist) -> ValidationReport:
    """Run the validation rules R1-R13 (subset implemented in v0.1.0)."""
    report = ValidationReport()

    # R1: Top module exists.
    if nl.top not in nl.modules:
        report.errors.append(f"R1: top module {nl.top!r} not in modules")
        return report

    # Per-module checks.
    for mod_name, mod in nl.modules.items():
        _validate_module(mod_name, mod, nl, report)

    # R11: no self-instantiation (transitively).
    for mod_name in nl.modules:
        _check_no_self_inst(mod_name, nl, report)

    return report


def _validate_module(
    mod_name: str, mod: Module, nl: Netlist, report: ValidationReport
) -> None:
    port_names = {p.name for p in mod.ports}
    net_names = {n.name for n in mod.nets}

    # Duplicate names.
    if len(port_names) != len(mod.ports):
        report.errors.append(f"module {mod_name!r}: duplicate port names")
    if len(net_names) != len(mod.nets):
        report.errors.append(f"module {mod_name!r}: duplicate net names")

    for inst in mod.instances:
        # R2: cell type resolves.
        sig = BUILTIN_CELL_TYPES.get(inst.cell_type)
        if sig is None:
            user_mod = nl.modules.get(inst.cell_type)
            if user_mod is None:
                report.errors.append(
                    f"R2: instance {mod_name}.{inst.name}: unknown cell type "
                    f"{inst.cell_type!r}"
                )
                continue
            # User module: build a virtual signature
            user_inputs = tuple(
                p.name for p in user_mod.ports if p.direction == Direction.INPUT
            )
            tuple(
                p.name for p in user_mod.ports if p.direction == Direction.OUTPUT
            )
            user_widths = {p.name: p.width for p in user_mod.ports}
            user_pin_set = port_names_from_module(user_mod)
        else:
            user_pin_set = set(sig.inputs) | set(sig.outputs)
            user_widths = dict(sig.pin_widths)
            user_inputs = sig.inputs

        # R3: every input pin has a connection.
        for in_pin in user_inputs:
            if in_pin not in inst.connections:
                report.errors.append(
                    f"R3: instance {mod_name}.{inst.name}: input pin "
                    f"{in_pin!r} not connected"
                )

        # R4: every connection key is an actual pin.
        for conn_pin in inst.connections:
            if conn_pin not in user_pin_set:
                report.errors.append(
                    f"R4: instance {mod_name}.{inst.name}: pin "
                    f"{conn_pin!r} is not declared on cell {inst.cell_type!r}"
                )

        # R5: width compatibility.
        for conn_pin, conn_slice in inst.connections.items():
            expected_width = user_widths.get(conn_pin, 1)
            if conn_slice.width() != expected_width:
                report.errors.append(
                    f"R5: instance {mod_name}.{inst.name}.{conn_pin}: "
                    f"connection width {conn_slice.width()} != expected {expected_width}"
                )

            # R6: net referenced exists.
            if conn_slice.net not in net_names and conn_slice.net not in port_names:
                report.errors.append(
                    f"R6: instance {mod_name}.{inst.name}.{conn_pin}: "
                    f"net {conn_slice.net!r} not declared"
                )
                continue

            # R7: bits in range.
            target_width = (
                mod.net(conn_slice.net).width  # type: ignore[union-attr]
                if mod.net(conn_slice.net) is not None
                else (mod.port(conn_slice.net).width if mod.port(conn_slice.net) else 0)  # type: ignore[union-attr]
            )
            for bit in conn_slice.bits:
                if not (0 <= bit < target_width):
                    report.errors.append(
                        f"R7: instance {mod_name}.{inst.name}.{conn_pin}: "
                        f"bit {bit} out of range for {conn_slice.net!r} "
                        f"(width {target_width})"
                    )


def port_names_from_module(mod: Module) -> set[str]:
    return {p.name for p in mod.ports}


def _check_no_self_inst(start: str, nl: Netlist, report: ValidationReport) -> None:
    """Detect transitive self-instantiation (R11)."""
    seen: set[str] = set()
    stack: list[str] = [start]
    first = True
    while stack:
        cur = stack.pop()
        if not first and cur == start:
            report.errors.append(
                f"R11: module {start!r} transitively instantiates itself"
            )
            return
        first = False
        if cur in seen:
            continue
        seen.add(cur)
        mod = nl.modules.get(cur)
        if mod is None:
            continue
        for inst in mod.instances:
            stack.append(inst.cell_type)
