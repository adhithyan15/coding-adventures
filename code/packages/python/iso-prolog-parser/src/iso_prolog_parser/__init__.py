"""ISO/Core Prolog parser."""

from prolog_core import OperatorTable, PrologDirective
from prolog_parser import ParsedQuery, ParsedSource, PrologParseError

from iso_prolog_parser.parser import (
    ISO_PROLOG_GRAMMAR_PATH,
    ParsedIsoSource,
    create_iso_prolog_parser,
    parse_iso_ast,
    parse_iso_program,
    parse_iso_query,
    parse_iso_source,
)

__all__ = [
    "__version__",
    "ISO_PROLOG_GRAMMAR_PATH",
    "OperatorTable",
    "ParsedQuery",
    "ParsedIsoSource",
    "ParsedSource",
    "PrologDirective",
    "PrologParseError",
    "create_iso_prolog_parser",
    "parse_iso_ast",
    "parse_iso_program",
    "parse_iso_query",
    "parse_iso_source",
]

__version__ = "0.1.0"
