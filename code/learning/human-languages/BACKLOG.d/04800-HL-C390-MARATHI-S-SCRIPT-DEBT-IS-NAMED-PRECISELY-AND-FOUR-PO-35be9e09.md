## HL-C390 — Marathi's script debt is named precisely, and four points of it are cheap

**Marathi is the next Indian track to move, and its script points are the place
to start.** This entry records the survey that chose it and the verification
that its notes are accurate, so the next unit does not redo either.

### Why Marathi

A1 exam-point coverage across the twelve Indian tracks:

| track | covered | track | covered |
|---|---|---|---|
| sanskrit | 142/164 (87%) | bengali | 141/244 (58%) |
| hindi | 239/282 (85%) | gujarati | 122/210 (58%) |
| kannada | 194/258 (75%) | urdu | 134/234 (57%) |
| tamil | 175/262 (67%) | marathi | 162/301 (54%) |
| malayalam | 163/243 (67%) | marwadi | 79/197 (40%) |
| telugu | 214/326 (66%) | punjabi | 137/227 (60%) |

**Probed-but-short is zero in every one of them**, so every remaining point is an
unmapped point that needs teaching rather than a probe that needs mending.

Marathi has the largest addressable gap — 139 unmapped, marwadi being blocked by
`HL-C380` — and it is **Devanagari**, so the whole Hindi script campaign
transfers: the glyph-inventory headword rule, `measureScriptClosure`, the
zero-new writing contract, the five wiring files.

### The debt, verified rather than assumed

Checked with the `HL-C386` rule — a glyph counts as taught only when its script
lesson's headword is a **glyph inventory** (every token, with U+25CC stripped, at
most two code points). The helper was sanity-checked against letters the notes
claim *are* taught, and it found every one of them at the chapter the notes
imply: ट ठ ड ण प at chapter 7, ब at 3, भ at 2, म at 1.

**Every letter the notes call missing is genuinely missing.** ढ, फ, ङ, ञ, ई, ओ,
ऐ, औ, all ten digits and the danda have no lesson whose headword is an
inventory containing them.

**Marathi's script closure is currently clean**: 0 violations, 0 never-taught
glyphs, 50 glyphs taught. The untaught letters appear in no lesson body at all.
That is a better starting position than Hindi's and it carries one obligation:
**an example word may use only the 50 taught glyphs plus the one being taught**,
or the track's clean sheet is the thing that pays for it.

### The order to take them in

**Unit one — three points, three lessons.** `MR-A1-OR-05` (ढ, the fifth of the
retroflex row), `MR-A1-OR-07` (फ, the labial row's gap) and `MR-A1-PU-01` (the
danda, and the full stop that has largely replaced it). Each is one lesson for
one shape, which is exactly how `MR-A1-OR-06` closed at chapter 39 — the route
is known and the track has the precedent in `MR-W39-tha`.

**Unit two — `MR-A1-OR-12`, four lessons.** The independent ई, ओ, ऐ and औ. The
note flags the hook and the trap together: keep it apart from `MR-A1-OR-14`,
which is the **ai and au signs** and closed at chapters 56 and 59. A sign hangs
on a consonant; a letter starts a word. The reader who has drawn ौ and ै meets
their standing forms, which is a real contrast rather than a repetition. It also
unblocks a number — **ऐंशी**, *eighty*, opens with the independent ऐ.

**Later, and bigger.** `MR-A1-OR-24` (the ten digits) is called the cheapest
large gap in the file and unlocks a whole paper part, but Hindi's `HL-C387`
established that digits need **two** lessons — the shapes, then something that
uses them — because there is nowhere to pay the atom off otherwise. `MR-A1-OR-20`
(the frequent conjuncts) and `MR-A1-OR-21` (reph and rakaar) are each their own
unit; the note on OR-21 is worth reading before starting, because the rakaar has
been **shown** since chapter 18 in मित्र and never named.

### Where they go

The track has 63 chapters. Script lessons sit throughout rather than in one
block — `MR-W39-tha` at 39, `MR-W40-i-independent` at 40, `MR-W56-au-matra` at
56, `MR-W59-ai-matra` at 59 — and punctuation has precedent too, in
`MR-W33-question-mark` and `MR-W34-comma`. A new chapter 64 is the clean home
for unit one.
