"""
csv_parser — A hand-rolled CSV state machine parser.

This package converts CSV text into a list of row maps (dicts). It does NOT use
Python's standard library csv module — the implementation is a pure state machine
built from scratch for educational purposes.

Public API:
    parse_csv(source: str, delimiter: str = ",") -> list[dict[str, str]]

Exceptions:
    UnclosedQuoteError — raised when a quoted field is never closed

Example:
    >>> from csv_parser import parse_csv
    >>> rows = parse_csv("name,age\\nAlice,30\\nBob,25")
    >>> rows
    [{'name': 'Alice', 'age': '30'}, {'name': 'Bob', 'age': '25'}]
"""

from csv_parser.errors import UnclosedQuoteError
from csv_parser.parser import parse_csv

__all__ = ["parse_csv", "UnclosedQuoteError"]
