---
category: Testing & coverage
---

# A ductus caption edit moves the stroke-ownership data hash, so re-measure that pin after the last label change

**Context:** `script-ductus`, adding the Japanese ductus for あ, い, う, え, お
and か.

**What happened:** I added the six entries to `src/strokes/japanese.ts`, ran
`tests/stroke-ownership.test.ts` and re-pinned `keys`, `keyHash`,
`nonTamilDataHash` and the Japanese count. Then I rendered the filmstrips.
か's last caption wrapped to three lines, so I renamed the label of the last
stroke of お and か ("draw the short stroke down to the right" became "draw
the dot down to the right"). The full suite then failed: `nonTamilDataHash`
no longer matched.

**Why:** that hash covers every parsed ductus record, including each
segment's `label`, not just the coordinates and the key order. So a caption
edit made only for layout still moves the pin. `keyHash` and the counts did
not move, which made the first re-pin look final.

**Fix:** re-ran the ownership test and pinned the new `nonTamilDataHash`.

**Do differently:** treat the stroke-ownership pins as the last step. Render
the filmstrips and settle every caption first, then measure the hashes, then
run the full suite once more.
