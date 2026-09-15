---
category: TypeScript / JavaScript
---

# `String.prototype.replace` regex flags: include `g` when replacing every occurrence

Without `g`, only the first match is replaced — common foot-gun when the pattern looks deceptively "all-occurrences." Use `/pattern/g`.
