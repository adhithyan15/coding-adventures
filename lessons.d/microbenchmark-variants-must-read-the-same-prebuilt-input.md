---
category: Testing & coverage
---

# Microbenchmark variants must read the same prebuilt input

Both measured variants need to consume one array constructed before timing.
If one variant regenerates a predictable sequence while the other traverses
the stored array, equal checksums can conceal that the operations differ. Keep
the input instance outside both callbacks and test the indexed and iteration
helpers with mutated, non-formula data to prove each reads its supplied array.
