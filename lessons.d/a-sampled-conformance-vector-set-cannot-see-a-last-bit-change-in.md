---
category: Testing & coverage
---

# A sampled conformance vector set cannot see a last-bit change in a high-order polynomial coefficient or a fused multiply-add on a small term, so the gate must name that masked set instead of demanding it

PHY00a 6.4 first required that flipping the last bit of every constant, and fusing
every one of 24 multiply-add sites, change at least one conformance vector. Measured
on 61,854 vectors (12,184 distinct inputs), 16 of 28 constants and 20 of 24 sites
change no result: the last bit of `S3` moves a sine by about 1e-4 ulp, of `S9` by
about 1e-17 ulp, and five reduction products (`k * PIO2_i`, `0.5 * z`) are exact so
fusing them cannot change anything. No sample can reach those, and a hand-picked
witness would need a table-maker's-dilemma search per constant.

What to do differently: make a sensitivity gate report what it cannot see. List the
masked constants and sites explicitly, assert the provably inert ones stay inert, fail
only on an unexpected miss, and protect the masked constants another way (here, each
port asserts the bits of every constant it embeds). Where one site can be made visible
by a deterministic search (the `x * INV_PIO2 + MAGIC` fusion needs inputs next to an
odd multiple of pi/4), add those inputs to the fixture instead of weakening the gate.
