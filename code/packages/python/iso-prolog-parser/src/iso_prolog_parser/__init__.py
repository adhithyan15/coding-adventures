"""ISO/Core Prolog parser."""

from iso_prolog_parser.parser import (
    ISO_PROLOG_GRAMMAR_PATH,
    create_iso_prolog_parser,
    parse_iso_ast,
    parse_iso_program,
    parse_iso_query,
    parse_iso_source,
)
from prolog_parser import ParsedQuery, ParsedSource, PrologParseError

__all__ = [
    "__version__",
    "ISO_PROLOG_GRAMMAR_PATH",
    "ParsedQuery",
    "ParsedSource",
    "PrologParseError",
    "create_iso_prolog_parser",
    "parse_iso_ast",
    "parse_iso_program",
    "parse_iso_query",
    "parse_iso_source",
]

__version__ = "0.1.0"
