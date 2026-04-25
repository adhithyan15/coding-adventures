"""hdl-elaboration: AST -> HIR.

Walks parser ASTs (Verilog today; VHDL and Ruby DSL traces in 0.2.0) and
emits HIR ready for the rest of the silicon stack."""

from hdl_elaboration.elaborator import (
    SymbolTable,
    bind_module,
    collect,
    elaborate,
    elaborate_verilog,
)
from hdl_elaboration.verilog_to_hir import (
    ExprElaborator,
    elaborate_module_decl,
)

__version__ = "0.1.0"

__all__ = [
    "ExprElaborator",
    "SymbolTable",
    "__version__",
    "bind_module",
    "collect",
    "elaborate",
    "elaborate_module_decl",
    "elaborate_verilog",
]
