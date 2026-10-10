// ---------------------------------------------------------------------------
// figure-filmstrip.ts — "just writing out instructions is not going to help"
// ---------------------------------------------------------------------------
//
// A writing lesson that says *"curl around the upper loop, sweep down the outer
// curve, turn around the lower loop"* is useless to the only reader who needs
// it. Somebody who can already write அ does not need the sentence; somebody who
// cannot has no idea which loop, from where, in which direction. Handwriting is
// taught by watching a hand move, and a printed book cannot move — so it does
// the next thing, which is a FILMSTRIP: the same letter, five times, each frame
// one movement further along, with that movement's own words underneath it.
//
//     ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
//     │   ◜    │ │   ◜    │ │   ◜    │ │   ◜    │ │   ◜  ▕ │
//     │        │ │   ╲    │ │  ╲◟    │ │  ╲◟──  │ │  ╲◟──▕ │
//     └────────┘ └────────┘ └────────┘ └────────┘ └────────┘
//      1. curl     2. sweep   3. turn    4. carry   5. draw the
//      around      down the   around     the        right
//      the upper   outer      the lower  horizontal upright
//      loop        curve      loop       right      down
//
// In every frame the finished letter sits behind in pale grey — and it is the
// letter, the real outline out of the font the lessons are set in, never a
// drawing of one. The strokes already written are a settled grey. Exactly one
// thing is in ink: the movement this frame's caption names.
//
// Where the picture comes from, and why this file draws so little of it
// ---------------------------------------------------------------------
// None of the geometry above is computed here. `@coding-adventures/script-
// ductus` owns the pen paths, reads the font, and renders the frames; it writes
// them into the curriculum as per-script owners in `data/ductus/filmstrip-geometry.d/`. This file
// reads that ledger and does one job the ledger cannot: LAY THE FRAMES OUT for
// a printed page — a grid, panel borders, a heading, and the citation.
//
// The split is not an accident of packaging. `script-ductus` cannot run under
// plain Node (its canonical inventories arrive through a Vite virtual module),
// and the Vite plugin that serves them imports THIS package — so a direct
// import back would close a cycle the repository's build tool rejects. The two
// meet on data instead, and because the ledger is regenerated and byte-checked
// by `script-ductus`'s own test suite, the book and the live app cannot drift:
// there is one renderer, and the ledger is its output written down.
//
// Why the frames arrive as markup, and why we still check it
// -----------------------------------------------------------
// Each frame is an SVG fragment, escaped once by `script-ductus`'s audited
// serialiser. Re-implementing that escaping here — in the file that decides
// what goes into a committed `.svg` — is exactly the duplication you do not
// want. But "it was escaped upstream" is a claim about today's generator, not a
// property of this file, so every fragment is re-checked against a small
// allowlist before it is written: five tags, sixteen attribute names, balanced
// nesting, and no text a real serialiser could not have produced. A ledger that
// has been tampered with fails the build rather than shipping a `<script>` —
// or a forged citation — inside a figure.
// ---------------------------------------------------------------------------

import { fnv1a64 } from "./hash.js";
import type { GeneratedFigure } from "./figure.js";
import { join } from "node:path";
import {
  META_SHARD,
  readMaybeSharded,
  type Shard,
} from "./shard.js";

// ---------------------------------------------------------------------------
// The ledger, as this package sees it
// ---------------------------------------------------------------------------

/** The box every frame of one letter shares, in the ledger's own units. */
export interface FilmstripViewBox {
  minX: number;
  minY: number;
  width: number;
  height: number;
}

/** One frame: what it teaches, and the picture that teaches it. */
export interface FilmstripFrame {
  number: number;
  label: string;
  startsAfterLift: boolean;
  markup: string;
}

/** Where a stroke ORDER came from. No letter may be drawn without one. */
export interface FilmstripSource {
  citation: string;
  url: string;
  variation?: string;
}

/** One letter's whole build-up. */
export interface FilmstripEntry {
  script: string;
  glyph: string;
  sequence?: string;
  font: string;
  source: FilmstripSource;
  penLifts: number;
  summary: string;
  viewBox: FilmstripViewBox;
  frames: FilmstripFrame[];
}

/** The generated file `script-ductus` writes into the curriculum. */
export interface FilmstripLedger {
  version: 1;
  generator: string;
  entries: FilmstripEntry[];
}

/** Where the ledger lives, relative to the curriculum root. */
export const FILMSTRIP_LEDGER_PATH = "data/ductus/filmstrip-geometry.json";
export const FILMSTRIP_LEDGER_DIRECTORY = "data/ductus/filmstrip-geometry.d";

function exactKeys(value: Record<string, unknown>, expected: string[], label: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new Error(`${label} must contain exactly: ${wanted.join(", ")}`);
  }
}

/** Reassemble deterministic per-script filmstrip owners into the public ledger. */
export function mergeFilmstripLedgerShards(shards: Shard[]): FilmstripLedger {
  const meta = shards.find((shard) => shard.name === META_SHARD);
  if (meta === undefined || typeof meta.value !== "object" || meta.value === null || Array.isArray(meta.value)) {
    throw new Error(`filmstrip geometry shards require one ${META_SHARD} object`);
  }
  const metadata = meta.value as Record<string, unknown>;
  exactKeys(metadata, ["version", "generator"], META_SHARD);
  if (metadata.version !== 1 || typeof metadata.generator !== "string") {
    throw new Error(`${META_SHARD} must declare version 1 and a generator`);
  }

  const entries: FilmstripEntry[] = [];
  const seen = new Set<string>();
  for (const shard of shards) {
    if (shard.name === META_SHARD) continue;
    const match = /^([a-z][a-z0-9-]*)\.json$/.exec(shard.name);
    if (match === null || typeof shard.value !== "object" || shard.value === null || Array.isArray(shard.value)) {
      throw new Error(`unsafe filmstrip geometry owner '${shard.name}'`);
    }
    const owner = shard.value as Record<string, unknown>;
    exactKeys(owner, ["script", "entries"], shard.name);
    const script = match[1]!;
    if (owner.script !== script || !Array.isArray(owner.entries) || owner.entries.length === 0) {
      throw new Error(`${shard.name} must own a non-empty '${script}' entry list`);
    }
    for (const raw of owner.entries) {
      if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
        throw new Error(`${shard.name} contains a non-object entry`);
      }
      const entry = raw as unknown as FilmstripEntry;
      if (entry.script !== script || typeof entry.glyph !== "string" || entry.glyph.length === 0) {
        throw new Error(`${shard.name} contains an entry owned by another script`);
      }
      const key = `${script}:${entry.glyph}`;
      if (seen.has(key)) throw new Error(`duplicate filmstrip geometry entry '${key}'`);
      seen.add(key);
      entries.push(entry);
    }
  }
  if (entries.length === 0) throw new Error("filmstrip geometry owner set must not be empty");
  entries.sort((left, right) =>
    left.script === right.script
      ? left.glyph < right.glyph ? -1 : left.glyph > right.glyph ? 1 : 0
      : left.script < right.script ? -1 : 1,
  );
  return { version: 1, generator: metadata.generator, entries };
}

