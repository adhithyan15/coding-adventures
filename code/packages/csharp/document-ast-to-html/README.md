# document-ast-to-html

Pure C# rendering from the shared Document AST into an HTML fragment.

This package is the back end of the document pipeline: parsers produce typed
`DocumentNode` values, and this renderer converts those values to deterministic
HTML without reading files, using the network, starting processes, or consulting
environment state.

## Usage

```csharp
using CodingAdventures.DocumentAst;
using CodingAdventures.DocumentAstToHtml;

var document = new DocumentNode([
    new HeadingNode(1, [new TextNode("Hello")]),
    new ParagraphNode([new TextNode("A & B")]),
]);

var html = DocumentAstToHtml.ToHtml(document);
// <h1>Hello</h1>
// <p>A &amp; B</p>
```

`ToHtml` supports every node currently exposed by the C# `document-ast`
package: headings, paragraphs, code blocks, blockquotes, lists and task items,
tables, raw blocks, all inline formatting, links, images, and line breaks.
Rendering table/task/strike AST nodes does not claim that the C# parser has
adopted the separate GFM conformance corpus.

## Security boundary

Text, code, titles, alternative text, and URL attributes are HTML-escaped.
Link, image, and autolink destinations remove browser-ignored control and
zero-width characters before blocking `javascript:`, `vbscript:`, `data:`, and
`blob:` schemes. Raw nodes tagged `html` pass through by default because that
is the CommonMark rendering contract. For untrusted content, pass
`new RenderOptions(SuppressRawHtml: true)` or sanitize the AST with a dedicated
policy before rendering.

This renderer is not a complete HTML sanitizer. In particular, enabling raw
HTML passthrough deliberately preserves caller-supplied markup.

## Resource and ownership limits

The renderer borrows the input tree and never mutates it. It returns a newly
owned string and rejects trees deeper than 256 nodes, trees containing more
than 100,000 rendered nodes, and output beyond 1,000,000 UTF-16 characters.
Each call also admits at most 1,000,000 aggregate UTF-16 code units of raw link,
image, and autolink destinations. Every visit counts before normalization,
including repeated aliases and URLs that normalize empty or are rejected.
This bounds work that does not appear in the output; the budget resets on
each call.
Limit failures throw `InvalidOperationException` and return no partial result.

## Development

Run the package build front from this directory:

```sh
sh BUILD
```

The native tests cover every supported AST node, exact formatting, tight and
loose lists, raw routing, URL attacks, and each resource boundary.
See [VALIDATION.md](VALIDATION.md) for the executed checks and their scope.
