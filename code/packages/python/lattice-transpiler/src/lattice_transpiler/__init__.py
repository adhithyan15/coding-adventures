"""lattice-transpiler — Full pipeline from Lattice source to CSS output.

This package is the top-level entry point for the Lattice transpiler. It
pipelines three independent packages:

1. ``lattice-parser`` — Parses Lattice source into an AST.
2. ``lattice-ast-to-css`` — Transforms the Lattice AST into a clean CSS AST.
3. ``lattice-ast-to-css`` — Emits CSS text from the clean AST.

Usage::

    from lattice_transpiler import transpile_lattice

    css = transpile_lattice('''
        $color: red;
        h1 { color: $color; }
    ''')
    # → "h1 {\\n  color: red;\\n}\\n"

For finer control, use the individual packages directly.
"""

__version__ = "0.1.0"

from lattice_transpiler.transpiler import transpile_lattice

__all__ = ["transpile_lattice"]
