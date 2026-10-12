---
category: Testing & coverage
---

# discover reading-reach floor shard paths before assuming their directory name

I assumed the direct reading-reach owner lived in a
`tests/reading-reach-floor.d` directory, so the comparator-read command failed
after the earlier files had printed. The owners use a different discovered
path. Locate a known level owner with `rg --files` before opening it, then add
the new level beside that real owner and run both reading-reach suites.
