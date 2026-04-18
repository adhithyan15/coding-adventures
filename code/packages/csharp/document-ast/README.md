# document-ast

Format-agnostic intermediate representation for structured documents in pure C#.

## What It Includes

- Root and block nodes such as document, heading, paragraph, code block, blockquote, list, and thematic break
- Inline nodes such as text, emphasis, strong, strikethrough, code span, link, image, autolink, and line breaks
- GFM-oriented extension nodes for task items and tables
- No parser or renderer logic; this package is the shared IR for those layers

## Example

```csharp
using CodingAdventures.DocumentAst;

var doc = new DocumentNode(
    new IBlockNode[]
    {
        new HeadingNode(1, new IInlineNode[] { new TextNode("Hello") }),
        new ParagraphNode(new IInlineNode[] { new TextNode("World") }),
    });

Console.WriteLine(doc.Type); // document
```

## Development

```bash
bash BUILD
```
