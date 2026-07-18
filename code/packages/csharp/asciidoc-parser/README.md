# AsciiDoc Parser (C#)

Pure C# parser for a portable AsciiDoc subset, producing the shared
`CodingAdventures.DocumentAst` representation.

```csharp
using CodingAdventures.AsciidocParser;

var document = AsciidocParser.Parse("= Title\n\nHello *world*.");
var inline = AsciidocParser.ParseInline("link:https://example.com[Example]");
```

The block parser supports headings, paragraphs, thematic breaks, source and
literal blocks, passthrough HTML, recursive quote blocks, comments, and nested
ordered or unordered lists. The inline parser supports strong and emphasized
text, code spans, links, images, cross-references, URLs, and hard or soft line
breaks.
