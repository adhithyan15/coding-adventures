# `@coding-adventures/script-ductus`

**Ductus** is the old word for the movement of the pen: not what a letter looks
like, but the order and direction of the strokes that make it. A printed ஔ tells
you the shape and nothing about the hand. This package is about the hand.

It answers one question — *how is this letter written?* — and produces a
**filmstrip** that shows it: frame *k* draws strokes 1…*k* in ink over the
finished letter in pale grey, with a dot where the pen is.

```
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│ ▏      │  │ ▏      │  │ ▏    ▕ │  │ ▏ ⌒  ▕ │  │ ▏ ⌒▕ ▕ │
│ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │
└────────┘  └────────┘  └────────┘  └────────┘  └────────┘
 1. down     2. along    3. up the   4. over     5. down
 the left    the bottom  right side  the top     the middle
```

## Where it fits in the stack

```
data/scripts/*.{json,d/} ─► scriptdata.ts ─┐
  (canonical files and                     ├──►  ductusview.ts  ──►  filmstrip
   build-time shard fold) strokes/*.ts ────┤        (the join)        (SvgNode
                          (owned paths)    │                           tree, or
     _fonts/*.ttf  ──►  truetype.ts  ──────┘                           SVG text)
       (shipped fonts)   (real outlines)
                                            consumers:  language-ladder (live SVG)
                                                        the book pipeline (figures)
```

| module | what it knows |
|---|---|
| `scriptdata.ts` | the curriculum's canonical script data; ordinary JSON is imported directly and the four sharded inventories arrive through one fixed build-time virtual module |
| `strokes.ts` + `strokes/*.ts` | **how** a letter is written — a fixed public registry assembled from writing-system-owned pen-path modules |
| `truetype.ts` | **what** the letter looks like — a zero-dependency TrueType reader pulling the real outline out of the shipped font |
| `ductusview.ts` | the join — the filmstrip, as a tree of plain objects plus a serialiser |
| `headline-word.ts` | a Devanagari **word** — its cited letters' bodies (and ā stems), then one headline across the word, fitted to the printed word (uses `ink.ts`); a phrase, word by word |

## The design idea worth knowing

**The target shape comes from the font, never from a second drawing.** That makes
a whole class of error impossible to hide: a pen path that has drifted from the
letter it claims to draw shows up as ink sitting outside the grey. The tests
check it mechanically rather than by eye —

- `fractionOnInk` — the authored path must lie on the font's own ink
- consecutive segments must actually **meet**
- the strokes together must **cover** the glyph, not trace a convenient part of it

So a wrong pen path fails a test rather than shipping a plausible-looking lie.

The shard-aware module is deliberately bounded: it exposes only Japanese,
Perso-Arabic, and Urdu-Nastaliq, watches every contributing shard, and never
places one eager browser key per glyph in the bundle. Script Ductus and Language
Ladder install the same plugin, so their tests, development servers, and
production builds all read the same canonical data.

Release notes follow the same ownership rule. Add one strict
`CHANGELOG.d/NNNNN-HEADING-SLUG-<digest>.md` fragment with a level-3 heading;
do not edit or commit `CHANGELOG.md`. Run `npm run unshard:docs` only when a
local aggregate is useful, and `npm run check:doc-shards` before committing.

## Stroke order is a citation, not an opinion

A stroke *order* cannot be verified against a font — the font knows the shape and
nothing about the hand. So every letter in its `strokes/<owner>.ts` module carries
a `strokeOrderSource`, and the rule the curriculum applies (HL11 §5) is:

> **No citation → no pen path → no figure.**

A letter with no sourced order still ships, taught by recognition and by tracing
the printed shape, with the gap recorded rather than filled. Inventing a
plausible order would be worse than shipping nothing: a learner cannot tell an
invented order from an attested one and will drill it for years.

### Native writers' pen traces as a source (Bengali)

Most owners cite a teaching animation or a tracing guide. Bengali
(`strokes/bengali.ts`) cites something different: the tablet pen traces of
native Bengali writers that HP Labs India's LipiTk 4.0 Bangla recognizer keeps
as its prototypes (MIT licence). A trace records where the pen went down, which
way it moved and where it lifted, so counting traces gives the modal stroke
count, the part order, the start and the turning direction, each as a share of
the writers. It does not give proportions: the recognizer scales every trace to
a square. So the counts go into the citation's `variation`, no coordinate is
copied, and the path itself is fitted to the bundled Noto Sans Bengali outline
and checked against it at the default tolerances.

