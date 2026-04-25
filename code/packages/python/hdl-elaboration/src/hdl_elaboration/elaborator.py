"""Top-level elaborator orchestrator.

Three-pass elaboration per ``hdl-elaboration.md``:
1. Collect — gather every module_declaration found in any input AST.
2. Bind — name resolution, parameter binding, type tagging.
3. Unroll — generate-for / generate-if expansion (basic; full support in 0.2.0).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from hdl_ir import HIR, Module
from lang_parser import ASTNode

from hdl_elaboration.verilog_to_hir import elaborate_module_decl


@dataclass
class SymbolTable:
    """Pass-1 collection of every module declaration we've encountered."""

    verilog_modules: dict[str, ASTNode] = field(default_factory=dict)


def collect(ast: ASTNode, symtab: SymbolTable) -> None:
    """Walk a Verilog AST and register every module_declaration."""
    if not hasattr(ast, "rule_name"):
        return
    if ast.rule_name == "module_declaration":
        # Find the module name (first NAME token).
        for c in ast.children:
            if (
                not hasattr(c, "rule_name")
                and hasattr(c, "value")
                and str(c.type) == "TokenType.NAME"  # type: ignore[union-attr]
            ):
                symtab.verilog_modules[c.value] = ast  # type: ignore[union-attr]
                return
    # Recurse.
    for c in ast.children:
        if hasattr(c, "rule_name"):
            collect(c, symtab)


def bind_module(name: str, symtab: SymbolTable, file: str = "<source>") -> Module:
    """Pass 2: take a collected module AST and elaborate it to HIR."""
    if name not in symtab.verilog_modules:
        raise KeyError(f"module {name!r} not found")
    return elaborate_module_decl(symtab.verilog_modules[name], file=file)


def elaborate(asts: list[ASTNode], top: str, file: str = "<source>") -> HIR:
    """Run all three passes and return a HIR document."""
    symtab = SymbolTable()
    for ast in asts:
        collect(ast, symtab)

    if top not in symtab.verilog_modules:
        raise KeyError(
            f"top module {top!r} not found among "
            f"{sorted(symtab.verilog_modules.keys())}"
        )

    # Pass 2 — bind starting from `top`, recurse through instances.
    bound: dict[str, Module] = {}
    _bind_recursive(top, symtab, bound, file)

    # Pass 3 — Unroll. (No-op in v0.1.0; the simple Verilog inputs we target
    # don't use generate constructs.)

    return HIR(top=top, modules=bound)


def _bind_recursive(
    name: str, symtab: SymbolTable, bound: dict[str, Module], file: str
) -> None:
    if name in bound:
        return
    if name not in symtab.verilog_modules:
        # Could be a primitive cell (defined externally); skip.
        return
    module = bind_module(name, symtab, file)
    bound[name] = module
    for inst in module.instances:
        _bind_recursive(inst.module, symtab, bound, file)


# ---------------------------------------------------------------------------
# Convenience entrypoint
# ---------------------------------------------------------------------------


def elaborate_verilog(
    source: str, top: str, file: str = "<source>", version: str | None = None
) -> HIR:
    """One-shot: parse + elaborate Verilog source. Uses the default Verilog
    edition from ``verilog-parser`` unless ``version`` is overridden."""
    from verilog_parser import parse_verilog

    ast = parse_verilog(source, version=version)
    return elaborate(asts=[ast], top=top, file=file)
