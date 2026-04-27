"""Module-level HIR constructs: Net, Variable, Port, Process, ContAssign,
Instance, Parameter, Module, Library.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

from hdl_ir.expr import Expr, expr_from_dict
from hdl_ir.provenance import Provenance
from hdl_ir.stmt import Stmt, stmt_from_dict
from hdl_ir.types import Ty, ty_from_dict


def _prov_to_dict(p: Provenance | None) -> dict[str, object] | None:
    return p.to_dict() if p is not None else None


def _prov_from_dict(d: object) -> Provenance | None:
    if isinstance(d, dict):
        return Provenance.from_dict(d)
    return None


# ----------------------------------------------------------------------------
# Net + Variable
# ----------------------------------------------------------------------------


class NetKind(Enum):
    SIGNAL = "signal"
    WIRE = "wire"
    REG = "reg"
    TRI = "tri"
    WAND = "wand"
    WOR = "wor"
    SUPPLY0 = "supply0"
    SUPPLY1 = "supply1"
    RESOLVED_SIGNAL = "resolved_signal"


@dataclass(frozen=True, slots=True)
class Net:
    """A wire / signal in a Module's scope."""

    name: str
    type: Ty
    kind: NetKind = NetKind.SIGNAL
    initial: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "name": self.name,
            "type": self.type.to_dict(),  # type: ignore[union-attr]
            "kind": self.kind.value,
        }
        if self.initial is not None:
            d["initial"] = self.initial.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Net:
        return cls(
            name=str(d["name"]),
            type=ty_from_dict(d["type"]),  # type: ignore[arg-type]
            kind=NetKind(d.get("kind", "signal")),
            initial=expr_from_dict(d["initial"]) if "initial" in d else None,  # type: ignore[arg-type]
            provenance=_prov_from_dict(d.get("provenance")),
        )


@dataclass(frozen=True, slots=True)
class Variable:
    """Process-local storage with immediate-update semantics (VHDL ``variable``)."""

    name: str
    type: Ty
    initial: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "name": self.name,
            "type": self.type.to_dict(),  # type: ignore[union-attr]
        }
        if self.initial is not None:
            d["initial"] = self.initial.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Variable:
        return cls(
            name=str(d["name"]),
            type=ty_from_dict(d["type"]),  # type: ignore[arg-type]
            initial=expr_from_dict(d["initial"]) if "initial" in d else None,  # type: ignore[arg-type]
            provenance=_prov_from_dict(d.get("provenance")),
        )


# ----------------------------------------------------------------------------
# Port
# ----------------------------------------------------------------------------


class Direction(Enum):
    IN = "in"
    OUT = "out"
    INOUT = "inout"
    BUFFER = "buffer"


@dataclass(frozen=True, slots=True)
class Port:
    """An external interface point of a Module."""

    name: str
    direction: Direction
    type: Ty
    default: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "name": self.name,
            "direction": self.direction.value,
            "type": self.type.to_dict(),  # type: ignore[union-attr]
        }
        if self.default is not None:
            d["default"] = self.default.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Port:
        return cls(
            name=str(d["name"]),
            direction=Direction(d["direction"]),
            type=ty_from_dict(d["type"]),  # type: ignore[arg-type]
            default=expr_from_dict(d["default"]) if "default" in d else None,  # type: ignore[arg-type]
            provenance=_prov_from_dict(d.get("provenance")),
        )


# ----------------------------------------------------------------------------
# Process
# ----------------------------------------------------------------------------


class ProcessKind(Enum):
    ALWAYS = "always"
    INITIAL = "initial"
    PROCESS = "process"
    ALWAYS_FF = "always_ff"
    ALWAYS_COMB = "always_comb"


@dataclass(frozen=True, slots=True)
class SensitivityItem:
    edge: str  # 'posedge' | 'negedge' | 'change'
    expr: Expr

    def __post_init__(self) -> None:
        if self.edge not in ("posedge", "negedge", "change"):
            raise ValueError(f"unknown edge: {self.edge!r}")

    def to_dict(self) -> dict[str, object]:
        return {"edge": self.edge, "expr": self.expr.to_dict()}  # type: ignore[union-attr]


@dataclass(frozen=True, slots=True)
class Process:
    """A unit of behavioral logic with either a sensitivity list or
    explicit ``wait`` statements (mutually exclusive — see HIR rule H9)."""

    kind: ProcessKind
    sensitivity: tuple[SensitivityItem, ...] = ()
    variables: tuple[Variable, ...] = ()
    body: tuple[Stmt, ...] = ()
    name: str | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": self.kind.value,
            "sensitivity": [s.to_dict() for s in self.sensitivity],
            "variables": [v.to_dict() for v in self.variables],
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        if self.name is not None:
            d["name"] = self.name
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Process:
        sens = tuple(
            SensitivityItem(edge=str(s["edge"]), expr=expr_from_dict(s["expr"]))  # type: ignore[arg-type]
            for s in d.get("sensitivity", [])  # type: ignore[union-attr]
        )
        vars_ = tuple(Variable.from_dict(v) for v in d.get("variables", []))  # type: ignore[arg-type, union-attr]
        body = tuple(stmt_from_dict(s) for s in d.get("body", []))  # type: ignore[arg-type, union-attr]
        return cls(
            kind=ProcessKind(d["kind"]),
            sensitivity=sens,
            variables=vars_,
            body=body,
            name=str(d["name"]) if "name" in d else None,
            provenance=_prov_from_dict(d.get("provenance")),
        )


