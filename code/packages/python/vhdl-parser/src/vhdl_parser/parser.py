"""VHDL Parser — parses VHDL source code into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the Verilog parser: the *same* parser
engine that handles Python and JavaScript can handle VHDL — just swap the
``.grammar`` file.

What Makes VHDL Different from Verilog?
---------------------------------------

VHDL and Verilog describe the same thing (digital hardware), but they take
fundamentally different philosophical approaches:

1. **Separation of interface and implementation.** In Verilog, a ``module``
   contains both the port list and the body. In VHDL, the *entity* declares
   the interface (ports and generics) and the *architecture* provides the
   implementation. One entity can have multiple architectures (behavioral,
   structural, RTL).

2. **Strong typing.** VHDL is strongly typed like Ada. You cannot connect
   an ``integer`` to a ``std_logic_vector`` without an explicit conversion.
   Verilog treats everything as bit vectors and silently truncates or extends.

3. **Case insensitivity.** ``ENTITY``, ``Entity``, and ``entity`` are all
   the same identifier. The lexer normalizes everything to lowercase.

4. **Keyword operators.** Where Verilog uses ``&`` for AND and ``|`` for OR,
   VHDL uses the keywords ``and``, ``or``, ``xor``, ``nand``, ``nor``, ``xnor``.

5. **Two assignment operators.** Signal assignment ``<=`` (like Verilog's
   non-blocking) and variable assignment ``:=`` (like Verilog's blocking).

6. **No preprocessor.** VHDL uses generics (compile-time parameters) and
   generate statements instead of text-level ``define``/``ifdef``.

The Grammar
-----------

The ``vhdl.grammar`` file defines the synthesizable subset of IEEE 1076-2008
VHDL. Key rules include:

- ``design_file`` — top-level: one or more design units
- ``entity_declaration`` — entity interface with ports and generics
- ``architecture_body`` — implementation with signals and concurrent statements
- ``process_statement`` — sequential behavior within concurrent context
- ``if_statement`` / ``case_statement`` — control flow inside processes
- ``component_instantiation`` — structural composition with port maps
- ``expression`` — full precedence chain using keyword operators

The Pipeline
------------

1. VHDL source code is tokenized using ``vhdl.tokens``
2. Tokens are case-normalized (VHDL is case-insensitive)
3. Token stream is parsed using ``vhdl.grammar``
4. Result is a generic ``ASTNode`` tree

Locating the Grammar File
--------------------------

The ``vhdl.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── vhdl_parser/   (parent)
        └── src/           (parent)
            └── vhdl-parser/  (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── vhdl.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from vhdl_lexer import tokenize_vhdl

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate 6 parent levels from this file to reach the repository root's
# code/ directory, then into grammars/.

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
VHDL_GRAMMAR_PATH = GRAMMAR_DIR / "vhdl.grammar"


def create_vhdl_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for VHDL source code.

    This function performs two steps:

    1. **Tokenize** — Calls ``tokenize_vhdl()`` to convert the source
       string into a list of ``Token`` objects. Because VHDL is case-
       insensitive, the tokenizer normalizes all keywords and identifiers
       to lowercase.

    2. **Load grammar** — Reads and parses ``vhdl.grammar`` to get the
       ``ParserGrammar`` rule set.

    The resulting ``GrammarParser`` is ready to call ``.parse()`` to produce
    an AST.

    Args:
        source: The VHDL source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.

    Example::

        parser = create_vhdl_parser('''
            entity counter is
                port(clk : in std_logic;
                     count : out std_logic_vector(7 downto 0));
            end entity counter;
        ''')
        ast = parser.parse()
    """
    # Step 1: Tokenize the VHDL source code.
    # The tokenizer handles keywords (entity, architecture, process, ...),
    # operators (<=, :=, =>, ...), numbers (including based literals like
    # 16#FF#), character literals ('0', '1', 'X'), and identifiers.
    # All NAME and KEYWORD tokens are lowercased for case insensitivity.
    tokens = tokenize_vhdl(source)

    # Step 2: Load and parse the grammar file.
    # The grammar defines the syntax rules — how tokens combine into
    # entities, architectures, processes, expressions, etc.
    grammar = parse_parser_grammar(VHDL_GRAMMAR_PATH.read_text())

    return GrammarParser(tokens, grammar)


def parse_vhdl(source: str) -> ASTNode:
    """Parse VHDL source code and return an AST.

    This is the main entry point for the VHDL parser. It combines
    tokenization, grammar loading, and parsing into a single call.

    The returned ``ASTNode`` tree mirrors the grammar structure. The root
    node has ``rule_name="design_file"`` (from the grammar's start rule),
    and its children are ``design_unit`` nodes, each containing either an
    ``entity_declaration`` or an ``architecture_body``.

    Args:
        source: The VHDL source code to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Example::

        ast = parse_vhdl('''
            entity and_gate is
                port(a, b : in std_logic; y : out std_logic);
            end entity and_gate;
        ''')
        # ASTNode(rule_name="design_file", children=[
        #     ASTNode(rule_name="design_unit", children=[
        #         ASTNode(rule_name="entity_declaration", children=[
        #             Token(KEYWORD, "entity"),
        #             Token(NAME, "and_gate"),
        #             Token(KEYWORD, "is"),
        #             ASTNode(rule_name="port_clause", ...),
        #             Token(KEYWORD, "end"),
        #             Token(KEYWORD, "entity"),
        #             Token(NAME, "and_gate"),
        #             Token(SEMICOLON, ";"),
        #         ])
        #     ])
        # ])
    """
    parser = create_vhdl_parser(source)
    return parser.parse()
