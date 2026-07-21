// ---------------------------------------------------------------------------
// truetype.ts — pulling the REAL shape of a letter out of a font file
// ---------------------------------------------------------------------------
//
// Why this file exists
// --------------------
// This app teaches handwriting. That means it has to show the learner the
// shape of a letter — and every shape it shows had better be the shape the
// letter actually has.
//
// The tempting shortcut is to hand-draw the letters as SVG paths. Do not. A
// hand-drawn Tamil ண that is subtly wrong looks completely fine to anyone who
// cannot already read Tamil, which is precisely the audience. The error would
// ship, and it would ship *as the lesson*.
//
// So instead we read the outline out of the font we already ship. The font is
// authoritative: it is what the rest of the app renders text with, it was made
// by typographers, and it cannot drift from what the learner sees. Extracting
// from it means the shapes are correct *by construction* rather than correct
// *if we were careful*.
//
// This module is a small, zero-dependency TrueType reader. It is not a general
// font library — it does exactly enough to answer one question:
//
//     "given a character, what is its outline?"
//
// A one-page tour of a TrueType file
// ----------------------------------
// A font is a bag of tables, each named by a 4-character tag, found through a
// directory at the front of the file:
//
//     [ header ][ directory: 'cmap' -> offset, 'glyf' -> offset, ... ][ tables... ]
//
// The four tables we need, and what each one is for:
//
//     head  — the design grid size (`unitsPerEm`) and which width `loca` uses
//     maxp  — how many glyphs the font has
//     cmap  — character -> glyph id      ("which drawing is 'க'?")
//     loca  — glyph id -> byte offset    ("where is that drawing?")
//     glyf  — the drawings themselves
//
// So looking up a letter is a three-hop chase: cmap gives an id, loca turns
// the id into an offset, glyf holds the contours at that offset.
//
// The coordinate system is worth stating up front because it bites: font units
// are **y-up** with the baseline at y=0, while SVG is **y-down**. Descenders
// are therefore *negative*. We do not flip here — we hand back honest font
// units and let the renderer apply one `scale(1,-1)`, so that everything in
// this file can be checked against the font spec without mental arithmetic.
// ---------------------------------------------------------------------------

/** A point on a glyph contour. `on` = on-curve; off-curve points are controls. */
export interface GlyphPoint {
  x: number;
  y: number;
  on: boolean;
}

/** One closed contour. A letter is one or more of these. */
export type Contour = GlyphPoint[];

export interface Glyph {
  /** Glyph id this came from — useful when debugging a bad mapping. */
  id: number;
  contours: Contour[];
  /** SVG path data in FONT units (y-up). Apply scale(1,-1) to draw it. */
  path: string;
}

export interface Font {
  unitsPerEm: number;
  numGlyphs: number;
  /** Which cmap subtable format we ended up using (4 or 12) — for diagnostics. */
  cmapFormat: number;
  /** How many characters the chosen subtable maps. */
  mappedCharacters: number;
  glyphIdFor(character: string): number | undefined;
  glyphFor(character: string): Glyph | undefined;
}

// A cursor over a byte buffer. Font files are big-endian throughout.
class Cursor {
  private p: number;
  constructor(private readonly b: DataView, offset = 0) {
    this.p = offset;
  }
  get position(): number {
    return this.p;
  }
  set position(v: number) {
    this.p = v;
  }
  skip(n: number): void {
    this.p += n;
  }
  u8(): number {
    return this.b.getUint8(this.p++);
  }
  i8(): number {
    return this.b.getInt8(this.p++);
  }
  u16(): number {
    const v = this.b.getUint16(this.p);
    this.p += 2;
    return v;
  }
  i16(): number {
    const v = this.b.getInt16(this.p);
    this.p += 2;
    return v;
  }
  u32(): number {
    const v = this.b.getUint32(this.p);
    this.p += 4;
    return v;
  }
  tag(): string {
    let s = "";
    for (let i = 0; i < 4; i++) s += String.fromCharCode(this.b.getUint8(this.p + i));
    this.p += 4;
    return s;
  }
}

const REQUIRED_TABLES = ["head", "maxp", "cmap", "loca", "glyf"] as const;

// Guard rails for hostile or corrupt input. The fonts we ship are trusted, but
// this parser runs in the browser and takes byte offsets and loop counts
// straight from the file, so every file-controlled bound is clamped. These are
// far above what a real font needs: the largest font here maps a few thousand
// characters, and Unicode itself only defines ~150k assigned codepoints.
const MAX_CMAP_GROUPS = 100_000;
const MAX_MAPPED_CHARACTERS = 200_000;

/**
 * Read a TrueType font from raw bytes.
 *
 * Throws on anything it does not understand rather than guessing — a wrong
 * glyph is worse than a missing one, because a wrong one still renders.
 */
