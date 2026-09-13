---
category: TypeScript / JavaScript
---

# Recursive walkers on attacker-controlled input need a depth bound

`walk()`/`deepEqual()` style functions on JSON Schemas / validator inputs / parsed TOML can crash with `RangeError` on adversarial 10k-deep nesting. Thread an explicit `depth` counter through the recursion and short-circuit at `MAX_WALK_DEPTH = 256` (well beyond any sane input). Never throw — push a synthetic violation/finding and return.
