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
// allowlist before it is written: five tags, sixteen attribute names, balanced
// nesting, and no text a real serialiser could not have produced. A ledger that
// has been tampered with fails the build rather than shipping a `<script>` —
// or a forged citation — inside a figure.
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

  // The panel takes its height from the letter's own box, so the nested viewport
  // below fits exactly and `preserveAspectRatio` never has to letterbox.
  const frameHeight = round((viewBox.height * FRAME_WIDTH) / viewBox.width);
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
    // A NESTED VIEWPORT, not a `<g transform>`.
    //
    // Both would place the frame. Only this one CONTAINS it. A nested `<svg>`
    // establishes a new viewport that clips to its own bounds, so whatever the
    // fragment's own transforms say, nothing it draws can appear outside the
    // panel it belongs to — a tampered ledger can spoil its own frame and
    // nothing else. With a `<g transform>` the containment would be an
    // assertion made by the allowlist, and the allowlist has to permit
    // `transform`, so one `translate` with the right numbers would drop an
    // allowlisted `<text>` exactly where the citation line goes.
    //
    // The viewBox also does the fitting arithmetic, so the scale factor is
    // stated once, as a ratio of two boxes, instead of being multiplied into a
    // translate. `preserveAspectRatio` is explicit rather than defaulted: the
    // panel is sized from this box's own aspect, so `meet` is exact, and saying
    // so keeps every renderer agreeing about it.
    parts.push(
      `<svg x="${x}" y="${y}" width="${FRAME_WIDTH}" height="${frameHeight}" ` +
        `viewBox="${viewBox.minX} ${viewBox.minY} ${viewBox.width} ` +
        `${viewBox.height}" preserveAspectRatio="xMidYMid meet" ` +
        `overflow="hidden">${frame.markup}</svg>`,
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
