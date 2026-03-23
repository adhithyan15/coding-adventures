"""Lisp Parser — parses Lisp source code into ASTs using the grammar-driven approach.

This module re-exports the public API from the parser module.
"""

from lisp_parser.parser import create_lisp_parser, parse_lisp

__all__ = ["create_lisp_parser", "parse_lisp"]