A letter is authored only where one order clearly wins. Bengali's headline is
the hard case: in isolated letters writers draw it first, last, partly or not
at all, depending on the letter, so ন, ক, ম and others whose traces split are
left out with the reason recorded in `data/scripts/bengali.json`.

### A tracing lesson as a source (Gurmukhi)

Gurmukhi (`strokes/gurmukhi.ts`) cites the Alphabet Tracing lesson of GNPS's
Gurmukhi Sikho app (Apache-2.0). For each letter, the developer entered ordered
checkpoints over the Noto Sans Gurmukhi letter, one list per pen-down stroke,
and the app makes a child reach them in order. That gives the order, the start,
the direction and the lifts; the record cites them and copies no coordinate.
The path is fitted to the bundled outline. The source draws the headline first,
and every record says this is a teaching order: fluent writers are often
described as adding the headline last. ਛ, ਨ and ਬ lift once less than the app,
down to Omniglot's copyist mode, which is an upper bound on native lifts.

### A teaching tool's formation arrows as a source (Malayalam consonants)

Seventeen Malayalam consonants (`strokes/malayalam.ts`, after ഴ) cite the
formation images of SPACE Kerala's *Thooval* (Society for Promotion of
Alternative Computing and Employment), a Malayalam alphabet teaching tool. Each
image marks where the pen starts (green), where it turns back along its own ink
(blue) and where it ends (red). Thooval is GPL-3.0, so only those facts are
cited, with the image linked at a pinned commit; nothing of it is copied.
Because Thooval makes the learner keep the pen down to the end of the letter,
it cannot show a lift, so the zero-lift claim is checked against two recordings
that could: Santhosh Thottingal's *hand* curves (MIT) and the *grahyam* samples
(no licence, cited as counts only). The paths retrace a stem wherever Thooval
turns back at its foot.

### A textbook's numbered movements as a source (more Malayalam letters and signs)

Twenty-two more Malayalam glyphs (`strokes/malayalam.ts`, after ബ) cite Rodney
F. Moag's *Malayalam: A University Course and Reference Grammar*, whose tables
number and arrow every movement of every letter and sign in a native writer's
hand: ക യ ഖ ങ ച ഛ ഞ ഥ ധ ഭ ഫ ള, ഏ, the anusvara ം and eight vowel signs drawn
alone (ാ ി ീ ു ൂ ൃ െ േ, read through `malayalamMarkSource`). The book is CC
BY-NC-SA 4.0, so only facts are cited, with each page's scan linked at a pinned
commit. Moag's numbers are movements, not lifts: each becomes one segment of a
single run, and the lift count again rests on *hand* and *grahyam*.

Two more follow: ജ (Moag's six movements; the short stem's descent is read
between arrows 2 and 3, at medium confidence) and the vowel sign ൈ, two coils
of െ with the one lift the gap between them forces.

### A children's tracing app as a source (Kannada digits and anusvara)

The Kannada digits ೧-೯ and the anusvara ಂ (`strokes/kannada.ts`, after ಠ)
cite Chimple, the literacy app of Bangalore's Sutara Learning Foundation:
its lesson pictures hold hidden, ordered tracing paths, one per pen-down run,
and its recorded traces agree. The lesson repository has no licence, so only
order, start, direction and stroke count are cited; nothing is copied. Each
digit is one run; the ring is one anticlockwise run, drawn after its carrier.
One source, designer-authored, so each record says confidence is medium. A
digit row with a cited ductus resolves its font like a letter
(`verifiedLetterFont`). The same source corrected ಃ, whose first path drew
the lower dot first.

### A school model and native writers as sources (Latin print letters)

