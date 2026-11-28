# HWEB06 — HTML 2.0 paragraph slice (1995)

This profile adds a deliberately small, separately versioned HTML 2.0 slice to
the historical-web reference. Its identifier is `html2-paragraphs-1995`; its
entry point is:

```python
parse_html2_paragraphs_1995(source: str) -> list[Node]
```

It returns the existing `historical_web.Element | str` tree representation.
Each returned element is `p` or `h1`, has an empty attributes mapping, and
contains only text children. The selected API does not alter
`parse_html_tags_1992` or the `html-tags-1992` profile.

## Source and syntax

The input must be a Python `str`; other values raise `TypeError`. CRLF is
normalized to LF across the complete source before scanning. The scanner is an
in-memory, iterative, linear pass. It recognizes only the four exact simple
tag forms `<P>`, `</P>`, `<H1>`, and `</H1>`, with ASCII case-insensitive tag
letters. No whitespace, attributes, or self-closing syntax is allowed inside
these tags. There are no declarations, comments, entity declarations, or
inline elements in this slice. Any other `<...`
construct, malformed tag candidate, or unsupported markup raises
`ValueError` rather than being guessed at or silently discarded.

Outside an element, only ASCII space, tab, LF, CR, and form feed are accepted;
this separator whitespace is discarded. Non-whitespace text outside a `p` or
`h1` raises `ValueError`. Inside an element, text is retained in source order.
The parser does not trim, collapse, or otherwise normalize text whitespace.
CRLF normalization is a source-preservation convention for this API, not
SGML record-start/record-end inference. It also does not implement HTML 2.0's
display-time whitespace treatment described in RFC 1866 sections 4.2.2 and 6.
The archived `&lt;`, `&gt;`, and `&amp;` references decode once in text;
unknown references remain literal, matching RFC 1866 section 4.2.1's
treatment of undeclared references. Decoded characters are text and are not
scanned again as markup.

## Paragraph and heading transitions

`P` has a required start tag and an optional end tag in the RFC 1866 DTD
(`<!ELEMENT P - O (%text)*>`). An open paragraph closes on `</P>`, the next
`<P>`, the next `<H1>`, or EOF. A new paragraph always creates a new `p`
element, including an empty one. An unmatched `</P>` is invalid. Starting H1
while P is open closes the P because the RFC 1866 section 9.1 DTD excludes
headings from P's `%text` content model and permits headings alongside P in
body flow.

`H1` requires both start and end tags in the DTD
(`<!ELEMENT H1 - - (%text;)*>`), so EOF with an open heading is a `ValueError`.
An H1 may be empty. A paragraph cannot start inside an H1, and an H1 cannot
nest in another H1. An unmatched `</H1>` is invalid.

The parser rejects nested and other element markup rather than accepting
RFC 1866 phrase content; this first slice models only plain text-bearing
paragraphs and level-one headings. It does not implement the full HTML 2.0
DTD, other heading levels, lists, inline markup, SGML minimization, entity
catalog, rendering, or WHATWG HTML parsing.

## Historical context

RFC 1866 (November 1995), section 5.5.1, defines paragraph text and explains
that a paragraph end tag may be omitted; its example includes successive
paragraphs with omitted end tags. The section 9.1 DTD records `P` as
`<!ELEMENT P - O (%text)*>`. Section 5.4 describes headings; the section 9.1
DTD records headings as `<!ELEMENT ( %heading ) - - (%text;)*>` and defines
`%heading` as H1 through H6, making H1's end tag required. This profile narrows
that vocabulary to H1 and excludes phrase markup from the DTD content model.
See [RFC 1866](https://www.rfc-editor.org/rfc/rfc1866).

The distinction from [HWEB00](HWEB00-historical-web.md) is intentional: the
November 1992 archive calls `<P>` an empty paragraph marker, while HTML 2.0
defines P as text-bearing. The repository's XML parser remains a separate
case-sensitive, explicitly closed XML grammar. Current WHATWG HTML adds a
larger tokenizer and recovery model; none of those behaviors are implied by
this profile.
