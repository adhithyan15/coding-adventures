# HWEB11 — Historical Web profiles in F#

This port implements HWEB00 `http-0.9` and `html-tags-1992` as an independent,
standard-library-only .NET 9 library. It does not implement the later HTML 2.0
paragraph slice from HWEB06. Public API lives in namespace
`CodingAdventures.HistoricalWeb.FSharp`, assembly
`CodingAdventures.HistoricalWeb.FSharp`, package
`code/packages/fsharp/historical-web`.

## API, diagnostics, and ownership

The HTTP module consumes explicit read-only byte spans and never stores those
spans:

```fsharp
module Http09 =
    val encodeRequest: ReadOnlySpan<byte> -> byte array
    val parseRequest: ReadOnlySpan<byte> -> byte array
    val decodeResponse: ReadOnlySpan<byte> -> string

module HtmlTags1992 =
    val parse: string -> HtmlDocument
```

Inputs must remain stable for the duration of a call. Successful HTTP results
own fresh byte arrays or strings. Output length arithmetic is checked before
the request array is allocated; representational overflow raises the standard
`OverflowException`.

`HistoricalWebException` derives from `FormatException` and exposes
`Kind: HistoricalWebErrorKind` and `Offset: int option`. HTTP offsets are byte
indexes; invalid HTML Unicode offsets are UTF-16 code-unit indexes. Error kinds
are `InvalidAddress`, `InvalidRequest`, `InvalidResponse`, and `InvalidUnicode`.
An empty encoded address has no offset. Request diagnostics check the `GET `
prefix first: a mismatch reports its first differing byte, a short prefix its
input length. Once the prefix matches, missing terminal LF reports input
length; with framing present, an empty address reports byte 4 and an invalid
address byte reports its first index. Non-ASCII response reports its first
byte above `0x7f`. An unpaired high or low surrogate reports its code-unit
index, including a high surrogate at EOF.

HTTP request encoding accepts a nonempty printable ASCII address (`0x21..0x7e`)
and emits `GET SP address CRLF`. Parsing accepts exactly one such request with
CRLF or LF framing and returns a fresh copy of the address. The response
decoder accepts ASCII and preserves every byte as text; an empty body returns
the empty string.

HTML source is a .NET UTF-16 string. Valid Unicode scalar sequences, including
NUL, are admitted; unpaired surrogates raise `InvalidUnicode`. A null source
raises `ArgumentNullException`. The returned document owns flat node and
attribute arrays. Immutable records link nodes by parent, first-child, and
next-sibling indexes, so parsing and teardown do not recurse with document
depth. The public collections implement only `IReadOnlyList` operations and do
not expose the backing arrays. `NodeAt` and `AttributeAt` throw
`ArgumentOutOfRangeException` for invalid indexes. `FirstRoot` is available
both as a property and through indexed access to `Nodes`; it is `-1` when there
are no roots. A first attribute index of `-1` and count of zero means that the
element has no recognized attributes. `HtmlAttribute.Value = None` means the
attribute is valueless, while `Some ""` is explicitly empty.

## HTML Tags 1992 behavior

`HtmlTags1992.parse` follows the Python HWEB00 reference. It normalizes CRLF to
LF, coalesces adjacent text, uses ASCII-only name folding, accepts the five
ASCII tag whitespace characters, and decodes `&lt;`, `&gt;`, and `&amp;` once
in ordinary text and anchor values. Anchor values may be single-quoted,
double-quoted, or unquoted. The first recognized `name`, `href`, or `type`
attribute wins; malformed attribute sequences preserve earlier attributes.

Containers include titles, addresses, anchors, headings, lists, and glossaries.
Paragraph and `isindex` tags are empty marker nodes. A new heading closes the
previous heading. A new `li` closes the newest `li` newer than the newest open
`ul`, `menu`, or `dir`; without a list boundary, it closes the newest orphan
`li`. A new `dt` or `dd` closes the newest entry newer than the newest `dl`, or
the newest orphan entry when no `dl` is open. A matching explicit end tag
closes through the nearest same-name element; unmatched ends are ignored and
open scopes remain until EOF. `xmp` and `listing` preserve literal source up to
their exact ASCII-case-insensitive end tag. `plaintext` preserves through EOF.
Comments disappear, unknown well-formed tags are skipped, and malformed `<`
candidates remain text. HWEB00 defines the complete shared profile.

This archived vocabulary is not a current browser parser. The implementation
does no rendering, DTD/entity-declaration processing, URL resolution, network
or filesystem I/O, or external-resource lookup. Its omitted end tags and
case-insensitive names make a useful comparison with XML's explicit,
case-sensitive tags and the repository's current WHATWG-oriented tree builder.
