// ---------------------------------------------------------------------------
// filmstrip-ledger.test.ts — generator AND gate for the book's filmstrip data
// ---------------------------------------------------------------------------
//
// This one file both WRITES `data/ductus/filmstrip-geometry.json` and CHECKS
// that the committed copy still matches what the current pen paths and fonts
// produce. Which of the two it does is decided by Vite's mode:
//
//     npm run generate:filmstrip-ledger   ->  vitest --mode write   (writes)
//     npm run check:filmstrip-ledger      ->  vitest               (compares)
//     npm test / the BUILD                ->  vitest               (compares)
//
// It lives in the test suite rather than in a `bin/` script for a blunt
// reason: this package cannot run under plain Node. `scriptdata.ts` reads the
// canonical Japanese/Perso-Arabic/Tamil/Urdu inventories through a Vite virtual
// module, so every entry point into `DUCTUS` needs a Vite process. Vitest is
// the Vite process this package already has, and putting the gate inside the
// suite means the BUILD enforces it without any new wiring — a stroke edited
// in `src/strokes/` and not regenerated fails this package's own tests.
//
// WHICH letters get an entry is decided by the book, not by this package: the
// generator reads the curriculum's `core/figure-generation.json` and emits an
// entry for every `script-filmstrip` target declared there. That keeps the
// generated file the size of what is actually printed instead of all 352
// authored glyphs, and it means adding a filmstrip to a lesson is one target
// plus one regeneration rather than an edit here.
// ---------------------------------------------------------------------------

import { describe, expect, it } from "vitest";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { SCRIPTS } from "../src/scriptdata.ts";
import { ductusFor, boundsOf, parseFont } from "../src/index.ts";
import type { GlyphOutline } from "../src/ductusview.ts";
import {
  buildFilmstripEntry,
  buildFilmstripLedger,
  captionSizeFor,
  serialiseFilmstripLedger,
  FILMSTRIP_LEDGER_PATH,
  type FilmstripEntry,
} from "../src/filmstrip-ledger.ts";

const CURRICULUM_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../../../learning/human-languages",
);

/** The book's own list of figures; only `script-filmstrip` targets matter here. */
interface FigureTarget {
  kind: string;
  script?: string;
  glyph?: string;
}

function filmstripTargets(): Array<{ script: string; glyph: string }> {
  const config = JSON.parse(
    readFileSync(join(CURRICULUM_ROOT, "core", "figure-generation.json"), "utf8"),
  ) as { targets?: FigureTarget[] };
  const wanted = new Map<string, { script: string; glyph: string }>();
  for (const target of config.targets ?? []) {
    if (target.kind !== "script-filmstrip") continue;
    if (typeof target.script !== "string" || typeof target.glyph !== "string") {
      throw new Error("script-filmstrip targets need a script and a glyph");
    }
    // Two lessons may legitimately print the same letter; the ledger holds it
    // once, and `buildFilmstripLedger` rejects an accidental second copy.
    wanted.set(`${target.script}:${target.glyph}`, {
      script: target.script,
      glyph: target.glyph,
    });
  }
  return [...wanted.values()];
}

const fonts = new Map<string, ReturnType<typeof parseFont>>();

/**
 * The letter's real outline, out of the font the curriculum says this script is
 * rendered in. The font is never named here: it is read from the script's own
 * canonical inventory, so a figure cannot be drawn from a font the lessons do
 * not use.
 */
function outlineFor(script: string, glyph: string): { outline: GlyphOutline; font: string } {
  const inventory = SCRIPTS.find((candidate) => candidate.script === script);
  if (inventory === undefined) throw new Error(`no ${script} inventory`);
  const font = inventory.font;
  let parsed = fonts.get(font);
  if (parsed === undefined) {
    const bytes = readFileSync(resolve(CURRICULUM_ROOT, font));
    const buffer = bytes.buffer.slice(
      bytes.byteOffset,
      bytes.byteOffset + bytes.byteLength,
    ) as ArrayBuffer;
    parsed = parseFont(buffer);
    fonts.set(font, parsed);
  }
  const drawing = parsed.glyphFor(glyph);
  if (drawing === undefined) {
    throw new Error(`${font} has no outline for ${script} ${glyph}`);
  }
  return {
    outline: { path: drawing.path, bounds: boundsOf(drawing.contours) },
    font,
  };
}

function currentLedgerBytes(): string {
  const entries = filmstripTargets().map(({ script, glyph }) => {
    const letter = ductusFor(glyph, script);
    if (letter === undefined) {
      throw new Error(
        `no cited ductus for ${script} ${glyph} — a filmstrip may never be ` +
          `drawn from an invented stroke order`,
      );
    }
    const { outline, font } = outlineFor(script, glyph);
    return buildFilmstripEntry(letter, outline, font);
  });
  return serialiseFilmstripLedger(buildFilmstripLedger(entries));
}

