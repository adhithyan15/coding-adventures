"""Versioned HTTP wire and HTML parsing profiles for the early Web."""

from .html2_paragraphs_1995 import parse_html2_paragraphs_1995
from .html_tags_1992 import Element, Node, parse_html_tags_1992
from .http09 import decode_http09_response, encode_http09_request, parse_http09_request
from .http10_request_line import (
    Http10RequestLine,
    encode_http10_request_line,
    parse_http10_request_line,
)

__all__ = [
    "Element",
    "Http10RequestLine",
    "Node",
    "decode_http09_response",
    "encode_http09_request",
    "encode_http10_request_line",
    "parse_html_tags_1992",
    "parse_html2_paragraphs_1995",
    "parse_http09_request",
    "parse_http10_request_line",
]
