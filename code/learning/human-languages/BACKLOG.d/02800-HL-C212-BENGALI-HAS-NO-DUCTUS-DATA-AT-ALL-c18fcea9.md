## HL-C212 — Bengali has no ductus data at all

**Filed deliberately to be picked up later.** Owner: "File Bengali missing ductus
for now in the backlog. We should circle back to that. Maybe we can launch a
separate agent for it to go look for it, or we can circle back later."

Not blocking anything today. Recorded so it is not rediscovered a third time.

### The gap

`code/packages/typescript/script-ductus/src/strokes/` covers **12 scripts** —
arabic-family, chinese, cyrillic, devanagari, gujarati, hebrew, japanese,
kannada, malayalam, tamil, telugu, and the perso-arabic / urdu-nastaliq variants
inside arabic-family. **349 glyphs total.**

**Bengali is absent.** There is no `src/strokes/bengali*`, no entry in `DUCTUS`,
and no row in the per-script `counts` map in `tests/stroke-ownership.test.ts`.

### Why it matters more for Bengali than for most tracks

Measured on main:

| bengali | |
|---|---:|
| script lessons | 45 |
| glyphs taught | 26 |
| glyphs shown | 48 |
| **glyphs shown but never taught** | **22** |
| closure violations | 41 |
| headwords without romanization | 0 |

Bengali teaches more script lessons than most tracks and still shows 22 shapes it
never teaches — the largest never-taught count in the corpus. Its own agent
measured the ceiling: with every script lesson hypothetically pre-taught, the
corpus floor is 36, so **reordering is worth at most five more and the rest is a
glyph-inventory problem.** Those lessons are being written with no stroke data
underneath them.

It also blocks the filmstrip work. The owner asked for stroke-order filmstrips
because "just writing out instructions is not going to help" — a filmstrip is
rendered from ductus strokes, so Bengali cannot have one at all until this
exists.

### What the work actually is — research, not authoring

**Stroke order must be SOURCED AND CITED, never invented.** That is a standing
repo rule and the reason several tranches this week deliberately shipped fewer
lessons: the Marathi agent refused to write ग, घ, ख lessons until it had queried
the Wikimedia Commons API and confirmed the files existed, and the Sanskrit agent
left ऋ and ङ untaught for exactly the same reason.

**The method Marathi proved, and the shape to copy:**

1. Query the Wikimedia Commons API for each Bengali glyph before planning
   anything. Marathi found `File:Deva-<glyph>-order.gif` (Opiaterein) for
   consonants and `File:Devanagari <glyph> stroke order.svg` (Saurmandal) for
   independent vowels. Establish the equivalent Bengali naming pattern first, and
   record which glyphs have nothing.
2. Where no animation exists, the existing fallback is citing the Unicode chart
   by codepoint, as the Marathi mātrā lessons already do. That is honest; a
   guessed stroke path is not.
3. **Shard per glyph, following Tamil.** Tamil is the model: 27 files under
   `src/strokes/tamil/` named `U-<codepoint>.ts`, with 27 matching evidence files
   under `tests/strokes/tamil/`. Per-glyph sharding is also what excludes Tamil
   from the corpus-wide `nonTamilDataHash` pin, so it is the shape that does not
   collide with parallel work. Every other script is still one monolithic file.
4. Data shape per glyph: ordered `strokes[]`, each with ordered `segments[]`,
   each carrying a human `label` and a coordinate `path` of `{x, y}` points. The
   labels are read aloud and printed as filmstrip captions, so they must be
   written for a learner ("curl around the upper loop"), not for a font engineer.

### Sequencing note

Bengali's own agent is mid-programme on its glyph inventory. This entry does not
block it — lessons can teach a letter without ductus data; they simply cannot
carry a filmstrip. Land ductus first if a dedicated agent is launched, so the
filmstrips arrive with the lessons rather than needing a second pass over them.
