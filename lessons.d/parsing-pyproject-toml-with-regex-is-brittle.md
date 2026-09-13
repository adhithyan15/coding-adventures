---
category: Python
---

# Parsing pyproject.toml with regex is brittle

Comment lines containing `[` break naive `[^[]*?` cross-line patterns. Parse line-by-line, skip `#` lines, track section headers explicitly.
