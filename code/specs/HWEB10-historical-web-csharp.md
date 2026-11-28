# HWEB10 — Historical Web profiles in C#

This port implements the HWEB00 `http-0.9` and `html-tags-1992` profiles as a
standard-library-only .NET 9 library. It does not implement HWEB06's later HTML
2.0 paragraph slice. The public namespace is
`CodingAdventures.HistoricalWeb`; the package is
`code/packages/csharp/historical-web`.

## Public API and ownership

The HTTP API uses explicit byte spans so that wire framing never depends on
string encoding or NUL termination:

```csharp
public static class Http09
{
    public static byte[] EncodeRequest(ReadOnlySpan<byte> address);
    public static byte[] ParseRequest(ReadOnlySpan<byte> request);
    public static string DecodeResponse(ReadOnlySpan<byte> body);
}

public static class HtmlTags1992
{
    public static HtmlDocument Parse(string source);
}
```

`HistoricalWebException` derives from `FormatException` and exposes a typed
`HistoricalWebErrorKind Kind` plus nullable `int Offset`. HTTP offsets count
input bytes. HTML Unicode validation offsets count UTF-16 code units. Kinds are
`InvalidAddress`, `InvalidRequest`, `InvalidResponse`, and `InvalidUnicode`.
An empty encoded address has no offset. Request diagnostics check the `GET `
prefix first: mismatch reports its first differing byte, and a short prefix
reports the input length. After a matching prefix, missing terminal LF reports
the input length; with framing present, an empty address reports byte 4 and a
nonprintable address reports its first byte. Response errors report the first
byte above `0x7f`. An unpaired high or low surrogate reports its UTF-16 code
unit index, including a high surrogate at the end of input.

`ReadOnlySpan<byte>` arguments are consumed synchronously and never retained.
Successful outputs own their bytes or text. HTTP framing allocation lengths
are formed with checked arithmetic before allocation, so overflow raises a
standard `OverflowException`, distinct from malformed profile input.

The HTTP encoder accepts only a nonempty address of printable ASCII bytes
`0x21..0x7e` and returns `GET SP address CRLF`. The parser accepts exactly one
such request, terminated by CRLF or LF, and returns the address bytes. It
rejects extra words and lines. The response decoder accepts ASCII bytes and
returns the corresponding string unchanged; an empty body returns an empty
string.

The HTML input is a .NET string interpreted as UTF-16. Valid Unicode scalar
sequences, including NUL, are accepted. Unpaired UTF-16 surrogates are rejected
with `InvalidUnicode` at their code-unit offset. Strings returned in the tree
are immutable .NET strings, and the document owns flat arrays of immutable
nodes and attributes. It exposes read-only wrappers, never writable backing
arrays, root/node/attribute counts, a first-root index (or `-1`), and
bounds-checked indexed accessors. A
node has `Kind`, `ParentIndex`, `FirstChildIndex`, `NextSiblingIndex`, `Tag`,
`Text`, `FirstAttributeIndex`, and `AttributeCount`. A valueless recognized
anchor attribute has `Value == null`; an explicitly empty value is `""`.

The arena uses integer indexes rather than recursively nested node ownership,
so parsing and document teardown remain iterative for deeply nested source.
Each returned value is independent of the input string. The public API has no
caller-supplied maximum size; .NET allocation and array-size failures propagate
as standard runtime exceptions, distinct from invalid profile input. A parse
failure never returns a partial document.

## HTML Tags 1992 behavior

`HtmlTags1992.Parse` mirrors the Python HWEB00 reference: normalize CRLF to LF,
coalesce adjacent text per parent, apply ASCII-only case folding to tag and
attribute names, preserve values, and decode only `&lt;`, `&gt;`, and `&amp;`
once in regular text and anchor values. Tag whitespace is ASCII space, tab,
LF, CR, or form feed. Anchor values accept single-quoted, double-quoted, and
unquoted forms. The first `name`, `href`, and `type` attribute wins; malformed
attribute sequences retain earlier recognized attributes. Passing a null source
throws `ArgumentNullException`.

Containers are `title`, `address`, `a`, headings, list containers/items, and
glossary containers/entries. Paragraph and `isindex` tags create empty marker
elements. A new heading closes the previous open heading. A new `li` closes
the previous open `li` when it is newer than the newest open `ul`, `menu`, or
`dir` boundary; when no such boundary is open, it closes the newest orphan
`li`. A new `dt`/`dd` closes the previous entry newer than the newest open
`dl`; without an open `dl`, it closes the newest orphan `dt` or `dd`. Matching end tags close through
the nearest matching scope; unmatched ends are ignored and remaining scopes
close at EOF. `xmp` and `listing` preserve literal text through their exact
ASCII-case-insensitive end tag; `plaintext` preserves text through EOF.
Comments disappear, unknown well-formed tags are skipped, and malformed `<`
candidates remain text. See [HWEB00](HWEB00-historical-web.md) for the complete
portable profile contract.

This is a teaching model of the archived tag list, not a current browser
parser. It performs no rendering, URL resolution, sockets, file access, DTD
processing, entity declarations, or external-resource lookup. Compare its
omitted tag ends and case-insensitive names with the repository XML parser,
which requires explicit, case-sensitive matching tags, and with the modern
WHATWG-oriented tree builder.