Latin (`strokes/latin.ts`, keys `latin:<glyph>`) draws 31 print glyphs: a b
c d e g h i l n o p q r s t u w y ß, G and H cite the Grundschrift-App, made in
a research project of the Laborschule at Bielefeld University with the
Grundschulverband, whose ordered paths (one per pen-down stroke) a child traces
in order. Its repository has no licence, so only the order, start, direction
and lifts are cited, per letter level at a pinned commit, and no point is
copied. Native writers corroborate it: UJIpenchars2 (Prat et al., UCI dataset
177, CC BY 4.0) holds 120 pen traces of each Spanish character from 60 adults,
cited as counts. It is the only source for the marks: ñ's tilde (after the n,
left to right), the acute of á é í ó ú (after the letter, rising to the right
as about six in ten writers draw it), ü's dots (left first), and ¿ and ¡ (dot
last). Each precomposed letter is its own entry whose first stroke is its base
letter's. The outlines are `LatinPrint-Subset.ttf`, a renamed subset of SIL's
literacy typeface Andika, chosen because its a is the one-storey a every
source teaches (Noto Sans prints a two-storey a). The grave, circumflex,
cedilla, macron, æ and œ have no source; ä ö ë ï ÿ would be analogy only.

## Usage

```ts
import { DUCTUS, parseFont, ductusFor, ductusFilmstrip, svgMarkup }
  from "@coding-adventures/script-ductus";

const font = parseFont(await readFont("NotoSansTamil-Static.ttf"));
const letter = ductusFor("வ", "tamil");
const glyph = font.glyphFor("வ");

const strip = ductusFilmstrip(letter, glyph);
const svg = svgMarkup(strip.root);   // for the book pipeline
```

`ductusFilmstrip` returns a tree of plain `SvgNode` objects. `svgMarkup`
serialises it to text; the app walks the same tree with `createElementNS` and
never touches `innerHTML`. One description, two consumers.

## The printed filmstrip, and the ledger the book reads

The app calls `ductusFilmstrip` and draws the `SvgNode` tree straight into the DOM.
The book cannot: `human-language-data` is a plain Node CLI, this package needs a
Vite process (`scriptdata.ts` reads the sharded inventories through a virtual
module), and this package already depends on that one — so importing it back would
close a build-graph cycle.

The two therefore meet on data. `filmstrip-ledger.ts` writes
`code/learning/human-languages/data/ductus/filmstrip-geometry.d/`: for each
letter the book prints, its frames as escaped SVG fragments in one shared viewBox,
plus the citation, the font the outline came from, the pen-lift count and the
summary line. Every value in it is read back out of the tree `ductusFilmstrip`
produced — nothing is recomputed — so the book and the app cannot draw different
pictures.

```bash
npm run generate:filmstrip-ledger   # rewrite it
npm run check:filmstrip-ledger      # fail if it is stale
```

The check also runs as part of `npm test`, so a stroke edited here and not
regenerated fails this package rather than the book. Which letters get an entry is
decided by the curriculum, not here: the generator emits one entry per letter a
`script-filmstrip` target in `core/figure-generation.json` draws, plus every letter
of the lesson targets `human-language-data` derives. A sequence target (a list of
letters, or a word whose letters stand apart) contributes each of its `letters`,
and only when all of them are cited.

`DuctusOptions.highlightSegment` is what the printed strip turns on. Frames sit
side by side and nothing animates, so the part of the current stroke travelled
before this frame drops back to the muted tone and only the movement the caption
names is in ink. The live app keeps the default whole-stroke shading.

The ledger also sizes two things per letter instead of using one default.
`captionSizeFor` scales the caption type to the letter's own box, and
`penSizeFor` shrinks the pen line and the dot marking where the pen is on a
TINY mark. Below `TINY_STROKE_EXTENT` (150 font units of pen path; today only
the nukta ़, Devanagari ं and Gujarati ં, at 44 to 60 units) both are scaled by
`extent / 150`, with a floor of `MIN_TINY_PEN_SCALE` (0.3). Without it, the
34-unit dot was wider than the mark's whole movement. Every letter whose pen
path is 150 units or more keeps the defaults. An explicit `penWidth` or
`tipRadius` overrides the scaling, as an explicit `captionSize` does.

A Devanagari word lesson (`composition: "shared-headline"`) gets ONE entry
keyed by the whole word (`devanagari:मम`). `composeHeadlineWord` takes each
letter's own "lift, then draw the shirorekha rightward" stroke off, places the
letters' bodies at the font's advance widths (`Font.advanceFor`), and ends with
one headline over the whole word: most native writers draw it last (HP Labs
India's LipiTk Devanagari data: 82% of 2,706 consonant prototypes; about 5%
first). A word whose composed path does not fit the printed word at the
default tolerances is refused, not drawn.

