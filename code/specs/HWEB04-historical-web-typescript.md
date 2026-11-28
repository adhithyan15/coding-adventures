# HWEB04 — TypeScript profiles for the early Web

This package ports HWEB00's `http-0.9` and `html-tags-1992` profiles to
TypeScript using only the standard runtime. It performs no network, filesystem,
process, URL-resolution, or rendering operations. Malformed markup follows the
deterministic HWEB00 recovery rules; HTTP boundaries return explicit tagged
errors.

## Public API

`Result<T, E>` is a discriminated union with `{ ok: true, value: T }` and
`{ ok: false, error: E }`. `Http09Address.create(value: string)` returns a
validated address or an `Http09RequestError` tagged `invalid-address`.
`encodeHttp09Request(address: Http09Address): Uint8Array` emits `GET `, the
printable-ASCII address, and CRLF. `parseHttp09Request(wire: Uint8Array)`
accepts exactly one request line terminated by CRLF or LF and returns the
validated address, or an `Http09RequestError` tagged `invalid-request`.
`decodeHttp09Response(body: Uint8Array)` preserves the complete ASCII response
and its line endings, returning an `Http09ResponseError` tagged `non-ascii` if
any byte exceeds `0x7f`.

`parseHtmlTags1992(source: string)` returns a `Result<HtmlDocument,
HtmlParseError>`. Its errors are tagged `invalid-source` for a non-string call
from untyped JavaScript or `invalid-tree-state` for an impossible arena
invariant. The document is a flat arena: `nodes: HtmlNode[]` and
`roots: NodeId[]` contain integer references into that same array. A node is
either `{ kind: "text", text }` or `{ kind: "element", tag, attributes,
children }`; attributes map the recognized anchor names to strings or `null`
for valueless attributes. This representation avoids recursive build and drop
for deeply nested input. Node indices remain stable for the lifetime of the
document.

## Scanner and source domain

The request codec reads and writes `Uint8Array` without converting an entire
wire through a spread call. It admits only nonempty printable-ASCII addresses
(`0x21` through `0x7e`). The HTML scanner consumes JavaScript strings in their
native UTF-16 code-unit domain. It preserves text code units, including
supplementary pairs and lone surrogates, and makes no Unicode scalar-value or
normalization claim. Only tag and attribute names use ASCII case folding.

The HTML scanner normalizes CRLF across the complete source, including quoted
attribute values. It decodes `&lt;`, `&gt;`, and `&amp;` once in regular text and
anchor values; other references remain literal. Anchor values accept
single-quoted, double-quoted, and unquoted forms, the first `name`, `href`, or
`type` wins, and already parsed values survive a malformed later attribute.
Adjacent text coalesces through linear fragment accumulation. Open-element
scope uses an explicit stack with tag-indexed positions; all source-dependent
depth work is iterative.

The parser follows HWEB00's tag table: containers, `p`/`isindex` empty
markers, `xmp`/`listing` raw text, `plaintext` through EOF, local list and
glossary omission, ignored unmatched end tags, skipped unknown tags, discarded
comments, and preserved non-tag text. It is not a general SGML or HTML5 parser.

## Historical comparisons

The November 1992 archive describes an SGML-era HTML vocabulary with omitted
`LI`, `DT`, and `DD` end tags. The source `<UL><LI>One<LI>Two</UL>` therefore
has two list items in this profile. XML 1.0 requires explicit end tags and
case-sensitive matching, so that exact source is not well-formed XML; compare
the repository's [XML parser](XML01-xml-parser.md). Venture's current browser
parser implements the WHATWG HTML tokenizer and modern tree-construction and
error-recovery rules, a much larger model than this archived teaching profile.

## Conformance boundary

The package replays shared HWEB00 vectors, including invalid wire and malformed
markup examples. Tests of the public exports are separate from this contract.
Finite replay, property checks, or fuzz runs establish only their stated
bounds; they do not prove behavior for every JavaScript string or execution
environment.
