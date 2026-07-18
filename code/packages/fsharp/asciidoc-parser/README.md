# AsciiDoc Parser (F#)

Pure F# parser for a portable AsciiDoc subset, producing the shared
`CodingAdventures.DocumentAst.FSharp` representation.

```fsharp
open CodingAdventures.AsciidocParser

let document = AsciidocParser.parse "= Title\n\nHello *world*."
let inlineNodes = AsciidocParser.parseInline "link:https://example.com[Example]"
```

The block parser supports headings, paragraphs, thematic breaks, source and
literal blocks, passthrough HTML, recursive quote blocks, comments, and nested
ordered or unordered lists. The inline parser supports strong and emphasized
text, code spans, links, images, cross-references, URLs, and hard or soft line
breaks.
