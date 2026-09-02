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
// them into the curriculum as `data/ductus/filmstrip-geometry.json`. This file
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
// allowlist before it is written: five tags, ordinary attribute names, nothing
// starting with `on`. A ledger that has been tampered with fails the build
// rather than shipping a `<script>` inside a figure.
// ---------------------------------------------------------------------------

import { fnv1a64 } from "./hash.js";
import type { GeneratedFigure } from "./figure.js";

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

// ---------------------------------------------------------------------------
// Checking a fragment before it becomes part of a committed file
// ---------------------------------------------------------------------------

/** Everything `ductusFrame` emits, and nothing else. */
const ALLOWED_TAGS = new Set(["g", "path", "circle", "text", "tspan"]);

/**
 * One tag: a name, then zero or more `name="value"` attributes with no angle
 * brackets inside the value. Anything a real serialiser would produce matches;
 * anything with an unquoted attribute, a stray bracket, a comment, a processing
 * instruction or a CDATA section does not, and is refused below.
 */
const TAG =
  /<\/?([A-Za-z][A-Za-z0-9]*)((?:\s+[A-Za-z_:][A-Za-z0-9_.:-]*="[^"<>]*")*)\s*\/?>/g;

const ATTRIBUTE_NAME = /(^|\s)([A-Za-z_:][A-Za-z0-9_.:-]*)=/g;

/**
 * Refuse a frame fragment that is anything other than a handful of drawing
 * tags. The text BETWEEN tags is checked too: `svgMarkup` escapes `<` and `>`
 * into entities, so a literal bracket outside a tag means the fragment did not
 * come from it.
 */
export function assertSafeFilmstripMarkup(markup: string, where: string): void {
  let cursor = 0;
  TAG.lastIndex = 0;
  for (let match = TAG.exec(markup); match !== null; match = TAG.exec(markup)) {
    const between = markup.slice(cursor, match.index);
    if (between.includes("<") || between.includes(">")) {
      throw new Error(`${where}: filmstrip markup has an unparsable fragment`);
    }
    const tag = match[1].toLowerCase();
    if (!ALLOWED_TAGS.has(tag)) {
      throw new Error(`${where}: filmstrip markup uses disallowed tag '${tag}'`);
    }
    const attributes = match[2] ?? "";
    ATTRIBUTE_NAME.lastIndex = 0;
    for (
      let attribute = ATTRIBUTE_NAME.exec(attributes);
      attribute !== null;
      attribute = ATTRIBUTE_NAME.exec(attributes)
    ) {
      // `onload` is a legal XML name and, in SVG, also a script. The whole
      // prefix goes, not a list of today's handler names.
      if (/^on/i.test(attribute[2])) {
        throw new Error(
          `${where}: filmstrip markup carries an event handler '${attribute[2]}'`,
        );
      }
    }
    cursor = match.index + match[0].length;
  }
  const tail = markup.slice(cursor);
  if (tail.includes("<") || tail.includes(">")) {
    throw new Error(`${where}: filmstrip markup has an unparsable fragment`);
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
  if (entry.frames.length === 0) {
    throw new Error(`${lessonId}: filmstrip entry has no frames`);
  }
  if (entry.viewBox.width <= 0 || entry.viewBox.height <= 0) {
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

  const scale = FRAME_WIDTH / entry.viewBox.width;
  const frameHeight = round(entry.viewBox.height * scale);
  const columns = Math.min(entry.frames.length, MAX_COLUMNS);
  const rows = Math.ceil(entry.frames.length / columns);
  const gridWidth = columns * FRAME_WIDTH + (columns - 1) * FRAME_GAP;

  const heading = `How it is written — ${entry.summary}`;

  // The footer prints the CITATION and, when the source records that the order
  // varies, one fixed sentence saying so. It does not print the `variation`
  // note itself: those notes run to a paragraph — for several scripts they were
  // taller than the filmstrip they sat under, which buries the teaching in
  // provenance. The full note is not lost; it goes into `<desc>`, so it travels
  // in the file and reaches a screen reader, while the printed page keeps the
  // one claim a learner has to see — that this is AN order, not THE order.
  const varies =
    entry.source.variation !== undefined && entry.source.variation.trim() !== "";
  const citationLines = wrapFigureText(
    `Stroke order after ${entry.source.citation}`,
    gridWidth,
    CITATION_SIZE,
  );
  if (varies) {
    citationLines.push(
      ...wrapFigureText(
        "This order is attested, not standardised; the source records where it varies.",
        gridWidth,
        CITATION_SIZE,
      ),
    );
  }

  const gridTop = MARGIN + HEADING_BAND;
  const gridHeight = rows * frameHeight + (rows - 1) * ROW_GAP;
  const citationTop = gridTop + gridHeight + CITATION_LEADING;
  const width = MARGIN * 2 + gridWidth;
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
  parts.push(
    `<desc>${escapeXml(
      `${entry.frames.length} frames; frame N shows movements 1 to N of ${entry.glyph} ` +
        `(${entry.script}), the movement being added drawn in ink over the finished letter, ` +
        `whose outline is read from ${entry.font}. Stroke order after ` +
        `${entry.source.citation} <${entry.source.url}>.` +
        (varies ? ` Source note on variation: ${entry.source.variation ?? ""}` : ""),
    )}</desc>`,
  );
  parts.push(
    `<rect x="0" y="0" width="${width}" height="${height}" fill="${BACKGROUND}"/>`,
  );
  parts.push(
    `<text x="${MARGIN}" y="${MARGIN + HEADING_SIZE}" font-family="Latin Modern Sans, sans-serif" ` +
      `font-size="${HEADING_SIZE}" fill="${HEADING_COLOR}">${escapeXml(heading)}</text>`,
  );

  entry.frames.forEach((frame, index) => {
    const column = index % columns;
    const row = Math.floor(index / columns);
    const x = round(MARGIN + column * (FRAME_WIDTH + FRAME_GAP));
    const y = round(gridTop + row * (frameHeight + ROW_GAP));
    parts.push(
      `<rect x="${x}" y="${y}" width="${FRAME_WIDTH}" height="${frameHeight}" rx="6" ` +
        `fill="${PANEL_FILL}" stroke="${PANEL_STROKE}" stroke-width="1"/>`,
    );
    const shiftX = round(x - scale * entry.viewBox.minX);
    const shiftY = round(y - scale * entry.viewBox.minY);
    parts.push(
      `<g transform="translate(${shiftX} ${shiftY}) scale(${round(scale)})">${frame.markup}</g>`,
    );
  });

  citationLines.forEach((line, index) => {
    parts.push(
      `<text x="${MARGIN}" y="${round(citationTop + index * CITATION_SIZE * 1.25)}" ` +
        `font-family="Latin Modern Sans, sans-serif" font-size="${CITATION_SIZE}" ` +
        `fill="${CITATION_COLOR}">${escapeXml(line)}</text>`,
    );
  });
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

/** Index a ledger by `script:glyph`, rejecting a malformed or duplicated file. */
export function indexFilmstripLedger(
  ledger: FilmstripLedger,
): Map<string, FilmstripEntry> {
  if (ledger.version !== 1 || !Array.isArray(ledger.entries)) {
    throw new Error(`${FILMSTRIP_LEDGER_PATH} must declare version 1 and entries`);
  }
  const index = new Map<string, FilmstripEntry>();
  for (const entry of ledger.entries) {
    const key = `${entry.script}:${entry.glyph}`;
    if (index.has(key)) {
      throw new Error(`${FILMSTRIP_LEDGER_PATH}: duplicate entry ${key}`);
    }
    index.set(key, entry);
  }
  return index;
}
