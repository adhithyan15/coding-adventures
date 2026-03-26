# commonmark

CommonMark 0.31.2 compliant markdown parser for TypeScript.

Parses Markdown source text into a typed AST, then renders to HTML.
Passes **705 of 714** CommonMark spec tests (98.7%).

## Installation

```bash
npm install @coding-adventures/commonmark
```

## Usage

```typescript
import { parse, toHtml } from "@coding-adventures/commonmark";

const ast = parse("# Hello\n\nWorld\n");
const html = toHtml(ast);
// => "<h1>Hello</h1>\n<p>World</p>\n"
```

## API

### `parse(markdown: string): DocumentNode`

Parse a Markdown string into an AST. Returns a `DocumentNode` whose `children`
are the top-level block nodes.

### `toHtml(document: DocumentNode): string`

Render an AST produced by `parse` to an HTML string.

## AST Node Types

### Block nodes

| Type | Fields |
|------|--------|
| `DocumentNode` | `children: BlockNode[]` |
| `HeadingNode` | `level: 1–6`, `children: InlineNode[]` |
| `ParagraphNode` | `children: InlineNode[]` |
| `CodeBlockNode` | `language: string \| null`, `value: string` |
| `BlockquoteNode` | `children: BlockNode[]` |
| `ListNode` | `ordered: boolean`, `start: number \| null`, `tight: boolean`, `children: ListItemNode[]` |
| `ListItemNode` | `children: BlockNode[]` |
| `ThematicBreakNode` | _(no extra fields)_ |
| `HtmlBlockNode` | `value: string` |
| `LinkDefinitionNode` | `label: string`, `destination: string`, `title: string \| null` |

### Inline nodes

| Type | Fields |
|------|--------|
| `TextNode` | `value: string` |
| `EmphasisNode` | `children: InlineNode[]` |
| `StrongNode` | `children: InlineNode[]` |
| `CodeSpanNode` | `value: string` |
| `LinkNode` | `destination: string`, `title: string \| null`, `children: InlineNode[]` |
| `ImageNode` | `destination: string`, `title: string \| null`, `alt: string` |
| `AutolinkNode` | `destination: string`, `isEmail: boolean` |
| `HtmlInlineNode` | `value: string` |
| `HardBreakNode` | _(no extra fields)_ |
| `SoftBreakNode` | _(no extra fields)_ |

## Architecture

Parsing is two-phase:

1. **Block phase** — splits the input into block-level structure (headings,
   paragraphs, code blocks, lists, etc.) collecting link reference definitions
   as a side-effect.
2. **Inline phase** — processes the raw text of each paragraph and heading
   into inline nodes (emphasis, links, code spans, etc.) using a delimiter-stack
   algorithm.

The `Scanner` class provides a cursor-based string scanner used by both phases.
Entity decoding is handled by a full HTML5 named-entity table in `entities.ts`.

## Dependencies

- `@coding-adventures/state-machine`

## Development

```bash
# Run tests
bash BUILD
```
