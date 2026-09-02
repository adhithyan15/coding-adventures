## HL-C212 — Bengali stroke-order sources: 11 vowels exist, no consonant does, and the real blocker is a missing script inventory

HL-C212 asked whether Bengali stroke-order sources exist at all, and the owner's
instruction was *"There should be something somewhere."* Answered by querying the
Wikimedia Commons API rather than by recall, with a control. **Something is
there: every independent vowel. Nothing is there for any consonant.** And the
sweep turned up a blocker HL-C212 and HL-C300 both missed.

### The blocker, first — Bengali has no `data/scripts/bengali.json`

HL-C300 said of Bengali ductus: *"Nothing else blocks it."* That is not true, and
it is the single most useful thing in this entry.

There is **no Bengali script inventory at all**. `data/scripts/` holds thirteen
inventories — arabic, chinese, cyrillic, devanagari, gujarati, hebrew,
japanese.d, kannada, malayalam, perso-arabic.d, tamil.d, telugu,
urdu-nastaliq.d — and Bengali is not among them. `grep '"script": "bengali"'`
over `data/` returns nothing.

That matters because a ductus record cannot stand alone. `tests/strokes.test.ts`
requires a 1:1 correspondence between the letters that carry `penLifts` /
`strokeOrderSource` in a script's canonical JSON and the letters in `DUCTUS`,
and `tests/support/stroke-honesty.ts` resolves a letter's FONT by finding the
`SCRIPTS` row whose `strokeOrderSource.url` matches the ductus's. With no
`bengali.json` there is no row to match, so a Bengali pen path cannot be
verified and cannot ship, however good its source.

So the work has three steps, not one, and the first is not ductus:

1. author `data/scripts/bengali.json` and add it to `SCRIPTS` in
   `script-ductus/src/scriptdata.ts` (the type comment in
   `human-language-data/src/types.ts` says this is deliberately low-ceremony:
   *"you drop in a `data/scripts/<script>.json` and point a track at it"*);
2. carry `penLifts` + `strokeOrderSource` on the eleven vowels below;
3. author the pen paths, sharded per glyph under `src/strokes/bengali/`
   following Tamil, and add `bengali` to `ownerNames` in
   `tests/stroke-ownership.test.ts`.

The font half is genuinely ready: `_fonts/NotoSansBengali-Static.ttf` is
vendored and carries an outline for all eleven vowels (bounds measured, not
assumed).

### What Commons actually has

The Bengali equivalent of Marathi's `File:Deva-<glyph>-order.gif` is
**`File:Beng <glyph> order.gif`** — note the spaces, not hyphens. There are
**eleven**, all uploaded by **Sriveenkat** on 2024-01-27, all 600x600,
all **CC BY-SA 4.0**, all in `Category:Animation of Bengali letters`:

| glyph | codepoint | file | frames |
|---|---|---|---:|
| অ | U+0985 | `File:Beng অ order.gif` | 61 |
| আ | U+0986 | `File:Beng আ order.gif` | 101 |
| ই | U+0987 | `File:Beng ই order.gif` | 74 |
| ঈ | U+0988 | `File:Beng ঈ order.gif` | 83 |
| উ | U+0989 | `File:Beng উ order.gif` | 71 |
| ঊ | U+098A | `File:Beng ঊ order.gif` | 81 |
| ঋ | U+098B | `File:Beng ঋ order.gif` | 83 |
| এ | U+098F | `File:Beng এ order.gif` | 40 |
| ঐ | U+0990 | `File:Beng ঐ order.gif` | 64 |
| ও | U+0993 | `File:Beng ও order.gif` | 46 |
| ঔ | U+0994 | `File:Beng ঔ order.gif` | 61 |

A **second, independent** set covers the same ground: twelve
`File:স্বরবর্ণ <vowel>.gif` (720x720, CC BY-SA 4.0, by ব্যা করণ), which adds
অ্যা and is otherwise the same eleven. Two independent attestations of the same
order is a stronger citation than one, and the entries should say so.

These are **not** four-panel buildup diagrams like the Devanagari SVGs. They are
frame-by-frame pen animations: a moving pen tip laying ink over a pale-grey
target letter. Diffing consecutive frames gives the pen's position at every
step, so **start point, direction and stroke order are all recoverable
mechanically** — richer source material than any other script in the corpus has.

### And what Commons does not have — no consonant, under any name

Checked explicitly, not assumed. All 32 base consonants
(ক খ গ ঘ ঙ চ ছ জ ঝ ঞ ট ঠ ড ঢ ণ ত থ দ ধ ন প ফ ব ভ ম য র ল শ ষ স হ) were queried
under four naming patterns — `File:Beng <c> order.gif`, `File:Beng-<c>-order.gif`,
`File:স্বরবর্ণ <c>.gif`, `File:ব্যঞ্জনবর্ণ <c>.gif` — 128 titles. **Every one is
missing.** The same query shape returns `PRESENT` for `File:Deva-क-order.gif` and
`File:Deva-ग-order.gif`, so the method is not silently failing.

