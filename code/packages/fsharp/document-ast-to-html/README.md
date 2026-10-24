# document-ast-to-html

Pure F# rendering from the shared Document AST into an HTML fragment.

This package is the final, mechanical stage of the document pipeline: parsers
produce typed `DocumentNode` values, and this renderer converts those values to
deterministic HTML without reading files, using the network, starting processes,
or consulting environment state.

## Usage

```fsharp
open CodingAdventures.DocumentAst.FSharp
open CodingAdventures.DocumentAstToHtml.FSharp

let document =
    { Children =
        [ HeadingNode(1, [ TextNode "Hello" ])
          ParagraphNode [ TextNode "A & B" ] ] }

let html = DocumentAstToHtml.render document
// <h1>Hello</h1>
// <p>A &amp; B</p>
```

`render` supports every node currently exposed by the F# `document-ast`
package: headings, paragraphs, code blocks, blockquotes, lists and task items,
tables, raw blocks, all inline formatting, links, images, and line breaks.
Rendering table, task-item, and strikethrough AST nodes does not claim that the
F# CommonMark parser has adopted a GFM conformance corpus.

## Security boundary

Text, code, titles, alternative text, and URL attributes are HTML-escaped.
Link, image, and autolink destinations remove browser-ignored control and
zero-width characters before blocking `javascript:`, `vbscript:`, `data:`, and
`blob:` schemes. Raw nodes tagged `html` pass through by default because that is
the CommonMark rendering contract. For untrusted content, call
`DocumentAstToHtml.renderWithOptions { SuppressRawHtml = true }` or sanitize the
AST with a dedicated policy before rendering.

This renderer is not a complete HTML sanitizer. Enabling raw HTML pass-through
deliberately preserves caller-supplied markup.

## Resource and ownership limits

The renderer borrows the immutable input tree and returns a newly owned string.
It rejects trees deeper than 256 nodes, trees containing more than 100,000
rendered nodes, output beyond 1,000,000 UTF-16 characters, and more than
1,000,000 aggregate UTF-16 destination units normalized by one render. The URL
budget charges repeated aliases and stripped or blocked values before scanning;
it resets for every call. Limit failures raise `InvalidOperationException` and
return no partial result.

## Development

Run the package build front from this directory:

```sh
sh BUILD
```

The native tests cover every supported AST node, exact formatting, tight and
loose lists, raw routing, URL attacks, deterministic ownership, and every
resource boundary. The build front enforces at least 95% line coverage for the
renderer assembly.
