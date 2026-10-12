---
category: Testing & coverage
---

# Discover a comparator test path before opening it because language suites shard levels differently

I assumed Hindi used the same per-level task-shape test path as the newly
sharded Sanskrit suite and tried to open a nonexistent `hindi/b2.test.ts`.
Language suites are at different migration stages. Use `rg --files` to locate
the comparator test first, then copy only its relevant assertions into the new
level-owned Sanskrit owner.
