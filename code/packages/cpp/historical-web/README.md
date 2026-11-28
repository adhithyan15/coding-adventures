# Historical Web (C++17)

An in-memory C++17 library for two early Web profiles: CERN's 1991
HTTP/0.9 request/response conventions and the W3C archive's November 3, 1992
HTML Tags vocabulary. It takes explicit byte spans, performs no network or
file I/O, and uses only the C++ standard library.

## Example

```cpp
#include "historical_web.hpp"

using namespace coding_adventures::historical_web;

const std::string wire = encode_http09_request("/index.html");
const std::string address = parse_http09_request(wire);
const HtmlDocument page = parse_html_tags_1992("<TITLE>Early &amp; Web</TITLE>");
```

`HtmlDocument` owns a flat node arena and ordered attribute arena. Nodes link to
their parent, first child, and next sibling by indexes; nested owning pointers
are avoided so deep documents can be built and destroyed iteratively. The
result owns its text and remains independent of the source buffer. A
`ProfileError` reports a typed profile error and source-relative byte offset.
Allocation failures remain standard C++ allocation exceptions.

## What the archived profile includes

The 1992 snapshot is a list of tags and entities, not a complete HTML grammar or
a modern browser's recovery algorithm. This teaching implementation recognizes
the containers, list and glossary elements, empty paragraph/index markers,
anchors, comments, and raw-text modes described by HWEB00. It applies the
archived omitted-end conventions for headings, list items, and glossary
entries. Unknown tags are skipped while their contents remain, and malformed
markup is recovered by preserving the unmatched `<` as text.

The parser does not resolve URLs, render layout, fetch resources, process DTDs,
or apply today's WHATWG HTML tree builder. That distinction is useful when
comparing the archive with the later XML parser in
[`XML01`](../../../specs/XML01-xml-parser.md): XML requires explicit matching
end tags and case-sensitive names. For current browser behavior, compare with
the repository's WHATWG-oriented [Mosaic-era HTML builder](../../../specs/BR03-html-tree-builder.md).
These profiles make those historical changes visible without pretending that
the 1992 tag list was already either later standard.

See [HWEB00](../../../specs/HWEB00-historical-web.md) for shared behavior and
[HWEB09](../../../specs/HWEB09-historical-web-cpp.md) for this port's public
API, ownership, and diagnostics.
