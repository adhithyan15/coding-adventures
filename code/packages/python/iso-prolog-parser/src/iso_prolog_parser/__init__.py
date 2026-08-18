"""ISO/Core Prolog parser."""

from prolog_core import OperatorTable, PrologDirective
from prolog_parser import ParsedQuery, ParsedSource, PrologParseError

from iso_prolog_parser.parser import (
    ParsedIsoSource,
    create_iso_prolog_parser,
    parse_iso_ast,
    parse_iso_program,
    parse_iso_query,
    parse_iso_source,
)

__all__ = [
    "__version__",
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
