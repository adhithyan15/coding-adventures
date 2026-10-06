---
category: Testing & coverage
---

# A new sharded curriculum chapter needs closure and attainment checks

**Context:** Punjabi chapter 155 added six short A1 form lessons and their
chapter, curriculum, book, narration, and modality owners.

**What happened:** the focused Punjabi corpus and integration tests passed,
but the full CI build found four missed contracts: two new JSON shards were
semantically right but not byte-for-byte canonical, the per-track curriculum
digest still counted 838 rather than 844 lessons, and two newly introduced
writing skills lacked later genuine practice, revoking Punjabi's A1 claim.

**Fix:** format the shards to match `shard(unshard(shards))`, update only
Punjabi's digest, and add real supported and delayed six-field revisits in
subsequent lessons. The repaired sequence keeps each lesson below five
minutes; its R1 misses fall by two and the level gate again recognizes A1.

**Do differently:** before pushing a new chapter, run the chapter and
curriculum shard tests, the per-track curriculum digest pin, and the level
gate in addition to the focused corpus, integration, and generated-file
checks. Do not change the attainment pin or weaken the reinforcement gate to
hide missing practice.
