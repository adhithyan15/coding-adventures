"""Verilog Parser — parses Verilog HDL source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python and JavaScript can parse Verilog HDL. No new parser code needed — just
a new grammar.

Verilog is a Hardware Description Language (HDL) used to design digital
circuits. Unlike Python or JavaScript which describe *computations*, Verilog
describes *hardware structures* — modules, wires, registers, and gates that
run in parallel.

Usage::

    from verilog_parser import parse_verilog

    ast = parse_verilog('''
        module and_gate(input a, input b, output y);
            assign y = a & b;
        endmodule
    ''')
    print(ast.rule_name)  # "source_text"
"""

from verilog_parser.parser import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_verilog_parser,
    parse_verilog,
)

__all__ = [
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "create_verilog_parser",
    "parse_verilog",
]
