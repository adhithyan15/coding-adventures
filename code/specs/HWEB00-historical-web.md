# HWEB00 — The early Web in executable profiles

Start with two independently versioned profiles. **`http-0.9`** follows the
1991 CERN protocol description. **`html-tags-1992`** models the vocabulary
documented in the W3C snapshot dated November 3, 1992. The snapshot date
identifies the source. The profile name is a repository identifier. The
first IETF HTML specification was HTML 2.0 (RFC 1866, November 1995).

- [The Original HTTP as defined in 1991](https://www.w3.org/Protocols/HTTP/AsImplemented.html)
- [HTML Tags](https://www.w3.org/History/19921103-hypertext/hypertext/WWW/MarkUp/Tags.html)
- [HTML Entities](https://www.w3.org/History/19921103-hypertext/hypertext/WWW/MarkUp/Entities.html)
- [RFC 1866](https://www.rfc-editor.org/rfc/rfc1866)

## HTTP/0.9 wire profile

The client sends ASCII `GET`, one space, a document address, and CRLF. The
address is a nonempty sequence of printable ASCII characters excluding space.
It can be a path with a query or a full address for a gateway. The encoder
accepts an address and returns bytes. The request parser accepts CRLF or LF
and returns the address verbatim. It rejects extra words and additional lines
to isolate this profile. The CERN description allowed a server to ignore
extra words or use them to dispatch to the full HTTP protocol.

The response is an ASCII byte stream terminated by the server closing its
connection. The decoder takes the completed byte stream and preserves line
endings. An empty stream yields empty text. HTTP-looking prefixes and
human-readable error pages are body content in this profile. A later socket
layer can feed the decoder after EOF. Request formatting, request parsing,
response decoding, and HTML parsing can already be exercised in memory.

## HTML Tags profile

Parse a character string into an ordered list of text and elements. An element
has a lowercase `tag`, an `attributes` mapping, and ordered `children`. Text
is a string. CRLF becomes LF, and adjacent text in the same parent coalesces.
Tag and attribute names use ASCII case folding. Whitespace in tags is ASCII
space, tab, LF, CR, or form feed. Anchor values accept single quotes, double
quotes, and unquoted values. The first `name`, `href`, or `type` wins if
repeated. A valueless recognized attribute maps to null. Values preserve
case. Decode the archived `&lt;`, `&gt;`, and `&amp;` once in regular text and
anchor values. Other references remain literal.

| Markup | Tree behavior |
| --- | --- |
| `title`, `address`, `a`, `h1` through `h6` | Containers. A new heading closes an open heading. |
| `ul`, `menu`, `dir`, `li` | List containers and items. A new item implies the previous item's end within the same list. |
| `dl`, `dt`, `dd` | A new term or definition implies the end of the previous term or definition within the same glossary. |
| `p`, `isindex` | Empty markers. In particular, `<p>` indicates a paragraph break in the archived description. |
| `xmp`, `listing` | Literal text through the matching end tag, ignoring ASCII case. |
| `plaintext` | Literal text through EOF, including any apparent end tag. |
| `nextid`, unknown well-formed tags, and SGML framing tags such as `header` and `body` | Skip the tag and keep its contents. |

Use a stack for containers. A recognized end tag closes through the closest
matching open element. Ignore unmatched end tags, and close open elements at
EOF. An orphan item or glossary entry is retained in the current container.
Discard `<!--` comments through `-->` or EOF. Preserve malformed tag text.
Recognized raw-text end tags have the exact spelling `</name>` apart from
case. These rules make underspecified recovery deterministic for ports. The
profile describes a teaching implementation based on the archive. Features
such as general DTD processing, external entities, URL resolution, rendering,
and modern HTML error recovery belong to other profiles or layers.

## How SGML, HTML, and XML connect

SGML is a system for defining markup languages. Its document type definitions
describe permitted elements, content, entity declarations, and whether tags
can be omitted. The CERN documents explain HTML in those terms. The early
list `<UL><LI>One<LI>Two</UL>` supplies two list items even though both item
end tags are omitted. In XML 1.0 the corresponding XML elements each need an
explicit end tag, and case must match. XML is a later restricted form of
SGML. HTML was already in use when XML 1.0 became a W3C Recommendation in 1998.

- [CERN's archived SGML notes](https://www.w3.org/History/19921103-hypertext/hypertext/WWW/MarkUp/SGML.html)
- [XML 1.0](https://www.w3.org/TR/1998/REC-xml-19980210)
- [Repository XML parser](XML01-xml-parser.md)

## Evolution and ports

The Python reference reads the same [portable vectors](fixtures/historical-web-v1/vectors.json)
that future ports can replay. Compare emitted request bytes and full trees.
Use hex for bytes and JSON strings for text. Add a new profile identifier when
later historical behavior differs. Replay earlier profiles as regression
checks in each supported language with a library target.

Grow the two histories independently. On the markup side, the next profiles
can study the SGML declaration and DTD in [RFC 1866](https://www.rfc-editor.org/rfc/rfc1866)
and use [the repository's Mosaic-era specs](TE04-html1.0-lexer.md) for the
intermediate browser vocabulary. Compare the same examples with the
[XML parser](XML01-xml-parser.md) and eventually [Venture's WHATWG builder](BR03-html-tree-builder.md).
On the wire side, add the HTTP/1.0 request/status lines and headers described
in [RFC 1945](https://www.rfc-editor.org/rfc/rfc1945) and the repository's
[HTTP/1.0 parser spec](NET04-http1.0-parser.md), then proceed to HTTP/1.1.
RFC 1945 is Informational and records the HTTP/1.0 protocol already in use.
