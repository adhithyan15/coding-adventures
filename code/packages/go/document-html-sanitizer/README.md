# document-html-sanitizer (Go)

A zero-dependency, regexp-based HTML sanitizer. String in, string out.
No AST, no DOM, no external packages beyond the Go standard library.

```
toHtml(doc)              ← document-ast-to-html (or any HTML source)
      ↓
sanitizeHtml(html, pol)  ← this package
      ↓
final output
```

Spec: [TE02 — Document Sanitization](../../../../specs/TE02-document-sanitization.md)

## Why no DOM?

Go (and many other languages/environments) do not have a native DOM parser.
Using regexp-based sanitization keeps this package portable across Go, Python,
Rust, Elixir, Lua, and any edge JS runtime — every language implements the
same algorithm with its own stdlib.

For the highest fidelity (e.g., a browser extension that has a real DOM),
pre-process with `golang.org/x/net/html` before calling `SanitizeHtml`.

## When to use this package

- You receive HTML from an external source (CMS, API, user paste)
- The AST is no longer available (pipeline already rendered)
- Belt-and-suspenders safety after `document-ast-sanitizer`

For structured Markdown, prefer Stage 1 (`document-ast-sanitizer`) because it
operates on structured data. Use this package as a safety net (Stage 2).

## Installation

```bash
go get github.com/adhithyan15/coding-adventures/code/packages/go/document-html-sanitizer
```

This package has **no external dependencies**.

## Quick Start

```go
import sanitizer "github.com/adhithyan15/coding-adventures/code/packages/go/document-html-sanitizer"

// Untrusted HTML from a CMS:
safe := sanitizer.SanitizeHtml(cmsApiResponse.Body, sanitizer.HTML_STRICT)

// Belt-and-suspenders after AST sanitization:
rawHtml := renderer.ToHtml(astSanitizer.Sanitize(doc, astSanitizer.STRICT), ...)
safe := sanitizer.SanitizeHtml(rawHtml, sanitizer.HTML_STRICT)
```

## Presets

### `HTML_STRICT` — untrusted HTML

Recommended for HTML from external sources.

- Removes: `script`, `style`, `iframe`, `object`, `embed`, `applet`, `form`,
  `input`, `button`, `select`, `textarea`, `noscript`, `meta`, `link`, `base`
- Strips all `on*` event handler attributes
- Strips `srcdoc` and `formaction` attributes
- Allows only `http`, `https`, `mailto` URL schemes in `href`/`src`
- Strips HTML comments
- Strips `style` attributes containing `expression()` or unsafe `url()`

### `HTML_RELAXED` — authenticated users

Recommended for internal tools and authenticated-user content.

- Removes: `script`, `iframe`, `object`, `embed`, `applet`
- Strips all `on*` event handler attributes
- Allows `http`, `https`, `mailto`, `ftp` URL schemes
- Keeps HTML comments
- Still strips dangerous `style` attribute values

### `HTML_PASSTHROUGH` — trusted HTML

No sanitization. Everything passes through unchanged.

## Custom Policies

```go
policy := sanitizer.HtmlSanitizationPolicy{
    DropElements:            []string{"script", "iframe"},
    DropAttributes:          []string{"class", "id"},
    AllowedUrlSchemes:       []string{"http", "https"},
    DropComments:            true,
    SanitizeStyleAttributes: true,
}

safe := sanitizer.SanitizeHtml(html, policy)
```

## `HtmlSanitizationPolicy` Fields

| Field | Type | Description |
|-------|------|-------------|
| `DropElements` | `[]string` | Element names removed including content |
| `DropAttributes` | `[]string` | Attribute names stripped from all elements |
| `AllowedUrlSchemes` | `[]string` | Schemes allowed in href/src |
| `AllowAllUrlSchemes` | `bool` | Bypass scheme check (PASSTHROUGH) |
| `DropComments` | `bool` | Remove `<!-- … -->` comments |
| `SanitizeStyleAttributes` | `bool` | Strip `expression()`/unsafe `url()` style attrs |

## What Gets Stripped

### Dangerous Elements (HTML_STRICT)

| Element | Risk |
|---------|------|
| `<script>` | Direct JavaScript execution |
| `<style>` | CSS `expression()` attacks |
| `<iframe>` | Framing attacks, clickjacking |
| `<object>` | Plugin execution |
| `<embed>` | Plugin execution |
| `<applet>` | Java applet execution (legacy) |
| `<form>` | CSRF, credential phishing |
| `<meta>` | Redirect via `http-equiv="refresh"` |
| `<base>` | Base URL hijacking |
| `<link>` | CSS import, DNS prefetch exfiltration |
| `<noscript>` | Parser context abuse |

### Dangerous Attributes (always)

| Pattern | Risk |
|---------|------|
| `on*` | All event handlers (onclick, onload, etc.) |
| `srcdoc` | Inline HTML frame content |
| `formaction` | Overrides form action URL |

### CSS Injection

When `SanitizeStyleAttributes: true`:
- `style="width: expression(alert(1))"` → style attribute dropped
- `style="background: url(javascript:alert(1))"` → style attribute dropped
- `style="background: url(https://example.com/img.png)"` → **kept** (safe)

## URL Bypass Prevention

The URL sanitizer strips invisible characters before scheme detection:

- `javascript\x00:alert(1)` → null byte stripped → `javascript:` → blocked
- `\u200bjavascript:alert(1)` → zero-width space stripped → blocked
- `JAVASCRIPT:alert(1)` → lowercased → blocked

## Limitations

Regexp-based HTML sanitization has inherent limitations compared to a full DOM
parser:

- Malformed HTML (unclosed tags, unquoted attributes) may not be handled
  exactly as a browser would.
- Deeply nested same-element structures (e.g., `<script><script>`) require
  iterative matching (the sanitizer runs up to 20 iterations).
- For the highest fidelity, use a real HTML parser as a pre-processing step.

## Test Coverage

```
go test ./... -coverprofile=coverage.out
go tool cover -func=coverage.out
```

Coverage target: 90%+. Current: 89%.

## Dependencies

None. This package uses only the Go standard library (`regexp`, `strings`).
