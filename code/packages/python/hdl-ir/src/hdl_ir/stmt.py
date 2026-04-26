"""HIR statement nodes — the body of a Process.

Statements are sequential within a Process. Their semantics depend on whether
the enclosing Process is sensitivity-list mode (re-runs from top each time)
or wait-mode (suspends at each WaitStmt). The simulation VM (``hardware-vm``)
implements both.

JSON encoding uses a ``kind`` discriminator on every statement object.
"""

from __future__ import annotations

from dataclasses import dataclass

from hdl_ir.expr import Expr, expr_from_dict
from hdl_ir.provenance import Provenance


def _prov_to_dict(p: Provenance | None) -> dict[str, object] | None:
    return p.to_dict() if p is not None else None


def _prov_from_dict(d: object) -> Provenance | None:
    if isinstance(d, dict):
        return Provenance.from_dict(d)
    return None


# ----------------------------------------------------------------------------
# Assignments
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class BlockingAssign:
    """Verilog ``=``; VHDL ``:=`` for variables. Immediate update."""

    target: Expr
    rhs: Expr
    delay: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "blocking",
            "target": self.target.to_dict(),  # type: ignore[union-attr]
            "rhs": self.rhs.to_dict(),  # type: ignore[union-attr]
        }
        if self.delay is not None:
            d["delay"] = self.delay.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class NonblockingAssign:
    """Verilog ``<=``; VHDL ``<=`` for signals. Deferred to next delta."""

    target: Expr
    rhs: Expr
    delay: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "nonblocking",
            "target": self.target.to_dict(),  # type: ignore[union-attr]
            "rhs": self.rhs.to_dict(),  # type: ignore[union-attr]
        }
        if self.delay is not None:
            d["delay"] = self.delay.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Control flow
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class IfStmt:
    cond: Expr
    then_branch: tuple[Stmt, ...]
    else_branch: tuple[Stmt, ...] = ()
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "if",
            "cond": self.cond.to_dict(),  # type: ignore[union-attr]
            "then_branch": [s.to_dict() for s in self.then_branch],  # type: ignore[union-attr]
        }
        if self.else_branch:
            d["else_branch"] = [s.to_dict() for s in self.else_branch]  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class CaseItem:
    choices: tuple[Expr, ...]
    body: tuple[Stmt, ...]

    def to_dict(self) -> dict[str, object]:
        return {
            "choices": [c.to_dict() for c in self.choices],  # type: ignore[union-attr]
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }


@dataclass(frozen=True, slots=True)
class CaseStmt:
    expr: Expr
    items: tuple[CaseItem, ...]
    default: tuple[Stmt, ...] | None = None
    kind_: str = "case"

    def __post_init__(self) -> None:
        valid_kinds = {"case", "casex", "casez", "casez_priority"}
        if self.kind_ not in valid_kinds:
            raise ValueError(f"unknown case kind: {self.kind_!r}")

    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "case",
            "case_kind": self.kind_,
            "expr": self.expr.to_dict(),  # type: ignore[union-attr]
            "items": [i.to_dict() for i in self.items],
        }
        if self.default is not None:
            d["default"] = [s.to_dict() for s in self.default]  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class ForStmt:
    init: Stmt
    cond: Expr
    step: Stmt
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "for",
            "init": self.init.to_dict(),  # type: ignore[union-attr]
            "cond": self.cond.to_dict(),  # type: ignore[union-attr]
            "step": self.step.to_dict(),  # type: ignore[union-attr]
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class WhileStmt:
    cond: Expr
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "while",
            "cond": self.cond.to_dict(),  # type: ignore[union-attr]
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class RepeatStmt:
    count: Expr
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "repeat",
            "count": self.count.to_dict(),  # type: ignore[union-attr]
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class ForeverStmt:
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "forever",
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Suspensions
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class WaitStmt:
    """``wait``, ``wait on``, ``wait until``, ``wait for`` — all forms.

    Empty ``on``+``until``+``for_`` (= None) means ``wait`` alone (forever)."""

    on: tuple[Expr, ...] = ()
    until: Expr | None = None
    for_: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "wait",
            "on": [e.to_dict() for e in self.on],  # type: ignore[union-attr]
        }
        if self.until is not None:
            d["until"] = self.until.to_dict()  # type: ignore[union-attr]
        if self.for_ is not None:
            d["for"] = self.for_.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class DelayStmt:
    """``#10`` — procedural delay before continuing."""

    amount: Expr
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "delay",
            "amount": self.amount.to_dict(),  # type: ignore[union-attr]
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class Event:
    """One event in an EventStmt's list. ``edge`` is one of
    ``posedge``/``negedge``/``change``."""

    edge: str
    expr: Expr

    def __post_init__(self) -> None:
        if self.edge not in ("posedge", "negedge", "change"):
            raise ValueError(f"unknown edge: {self.edge!r}")

    def to_dict(self) -> dict[str, object]:
        return {"edge": self.edge, "expr": self.expr.to_dict()}  # type: ignore[union-attr]


