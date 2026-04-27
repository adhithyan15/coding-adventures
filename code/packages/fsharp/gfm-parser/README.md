# gfm-parser

Pure F# GFM parser that extends the F# CommonMark parser with GFM-specific constructs.

## What It Includes

- Task list item parsing
- Pipe table parsing with column alignment
- Strikethrough inline parsing
- No external Markdown libraries and no cross-language bridging

## Example

```fsharp
open CodingAdventures.GfmParser.FSharp

let doc = GfmParser.Parse "- [x] done\n"
printfn "%d" doc.Children.Length
```

## Development

```bash
bash BUILD
```
