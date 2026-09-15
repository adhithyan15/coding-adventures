---
category: Compiler / VM / language pipeline
---

# Bracket-aware regex delimiter scanning

in `.tokens` parsers — `/` inside `[...]` is not the closing delim. Don't escape it as `[^\/]`; the parser handles it correctly.