@dataclass(frozen=True, slots=True)
class EventStmt:
    """``@(posedge clk)`` followed by a body."""

    events: tuple[Event, ...]
    body: tuple[Stmt, ...]
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "event",
            "events": [e.to_dict() for e in self.events],
            "body": [s.to_dict() for s in self.body],  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Verification
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class AssertStmt:
    cond: Expr
    message: Expr | None = None
    severity: str = "error"
    provenance: Provenance | None = None

    def __post_init__(self) -> None:
        if self.severity not in ("note", "warning", "error", "failure"):
            raise ValueError(f"unknown severity: {self.severity!r}")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "assert",
            "cond": self.cond.to_dict(),  # type: ignore[union-attr]
            "severity": self.severity,
        }
        if self.message is not None:
            d["message"] = self.message.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class ReportStmt:
    message: Expr
    severity: str = "note"
    provenance: Provenance | None = None

    def __post_init__(self) -> None:
        if self.severity not in ("note", "warning", "error", "failure"):
            raise ValueError(f"unknown severity: {self.severity!r}")

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "report",
            "message": self.message.to_dict(),  # type: ignore[union-attr]
            "severity": self.severity,
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Misc
# ----------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class DisableStmt:
    target: str
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"kind": "disable", "target": self.target}
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class ReturnStmt:
    value: Expr | None = None
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"kind": "return"}
        if self.value is not None:
            d["value"] = self.value.to_dict()  # type: ignore[union-attr]
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class NullStmt:
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {"kind": "null"}
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


@dataclass(frozen=True, slots=True)
class ExprStmt:
    """A bare expression used for side effect (e.g., ``$display(...)``)."""

    expr: Expr
    provenance: Provenance | None = None

    def to_dict(self) -> dict[str, object]:
        d: dict[str, object] = {
            "kind": "expr_stmt",
            "expr": self.expr.to_dict(),  # type: ignore[union-attr]
        }
        prov = _prov_to_dict(self.provenance)
        if prov is not None:
            d["provenance"] = prov
        return d


# ----------------------------------------------------------------------------
# Union and JSON deserialization
# ----------------------------------------------------------------------------


Stmt = (
    BlockingAssign
    | NonblockingAssign
    | IfStmt
    | CaseStmt
    | ForStmt
    | WhileStmt
    | RepeatStmt
    | ForeverStmt
    | WaitStmt
    | DelayStmt
    | EventStmt
    | AssertStmt
    | ReportStmt
    | DisableStmt
    | ReturnStmt
    | NullStmt
    | ExprStmt
)


