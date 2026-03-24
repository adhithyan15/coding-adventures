"""Verilog Parser — parses Verilog HDL source code into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the JavaScript parser: the *same* parser
engine that handles Python and JavaScript can handle Verilog — just swap the
``.grammar`` file.

What Makes Verilog Different from Software Languages?
------------------------------------------------------

Verilog describes *hardware*, not *software*. This has profound implications:

1. **Parallelism is the default.** Every ``always`` block, every ``assign``
   statement runs *concurrently*. There is no sequential "main function."

2. **Modules replace classes.** A Verilog "module" is like a hardware
   component — it has input/output ports (like pins on a chip) and internal
   wiring.

3. **Two assignment types.** Blocking (``=``) executes sequentially within
   a block. Non-blocking (``<=``) schedules updates at the end of a time
   step, modeling how real flip-flops work.

4. **Bit widths matter.** Every signal has an explicit width: ``[7:0]``
   means 8 bits. There are no arbitrary-precision integers.

5. **Time is explicit.** Delays (``#10``), clock edges (``posedge clk``),
   and sensitivity lists (``@(*)`) control when things happen.

The Grammar
-----------

The ``verilog.grammar`` file defines the synthesizable subset of
IEEE 1364-2005 Verilog. Key rules include:

- ``source_text`` — top-level: one or more module declarations
- ``module_declaration`` — ``module NAME ... endmodule``
- ``continuous_assign`` — ``assign y = a & b;``
- ``always_construct`` — ``always @(...) statement``
- ``module_instantiation`` — structural connections between modules
- ``expression`` — full precedence chain from ternary down to primary

The Pipeline
------------

1. Verilog source code is preprocessed (expand macros, evaluate ``ifdef``)
2. Preprocessed source is tokenized using ``verilog.tokens``
3. Token stream is parsed using ``verilog.grammar``
4. Result is a generic ``ASTNode`` tree

Locating the Grammar File
--------------------------

The ``verilog.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── verilog_parser/  (parent)
        └── src/            (parent)
            └── verilog-parser/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── verilog.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from verilog_lexer import tokenize_verilog

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate 6 parent levels from this file to reach the repository root's
# code/ directory, then into grammars/.

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
VERILOG_GRAMMAR_PATH = GRAMMAR_DIR / "verilog.grammar"


def create_verilog_parser(
    source: str, *, preprocess: bool = True
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Verilog source code.

    This function performs two steps:

    1. **Tokenize** — Calls ``tokenize_verilog()`` to convert the source
       string into a list of ``Token`` objects. If ``preprocess=True``
       (the default), Verilog preprocessor directives (``` `define ```,
       ``` `ifdef ```, etc.) are expanded before tokenization.

    2. **Load grammar** — Reads and parses ``verilog.grammar`` to get the
       ``ParserGrammar`` rule set.

    The resulting ``GrammarParser`` is ready to call ``.parse()`` to produce
    an AST.

    Args:
        source: The Verilog source code to parse.
        preprocess: Whether to run the Verilog preprocessor before
            tokenization. Defaults to True. Set to False when parsing
            source that has already been preprocessed.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_verilog_parser('''
            module counter(input clk, output reg [7:0] count);
                always @(posedge clk) count <= count + 1;
            endmodule
        ''')
        ast = parser.parse()
    """
    # Step 1: Tokenize the Verilog source code.
    # The tokenizer handles keywords (module, endmodule, assign, always, ...),
    # operators (&, |, ^, ~, ...), numbers (including sized literals like
    # 8'hFF), and identifiers.
    tokens = tokenize_verilog(source, preprocess=preprocess)

    # Step 2: Load and parse the grammar file.
    # The grammar defines the syntax rules — how tokens combine into
    # modules, statements, expressions, etc.
    grammar = parse_parser_grammar(VERILOG_GRAMMAR_PATH.read_text())

    return GrammarParser(tokens, grammar)


def parse_verilog(source: str, *, preprocess: bool = True) -> ASTNode:
    """Parse Verilog source code and return an AST.

    This is the main entry point for the Verilog parser. It combines
    tokenization, grammar loading, and parsing into a single call.

    The returned ``ASTNode`` tree mirrors the grammar structure. The root
    node has ``rule_name="source_text"`` (from the grammar's start rule),
    and its children are ``description`` nodes, each containing a
    ``module_declaration``.

    Args:
        source: The Verilog source code to parse.
        preprocess: Whether to run the Verilog preprocessor. Defaults to True.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Example::

        ast = parse_verilog('''
            module and_gate(input a, input b, output y);
                assign y = a & b;
            endmodule
        ''')
        # ASTNode(rule_name="source_text", children=[
        #     ASTNode(rule_name="description", children=[
        #         ASTNode(rule_name="module_declaration", children=[
        #             Token(KEYWORD, "module"),
        #             Token(NAME, "and_gate"),
        #             ASTNode(rule_name="port_list", ...),
        #             Token(SEMICOLON, ";"),
        #             ASTNode(rule_name="module_item", children=[
        #                 ASTNode(rule_name="continuous_assign", ...)
        #             ]),
        #             Token(KEYWORD, "endmodule"),
        #         ])
        #     ])
        # ])
    """
    parser = create_verilog_parser(source, preprocess=preprocess)
    return parser.parse()
