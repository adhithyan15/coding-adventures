# Asciidoc — Swift AsciiDoc → HTML Convenience Wrapper

Converts AsciiDoc source text to HTML in a single function call.

## Overview

This package chains `AsciidocParser` and `DocumentAstToHtml` into the
`toHtml(_:)` function. It is the simplest way to render AsciiDoc as HTML.

```swift
import Asciidoc

let html = toHtml("""
= Document Title

Introduction paragraph with *bold* and _italic_ text.

== Section

[source,swift]
----
let greeting = "Hello, world!"
print(greeting)
----

* First item
* Second item
""")
```

Output:

```html
<h1>Document Title</h1>
<p>Introduction paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
<h2>Section</h2>
<pre><code class="language-swift">let greeting = "Hello, world!"
print(greeting)
</code></pre>
<ul>
<li>First item</li>
<li>Second item</li>
</ul>
```

## AsciiDoc vs. CommonMark

| Feature   | CommonMark | AsciiDoc         |
|-----------|------------|------------------|
| Heading   | `# H1`     | `= H1`           |
| Bold      | `**bold**` | `*bold*`         |
| Italic    | `*italic*` | `_italic_`       |
| Code      | `` `code` `` | `` `code` ``  |
| Code block | ` ```lang ` | `[source,lang]` + `----` |
| Quote block | `> text` | `____` … `____` |
| Link      | `[text](url)` | `link:url[text]` |
| Image     | `![alt](url)` | `image:url[alt]` |

## Architecture

```
document-ast         (Layer 0) — shared IR types
asciidoc-parser      (Layer 1) — AsciiDoc text → Document AST
document-ast-to-html (Layer 1) — Document AST → HTML
asciidoc             (Layer 2) — this package: toHtml() wrapper
```

## API

```swift
/// Convert an AsciiDoc string to an HTML string.
public func toHtml(_ text: String) -> String
```
