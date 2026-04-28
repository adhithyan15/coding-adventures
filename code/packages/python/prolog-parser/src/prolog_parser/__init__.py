"""Parser for the first executable Prolog syntax subset."""

from prolog_parser.parser import (
    ParsedQuery,
    ParsedSource,
    PrologParseError,
    create_prolog_parser,
    lower_ast,
    lower_goal_ast,
    parse_ast,
    parse_program,
    parse_query,
    parse_source,
)

__all__ = [
    "__version__",
    "ParsedQuery",
    "ParsedSource",
    "PrologParseError",
    "create_prolog_parser",
    "lower_ast",
    "lower_goal_ast",
    "parse_ast",
    "parse_program",
    "parse_query",
    "parse_source",
]

__version__ = "0.1.0"
