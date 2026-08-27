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
| `scriptdata.ts` | the curriculum's canonical script data; ordinary JSON is imported directly and the three sharded inventories arrive through one fixed build-time virtual module |
| `strokes.ts` + `strokes/*.ts` | **how** a letter is written — a fixed public registry assembled from writing-system-owned pen-path modules |
| `truetype.ts` | **what** the letter looks like — a zero-dependency TrueType reader pulling the real outline out of the shipped font |
| `ductusview.ts` | the join — the filmstrip, as a tree of plain objects plus a serialiser |

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

## Stroke order is a citation, not an opinion

A stroke *order* cannot be verified against a font — the font knows the shape and
nothing about the hand. So every letter in its `strokes/<owner>.ts` module carries
a `strokeOrderSource`, and the rule the curriculum applies (HL11 §5) is:

> **No citation → no pen path → no figure.**

A letter with no sourced order still ships, taught by recognition and by tracing
the printed shape, with the gap recorded rather than filled. Inventing a
plausible order would be worse than shipping nothing: a learner cannot tell an
invented order from an attested one and will drill it for years.

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
name under `src/strokes/`, `tests/strokes/`, and `tests/ductusview/`. Adding an
ordinary glyph changes only those owner files; `strokes.ts` remains the bounded
public facade and duplicate-rejecting assembly point.

More than 2,200 tests cover the registry, paths, font fit, provenance, and
rendering. `jsdom` is a devDependency for exactly two of them: the SVG
serialiser's escaping is checked by handing its output to a **real** parser and
asserting that a hostile caption cannot break out of an attribute or smuggle in a
`<script>`. A string comparison would pass on markup no browser accepts, which is
the bug those two tests exist to catch.