def stmt_from_dict(d: dict[str, object]) -> Stmt:  # noqa: PLR0911, PLR0912
    """Reconstruct a Stmt from its JSON form."""
    kind = d["kind"]
    prov = _prov_from_dict(d.get("provenance"))

    if kind == "blocking":
        return BlockingAssign(
            target=expr_from_dict(d["target"]),  # type: ignore[arg-type]
            rhs=expr_from_dict(d["rhs"]),  # type: ignore[arg-type]
            delay=expr_from_dict(d["delay"]) if "delay" in d else None,  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "nonblocking":
        return NonblockingAssign(
            target=expr_from_dict(d["target"]),  # type: ignore[arg-type]
            rhs=expr_from_dict(d["rhs"]),  # type: ignore[arg-type]
            delay=expr_from_dict(d["delay"]) if "delay" in d else None,  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "if":
        return IfStmt(
            cond=expr_from_dict(d["cond"]),  # type: ignore[arg-type]
            then_branch=tuple(stmt_from_dict(s) for s in d["then_branch"]),  # type: ignore[arg-type, union-attr]
            else_branch=tuple(
                stmt_from_dict(s) for s in d.get("else_branch", [])  # type: ignore[arg-type, union-attr]
            ),
            provenance=prov,
        )
    if kind == "case":
        items = tuple(
            CaseItem(
                choices=tuple(expr_from_dict(c) for c in i["choices"]),  # type: ignore[arg-type, union-attr]
                body=tuple(stmt_from_dict(s) for s in i["body"]),  # type: ignore[arg-type, union-attr]
            )
            for i in d["items"]  # type: ignore[union-attr]
        )
        default_raw = d.get("default")
        default = (
            tuple(stmt_from_dict(s) for s in default_raw)  # type: ignore[arg-type]
            if isinstance(default_raw, list)
            else None
        )
        return CaseStmt(
            expr=expr_from_dict(d["expr"]),  # type: ignore[arg-type]
            items=items,
            default=default,
            kind_=str(d.get("case_kind", "case")),
            provenance=prov,
        )
    if kind == "for":
        return ForStmt(
            init=stmt_from_dict(d["init"]),  # type: ignore[arg-type]
            cond=expr_from_dict(d["cond"]),  # type: ignore[arg-type]
            step=stmt_from_dict(d["step"]),  # type: ignore[arg-type]
            body=tuple(stmt_from_dict(s) for s in d["body"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "while":
        return WhileStmt(
            cond=expr_from_dict(d["cond"]),  # type: ignore[arg-type]
            body=tuple(stmt_from_dict(s) for s in d["body"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "repeat":
        return RepeatStmt(
            count=expr_from_dict(d["count"]),  # type: ignore[arg-type]
            body=tuple(stmt_from_dict(s) for s in d["body"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "forever":
        return ForeverStmt(
            body=tuple(stmt_from_dict(s) for s in d["body"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "wait":
        return WaitStmt(
            on=tuple(expr_from_dict(e) for e in d.get("on", [])),  # type: ignore[arg-type, union-attr]
            until=expr_from_dict(d["until"]) if "until" in d else None,  # type: ignore[arg-type]
            for_=expr_from_dict(d["for"]) if "for" in d else None,  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "delay":
        return DelayStmt(
            amount=expr_from_dict(d["amount"]),  # type: ignore[arg-type]
            body=tuple(stmt_from_dict(s) for s in d["body"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "event":
        events = tuple(
            Event(edge=str(e["edge"]), expr=expr_from_dict(e["expr"]))  # type: ignore[arg-type]
            for e in d["events"]  # type: ignore[union-attr]
        )
        return EventStmt(
            events=events,
            body=tuple(stmt_from_dict(s) for s in d["body"]),  # type: ignore[arg-type, union-attr]
            provenance=prov,
        )
    if kind == "assert":
        return AssertStmt(
            cond=expr_from_dict(d["cond"]),  # type: ignore[arg-type]
            message=expr_from_dict(d["message"]) if "message" in d else None,  # type: ignore[arg-type]
            severity=str(d.get("severity", "error")),
            provenance=prov,
        )
    if kind == "report":
        return ReportStmt(
            message=expr_from_dict(d["message"]),  # type: ignore[arg-type]
            severity=str(d.get("severity", "note")),
            provenance=prov,
        )
    if kind == "disable":
        return DisableStmt(target=str(d["target"]), provenance=prov)
    if kind == "return":
        return ReturnStmt(
            value=expr_from_dict(d["value"]) if "value" in d else None,  # type: ignore[arg-type]
            provenance=prov,
        )
    if kind == "null":
        return NullStmt(provenance=prov)
    if kind == "expr_stmt":
        return ExprStmt(
            expr=expr_from_dict(d["expr"]),  # type: ignore[arg-type]
            provenance=prov,
        )
    raise ValueError(f"unknown statement kind: {kind!r}")
