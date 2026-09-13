---
category: Lua
---

# Lua decoder hygiene

After block-loop exits (`last_block == 1`), assert read cursor equals input length — silently ignoring trailing bytes hides truncation/concatenation bugs.