One sign may join a word: ā (ा), straight after a consonant. Its stem is drawn
after the consonant's body and its piece of headline becomes part of the
word's one headline; its place is cited on its mark record (the cited आ draws
the same bar after the body and before the headline) and read from there by
`HEADLINE_WORD_SIGNS`. A phrase (words separated by single spaces) is composed
word by word by `composeHeadlinePhrase`, each word with its own headline, and
refused whole if any word fails.

```ts
const word = composeHeadlineWord("नाम", "devanagari", font);
if (word.ok) buildFilmstripEntry(word.ductus, word.outline, fontPath);
else console.log(word.reason); // e.g. मथ: the shared headline is only 88.2% on ink

const phrase = composeHeadlinePhrase("मम नाम", "devanagari", font);
if (phrase.ok) for (const w of phrase.words) buildFilmstripEntry(w.ductus, w.outline, fontPath);
```

## No DOM, no filesystem

Nothing here touches `document` or reads a file. Fonts arrive as an
`ArrayBuffer`. That is what lets every claim above be tested without a browser —
and it is why this is a package rather than part of the app.

## Why it is a package

These modules lived in `code/programs/typescript/language-ladder/src`, which made
them reachable by the app and by nothing else: nothing under `code/packages/` may
depend on something under `code/programs/`, so the book generator — the other
consumer that wants filmstrips, as printed figures rather than live SVG — could
not import them at all. `ductusview.ts`'s own header anticipated the move: *"the
book pipeline can take the serialised string instead."* Now it can.

## Tests

```bash
npm install && npx vitest run
```

Authored paths and their exact geometry/filmstrip evidence use the same owner
name under `src/strokes/`, `tests/strokes/`, and `tests/ductusview/`. Tamil goes
one level deeper: every existing record and both evidence suites use the same
ASCII `U-<CODEPOINT>` owner below `tamil/`, while `tamil.ts` remains assembly
only. Adding an ordinary glyph changes only its owner files; `strokes.ts`
remains the bounded public facade and duplicate-rejecting assembly point.
The six Tamil vowel signs written as separate symbols beside their consonant
(ா ி ீ ெ ே ை) are owners of the same kind, cited to the native-writer pen
traces in HP Labs India's LipiTk Tamil recognizer; the book decides where each
is drawn in a word (`human-language-data`'s `WRITTEN_SIGN_SIDES`). The pulli ்
(`U-BCD`) is the last Tamil owner: one short dab inside the font's disc, its
order (body first, then the dot, one lift) cited to Abhinaya Rajarajan's
*Varai* recordings of the 18 consonants with pulli (one writer, facts only,
confidence medium).
Gujarati's eleven signs (ા િ ી ુ ૂ ે ૈ ો ૌ ં ઃ) sit at the end of the Gujarati
owner, keyed `gujarati:<sign>` like its letters, and take their source from the
sign's mark record (`gujaratiMarkSource`): order, start, direction and lifts
from KanoAI's hand-made barakhadi templates, paths fitted to Noto Sans
Gujarati. `tests/strokes/gujarati-marks.test.ts` and
`tests/ductusview/gujarati-marks.test.ts` hold their evidence.
Eight Devanagari signs (ु ू े ं ़ ् ृ ँ) sit at the end of the Devanagari
owner the same way, sourced through `devanagariMarkSource` to native writers'
pen traces in HP Labs India's LipiTk Devanagari recognizer (counts and shares
only). Those writers wrote each sign alone, so the entries draw the sign by
itself and say nothing about its order against a consonant or the headline;
ा ि ी ो ौ ै and ः are left out because Noto prints a headline piece the traces
never draw, or the traces split. `tests/strokes/devanagari-marks.test.ts` and
`tests/ductusview/devanagari-marks.test.ts` hold their evidence.

More than 2,200 tests cover the registry, paths, font fit, provenance, and
rendering. `jsdom` is a devDependency for exactly two of them: the SVG
serialiser's escaping is checked by handing its output to a **real** parser and
asserting that a hostile caption cannot break out of an attribute or smuggle in a
`<script>`. A string comparison would pass on markup no browser accepts, which is
the bug those two tests exist to catch.
