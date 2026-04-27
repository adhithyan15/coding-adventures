# document-ast

Format-agnostic intermediate representation for structured documents in pure F#.

## What It Includes

- Root and block nodes such as document, heading, paragraph, code block, blockquote, list, and thematic break
- Inline nodes such as text, emphasis, strong, strikethrough, code span, link, image, autolink, and line breaks
- GFM-oriented extension nodes for task items and tables
- No parser or renderer logic; this package is the shared IR for those layers

## Example

```fsharp
open CodingAdventures.DocumentAst.FSharp

let doc =
    { Children =
        [ HeadingNode (1, [ TextNode "Hello" ])
          ParagraphNode [ TextNode "World" ] ] }

printfn "%s" (BlockNode.typeName doc.Children[0]) // heading
```

## Development

```bash
bash BUILD
```
