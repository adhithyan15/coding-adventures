# HWEB05 — Ruby profiles for the early Web

This package implements HWEB00's `http-0.9` and `html-tags-1992` profiles in
Ruby with only the standard runtime. It performs no network, filesystem,
process, URL-resolution, or rendering operations. Expected input failures are
reported as `Result` values carrying a small error symbol; malformed historical
markup follows HWEB00's deterministic recovery rules.

## Public API

Load the package with `require "coding_adventures/historical_web"`. Its public
constants and functions live in `CodingAdventures::HistoricalWeb`.

`Result#ok?` reports success; `#value` contains the success value or `nil`, and
`#error` contains the failure symbol or `nil`. `Http09Address.create(value)`
returns a `Result` with either a validated address or `:invalid_address`.
Addresses are copied to a frozen `ASCII-8BIT` string of printable ASCII bytes.
`encode_http09_request(address)` returns a `Result` with a fresh binary string
containing `GET `, the address, and CRLF; if the object's stored value is
missing or fails validation, the error is `:invalid_address`. It rechecks a
frozen snapshot of the address object's stored value, including for objects
created with `allocate`. The successful wire string is frozen.
`parse_http09_request(wire)` treats a Ruby `String` as uninterpreted octets,
accepts exactly one request line terminated by CRLF or LF, and returns an
address or `:invalid_request`. `decode_http09_response(body)` checks each byte
for ASCII and returns a fresh `US-ASCII` string with exactly the original
octets; a byte above `0x7f` yields `:non_ascii`. For example, bytes `41 00`
tagged UTF-16LE decode to US-ASCII `A` followed by NUL.

`parse_html_tags_1992(source)` accepts only valid UTF-8 and valid US-ASCII
strings. It returns a `Result` containing an `HtmlDocument` or `:invalid_source`
for another encoding or invalid byte sequence. An impossible internal arena
invariant is reported as `:invalid_tree_state`.

The returned document has flat `nodes` and `roots` arrays. `NodeId` values are
integer indices into the same node array. `HtmlTextNode` has `kind: :text` and
`text`; `HtmlElementNode` has `kind: :element`, `tag`, string-keyed
`attributes`, and ordered `children` indices. A `nil` attribute value means
that a recognized anchor attribute was present without a value. The arena,
node records, child arrays, attribute hashes, and attribute values are frozen.
No node
recursively owns another node, so building and dropping deeply nested input
does not recurse through the tree.

## Scanner and recovery

The HTTP codec scans bytes and constructs output without converting or passing
an entire string as positional arguments. The HTML scanner indexes by byte
offsets after admitting valid UTF-8 or US-ASCII. It folds only ASCII letters
in tag names, attribute names, and raw-text end tags. CRLF becomes LF across
the whole source, including quoted attribute values. The archived `&lt;`, `&gt;`, and
`&amp;` references decode once in ordinary text and anchor values; other
references remain literal. Adjacent text fragments coalesce in linear time.

Anchor attributes accept single-quoted, double-quoted, and unquoted values; the
first `name`, `href`, or `type` occurrence wins, and prior attributes survive
a malformed later sequence. Open-element lookup uses a stack with tag-indexed
positions. Containers, empty `p`/`isindex` markers, `xmp`/`listing` raw text,
`plaintext` through EOF, local list/glossary omission, ignored unmatched end
tags, skipped unknown tags, discarded comments, and preserved non-tag text
follow HWEB00. The package is not a general SGML or HTML5 parser.

## Historical comparisons

The November 1992 archive describes SGML-era HTML with omitted list and
glossary end tags. Thus `<UL><LI>One<LI>Two</UL>` has two list items in this
profile. XML 1.0 requires explicit end tags and case-sensitive matching, so
that source is not well-formed XML; compare the repository's
[XML parser](XML01-xml-parser.md). Venture's current browser parser implements
the WHATWG HTML tokenizer and modern tree construction and error recovery,
which is a much larger model than this historical teaching profile.

## Conformance boundary

The package replays shared HWEB00 vectors, including negative wire and markup
cases. Property and fuzz results apply to their recorded bounds; they do not
establish behavior for every Ruby encoding or every possible input length.