export function parseFont(bytes: ArrayBuffer): Font {
  const view = new DataView(bytes);
  const r = new Cursor(view);

  const version = r.u32();
  // 0x74746366 = 'ttcf', a collection of fonts in one file. We ship plain
  // .ttf files, so rather than support collections we say so loudly.
  if (version === 0x74746366) throw new Error("TrueType collections (.ttc) are not supported");
  // OpenType fonts with CFF (PostScript) outlines use 'OTTO' and store curves
  // as cubics in a 'CFF ' table — a completely different format from glyf.
  if (version === 0x4f54544f) throw new Error("CFF/OpenType ('OTTO') outlines are not supported; need glyf");

  const numTables = r.u16();
  r.skip(6); // searchRange, entrySelector, rangeShift — accelerators we don't need
  const tables = new Map<string, { offset: number; length: number }>();
  for (let i = 0; i < numTables; i++) {
    const tag = r.tag();
    r.skip(4); // checksum
    tables.set(tag, { offset: r.u32(), length: r.u32() });
  }
  for (const t of REQUIRED_TABLES) {
    if (!tables.has(t)) throw new Error(`font is missing the '${t}' table`);
  }
  const tableAt = (name: string) => tables.get(name)!;

  // ---- head: grid size, and the width of loca's entries ---------------------
  const headOffset = tableAt("head").offset;
  const head = new Cursor(view, headOffset + 18);
  const unitsPerEm = head.u16();
  head.position = headOffset + 50;
  // 0 => loca stores 16-bit values meaning offset/2; 1 => plain 32-bit offsets.
  const indexToLocFormat = head.i16();

  // ---- maxp: glyph count ----------------------------------------------------
  const maxp = new Cursor(view, tableAt("maxp").offset + 4);
  const numGlyphs = maxp.u16();

  // ---- loca: glyph id -> offset into glyf -----------------------------------
  // There are numGlyphs+1 entries: a glyph's data runs from its own entry to
  // the next one. Equal neighbours mean an EMPTY glyph (a space), not an error.
  const loca: number[] = [];
  {
    const c = new Cursor(view, tableAt("loca").offset);
    for (let i = 0; i <= numGlyphs; i++) loca.push(indexToLocFormat ? c.u32() : c.u16() * 2);
  }

  // ---- cmap: character -> glyph id ------------------------------------------
  // A font carries several encodings. We want a Unicode one, and we prefer
  // format 12 (full 32-bit range, so it can reach beyond the BMP) over the
  // older format 4 (16-bit segments).
  const cmapOffset = tableAt("cmap").offset;
  const cmapHeader = new Cursor(view, cmapOffset);
  cmapHeader.skip(2); // version
  const subtableCount = cmapHeader.u16();
  let chosen: { offset: number; format: number } | undefined;
  let chosenScore = -1;
  for (let i = 0; i < subtableCount; i++) {
    const platformId = cmapHeader.u16();
    const encodingId = cmapHeader.u16();
    const offset = cmapOffset + cmapHeader.u32();
    const isUnicode =
      platformId === 0 || (platformId === 3 && (encodingId === 1 || encodingId === 10));
    if (!isUnicode) continue;
    const format = view.getUint16(offset);
    const score = format === 12 ? 20 : format === 4 ? 10 : -1;
    if (score > chosenScore) {
      chosenScore = score;
      chosen = { offset, format };
    }
  }
  if (!chosen) throw new Error("font has no Unicode cmap subtable in format 4 or 12");

  const charToGlyph = new Map<number, number>();
  if (chosen.format === 4) {
    readCmapFormat4(view, chosen.offset, charToGlyph);
  } else {
    readCmapFormat12(view, chosen.offset, charToGlyph);
  }

  // ---- glyf: the outlines ---------------------------------------------------
  const glyfOffset = tableAt("glyf").offset;

  function contoursOf(glyphId: number, depth = 0): Contour[] {
    // Composite glyphs point at other glyphs; a malformed font could loop.
    if (depth > 5) return [];
    if (glyphId < 0 || glyphId + 1 >= loca.length) return [];
    if (loca[glyphId] === loca[glyphId + 1]) return []; // empty glyph, e.g. space

    const start = glyfOffset + loca[glyphId];
    const c = new Cursor(view, start);
    const contourCount = c.i16();
    c.skip(8); // xMin, yMin, xMax, yMax

    if (contourCount < 0) return compositeContours(c, contoursOf, depth);
    return simpleContours(c, contourCount);
  }

  return {
    unitsPerEm,
    numGlyphs,
    cmapFormat: chosen.format,
    mappedCharacters: charToGlyph.size,
    glyphIdFor(character: string) {
      const cp = character.codePointAt(0);
      return cp === undefined ? undefined : charToGlyph.get(cp);
    },
    glyphFor(character: string) {
      const cp = character.codePointAt(0);
      if (cp === undefined) return undefined;
      const id = charToGlyph.get(cp);
      if (id === undefined) return undefined;
      const contours = contoursOf(id);
      return { id, contours, path: contoursToPath(contours) };
    },
  };
}

