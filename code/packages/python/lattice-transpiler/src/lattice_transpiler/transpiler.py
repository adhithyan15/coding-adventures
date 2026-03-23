"""Lattice transpiler — pipelines parse → transform → emit.

This module is intentionally thin. It wires together three packages:

1. ``lattice_parser.parse_lattice()`` — Source text → Lattice AST
2. ``LatticeTransformer.transform()`` — Lattice AST → Clean CSS AST
3. ``CSSEmitter.emit()`` — Clean CSS AST → CSS text

Each step is a standalone package with its own tests. This module just
connects them in sequence.

Pipeline Diagram::

    Lattice Source
         │
         ▼
    ┌─────────────┐
    │ Lattice Lexer│  ← lattice.tokens
    └──────┬──────┘
           │ tokens
           ▼
    ┌─────────────┐
    │Lattice Parser│  ← lattice.grammar
    └──────┬──────┘
           │ AST (CSS + Lattice nodes)
           ▼
    ┌─────────────┐
    │ Transformer  │  ← scope, evaluator
    └──────┬──────┘
           │ AST (CSS nodes only)
           ▼
    ┌─────────────┐
    │  CSS Emitter │
    └──────┬──────┘
           │
           ▼
      CSS Text
"""

from __future__ import annotations

from lattice_ast_to_css import CSSEmitter, LatticeTransformer
from lattice_parser import parse_lattice


def transpile_lattice(
    source: str,
    *,
    minified: bool = False,
    indent: str = "  ",
) -> str:
    """Transpile Lattice source text to CSS.

    This is the main entry point for the Lattice transpiler. Pass in
    a string of Lattice source, get back CSS text.

    Args:
        source: The Lattice source text to transpile.
        minified: If True, emit minified CSS (no unnecessary whitespace).
        indent: The indentation string per nesting level (default: 2 spaces).

    Returns:
        The transpiled CSS text.

    Raises:
        LatticeError: If the source has Lattice-specific errors
            (undefined variables, circular mixins, type errors, etc.).
        GrammarParseError: If the source has syntax errors.
        LexerError: If the source has lexical errors.

    Example::

        css = transpile_lattice('''
            $primary: #4a90d9;
            h1 { color: $primary; }
        ''')
        # h1 {
        #   color: #4a90d9;
        # }
    """
    # Step 1: Parse
    ast = parse_lattice(source)

    # Step 2: Transform
    transformer = LatticeTransformer()
    css_ast = transformer.transform(ast)

    # Step 3: Emit
    emitter = CSSEmitter(indent=indent, minified=minified)
    return emitter.emit(css_ast)
