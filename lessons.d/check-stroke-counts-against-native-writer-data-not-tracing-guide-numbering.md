---
category: Testing & coverage
---

# check stroke counts against native-writer data, not tracing-guide numbering

**Context:** `script-ductus`, the Telugu ductus in `src/strokes/telugu.ts`
and the records in `code/learning/human-languages/data/scripts/telugu.json`.

**What happened:** most Telugu consonants cite the "Sathish Shanmugam, Write
Telugu Alphabets" tracing guides. Each guide numbers its directional
movements, and every numbered movement became a separate pen-down run. ఞ got
7 pen lifts, థ and మ 6, and ట, ధ, భ and ఢ 5. Across the 43 Telugu glyphs the
ledger averaged 2.84 lifts. HP Labs India's native-writer samples (the
prototypes in the MIT-licensed LipiTk 4.0 Telugu recognizer, from
hpl-telugu-iso-char) average a modal 0.86. For these seven glyphs, 0% of the
writers used our stroke count. Every test passed, because the tests pinned
whatever the ductus said, and the filmstrips looked plausible.

**Why:** a tracing guide numbers movements so a child can follow them one at
a time. A number is not a pen lift. Native writers draw a Telugu letter's body
in one run and lift only for its detached parts (the talakattu flourish when it
stands apart, a dot, a separate stem or bar).

**Fix:** keep each numbered movement as a segment inside one stroke and lift
only at the body/detached-part boundaries. The seven glyphs went to 2, 2, 1,
1, 1, 1 and 2 lifts, matching the native modal stroke count (78% to 94% of
samples). Each record's `strokeOrderNote` gives the native share and says the
counts are counts only.

**Do differently:** before you turn a source's numbering into strokes, look
up how many strokes native writers use. The pen-up counts are in
LipiTk-derived tables such as the HP Labs India Telugu and Devanagari ones.
Only lift where those writers lift. If a source gives K numbered movements
and the native mode is far below K, the numbers are movements, not lifts.