/** Load the generated filmstrip ledger from its per-script owners or legacy monolith. */
export function loadFilmstripLedger(root: string): FilmstripLedger {
  return readMaybeSharded<FilmstripLedger>(
    join(root, FILMSTRIP_LEDGER_PATH),
    mergeFilmstripLedgerShards,
  );
}

// ---------------------------------------------------------------------------
// Checking a fragment before it becomes part of a committed file
// ---------------------------------------------------------------------------

/** Everything `ductusFrame` emits, and nothing else. */
const ALLOWED_TAGS = new Set(["g", "path", "circle", "text", "tspan"]);

/**
 * Every attribute name `ductusFrame` emits, and nothing else.
 *
 * A name DENYLIST (`on*` and friends) would be the easy version and the wrong
 * one. The five tags above are inert today, so nothing in a value can execute —
 * but that is a fact about today's tag list, not about this check. The day
 * somebody adds `use` or `image` to draw a ligature, `href="javascript:..."`,
 * `style="background-image:url(...)"` and `filter="url(http://...)"` all become
 * live, and a denylist that never heard of them would wave them through. An
 * allowlist fails instead, loudly, in the commit that widens the tag list.
 *
 * A ductus renderer that legitimately grows an attribute therefore has to add
 * it here too. That is the intended cost.
 */
const ALLOWED_ATTRIBUTES = new Set([
  "transform",
  "class",
  "d",
  "fill",
  "fill-rule",
  "stroke",
  "stroke-width",
  "stroke-linecap",
  "stroke-linejoin",
  "cx",
  "cy",
  "r",
  "x",
  "y",
  "text-anchor",
  "font-size",
]);

/**
 * One tag: an optional slash, a name, zero or more `name="value"` attributes
 * with no angle brackets inside the value, and an optional self-closing slash.
 * Anything a real serialiser would produce matches; an unquoted attribute, a
 * stray bracket, a comment, a processing instruction, a doctype or a CDATA
 * section does not, and is refused below.
 *
 * The alternatives inside the attribute group are disjoint — whitespace, then a
 * name, then `="` — so the group cannot backtrack into itself and the scan stays
 * linear in the fragment's length. That matters: this input is a file on disk,
 * and a quadratic checker would be a denial of service on the build.
 */
const TAG =
  /<(\/?)([A-Za-z][A-Za-z0-9]*)((?:\s+[A-Za-z_:][A-Za-z0-9_.:-]*="[^"<>]*")*)\s*(\/?)>/g;

/** One attribute of a matched tag: its name, and its value without the quotes. */
const ATTRIBUTE = /(?:^|\s)([A-Za-z_:][A-Za-z0-9_.:-]*)="([^"<>]*)"/g;

