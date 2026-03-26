# commonmark_native (Ruby)

Rust-backed CommonMark Markdown to HTML converter for Ruby. Wraps the
`commonmark` Rust crate via `ruby-bridge` FFI — zero third-party Ruby
dependencies, no Magnus, no rb-sys.

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
                 │  commonmark_native  ← you are here (Ruby FFI)   │
                 └─────────────────────────────────────────────────┘
```

## Installation

```bash
# Build from source (requires Rust toolchain)
bundle install
bundle exec rake compile
```

## Usage

```ruby
require "coding_adventures_commonmark_native"

# Full CommonMark — raw HTML passes through (trusted content only)
html = CodingAdventures::CommonmarkNative.markdown_to_html("# Hello\n\nWorld\n")
# => "<h1>Hello</h1>\n<p>World</p>\n"

# Safe variant — strips all raw HTML (use for untrusted user content)
html = CodingAdventures::CommonmarkNative.markdown_to_html_safe(
  "<script>alert(1)</script>\n\n**bold**\n"
)
# => "<p><strong>bold</strong></p>\n"
```

## API

### `CodingAdventures::CommonmarkNative.markdown_to_html(markdown) → String`

Convert CommonMark Markdown to HTML. Raw HTML blocks are passed through
unchanged — required for full CommonMark 0.31.2 spec compliance.

**Use for**: trusted, author-controlled content (documentation, blog posts).

**Do not use for**: untrusted user-supplied content — use
`markdown_to_html_safe` instead.

### `CodingAdventures::CommonmarkNative.markdown_to_html_safe(markdown) → String`

Convert CommonMark Markdown to HTML, stripping all raw HTML blocks and inline
HTML. Prevents XSS attacks by dropping every raw HTML node before rendering.

**Use for**: user-supplied content in web applications (comments, forum posts,
chat messages, wiki edits).

## Security

```ruby
# Attacker tries to inject a script tag
attacker_input = "<script>document.cookie='stolen'</script>\n\n# Title\n"

# UNSAFE — never use for user content
html = CodingAdventures::CommonmarkNative.markdown_to_html(attacker_input)
# => "<script>document.cookie='stolen'</script>\n<h1>Title</h1>\n"

# SAFE — script tag is stripped
html = CodingAdventures::CommonmarkNative.markdown_to_html_safe(attacker_input)
# => "<h1>Title</h1>\n"
```
