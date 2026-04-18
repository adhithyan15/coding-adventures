# commonmark-parser

Pure F# CommonMark parser that converts Markdown into the shared document AST.

## What It Includes

- Block parsing for headings, paragraphs, blockquotes, lists, thematic breaks, fenced code blocks, and simple raw HTML blocks
- Inline parsing for emphasis, strong, code spans, links, images, autolinks, line breaks, and HTML entity decoding
- A public `MarkdownParser` entry point that `gfm-parser` can extend without crossing language boundaries

## Example

```fsharp
open CodingAdventures.CommonmarkParser.FSharp

let doc = CommonmarkParser.Parse "# Hello\n\nWorld *in* Markdown.\n"
printfn "%d" doc.Children.Length
```

## Development

```bash
bash BUILD
```
