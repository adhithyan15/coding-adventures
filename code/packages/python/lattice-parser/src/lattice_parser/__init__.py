"""lattice-parser — Parses Lattice CSS superset source into ASTs.

This package is a thin wrapper around the generic ``GrammarParser``, loading
the ``lattice.grammar`` file. It produces ``ASTNode`` trees containing both
CSS nodes and Lattice nodes (variables, mixins, control flow, etc.).

This package is part of the coding-adventures monorepo.
"""

__version__ = "0.1.0"

from lattice_parser.parser import create_lattice_parser, parse_lattice

__all__ = ["create_lattice_parser", "parse_lattice"]
