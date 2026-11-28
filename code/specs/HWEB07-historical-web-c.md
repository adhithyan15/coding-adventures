# HWEB07 — historical Web profiles in ISO C17

This port implements the HWEB00 `http-0.9` and `html-tags-1992` profiles in
pure ISO C17. It does not include the later HWEB06 HTML 2.0 paragraph slice.
The profile behavior remains the one in [HWEB00](HWEB00-historical-web.md);
this document specifies the C byte-buffer API, ownership, and implementation
constraints.

## Public API

The public header is `code/packages/c/historical-web/include/historical_web.h`.
All input is an explicit byte pointer and length. A null pointer is accepted
only for a zero-length input. The caller guarantees each non-empty input is
readable and remains stable throughout the call, and that output/error objects
are writable and do not overlap input or one another. Native C cannot validate
a dishonest pointer/length pair; forged or undersized buffers are outside the
contract. The HTTP wire APIs are:

```c
HwebStatus hweb_http09_encode_request(
    const uint8_t *address, size_t address_length,
    HwebBytes *out, HwebError *error);
HwebStatus hweb_http09_parse_request(
    const uint8_t *request, size_t request_length,
    HwebBytes *out, HwebError *error);
HwebStatus hweb_http09_decode_response(
    const uint8_t *response, size_t response_length,
    HwebBytes *out, HwebError *error);
void hweb_bytes_free(HwebBytes *bytes);
```

`HwebBytes` owns a malloc-allocated byte span and is not NUL terminated. The
output must initially be empty (`data == NULL`, `length == 0`). On success it
owns the result; on failure it remains empty. `hweb_bytes_free` is null-safe,
releases the allocation, and resets the fields. A `HwebError *` is optional;
when supplied it receives the status and the byte offset of an input error, or
`HWEB_NO_INDEX` if the condition has no specific input byte.
`HWEB_ERR_PROTOCOL` reports invalid HTTP/0.9 address/request bytes and
non-ASCII response bytes.

The HTML API is:

```c
HwebStatus hweb_parse_html_tags_1992(
    const uint8_t *source, size_t source_length,
    HwebHtmlDoc **out, HwebError *error);
void hweb_html_doc_free(HwebHtmlDoc **document);
size_t hweb_html_node_count(const HwebHtmlDoc *document);
const HwebNode *hweb_html_node_at(const HwebHtmlDoc *document, size_t index);
size_t hweb_html_attribute_count(const HwebHtmlDoc *document);
const HwebAttribute *hweb_html_attribute_at(
    const HwebHtmlDoc *document, size_t index);
size_t hweb_html_root_first(const HwebHtmlDoc *document);
```

The HTML input must be valid UTF-8; malformed, overlong, or truncated encoding
returns `HWEB_ERR_UTF8` with the offending byte offset. An embedded NUL is
valid input text because all spans are length-delimited. `HwebHtmlDoc` is
opaque and owns its node, attribute, and string storage. The output handle must
be null before parsing and stays null on error. `hweb_html_doc_free` accepts a
null handle pointer, an empty handle, and repeated calls; it frees the document
and resets the handle. Output and error structures must be writable and must
not overlap the input or one another. The caller keeps the input stable for the
full call. The error is reset to `{HWEB_OK, HWEB_NO_INDEX}` on success; on
failure it records the returned status and either the offending byte offset or
`HWEB_NO_INDEX` when there is no input byte to identify. A malformed UTF-8
sequence reports the first invalid byte; a truncated sequence reports its
first missing byte, which is the input length.

The document is a flat arena, never a recursive allocation tree. Each
`HwebNode` stores `kind`, `parent_index`, `first_child_index`,
`next_sibling_index`, `tag`, `text`, `first_attribute_index`, and
`attribute_count`. Roots have parent `HWEB_NO_INDEX`. Text nodes use `text`;
element nodes use lowercase `tag` and an attribute range. A node's unused
views are empty. `HwebAttribute` stores a lowercase `name`, `value`, and
`has_value`; a false `has_value` represents a valueless recognized anchor
attribute. Every `HwebStringView` points into document-owned storage and has an
explicit byte length; views are immutable and expire when the document is
freed, even when a particular string happens to refer to immutable static
storage. Views are not guaranteed NUL-terminated. The root accessor returns the
first root index or `HWEB_NO_INDEX`; follow `next_sibling_index` for later
roots. Node and attribute accessors return null for invalid indexes or a null
document.

`HwebStatus` distinguishes `HWEB_ERR_ARGUMENT`, `HWEB_ERR_PROTOCOL`,
`HWEB_ERR_UTF8`, `HWEB_ERR_NO_MEMORY`, and `HWEB_ERR_OVERFLOW`. In the HTML
entry point, valid UTF-8 with malformed markup follows HWEB00 recovery:
malformed `<` candidates remain text, unmatched ends are ignored, and other
well-formed unknown tags are skipped. It does not return `HWEB_ERR_PROTOCOL`
for markup recovery.
Checked size arithmetic precedes every growth/allocation. Allocation failure
cleans all partial state and does not expose a partial result. No fixed input
limit is added beyond representable `size_t` allocation sizes.

## Parser implementation

The HTML scanner walks UTF-8 bytes after validating the complete source. Tag
syntax and names use the HWEB00 ASCII grammar; character data preserves valid
UTF-8. CRLF normalization covers the whole source. Entity decoding is one pass
over exactly `&lt;`, `&gt;`, and `&amp;` in regular text and anchor values.
Parsing, tree linking, and cleanup are iterative. Open element scopes use
indexed tag positions and boundary scopes, so implied `LI`, `DT` and `DD`
closures do not rescan the open stack for every token. Adjacent text fragments
are accumulated linearly and coalesced per parent.

The runtime has no dependencies beyond the C standard library and does no
file, clock, process, network, or environment access. `c/test-bootstrap`,
`c/bounded-json`, and `c/iso-harness` are test/build dependencies only. The C
assurance suite consumes [the shared HWEB00 vectors](fixtures/historical-web-v1/vectors.json)
through the bounded JSON test dependency.
