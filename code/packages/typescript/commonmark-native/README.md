# @coding-adventures/commonmark-native (Node.js)

Rust-backed CommonMark Markdown to HTML converter for Node.js. Wraps the
`commonmark` Rust crate via `node-bridge` N-API FFI — zero npm dependencies,
no napi-rs, no napi-sys.

## Where it fits

```
                 ┌─────────────────────────────────────────────────────┐
                 │                 Rust crate stack                   │
                 │                                                     │
                 │  document-ast                                       │
                 │      ↓                                              │
                 │  commonmark-parser    document-ast-to-html          │
                 │      ↓ depends on both ↓                            │
                 │  commonmark  ← markdownToHtml() lives here          │
                 │      ↓                                              │
                 │  commonmark-native  ← you are here (Node.js FFI)   │
                 └─────────────────────────────────────────────────────┘
```

## Installation

```bash
# Build from source (requires Rust toolchain and Node.js)
npm ci
cargo build --release
cp target/release/libcommonmark_native_node.so commonmark_native_node.node
```

## Usage

```typescript
import { markdownToHtml, markdownToHtmlSafe } from "@coding-adventures/commonmark-native";

// Full CommonMark — raw HTML passes through (trusted content only)
const html = markdownToHtml("# Hello\n\nWorld *with* emphasis.\n");
// → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"

// Safe variant — strips all raw HTML (use for untrusted user content)
const safeHtml = markdownToHtmlSafe("<script>alert(1)</script>\n\n**bold**\n");
// → "<p><strong>bold</strong></p>\n"
```

## API

### `markdownToHtml(markdown: string): string`

Convert CommonMark Markdown to HTML. Raw HTML blocks are passed through
unchanged — required for full CommonMark 0.31.2 spec compliance.

**Use for**: trusted, author-controlled content (documentation, blog posts).

**Do not use for**: untrusted user-supplied content.

### `markdownToHtmlSafe(markdown: string): string`

Convert CommonMark Markdown to HTML, stripping all raw HTML. Prevents XSS
attacks by dropping every raw HTML block and inline HTML node before rendering.

**Use for**: user-supplied content in web applications (comments, forum posts,
chat messages, wiki edits).

## Security

```typescript
const attackerMarkdown = "<script>document.cookie='stolen'</script>\n\n# Title\n";

// UNSAFE — never use for user content
markdownToHtml(attackerMarkdown);
// → "<script>document.cookie='stolen'</script>\n<h1>Title</h1>\n"

// SAFE — script tag is stripped
markdownToHtmlSafe(attackerMarkdown);
// → "<h1>Title</h1>\n"
```

## Implementation

The native addon (`src/lib.rs`) uses `node-bridge` to call Node.js's N-API
directly. The `napi_create_function` N-API function creates two standalone
JS function objects that are attached to the module's exports. The `.node`
binary is ABI-stable across Node.js versions (N-API v1+).
