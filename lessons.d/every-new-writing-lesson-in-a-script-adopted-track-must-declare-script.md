---
category: Testing & coverage
---

# Every new writing lesson in a script-adopted track must declare script delivery

Punjabi R3's two Chapter 173 retrieval lessons had `type: writing` and a pen
modality, but omitted `delivery: script`. The focused Punjabi tests and generated
artifact checks passed; the full CI suite failed the cross-track
`modality-manifest.test.ts` assertion because Punjabi had already adopted the
script marker for every writing lesson. The failure named both new lesson IDs.

Declare `delivery: script` in each new writing lesson on an adopted track, then
regenerate modality, book, and narration artifacts. Before pushing, run the
cross-track marker test as well as the track-specific tests. Do not relax the
coverage assertion: an unmarked writing lesson would disappear from the
published script strand even though its authored modality says pen.
