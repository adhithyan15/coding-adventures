// ---------------------------------------------------------------------------
// script-ductus — how a letter is written, and how to show it
// ---------------------------------------------------------------------------
//
// "Ductus" is the old word for the movement of the pen: not what a letter looks
// like, but the order and direction of the strokes that make it. A printed ஔ
// tells you the shape and nothing about the hand. This package is about the
// hand.
//
// THREE MODULES, AND WHY THE SPLIT IS THE WHOLE DESIGN
// ----------------------------------------------------
//
//   strokes.ts     HOW a letter is written. Hand-authored pen paths, each
//                  broken into labelled segments, with the points where the
//                  pen lifts. This is the only file whose contents are a
//                  CLAIM about the world, and every letter in it carries a
//                  citation for the order it teaches.
//
//   truetype.ts    WHAT the letter looks like. A zero-dependency TrueType
//                  reader that pulls the real outline out of the font this
//                  project already ships, so the target shape is never a
//                  drawing of a letter — it IS the letter.
//
//   ductusview.ts  The join. Given one letter's pen path and that letter's
//                  font outline, it builds a FILMSTRIP: frame k shows strokes
//                  1..k travelled in ink over the finished glyph in pale grey,
//                  with a dot where the pen is.
//
// The reason the outline comes from the font rather than from a second
// hand-drawn shape is that it makes a whole class of error impossible to hide.
// A pen path that has drifted away from the letter it claims to draw shows up
// immediately as ink sitting outside the grey — and the tests here check that
// mechanically: `fractionOnInk` requires the authored path to lie on the font's
// own ink, consecutive segments must actually meet, and the strokes together
// must cover the glyph rather than tracing a convenient part of it.
//
// WHY THIS IS A PACKAGE AND NOT PART OF THE APP
// ----------------------------------------------
// These three modules lived in `code/programs/typescript/language-ladder/src`,
// which made them reachable by the app and by nothing else: nothing under
// `code/packages/` may depend on something under `code/programs/`, so the book
// generator — the other consumer that wants filmstrips, as printed figures
// rather than as a live SVG — could not import them at all. `ductusview.ts`'s
// own header anticipated the move: *"the book pipeline can take the serialised
// string instead."* Now it can.
//
// Nothing in here touches the DOM or the filesystem. `ductusFilmstrip` returns
// a tree of plain objects (`SvgNode`) and `svgMarkup` serialises it; the app
// walks the same tree with `createElementNS`, and the book pipeline takes the
// string. One description, two consumers, and every claim testable without a
// browser.

export {
  type Letter,
  type Mark,
  type ScriptData,
  SCRIPTS,
  verifiedLetterFont,
} from "./scriptdata.ts";

export {
  type Point,
  type Segment,
  type Stroke,
  type LetterDuctus,
  DUCTUS,
} from "./strokes.ts";

export {
  type GlyphPoint,
  type Contour,
  type Glyph,
  type Font,
  parseFont,
  contoursToPath,
  boundsOf,
} from "./truetype.ts";

export {
  type SvgNode,
  type GlyphOutline,
  type DuctusStep,
  type Filmstrip,
  type DuctusOptions,
  segmentEndFractions,
  ductusSteps,
  ductusFrame,
  wrapCaption,
  ductusFilmstrip,
  ductusFor,
  viewBoxFor,
  escapeXml,
  isSafeName,
  svgMarkup,
} from "./ductusview.ts";
