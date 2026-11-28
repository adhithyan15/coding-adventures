# CodingAdventures.HistoricalWeb (C#)

A standard-library-only .NET 9 library for the CERN 1991 HTTP/0.9 wire profile
and the archived HTML Tags vocabulary dated November 3, 1992. Both parsers work
entirely in memory and accept no URLs, files, sockets, or browser context.

## Usage

```csharp
using CodingAdventures.HistoricalWeb;
using System.Text;

byte[] request = Http09.EncodeRequest(Encoding.ASCII.GetBytes("/index.html"));
byte[] address = Http09.ParseRequest(request);
string body = Http09.DecodeResponse(Encoding.ASCII.GetBytes("<TITLE>Hello</TITLE>"));

HtmlDocument page = HtmlTags1992.Parse("<TITLE>Early &amp; Web</TITLE>");
HtmlNode root = page.NodeAt(page.FirstRoot);
Console.WriteLine(root.Tag); // title
```

HTTP methods operate on length-delimited byte spans. The caller keeps those
bytes stable while a call runs; successful outputs are new arrays or strings.
`HistoricalWebException` supplies a typed reason and the first source offset.

The HTML result owns a flat arena of readonly node and attribute records. Links
between parent, child, and sibling nodes are integer indexes, so deeply nested
source does not create recursive object ownership. Text and attribute strings
are immutable .NET strings. A null attribute value means a valueless recognized
attribute; `""` is an explicitly empty value.

## What the archived profile demonstrates

The November 1992 source is a list of tag names and entities, not a complete
grammar or today's browser recovery algorithm. The teaching parser recognizes
the containers, list and glossary elements, empty paragraph/index markers,
anchors, comments, and raw-text modes documented by HWEB00. It models omitted
list-item, glossary-entry, and heading ends; retains malformed `<` candidates
as text; and leaves unknown references literal.

The output is a useful comparison with XML in the repository: XML requires
explicit, case-sensitive matching end tags. It also differs from the current
WHATWG tree builder, which includes modern tokenization and recovery rules.
This package does no layout or rendering, URL resolution, DTD processing,
entity declarations, resource lookup, or network and filesystem work.

See [HWEB00](../../../specs/HWEB00-historical-web.md) for shared profile
semantics and [HWEB10](../../../specs/HWEB10-historical-web-csharp.md) for this
port's API, Unicode, ownership, and error-offset contract. Compare with the
repository [XML parser](../../../specs/XML01-xml-parser.md) and
[WHATWG-oriented tree builder](../../../specs/BR03-html-tree-builder.md).
