---
category: Ruby
---

# Predicate methods use `?` suffix

`contains?`, `empty?`, `valid?`, `halted?`, `idle?`. Tests calling `obj.contains("x")` raise `NoMethodError` — must be `obj.contains?("x")`.
