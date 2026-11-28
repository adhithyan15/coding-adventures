# CodingAdventures.HistoricalWeb.FSharp

A standard-library-only .NET 9 library for the CERN 1991 HTTP/0.9 wire
profile and the archived HTML Tags vocabulary dated November 3, 1992. It does
all work in memory and does not open files, sockets, or URLs.

## Usage

```fsharp
open System
open CodingAdventures.HistoricalWeb.FSharp

let request = Http09.encodeRequest (ReadOnlySpan<byte>([| 47uy; 105uy; 110uy; 100uy; 101uy; 120uy |]))
let address = Http09.parseRequest (ReadOnlySpan<byte>(request))
let body = Http09.decodeResponse (ReadOnlySpan<byte>([| 60uy; 84uy; 73uy; 84uy; 76uy; 69uy; 62uy |]))
let page = HtmlTags1992.parse "<TITLE>Early &amp; Web</TITLE>"
let title = page.NodeAt(page.FirstRoot)
// title.Tag is "title"; the flat tree owns every string and index.
```

The HTTP modules use explicit spans of bytes, not runtime string encodings.
Inputs must remain stable until each synchronous call returns. Successful
request/address results are fresh arrays; response and HTML results are
immutable .NET strings. `HistoricalWebException` carries a typed kind and a
source offset.

HTML documents store readonly struct records in a flat arena. Parent, first
child, and next sibling are indexes, so deep markup does not produce
recursively owned F# values. Public `IReadOnlyList` views do not expose the
private arrays. `HtmlAttribute.Value = None` means valueless; `Some ""` means
explicitly empty.

## Archived behavior

The 1992 archive lists tag and entity spellings rather than prescribing a
modern browser tree builder. This parser models the HWEB00 teaching rules:
ASCII-only name folding, the original small entity set, omitted heading/list/
glossary ends, raw `xmp`/`listing`/`plaintext`, comments, and deterministic
malformed-tag recovery. Unknown tags are skipped while their content remains.

The model does not render, resolve URLs, process declarations or external
entities, fetch resources, or use the network or filesystem. It is useful to
contrast its omitted ends and case-insensitive names with XML's explicit,
case-sensitive end tags and with the repository's modern WHATWG-oriented
builder.

See [HWEB00](../../../specs/HWEB00-historical-web.md) and
[HWEB11](../../../specs/HWEB11-historical-web-fsharp.md) for complete profile
and API contracts. Compare with the [XML parser](../../../specs/XML01-xml-parser.md)
and [WHATWG-oriented tree builder](../../../specs/BR03-html-tree-builder.md).
