# commonmark-parser

Pure C# CommonMark parser that converts Markdown into the shared document AST.

## What It Includes

- Block parsing for headings, paragraphs, blockquotes, lists, thematic breaks, fenced code blocks, and simple raw HTML blocks
- Inline parsing for emphasis, strong, code spans, links, images, autolinks, line breaks, and HTML entity decoding
- A public `MarkdownParser` entry point that `gfm-parser` can extend without crossing language boundaries

## Example

```csharp
using CodingAdventures.CommonmarkParser;

var doc = CommonmarkParser.Parse("# Hello\n\nWorld *in* Markdown.\n");
Console.WriteLine(doc.Children.Count); // 2
```

## Development

```bash
bash BUILD
```
