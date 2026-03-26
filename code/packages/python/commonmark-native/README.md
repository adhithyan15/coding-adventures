# commonmark-native (Python)

Rust-backed CommonMark Markdown to HTML converter for Python. Wraps the
`commonmark` Rust crate via `python-bridge` FFI — zero third-party Python
dependencies, no PyO3, no bindgen.

## Where it fits

```
                 ┌─────────────────────────────────────────────────┐
                 │              Rust crate stack                   │
                 │                                                 │
                 │  document-ast                                   │
                 │      ↓                                          │
                 │  commonmark-parser    document-ast-to-html      │
                 │      ↓ depends on both ↓                        │
                 │  commonmark  ← markdown_to_html() lives here    │
                 │      ↓                                          │
                 │  commonmark-native  ← you are here (Python FFI) │
                 └─────────────────────────────────────────────────┘
```

## Installation

```bash
# Build from source (requires Rust toolchain)
cargo build --release
cp target/release/libcommonmark_native.so src/commonmark_native/commonmark_native.so
pip install -e .
```

## Usage

```python
from commonmark_native import markdown_to_html, markdown_to_html_safe

# Full CommonMark — raw HTML passes through (trusted content only)
html = markdown_to_html("# Hello\n\nWorld *with* emphasis.\n")
# → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"

# Safe variant — strips all raw HTML (use for untrusted user content)
html = markdown_to_html_safe("<script>alert(1)</script>\n\n**bold**\n")
# → "<p><strong>bold</strong></p>\n"
```

## API

### `markdown_to_html(markdown: str) -> str`

Convert CommonMark Markdown to HTML. Raw HTML blocks are passed through
unchanged — required for full CommonMark 0.31.2 spec compliance.

**Use for**: trusted, author-controlled content (documentation sites, blog
posts, README rendering).

**Do not use for**: untrusted user-supplied content — use
`markdown_to_html_safe` instead.

### `markdown_to_html_safe(markdown: str) -> str`

Convert CommonMark Markdown to HTML, stripping all raw HTML blocks and inline
HTML. Prevents XSS attacks by dropping every `RawBlockNode` and `RawInlineNode`
before rendering.

**Use for**: user-supplied content in web applications (comments, forum posts,
chat messages, wiki edits).

## Security

The safe variant defends against raw-HTML injection attacks:

```python
# Attacker tries to inject a script tag through Markdown
attacker_input = "<script>document.cookie='stolen=' + document.cookie</script>\n\n# Title\n"

# UNSAFE — never use this for user content
html = markdown_to_html(attacker_input)
# → "<script>document.cookie=...</script>\n<h1>Title</h1>\n"

# SAFE — script tag is stripped
html = markdown_to_html_safe(attacker_input)
# → "<h1>Title</h1>\n"
```

## Implementation

The native extension (`src/lib.rs`) uses `python-bridge` to call CPython's
stable C API directly. No headers required at build time. The `.so`/`.pyd`
artifact is linked against the running Python interpreter at load time.
