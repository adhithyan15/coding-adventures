---
category: TypeScript / JavaScript
---

# Browser form normalization must preserve unchanged canonical AST strings

A fenced authoring form used a single-line input for canonical metadata and
removed the final LF from payload display. The browser stripped decoded metadata
newlines; saving a blank payload collapsed `"\n"` to `""`. Existing tests edited
the fields and missed unchanged round trips.

Use a multiline metadata control and show the complete fenced payload. Browser
textareas also normalize CR/CRLF to LF: compare against the normalized display
and retain the original AST string on an unchanged submission. Assert unchanged
round trips for decoded LF/CR/CRLF and zero, one, and multiple blank payload
lines. Preserve legacy code-block behavior separately.
