---
category: Elixir
---

# `if` expressions return values that are silently discarded

if not bound. `if cond do compiler = ...; compiler end` discards the rebinding — wrap as `compiler = if cond do ... else compiler end`.