/** The five references `escapeXml` produces, plus numeric character references. */
const ENTITY = /&(?:amp|lt|gt|quot|apos|#\d+|#x[0-9A-Fa-f]+);/g;

/** C0 controls other than tab, newline and carriage return: illegal in XML 1.0. */
// eslint-disable-next-line no-control-regex
const CONTROL_CHARACTER = /[\u0000-\u0008\u000B\u000C\u000E-\u001F]/;

/**
 * `fill` and `stroke` are the only PAINT attributes on the list, and paint is
 * the one value shape that can name an external resource: `fill="url(http://
 * evil/x#p)"` is a live reference in a browser-rendered figure. A colour, the
 * keyword `none`, or a local `url(#id)` is all the ductus renderer has ever
 * emitted, so that is all this accepts.
 */
const PAINT_VALUE = /^(?:none|currentColor|#[0-9A-Fa-f]{3,8}|[a-z]+|url\(#[A-Za-z_][\w.:-]*\))$/;
const PAINT_ATTRIBUTES = new Set(["fill", "stroke"]);

/**
 * Check the text between two tags.
 *
 * `svgMarkup` escapes `<`, `>` and `&` into references, so a literal bracket out
 * here means the fragment did not come from it — and a bare `&`, or an entity
 * name nobody defined, means the same thing while ALSO producing a file
 * `rsvg-convert` will refuse during the book build. Catching it here turns a
 * confusing failure at PDF time into a named failure at generation time.
 */
function assertSafeText(text: string, where: string): void {
  if (text.includes("<") || text.includes(">")) {
    throw new Error(`${where}: filmstrip markup has an unparsable fragment`);
  }
  if (CONTROL_CHARACTER.test(text)) {
    throw new Error(`${where}: filmstrip markup contains a control character`);
  }
  if (text.replace(ENTITY, "").includes("&")) {
    throw new Error(`${where}: filmstrip markup has an unescaped or unknown entity`);
  }
}

/**
 * Refuse a frame fragment that is anything other than a balanced tree of a
 * handful of drawing tags.
 *
 * BALANCE is not a nicety here. Each fragment is placed inside a wrapper that
 * positions it in its panel, so a fragment starting with `</g>` would close that
 * wrapper and leave the rest of the fragment as a sibling of the whole figure.
 * Nothing in the tag-by-tag check above notices that, which is why the stack
 * exists.
 *
 * What the stack does NOT do — and the reason the wrapper is a nested viewport
 * rather than a `<g transform>` — is stop a perfectly balanced fragment from
 * PAINTING outside its panel. `transform` is on the allowlist and has to be;
 * one `translate` with the right numbers puts an allowlisted `<text>` exactly
 * where the real citation goes. That is a geometry problem, so it has a
 * geometry answer: see `renderScriptFilmstripFigure`.
 */
export function assertSafeFilmstripMarkup(markup: string, where: string): void {
  let cursor = 0;
  const open: string[] = [];
  TAG.lastIndex = 0;
  for (let match = TAG.exec(markup); match !== null; match = TAG.exec(markup)) {
    assertSafeText(markup.slice(cursor, match.index), where);
    const closing = match[1] === "/";
    const tag = match[2].toLowerCase();
    const attributes = match[3] ?? "";
    const selfClosing = match[4] === "/";
    if (!ALLOWED_TAGS.has(tag)) {
      throw new Error(`${where}: filmstrip markup uses disallowed tag '${tag}'`);
    }
    if (closing && (attributes !== "" || selfClosing)) {
      throw new Error(`${where}: filmstrip markup has a malformed closing '${tag}'`);
    }
    ATTRIBUTE.lastIndex = 0;
    for (
      let attribute = ATTRIBUTE.exec(attributes);
      attribute !== null;
      attribute = ATTRIBUTE.exec(attributes)
    ) {
      const [, name, value] = attribute;
      if (!ALLOWED_ATTRIBUTES.has(name)) {
        throw new Error(`${where}: filmstrip markup uses disallowed attribute '${name}'`);
      }
      // A value gets the same treatment as a text node. Without this, a NUL or a
      // bare `&` inside `d="..."` sails through here and fails much later, in
      // `rsvg-convert`, with a message that names neither the letter nor the
      // frame it came from.
      if (CONTROL_CHARACTER.test(value)) {
        throw new Error(`${where}: filmstrip attribute '${name}' has a control character`);
      }
      if (value.replace(ENTITY, "").includes("&")) {
        throw new Error(`${where}: filmstrip attribute '${name}' has an unknown entity`);
      }
      if (PAINT_ATTRIBUTES.has(name) && !PAINT_VALUE.test(value)) {
        throw new Error(`${where}: filmstrip attribute '${name}' is not a plain colour`);
      }
    }
    if (closing) {
      // The RAW name is compared, not the lowered one. XML is case-sensitive, so
      // `<G></g>` is a mismatch a renderer would reject; catching it here names
      // the frame instead of failing later in the book's SVG-to-PDF step.
      if (open.pop() !== match[2]) {
        throw new Error(`${where}: filmstrip markup closes '${tag}' that is not open`);
      }
    } else if (!selfClosing) {
      open.push(match[2]);
    }
    cursor = match.index + match[0].length;
  }
  assertSafeText(markup.slice(cursor), where);
  if (open.length > 0) {
    throw new Error(`${where}: filmstrip markup leaves '${open[open.length - 1]}' open`);
  }
}

// ---------------------------------------------------------------------------
// The page: how the frames sit on it
// ---------------------------------------------------------------------------
//
// All of these are in the OUTPUT unit (CSS pixels for the app, scaled by
// `\includegraphics` for the book). The frames' own contents are in font units
// and get there through one `translate(...) scale(...)` per panel.

/** Rendered width of one frame. Tall scripts get a taller panel, not a wider one. */
const FRAME_WIDTH = 150;
/** A strip longer than this wraps onto another row rather than off the page. */
const MAX_COLUMNS = 6;
const FRAME_GAP = 10;
const ROW_GAP = 12;
const MARGIN = 16;
/**
 * The narrowest a one-letter strip's heading and footer may wrap: the width
 * of a two-frame strip. A ONE-frame strip (a dab such as the nukta ़, the
 * virama ्, Perso-Arabic ا) would otherwise be a single 150 px panel with its
 * heading broken over three lines and its citation over ten, a figure taller
 * than it is useful. The panel keeps its size and its place at the left
 * margin; only the text gets the room. Two or more frames are already at
 * least this wide, so their strips do not change.
 */
const MIN_TEXT_WIDTH = 2 * FRAME_WIDTH + FRAME_GAP;
/** Space above the frames for the heading. */
const HEADING_BAND = 26;
/** Space below the frames for the citation. */
const CITATION_LEADING = 13;

const BACKGROUND = "#ffffff";
const PANEL_FILL = "#fdfdfc";
const PANEL_STROKE = "#dbe1ea";
const HEADING_COLOR = "#172033";
const CITATION_COLOR = "#64748b";

const HEADING_SIZE = 15;
const CITATION_SIZE = 10;

/**
 * A sans-serif line is roughly half an em per character. We cannot measure text
 * without a browser, and this package refuses to need one, so the citation
 * wraps on that estimate — the cost of being slightly off is a short or long
 * line, not a wrong figure.
 */
const AVERAGE_CHAR_WIDTH = 0.52;

const round = (n: number): number => Math.round(n * 100) / 100;

/** Greedy wrap; an over-long single word keeps its own line and overhangs. */
export function wrapFigureText(text: string, width: number, size: number): string[] {
  const perLine = Math.max(8, Math.floor(width / (size * AVERAGE_CHAR_WIDTH)));
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(/\s+/).filter(Boolean)) {
    const candidate = line === "" ? word : `${line} ${word}`;
    if (candidate.length > perLine && line !== "") {
      lines.push(line);
      line = word;
    } else {
      line = candidate;
    }
  }
  if (line !== "") lines.push(line);
  return lines.length > 0 ? lines : [""];
}

/** Escape the five XML metacharacters. Applied to every value this file writes. */
export function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

/**
 * The canonical subset that is allowed to change a filmstrip figure.
 *
 * The whole entry is in it, because the whole entry is the picture — but the
 * lesson's prose is NOT, so an author rewording a lesson does not churn a
 * committed vector file. That is the same contract `etymologyFigureSource`
 * makes for the etymology route.
 */
export function scriptFilmstripFigureSource(
  lessonId: string,
  entry: FilmstripEntry,
): string {
  return JSON.stringify({
    kind: "script-filmstrip",
    lessonId,
    layout: { FRAME_WIDTH, MAX_COLUMNS, FRAME_GAP, ROW_GAP, MARGIN },
    entry,
  });
}

/**
 * Lay one letter's frames out as a printed filmstrip.
 *
 * The arithmetic, once: every frame shares the letter's `viewBox`, so one
 * scale factor `s = FRAME_WIDTH / viewBox.width` serves all of them, and a
 * frame is placed by `translate(x - s*minX, y - s*minY) scale(s)` — the scale
 * applied first to the frame's own coordinates, then the shift that puts the
 * box's top-left corner at the panel's top-left corner.
 */
export function renderScriptFilmstripFigure(
  lessonId: string,
  entry: FilmstripEntry,
): GeneratedFigure {
  const viewBox = checkedFilmstripEntry(lessonId, entry);

  // The panel takes its height from the letter's own box, so the nested viewport
  // below fits exactly and `preserveAspectRatio` never has to letterbox.
  const frameHeight = round((viewBox.height * FRAME_WIDTH) / viewBox.width);
  const columns = Math.min(entry.frames.length, MAX_COLUMNS);
  const rows = Math.ceil(entry.frames.length / columns);
  const gridWidth = columns * FRAME_WIDTH + (columns - 1) * FRAME_GAP;
  const textWidth = Math.max(gridWidth, MIN_TEXT_WIDTH);

  // The heading WRAPS, like the citation below. It used to be one `<text>` line,
  // which is fine for a strip of three or more frames — but a one-stroke letter
  // (し, へ) gets a two-frame strip only 310 px wide, and "How it is written —
  // one unbroken stroke · 2 movements" ran straight past the figure's right
  // edge, clipped mid-word on the printed page. Each extra line pushes the
  // frames down by one line height, so a heading that fits on one line (every
  // strip of three or more frames today) lays out byte-for-byte as before.
  const headingLines = wrapHeading(`How it is written — ${entry.summary}`, textWidth);
  const headingBand = HEADING_BAND + (headingLines.length - 1) * HEADING_SIZE * 1.25;

  // The footer prints the CITATION and, when the source records that the order
  // varies, one fixed sentence saying so. It does not print the `variation`
  // note itself: those notes run to a paragraph — for several scripts they were
  // taller than the filmstrip they sat under, which buries the teaching in
  // provenance. The full note is not lost; it goes into `<desc>`, so it travels
  // in the file and reaches a screen reader, while the printed page keeps the
  // one claim a learner has to see — that this is AN order, not THE order.
  const varies = sourceVaries(entry);
  const citationLines = wrapFigureText(
    `Stroke order after ${entry.source.citation}`,
    textWidth,
    CITATION_SIZE,
  );
  if (varies) citationLines.push(...wrapFigureText(VARIATION_SENTENCE, textWidth, CITATION_SIZE));

  const gridTop = round(MARGIN + headingBand);
  const gridHeight = rows * frameHeight + (rows - 1) * ROW_GAP;
  const citationTop = gridTop + gridHeight + CITATION_LEADING;
  const width = MARGIN * 2 + textWidth;
  const height = round(
    citationTop + CITATION_SIZE * citationLines.length * 1.25 + MARGIN - CITATION_SIZE * 0.25,
  );

  const parts: string[] = [];
  parts.push(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" ` +
      `viewBox="0 0 ${width} ${height}" role="img" ` +
      `aria-label="${escapeXml(`How to write ${entry.glyph}: ${entry.summary}`)}">`,
  );
  parts.push(`<title>${escapeXml(`Writing ${entry.glyph}`)}</title>`);
  // A ledger entry for a whole WORD (a Devanagari word composed with one
  // shared headline by script-ductus; every letter's own entry is one code
  // point) says what its frames draw, in that order, and that the pale shape
  // behind them is the printed word, not one letter.
  const word = [...entry.glyph].length > 1;
  // A digit drawn alone (Kannada ೧, Malayalam ൧) is not a letter, and the
  // description a screen reader speaks should not call it one.
  const single = DIGIT_GLYPH.test(entry.glyph) ? "digit" : "letter";
  const drawn = word
    ? `(${entry.script}): each letter's body in reading order, then one headline over the whole word; ` +
      `the movement being added is drawn in ink over the finished word, whose outline (each letter's ` +
      `at the font's advance) is read from ${entry.font}.`
    : `(${entry.script}), the movement being added drawn in ink over the finished ${single}, ` +
      `whose outline is read from ${entry.font}.`;
  parts.push(
    `<desc>${escapeXml(
      `${entry.frames.length} frames; frame N shows movements 1 to N of ${entry.glyph} ` +
        `${drawn} Stroke order after ` +
        `${entry.source.citation} <${entry.source.url}>.` +
        (varies ? ` Source note on variation: ${entry.source.variation ?? ""}` : ""),
    )}</desc>`,
  );
  parts.push(
    `<rect x="0" y="0" width="${width}" height="${height}" fill="${BACKGROUND}"/>`,
  );
  pushHeading(parts, headingLines);

  entry.frames.forEach((frame, index) => {
    const column = index % columns;
    const row = Math.floor(index / columns);
    const x = round(MARGIN + column * (FRAME_WIDTH + FRAME_GAP));
    const y = round(gridTop + row * (frameHeight + ROW_GAP));
    parts.push(...framePanel(x, y, frameHeight, viewBox, frame.markup));
  });

  pushCitation(parts, citationLines, citationTop);
  parts.push("</svg>");

  const svg = `${parts.join("")}\n`;
  const sourceHash = fnv1a64(scriptFilmstripFigureSource(lessonId, entry));
  return {
    svg,
    sourceHash,
    svgHash: fnv1a64(svg),
    labels: entry.frames.map((frame) => frame.label),
  };
}

// ---------------------------------------------------------------------------
// Pieces both strips share
// ---------------------------------------------------------------------------
//
// The one-letter strip above and the several-letter strip below print the same
// panels, the same heading and the same footer. These helpers are those parts,
// lifted out WITHOUT changing a byte: every committed one-letter filmstrip is
// byte-checked by `check:figures`, so a refactor that moved one attribute
// would fail that gate on hundreds of files.

/** Printed under the citation when a source records that the order varies. */
const VARIATION_SENTENCE =
  "This order is attested, not standardised; the source records where it varies.";

/** Does this letter's source record that its order varies? */
function sourceVaries(entry: FilmstripEntry): boolean {
  return entry.source.variation !== undefined && entry.source.variation.trim() !== "";
}

/**
 * Wrap a heading to the strip's width.
 *
 * A " · " separator is glued to the word before it (U+E000, a private-use
 * character the wrapper does not treat as a space, stands in for the space
 * while wrapping), so a wrapped line ends "stroke ·" instead of the next one
 * beginning "· 2 movements".
 */
function wrapHeading(text: string, width: number, size = HEADING_SIZE): string[] {
  return wrapFigureText(text.replace(/ · /g, "\uE000· "), width, size).map((line) =>
    line.replace(/\uE000/g, " "),
  );
}

function pushHeading(parts: string[], lines: string[]): void {
  lines.forEach((line, index) => {
    parts.push(
      `<text x="${MARGIN}" y="${round(MARGIN + HEADING_SIZE + index * HEADING_SIZE * 1.25)}" ` +
        `font-family="Latin Modern Sans, sans-serif" ` +
        `font-size="${HEADING_SIZE}" fill="${HEADING_COLOR}">${escapeXml(line)}</text>`,
    );
  });
}

function pushCitation(parts: string[], lines: string[], top: number): void {
  lines.forEach((line, index) => {
    parts.push(
      `<text x="${MARGIN}" y="${round(top + index * CITATION_SIZE * 1.25)}" ` +
        `font-family="Latin Modern Sans, sans-serif" font-size="${CITATION_SIZE}" ` +
        `fill="${CITATION_COLOR}">${escapeXml(line)}</text>`,
    );
  });
}

/**
 * One frame's panel: a rounded card, and the frame inside it.
 *
 * The frame goes in a NESTED VIEWPORT, not a `<g transform>`.
 *
 * Both would place the frame. Only this one CONTAINS it. A nested `<svg>`
 * establishes a new viewport that clips to its own bounds, so whatever the
 * fragment's own transforms say, nothing it draws can appear outside the
 * panel it belongs to — a tampered ledger can spoil its own frame and
 * nothing else. With a `<g transform>` the containment would be an
 * assertion made by the allowlist, and the allowlist has to permit
 * `transform`, so one `translate` with the right numbers would drop an
 * allowlisted `<text>` exactly where the citation line goes.
 *
 * The viewBox also does the fitting arithmetic, so the scale factor is
 * stated once, as a ratio of two boxes, instead of being multiplied into a
 * translate. `preserveAspectRatio` is explicit rather than defaulted: the
 * panel is sized from this box's own aspect, so `meet` is exact, and saying
 * so keeps every renderer agreeing about it.
 */
function framePanel(
  x: number,
  y: number,
  frameHeight: number,
  viewBox: FilmstripViewBox,
  markup: string,
): string[] {
  return [
    `<rect x="${x}" y="${y}" width="${FRAME_WIDTH}" height="${frameHeight}" rx="6" ` +
      `fill="${PANEL_FILL}" stroke="${PANEL_STROKE}" stroke-width="1"/>`,
    `<svg x="${x}" y="${y}" width="${FRAME_WIDTH}" height="${frameHeight}" ` +
      `viewBox="${viewBox.minX} ${viewBox.minY} ${viewBox.width} ` +
      `${viewBox.height}" preserveAspectRatio="xMidYMid meet" ` +
      `overflow="hidden">${markup}</svg>`,
  ];
}

/**
 * Everything a ledger entry must satisfy before any of it reaches a file, for
 * either strip: frames to draw, a real box, a citation, and markup that passes
 * the allowlist. Answers with the checked box.
 */
function checkedFilmstripEntry(lessonId: string, entry: FilmstripEntry): FilmstripViewBox {
  if (entry.frames.length === 0) {
    throw new Error(`${lessonId}: filmstrip entry has no frames`);
  }
  // The four viewBox members are the ONLY ledger values that reach an attribute
  // without going through `escapeXml`, because they are supposed to be numbers.
  // "Supposed to be" is a claim about the type declaration; the ledger is JSON,
  // and JSON parses into whatever it says. A string here would be interpolated
  // straight into `viewBox="..."` and could close the attribute and open an
  // `onload` — so the type is CHECKED, not assumed. A real number cannot
  // contain a quote, which ends the whole class of problem rather than escaping
  // around it.
  //
  // Note what the old `<g transform="translate(x - s*minX, ...)">` form did for
  // free: it consumed these values in arithmetic, so a string degraded to `NaN`
  // and never reached the output as text. Moving to a nested viewport put them
  // in an attribute verbatim, which is exactly the kind of consequence a
  // security fix is most likely to carry in with it.
  const viewBox = assertFiniteViewBox(entry.viewBox, lessonId);
  if (viewBox.width <= 0 || viewBox.height <= 0) {
    throw new Error(`${lessonId}: filmstrip entry has an empty viewBox`);
  }
  if (entry.source.citation.trim() === "" || entry.source.url.trim() === "") {
    throw new Error(
      `${lessonId}: a filmstrip may not be drawn from an uncited stroke order`,
    );
  }
  for (const frame of entry.frames) {
    assertSafeFilmstripMarkup(frame.markup, `${lessonId} frame ${frame.number}`);
  }
  return viewBox;
}

// ---------------------------------------------------------------------------
// Several letters: the sequence strip
// ---------------------------------------------------------------------------
//
// A lesson whose headword is "வ, க", "ક — ણ — શ" or "はい" teaches more than
// one letter, and until now printed nothing at all, because a filmstrip was one
// ledger entry. The sequence strip prints every letter's OWN cited strip, in
// writing order, one labelled group per letter (short letters share a row;
// see `shelveLetters`):
//
//     How it is written — 2 letters, one after another
//     Letter 1 of 2 — 3 strokes · 2 pen lifts · 6 movements
//     ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
//     │  は  │ │  は  │ │  は  │ │  は  │ │  は  │ │  は  │
//     └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘
//     Letter 2 of 2 — 2 strokes · 1 pen lift · 3 movements
//     ┌──────┐ ┌──────┐ ┌──────┐
//     │  い  │ │  い  │ │  い  │
//     └──────┘ └──────┘ └──────┘
//     Letter 1: stroke order after KanjiVG, kanji/0306f.svg, …
//     Letter 2: stroke order after KanjiVG, kanji/03044.svg, …
//
// WHAT IT DOES NOT CLAIM, and why that is the honest version. The ledger
// knows how each letter is written ON ITS OWN. It does not know how big one
// letter is next to the next, how far apart they sit, or whether the pen joins
// them. So each letter keeps its own panels at its own scale — exactly the
// frames its one-letter strip prints — and nothing is drawn between letters.
// The `<desc>` says this in words. Which headwords may be drawn this way at
// all (a Devanagari word may NOT: its letters share one headline, so it is
// drawn as ONE composed entry by the one-letter strip above) is decided
// upstream, in `figure-targets.ts`.
//
// The labels name each letter by NUMBER, never by the letter itself: a
// figure's printed text is set in Latin Modern Sans, which has no Tamil,
// Gujarati or kana glyphs. The letters themselves are in the title, the
// aria-label and the `<desc>`, where no font is needed.

/** Type size of the "Letter k of n" line above each letter's frames. */
const GROUP_LABEL_SIZE = 12;
/** Space between a group label's baseline and that letter's first panel. */
const GROUP_LABEL_LEADING = 7;
/** Space between one shelf's lowest panel and the next shelf's labels. */
const GROUP_GAP = 14;

/**
 * The canonical subset that is allowed to change a sequence figure: every
 * letter's whole entry, in order, the headword it spells, and the layout.
 * Like `scriptFilmstripFigureSource`, the lesson's prose is not in it.
 */
export function scriptSequenceFilmstripFigureSource(
  lessonId: string,
  text: string,
  entries: readonly FilmstripEntry[],
  unit?: SequenceUnit,
): string {
  return JSON.stringify({
    kind: "script-filmstrip-sequence",
    lessonId,
    text,
    // Only a phrase and a strip of digits name their unit, so every other
    // strip keeps the source (and so the hash) it always had:
    //
    //     unit     in the source?   why
    //     -------  ---------------  ------------------------------------------
    //     Letter   no               the entries decide it, and its strips
    //     Part     no                 print what they always printed
    //     Word     yes              the caller says so; entries cannot
    //     Digit    yes              its strips' printed words changed
    //
    // A digit strip USED to print "Letter 1 of 2" from these same entries.
    // When it began to print "Digit 1 of 2", its SVG changed while its
    // entries did not, so the unit goes into the source: a figure's source
    // hash moves whenever the figure it describes does.
    ...(unit === "Word" || unit === "Digit" ? { unit } : {}),
    layout: {
      FRAME_WIDTH,
      MAX_COLUMNS,
      FRAME_GAP,
      ROW_GAP,
      MARGIN,
      GROUP_LABEL_SIZE,
      GROUP_LABEL_LEADING,
      GROUP_GAP,
      GROUP_GAP_X,
    },
    entries,
  });
}

/** "Letter 2", "Letters 1 and 3", "Letters 1, 2 and 4" (or "Part 2", "Digits 1 and 2", …). */
export function letterNumbers(numbers: readonly number[], unit: SequenceUnit = "Letter"): string {
  if (numbers.length === 1) return `${unit} ${numbers[0]}`;
  const head = numbers.slice(0, -1).join(", ");
  return `${unit}s ${head} and ${numbers[numbers.length - 1]}`;
}

/**
 * What one group of a sequence strip is called.
 *
 * A strip of letters says "Letter 2 of 3". Once a vowel sign is one of the
 * groups (Tamil மேசை is drawn ே, ம, ை, ச), "letter" would be wrong: a sign is
 * not a letter, and the groups no longer spell the word in typed order. Such a
 * strip says "Part 2 of 4" instead. A strip with no sign prints exactly what
 * it printed before signs could be drawn.
 *
 * A Devanagari PHRASE ("मम नाम") is a strip of words, each composed with its
 * own headline: "Word 2 of 2". Only the caller knows a phrase from a list, so
 * the unit is passed in (`renderScriptSequenceFilmstripFigure`), never
 * guessed from the entries.
 *
 * A strip of DIGITS (Persian ۰ ۱, Malayalam ൧ ൨ ൩) says "Digit 1 of 2": a
 * digit is not a letter, and the lesson teaches it as a number's shape.
 */
export type SequenceUnit = "Letter" | "Part" | "Word" | "Digit";

/** A ledger glyph made only of combining signs is a vowel sign drawn alone. */
const SIGN_GLYPH = /^\p{M}+$/u;

/**
 * What a strip of parts says about their order, in its `<desc>`.
 *
 * Tamil writes three signs to the LEFT of their consonant and writes them
 * first, so its parts are not in typed order. Gujarati writes every sign after
 * its consonant, even િ, which sits to the left of it, so its parts are in
 * typed order. The note must say which, or the description would claim a
 * reordering the strip does not make.
 */
const WRITTEN_ORDER_NOTES: Readonly<Record<string, string>> = {
  gujarati:
    `The parts are in the order the hand writes them, which in Gujarati is the order ` +
    `they are typed: each sign comes after its consonant, even the i sign, which sits ` +
    `to the left of it. Each sign is drawn on its own, without the consonant it attaches to. `,
};

/** The note for scripts that write some signs before their consonant (Tamil). */
const SIGN_FIRST_NOTE =
  `The parts are in the order the hand writes them, which is not always the order ` +
  `they are typed: a vowel sign written to the left of its consonant comes before ` +
  `it. Each vowel sign is drawn on its own, without the consonant it attaches to. `;

/** What a strip of words says about how each word is drawn, in its `<desc>`. */
const WORD_NOTE =
  `Each word is drawn as its letters' bodies in reading order, then one headline over that ` +
  `word; the space between words breaks the headline. `;

/** The `<desc>` sentence on written order for a strip of parts in `script`. */
export function writtenOrderNote(script: string): string {
  return WRITTEN_ORDER_NOTES[script] ?? SIGN_FIRST_NOTE;
}

/** A ledger glyph that is one decimal digit (`\p{Nd}`): ۰, ൧, ೨, 7. */
const DIGIT_GLYPH = /^\p{Nd}$/u;

/**
 * What a strip's groups are called, read from their glyphs:
 *
 *     glyphs              any sign?   every one a digit?   unit
 *     ------------------  ----------  -------------------  ------
 *     ே, ம, ை, ச          yes         -                    Part
 *     வ, க                no          no                   Letter
 *     ۰, ۱                no          yes                  Digit
 *     ക, ൧                no          no                   Letter
 *
 * A sign makes the strip one of parts whatever else is in it, because the
 * parts are then in written order. A strip mixing letters and digits (none
 * is drawn today) stays "Letter", the name it had before digits were told
 * apart; only a strip that is digits through and through says "Digit".
 */
export function sequenceUnit(entries: readonly FilmstripEntry[]): SequenceUnit {
  if (entries.some((entry) => SIGN_GLYPH.test(entry.glyph))) return "Part";
  if (entries.length > 0 && entries.every((entry) => DIGIT_GLYPH.test(entry.glyph))) return "Digit";
  return "Letter";
}

/**
 * The footer's citation lines: every letter's source, each printed once.
 *
 * When every letter shares one source the line reads exactly as a one-letter
 * strip's does. Otherwise each source is introduced by the letters it covers,
 * in the order the letters are written, so a reader can match a source to a
 * row without counting.
 */
function sequenceCitationLines(
  entries: readonly FilmstripEntry[],
  width: number,
  unit: SequenceUnit = "Letter",
): string[] {
  const bySource = new Map<string, number[]>();
  entries.forEach((entry, index) => {
    const numbers = bySource.get(entry.source.citation) ?? [];
    numbers.push(index + 1);
    bySource.set(entry.source.citation, numbers);
  });
  const lines: string[] = [];
  if (bySource.size === 1) {
    lines.push(...wrapFigureText(`Stroke order after ${entries[0]!.source.citation}`, width, CITATION_SIZE));
  } else {
    for (const [citation, numbers] of bySource) {
      lines.push(
        ...wrapFigureText(`${letterNumbers(numbers, unit)}: stroke order after ${citation}`, width, CITATION_SIZE),
      );
    }
  }
  if (entries.some(sourceVaries)) {
    lines.push(...wrapFigureText(VARIATION_SENTENCE, width, CITATION_SIZE));
  }
  return lines;
}

/**
 * Which letters share a shelf (a band of the figure), in writing order.
 *
 * A book prints a block figure no taller than 0.45 of the text height, so a
 * strip that stacks four short letters one under another — two frames each,
 * in a figure two frames wide — is shrunk until its captions cannot be read.
 * Instead, consecutive letters share a shelf while their frames fit across
 * `maxColumns` panels together:
 *
 *     frames per letter   3  2  2  2        6  3        8  2
 *     shelves             [3 2] [2 2]       [6] [3]     [8] [2]
 *
 * A letter with more frames than fit on one shelf gets a shelf of its own and
 * wraps inside it, exactly as a one-letter strip wraps. Order never changes:
 * a shelf is read left to right, shelves top to bottom.
 */
export function shelveLetters(frameCounts: readonly number[], maxColumns = MAX_COLUMNS): number[][] {
  const shelves: number[][] = [];
  let used = maxColumns;
  frameCounts.forEach((frames, index) => {
    const columns = Math.min(frames, maxColumns);
    if (used + columns > maxColumns) {
      shelves.push([index]);
      used = columns;
    } else {
      shelves[shelves.length - 1]!.push(index);
      used += columns;
    }
  });
  return shelves;
}

/** Horizontal space between two letters that share a shelf. */
const GROUP_GAP_X = 26;

/**
 * Lay several letters' frames out as one printed strip, a group per letter.
 *
 * Each group is the one-letter strip's grid on its own: the letter's frames,
 * `MAX_COLUMNS` to a row, at that letter's own scale, under a label line that
 * names the letter by number. Groups sit on shelves (`shelveLetters`), so short
 * letters print side by side; a shelf is as tall as its tallest group, and its
 * groups hang from the same top edge.
 */
export function renderScriptSequenceFilmstripFigure(
  lessonId: string,
  text: string,
  entries: readonly FilmstripEntry[],
  unitOverride?: "Word",
): GeneratedFigure {
  if (entries.length < 2) {
    throw new Error(`${lessonId}: a sequence filmstrip needs at least two letters`);
  }
  const script = entries[0]!.script;
  if (entries.some((entry) => entry.script !== script)) {
    throw new Error(`${lessonId}: a sequence filmstrip draws letters of one script`);
  }
  const boxes = entries.map((entry) => checkedFilmstripEntry(lessonId, entry));
  const count = entries.length;
  const unit: SequenceUnit = unitOverride ?? sequenceUnit(entries);
  const units = `${unit.toLowerCase()}s`;

  // Each group's own geometry, before anything is placed: how many columns it
  // spans, how wide that is, how tall one of its panels is, and its label.
  const groups = entries.map((entry, index) => {
    const columns = Math.min(entry.frames.length, MAX_COLUMNS);
    const width = columns * FRAME_WIDTH + (columns - 1) * FRAME_GAP;
    const viewBox = boxes[index]!;
    const frameHeight = round((viewBox.height * FRAME_WIDTH) / viewBox.width);
    const rows = Math.ceil(entry.frames.length / columns);
    return {
      entry,
      viewBox,
      columns,
      width,
      frameHeight,
      height: rows * frameHeight + (rows - 1) * ROW_GAP,
      label: wrapHeading(`${unit} ${index + 1} of ${count} — ${entry.summary}`, width, GROUP_LABEL_SIZE),
    };
  });
  const shelves = shelveLetters(entries.map((entry) => entry.frames.length));
  const shelfWidth = (shelf: number[]): number =>
    shelf.reduce((sum, index) => sum + groups[index]!.width, 0) + (shelf.length - 1) * GROUP_GAP_X;
  const gridWidth = Math.max(...shelves.map(shelfWidth));

  const headingLines = wrapHeading(`How it is written — ${count} ${units}, one after another`, gridWidth);
  const headingBand = HEADING_BAND + (headingLines.length - 1) * HEADING_SIZE * 1.25;

  // Walk down the page once, shelf by shelf. `cursor` is the top of the next
  // shelf; within a shelf, `x` is the left edge of the next group.
  const body: string[] = [];
  let cursor = round(MARGIN + headingBand);
  shelves.forEach((shelf, shelfIndex) => {
    const labelLines = Math.max(...shelf.map((index) => groups[index]!.label.length));
    const frameTop = round(
      cursor + GROUP_LABEL_SIZE + (labelLines - 1) * GROUP_LABEL_SIZE * 1.25 + GROUP_LABEL_LEADING,
    );
    let x = MARGIN;
    for (const index of shelf) {
      const group = groups[index]!;
      group.label.forEach((line, lineIndex) => {
        body.push(
          `<text x="${round(x)}" y="${round(cursor + GROUP_LABEL_SIZE + lineIndex * GROUP_LABEL_SIZE * 1.25)}" ` +
            `font-family="Latin Modern Sans, sans-serif" ` +
            `font-size="${GROUP_LABEL_SIZE}" fill="${HEADING_COLOR}">${escapeXml(line)}</text>`,
        );
      });
      group.entry.frames.forEach((frame, frameIndex) => {
        const panelX = round(x + (frameIndex % group.columns) * (FRAME_WIDTH + FRAME_GAP));
        const panelY = round(frameTop + Math.floor(frameIndex / group.columns) * (group.frameHeight + ROW_GAP));
        body.push(...framePanel(panelX, panelY, group.frameHeight, group.viewBox, frame.markup));
      });
      x += group.width + GROUP_GAP_X;
    }
    cursor = round(frameTop + Math.max(...shelf.map((index) => groups[index]!.height)));
    if (shelfIndex < shelves.length - 1) cursor = round(cursor + GROUP_GAP);
  });

  const citationLines = sequenceCitationLines(entries, gridWidth, unit);
  const citationTop = cursor + CITATION_LEADING;
  const width = MARGIN * 2 + gridWidth;
  const height = round(
    citationTop + CITATION_SIZE * citationLines.length * 1.25 + MARGIN - CITATION_SIZE * 0.25,
  );

  const letters = entries.map((entry) => entry.glyph);
  const fonts = [...new Set(entries.map((entry) => entry.font))].join(", ");
  const sources = entries
    .map((entry) => `${entry.glyph}: ${entry.source.citation} <${entry.source.url}>`)
    .join("; ");
  const variations = entries
    .filter(sourceVaries)
    .map((entry) => `${entry.glyph}: ${entry.source.variation ?? ""}`)
    .join(" ");

  const parts: string[] = [];
  parts.push(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" ` +
      `viewBox="0 0 ${width} ${height}" role="img" ` +
      `aria-label="${escapeXml(`How to write ${text}: ${count} ${units}, ${letters.join(", ")}, one after another`)}">`,
  );
  parts.push(`<title>${escapeXml(`Writing ${text}`)}</title>`);
  parts.push(
    `<desc>${escapeXml(
      `${count} ${units} written one after another: ${letters.join(", ")} (${script}). ` +
        (unit === "Part" ? writtenOrderNote(script) : "") +
        (unit === "Word" ? WORD_NOTE : "") +
        `Each ${unit.toLowerCase()} has its own group of frames; frame N of a group shows movements 1 to N ` +
        `of that ${unit.toLowerCase()}, the movement being added drawn in ink over the finished ${unit.toLowerCase()}, ` +
        `whose outline is read from ${fonts}. Each ${unit.toLowerCase()} is drawn at its own scale from ` +
        `its own cited stroke order, so the strip does not show how large the ${units} are ` +
        `next to each other, how far apart they sit, or any join between them. ` +
        `Stroke order: ${sources}.` +
        (variations === "" ? "" : ` Source notes on variation: ${variations}`),
    )}</desc>`,
  );
  parts.push(`<rect x="0" y="0" width="${width}" height="${height}" fill="${BACKGROUND}"/>`);
  pushHeading(parts, headingLines);
  parts.push(...body);
  pushCitation(parts, citationLines, citationTop);
  parts.push("</svg>");

  const svg = `${parts.join("")}\n`;
  return {
    svg,
    sourceHash: fnv1a64(scriptSequenceFilmstripFigureSource(lessonId, text, entries, unit)),
    svgHash: fnv1a64(svg),
    labels: entries.flatMap((entry) => entry.frames.map((frame) => frame.label)),
  };
}

/**
 * The viewBox, proven to be four finite numbers.
 *
 * Exported because `renderScriptFilmstripFigure` is not the only door: a caller
 * with a ledger entry in hand should be able to make the same check.
 */
export function assertFiniteViewBox(
  box: FilmstripViewBox,
  where: string,
): FilmstripViewBox {
  const checked = { minX: 0, minY: 0, width: 0, height: 0 };
  for (const key of ["minX", "minY", "width", "height"] as const) {
    const value: unknown = box?.[key];
    if (typeof value !== "number" || !Number.isFinite(value)) {
      throw new Error(`${where}: filmstrip viewBox ${key} is not a finite number`);
    }
    checked[key] = value;
  }
  return checked;
}

function assertString(value: unknown, where: string, field: string): string {
  if (typeof value !== "string") {
    throw new Error(`${where}: filmstrip ${field} is not a string`);
  }
  // Escaping makes a value SAFE, not necessarily WELL-FORMED: `escapeXml` leaves
  // a NUL exactly where it found it, and the result is a committed `.svg` that
  // XML rejects. That failure would surface in the book's SVG-to-PDF step,
  // naming a file rather than a field. The markup path already refuses control
  // characters for this reason; the escaped fields get the same treatment so the
  // standard is one standard.
  if (CONTROL_CHARACTER.test(value)) {
    throw new Error(`${where}: filmstrip ${field} contains a control character`);
  }
  return value;
}

/**
 * Prove one entry has the shape its type claims.
 *
 * `readLedgerFile<T>` parses JSON and casts; the cast is a promise to the
 * compiler, not a check at runtime. Everything downstream — the escaping, the
 * markup allowlist, the viewBox interpolation — assumes strings are strings and
 * numbers are numbers, so that assumption is established here, once, at the
 * point the file is read, rather than re-argued at each use.
 */
export function assertFilmstripEntry(entry: FilmstripEntry, where: string): void {
  assertString(entry.script, where, "script");
  assertString(entry.glyph, where, "glyph");
  assertString(entry.font, where, "font");
  if (entry.sequence !== undefined) assertString(entry.sequence, where, "sequence");
  assertString(entry.summary, where, "summary");
  assertString(entry.source?.citation, where, "source.citation");
  assertString(entry.source?.url, where, "source.url");
  if (entry.source.variation !== undefined) {
    assertString(entry.source.variation, where, "source.variation");
  }
  if (typeof entry.penLifts !== "number" || !Number.isInteger(entry.penLifts)) {
    throw new Error(`${where}: filmstrip penLifts is not a whole number`);
  }
  assertFiniteViewBox(entry.viewBox, where);
  if (!Array.isArray(entry.frames)) {
    throw new Error(`${where}: filmstrip frames is not a list`);
  }
  for (const frame of entry.frames) {
    if (typeof frame?.number !== "number" || !Number.isInteger(frame.number)) {
      throw new Error(`${where}: filmstrip frame number is not a whole number`);
    }
    assertString(frame.label, where, `frame ${frame.number} label`);
    assertString(frame.markup, where, `frame ${frame.number} markup`);
    if (typeof frame.startsAfterLift !== "boolean") {
      throw new Error(`${where}: filmstrip frame ${frame.number} lift flag is not a boolean`);
    }
  }
}

/** Index a ledger by `script:glyph`, rejecting a malformed or duplicated file. */
export function indexFilmstripLedger(
  ledger: FilmstripLedger,
): Map<string, FilmstripEntry> {
  if (ledger.version !== 1 || !Array.isArray(ledger.entries)) {
    throw new Error(`${FILMSTRIP_LEDGER_PATH} must declare version 1 and entries`);
  }
  const index = new Map<string, FilmstripEntry>();
  for (const entry of ledger.entries) {
    assertFilmstripEntry(entry, FILMSTRIP_LEDGER_PATH);
    const key = `${entry.script}:${entry.glyph}`;
    if (index.has(key)) {
      throw new Error(`${FILMSTRIP_LEDGER_PATH}: duplicate entry ${key}`);
    }
    index.set(key, entry);
  }
  return index;
}
