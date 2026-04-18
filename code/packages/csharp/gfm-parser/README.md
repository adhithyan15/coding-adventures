# gfm-parser

Pure C# GFM parser that extends the C# CommonMark parser with GFM-specific constructs.

## What It Includes

- Task list item parsing
- Pipe table parsing with column alignment
- Strikethrough inline parsing
- No external Markdown libraries and no cross-language bridging

## Example

```csharp
using CodingAdventures.GfmParser;

var doc = GfmParser.Parse("- [x] done\n");
Console.WriteLine(doc.Children.Count); // 1
```

## Development

```bash
bash BUILD
```
