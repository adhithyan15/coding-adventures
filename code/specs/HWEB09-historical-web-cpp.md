# HWEB09 — Historical Web profiles in ISO C++17

This port implements the HWEB00 `http-0.9` and `html-tags-1992` profiles in
ISO C++17. The header is
`code/packages/cpp/historical-web/include/historical_web.hpp`; the compiled
implementation is `src/historical_web.cpp`. It does not implement HWEB06's
later HTML 2.0 paragraph slice.

Public declarations live in namespace `coding_adventures::historical_web`.

## API and diagnostics

All four public functions take length-delimited `std::string_view` inputs and
copy successful results into owning `std::string` or `HtmlDocument` values:

```cpp
std::string encode_http09_request(std::string_view address);
std::string parse_http09_request(std::string_view wire);
std::string decode_http09_response(std::string_view body);
HtmlDocument parse_html_tags_1992(std::string_view source);
```

`HtmlDocument` owns a flat node arena and a flat ordered attribute arena.
`nodes()` and `attributes()` return const vector references; `first_root()`
returns a node index or `no_index`; `root_count()`, `node_count()` and
`attribute_count()` report collection sizes. `node_at()` and `attribute_at()`
return const references and throw `std::out_of_range` for invalid indexes.
Each `HtmlNode` has `kind`, `parent_index`, `first_child_index`,
`next_sibling_index`, `tag`, `text`, `first_attribute_index`, and
`attribute_count`. Element nodes have lowercase tags and an attribute range;
text nodes carry text. Roots have `parent_index == no_index`. A valueless
recognized anchor attribute has `std::nullopt`; an explicitly empty value is
an engaged optional containing an empty string. Strings own their contents,
including any embedded NUL bytes, and remain valid independently of the input
buffer. Copy assignment has the strong exception guarantee and self-assignment
preserves the document. Moving transfers both arenas and the root count and
leaves the source document empty (`node_count() == 0`, `attribute_count() == 0`,
`root_count() == 0`, `first_root() == no_index`).

Invalid protocol inputs and malformed UTF-8 raise `ProfileError`, derived
from `std::runtime_error`, with a typed `ErrorKind` and source-relative
`byte_offset()`. `ErrorKind` is one of `invalid_address`, `invalid_request`,
`invalid_response`, or `invalid_utf8`. The offset identifies the first
nonprintable HTTP address byte, the first invalid request byte or first
missing framing byte (input length), the first non-ASCII response byte, or
the first invalid UTF-8 byte. A truncated UTF-8 sequence reports input length.
Request diagnostics check the `GET ` prefix first: a mismatch reports its byte
position and a short prefix reports the input length. After a valid prefix,
missing terminal LF is reported at the input length; with valid framing, an
empty address reports offset 4 and a nonprintable address byte reports its
first position. Empty address encoding uses `no_index`. Invalid HTML markup
follows HWEB00's recovery rules instead of raising a protocol exception.
Standard allocation failures
(`std::bad_alloc`) and container size exhaustion (`std::length_error`) remain
standard exceptions, distinct from invalid input; the parser publishes no
partial document when construction throws.

`std::string_view` has no runtime type discriminator. Every input is consumed
only as the explicit byte span represented by the view; embedded NUL is not a
terminator. HTTP address rules reject NUL as nonprintable. HTML accepts NUL in
otherwise valid input and preserves it as text.

## HTTP/0.9 behavior

`encode_http09_request` accepts a nonempty sequence of printable ASCII bytes
`0x21` through `0x7e` and returns `GET SP address CRLF`. The parser accepts
that exact one-line request terminated by CRLF or LF, extracts the address
verbatim, and rejects extra words or additional lines. The response decoder
accepts an ASCII body and preserves every byte as a string; an empty body
returns an empty string. No function opens a socket or waits for EOF.

## HTML Tags 1992 behavior

`parse_html_tags_1992` validates the full input as UTF-8 before scanning,
normalizes CRLF to LF throughout the source, and returns an ordered flat tree
equivalent to the Python reference. ASCII tag and attribute names fold to
lowercase using locale-independent byte rules. Tag whitespace is ASCII space,
tab, LF, CR, or form feed. Anchor attributes accept single-quoted,
double-quoted, or unquoted values; quoted values may contain `>`. The first
`name`, `href`, or `type` attribute wins, and valueless recognized attributes
are retained as empty optionals. `&lt;`, `&gt;`, and `&amp;` decode once in
regular text and anchor values; other references remain literal.

The container stack is represented by indices, not nested owning objects, so
construction and destruction are iterative even for deep input. Child and
root order use first-child/next-sibling indexes. Text fragments coalesce
linearly per parent. Indexed open-tag chains locate matching headings, list
items, and glossary entries without repeatedly scanning stack depth for
unmatched or implied end tags. Vector and string growth use standard C++
allocation and size checks; there is no additional profile-specific input cap.

The parser retains HWEB00 recovery: containers include title, address,
anchors, headings and list/glossary elements; paragraph and `isindex` are
empty marker elements. New headings, list items, and glossary entries imply
the documented closes. Matching raw-text sections consume their exact
case-insensitive end tag, `plaintext` consumes through EOF, unknown well-formed
tags are skipped, unmatched ends are ignored, comments disappear, and
malformed `<` candidates remain text. See the complete profile table and
examples in [HWEB00](HWEB00-historical-web.md).

The implementation uses only the C++17 standard library. It performs no file,
network, clock, process, locale, or environment I/O and has no dependency on
the C port. It assumes an ASCII-compatible execution character set for source
literal tag names; input text bytes are validated UTF-8 and processed by
explicit byte length.
