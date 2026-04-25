"""HIR validation rules.

A subset of rules H1-H20 from the spec is implemented here. The remaining
rules (combinational-loop H10, width-mismatch H5, single-driver H7) require
deeper analysis and land as the elaboration / synthesis layers come online.

The intent: catch obvious structural errors at the IR level (missing
references, duplicate names, self-instantiation) so downstream passes can
trust the input.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from hdl_ir.expr import (
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
from hdl_ir.hir import HIR
from hdl_ir.module import Level, Module


@dataclass
class ValidationReport:
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.errors


def validate(hir: HIR) -> ValidationReport:
    """Run all implemented validation rules on a HIR document."""
    report = ValidationReport()

    # H1 — Top exists.
    if hir.top not in hir.modules:
        report.errors.append(
            f"H1: top module {hir.top!r} not in HIR.modules"
        )
        # Without top, downstream checks make less sense; but continue.

    # H1b — Module names unique (already enforced by dict, but check from libs).
    seen_names: set[str] = set(hir.modules.keys())
    for lib in hir.libraries.values():
        for name in lib.modules:
            if name in seen_names:
                report.warnings.append(
                    f"module {name!r} shadowed across library {lib.name!r}"
                )
            seen_names.add(name)

    # H3, H4, H6 — per-module checks.
    for mod_name, mod in hir.modules.items():
        _check_module(mod_name, mod, hir, report)

    # H20 — no self-instantiation (transitive).
    for mod_name in hir.modules:
        _check_no_self_instantiation(mod_name, hir, report)

    return report


def _check_module(
    mod_name: str, mod: Module, hir: HIR, report: ValidationReport
) -> None:
    # H12 — structural module has no processes.
    if mod.level == Level.STRUCTURAL and mod.processes:
        report.errors.append(
            f"H12: module {mod_name!r} is structural but has "
            f"{len(mod.processes)} process(es)"
        )

    # Collect symbol tables for this module's scope.
    port_names = {p.name for p in mod.ports}
    net_names = {n.name for n in mod.nets}

    # Duplicate names within the same module.
    if len(port_names) != len(mod.ports):
        report.errors.append(f"module {mod_name!r}: duplicate port names")
    if len(net_names) != len(mod.nets):
        report.errors.append(f"module {mod_name!r}: duplicate net names")

    # Names overlap.
    overlap = port_names & net_names
    if overlap:
        report.warnings.append(
            f"module {mod_name!r}: name(s) {sorted(overlap)} are both port and net"
        )

    # H11 — every parameter has a default. (Parameter has required `default`,
    # so this is structurally enforced; we re-check value type at elaboration.)

    # H3, H4 — Instance.module resolves; connection keys are real ports.
    for inst in mod.instances:
        target = hir.modules.get(inst.module)
        if target is None:
            # Module may live in a library.
            target_in_lib = None
            for lib in hir.libraries.values():
                if inst.module in lib.modules:
                    target_in_lib = lib.modules[inst.module]
                    break
            if target_in_lib is None:
                report.errors.append(
                    f"H3: instance {mod_name}.{inst.name} references unknown "
                    f"module {inst.module!r}"
                )
                continue
            target = target_in_lib

        target_port_names = {p.name for p in target.ports}
        for conn_pin in inst.connections:
            if conn_pin not in target_port_names:
                report.errors.append(
                    f"H4: instance {mod_name}.{inst.name}: connection key "
                    f"{conn_pin!r} is not a port of module {inst.module!r}"
                )

        target_param_names = {p.name for p in target.parameters}
        for param_name in inst.parameters:
            if param_name not in target_param_names:
                report.warnings.append(
                    f"instance {mod_name}.{inst.name}: parameter "
                    f"{param_name!r} not declared in module {inst.module!r}"
                )

    # H6 — every NetRef / PortRef in expressions resolves.
    for ca in mod.cont_assigns:
        _check_expr_refs(mod_name, "cont_assign.target", ca.target, port_names, net_names, report)
        _check_expr_refs(mod_name, "cont_assign.rhs", ca.rhs, port_names, net_names, report)

    for proc in mod.processes:
        proc_var_names = {v.name for v in proc.variables}
        for sens in proc.sensitivity:
            _check_expr_refs(
                mod_name,
                "process.sensitivity",
                sens.expr,
                port_names,
                net_names,
                report,
                local_vars=proc_var_names,
            )
        # Body expressions / statements: deeper traversal not done here,
        # but the validator can be extended in a future revision.


def _check_expr_refs(
    mod_name: str,
    site: str,
    expr: Expr,
    port_names: set[str],
    net_names: set[str],
    report: ValidationReport,
    local_vars: set[str] | None = None,
) -> None:
    local = local_vars or set()
    if isinstance(expr, NetRef):
        if expr.name not in net_names and expr.name not in port_names:
            # Some elaborators leave port-as-net references; allow if a port.
            report.errors.append(
                f"H6: {mod_name}.{site}: NetRef {expr.name!r} not declared"
            )
    elif isinstance(expr, PortRef):
        if expr.name not in port_names:
            report.errors.append(
                f"H6: {mod_name}.{site}: PortRef {expr.name!r} not declared"
            )
    elif isinstance(expr, VarRef):
        if expr.name not in local:
            report.errors.append(
                f"H6: {mod_name}.{site}: VarRef {expr.name!r} not in scope"
            )
    elif isinstance(expr, Lit):
        return
    elif isinstance(expr, Slice):
        _check_expr_refs(mod_name, site, expr.base, port_names, net_names, report, local)
    elif isinstance(expr, Concat):
        for p in expr.parts:
            _check_expr_refs(mod_name, site, p, port_names, net_names, report, local)
    elif isinstance(expr, Replication):
        _check_expr_refs(mod_name, site, expr.count, port_names, net_names, report, local)
        _check_expr_refs(mod_name, site, expr.body, port_names, net_names, report, local)
    elif isinstance(expr, UnaryOp):
        _check_expr_refs(mod_name, site, expr.operand, port_names, net_names, report, local)
    elif isinstance(expr, BinaryOp):
        _check_expr_refs(mod_name, site, expr.lhs, port_names, net_names, report, local)
        _check_expr_refs(mod_name, site, expr.rhs, port_names, net_names, report, local)
    elif isinstance(expr, Ternary):
        _check_expr_refs(mod_name, site, expr.cond, port_names, net_names, report, local)
        _check_expr_refs(mod_name, site, expr.then_expr, port_names, net_names, report, local)
        _check_expr_refs(mod_name, site, expr.else_expr, port_names, net_names, report, local)
    elif isinstance(expr, (FunCall, SystemCall)):
        for a in expr.args:
            _check_expr_refs(mod_name, site, a, port_names, net_names, report, local)
    elif isinstance(expr, Attribute):
        _check_expr_refs(mod_name, site, expr.base, port_names, net_names, report, local)
        for a in expr.args:
            _check_expr_refs(mod_name, site, a, port_names, net_names, report, local)


def _check_no_self_instantiation(
    start: str, hir: HIR, report: ValidationReport
) -> None:
    """Walk instance graph from ``start``; if we ever hit ``start`` again,
    that's a transitive self-instantiation."""
    seen: set[str] = set()
    stack: list[str] = [start]
    first = True
    while stack:
        cur = stack.pop()
        if not first and cur == start:
            report.errors.append(
                f"H20: module {start!r} transitively instantiates itself"
            )
            return
        first = False
        if cur in seen:
            continue
        seen.add(cur)

        mod = hir.modules.get(cur)
        if mod is None:
            for lib in hir.libraries.values():
                if cur in lib.modules:
                    mod = lib.modules[cur]
                    break
        if mod is None:
            continue

        for inst in mod.instances:
            stack.append(inst.module)
