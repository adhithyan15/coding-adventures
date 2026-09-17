---
category: Security boundaries
---

# A marker that a later pass scans for must be unspellable by data an earlier pass rendered

**What went wrong.** UI86's HTML lowering (#15420) emitted a dynamic
`selected` state as an attribute, `data-mosaic-pressed-when="…"`. A new
`renderPressed` pass in the generated `main.js` scanned for that attribute
with a regex and rewrote it. But `renderTemplate` renders loop rows (host data
included) *before* that pass runs, so the scanned text already contained host
strings.

`escapeHtml` escaped `" < > & '` but not `=`, so a task name of
` data-mosaic-pressed-when=` spelled the marker next to the template's own
closing quote. The rewrite then shifted every later attribute on the line out
of its quotes, and in a Node reproduction a host-supplied `onfocus=alert(1)`
became a live attribute. The pass also ran on every page, not only pages that
used `selected`. The security review caught it before push.

The same review found an older bug of the same kind: `{` and `}` were not
escaped either, so a loop item containing `{{{key}}}` was expanded again by
the outer mustache pass, through the unescaped path (#15460).

**The fix.**
- The marker is now `data-mosaic&pressed`. Every raw `&` in host text
  (`escapeHtml`) and in authored literals (`escape_html_attr`,
  `escape_html_text`) is escaped, so neither can spell the marker.
- Runtime `escapeHtml` also encodes `=`, `{` and `}`.
- The Node check runs both reproductions.

**What to do differently.**
- **When adding a string-rewriting pass, ask what text it scans.** If
  earlier passes have already inserted data, the marker must contain a
  character that the data's escaping always removes. "The data is escaped
  for HTML" is not enough, because escaping is relative to the consumer,
  and the later pass is a consumer too.
- **Prefer a scan that sees only template text.** Otherwise make the marker
  unforgeable by construction, and say why in a comment.
- **Test the generated runtime, not just the emitter's string output.** The
  emitter tests were green throughout. Only executing `main.js` with hostile
  row data showed the problem.
