## HL-C241 — Urdu's ladder moved to the front, and redistribution turns out to have a floor

HL-C240 said the remaining Urdu closure debt was bought by moving letters
earlier rather than teaching more of them. That has now been done in full:
all fourteen Nastaliq letter lessons were lifted out of chapters 16–18 and
interleaved through chapters 1–8, two content lessons apart, so the ladder now
runs from sequence 35 to sequence 295 instead of 730 to 1000.

**Closure violations: 46 → 41.** Glyphs shown but never taught stayed at 22.
Headwords without romanization stayed at 0. The number the exposure rule was
quietly carrying fell hard — glyphs exempted by the headword rule dropped
2046 → 1925 corpus-wide, and 263 → 142 inside Urdu — which is the honest half
of this result: far more of what the reader sees is now genuinely taught rather
than excused.

### The floor is 41, and it is not a matter of trying harder

41 is not where the redistribution ran out of care. It is the arithmetic
maximum of resequencing, and it was measured before any file was edited by
simulating placements against the corpus's own load-bearing glyph sets.

Assume the ideal: every one of the fifteen taught glyphs available before every
lesson that needs it. Exactly five lessons become clean — `UR-C02-mera-naam`,
`UR-C03-practice`, `UR-C04-practice`, `UR-C06-hona`, `UR-C06-ana` — and the
other 41 need at least one of the 22 glyphs no lesson teaches. There is no
ordering of fourteen lessons that does better, and the shipped ordering hits
all five.

Then assume something much stronger: that all 22 missing letters get authored
and taught at the policy's own pace of one letter per two lessons, in first-
demand order. Simulated over the real corpus:

| letters taught | spacing 2 | spacing 3 |
|---|---|---|
| 14 | 50 | 51 |
| 20 | 44 | 45 |
| 26 | 40 | 45 |
| 32 | 40 | 45 |

Teaching the entire alphabet the track uses, as fast as the policy permits,
leaves 40. The reason is a wall rather than a slope: eight of the untaught
glyphs — **ب چ ئ ز ح ظ ق ع** — are first demanded at content lesson 27 and 28,
inside chapter 7, and no ladder can deliver 33 letters before lesson 27 without
becoming the course, which is the thing `minLessonsBetweenScriptSegments: 2`
exists to prevent. **The remaining Urdu debt is not a ladder problem. It is a
prose problem: chapters 7–15 print review words in Nastaliq that the reader
provably cannot decode yet.**

### The cheapest real lever, and it already exists in the corpus

Seven content lessons already introduce a named letter atom and explain the
shape in prose, but get no closure credit, because `measureScriptClosure` only
credits lessons with `type: writing` or `delivery: script`:

| lesson | atom | glyph |
|---|---|---|
| `UR-C06-bolna` | `UR-SCRIPT-BE-LETTER` | **ب** |
| `UR-C07-sochna` | `UR-SCRIPT-CHE-LETTER` | **چ** |
| `UR-C07-samajhna` | `UR-SCRIPT-DO-CHASHMI-HE` | **ھ** |
| `UR-C07-parhna` | `UR-SCRIPT-RRE-LETTER` | **ڑ** |
| `UR-C09-bhai` | `UR-SCRIPT-HAMZA-YE` | **ئ** |
| `UR-C14-juta` | `UR-SCRIPT-TE-VS-TTE` | **ت ٹ** |
| `UR-C15-garmi` | `UR-SCRIPT-GAF-LETTER` | **گ** |

That is eight of the 22 untaught glyphs whose pedagogy is already written and
already sits at the right point in the book. Splitting each into a short script
segment beside its content lesson is authoring the corpus mostly already did.
Do **not** simply relabel these lessons `delivery: script` — that would credit
them with every glyph in their bodies, including untaught ones, which is
laundering rather than teaching.

### The order to author the rest in, measured rather than guessed

Closure is all-or-nothing per lesson, so the payoff is violently back-loaded.
Greedy over the corpus, assuming redistribution is already done:

`ج ت د خ ح ھ ٹ چ ڑ ئ ب ف ظ گ ز ق ض ع`

Twelve letters take 46 → 20. **The thirteenth, ظ, alone takes 20 → 7**, because
**حافظ** appears in the body of fifteen lessons and ح ف ظ are its only untaught
shapes. Anyone authoring fewer than thirteen letters should expect almost
nothing to move, and should say so up front rather than discover it afterwards.

### Four of the 22 "never taught" glyphs are a measurement artifact

**ه** (U+0647), **ء** (U+0621), **َ** (U+064E) and **ِ** (U+0650) appear
nowhere in Urdu words in this track. They occur only inside Arabic and Persian
etymon citations that are deliberately spelled the source language's way:
**هَوَاء** in `UR-C15-hawa`, **لَال** in `UR-C13-lal`, **پرسانِ حال** in
`UR-C08-puchhna`, **مِیرا** in `UR-C13-kala`. The report's cousin-layer
exemption — "shows another script, context, never charged to the budget" —
does not fire, because Arabic and Urdu are the same script system as far as
`SCRIPT_SYSTEMS` is concerned. These are scholarship, not typos, and repairing
them would damage the etymology layer. The fix belongs in the measurement:
an etymon citation in a sibling language's orthography should be exempted the
way cousin script already is.

### What this redistribution cost, stated plainly

- **Chapters over the 12-atom budget: 23 → 26 corpus-wide.** Urdu chapters 1
  (13), 5 (14) and 7 (13) crossed the line; chapters 3 (16), 4 (19) and 6 (16)
  were already over. Letters carry atoms, so moving them into early chapters
  loads those chapters. The fix is a chapter split in Urdu 3–7, not a reversal.
- **Payoffs below the 0.5 representativeness floor: 80 → 81.**
- **Urdu's hands-free chapter-prefix reach: 60 → 44 lessons.** This is the
  sharpest cost and it is structural. `drivablePrefix` counts, per chapter, how
  many lessons you can drive by voice before the first one that needs your
  hands; interleaving a letter lesson into chapter 4 truncates chapter 4's
  prefix at its first script lesson. Before, all fifteen letter lessons were
  parked past chapter 16 and chapters 1–15 were uninterrupted. A letter lesson
  is inherently `pen`, so the mitigation is not reordering: it is giving each
  letter lesson a detachable writing segment so its recognition half is
  voice-drivable, which is what already rescues 26 other Urdu lessons.

### Two mechanical notes for the next person

- **`tests/integration.test.ts` hard-codes Persian and Urdu chapter sizes in a
  shared loop.** Chapters 3, 4 and 5 assert one length for both tracks. Urdu's
  counts now differ (7, 9, 6 against Persian's 5, 6, 4), so those three
  assertions were made per-language with a comment; Persian's expected values
  are untouched. Any further Urdu resequencing will hit the same three lines.
- **Sharded curriculum extension files must be numbered contiguously by ten in
  sorted order.** `check:shards` passes on half-step ordinals such as `0035`,
  but `curriculum-shards.test.ts` rebuilds the shard set and compares filenames,
  so inserting a node means renumbering every file after it. Urdu's curriculum
  monolith was removed by its migration, so `--unshard` refuses and the
  renumbering has to be done by hand.
