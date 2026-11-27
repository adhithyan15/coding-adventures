---
category: Testing & coverage
---

# A pre-flight work meter written while most suites were not-live becomes the binding limit the moment a lane implements their level

The TypeScript v2 lane's `checkWork` meters a `slim` run before the first case: every `ok` control is counted as re-running the whole slim chunk
set of every claim in its `required_claims`, and the total must stay under `MAX_TOTAL_WORK_SLIM` (32,000,000). It was written and tested while the
lane implemented `ir.core` alone, so wasm-leb128 had 4 live suites of 44 and the meter never came near the limit. When S10f made all 44 suites
live, the same meter read 37,782,980 ordinals (36,803,960 admitted cases) and `runSuite(..., "slim")` refused a committed, oracle-verified corpus:
172 of its `required_claims` entries are `unproven` (no stand-in, so no witness chunk), and section 5.6 has a control stop at its first failure,
which a worst-case pre-flight bound cannot know. Nothing in the lane's own tests had ever run the meter at the size the corpus really has.

What to do differently: when a lane gains a level, re-run every whole-corpus bound (work meters, size caps, counts) at the corpus's real size
before calling the level done, and read what the meter *counts* against the sentence of the spec that defines the limit (cases, stopping at the
first failure) and not only against its number. The fix is the corpus runner's (S7b: count a control up to its witness chunk, or to its first
failure); this lane keeps the conservative meter, its test asserts the refusal for wasm-leb128 at the three levels, and the slim chunks are still
enumerated and checked directly against the chunk file there.
