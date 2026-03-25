# @coding-adventures/document-ast-sanitizer

Policy-driven AST sanitizer for the Document AST (TE00). Transforms a `DocumentNode` according to a caller-defined `SanitizationPolicy`, producing a new sanitized document. Pure and immutable — the input is never mutated.

## Where it fits

```
parse(markdown)          ← @coding-adventures/commonmark-parser
      ↓
sanitize(doc, policy)    ← THIS PACKAGE
      ↓
toHtml(doc)              ← @coding-adventures/document-ast-to-html
      ↓
final HTML
```

## Installation

```bash
npm install @coding-adventures/document-ast-sanitizer
```

## Quick Start

```typescript
import { sanitize, STRICT, RELAXED, PASSTHROUGH } from "@coding-adventures/document-ast-sanitizer";
import { parse } from "@coding-adventures/commonmark-parser";
import { toHtml } from "@coding-adventures/document-ast-to-html";

// User-generated content — use STRICT
const html = toHtml(sanitize(parse(userMarkdown), STRICT));

// Custom policy — allow HTML blocks but restrict headings
const html = toHtml(sanitize(parse(editorMarkdown), {
  ...RELAXED,
  minHeadingLevel: 2,
  allowedUrlSchemes: ["http", "https"],
}));
```

## Named Presets

| Preset        | Use case                                           |
|---------------|----------------------------------------------------|
| `STRICT`      | User-generated content (comments, forum posts)     |
| `RELAXED`     | Authenticated users, internal wikis, CMS editors   |
| `PASSTHROUGH` | Fully trusted content (documentation, static sites) |

## Policy Options

```typescript
interface SanitizationPolicy {
  // Raw node control
  allowRawBlockFormats?:  "drop-all" | "passthrough" | string[];
  allowRawInlineFormats?: "drop-all" | "passthrough" | string[];

  // URL scheme allowlist
  allowedUrlSchemes?: string[] | null;  // null = allow all

  // Node type control
  dropLinks?:               boolean;
  dropImages?:              boolean;
  transformImageToText?:    boolean;
  maxHeadingLevel?:         1 | 2 | 3 | 4 | 5 | 6 | "drop";
  minHeadingLevel?:         1 | 2 | 3 | 4 | 5 | 6;
  dropBlockquotes?:         boolean;
  dropCodeBlocks?:          boolean;
  transformCodeSpanToText?: boolean;
}
```

### Policy composition

Policies are plain data objects — composable via spread:

```typescript
const myPolicy = { ...STRICT, maxHeadingLevel: 3, dropLinks: false };
```

## XSS Vectors Blocked (STRICT preset)

| Vector | How blocked |
|--------|-------------|
| `<script>` injection via raw blocks | `allowRawBlockFormats: "drop-all"` |
| `javascript:` links | `allowedUrlSchemes: ["http", "https", "mailto"]` |
| Null-byte bypass `java\x00script:` | Control character stripping before scheme detection |
| CR bypass `java\rscript:` | Control character stripping |
| Zero-width-space bypass | Unicode invisible character stripping |
| `data:` / `blob:` / `vbscript:` URLs | Not in allowed schemes |
| h1 page structure override | `minHeadingLevel: 2` |

## Transformation Rules

Every node type is handled explicitly (no silent pass-through):

| Node | Action |
|------|--------|
| `DocumentNode` | Always recurse |
| `HeadingNode` | Clamp level or drop |
| `ParagraphNode` | Recurse; drop if empty |
| `CodeBlockNode` | Drop if `dropCodeBlocks` |
| `BlockquoteNode` | Drop if `dropBlockquotes`; else recurse |
| `ListNode` / `ListItemNode` | Recurse |
| `ThematicBreakNode` | Keep as-is |
| `RawBlockNode` | Drop / allowlist |
| `TextNode` | Keep as-is |
| `EmphasisNode` / `StrongNode` | Recurse; drop if empty |
| `CodeSpanNode` | Convert to text or keep |
| `LinkNode` | Promote children / sanitize URL |
| `ImageNode` | Drop / convert to text / sanitize URL |
| `AutolinkNode` | Drop if URL not allowed |
| `RawInlineNode` | Drop / allowlist |
| `HardBreakNode` / `SoftBreakNode` | Keep as-is |

## Spec

TE02 — Document Sanitization
