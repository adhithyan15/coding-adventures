# Historical Web

Python reference for `http-0.9`, the 1991 CERN wire protocol, and
`html-tags-1992`, a markup profile based on the archived 1992 HTML Tags
document. The wire codec handles bytes in memory. The HTML parser takes
characters and returns an ordered tree of elements and text.

```python
from historical_web import (
    decode_http09_response,
    encode_http09_request,
    parse_html_tags_1992,
)

request = encode_http09_request("/index.html")
assert request == b"GET /index.html\r\n"

# In a network client these bytes would be read until the server closes TCP.
response = b"<TITLE>Early Web</TITLE><UL><LI>HTTP<LI>HTML</UL>"
title, topics = parse_html_tags_1992(decode_http09_response(response))
assert title.children == ["Early Web"]
assert [item.children for item in topics.children] == [["HTTP"], ["HTML"]]
```

Omitted list-item end tags show how early HTML used SGML minimization. In
XML, the equivalent list needs an end tag for each item. The
[historical specification](../../../specs/HWEB00-historical-web.md) traces
that relationship and defines the recovery rules for this teaching profile.

Run `sh BUILD` to check the reference on Unix, or run `BUILD_windows` commands
in `cmd.exe` on Windows. Future ports can replay the same
[language-neutral vectors](../../../specs/fixtures/historical-web-v1/vectors.json).

The BUILD check also runs the bounded VER05c/VER06d assurance suite: independent
wire acceptance/output models, generated omitted-`LI` semantics, and an
instrumented 2,048-iteration campaign over all public exports. Its contract,
explicit limits, and evidence boundary are recorded in
[HWEB01](../../../specs/HWEB01-historical-web-assurance.md).
