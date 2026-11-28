"""Versioned HTTP wire and HTML parsing profiles for the early Web."""

from .html_tags_1992 import Element, Node, parse_html_tags_1992
from .http09 import decode_http09_response, encode_http09_request, parse_http09_request

__all__ = [
    "Element",
    "Node",
    "decode_http09_response",
    "encode_http09_request",
    "parse_html_tags_1992",
    "parse_http09_request",
]