// cmap format 4: parallel arrays of segments. The `idRangeOffset` trick reads
// into a glyph-id array that *follows* the offsets, addressed relative to the
// position of the offset entry itself — which is why we remember that position.
function readCmapFormat4(view: DataView, offset: number, out: Map<number, number>): void {
  const c = new Cursor(view, offset + 6);
  const segCount = c.u16() / 2;
  c.position = offset + 14;
  const ends: number[] = [];
  const starts: number[] = [];
  const deltas: number[] = [];
  const rangeOffsetPositions: number[] = [];
  const rangeOffsets: number[] = [];
  for (let i = 0; i < segCount; i++) ends.push(c.u16());
  c.skip(2); // reservedPad
  for (let i = 0; i < segCount; i++) starts.push(c.u16());
  for (let i = 0; i < segCount; i++) deltas.push(c.i16());
  for (let i = 0; i < segCount; i++) {
    rangeOffsetPositions.push(c.position);
    rangeOffsets.push(c.u16());
  }
  for (let i = 0; i < segCount; i++) {
    for (let ch = starts[i]; ch <= ends[i] && ch !== 0xffff; ch++) {
      let g: number;
      if (rangeOffsets[i] === 0) {
        g = (ch + deltas[i]) & 0xffff;
      } else {
        const p = rangeOffsetPositions[i] + rangeOffsets[i] + (ch - starts[i]) * 2;
        if (p + 1 >= view.byteLength) continue;
        g = view.getUint16(p);
        if (g !== 0) g = (g + deltas[i]) & 0xffff;
      }
      if (g !== 0) out.set(ch, g);
    }
  }
}

// cmap format 12: a flat list of (startChar, endChar, startGlyph) groups.
//
// Every number here comes from the file, including the group count and the
// range bounds — so all three are clamped. A font claiming a group running
// from 0 to 0xFFFFFFFF would otherwise ask us to build a four-billion-entry
// Map and hang the tab. Unicode stops at U+10FFFF; anything beyond that is
// malformed, and we drop the excess rather than trust it.
function readCmapFormat12(view: DataView, offset: number, out: Map<number, number>): void {
  const MAX_CODEPOINT = 0x10ffff;
  const groupCount = Math.min(view.getUint32(offset + 12), MAX_CMAP_GROUPS);
  const c = new Cursor(view, offset + 16);
  for (let i = 0; i < groupCount; i++) {
    if (c.position + 12 > view.byteLength) break; // truncated table
    const startChar = c.u32();
    const endChar = Math.min(c.u32(), MAX_CODEPOINT);
    const startGlyph = c.u32();
    if (startChar > endChar || startChar > MAX_CODEPOINT) continue;
    for (let ch = startChar; ch <= endChar; ch++) {
      if (out.size >= MAX_MAPPED_CHARACTERS) return;
      out.set(ch, startGlyph + (ch - startChar));
    }
  }
}

// A simple glyph: contour end-indices, then flags, then x's, then y's — each
// coordinate stored as a DELTA from the previous one, in one of three widths
// depending on two flag bits. Repeated flags are run-length encoded.
function simpleContours(c: Cursor, contourCount: number): Contour[] {
  const endPoints: number[] = [];
  for (let i = 0; i < contourCount; i++) endPoints.push(c.u16());
  const pointCount = contourCount > 0 ? endPoints[contourCount - 1] + 1 : 0;
  c.skip(c.u16()); // hinting instructions — irrelevant to the outline

  const ON_CURVE = 0x01;
  const X_SHORT = 0x02;
  const Y_SHORT = 0x04;
  const REPEAT = 0x08;
  const X_SAME_OR_POSITIVE = 0x10;
  const Y_SAME_OR_POSITIVE = 0x20;

  const flags: number[] = [];
  while (flags.length < pointCount) {
    const f = c.u8();
    flags.push(f);
    if (f & REPEAT) {
      let n = c.u8();
      while (n-- > 0 && flags.length < pointCount) flags.push(f);
    }
  }

  const readCoords = (shortBit: number, sameBit: number): number[] => {
    const values: number[] = [];
    let v = 0;
    for (let i = 0; i < pointCount; i++) {
      const f = flags[i];
      if (f & shortBit) {
        const d = c.u8();
        v += f & sameBit ? d : -d;
      } else if (!(f & sameBit)) {
        v += c.i16();
      } // else: same as previous, delta 0
      values.push(v);
    }
    return values;
  };
  const xs = readCoords(X_SHORT, X_SAME_OR_POSITIVE);
  const ys = readCoords(Y_SHORT, Y_SAME_OR_POSITIVE);

  const contours: Contour[] = [];
  let start = 0;
  for (let i = 0; i < contourCount; i++) {
    const end = endPoints[i];
    const pts: Contour = [];
    for (let p = start; p <= end; p++) pts.push({ x: xs[p], y: ys[p], on: !!(flags[p] & ON_CURVE) });
    contours.push(pts);
    start = end + 1;
  }
  return contours;
}

