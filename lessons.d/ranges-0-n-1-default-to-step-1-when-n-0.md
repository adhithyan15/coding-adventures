---
category: Elixir
---

# Ranges `0..(n-1)` default to step `-1` when `n=0`

— iterates `[0, -1]`. Always use explicit step `0..(n-1)//1`. Ascending range `0..-1//1` is correctly empty.
