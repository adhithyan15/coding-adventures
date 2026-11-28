# Historical Web (C)

ISO C17 implementations of the CERN 1991 HTTP/0.9 wire profile and the
archived 1992 HTML Tags profile. This package ports HWEB00; it does not
implement the separate 1995 HTML 2.0 paragraph slice. Runtime code uses only
the C standard library on ASCII-compatible execution character sets. The
shared test harness, bounded JSON reader, and VER05/VER06 engines are
build/test dependencies only.

The HTTP functions take byte pointers and explicit lengths. They do not open a
socket, read files, or depend on NUL termination:

```c
#include "historical_web.h"

HwebBytes wire = {NULL, 0};
HwebError error;
const uint8_t address[] = "/index.html";
if (hweb_http09_encode_request(address, sizeof(address) - 1,
                               &wire, &error) == HWEB_OK) {
    /* wire contains `GET /index.html\r\n`; it is length-delimited. */
}
hweb_bytes_free(&wire);
```

The HTML parser validates UTF-8 and builds an owned flat arena. Node and
attribute indexes link the tree without recursion; all text, tag, and
attribute views remain valid until the document is freed:

```c
HwebHtmlDoc *document = NULL;
const uint8_t markup[] = "<UL><LI>HTTP<LI>HTML</UL>";
if (hweb_parse_html_tags_1992(markup, sizeof(markup) - 1,
                              &document, &error) == HWEB_OK) {
    size_t root = hweb_html_root_first(document);
    const HwebNode *list = hweb_html_node_at(document, root);
    size_t first_item = list->first_child_index;
    const HwebNode *item = hweb_html_node_at(document, first_item);
    /* item->tag is `li`; text children link through next_sibling_index. */
    (void)item;
}
hweb_html_doc_free(&document);
```

The profile keeps the historical distinction between SGML-era HTML and XML.
The archived 1992 document describes `<P>` as an empty paragraph marker and
permits omitted list-item ends. The repository's XML parser requires explicit
closing tags and case-sensitive names. HTML 2.0 in RFC 1866 later made P
text-bearing. Today's WHATWG parser has a much larger tokenizer and recovery
model. See [HWEB00](../../../specs/HWEB00-historical-web.md) and the C API and
ownership contract in [HWEB07](../../../specs/HWEB07-historical-web-c.md).

Run `sh BUILD` on Unix or the commands in `BUILD_windows` on Windows. The
assurance suite replays the same [portable vectors](../../../specs/fixtures/historical-web-v1/vectors.json)
as the Python, Go, Rust, TypeScript, and Ruby implementations.
