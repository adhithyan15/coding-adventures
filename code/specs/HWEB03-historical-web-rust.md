# HWEB03 — Rust profile for the early Web

This crate ports the `http-0.9` and `html-tags-1992` profiles in
[HWEB00](HWEB00-historical-web.md) to Rust. It uses only the standard library.
It performs no network, filesystem, process, URL resolution, or rendering
operations. The public API keeps the historical profile explicit and returns
typed errors for invalid wire values; underspecified markup recovery remains
deterministic as defined by HWEB00.

## Public API

`Http09Address::new(String) -> Result<Http09Address, Http09RequestError>`
constructs a nonempty printable-ASCII address with no spaces.
`encode_http09_request(&Http09Address) -> Vec<u8>` emits `GET `, the address,
and CRLF. `parse_http09_request(&[u8]) -> Result<Http09Address,
Http09RequestError>` accepts exactly one `GET` line terminated by CRLF or LF.
`decode_http09_response(&[u8]) -> Result<String, Http09ResponseError>` decodes
the complete ASCII body without changing line endings; empty input yields an
empty string.

`parse_html_tags_1992(&str) -> Result<HtmlDocument, HtmlParseError>` returns an
ordered tree in a flat arena. `HtmlDocument` exposes `nodes: Vec<HtmlNode>` and
`roots: Vec<NodeId>`; each `NodeId` is an index into `nodes`. `HtmlNodeKind` is
either `Text(String)` or `Element { tag, attributes, children }`. Attributes
are an ordered `Vec<HtmlAttribute>` with `name` and `Option<String>` value;
`None` means the recognized attribute was present without a value. The arena
uses integer child references rather than recursively owned child objects, so
building, traversing, and dropping deeply nested documents do not recurse
through the tree. Node indices are stable for the document's lifetime.

## Scanner and recovery

The implementation scans source bytes and folds only ASCII tag and attribute
letters. CRLF becomes LF; adjacent text is coalesced in linear time. It
recognizes the archived three entities (`lt`, `gt`, `amp`) once in regular text
and anchor values. It preserves other references, attribute value case, and
UTF-8 source bytes. Anchor attributes `name`, `href`, and `type` accept
single-quoted, double-quoted, and unquoted values; the first occurrence wins,
and malformed later attributes do not erase earlier parsed values.

Containers, empty markers, raw-text elements, comments, omitted list and
glossary endings, unmatched closing tags, unknown tags, declarations, and
plaintext follow HWEB00's table and recovery rules. Open-element lookup is
indexed by tag, and parser work over nesting is iterative. Malformed profile
markup is recovered; `HtmlParseError` is reserved for an impossible internal
state rather than ordinary malformed input.

## Historical comparisons

The November 1992 archive documents an SGML-era HTML vocabulary with omitted
`LI`, `DT`, and `DD` end tags. XML 1.0 requires explicit end tags and
case-sensitive matching, so the archived list example
`<UL><LI>One<LI>Two</UL>` is not well-formed XML. See the repository's
[XML parser](XML01-xml-parser.md) for that later, stricter tree model.
Venture's current browser parser targets WHATWG HTML and its modern tokenizer,
tree-construction, and error-recovery rules; this crate intentionally preserves
the much smaller 1992 teaching profile and must not be treated as an HTML5
parser.

## Conformance and evidence limits

The crate replays the portable HWEB-00 vectors, including negative HTTP and
markup cases, and checks its exports with package-level Rust tests. Deep nesting
regressions cover iterative parsing and arena ownership. A separate assurance
record must state bounds, controls, and any solver or fuzz evidence; example
tests alone do not establish unbounded safety or a source-level proof.