`Category:Animation of Bengali letters` has 23 members and all 23 are vowels.
There is no `Category:Bengali stroke order` — Commons has stroke-order
categories for CJK, Devanagari, Hangeul, Hiragana and Katakana only.

The trap to avoid: **`File:Beng-<glyph>.png` covers vowels AND consonants
(Opiaterein, CC BY 3.0) and is NOT stroke order.** Its own description reads
"PNG file of the Bengali character ক" — a static 200x200 picture of the letter.
Opiaterein made ordered GIFs for Devanagari and static PNGs for Bengali. Anyone
scanning filenames will find 48 Bengali files by the author of the Devanagari
stroke-order set and conclude the coverage is complete. It is not.

So Bengali ductus coverage is **11 of 11 independent vowels, 0 of 32
consonants** — and the never-taught glyphs HL-C218 measured down to 11 are
mostly consonants. The filmstrip win is real but partial, and no amount of
further searching changes the consonant half. Where an animation does not exist
the established fallback is citing the Unicode chart by codepoint, as the
Marathi matra lessons do. A guessed pen path is not an option.

### Pen-down runs, read off each animation

Read from the plotted trajectories, not inferred from the letter's shape. Two
detectors were used and disagreed, so every count below was also looked at:

| glyph | pen-down runs | penLifts | order the animation shows |
|---|---:|---:|---|
| অ | 3 | 2 | matra L->R; body spiral inner->outer clockwise, ending at the open upper-left tip; tail down-right then up the right upright |
| আ | 3 | 2 | body spiral; tail then up the inner upright; matra L->R carrying the right a-kar upright |
| ই | 2 | 1 | body spiral then the long tail; matra L->R continuing into the upper hook |
| ঈ | 2 | 1 | as ই plus the i-hook in the same run; matra + upper hook |
| উ | 2 | 1 | stem down, shelf right, bowl round; matra L->R continuing into the upper hook |
| ঊ | 3 | 2 | stem + shelf + bowl; the lower u-hook; matra + upper hook |
| ঋ | 4 | 3 | tall stem top-to-bottom; the `<` wedge; the upper-left flag; the right hook into its upright |
| এ | 1 | 0 | one unbroken spiral — no matra on this letter |
| ঐ | 2 | 1 | the এ body; then the ai-diagonal up into its flag |
| ও | 2 | 1 | the head loop clockwise; then the bowl |
| ঔ | 2 | 1 | the ও body; then the au-diagonal up into its flag |

**One honest caveat, and it is structural.** In ই, ঈ, উ and ঊ the upper hook
begins on ink the matra already laid. A frame-diff sees new ink only, so it
cannot tell a genuine pen lift from a silent retrace back along the matra. Those
four are recorded as one run because that is the simpler reading, but the count
is 2 rather than 1 if the hook is a separate run, and the animation cannot
settle it. Do not present those `penLifts` as measured without saying so — or
find a second source that does settle it. The same ambiguity is why the
blob-contiguity detector and the centroid-jump detector disagreed: a stroke that
RESTARTS on existing ink looks contiguous to the first and looks like a lift to
the second.

### Method, so it is not re-derived

- Enumerate, do not guess: `action=query&list=allpages&apnamespace=6&apprefix=Beng`
  found the naming pattern; `list=categorymembers` on
  `Category:Animation of Bengali letters` bounded the set; a 128-title
  `titles=` batch proved the consonant absence.
- Always run a control. A query shape that returns nothing for Bengali and
  nothing for a file you know exists has told you nothing.
- The pen path is recovered by diffing consecutive coalesced frames and taking
  the centroid of the newly-dark pixels. Frame 0 of these GIFs is the FINISHED
  letter (the loop's last frame), so start from the frame with the least ink.
- Order and direction come from the animation; GEOMETRY must still be fitted to
  Noto Sans Bengali, because `stroke-honesty.ts` checks every point against the
  shipped font's own ink. The animation is a different typeface and its
  coordinates are not transferable — the same rule KanjiVG taught us.

### Not blocking, and deliberately not done here

This was found while unblocking Devanagari ऋ, whose citation had no ductus. That
one is fixed. Bengali is left un-authored on purpose: creating
`data/scripts/bengali.json` is the very artifact Bengali's own agent is
mid-programme on (HL-C201, HL-C218), and a ductus branch inventing a second one
would collide with it. The sources are now known, listed and licence-checked, so
the authoring pass is short whenever that inventory lands.

### One more thing the sweep settled

Every other track was checked for the failure ऋ had — a `strokeOrderSource` with
no matching ductus. **There are none.** Claims carrying a source, counted per
inventory: arabic 32, chinese 43, cyrillic 33, devanagari 44, gujarati 44,
hebrew 22, kannada 13, malayalam 13, telugu 9, japanese.d 15, perso-arabic.d 24,
tamil.d 27, urdu-nastaliq.d 31 — 350, exactly the size of `DUCTUS`, and matching
its per-script counts row for row. ऋ was the only one, and it is closed.
