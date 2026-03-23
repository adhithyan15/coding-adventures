"""lattice-ast-to-css — Lattice AST-to-CSS compiler.

Transforms a Lattice AST (produced by ``lattice-parser``) into plain CSS text.
The AST contains both CSS nodes and Lattice nodes (variables, mixins, control
flow, functions). This package expands all Lattice nodes into pure CSS and
emits the resulting CSS text.

Pipeline::

    Lattice AST → Transformer → Clean CSS AST → Emitter → CSS text

Components:

- ``errors`` — Structured error types for all compiler phases
- ``scope`` — Lexical scope chain for variable/mixin/function lookup
- ``evaluator`` — Compile-time expression evaluator for @if/@for/@return
- ``emitter`` — CSS source text emitter from clean CSS AST
- ``transformer`` — Multi-pass AST transformer (the core)

This package is part of the coding-adventures monorepo.
"""

__version__ = "0.1.0"

from lattice_ast_to_css.emitter import CSSEmitter
from lattice_ast_to_css.errors import LatticeError
from lattice_ast_to_css.scope import ScopeChain
from lattice_ast_to_css.transformer import LatticeTransformer

__all__ = [
    "CSSEmitter",
    "LatticeError",
    "LatticeTransformer",
    "ScopeChain",
]