# ----------------------------------------------------------------------------
# Continuous assignment
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ContAssign:
    """Continuous (combinational) assignment. VHDL concurrent signal assign;
    Verilog ``assign``."""

    target: Expr
    rhs: Expr
    delay: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "target": self.target.to_dict(),  # type: ignore[union-attr]
            "rhs": self.rhs.to_dict(),  # type: ignore[union-attr]
        }
        if self.delay is not None:
            d["delay"] = self.delay.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> ContAssign:
        return cls(
            target=expr_from_dict(d["target"]),  # type: ignore[arg-type]
            rhs=expr_from_dict(d["rhs"]),  # type: ignore[arg-type]
            delay=expr_from_dict(d["delay"]) if "delay" in d else None,  # type: ignore[arg-type]
            provenance=_prov_from_dict(d.get("provenance")),
        )


# ----------------------------------------------------------------------------
# Instance
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Instance:
    """An instantiation of another Module (or a primitive cell type)."""

    name: str
    module: str
    parameters: dict[str, Expr] = field(default_factory=dict)
    connections: dict[str, Expr] = field(default_factory=dict)
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "name": self.name,
            "module": self.module,
            "parameters": {
                k: v.to_dict() for k, v in self.parameters.items()  # type: ignore[union-attr]
            },
            "connections": {
                k: v.to_dict() for k, v in self.connections.items()  # type: ignore[union-attr]
            },
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Instance:
        params_raw = d.get("parameters", {})
        conns_raw = d.get("connections", {})
        return cls(
            name=str(d["name"]),
            module=str(d["module"]),
            parameters={
                k: expr_from_dict(v)  # type: ignore[arg-type]
                for k, v in params_raw.items()  # type: ignore[union-attr]
            },
            connections={
                k: expr_from_dict(v)  # type: ignore[arg-type]
                for k, v in conns_raw.items()  # type: ignore[union-attr]
            },
            provenance=_prov_from_dict(d.get("provenance")),
        )


# ----------------------------------------------------------------------------
# Parameter
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Parameter:
    """A module parameter / generic, with required default value."""

    name: str
    type: Ty
    default: Expr
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "name": self.name,
            "type": self.type.to_dict(),  # type: ignore[union-attr]
            "default": self.default.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Parameter:
        return cls(
            name=str(d["name"]),
            type=ty_from_dict(d["type"]),  # type: ignore[arg-type]
            default=expr_from_dict(d["default"]),  # type: ignore[arg-type]
            provenance=_prov_from_dict(d.get("provenance")),
        )


# ----------------------------------------------------------------------------
# Module
# ----------------------------------------------------------------------------


class Level(Enum):
    BEHAVIORAL = "behavioral"
    STRUCTURAL = "structural"


@dataclass
class Module:
    """A circuit definition. Mutable (lists, not tuples) for ergonomic
    incremental construction by elaborators."""

    name: str
    ports: list[Port] = field(default_factory=list)
    parameters: list[Parameter] = field(default_factory=list)
    nets: list[Net] = field(default_factory=list)
    instances: list[Instance] = field(default_factory=list)
    cont_assigns: list[ContAssign] = field(default_factory=list)
    processes: list[Process] = field(default_factory=list)
    level: Level = Level.BEHAVIORAL
    attributes: dict[str, Expr] = field(default_factory=dict)
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "name": self.name,
            "level": self.level.value,
            "ports": [p.to_dict() for p in self.ports],
            "parameters": [p.to_dict() for p in self.parameters],
            "nets": [n.to_dict() for n in self.nets],
            "instances": [i.to_dict() for i in self.instances],
            "cont_assigns": [c.to_dict() for c in self.cont_assigns],
            "processes": [p.to_dict() for p in self.processes],
            "attributes": {
                k: v.to_dict() for k, v in self.attributes.items()  # type: ignore[union-attr]
            },
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Module:
        attrs_raw = d.get("attributes", {})
        return cls(
            name=str(d["name"]),
            level=Level(d.get("level", "behavioral")),
            ports=[Port.from_dict(p) for p in d.get("ports", [])],  # type: ignore[arg-type]
            parameters=[
                Parameter.from_dict(p) for p in d.get("parameters", [])  # type: ignore[arg-type]
            ],
            nets=[Net.from_dict(n) for n in d.get("nets", [])],  # type: ignore[arg-type]
            instances=[Instance.from_dict(i) for i in d.get("instances", [])],  # type: ignore[arg-type]
            cont_assigns=[
                ContAssign.from_dict(c) for c in d.get("cont_assigns", [])  # type: ignore[arg-type]
            ],
            processes=[Process.from_dict(p) for p in d.get("processes", [])],  # type: ignore[arg-type]
            attributes={
                k: expr_from_dict(v)  # type: ignore[arg-type]
                for k, v in attrs_raw.items()  # type: ignore[union-attr]
            },
            provenance=_prov_from_dict(d.get("provenance")),
        )

    def find_port(self, name: str) -> Port | None:
        for p in self.ports:
            if p.name == name:
                return p
        return None

    def find_net(self, name: str) -> Net | None:
        for n in self.nets:
            if n.name == name:
                return n
        return None


# ----------------------------------------------------------------------------
# Library
# ----------------------------------------------------------------------------


@dataclass
class Library:
    """A namespace of related modules — corresponds to a VHDL library."""

    name: str
    modules: dict[str, Module] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        return {
            "name": self.name,
            "modules": {k: m.to_dict() for k, m in self.modules.items()},
        }

    @classmethod
    def from_dict(cls, d: dict[str, object]) -> Library:
        mods_raw = d.get("modules", {})
        return cls(
            name=str(d["name"]),
            modules={
                k: Module.from_dict(v)  # type: ignore[arg-type]
                for k, v in mods_raw.items()  # type: ignore[union-attr]
            },
        )