// A composite glyph is built from other glyphs — used for accented letters and
// for Indic glyphs assembled from parts. We honour translation, which is what
// the overwhelming majority use; a scaled component keeps its own shape.
function compositeContours(
  c: Cursor,
  contoursOf: (id: number, depth: number) => Contour[],
  depth: number,
): Contour[] {
  const ARGS_ARE_WORDS = 0x0001;
  const ARGS_ARE_XY = 0x0002;
  const HAS_SCALE = 0x0008;
  const MORE_COMPONENTS = 0x0020;
  const HAS_XY_SCALE = 0x0040;
  const HAS_2X2 = 0x0080;

  const out: Contour[] = [];
  for (;;) {
    const flags = c.u16();
    const componentId = c.u16();
    let dx = 0;
    let dy = 0;
    if (flags & ARGS_ARE_WORDS) {
      dx = c.i16();
      dy = c.i16();
    } else {
      dx = c.i8();
      dy = c.i8();
    }
    if (flags & HAS_SCALE) c.skip(2);
    else if (flags & HAS_XY_SCALE) c.skip(4);
    else if (flags & HAS_2X2) c.skip(8);

    // When ARGS_ARE_XY is clear the arguments are point indices to align,
    // not offsets. That is rare; we place the component unshifted rather
    // than pretend the numbers are coordinates.
    const ox = flags & ARGS_ARE_XY ? dx : 0;
    const oy = flags & ARGS_ARE_XY ? dy : 0;
    for (const contour of contoursOf(componentId, depth + 1)) {
      out.push(contour.map((p) => ({ x: p.x + ox, y: p.y + oy, on: p.on })));
    }
    if (!(flags & MORE_COMPONENTS)) break;
  }
  return out;
}

/**
 * Turn contours into SVG path data.
 *
 * TrueType curves are QUADRATIC (one control point), which maps onto SVG's
 * `Q` command. The one wrinkle: the format lets two off-curve points sit next
 * to each other, with an on-curve point implied exactly halfway between them.
 * We have to insert those midpoints or the outline collapses.
 *
 * A contour may also begin on an off-curve point, in which case we synthesise
 * a start point the same way.
 */
export function contoursToPath(contours: Contour[]): string {
  const round = (n: number) => Math.round(n * 100) / 100;
  const out: string[] = [];

  for (const points of contours) {
    if (points.length === 0) continue;

    let startIndex = points.findIndex((p) => p.on);
    let startPoint: { x: number; y: number };
    if (startIndex === -1) {
      const last = points[points.length - 1];
      const first = points[0];
      startPoint = { x: (last.x + first.x) / 2, y: (last.y + first.y) / 2 };
      startIndex = 0;
    } else {
      startPoint = points[startIndex];
      startIndex += 1;
    }
    out.push(`M${round(startPoint.x)} ${round(startPoint.y)}`);

    let control: GlyphPoint | null = null;
    for (let k = 0; k < points.length; k++) {
      const p = points[(startIndex + k) % points.length];
      if (p.on) {
        if (control) {
          out.push(`Q${round(control.x)} ${round(control.y)} ${round(p.x)} ${round(p.y)}`);
          control = null;
        } else {
          out.push(`L${round(p.x)} ${round(p.y)}`);
        }
      } else {
        if (control) {
          const midX = (control.x + p.x) / 2;
          const midY = (control.y + p.y) / 2;
          out.push(`Q${round(control.x)} ${round(control.y)} ${round(midX)} ${round(midY)}`);
        }
        control = p;
      }
    }
    if (control) {
      out.push(`Q${round(control.x)} ${round(control.y)} ${round(startPoint.x)} ${round(startPoint.y)}`);
    }
    out.push("Z");
  }

  return out.join("");
}

/** Bounding box of a set of contours, in font units. */
export function boundsOf(contours: Contour[]): { x0: number; y0: number; x1: number; y1: number } {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const contour of contours) {
    for (const p of contour) {
      if (p.x < x0) x0 = p.x;
      if (p.x > x1) x1 = p.x;
      if (p.y < y0) y0 = p.y;
      if (p.y > y1) y1 = p.y;
    }
  }
  if (x0 === Infinity) return { x0: 0, y0: 0, x1: 0, y1: 0 };
  return { x0, y0, x1, y1 };
}
