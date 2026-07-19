# document-ast

A pure Haskell implementation of the format-agnostic Document AST defined by
TE00. Front-end parsers produce this immutable intermediate representation and
back-end renderers consume it, so document formats can interoperate through one
stable typed tree.

## API

- Concrete immutable records model documents, headings, paragraphs, code
  blocks, blockquotes, lists, list items, task items, raw blocks, and tables.
- Concrete immutable records model text, emphasis, strong text,
  strikethrough, code spans, resolved links, images, autolinks, raw inline
  content, and hard and soft breaks.
- `BlockNode`, `ListChildNode`, `InlineNode`, and `Node` are exhaustive
  algebraic unions for type-safe traversal.
- `TableAlignment` preserves left, right, center, and absent alignment hints.
- `blockNodeTypeName`, `listChildNodeTypeName`, `tableRowNodeTypeName`,
  `tableCellNodeTypeName`, `inlineNodeTypeName`, and `nodeTypeName` expose the
  stable cross-language discriminator strings.

The data model performs no parsing or rendering and has no dependencies beyond
`base`. Producers are responsible for TE00 invariants such as heading levels
1 through 6, resolved links, non-nested documents, and trailing newlines in
code blocks.

## Running the tests

```sh
cabal test all
```
