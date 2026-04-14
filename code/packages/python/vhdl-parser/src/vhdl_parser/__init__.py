"""VHDL Parser — parses VHDL source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python, JavaScript, and Verilog can parse VHDL. No new parser code needed —
just a new grammar.

VHDL (VHSIC Hardware Description Language) is a Hardware Description Language
designed by the US Department of Defense. Unlike Verilog which is terse and
C-like, VHDL is verbose and Ada-like, with strong typing, explicit
declarations, and case-insensitive identifiers.

Usage::

    from vhdl_parser import parse_vhdl

    ast = parse_vhdl('''
        entity and_gate is
            port(a, b : in std_logic; y : out std_logic);
        end entity and_gate;
    ''')
    print(ast.rule_name)  # "design_file"
"""

from vhdl_parser.parser import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_vhdl_parser,
    parse_vhdl,
)

__all__ = [
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "create_vhdl_parser",
    "parse_vhdl",
]
