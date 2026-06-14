"""SWI-Prolog parser."""

from prolog_core import OperatorTable, PrologDirective
from prolog_parser import ParsedQuery, PrologParseError

from swi_prolog_parser.parser import (
    SWI_PROLOG_GRAMMAR_PATH,
    ParsedSwiDirective,
    ParsedSwiSource,
    create_swi_prolog_parser,
    parse_swi_ast,
    parse_swi_program,
    parse_swi_query,
    parse_swi_source,
    parse_swi_term,
)

__all__ = [
    "__version__",
    "OperatorTable",
    "ParsedQuery",
    "PrologDirective",
    "ParsedSwiDirective",
    "ParsedSwiSource",
    "PrologParseError",
    "SWI_PROLOG_GRAMMAR_PATH",
    "create_swi_prolog_parser",
    "parse_swi_ast",
    "parse_swi_program",
    "parse_swi_query",
    "parse_swi_source",
    "parse_swi_term",
]

__version__ = "0.1.0"
