---
category: TypeScript / JavaScript
---

# `String.prototype.replace` with a STRING second arg honours `$&`, `$1`, `$<name>`, `$$`

A frontmatter slug like `"$&"` injected via `template.replace(/\{slug\}/g, slug)` expands to the regex match (`"{slug}"`) — unintended substitution. Fix: pass a function replacement (`() => slug`), immune to `$`-parsing. Found in `forme-router`, `forme-collect-chronological`, `forme-render-static` (same bug pattern across all three from copy-paste).
