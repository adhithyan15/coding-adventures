## HL-C300 — Filmstrips ship: which scripts render, and Bengali has no ductus at all

The book can now print a **stroke-order filmstrip**: one letter, one frame per
labelled movement, the finished glyph behind in pale grey, the movement being
added in ink, and the segment's own authored words underneath. The new figure
kind is `script-filmstrip` in `core/figure-generation.json`; the geometry comes
from `data/ductus/filmstrip-geometry.json`, which `script-ductus` regenerates
and byte-checks in its own suite. Three targets ship as proof: Tamil **அ**,
Devanagari **आ**, Perso-Arabic **چ**.

Recording what the sweep across every authored script actually showed, so the
next agent does not re-measure it.

### Every script in `script-ductus` renders — all 349 glyphs resolve an outline

One representative letter per script was built into a ledger entry and rendered
through the book's own figure renderer, then rasterised and looked at. Nothing
failed, and no glyph in any script is missing an outline in the font its
canonical inventory names:

| script | glyphs with a cited ductus | outline found | how it reads |
| --- | --- | --- | --- |
| arabic | 32 | 32 | good |
| chinese | 43 | 43 | good; long strips (up to 17 movements) wrap onto a second row |
| cyrillic | 33 | 33 | good |
| devanagari | 43 | 43 | good |
| gujarati | 44 | 44 | good |
| hebrew | 22 | 22 | good |
| japanese | 15 | 15 | good |
| kannada | 13 | 13 | good |
| malayalam | 13 | 13 | good |
| perso-arabic | 24 | 24 | good |
| tamil | 27 | 27 | good |
| telugu | 9 | 9 | good |
| urdu-nastaliq | 31 | 31 | good, with the caveat below |

So "which scripts render well" has a short answer: **all of them**. The gaps
that matter are gaps in the DATA, not in the drawing.

### Bengali has no ductus data at all — this is the highest-value follow-up

`src/strokes/` has no `bengali.ts`. Bengali is not in `DUCTUS`, has no cited
stroke order for a single letter, and therefore cannot have a filmstrip in any
lesson, however the figure pipeline improves.

Three things make it the best next investment:

1. **The font is already shipped.** `_fonts/NotoSansBengali-Static.ttf` is in
   the repository, so the outline half of every frame — the half that cannot be
   hand-drawn — is available the moment a pen path exists.
2. **Bengali is the track with 22 never-taught glyphs.** It is the track where a
   learner most often meets a character the course never showed them how to
   write, so filmstrips buy more there than anywhere else.
3. **Nothing else blocks it.** The engine, the ledger, the figure kind and the
   check gate all exist and are proven on three other scripts.

**Stroke order must be sourced and cited, never invented.** That is a standing
repository rule and `strokes.ts` enforces it structurally: `LetterDuctus.source`
is required, and the tests refuse a pen path that does not lie on the font's own
ink. The Marathi agent set the working precedent this week by querying the
Wikimedia Commons API for existing stroke-order animations before authoring
anything. Do that first for Bengali; author only the letters a real source
covers, and leave the rest absent rather than guessed.

### Three real caveats found while sweeping

- **Urdu-Nastaliq is drawn against a Naskh fallback outline.** The pen paths are
  fitted to the vendored Noto Naskh shape, and its own source notes say so; the
  app's true Nastaliq face is a separate thing. A Nastaliq filmstrip will look
  right and still not be Nastaliq. Fix by sourcing a Nastaliq outline, not by
  redrawing the pen paths.
- **There is no ductus for a composed consonant + matra.** `DUCTUS` holds base
  letters and independent vowels only. आ can be filmstripped because the
  ā-stroke is part of the letter; क + ी cannot, because no such record exists.
  Any lesson wanting "the letter, then the matra" needs new cited records for
  the composed forms.
- **Telugu (9), Kannada (13), Malayalam (13) and Japanese (15) have thin
  coverage.** Those tracks will run out of filmstrippable letters long before
  they run out of letters to teach.

### What wiring lessons to filmstrips still needs

Nothing in this change touches a lesson file. A `script-filmstrip` target names
the lesson whose book the figure belongs to, but the lesson does not yet
reference the image. To wire them up, per writing lesson:

1. add a target to `core/figure-generation.json` (`kind`, `lessonId`, `script`,
   `glyph`, `output`);
2. run `npm run generate:filmstrip-ledger` in `script-ductus`, then
   `npm run generate:figures` in `human-language-data`;
3. add one Markdown image to the lesson —
   `![How அ is written](figures/TA-S119-letter-a-filmstrip.svg)`. The book
   pipeline already rewrites `.svg` to the `.pdf` its deterministic
   `rsvg-convert` step produces, so no LaTeX change is needed.

The writing lessons are the natural first wave: they already carry the letter as
`headword` and `delivery: script`, so the target is derivable from the lesson
that will print it.