describe("the printed filmstrip ledger", () => {
  const path = join(CURRICULUM_ROOT, FILMSTRIP_LEDGER_PATH);

  it("matches the pen paths and fonts it was generated from", () => {
    const expected = currentLedgerBytes();

    if (import.meta.env.MODE === "write") {
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, expected, "utf8");
    }

    let actual: string;
    try {
      actual = readFileSync(path, "utf8");
    } catch {
      throw new Error(
        `${FILMSTRIP_LEDGER_PATH} is missing — run ` +
          `\`npm run generate:filmstrip-ledger\` in script-ductus`,
      );
    }
    expect(
      actual,
      `${FILMSTRIP_LEDGER_PATH} is stale — run ` +
        `\`npm run generate:filmstrip-ledger\` in script-ductus`,
    ).toBe(expected);
  });

  it("is byte-identical when generated twice", () => {
    // The book's `check:figures` gate compares bytes, so a filmstrip that
    // rendered differently on two runs would turn a green build red at random.
    expect(currentLedgerBytes()).toBe(currentLedgerBytes());
  });

  it("draws every frame from a cited stroke order", () => {
    const ledger = JSON.parse(currentLedgerBytes()) as {
      entries: Array<{ source: { url: string; citation: string }; frames: unknown[] }>;
    };
    expect(ledger.entries.length).toBeGreaterThan(0);
    for (const entry of ledger.entries) {
      expect(entry.source.citation).not.toBe("");
      expect(entry.source.url).toMatch(/^https?:\/\//);
      expect(entry.frames.length).toBeGreaterThan(0);
    }
  });
});

describe("building one entry", () => {
  const letter = ductusFor("\u0b85", "tamil")!;
  const tamil = outlineFor("tamil", "\u0b85");

  it("carries the authored labels through untouched", () => {
    const built = buildFilmstripEntry(letter, tamil.outline, tamil.font);
    expect(built.frames.map((frame) => frame.label)).toEqual(
      letter.strokes.flatMap((stroke) => stroke.segments.map((segment) => segment.label)),
    );
    expect(built.font).toBe(tamil.font);
    expect(built.source).toEqual(letter.source);
  });

  it("emits only drawing tags, never a script or a handler", () => {
    for (const frame of buildFilmstripEntry(letter, tamil.outline, tamil.font).frames) {
      expect(frame.markup).not.toMatch(/<(?!\/?(?:g|path|circle|text|tspan)[\s/>])/);
      expect(frame.markup).not.toMatch(/\son[a-z]+=/i);
      expect(frame.markup.startsWith("<g ")).toBe(true);
    }
  });

  it("sizes the caption from the letter's own box", () => {
    const size = captionSizeFor(letter, tamil.outline, {});
    expect(size).toBeGreaterThan(0);
    // An explicit caption size wins over the automatic one.
    const forced = buildFilmstripEntry(letter, tamil.outline, tamil.font, {
      highlightSegment: true,
      captionSize: size * 2,
    });
    expect(forced.viewBox.height).toBeGreaterThan(
      buildFilmstripEntry(letter, tamil.outline, tamil.font).viewBox.height,
    );
  });

  it("refuses a letter with nothing to draw", () => {
    expect(() =>
      buildFilmstripEntry({ ...letter, strokes: [] }, tamil.outline, tamil.font),
    ).toThrow(/no filmstrip frames/);
  });
});

describe("assembling the ledger", () => {
  const sample = (script: string, glyph: string): FilmstripEntry => ({
    script,
    glyph,
    font: "_fonts/X.ttf",
    source: { citation: "c", url: "https://example.org/c" },
    penLifts: 0,
    summary: "one unbroken stroke \u00b7 1 movement",
    viewBox: { minX: 0, minY: 0, width: 1, height: 1 },
    frames: [{ number: 1, label: "l", startsAfterLift: false, markup: "<path/>" }],
  });

  it("puts the entries in one order that depends only on their content", () => {
    const forward = buildFilmstripLedger([
      sample("tamil", "\u0b86"),
      sample("devanagari", "\u0906"),
      sample("tamil", "\u0b85"),
    ]);
    const shuffled = buildFilmstripLedger([
      sample("tamil", "\u0b85"),
      sample("tamil", "\u0b86"),
      sample("devanagari", "\u0906"),
    ]);
    expect(serialiseFilmstripLedger(forward)).toBe(serialiseFilmstripLedger(shuffled));
    expect(forward.entries.map((entry) => entry.script)).toEqual([
      "devanagari",
      "tamil",
      "tamil",
    ]);
  });

  it("refuses to hold the same letter twice", () => {
    expect(() =>
      buildFilmstripLedger([sample("tamil", "\u0b85"), sample("tamil", "\u0b85")]),
    ).toThrow(/duplicate entry/);
  });

  it("ends in exactly one newline, so the file is diffable", () => {
    const text = serialiseFilmstripLedger(buildFilmstripLedger([sample("tamil", "\u0b85")]));
    expect(text.endsWith("}\n")).toBe(true);
    expect(text.endsWith("\n\n")).toBe(false);
  });
});
