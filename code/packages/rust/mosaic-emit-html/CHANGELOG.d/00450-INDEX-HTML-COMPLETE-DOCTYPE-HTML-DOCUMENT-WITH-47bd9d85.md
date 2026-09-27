- `index.html` — complete `<!DOCTYPE html>` document with
  `<head>` (charset, viewport, `<title>`) + `<body>` containing
  `<section data-component="X">` that **inlines the component
  fragment** (4-space indent for readability). Open in any
  browser; no install step, no `<script>` tags. The inlined
  fragment is the emitter's own previous output, so no new
  escape surface.
