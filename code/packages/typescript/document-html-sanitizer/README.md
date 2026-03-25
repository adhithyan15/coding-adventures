# @coding-adventures/document-html-sanitizer

Pattern-based HTML string sanitizer. String in, string out. No dependency on `@coding-adventures/document-ast`.

Strips dangerous HTML elements, event handler attributes, dangerous URL schemes, and CSS injection vectors from raw HTML strings.

## Where it fits

```
toHtml(doc)              ← @coding-adventures/document-ast-to-html
      ↓
sanitizeHtml(html, pol)  ← THIS PACKAGE (safety net)
      ↓
final safe HTML
```

Or as the only sanitization layer when the HTML arrives from an external source (CMS API, user paste):

```
external HTML → sanitizeHtml(html, HTML_STRICT) → safe HTML
```

## Installation

```bash
npm install @coding-adventures/document-html-sanitizer
```

## Quick Start

```typescript
import { sanitizeHtml, HTML_STRICT } from "@coding-adventures/document-html-sanitizer";

// Safety net after rendering
const safeHtml = sanitizeHtml(toHtml(parse(markdown)), HTML_STRICT);

// External HTML (CMS, API response)
const safeHtml = sanitizeHtml(cmsApiResponse.body, HTML_STRICT);

// Two-stage (belt and suspenders)
import { sanitize, STRICT } from "@coding-adventures/document-ast-sanitizer";
const safeHtml = sanitizeHtml(
  toHtml(sanitize(parse(userMarkdown), STRICT)),
  HTML_STRICT
);
```

## Named Presets

| Preset            | Use case                                              |
|-------------------|-------------------------------------------------------|
| `HTML_STRICT`     | Untrusted HTML from external sources                  |
| `HTML_RELAXED`    | Authenticated users / internal tools                  |
| `HTML_PASSTHROUGH`| Fully trusted HTML (no sanitization)                  |

## Policy Options

```typescript
interface HtmlSanitizationPolicy {
  dropElements?:           string[];       // element names to remove + content
  dropAttributes?:         string[];       // attribute names to strip
  allowedUrlSchemes?:      string[] | null; // null = allow all
  dropComments?:           boolean;        // default: true
  sanitizeStyleAttributes?: boolean;       // default: true
  domAdapter?:             HtmlSanitizerDomAdapter; // optional DOM path
}
```

## XSS Vectors Blocked (HTML_STRICT)

| Vector | How blocked |
|--------|-------------|
| `<script>alert(1)</script>` | `dropElements: ["script", ...]` |
| `<SCRIPT>` uppercase | Case-insensitive element dropping |
| `onclick="alert(1)"` | `on*` attribute stripping (always active) |
| `href="javascript:alert(1)"` | URL scheme check on href/src |
| `href="JAVASCRIPT:alert(1)"` | Case-insensitive scheme detection |
| `style="width:expression(alert(1))"` | CSS expression() detection |
| `style="background:url(javascript:...)"` | CSS url() scheme check |
| `<!-- <img onerror=...> -->` | Comment stripping |
| `srcdoc="<script>..."` | Always-stripped attribute |

## DOM Adapter (Optional)

For browser environments, supply a `domAdapter` for higher-fidelity sanitization:

```typescript
const result = sanitizeHtml(html, {
  ...HTML_STRICT,
  domAdapter: {
    parse: (html) => new DOMParser().parseFromString(html, "text/html"),
    walk: (dom, visitor) => { /* walk DOM calling visitor.element/comment */ },
    serialize: (dom) => (dom as Document).body.innerHTML,
  },
});
```

## Note: on* handlers always stripped

Even with `HTML_PASSTHROUGH`, `on*` event handler attributes (`onclick`, `onload`, etc.) are stripped during attribute processing. This is the minimum defense-in-depth.

## Spec

TE02 — Document Sanitization
