# Historical Web

Python reference for `http-0.9`, the 1991 CERN wire protocol, a closed
`http-1.0-request-line-1996` request-line slice, and
`html-tags-1992`, a markup profile based on the archived 1992 HTML Tags
document. The wire codec handles bytes in memory. HTML parsers take characters
and return ordered trees of elements and text. `parse_html2_paragraphs_1995`
adds a separately versioned, text-only HTML 2.0 slice for `P` and `H1`,
following RFC 1866's optional paragraph end tag and required H1 end tag rules.
Unsupported markup raises `ValueError`; it is not a general SGML or browser
parser.

```python
from historical_web import (
    Http10RequestLine,
    decode_http09_response,
    encode_http09_request,
    encode_http10_request_line,
    parse_html2_paragraphs_1995,
    parse_http10_request_line,
    parse_html_tags_1992,
)

request = encode_http09_request("/index.html")
assert request == b"GET /index.html\r\n"

line_bytes = encode_http10_request_line("HEAD", "/index.html")
assert line_bytes == b"HEAD /index.html HTTP/1.0\r\n"
assert parse_http10_request_line(line_bytes) == Http10RequestLine(
    method="HEAD", target="/index.html"
)

# In a network client these bytes would be read until the server closes TCP.
response = b"<TITLE>Early Web</TITLE><UL><LI>HTTP<LI>HTML</UL>"
title, topics = parse_html_tags_1992(decode_http09_response(response))
assert title.children == ["Early Web"]
assert [item.children for item in topics.children] == [["HTTP"], ["HTML"]]

paragraphs = parse_html2_paragraphs_1995(
    "<H1>Early Web</H1><P>First paragraph<P>Second paragraph"
)
assert [node.tag for node in paragraphs] == ["h1", "p", "p"]
assert paragraphs[1].children == ["First paragraph"]
```

Omitted list-item end tags show how early HTML used SGML minimization. In
XML, the equivalent list needs an end tag for each item. The
[historical specification](../../../specs/HWEB00-historical-web.md) traces
that relationship and defines the recovery rules for this teaching profile.
The [HTML 2.0 paragraph slice](../../../specs/HWEB06-html2-paragraph-slice.md)
is a separate profile: HTML 2.0 P elements contain text, unlike the 1992
archive's empty paragraph marker. It keeps CRLF normalization and source text
preservation distinct from SGML record handling and display whitespace rules.
The [HTTP/1.0 request-line slice](../../../specs/HWEB08-http10-request-line.md)
accepts only uppercase `GET`, `HEAD`, or `POST`, a restricted ASCII absolute
path, the exact `HTTP/1.0` token, and terminal CRLF. It does not parse headers
or bodies and does not change the earlier HTTP/0.9 API.

Run `sh BUILD` to check the reference on Unix, or run `BUILD_windows` commands
in `cmd.exe` on Windows. Future ports can replay the same
[language-neutral vectors](../../../specs/fixtures/historical-web-v1/vectors.json).

The BUILD check also runs the bounded VER05c/VER06d assurance suite: independent
wire acceptance/output models, generated omitted-`LI` semantics, and an
instrumented 2,048-iteration campaign over all public exports. Its contract,
explicit limits, and evidence boundary are recorded in
[HWEB01](../../../specs/HWEB01-historical-web-assurance.md).
