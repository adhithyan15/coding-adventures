"""Operator-aware token-level Prolog parser."""

from prolog_operator_parser.parser import (
    ParsedOperatorSource,
    ParsedOperatorTerm,
    __version__,
    parse_operator_goal_tokens,
    parse_operator_named_term_tokens,
    parse_operator_program_tokens,
    parse_operator_query_tokens,
    parse_operator_source_tokens,
    parse_operator_term_tokens,
)

__all__ = [
    "__version__",
    "ParsedOperatorSource",
    "ParsedOperatorTerm",
    "parse_operator_goal_tokens",
    "parse_operator_named_term_tokens",
    "parse_operator_program_tokens",
    "parse_operator_query_tokens",
    "parse_operator_source_tokens",
    "parse_operator_term_tokens",
]
